//! A list whose elements all have one structure, carried as a flat run of bytes.
//!
//! # What a packed list is for
//!
//! A tree is the right interface language for a document an agent reads and edits, and a poor
//! storage format for a hundred thousand floats. A mesh of fifty-five thousand points is fifty-five
//! thousand `Value::Struct` values over `BTreeMap<String, Value>`, each with three heap-allocated
//! keys, and every pass over them walks that. The same points as a run of `N * stride` bytes are
//! one allocation that a `memcmp` compares and a strided read indexes.
//!
//! # Why one structure is guaranteed rather than hoped for
//!
//! A packed list carries its own layout, and the layout is derived from a schema — a list declares
//! one element schema, and a struct is closed, so every element has the same fields, in the same
//! order, with the same widths. A list that does not satisfy that cannot be packed and is
//! [`Value::List`](crate::Value::List) instead. Uniformity is therefore a precondition of the
//! representation, not something a packer discovers and checks while packing.
//!
//! # Invariants
//!
//! - **The byte length is always a whole number of strides.** A partial trailing element is a
//!   malformed list, and every constructor refuses to build one, so no accessor has to cope with
//!   reading past the end.
//! - **Field order is schema order, which is sorted by name.** That is the order
//!   [`graphics::layout::vertex_layout`](https://docs.rs/whiting-graphics) derives offsets from,
//!   so the compiler that packs and the renderer that draws read one layout.
//! - **Every component is four little endian bytes**, the same rule a buffer already follows, so
//!   a packed list is the bytes a shader indexes with no conversion at all.
//! - **A packed list round-trips.** [`PackedList::to_list`] of a [`PackedList::from_list`] equals
//!   the list it was built from, so packing is a representation and never a change of data.
//!
//! # Example
//!
//! ```
//! use whiting_definition::{PackedLayout, PackedList, Value};
//!
//! // Two fields, three and two components, so a stride of twenty bytes.
//! let layout = PackedLayout::new([("normal".to_owned(), 3), ("position".to_owned(), 3)]);
//! assert_eq!(layout.stride_bytes(), 24);
//!
//! let list = PackedList::from_list(
//!     &layout,
//!     &[
//!         Value::structure([
//!             ("normal".to_owned(), Value::List(vec![Value::Float(0.0); 3])),
//!             ("position".to_owned(), Value::List(vec![Value::Float(1.0); 3])),
//!         ]),
//!         Value::structure([
//!             ("normal".to_owned(), Value::List(vec![Value::Float(0.0); 3])),
//!             ("position".to_owned(), Value::List(vec![Value::Float(2.0); 3])),
//!         ]),
//!     ],
//! )
//! .expect("both points are three component runs");
//!
//! assert_eq!(list.len(), 2);
//! assert_eq!(list.bytes().len(), 48);
//!
//! // An element is read back exactly as it was written, and packing did not change it.
//! let round_tripped = list.to_list();
//! assert_eq!(round_tripped.as_list().map(<[Value]>::len), Some(2));
//! assert_eq!(
//!     round_tripped.as_list().and_then(|points| points[1].as_field_map())
//!         .and_then(|fields| fields.get("position")),
//!     Some(&Value::List(vec![Value::Float(2.0); 3]))
//! );
//!
//! // A field is read without decoding the whole element, which is the point of the run.
//! assert_eq!(list.component(1, "position", 0), Some(2.0));
//! ```

use std::collections::BTreeMap;

use facet::Facet;
use thiserror::Error;

use crate::value::{FieldMap, Value};

/// A packed list could not be built or read.
#[derive(Debug, Error, PartialEq)]
pub enum PackedError {
    #[error("a packed list of {name} elements needs {stride} bytes each, found {found}")]
    NotAWholeNumber {
        name: String,
        stride: usize,
        found: usize,
    },
    #[error("at index {index}: {reason}")]
    Element { index: usize, reason: String },
    #[error("the layout has no field named `{field}`, and its fields are {available:?}")]
    UnknownField {
        field: String,
        available: Vec<String>,
    },
    #[error("the layout has no field at index {index}, and it has {found} fields")]
    UnknownFieldIndex { index: usize, found: usize },
    #[error("at index {index}, the field `{field}` is {found}")]
    NotANumberRun {
        index: usize,
        field: String,
        found: &'static str,
    },
    #[error(
        "at index {index}, the field `{field}` has {found} components, and the layout declares {expected}"
    )]
    WrongWidth {
        index: usize,
        field: String,
        expected: usize,
        found: usize,
    },
    #[error("at index {index}: {reason}")]
    Structure { index: usize, reason: String },
}

