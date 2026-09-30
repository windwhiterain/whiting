//! Addressing a particular value inside a [`Definition`](crate::Definition).

use std::fmt;

use facet::Facet;
use thiserror::Error;

use crate::value::Value;

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

    /// Borrows the value this path addresses.
    pub fn resolve<'value>(&self, value: &'value Value) -> Result<&'value Value, PathError> {
        let mut current = value;
        for segment in &self.0 {
            current = match (segment, current) {
                (PathSegment::Key(name), Value::Struct(fields)) => fields
                    .get(name)
                    .ok_or_else(|| PathError::AbsentKey(name.clone()))?,
                (PathSegment::Key(name), Value::Dict(entries)) => entries
                    .get(name)
                    .ok_or_else(|| PathError::AbsentKey(name.clone()))?,
                (PathSegment::Key(name), Value::Enum { fields, .. }) => fields
                    .get(name)
                    .ok_or_else(|| PathError::AbsentKey(name.clone()))?,
                (PathSegment::Key(_), other) => return Err(PathError::NotKeyed(other.kind_name())),
                (PathSegment::Index(index), Value::List(elements)) => elements
                    .get(*index)
                    .ok_or(PathError::IndexOutOfBounds {
                        index: *index,
                        length: elements.len(),
                    })?,
                (PathSegment::Index(_), other) => {
                    return Err(PathError::NotIndexed(other.kind_name()));
                }
            };
        }
        Ok(current)
    }

    /// Mutably borrows the value this path addresses.
    pub fn resolve_mut<'value>(
        &self,
        value: &'value mut Value,
    ) -> Result<&'value mut Value, PathError> {
        let mut current = value;
        for segment in &self.0 {
            current = match (segment, current) {
                (PathSegment::Key(name), Value::Struct(fields)) => fields
                    .get_mut(name)
                    .ok_or_else(|| PathError::AbsentKey(name.clone()))?,
                (PathSegment::Key(name), Value::Dict(entries)) => entries
                    .get_mut(name)
                    .ok_or_else(|| PathError::AbsentKey(name.clone()))?,
                (PathSegment::Key(name), Value::Enum { fields, .. }) => fields
                    .get_mut(name)
                    .ok_or_else(|| PathError::AbsentKey(name.clone()))?,
                (PathSegment::Key(_), other) => return Err(PathError::NotKeyed(other.kind_name())),
                (PathSegment::Index(index), Value::List(elements)) => {
                    let length = elements.len();
                    elements
                        .get_mut(*index)
                        .ok_or(PathError::IndexOutOfBounds {
                            index: *index,
                            length,
                        })?
                }
                (PathSegment::Index(_), other) => {
                    return Err(PathError::NotIndexed(other.kind_name()));
                }
            };
        }
        Ok(current)
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
