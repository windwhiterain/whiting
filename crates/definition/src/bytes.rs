//! How a byte sequence is written in a document.
//!
//! # Why not as numbers
//!
//! `facet-json` writes a `Vec<u8>` as a json array of decimal octets, so a byte costs about
//! 2.6 text bytes and a number to parse. Measured on a run of 20,000 points at a 24 byte
//! stride, which is 480,000 bytes of run: 1,262,194 bytes of text, 30 ms to write and 608 ms
//! to read back. A construction that is a `memcmp` and a `len % stride` check in memory
//! therefore costs a walk of half a million number tokens on every trip through json, which is
//! what makes a mesh impossible to hand to a device.
//!
//! # Why base64
//!
//! Base64 carries three bytes in four characters, so the same run is 640,073 text bytes rather
//! than 1,262,194, and reading it is one pass over the text rather than one parse per octet: 14
//! ms to write and 33 ms to read. Hex, which `facet-json` can write but not read, would be
//! 960,000; it is not used, because a byte in two characters is only worth reading by eye and
//! no document of this size is.
//!
//! The measurement is `crates/device/examples/bytes_format_probe.rs`, which reports the same
//! data in the spelling this replaced as well as in this one.
//!
//! Base64 is implemented here rather than taken from a crate because this crate depends on
//! nothing but `facet` and `thiserror`, and the codec is small enough to be read.
//!
//! # Invariants
//!
//! [`encode_bytes`] and [`decode_bytes`] are inverses on every byte sequence, and
//! `decode_bytes` accepts exactly the spelling `encode_bytes` writes: the standard alphabet,
//! `=` padding to a multiple of four, and no bits set in the padding. A document that spells
//! a byte sequence in any other way is refused rather than read as different bytes.

use facet::Facet;

/// The standard base64 alphabet, in value order.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The character that carries no bits, at the end of a spelling that is not a whole number of
/// four character groups.
const PADDING: char = '=';

/// Writes a byte sequence as the base64 text a document spells it with.
pub fn encode_bytes(bytes: &[u8]) -> String {
    // Three bytes become four characters, and a trailing group of one or two is padded, so the
    // text is a whole number of the four character groups a reader takes.
    let mut text = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for group in bytes.chunks(3) {
        let first = group[0];
        let second = group.get(1).copied().unwrap_or(0);
        let third = group.get(2).copied().unwrap_or(0);
        text.push(ALPHABET[(first >> 2) as usize] as char);
        text.push(ALPHABET[(((first & 0b11) << 4) | (second >> 4)) as usize] as char);
        text.push(if group.len() > 1 {
            ALPHABET[(((second & 0b1111) << 2) | (third >> 6)) as usize] as char
        } else {
            PADDING
        });
        text.push(if group.len() > 2 {
            ALPHABET[(third & 0b111111) as usize] as char
        } else {
            PADDING
        });
    }
    text
}

/// Reads the byte sequence a base64 text carries.
///
/// # Invariants
///
/// Only the spelling [`encode_bytes`] writes is accepted; anything else is an error naming
/// what is wrong, so a malformed document cannot be read as bytes that are merely different.
pub fn decode_bytes(text: &str) -> Result<Vec<u8>, BytesError> {
    let characters: Vec<char> = text.chars().collect();
    if characters.len() % 4 != 0 {
        return Err(BytesError::NotAWholeNumber {
            found: characters.len(),
        });
    }
    let groups = characters.len() / 4;
    let mut bytes = Vec::with_capacity(groups * 3);
    for (index, group) in characters.chunks(4).enumerate() {
        let last = index + 1 == groups;
        // Padding is only ever at the end, and only ever fills the last two characters of the
        // final group, because that is the only place a partial group can be.
        let filled = group
            .iter()
            .filter(|character| **character == PADDING)
            .count();
        if filled > 0 && (!last || filled > 2) {
            return Err(BytesError::MisplacedPadding { at: index * 4 });
        }
        if filled > 0 && group[4 - filled..].iter().any(|c| *c != PADDING) {
            return Err(BytesError::MisplacedPadding { at: index * 4 });
        }
        let mut values = [0u8; 4];
        for (offset, character) in group.iter().enumerate() {
            if *character == PADDING {
                continue;
            }
            values[offset] = value_of(*character).ok_or(BytesError::NotABase64Character {
                character: *character,
                at: index * 4 + offset,
            })?;
        }
        // The bits a padded group leaves over must be zero, so that every byte sequence has one
        // spelling rather than one per way of filling the tail of the last group.
        if filled == 1 && values[2] & 0b11 != 0 {
            return Err(BytesError::UnusedBits { at: index * 4 });
        }
        if filled == 2 && values[1] & 0b1111 != 0 {
            return Err(BytesError::UnusedBits { at: index * 4 });
        }

        bytes.push((values[0] << 2) | (values[1] >> 4));
        if filled < 2 {
            bytes.push((values[1] << 4) | (values[2] >> 2));
        }
        if filled < 1 {
            bytes.push((values[2] << 6) | values[3]);
        }
    }
    Ok(bytes)
}