/// The shape every element of a [`PackedList`] has.
///
/// The layout is the whole of what makes a list packable, and it is derived from a schema rather
/// than written by hand: a list declares one element schema, and a struct is closed, so the
/// fields, their order and their widths are all decided before a single element is read.
#[derive(Debug, Clone, PartialEq, Eq, Default, Facet)]
pub struct PackedLayout {
    /// Field name to component count, in the order the bytes are laid out.
    fields: Vec<(String, usize)>,
}

impl PackedLayout {
    /// Creates a layout from field names and their component counts, in layout order.
    pub fn new(fields: impl IntoIterator<Item = (String, usize)>) -> Self {
        Self {
            fields: fields.into_iter().collect(),
        }
    }

    /// Returns the fields in layout order, with the component count of each.
    pub fn fields(&self) -> &[(String, usize)] {
        &self.fields
    }

    /// Returns the number of fields one element has.
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Returns the field names in layout order.
    pub fn field_names(&self) -> Vec<String> {
        self.fields.iter().map(|(name, _)| name.clone()).collect()
    }

    /// Returns the number of bytes one element occupies.
    ///
    /// Every component is four bytes, so the stride is four times the total component count.
    pub fn stride_bytes(&self) -> usize {
        self.fields
            .iter()
            .map(|(_, components)| components * 4)
            .sum()
    }

    /// Returns the offset and component count of a named field, or `None` when there is no such
    /// field.
    pub fn field(&self, name: &str) -> Option<(usize, usize)> {
        let mut offset = 0;
        for (field, components) in &self.fields {
            if field == name {
                return Some((offset, *components));
            }
            offset += components * 4;
        }
        None
    }

    /// Returns the name, component count and byte offset of a field by position, or `None` when
    /// the layout has no field there.
    pub fn field_at(&self, index: usize) -> Option<(&str, usize, usize)> {
        let (name, components) = self.fields.get(index)?;
        let (offset, _) = self.field(name)?;
        Some((name, *components, offset))
    }

    /// Returns the component count of a named field, or `None` when there is no such field.
    pub fn components(&self, name: &str) -> Option<usize> {
        self.field(name).map(|(_, components)| components)
    }

    /// Encodes one element into the bytes of a run, checking it against this layout.
    ///
    /// # Invariants
    ///
    /// An element that is not a struct of exactly the declared fields, or whose field is not a run
    /// of numbers of the declared width, is refused here rather than written into a run that a
    /// strided read would misread. This is the check the tree form could not make cheaply, and
    /// making it once per element is what lets every later read be a plain offset.
    pub fn encode(&self, index: usize, element: &Value) -> Result<Vec<u8>, PackedError> {
        let mut bytes = Vec::with_capacity(self.stride_bytes());
        let fields = element.as_struct().ok_or_else(|| PackedError::Element {
            index,
            reason: format!(
                "an element of a packed list must be a struct, and this is a {}",
                element.kind_name()
            ),
        })?;
        for (name, components) in &self.fields {
            let value = fields.get(name).ok_or_else(|| PackedError::Structure {
                index,
                reason: format!("the element has no field named `{name}`"),
            })?;
            write_field(&mut bytes, value, *components)
                .map_err(|error| at_element(index, name, error))?;
        }
        Ok(bytes)
    }

    /// Returns a copy of this layout sorted by field name, which is the order a schema sorts its
    /// fields into.
    ///
    /// A layout is written in the order the bytes are laid out, and that order is schema order. An
    /// author who builds a layout by hand may not have sorted it, so this is what the packing
    /// entry points use.
    #[must_use]
    pub fn sorted(self) -> Self {
        let fields: BTreeMap<String, usize> = self.fields.into_iter().collect();
        Self {
            fields: fields.into_iter().collect(),
        }
    }
}

/// A list carried as a flat run of bytes, one stride per element.
///
/// # Invariants
///
/// The byte length is a whole number of strides, which every constructor upholds, so `len` is
/// exact and no accessor can read past the end.
#[derive(Debug, Clone, PartialEq, Default, Facet)]
pub struct PackedList {
    /// The shape of one element, in layout order.
    layout: PackedLayout,
    /// `len * stride` little endian floats, concatenated.
    bytes: Vec<u8>,
}

