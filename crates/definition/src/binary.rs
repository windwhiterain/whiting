//! How a document is written in bytes.
//!
//! # Why a binary form
//!
//! A document is a [`Value`] tree, and a tree of text is a tree of decimal numbers and of
//! base64. The `alpine_oak` scene is 652,794,206 bytes of json for 421,799,808 bytes of packed
//! runs and 3,315,173 values, and the bulk of it is spent twice over: the runs are carried
//! as base64, which costs four text bytes for every three bytes of data, and every row and
//! every float is written as decimal text that has to be parsed back.
//!
//! Written as bytes the same document is 480,911,625 bytes, and it reads back in 0.63 s rather
//! than 7.14 s: a run is a length and the bytes, and a float is its bits.
//!
//! The measurement is `crates/gallery/examples/document_binary_probe.rs`, which reports the
//! same document in both spellings and says what each costs to write and to read, and
//! `.agents/investigate/the-cost-of-the-binary-form-of-a-scene-document.md`, which records it.
//!
//! # The format
//!
//! A document is a magic, a version, and one value.
//!
//! ```text
//! magic     8 bytes   b"WHITDOC\0"
//! version   8 bytes   the format version
//! value     ...       the document
//! ```
//!
//! **Every number in the format is a little endian `u64`**, except the tag in front of a
//! value and the byte in front of a boolean, which are one byte each. Every length counts
//! bytes and every count counts elements, and the two are never mixed.
//!
//! ## The tags
//!
//! | tag | value | what follows it |
//! | --- | --- | --- |
//! | `0x01` | `Unit` | nothing |
//! | `0x02` | `Bool` | one byte, `0` or `1` |
//! | `0x03` | `Int` | the eight bytes of an `i64` |
//! | `0x04` | `Float` | the eight bytes of the `f64`'s bits |
//! | `0x05` | `String` | a length, then the utf-8 bytes |
//! | `0x06` | `Bytes` | a length, then the bytes |
//! | `0x07` | `Struct` | a field map |
//! | `0x08` | `Enum` | the tag as a string, then a field map |
//! | `0x09` | `List` | a count, then that many values |
//! | `0x0A` | `Packed` | a layout, then a run |
//! | `0x0B` | `Dict` | a field map |
//!
//! A **field map** is a count and then that many (name, value) pairs. A **layout** is a
//! count and then that many (name, component count) pairs, in the order the bytes are laid
//! out in. A **run** is a length and then the bytes.
//!
//! # Invariants
//!
//! - **The magic and the version are at the head, and they are checked first.** A file that
//!   is not this format is refused by name, so it can never be read as a document that
//!   happens to be spelled differently. The version is in the file rather than in the
//!   reader's head because the reader's head is a moving target: the `facet-json` coupling
//!   recorded in `crates/device/examples/facet_value_probe.rs` is exactly the failure a
//!   version number turns into a refusal.
//! - **One value has one spelling.** A field map is written in the map's own order, which
//!   is sorted by name, and a layout is written in its own order, which is the order its
//!   bytes are laid out in. Nothing else varies: the same value is the same bytes on every
//!   run and in every process, so two documents can be compared by their bytes.
//! - **A float is written as its bits.** A `NaN` payload and a negative zero therefore
//!   survive the trip, which decimal text cannot promise: `f64::NAN` has no decimal
//!   spelling at all, and `-0.0` and `0.0` are the same number to every comparison that is
//!   not a comparison of bits.
//! - **Bytes are raw.** A [`Value::Bytes`] and the run of a [`PackedList`] are a length and
//!   the bytes, never base64 and never a decimal number per octet. A packed list writes its
//!   layout — the field name and the component count of each field, in layout order — and
//!   then the run itself, so a reader knows what the bytes are without decoding them.
//! - **Every refusal is named.** A bad magic, an unknown version, a truncated payload, an
//!   unknown tag, bytes that are not utf-8, a boolean that is not a boolean, a field map
//!   that holds one name twice, a layout that cannot be addressed, a run that is not a whole
//!   number of strides and bytes left over after the document are each an error of their
//!   own, and each says what it found.
//! - **The round trip is exact.** [`decode`] of [`encode`] is the value that was encoded,
//!   bit for bit: an empty byte sequence stays empty, an empty dictionary stays empty, a
//!   packed list keeps its layout and its bytes, and a `NaN` and a negative zero come back
//!   as themselves.
//!
//! # Version history
//!
//! - **Version 1** — the first form: the magic, the version, the eleven tags, a field map
//!   in sorted order, a layout in layout order, and a run of raw bytes. Nothing has been
//!   added to it and nothing has been taken from it.
//!
//! A later version is a new number at the head of the file, and a reader that does not
//! know that number refuses the file by name rather than reading it as something it is not.