/// Returns the six bit value a base64 character carries, or `None` when it is not one.
fn value_of(character: char) -> Option<u8> {
    match character {
        'A'..='Z' => Some(character as u8 - b'A'),
        'a'..='z' => Some(character as u8 - b'a' + 26),
        '0'..='9' => Some(character as u8 - b'0' + 52),
        '+' => Some(62),
        '/' => Some(63),
        _ => None,
    }
}

/// A byte sequence cannot be read from the text a document spells it with.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BytesError {
    #[error(
        "a base64 spelling is a whole number of four character groups, and this is {found} characters"
    )]
    NotAWholeNumber { found: usize },
    #[error("`{character}` at {at} is not a base64 character")]
    NotABase64Character { character: char, at: usize },
    #[error(
        "the group at {at} has padding where only the last two characters of the last group may have it"
    )]
    MisplacedPadding { at: usize },
    #[error("the group at {at} has bits set that its padding leaves no room for")]
    UnusedBits { at: usize },
}

/// A byte sequence as a document spells it.
///
/// This is what [`Value::Bytes`](crate::Value::Bytes) and the run of a
/// [`PackedList`](crate::PackedList) are written and read through, so a document carries a byte
/// sequence as one text field rather than as a decimal number per octet. It is `transparent`,
/// so the spelling is the text itself and not a structure around it.
///
/// # Invariants
///
/// The text is a whole number of four character groups, and every group but a padded tail
/// carries six bits of a byte; [`BytesText::bytes`] is therefore the inverse of
/// [`BytesText::of`] for every byte sequence.
///
/// # Example
///
/// ```
/// use whiting_definition::bytes::{decode_bytes, encode_bytes, BytesText};
///
/// let bytes = vec![0, 1, 127, 255];
/// let text = BytesText::of(&bytes);
/// assert_eq!(text.as_str(), "AAF//w==");
/// assert_eq!(text.bytes().expect("the spelling is one this crate wrote"), bytes);
///
/// // And the two functions are the pair the spelling is built from.
/// assert_eq!(decode_bytes(&encode_bytes(&bytes)).expect("it is canonical"), bytes);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Facet)]
#[facet(transparent)]
pub struct BytesText(String);

impl BytesText {
    /// Writes a byte sequence as the text a document spells it with.
    pub fn of(bytes: &[u8]) -> Self {
        Self(encode_bytes(bytes))
    }

    /// Borrows the text as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Reads the byte sequence the text carries.
    pub fn bytes(&self) -> Result<Vec<u8>, BytesError> {
        decode_bytes(&self.0)
    }
}

/// Spells the bytes of a value that holds them as a run, which is what a `proxy` conversion is
/// asked for and cannot fail.
impl From<&Vec<u8>> for BytesText {
    fn from(bytes: &Vec<u8>) -> Self {
        Self::of(bytes)
    }
}

/// Reads back the bytes a spelling carries, refusing a text that is not this crate's spelling.
impl TryFrom<BytesText> for Vec<u8> {
    type Error = BytesError;

    fn try_from(text: BytesText) -> Result<Self, Self::Error> {
        text.bytes()
    }
}
