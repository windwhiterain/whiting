//! The image a scene is drawn into.
//!
//! # Why the frame belongs to the scene
//!
//! Every image the standard pipeline draws into — the shaded frame, the present target
//! and the depth buffer the lighting passes test against — is the same size, and that size
//! used to be a constant of the pipeline. It is not a fact about the pipeline: it is a fact
//! about the picture. A wide valley drawn into a square is cropped at the sides, and the
//! same valley drawn into a wide frame keeps the ridges that were outside it.
//!
//! # Invariants
//!
//! - **A scene that names no frame is drawn into the one it was always drawn into.** A
//!   field that is absent is the behaviour the pipeline always had, so a scene that says
//!   nothing about its frame draws exactly the frame it drew before a frame was a field.
//! - **The three images are the same size.** A post stage reads the shaded image and the
//!   depth that produced it and writes the frame, and a sample of one read at the
//!   coordinates of another is a frame of the wrong picture rather than a wrong picture.
//! - A dimension of zero or less is the default, because a texture a backend cannot
//!   allocate is a render that cannot happen and a compile error about a pixel count is
//!   not a useful one.
//! - A dimension **above** what the machine can make is the ceiling and a note, because a
//!   frame smaller than the document asked for is a frame that still draws. It is the same
//!   rule the shadow page pool is held to, and the same reason.

use definition::{Path, Value};

use crate::limits::clamp_to_ceiling;
use crate::standard::scene_schema::FRAME;
use crate::{CompileNote, DeviceLimits};

/// The width a scene is drawn into when it names none, in pixels.
pub const DEFAULT_WIDTH: i64 = 1024;

/// The height a scene is drawn into when it names none, in pixels.
pub const DEFAULT_HEIGHT: i64 = 1024;

/// The size of every image the standard pipeline draws a scene into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    /// How wide the shaded image, the present target and the depth buffer are.
    pub width: i64,
    /// How tall they are.
    pub height: i64,
}

impl Frame {
    /// Returns the frame a scene is drawn into, which is the default unless it says, bounded
    /// to what the machine can make.
    ///
    /// # Invariants
    ///
    /// The two directions are bounded separately and both say so, because they are two fields
    /// an author changes separately. A frame narrowed in one direction is a scene drawn into a
    /// different picture from the one it named, and that is a thing to be told about rather
    /// than a thing to fail on: the document draws on a machine that can give it the size it
    /// asked for.
    ///
    /// A convention is never bounded, because [`DeviceLimits::downlevel`] is above every
    /// convention here and a machine below it is not one this pipeline runs on.
    pub fn read(scene: &Value, limits: DeviceLimits) -> (Self, Vec<CompileNote>) {
        Self::read_declared(
            scene
                .as_field_map()
                .and_then(|fields| fields.get(FRAME))
                .and_then(Value::as_field_map),
            limits,
        )
    }

    /// Returns the frame a `frame` field declares, for a caller that already holds one.
    ///
    /// # Invariants
    ///
    /// This is the whole of [`Frame::read`] past taking the field out of the document, and it is
    /// separate so that a reader holding a scene's field map does not have to rebuild the scene
    /// to ask the same question. The frame is the **scene's** and not a camera's, so a camera
    /// that wants to know how wide its own picture is has to come here.
    pub fn read_declared(
        declared: Option<&definition::FieldMap>,
        limits: DeviceLimits,
    ) -> (Self, Vec<CompileNote>) {
        let read = |name: &str, fallback: i64| {
            declared
                .and_then(|fields| fields.get(name))
                .and_then(Value::as_int)
                .filter(|size| *size > 0)
                .unwrap_or(fallback)
        };
        let ceiling = limits.texture_side_ceiling();
        let mut notes = Vec::new();
        let width = clamp_to_ceiling(
            read("width", DEFAULT_WIDTH),
            ceiling,
            "width",
            Self::path("width"),
            "Every pass that draws into this frame is drawn into a target smaller than the \
             document named, so the picture is a crop of the one it asked for.",
            &mut notes,
        );
        let height = clamp_to_ceiling(
            read("height", DEFAULT_HEIGHT),
            ceiling,
            "height",
            Self::path("height"),
            "Every pass that draws into this frame is drawn into a target smaller than the \
             document named, so the picture is a crop of the one it asked for.",
            &mut notes,
        );
        (Self { width, height }, notes)
    }

    /// Returns the width divided by the height, which is what a camera's field of view is
    /// widened by.
    pub fn aspect(self) -> f64 {
        self.width as f64 / self.height.max(1) as f64
    }

    /// Reports the path a field of the frame is at, for a message about it.
    pub fn path(name: &str) -> Path {
        Path::root().key(FRAME).key(name)
    }
}

impl Default for Frame {
    /// Returns the square the pipeline drew into before a frame was a field.
    fn default() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
        }
    }
}
