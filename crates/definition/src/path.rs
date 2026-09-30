//! Addressing a particular value inside a [`Definition`](crate::Definition).

use std::fmt;
use std::ops::Deref;

use facet::Facet;
use thiserror::Error;

use crate::value::Value;

/// The value a path addresses, which is either in the tree or decoded out of a run.
///
/// # Why this is an enum
///
/// A packed list carries a run of bytes, and an element of it has to be decoded to be a
/// [`Value`]. A decoded element is a temporary with no address in the tree, so a path walk that
/// reached into a run cannot hand back a borrow of it. It hands back the value instead, and the
/// two cases are told apart here rather than at every call site.
///
/// `Deref` makes this read like the borrow it usually is: a caller that only reads writes
/// `&*resolved`, or calls a method on it directly, and never names this type at all.
#[derive(Debug, Clone, PartialEq)]
pub enum Resolved<'value> {
    /// The value is in the tree, and this borrows it.
    Borrowed(&'value Value),
    /// The value was decoded out of a packed run and is owned here.
    Owned(Value),
}

impl Deref for Resolved<'_> {
    type Target = Value;

    fn deref(&self) -> &Value {
        match self {
            Self::Borrowed(value) => value,
            Self::Owned(value) => value,
        }
    }
}

impl<'value> Resolved<'value> {
    /// Borrows the value this addresses.
    pub fn as_value(&self) -> &Value {
        match self {
            Self::Borrowed(value) => value,
            Self::Owned(value) => value,
        }
    }

    /// Returns whether this borrows the tree rather than owning a decoded element.
    pub const fn is_borrowed(&self) -> bool {
        matches!(self, Self::Borrowed(_))
    }
}

/// One step of a [`Path`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Facet)]
#[repr(C)]
pub enum PathSegment {
    /// Addresses a field of a struct, a dict, or the fields of an enum variant.
    Key(String),
    /// Addresses an element of a list.
    Index(usize),
}

impl fmt::Display for PathSegment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Key(name) => formatter.write_str(name),
            Self::Index(index) => write!(formatter, "[{index}]"),
        }
    }
}

