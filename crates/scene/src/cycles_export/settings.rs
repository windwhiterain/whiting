//! Film and integrator: the settings that make two renders comparable.

use super::EmitOptions;
use super::writer::Writer;

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
pub fn write_film(writer: &mut Writer, options: &EmitOptions) {
    writer.comment("Film");
    writer.leaf("film", &[("exposure", options.exposure.to_string())]);
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
