//! Film and integrator: the settings that make two renders comparable.

use super::EmitOptions;
use super::writer::Writer;
use crate::standard::EXPOSURE;
use definition::Value;

/// Returns the linear exposure multiplier a document asked for, or `fallback` when it asked for
/// none.
///
/// # Invariants
///
/// **The document's own field wins, and the caller's number is the default for a scene that
/// names none.** They are the same quantity — `scene_schema::EXPOSURE` documents a linear
/// multiplier on the radiance the post stage reads, and [`EmitOptions::exposure`] documents a
/// linear multiplier on the radiance Cycles writes — so there is nothing to convert and nothing
/// to choose between. Taking the caller's number first would be the same mistake the camera
/// made: a field the emitter never read, which a scene could add and see nothing happen.
///
/// A field that is present but is not a number is **ignored rather than refused**, because a
/// scene that cannot express the field at all is exactly a scene that leaves it out, and this
/// backend has never refused a document over a malformed float anywhere else.
pub fn exposure(scene: &Value, fallback: f32) -> f32 {
    scene
        .as_field_map()
        .and_then(|fields| fields.get(EXPOSURE))
        .and_then(Value::as_float)
        .map(|value| value as f32)
        .unwrap_or(fallback)
}

/// Writes the `<film>` element.
///
/// Only `exposure` is written. The film also has a filter type, a filter width and a
/// display pass, and leaving them at their defaults is deliberate: a comparison render wants
/// the same reconstruction filter on both sides, and the default is whatever the renderer
/// ships, so writing nothing is what makes the two sides agree.
///
/// # Units
///
/// `exposure` is a linear multiplier here and in Cycles alike, which was measured rather
/// than assumed. See [`EmitOptions::exposure`].
pub fn write_film(writer: &mut Writer, exposure: f32) {
    writer.comment("Film");
    writer.leaf("film", &[("exposure", exposure.to_string())]);
}

/// Writes the `<integrator>` element.
///
/// The `seed` and `use_denoise` settings are what make a render reproducible and a
/// comparison honest, and they are written from the options rather than left to the renderer's
/// defaults, which differ between builds.
pub fn write_integrator(writer: &mut Writer, options: &EmitOptions) {
    writer.comment("Integrator");
    writer.leaf(
        "integrator",
        &[
            ("seed", options.seed.to_string()),
            (
                "use_denoise",
                if options.denoise { "true" } else { "false" }.to_owned(),
            ),
            ("max_bounce", options.max_bounce.to_string()),
        ],
    );
}