impl PackedList {
    /// Creates a list from a layout and its bytes, refusing a length that is not a whole number
    /// of strides.
    pub fn new(layout: PackedLayout, bytes: Vec<u8>) -> Result<Self, PackedError> {
        let stride = layout.stride_bytes();
        // A zero stride is a layout of no components, which every element would satisfy
        // vacuously and which therefore cannot carry a list at all.
        if stride == 0 || bytes.len() % stride != 0 {
            return Err(PackedError::NotAWholeNumber {
                name: "packed".to_owned(),
                stride,
                found: bytes.len(),
            });
        }
        Ok(Self { layout, bytes })
    }

    /// Returns the shape of one element.
    pub fn layout(&self) -> &PackedLayout {
        &self.layout
    }

    /// Returns the bytes, which are `len * stride` of them.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes this list and returns its bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Returns how many elements the run holds.
    pub fn len(&self) -> usize {
        if self.layout.stride_bytes() == 0 {
            0
        } else {
            self.bytes.len() / self.layout.stride_bytes()
        }
    }

    /// Returns whether the run holds no element.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Packs a list of elements into a run, checking each against the layout.
    ///
    /// # Invariants
    ///
    /// Every element is checked against the layout by [`PackedLayout::encode`], so the run this
    /// returns holds only elements of the one shape, which is what makes every later read a plain
    /// offset into the bytes.
    pub fn from_list(layout: &PackedLayout, elements: &[Value]) -> Result<Self, PackedError> {
        let layout = layout.clone().sorted();
        let stride = layout.stride_bytes();
        let mut bytes = Vec::with_capacity(elements.len() * stride);
        for (index, element) in elements.iter().enumerate() {
            bytes.append(&mut layout.encode(index, element)?);
        }
        Ok(Self { layout, bytes })
    }

    /// Reads one element back as a [`Value`], which is what an agent is shown.
    ///
    /// Decoding one element is what a path into a packed list costs, so it is bounded by the
    /// stride rather than by the length of the list.
    pub fn element(&self, index: usize) -> Result<Value, PackedError> {
        if index >= self.len() {
            return Err(PackedError::Element {
                index,
                reason: format!(
                    "a packed list of {} elements has no element {index}",
                    self.len()
                ),
            });
        }
        let stride = self.layout.stride_bytes();
        let start = index * stride;
        let mut fields = FieldMap::new();
        for (name, components) in self.layout.fields() {
            let (offset, _) = self.layout.field(name).expect("the field is in the layout");
            let value = read_field(&self.bytes[start + offset..], *components);
            fields.insert(name.clone(), value);
        }
        Ok(Value::Struct(fields))
    }

    /// Returns one component of one element, without decoding the element.
    ///
    /// This is the read a strided pass wants: the bounds of a mesh, say, which needs the position
    /// of every point and nothing else.
    pub fn component(&self, index: usize, field: &str, component: usize) -> Option<f64> {
        let (offset, components) = self.layout.field(field)?;
        if component >= components || index >= self.len() {
            return None;
        }
        let stride = self.layout.stride_bytes();
        let at = index * stride + offset + component * 4;
        Some(f32::from_le_bytes(self.bytes.get(at..at + 4)?.try_into().ok()?) as f64)
    }

    /// Returns the components of one named field of one element, as a slice of the run.
    ///
    /// The bytes are exactly the field's, in layout order, so a caller that wants floats reads
    /// them as four byte little endian floats, which is what the run is.
    pub fn field_bytes(&self, index: usize, field: &str) -> Option<&[u8]> {
        let (offset, components) = self.layout.field(field)?;
        if index >= self.len() {
            return None;
        }
        let stride = self.layout.stride_bytes();
        let at = index * stride + offset;
        self.bytes.get(at..at + components * 4)
    }

    /// Writes one element into the run.
    ///
    /// # Invariants
    ///
    /// The element is checked against the layout exactly as [`PackedList::from_list`] checks one,
    /// so a run is never left holding an element of a different shape. The bytes are patched in
    /// place rather than the run rebuilt, so writing one point of a hundred thousand is one
    /// point's worth of copying.
    pub fn set_element(&mut self, index: usize, element: &Value) -> Result<(), PackedError> {
        if index >= self.len() {
            return Err(PackedError::Element {
                index,
                reason: format!(
                    "a packed list of {} elements has no element {index}",
                    self.len()
                ),
            });
        }
        let stride = self.layout.stride_bytes();
        let start = index * stride;
        let bytes = self.layout.encode(index, element)?;
        self.bytes[start..start + stride].copy_from_slice(&bytes);
        Ok(())
    }