use thiserror::Error;

use crate::packed::{PackedLayout, PackedList};
use crate::value::{FieldMap, Value};

/// The eight bytes every whiting document starts with.
pub const MAGIC: [u8; 8] = *b"WHITDOC\0";

/// The version of the format this crate writes and reads.
pub const VERSION: u64 = 1;

/// Returns whether these bytes begin a whiting document.
///
/// # Invariants
///
/// **Only the magic decides.** A reader that chose by the extension would read a file that
/// was renamed as something it is not, and a file that was truncated to nothing would be
/// read as a document rather than refused.
pub fn has_magic(head: &[u8]) -> bool {
    head.starts_with(&MAGIC)
}

/// Writes a value as the bytes a document carries it in.
///
/// # Invariants
///
/// The same value is written as the same bytes on every run and in every process, so two
/// documents can be compared by their bytes rather than by a walk of their trees.
pub fn encode(value: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    write_value(&mut out, value);
    out
}

/// Reads the value a document carries.
///
/// # Invariants
///
/// The bytes read are exactly the bytes [`encode`] writes for the same value: a float is
/// read as its bits, so a `NaN` payload and a negative zero survive, and a run is read as
/// the bytes it is rather than as a spelling of them.
pub fn decode(bytes: &[u8]) -> Result<Value, BinaryError> {
    let mut reader = Reader::new(bytes);
    reader.expect_magic()?;
    reader.expect_version()?;
    let value = reader.value()?;
    reader.expect_end()?;
    Ok(value)
}

/// The tag in front of each kind of value.
mod tag {
    /// A value that is nothing.
    pub const UNIT: u8 = 0x01;
    /// A value that is true or false.
    pub const BOOL: u8 = 0x02;
    /// A value that is a whole number.
    pub const INT: u8 = 0x03;
    /// A value that is a number with a fraction.
    pub const FLOAT: u8 = 0x04;
    /// A value that is text.
    pub const STRING: u8 = 0x05;
    /// A value that is a byte sequence.
    pub const BYTES: u8 = 0x06;
    /// A value that is named fields.
    pub const STRUCT: u8 = 0x07;
    /// A value that is a variant and the fields that variant carries.
    pub const ENUM: u8 = 0x08;
    /// A value that is an ordered sequence.
    pub const LIST: u8 = 0x09;
    /// A value that is a list carried as a run of bytes.
    pub const PACKED: u8 = 0x0A;
    /// A value that is an open map of string keys.
    pub const DICT: u8 = 0x0B;
}

/// Writes one value and what follows it.
fn write_value(out: &mut Vec<u8>, value: &Value) {
    match value {
        Value::Unit => out.push(tag::UNIT),
        Value::Bool(flag) => {
            out.push(tag::BOOL);
            out.push(u8::from(*flag));
        }
        Value::Int(number) => {
            out.push(tag::INT);
            out.extend_from_slice(&number.to_le_bytes());
        }
        Value::Float(number) => {
            out.push(tag::FLOAT);
            out.extend_from_slice(&number.to_bits().to_le_bytes());
        }
        Value::String(text) => {
            out.push(tag::STRING);
            write_string(out, text);
        }
        Value::Bytes(bytes) => {
            out.push(tag::BYTES);
            write_bytes(out, bytes);
        }
        Value::Struct(fields) => {
            out.push(tag::STRUCT);
            write_fields(out, fields);
        }
        Value::Enum { tag: name, fields } => {
            out.push(tag::ENUM);
            write_string(out, name);
            write_fields(out, fields);
        }
        Value::List(values) => {
            out.push(tag::LIST);
            out.extend_from_slice(&(values.len() as u64).to_le_bytes());
            for value in values {
                write_value(out, value);
            }
        }
        Value::Packed(list) => {
            out.push(tag::PACKED);
            write_packed(out, list);
        }
        Value::Dict(fields) => {
            out.push(tag::DICT);
            write_fields(out, fields);
        }
    }
}

/// Writes a field map in the map's own order, which is sorted by name.
fn write_fields(out: &mut Vec<u8>, fields: &FieldMap) {
    out.extend_from_slice(&(fields.len() as u64).to_le_bytes());
    for (name, value) in fields {
        write_string(out, name);
        write_value(out, value);
    }
}

/// Writes a packed list's layout and then its run.
fn write_packed(out: &mut Vec<u8>, list: &PackedList) {
    let fields = list.layout().fields();
    out.extend_from_slice(&(fields.len() as u64).to_le_bytes());
    for (name, components) in fields {
        write_string(out, name);
        out.extend_from_slice(&(*components as u64).to_le_bytes());
    }
    out.extend_from_slice(&(list.bytes().len() as u64).to_le_bytes());
    out.extend_from_slice(list.bytes());
}

