//! What the scene crate's subjects share: the helpers that more than one of them uses.
//!
//! A helper that one subject uses stays inside that subject, private to it. A helper that two
//! or more use is the same helper, so it is written once here and the subjects that use it
//! name it from here. Where two subjects had a helper of the same name and the two were
//! different helpers, each keeps its own body and takes a name of its own where it lives.

use definition::{Path, Value};
use graphics::{LABEL, PASSES_FIELD};
use whiting_scene::CompileOutput;
use whiting_scene::standard::{Bounds, Camera, Projection};

/// The instance entry holding the record of every light.
pub const LIGHT_RECORDS: &str = "light_records";

/// The instance entry holding the cone of every light.
pub const SPOT_CONES: &str = "spot_cones";

/// A light record is seven four byte blocks: a colour and a page, a position and a radius, a four
/// by four projection, and the address of the light's virtual shadow pages.
///
/// It is the size the cone run's existence is protecting. A directional light is a **stated
/// input of the pass** and not a record, so this is the size it still costs.
pub const RECORD_BYTES: i64 = 112;

/// A cone record is two four component vectors, because a `WGSL` `array<T>` in a storage buffer
/// lays its elements out at `roundUp(align_of_T, size_of_T)` and a struct holding a vector is
/// aligned to sixteen bytes.
pub const CONE_BYTES: i64 = 32;

/// Builds a three component list of floats.
pub fn triple(components: [f64; 3]) -> Value {
    Value::List(components.into_iter().map(Value::Float).collect())
}

/// Returns the identity matrix, which is the projection a light no shadow is looked up
/// through carries.
///
/// The assignment does not read it and the packing only writes it, so a test that is about
/// which lights reach which clusters has nothing to say about a projection — but the record
/// has to carry one, and a light with no page is never looked up through it.
///
/// Nothing in the pool reads it either — a page is a page and a light either has one or does
/// not — so what a projection holds is of no interest to which lights are given a page.
pub fn identity() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
}

/// Returns the box the cameras frame, whose half width the tests read their light positions off.
pub fn bounds() -> Bounds {
    Bounds::around([0.0, 0.0, 0.0], 4.0)
}

/// Returns the parallel camera the convention gives a box of half width four.
///
/// Its eye stands at z twelve looking at the origin, its frame is eight and a bit wide each
/// way, and its depth range is five to twenty — so a world position is a distance from the eye
/// by subtracting its z from twelve.
pub fn parallel() -> Projection {
    Camera::read(None, &Path::root(), &bounds(), 1.0)
        .expect("the convention camera reads")
        .projection()
}

/// Returns the scene's one lighting pass.
pub fn lighting_pass(compiled: &CompileOutput) -> Value {
    compiled
        .graphics
        .value
        .as_field_map()
        .and_then(|fields| fields.get(PASSES_FIELD))
        .and_then(Value::as_list)
        .expect("the graphics document holds passes")
        .iter()
        .find(|pass| {
            pass.variant_fields()
                .and_then(|fields| fields.get(LABEL))
                .and_then(Value::as_str)
                .is_some_and(|label| label.starts_with("lighting/"))
        })
        .expect("the example scene has one lighting pass")
        .clone()
}