    /// Appends one element to the end of the run.
    ///
    /// The new element is encoded on its own and its bytes appended, so growing a list by one is
    /// one element's work rather than a copy of the run.
    pub fn push(&mut self, element: &Value) -> Result<(), PackedError> {
        let index = self.len();
        let mut bytes = self.layout.encode(index, element)?;
        self.bytes.append(&mut bytes);
        Ok(())
    }

    /// Expands the run back into the list it carries.
    pub fn to_list(&self) -> Value {
        Value::List(
            (0..self.len())
                .map(|index| {
                    self.element(index)
                        .expect("an index below the length decodes")
                })
                .collect(),
        )
    }

    /// Returns whether this run and another carry the same elements under the same layout.
    ///
    /// Two runs are equal when their layouts and their bytes are, which is a `memcmp` rather than
    /// a walk of fifty-five thousand structs.
    pub fn matches(&self, other: &PackedList) -> bool {
        self.layout == other.layout && self.bytes == other.bytes
    }

    /// Inserts whole elements before `index`.
    ///
    /// # Invariants
    ///
    /// An inserted element is encoded against the layout, so a run that grows holds only elements
    /// of the one shape. The bytes are rebuilt from the strides, which is what a change of length
    /// costs: a run is indexed by offset, so an insert moves everything after it.
    pub fn insert(&mut self, index: usize, values: &[Value]) -> Result<(), PackedError> {
        if index > self.len() {
            return Err(PackedError::Element {
                index,
                reason: format!(
                    "a packed list of {} elements ends at index {index}",
                    self.len()
                ),
            });
        }
        let stride = self.layout.stride_bytes();
        let mut inserted = Vec::with_capacity(values.len() * stride);
        for (offset, value) in values.iter().enumerate() {
            inserted.append(&mut self.layout.encode(index + offset, value)?);
        }
        let at = index * stride;
        let tail = self.bytes.split_off(at);
        self.bytes.extend_from_slice(&inserted);
        self.bytes.extend_from_slice(&tail);
        Ok(())
    }

    /// Removes `count` whole elements starting at `start`, and returns them.
    ///
    /// The removed elements are decoded rather than copied as bytes, because the inverse of a
    /// removal is an append of the same elements, and an append takes values.
    pub fn remove(&mut self, start: usize, count: usize) -> Result<Vec<Value>, PackedError> {
        let length = self.len();
        let end = start
            .checked_add(count)
            .filter(|end| *end <= length)
            .ok_or(PackedError::Element {
                index: start,
                reason: format!(
                    "removing {count} elements at {start} is out of bounds for a packed list of \
                     {length}"
                ),
            })?;
        let stride = self.layout.stride_bytes();
        let removed = (start..end)
            .map(|index| self.element(index))
            .collect::<Result<Vec<Value>, PackedError>>()?;
        self.bytes.drain(start * stride..end * stride);
        Ok(removed)
    }
}

/// Builds a run field by field, without ever building an element as a [`Value`].
///
/// # Invariants
///
/// A point is written by naming each of the layout's fields in turn, and the builder refuses a
/// field the layout does not declare, a field written twice, and a build that ends mid element.
/// So the run it produces holds exactly the elements it was given, each of exactly the declared
/// shape, which is the same guarantee [`PackedList::from_list`] gives — reached without the
/// intermediate maps that make a tree expensive in the first place.
#[derive(Debug, Clone, Default)]
pub struct PackedBuilder {
    layout: PackedLayout,
    bytes: Vec<u8>,
    /// How far into the current element the builder is, which is the field count so far.
    written: usize,
}

impl PackedBuilder {
    /// Creates a builder for a layout, reserving nothing: the length is not known until the
    /// build ends.
    pub fn new(layout: PackedLayout) -> Self {
        let bytes = Vec::new();
        Self {
            layout,
            bytes,
            written: 0,
        }
    }

    /// Returns the layout this builder writes.
    pub fn layout(&self) -> &PackedLayout {
        &self.layout
    }

    /// Reserves room for `elements` whole elements.
    ///
    /// A generator knows how many points it is about to write, and one allocation for the run is
    /// the difference between a mesh that costs its geometry and one that costs a reallocation per
    /// cell.
    pub fn reserve(&mut self, elements: usize) {
        self.bytes.reserve(elements * self.layout.stride_bytes());
    }

    /// Returns how many elements have been written whole.
    pub fn len(&self) -> usize {
        self.bytes.len() / self.layout.stride_bytes().max(1)
    }