/// Writes text as its length and its utf-8 bytes.
fn write_string(out: &mut Vec<u8>, text: &str) {
    out.extend_from_slice(&(text.len() as u64).to_le_bytes());
    out.extend_from_slice(text.as_bytes());
}

/// Writes bytes as their length and themselves.
fn write_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(bytes);
}

/// A cursor over the bytes of a document.
struct Reader<'a> {
    /// The bytes of the document.
    bytes: &'a [u8],
    /// How far into them the reader is.
    at: usize,
}

impl<'a> Reader<'a> {
    /// Starts a reader at the head of a document.
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    /// Returns how many bytes are left to read.
    fn remaining(&self) -> usize {
        self.bytes.len() - self.at
    }

    /// Returns the capacity a run of `count` elements is given, which is bounded by what is
    /// left to read.
    fn capacity(&self, count: u64) -> usize {
        usize::try_from(count)
            .unwrap_or(usize::MAX)
            .min(self.remaining())
    }

    /// Takes the next `count` bytes.
    ///
    /// # Invariants
    ///
    /// A count that reaches past the end of the document is refused rather than read, so
    /// the reader never reads a byte that is not there.
    fn take(&mut self, count: u64) -> Result<&'a [u8], BinaryError> {
        let end = usize::try_from(count)
            .ok()
            .and_then(|count| self.at.checked_add(count))
            .filter(|end| *end <= self.bytes.len());
        match end {
            Some(end) => {
                let slice = &self.bytes[self.at..end];
                self.at = end;
                Ok(slice)
            }
            None => Err(BinaryError::Truncated {
                at: self.at,
                wanted: count,
            }),
        }
    }

    /// Reads one byte.
    fn byte(&mut self) -> Result<u8, BinaryError> {
        Ok(self.take(1)?[0])
    }

    /// Reads one number.
    fn number(&mut self) -> Result<u64, BinaryError> {
        let bytes = self.take(8)?;
        Ok(u64::from_le_bytes(
            bytes.try_into().expect("eight bytes were taken"),
        ))
    }

    /// Reads a count of elements.
    ///
    /// # Invariants
    ///
    /// A count is refused when it is larger than what is left to read, because every
    /// element takes at least one byte. A file that claims a hundred million elements is
    /// therefore refused rather than allocated.
    fn count(&mut self) -> Result<u64, BinaryError> {
        let count = self.number()?;
        if count > self.remaining() as u64 {
            return Err(BinaryError::Truncated {
                at: self.at,
                wanted: count,
            });
        }
        Ok(count)
    }

    /// Reads a length and the bytes it counts.
    fn raw(&mut self) -> Result<&'a [u8], BinaryError> {
        let length = self.number()?;
        self.take(length)
    }

    /// Reads text as a length and the utf-8 bytes it counts.
    fn string(&mut self) -> Result<String, BinaryError> {
        let length = self.number()?;
        let at = self.at;
        let bytes = self.take(length)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| BinaryError::InvalidUtf8 { at })
    }

    /// Checks the magic, which is the head of every document of this format.
    fn expect_magic(&mut self) -> Result<(), BinaryError> {
        let found = self.bytes.get(..MAGIC.len()).unwrap_or(self.bytes);
        if found == MAGIC {
            self.at = MAGIC.len();
            Ok(())
        } else {
            Err(BinaryError::BadMagic {
                expected: MAGIC,
                found: found.to_vec(),
            })
        }
    }

    /// Checks the version, which is the number after the magic.
    fn expect_version(&mut self) -> Result<(), BinaryError> {
        let found = self.number()?;
        if found == VERSION {
            Ok(())
        } else {
            Err(BinaryError::UnknownVersion {
                found,
                supported: VERSION,
            })
        }
    }

    /// Checks that the document ends where the reader does.
    fn expect_end(&self) -> Result<(), BinaryError> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(BinaryError::TrailingBytes {
                at: self.at,
                found: self.bytes.len() - self.at,
            })
        }
    }

    /// Reads one value and what follows it.
    fn value(&mut self) -> Result<Value, BinaryError> {
        let at = self.at;
        let tag = self.byte()?;
        match tag {
            tag::UNIT => Ok(Value::Unit),
            tag::BOOL => {
                let flag = self.byte()?;
                match flag {
                    0 => Ok(Value::Bool(false)),
                    1 => Ok(Value::Bool(true)),
                    found => Err(BinaryError::NotABoolean { at, found }),
                }
            }
            tag::INT => Ok(Value::Int(self.number()? as i64)),
            tag::FLOAT => Ok(Value::Float(f64::from_bits(self.number()?))),
            tag::STRING => Ok(Value::String(self.string()?)),
            tag::BYTES => Ok(Value::Bytes(self.raw()?.to_vec())),
            tag::STRUCT => Ok(Value::Struct(self.fields()?)),
            tag::ENUM => {
                let name = self.string()?;
                Ok(Value::Enum {
                    tag: name,
                    fields: self.fields()?,
                })
            }
            tag::LIST => {
                let count = self.count()?;
                let mut values = Vec::with_capacity(self.capacity(count));
                for _ in 0..count {
                    values.push(self.value()?);
                }
                Ok(Value::List(values))
            }
            tag::PACKED => Ok(Value::Packed(self.packed()?)),
            tag::DICT => Ok(Value::Dict(self.fields()?)),
            found => Err(BinaryError::UnknownTag { at, tag: found }),
        }
    }

    /// Reads a field map.
    ///
    /// # Invariants
    ///
    /// A name that appears twice is refused, because one name is one field: a map that
    /// held two values under one name would be read as the last of them, which is a
    /// document different from the one that was written.
    fn fields(&mut self) -> Result<FieldMap, BinaryError> {
        let count = self.count()?;
        let mut fields = FieldMap::new();
        for _ in 0..count {
            let name = self.string()?;
            let value = self.value()?;
            if fields.insert(name.clone(), value).is_some() {
                return Err(BinaryError::DuplicateField { name });
            }
        }
        Ok(fields)
    }

    /// Reads a packed list's layout and then its run.
    fn packed(&mut self) -> Result<PackedList, BinaryError> {
        let count = self.count()?;
        let mut fields: Vec<(String, u64)> = Vec::with_capacity(self.capacity(count));
        let mut components = 0u64;
        for _ in 0..count {
            let name = self.string()?;
            let width = self.number()?;
            components = components
                .checked_add(width)
                .ok_or(BinaryError::StrideTooLarge { found: u64::MAX })?;
            fields.push((name, width));
        }
        let stride = components
            .checked_mul(4)
            .ok_or(BinaryError::StrideTooLarge { found: components })?;
        let stride = usize::try_from(stride)
            .map_err(|_| BinaryError::StrideTooLarge { found: components })?;
        if stride == 0 {
            return Err(BinaryError::EmptyLayout);
        }
        let bytes = self.raw()?;
        if bytes.len() % stride != 0 {
            return Err(BinaryError::NotAWholeNumberOfStrides {
                stride,
                found: bytes.len(),
            });
        }
        let layout = PackedLayout::new(
            fields
                .into_iter()
                .map(|(name, width)| (name, width as usize)),
        );
        Ok(PackedList::new(layout, bytes.to_vec())
            .expect("the stride and the length are checked above"))
    }
}