/// A path could not be resolved against a value.
#[derive(Debug, Error)]
pub enum PathError {
    #[error("a key segment cannot address {0}, which has no named fields")]
    NotKeyed(&'static str),
    #[error("an index segment cannot address {0}, which is not a list")]
    NotIndexed(&'static str),
    #[error("`{0}` is absent")]
    AbsentKey(String),
    #[error("index {index} is out of bounds for a list of length {length}")]
    IndexOutOfBounds { index: usize, length: usize },
    #[error("index {index} is out of bounds for a packed list of length {length}: {reason}")]
    NotIndexedInRun {
        index: usize,
        length: usize,
        reason: String,
    },
    #[error("a path is a list of segments, and this is {found}")]
    NotAPath { found: &'static str },
    #[error("a path segment is a string key or an integer index, and this is {found}")]
    NotASegment { found: &'static str },
}

/// The chain of keys and indices that accesses a certain value in a definition.
///
/// # Invariants
///
/// An index segment only addresses a list, and a key segment only addresses a struct,
/// a dict, or the fields of an enum variant. [`Path::resolve`] and [`Path::resolve_mut`]
/// are the only places these invariants are enforced.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Facet)]
pub struct Path(Vec<PathSegment>);

impl Path {
    /// Creates a path from its segments.
    pub fn new(segments: impl IntoIterator<Item = PathSegment>) -> Self {
        Self(segments.into_iter().collect())
    }

    /// Creates the path that addresses the definition root.
    pub fn root() -> Self {
        Self(Vec::new())
    }

    /// Returns the segments of this path.
    pub fn segments(&self) -> &[PathSegment] {
        &self.0
    }

    /// Returns whether this path addresses the definition root.
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns a new path with `segment` appended.
    #[must_use]
    pub fn child(&self, segment: PathSegment) -> Self {
        let mut segments = self.0.clone();
        segments.push(segment);
        Self(segments)
    }

    /// Returns a new path with a key segment appended.
    #[must_use]
    pub fn key(&self, name: impl Into<String>) -> Self {
        self.child(PathSegment::Key(name.into()))
    }

    /// Returns a new path with an index segment appended.
    #[must_use]
    pub fn index(&self, index: usize) -> Self {
        self.child(PathSegment::Index(index))
    }

    /// Returns the path of the parent, or `None` when this path is the root.
    pub fn parent(&self) -> Option<Self> {
        if self.0.is_empty() {
            return None;
        }
        Some(Self(self.0[..self.0.len() - 1].to_vec()))
    }

    /// Splits off the first segment, leaving the rest of the path.
    ///
    /// This is what an edit walks with: the first segment says which node to descend into and the
    /// rest says what to do once there.
    fn split_first(&self) -> Option<(&PathSegment, Path)> {
        let (first, rest) = self.0.split_first()?;
        Some((first, Self(rest.to_vec())))
    }

    /// Encodes this path as the value that carries it, which is how a reference spells the
    /// path it points at.
    ///
    /// # Invariants
    ///
    /// A key is a string and an index an integer, and [`Path::from_value`] is its inverse.
    /// The encoding is unambiguous by construction rather than by convention, because a
    /// dict in this language is keyed by strings and a list is addressed by position, so a
    /// segment's kind follows from the kind of value it is written as. Nothing needs
    /// quoting, so a key that contains a dot or a bracket is written as it is.
    pub fn to_value(&self) -> Value {
        Value::List(
            self.0
                .iter()
                .map(|segment| match segment {
                    PathSegment::Key(name) => Value::String(name.clone()),
                    PathSegment::Index(index) => Value::Int(*index as i64),
                })
                .collect(),
        )
    }

    /// Decodes the path a value carries, which [`Path::to_value`] wrote.
    pub fn from_value(value: &Value) -> Result<Self, PathError> {
        let Value::List(segments) = value else {
            return Err(PathError::NotAPath {
                found: value.kind_name(),
            });
        };
        let mut path = Vec::with_capacity(segments.len());
        for segment in segments {
            path.push(match segment {
                Value::String(name) => PathSegment::Key(name.clone()),
                Value::Int(index) if *index >= 0 => PathSegment::Index(*index as usize),
                Value::Int(_) => {
                    return Err(PathError::NotASegment {
                        found: "a negative index",
                    });
                }
                other => {
                    return Err(PathError::NotASegment {
                        found: other.kind_name(),
                    });
                }
            });
        }
        Ok(Self(path))
    }

    /// Borrows the value this path addresses, decoding it out of a packed run if it is in one.
    ///
    /// # Invariants
    ///
    /// An index segment addresses a list, in either of its two representations, and a key segment
    /// addresses a struct, a dict, or the fields of an enum variant. Reaching into a packed run
    /// decodes exactly one element, so the cost of a path is bounded by an element rather than by
    /// the length of the list it names.
    pub fn resolve<'value>(&self, value: &'value Value) -> Result<Resolved<'value>, PathError> {
        let mut at = 0;
        // The borrowed walk keeps a `&'value Value` the whole way, which is what lets it return a
        // borrow of the tree. A packed list has no address in the tree, so reaching one hands the
        // rest of the walk to an owned one, which decodes the element and walks the rest over it.
        let mut current = value;
        while let Some(segment) = self.0.get(at) {
            match segment {
                PathSegment::Key(name) => {
                    current = field(current, name)?;
                    at += 1;
                }
                PathSegment::Index(index) => match current {
                    Value::List(elements) => {
                        current = elements.get(*index).ok_or(PathError::IndexOutOfBounds {
                            index: *index,
                            length: elements.len(),
                        })?;
                        at += 1;
                    }
                    Value::Packed(list) => {
                        let element =
                            list.element(*index)
                                .map_err(|error| PathError::NotIndexedInRun {
                                    index: *index,
                                    length: list.len(),
                                    reason: error.to_string(),
                                })?;
                        return walk_owned(&self.0[at + 1..], element).map(Resolved::Owned);
                    }
                    other => return Err(PathError::NotIndexed(other.kind_name())),
                },
            }
        }
        Ok(Resolved::Borrowed(current))
    }

    /// Applies an edit to the value this path addresses, and returns what the edit returned.
    ///
    /// # Why this rather than a mutable borrow
    ///
    /// An edit into a packed run decodes the element, hands it to the edit, and encodes the result
    /// back, because a decoded element is a temporary and a borrow of it could not outlive the
    /// call. Doing it through a closure is what lets the two representations share one edit path:
    /// a caller writes the same code whether the value it is changing is a map in a tree or a
    /// stride in a run.
    ///
    /// # Invariants
    ///
    /// The walk descends one segment at a time, and a segment into a packed run decodes exactly
    /// the element it names, edits it, and writes it back. So editing `points[42].position[1]`
    /// of a run of a hundred thousand points costs one point, not one list.
    ///
    /// The edit runs at most once, and a path that does not resolve never runs it.
    pub fn update<R>(
        &self,
        value: &mut Value,
        edit: impl FnOnce(&mut Value) -> R,
    ) -> Result<R, PathError> {
        let Some((first, rest)) = self.split_first() else {
            return Ok(edit(value));
        };

        match first {
            PathSegment::Key(name) => {
                let found = value.kind_name();
                let fields = value.as_field_map_mut().ok_or(PathError::NotKeyed(found))?;
                let field = fields
                    .get_mut(name)
                    .ok_or_else(|| PathError::AbsentKey(name.clone()))?;
                if rest.is_root() {
                    Ok(edit(field))
                } else {
                    rest.update(field, edit)
                }
            }
            PathSegment::Index(index) => match value {
                Value::List(elements) => {
                    let length = elements.len();
                    let element = elements
                        .get_mut(*index)
                        .ok_or(PathError::IndexOutOfBounds {
                            index: *index,
                            length,
                        })?;
                    if rest.is_root() {
                        Ok(edit(element))
                    } else {
                        rest.update(element, edit)
                    }
                }
                Value::Packed(list) => {
                    let length = list.len();
                    let mut element =
                        list.element(*index)
                            .map_err(|error| PathError::NotIndexedInRun {
                                index: *index,
                                length,
                                reason: error.to_string(),
                            })?;
                    let result = if rest.is_root() {
                        edit(&mut element)
                    } else {
                        rest.update(&mut element, edit)?
                    };
                    list.set_element(*index, &element).map_err(|error| {
                        PathError::NotIndexedInRun {
                            index: *index,
                            length,
                            reason: error.to_string(),
                        }
                    })?;
                    Ok(result)
                }
                other => Err(PathError::NotIndexed(other.kind_name())),
            },
        }
    }

    /// Returns whether this path addresses something inside `value`.
    pub fn resolves(&self, value: &Value) -> bool {
        self.resolve(value).is_ok()
    }
}

impl fmt::Display for Path {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return formatter.write_str("<root>");
        }
        for (position, segment) in self.0.iter().enumerate() {
            match segment {
                PathSegment::Key(name) if position == 0 => formatter.write_str(name)?,
                PathSegment::Key(name) => write!(formatter, ".{name}")?,
                PathSegment::Index(index) => write!(formatter, "[{index}]")?,
            }
        }
        Ok(())
    }
}

