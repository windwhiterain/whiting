//! Lights: our four light kinds as Cycles lights.
//!
//! # A light does not emit unless its shader emits
//!
//! This is the one fact about the format that is not visible anywhere in it, and getting it
//! wrong produces a render that succeeds, logs nothing, and comes back with every light in
//! the scene contributing nothing.
//!
//! `Light::has_contribution` in `src/scene/light.cpp` ends with this:
//!
//! ```text
//! const Shader *effective_shader = (get_shader()) ? get_shader() : scene->default_light;
//! return !is_zero(effective_shader->emission_estimate);
//! ```
//!
//! `get_shader()` returns the light's first used shader, and `xml_read_light` fills that in
//! from `state.shader` — the shader of the enclosing `<state>`. A `<light>` written bare sits
//! in whatever state the document happens to be in, which at the top level is
//! `scene->default_surface`, a diffuse closure whose emission estimate is zero. So the light
//! is created, is in the scene, is not culled, and emits nothing.
//!
//! Every punctual light therefore gets an `<emission>` shader of its own and a `<state>` that
//! names it. This was confirmed by rendering: the same area light with and without the
//! wrapper went from black to lit with no other change.
//!
//! # The sockets are not what they look like
//!
//! The loader's dispatch is a hardcoded chain over five strings: `point`, `sun`,
//! `background`, `area`, `spot`. Past that point everything is the node's own
//! `NODE_SOCKET_API`, and there is no `energy` and no `color` on any light. A light is
//! `strength`, and `strength` is a **colour**, not a number. An attribute the node does not
//! declare is ignored without a word, so a light written with `energy` and `color` renders
//! perfectly happily at the default strength of white.
//!
//! | kind    | its own sockets                            |
//! |---------|--------------------------------------------|
//! | point   | `radius`, `is_sphere`                      |
//! | spot    | `radius`, `is_sphere`, `angle`, `smooth`   |
//! | sun     | `angle`                                    |
//! | area    | `sizeu`, `sizev`, `ellipse`, `spread`      |
//!
//! # A point light here is a sphere light, not a delta light
//!
//! `PointLight` declares `radius` and `is_sphere`, which defaults to true, and the kernel
//! copy sets `is_sphere = is_sphere && radius != 0.0f`. So Cycles' point light is a genuine
//! sphere source: omnidirectional, and soft-shadowed as soon as it has a radius. It is a
//! delta light only in the special case of a radius of zero.
//!
//! That makes our `Point` map onto it directly — same shape of thing, same falloff, no
//! conversion. An earlier version of this file mapped ours onto an `area` light instead,
//! sized from the radius to match a solid angle, on the belief that Cycles' point light was
//! sharp. It is not, and the area light was worse than a direct mapping on two counts: it is
//! one-sided, so it lit only half of what our light lit, and matching a solid angle is a
//! calculation standing in for a mapping that was already exact.
//!
//! Rendering confirms it: two point lights of equal `strength`, one at `radius="0"` and one
//! at `radius="1.2"`, cast shadows whose penumbra visibly differs.

use super::LossyMode;
use super::error::{EmitError, EmitResult, UnsupportedField};
use super::lossy::LossyReport;
use super::read;
use super::writer::Writer;
use definition::Value;

/// One light, read out of the document.
pub struct Light {
    pub label: String,
    /// The `light_type` string to write. Always one of the five the loader dispatches on,
    /// because it comes from a `match` on our own variant tag and never from a string that
    /// travelled through the document.
    pub kind: &'static str,
    /// The radiance the emission node is given, as a colour.
    pub color: [f64; 3],
    /// How bright, as a multiplier.
    pub intensity: f64,
    /// Where a punctual light stands.
    pub position: Option<[f64; 3]>,
    /// Which way a directional or spot light points.
    pub direction: Option<[f64; 3]>,
    /// The outer half-angle of a spot cone, in radians.
    pub outer_angle: Option<f64>,
    /// The inner half-angle of a spot cone, in radians.
    pub inner_angle: Option<f64>,
    /// The name of the image an environment light reads.
    pub image: Option<String>,
    /// How much an environment light is scaled by.
    pub strength: f64,
}