/// A document could not be read as this format.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BinaryError {
    #[error("this is not a whiting document: the magic is {found:?}, and it must be {expected:?}")]
    BadMagic {
        /// The magic every document of this format starts with.
        expected: [u8; 8],
        /// The bytes that were found at the head instead.
        found: Vec<u8>,
    },
    #[error("the format version is {found}, and this crate reads version {supported}")]
    UnknownVersion {
        /// The version the file carries.
        found: u64,
        /// The version this crate reads.
        supported: u64,
    },
    #[error("the payload ends at {at}, and {wanted} more bytes were needed")]
    Truncated {
        /// Where the payload ended.
        at: usize,
        /// How many more bytes were needed.
        wanted: u64,
    },
    #[error("at {at} the tag is {tag:#04x}, and it is not a tag this format defines")]
    UnknownTag {
        /// Where the tag was found.
        at: usize,
        /// The tag that was found.
        tag: u8,
    },
    #[error("at {at} the bytes are not valid utf-8")]
    InvalidUtf8 {
        /// Where the text was found.
        at: usize,
    },
    #[error("at {at} the boolean is {found}, and it must be 0 or 1")]
    NotABoolean {
        /// Where the boolean was found.
        at: usize,
        /// The byte that was found.
        found: u8,
    },
    #[error("a field map holds the field `{name}` twice, and one name is one field")]
    DuplicateField {
        /// The name that appears twice.
        name: String,
    },
    #[error("the layout declares {found} components, which is more than this platform can address")]
    StrideTooLarge {
        /// The number of components the layout declares.
        found: u64,
    },
    #[error("the layout has no components, so it cannot carry a list")]
    EmptyLayout,
    #[error(
        "the run is {found} bytes, and the layout's stride is {stride}, so it is not a whole number of strides"
    )]
    NotAWholeNumberOfStrides {
        /// The stride the layout declares.
        stride: usize,
        /// The length of the run.
        found: usize,
    },
    #[error("the document ends at {at}, and {found} bytes are left over")]
    TrailingBytes {
        /// Where the document ended.
        at: usize,
        /// How many bytes are left over.
        found: usize,
    },
}