/// Borrows a named field of a struct, a dict, or the fields of an enum variant.
fn field<'value>(value: &'value Value, name: &str) -> Result<&'value Value, PathError> {
    match value {
        Value::Struct(fields) | Value::Dict(fields) | Value::Enum { fields, .. } => fields
            .get(name)
            .ok_or_else(|| PathError::AbsentKey(name.to_owned())),
        other => Err(PathError::NotKeyed(other.kind_name())),
    }
}

/// Walks the segments left after a packed list has been decoded, and returns what they name.
///
/// This is the owned half of a walk, and it exists for a concrete reason: a decoded element has
/// no address in the document, so nothing below it can be borrowed out to a caller. The walk
/// therefore finishes by value. It recurses when the path reaches *another* run, which is the one
/// case where the remaining segments are not already in hand.
fn walk_owned(segments: &[PathSegment], value: Value) -> Result<Value, PathError> {
    let mut current = value;
    let mut at = 0;
    while let Some(segment) = segments.get(at) {
        match segment {
            PathSegment::Key(name) => {
                current = field(&current, name)?.clone();
                at += 1;
            }
            PathSegment::Index(index) => match &current {
                Value::List(elements) => {
                    let length = elements.len();
                    current = elements
                        .get(*index)
                        .cloned()
                        .ok_or(PathError::IndexOutOfBounds {
                            index: *index,
                            length,
                        })?;
                    at += 1;
                }
                Value::Packed(list) => {
                    let element =
                        list.element(*index)
                            .map_err(|error| PathError::NotIndexedInRun {
                                index: *index,
                                length: list.len(),
                                reason: error.to_string(),
                            })?;
                    return walk_owned(&segments[at + 1..], element);
                }
                other => return Err(PathError::NotIndexed(other.kind_name())),
            },
        }
    }
    Ok(current)
}

impl FromIterator<PathSegment> for Path {
    fn from_iter<Segments: IntoIterator<Item = PathSegment>>(segments: Segments) -> Self {
        Self::new(segments)
    }
}

impl IntoIterator for Path {
    type Item = PathSegment;
    type IntoIter = std::vec::IntoIter<PathSegment>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