/// Reads one light out of the document.
///
/// The variant tag decides the mapping, and every value written to the document comes from
/// the `match` below rather than from a string the author typed, so a `light_type` can only
/// be one of the five the loader dispatches on.
pub fn read_light(light: &Value) -> EmitResult<Light> {
    // A light is a variant, and its fields live in the variant payload rather than in a
    // struct, so the two accessors are not interchangeable here. `as_field_map` would
    // return `None` for every light and the failure would read as a malformed scene rather
    // than as a light.
    let fields = light.variant_fields().ok_or_else(|| EmitError::Malformed {
        detail: "a light is not a variant, so it has no fields".to_owned(),
    })?;
    let tag = light.variant_tag().ok_or_else(|| EmitError::Malformed {
        detail: "a light is not a variant, so its kind is unknown".to_owned(),
    })?;
    let label = read::text_field(fields, "label", "a light")?;
    let context = format!("light '{label}'");
    let color = match read::field(fields, "color") {
        Some(value) => read::triple(value, &format!("{context}'s color"))?,
        None => [1.0, 1.0, 1.0],
    };
    let intensity = read::field(fields, "intensity")
        .and_then(|value| read::number(value, "intensity").ok())
        .unwrap_or(1.0);

    // The shared part of every light, before the tag decides which kind it is.
    let base = Light {
        label,
        kind: "point",
        color,
        intensity,
        position: None,
        direction: None,
        outer_angle: None,
        inner_angle: None,
        image: None,
        strength: 1.0,
    };

    match tag {
        "Point" => Ok(Light {
            kind: "point",
            position: read_position(fields, &context)?,
            ..base
        }),
        "Directional" => Ok(Light {
            kind: "sun",
            direction: read_direction(fields, &context)?,
            ..base
        }),
        "Spot" => Ok(Light {
            kind: "spot",
            position: read_position(fields, &context)?,
            direction: read_direction(fields, &context)?,
            outer_angle: read_angle(fields, "outer_angle"),
            inner_angle: read_angle(fields, "inner_angle"),
            ..base
        }),
        "Environment" => Ok(Light {
            kind: "background",
            image: read::field(fields, "image")
                .map(|value| read::text(value, &format!("{context}'s image")))
                .transpose()?,
            strength: read::field(fields, "strength")
                .and_then(|value| read::number(value, "strength").ok())
                .unwrap_or(1.0),
            ..base
        }),
        other => Err(EmitError::Unsupported(UnsupportedField::Light {
            kind: other.to_owned(),
        })),
    }
}

fn read_position(fields: &definition::FieldMap, context: &str) -> EmitResult<Option<[f64; 3]>> {
    read::field(fields, "position")
        .map(|value| read::triple(value, &format!("{context}'s position")))
        .transpose()
}

fn read_direction(fields: &definition::FieldMap, context: &str) -> EmitResult<Option<[f64; 3]>> {
    read::field(fields, "direction")
        .map(|value| read::triple(value, &format!("{context}'s direction")))
        .transpose()
}

/// Reads a cone half-angle, in radians.
///
/// The document states these as half-angles and Cycles wants the whole cone, so this stays a
/// half-angle and the doubling happens where it is written. Values are clamped to a quarter
/// turn because a cone wider than that is a hemisphere, and a light that wraps behind itself
/// is not a cone.
fn read_angle(fields: &definition::FieldMap, name: &str) -> Option<f64> {
    read::field(fields, name)
        .and_then(|value| read::number(value, name).ok())
        .map(|angle| angle.clamp(0.0, std::f64::consts::FRAC_PI_2))
}

/// Writes one light, with the emission shader it needs in order to be seen.
///
/// The two blocks are written together because neither is meaningful alone: the `<shader>`
/// block is what makes the light emit, and the `<state>` is what binds the two.
pub fn write_light(writer: &mut Writer, light: &Light) {
    let shader_name = format!("{}_emission", light.label);
    let node_name = format!("{}_source", light.label);

    writer.open("shader", &[("name", shader_name.clone())]);
    writer.leaf(
        "emission",
        &[
            ("name", node_name.clone()),
            (
                "color",
                Writer::color(light.color[0], light.color[1], light.color[2]),
            ),
            ("strength", light.intensity.to_string()),
        ],
    );
    writer.leaf(
        "connect",
        &[
            ("from", format!("{node_name} emission")),
            ("to", "output surface".to_owned()),
        ],
    );
    writer.close("shader");

    // The light takes its placement from the scope around it, and takes its brightness from
    // the state around that, so the two nest with the light at the centre.
    writer.open("state", &[("shader", shader_name)]);
    write_light_scope(writer, light);
    writer.close("state");
}