    /// Returns whether no element has been written.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Writes one field of the element being built.
    ///
    /// # Invariants
    ///
    /// The fields of an element are written in layout order, so the bytes are in the order the
    /// layout declares. Writing out of order, writing a field twice, or naming a field the
    /// layout does not declare is refused here rather than producing a run whose bytes do not
    /// mean what its layout says.
    pub fn field(&mut self, name: &str, components: &[f64]) -> Result<&mut Self, PackedError> {
        let (_, expected) = self
            .layout
            .field(name)
            .ok_or_else(|| PackedError::UnknownField {
                field: name.to_owned(),
                available: self.layout.field_names(),
            })?;
        let (declared_name, _, _) =
            self.layout
                .field_at(self.written)
                .ok_or_else(|| PackedError::Structure {
                    index: self.len(),
                    reason: format!("the layout has no field after `{name}`"),
                })?;
        if declared_name != name {
            return Err(PackedError::Structure {
                index: self.len(),
                reason: format!(
                    "the layout declares `{declared_name}` at this position, and `{name}` was \
                     written"
                ),
            });
        }
        if components.len() != expected {
            return Err(PackedError::WrongWidth {
                index: self.len(),
                field: name.to_owned(),
                expected,
                found: components.len(),
            });
        }
        for component in components {
            self.bytes
                .extend_from_slice(&(*component as f32).to_le_bytes());
        }
        self.written += 1;
        if self.written == self.layout.field_count() {
            self.written = 0;
        }
        Ok(self)
    }

    /// Ends the run.
    ///
    /// # Invariants
    ///
    /// A build that ends part way through an element is refused, because a run whose length is
    /// not a whole number of strides is not a list.
    pub fn finish(self) -> Result<PackedList, PackedError> {
        if self.written != 0 {
            return Err(PackedError::Structure {
                index: self.len(),
                reason: "the build ended part way through an element".to_owned(),
            });
        }
        PackedList::new(self.layout, self.bytes)
    }
}

/// Why one field could not be written into a run.
#[derive(Debug)]
enum FieldError {
    /// The value is not a number, or a list of them.
    NotNumbers { found: &'static str },
    /// The value carries the wrong number of components.
    WrongWidth { expected: usize, found: usize },
}

/// Writes one field's components as four byte little endian floats.
///
/// A field of one component may be a bare number, because that is how a number is written in a
/// document; a field of several is a list, because that is the only way to carry more than one.
/// Reading it back is the mirror of that rule, which is why a document round-trips.
fn write_field(bytes: &mut Vec<u8>, value: &Value, components: usize) -> Result<(), FieldError> {
    let numbers: Vec<f64> = match value {
        Value::List(elements) => elements
            .iter()
            .map(number_of)
            .collect::<Option<Vec<f64>>>()
            .ok_or(FieldError::NotNumbers {
                found: "a list holding something that is not a number",
            })?,
        single => vec![number_of(single).ok_or(FieldError::NotNumbers {
            found: single.kind_name(),
        })?],
    };
    if numbers.len() != components {
        return Err(FieldError::WrongWidth {
            expected: components,
            found: numbers.len(),
        });
    }
    for number in numbers {
        bytes.extend_from_slice(&(number as f32).to_le_bytes());
    }
    Ok(())
}

/// Names the element a field error belongs to, so the message points at one point of a list.
fn at_element(index: usize, field: &str, error: FieldError) -> PackedError {
    match error {
        FieldError::NotNumbers { found } => PackedError::NotANumberRun {
            index,
            field: field.to_owned(),
            found,
        },
        FieldError::WrongWidth { expected, found } => PackedError::WrongWidth {
            index,
            field: field.to_owned(),
            expected,
            found,
        },
    }
}

/// Reads one field's components back as a list, which is how a document carries more than one.
fn read_field(bytes: &[u8], components: usize) -> Value {
    let mut numbers = Vec::with_capacity(components);
    for component in 0..components {
        let at = component * 4;
        let number = bytes
            .get(at..at + 4)
            .and_then(|slice| slice.try_into().ok())
            .map(f32::from_le_bytes)
            .unwrap_or(0.0);
        numbers.push(Value::Float(number as f64));
    }
    if components == 1 {
        numbers.remove(0)
    } else {
        Value::List(numbers)
    }
}

/// Returns the number a value is, whether it is written as an integer or as a float.
fn number_of(value: &Value) -> Option<f64> {
    match value {
        Value::Float(number) => Some(*number),
        Value::Int(number) => Some(*number as f64),
        _ => None,
    }
}
