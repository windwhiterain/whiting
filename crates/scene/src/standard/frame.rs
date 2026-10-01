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

use definition::{Path, Value};

use crate::standard::scene_schema::FRAME;

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
    /// Returns the frame a scene is drawn into, which is the default unless it says.
    pub fn read(scene: &Value) -> Self {
        let declared = scene
            .as_field_map()
            .and_then(|fields| fields.get(FRAME))
            .and_then(Value::as_field_map);
        let read = |name: &str, fallback: i64| {
            declared
                .and_then(|fields| fields.get(name))
                .and_then(Value::as_int)
                .filter(|size| *size > 0)
                .unwrap_or(fallback)
        };
        Self {
            width: read("width", DEFAULT_WIDTH),
            height: read("height", DEFAULT_HEIGHT),
        }
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
