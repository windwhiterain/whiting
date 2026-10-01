//! What the machine behind a compile can hold, and how a document asking for more is answered.

use definition::Path;

use crate::CompileNote;

/// What the machine behind a compile can actually do, for the two limits a scene document can
/// ask past.
///
/// # Invariants
///
/// These are **the device's** numbers rather than the adapter's. wgpu creates a device with the
/// lower of what a request asked for and what an adapter reports, and every resource is
/// validated against the numbers the device was created with, so a compiler bounding a document
/// by anything else would be bounding it by a number no device ever agreed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceLimits {
    /// The widest or tallest a texture may be, in pixels.
    pub max_texture_dimension_2d: u32,
    /// The deepest a texture array may be, in layers.
    pub max_texture_array_layers: u32,
}

impl DeviceLimits {
    /// The floor every device agrees to, which is what a compile with no machine behind it is
    /// made against.
    ///
    /// # Invariants
    ///
    /// A host that has not asked an adapter — a compile in a unit test, a server whose renderer
    /// is a placeholder — has no machine to answer for, and the floor is then the only honest
    /// number to bound a document by: it is the smallest answer that is true everywhere. Every
    /// convention this project uses is below it, so a convention is never itself clamped.
    pub const fn downlevel() -> Self {
        Self {
            max_texture_dimension_2d: 2048,
            max_texture_array_layers: 256,
        }
    }

    /// The widest or tallest a texture may be, in pixels, as a count.
    pub const fn texture_side_ceiling(&self) -> i64 {
        self.max_texture_dimension_2d as i64
    }

    /// The deepest a texture array may be, in layers, as a count.
    pub const fn texture_layer_ceiling(&self) -> i64 {
        self.max_texture_array_layers as i64
    }
}

/// Bounds one count a document declared to what the machine can hold, reporting what it lost.
///
/// # Invariants
///
/// A count at or below the ceiling is returned untouched and says nothing: a document that asks
/// for what the machine has is not a document that needs a note. A count above it is given the
/// ceiling and gets a note naming the field, the number asked for, the number given and the
/// ceiling it hit, because **the note is the whole of the answer**. A frame drawn into a
/// smaller target than the document named, or a pool of fewer pages, is otherwise a wrong image
/// with nothing said about it — and a refusal is not the alternative, because a document
/// refused on this machine would draw on another one, and portability is the property a
/// document is supposed to have.
///
/// `consequence` is what the loss costs the frame, in the words of the thing that lost it: one
/// ceiling bounds a target, a shadow page and a page count, and what a smaller number means is
/// not the same in all three.
pub(crate) fn clamp_to_ceiling(
    declared: i64,
    ceiling: i64,
    field: &str,
    path: Path,
    consequence: &str,
    notes: &mut Vec<CompileNote>,
) -> i64 {
    if declared <= ceiling {
        return declared;
    }
    notes.push(CompileNote {
        path,
        message: format!(
            "`{field}` asks for {declared}, which is past this machine's ceiling of {ceiling}, \
             so this compile uses {ceiling}. {consequence}"
        ),
    });
    ceiling
}
