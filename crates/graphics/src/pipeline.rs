//! How a rasterize pass is built: what it blends, how it uses depth, how finely it samples,
//! and which part of its target it draws into.
//!
//! # Why this is data rather than a constant of the renderer
//!
//! These were fixed in the backend, which meant a document could not express an
//! additive pass, a pass that draws over the depth it is drawn over, an antialiased
//! edge, or a pass that fills one tile of a shared target. Each of those is a statement
//! about how a frame is put together, so each belongs
//! on the pass that makes it rather than in the code that runs it.
//!
//! # Invariants
//!
//! - **A field that is absent is the behaviour the backend always had.** A document that
//!   names nothing builds the pipeline it built before any of this was a field, so every
//!   existing pass renders as it did. The defaults are therefore not a policy: they are
//!   what `Replace`, depth written and `Less`, one sample, and the whole target mean.
//! - **Depth and blend are independent.** A pass may write depth without testing it, test
//!   without writing, or do neither. A fullscreen pass that samples the depth buffer it is
//!   drawn over needs `Always` and no write, and cannot be expressed as a variation of
//!   either half.

use definition::{Path, Value};

use crate::GraphicsError;
use crate::pass::{BLEND, DEPTH, DEPTH_COMPARE, DEPTH_WRITE, SAMPLES, VIEWPORT};
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

/// Which part of its targets a rasterize pass draws into, in texels of the target.
///
/// # Invariants
///
/// The origin is measured from the top left of the target, which is where a backend measures a
/// viewport and a scissor rect from, so the rectangle a document names is the one the pass is
/// clipped to without a conversion between the two that could be wrong in one direction only.
///
/// A width or a height of zero is refused rather than read as "draw nothing": a pass that covers
/// no texels is a document that miscounted the region it meant to name, and a render that quietly
/// produced nothing would leave whatever was already in the target standing as the answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    /// How far from the left edge of the target the region starts, in texels.
    pub x: u32,
    /// How far from the top edge it starts, in texels.
    pub y: u32,
    /// How wide it is, in texels.
    pub width: u32,
    /// How tall it is, in texels.
    pub height: u32,
}

/// The side a viewport is measured along, for a message about one that has no length.
const SIDES: [&str; 2] = ["width", "height"];

impl Viewport {
    /// Reads a viewport out of the `[ x, y, width, height ]` a pass declares.
    fn read(declared: &Value, path: &Path) -> Result<Self, GraphicsError> {
        let Some(edges) = declared.as_list() else {
            return Err(GraphicsError::Malformed {
                path: path.clone(),
                field: VIEWPORT.to_owned(),
                expected: "a list of four integers",
                found: declared.kind_name().to_owned(),
            });
        };
        let [x, y, width, height] = edges else {
            return Err(GraphicsError::Malformed {
                path: path.clone(),
                field: VIEWPORT.to_owned(),
                expected: "four integers: origin x, origin y, width and height",
                found: format!("{} numbers", edges.len()),
            });
        };
        let edge = |value: &Value, at: usize| -> Result<u32, GraphicsError> {
            let number = value.as_int().ok_or_else(|| GraphicsError::Malformed {
                path: path.index(at),
                field: VIEWPORT.to_owned(),
                expected: "a texel count",
                found: value.kind_name().to_owned(),
            })?;
            u32::try_from(number).map_err(|_| GraphicsError::Malformed {
                path: path.index(at),
                field: VIEWPORT.to_owned(),
                expected: "a texel offset or count",
                found: number.to_string(),
            })
        };
        let out = Self {
            x: edge(x, 0)?,
            y: edge(y, 1)?,
            width: edge(width, 2)?,
            height: edge(height, 3)?,
        };
        for (side, length) in SIDES.iter().zip([out.width, out.height]) {
            if length == 0 {
                return Err(GraphicsError::Malformed {
                    path: path.clone(),
                    field: VIEWPORT.to_owned(),
                    expected: "a region with a nonzero side",
                    found: format!("{side} is zero"),
                });
            }
        }
        Ok(out)
    }
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
    /// Which region of its target the pass draws into, or `None` for the whole of it.
    pub viewport: Option<Viewport>,
}

impl Default for PipelineState {
    /// Returns what the backend built before any of this was a field.
    fn default() -> Self {
        Self {
            blend: Blend::Replace,
            depth_write: true,
            depth_compare: Compare::Less,
            samples: 1,
            viewport: None,
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

        let declared = match pass.optional(VIEWPORT) {
            None => None,
            Some(value) => Some(Viewport::read(value, &path.key(VIEWPORT))?),
        };

        let Some(depth) = pass.optional(DEPTH) else {
            return Ok(Self {
                blend,
                viewport: declared,
                ..defaults
            });
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
            viewport: declared,
        })
    }
}
