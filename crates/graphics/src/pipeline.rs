//! How a rasterize pass is built: what it blends, how it uses depth, and how finely it samples.
//!
//! # Why this is data rather than a constant of the renderer
//!
//! These three were fixed in the backend, which meant a document could not express an
//! additive pass, a pass that draws over the depth it is drawn over, or an antialiased
//! edge. Each of those is a statement about how a frame is put together, so each belongs
//! on the pass that makes it rather than in the code that runs it.
//!
//! # Invariants
//!
//! - **A field that is absent is the behaviour the backend always had.** A document that
//!   names nothing builds the pipeline it built before any of this was a field, so every
//!   existing pass renders as it did. The defaults are therefore not a policy: they are
//!   what `Replace`, depth written and `Less`, and one sample mean.
//! - **Depth and blend are independent.** A pass may write depth without testing it, test
//!   without writing, or do neither. A fullscreen pass that samples the depth buffer it is
//!   drawn over needs `Always` and no write, and cannot be expressed as a variation of
//!   either half.

use definition::Value;

use crate::GraphicsError;
use crate::pass::{BLEND, DEPTH, DEPTH_COMPARE, DEPTH_WRITE, SAMPLES};
use crate::read::PassView;

/// How what a pass draws is combined with what is already in its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blend {
    /// Overwrite what was there.
    Replace,
    /// Mix by the fragment's own alpha.
    Alpha,
    /// Add the fragment's colour to what was there.
    Add,
}

impl Blend {
    /// Returns the tag this blend is stored under in a pass document.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Replace => "Replace",
            Self::Alpha => "Alpha",
            Self::Add => "Add",
        }
    }
}

/// How a fragment's depth is compared against what the depth buffer holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compare {
    /// Every fragment is discarded.
    Never,
    /// The nearer fragment wins, which is what an opaque pass wants.
    Less,
    /// The nearer fragment wins, and a tie is kept.
    LessEqual,
    /// Only a fragment at exactly the recorded depth survives.
    Equal,
    /// The further fragment wins.
    Greater,
    /// The further fragment wins, and a tie is kept.
    GreaterEqual,
    /// Every fragment survives.
    Always,
}

/// Everything about a rasterize pass that decides how its pixels are produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipelineState {
    /// How what the pass draws is combined with what is in its target.
    pub blend: Blend,
    /// Whether the pass records the depth of what it draws.
    pub depth_write: bool,
    /// Whether a fragment survives on how its depth compares against the buffer's.
    pub depth_compare: Compare,
    /// How many samples each pixel takes.
    pub samples: u32,
}

impl Default for PipelineState {
    /// Returns what the backend built before any of this was a field.
    fn default() -> Self {
        Self {
            blend: Blend::Replace,
            depth_write: true,
            depth_compare: Compare::Less,
            samples: 1,
        }
    }
}

/// The largest sample count a pass may ask for, which is what a backend resolves into one
/// texture per pixel.
pub const MAX_SAMPLES: i64 = 4;

impl PipelineState {
    /// Reads the pipeline state of a pass, filling in what it does not name.
    pub fn read(pass: &PassView<'_>) -> Result<Self, GraphicsError> {
        let path = pass.path();
        let defaults = Self::default();

        let blend = match pass.optional(BLEND) {
            None => defaults.blend,
            Some(value) => match value.variant_tag() {
                Some("Replace") => Blend::Replace,
                Some("Alpha") => Blend::Alpha,
                Some("Add") => Blend::Add,
                Some(other) => {
                    return Err(GraphicsError::Malformed {
                        path: path.key(BLEND),
                        field: BLEND.to_owned(),
                        expected: "Replace, Alpha or Add",
                        found: other.to_owned(),
                    });
                }
                None => {
                    return Err(GraphicsError::Malformed {
                        path: path.key(BLEND),
                        field: BLEND.to_owned(),
                        expected: "a blend",
                        found: value.kind_name().to_owned(),
                    });
                }
            },
        };

        let Some(depth) = pass.optional(DEPTH) else {
            return Ok(Self { blend, ..defaults });
        };
        let field = |name: &str| depth.as_field_map().and_then(|fields| fields.get(name));
        let depth_write = field(DEPTH_WRITE)
            .and_then(Value::as_bool)
            .unwrap_or(defaults.depth_write);
        let depth_compare = match field(DEPTH_COMPARE).and_then(Value::variant_tag) {
            None => defaults.depth_compare,
            Some("Never") => Compare::Never,
            Some("Less") => Compare::Less,
            Some("LessEqual") => Compare::LessEqual,
            Some("Equal") => Compare::Equal,
            Some("Greater") => Compare::Greater,
            Some("GreaterEqual") => Compare::GreaterEqual,
            Some("Always") => Compare::Always,
            Some(other) => {
                return Err(GraphicsError::Malformed {
                    path: path.key(DEPTH).key(DEPTH_COMPARE),
                    field: DEPTH_COMPARE.to_owned(),
                    expected: "a depth comparison",
                    found: other.to_owned(),
                });
            }
        };

        let declared_samples = pass.optional(SAMPLES).and_then(Value::as_int);
        let samples = match declared_samples {
            None => defaults.samples,
            // A sample count below one is a document that means "one" and wrote it oddly,
            // and one above what a backend resolves is a document asking for something no
            // backend will build. Both are reported rather than quietly rounded.
            Some(found) if !(1..=MAX_SAMPLES).contains(&found) => {
                return Err(GraphicsError::Malformed {
                    path: path.key(SAMPLES),
                    field: SAMPLES.to_owned(),
                    expected: "a sample count between one and four",
                    found: found.to_string(),
                });
            }
            Some(found) => found as u32,
        };

        Ok(Self {
            blend,
            depth_write,
            depth_compare,
            samples,
        })
    }
}