/// Writes the placement scope and the light inside it.
fn write_light_scope(writer: &mut Writer, light: &Light) {
    let mut scope: Vec<(&str, String)> = Vec::with_capacity(2);
    if let Some(position) = light.position {
        scope.push((
            "translate",
            Writer::color(position[0], position[1], position[2]),
        ));
    }
    if let Some((axis, angle)) = super::transform::axis_angle(light_rotation(light)) {
        scope.push((
            "rotate",
            format!("{}, {}, {}, {}", angle, axis[0], axis[1], axis[2]),
        ));
    }
    if scope.is_empty() {
        write_light_element(writer, light);
    } else {
        writer.open("transform", &scope);
        write_light_element(writer, light);
        writer.close("transform");
    }
}

/// Writes the `<light>` element itself.
fn write_light_element(writer: &mut Writer, light: &Light) {
    let mut attributes: Vec<(&str, String)> = vec![
        ("name", light.label.clone()),
        ("light_type", light.kind.to_owned()),
        // `strength` is a colour on every light kind, and it is not the emission node's
        // strength: the node sets the radiance and this scales the whole source.
        (
            "strength",
            Writer::color(light.color[0], light.color[1], light.color[2]),
        ),
    ];

    match light.kind {
        "sun" => {
            attributes.push(("angle", "0.0".to_owned()));
        }
        "spot" => {
            // The document gives half-angles in radians and the loader wants the whole cone,
            // because it takes the cosine and the tangent of `angle * 0.5`.
            let outer = light.outer_angle.unwrap_or(std::f64::consts::FRAC_PI_4);
            let inner = light.inner_angle.unwrap_or(outer).min(outer);
            attributes.push(("angle", format!("{}", outer * 2.0)));
            // `smooth` is the width of the falloff band as a fraction of the cone, so a zero
            // here is the hard cutoff the loader treats as infinitely sharp.
            attributes.push((
                "smooth",
                format!("{}", ((outer - inner) / outer).max(0.001)),
            ));
        }
        _ => {}
    }

    writer.leaf("light", &attributes);
}

/// The rotation to write for a light, in degrees.
///
/// A light that names a direction is rotated from `+Z` onto it, because the loader's sun and
/// spot both emit along their own negative Z and a light with no rotation emits along the
/// world's. A light with no direction — a point light — gets no rotation, which is correct
/// because it radiates every way and has no facing.
fn light_rotation(light: &Light) -> [f64; 3] {
    match light.direction {
        Some(direction) => {
            let length =
                (direction[0].powi(2) + direction[1].powi(2) + direction[2].powi(2)).sqrt();
            if length < 1e-9 {
                return [0.0; 3];
            }
            direction_from_z(direction, length)
        }
        None => [0.0; 3],
    }
}

/// The Euler angles that turn `+Z` into `direction`.
///
/// Two axes are enough and are what a camera uses: a yaw about `+Y` followed by a pitch
/// about the resulting `+X`. Composing them as Euler angles rather than as a quaternion pair
/// keeps this consistent with the axis-angle conversion in the transform module, which is
/// the one place the two parameterisations meet.
fn direction_from_z(direction: [f64; 3], length: f64) -> [f64; 3] {
    let normalized = [
        direction[0] / length,
        direction[1] / length,
        direction[2] / length,
    ];
    let yaw = normalized[0].atan2(normalized[2]);
    let horizontal = (normalized[0].powi(2) + normalized[2].powi(2)).sqrt();
    let pitch = (-normalized[1]).atan2(horizontal);
    [pitch.to_degrees(), yaw.to_degrees(), 0.0]
}

/// Records that a light could not be emitted, honouring the lossy mode.
///
/// Shared by the scene writer so that one light being unrepresentable does not need a
/// second copy of the branch.
pub fn reject_or_record(kind: &str, mode: LossyMode, report: &mut LossyReport) -> EmitResult<()> {
    let unsupported = UnsupportedField::Light {
        kind: kind.to_owned(),
    };
    match mode {
        LossyMode::Reject => Err(EmitError::Unsupported(unsupported)),
        LossyMode::Degrade => {
            report.record(unsupported);
            Ok(())
        }
    }
}
