//! A field of monoliths standing in shallow water, lit by one low light.
//!
//! The scene is the same pipeline as the dune field and a deliberately different use of
//! it: hard vertical surfaces, a mirror under them, and a light low enough that the
//! reflections run the length of the frame.

use definition::Value;

use crate::gallery::geometry::heightfield;
use crate::gallery::kit::{
    camera, instance, light, material, mesh, shaded_point_schema, surface, surface_reflecting,
};
use crate::gallery::shaders::{mesh_shaders, post_shader};

/// How many monoliths stand in the field.
const COUNT: usize = 40;

/// Builds the monolith field.
pub fn monoliths() -> Value {
    let mut instances = Vec::new();
    for index in 0..COUNT as i64 {
        // A deterministic scatter, so an unchanged scene compiles to an unchanged field.
        let x = (crate::gallery::geometry::hash01(index, 7, 3) - 0.5) * 90.0;
        let z = (crate::gallery::geometry::hash01(index, 13, 5) - 0.5) * 90.0;
        let height = 6.0 + crate::gallery::geometry::hash01(index, 29, 11) * 22.0;
        let turn = crate::gallery::geometry::hash01(index, 41, 13) * std::f64::consts::TAU;

        // One instance per stage, as the dune field has: the geometry is authored once and
        // drawn three times over.
        for (label, shader, tint) in [
            ("cast", "shadowing", [1.0, 1.0, 1.0]),
            ("depth", "depthing", [1.0, 1.0, 1.0]),
            (
                "shade",
                "lighting",
                [0.05 + 0.03 * (index % 3) as f64, 0.055, 0.07],
            ),
        ] {
            instances.push(instance(
                &format!("{label}-{index}"),
                "monolith",
                [x, -0.5, z],
                [0.0, turn, 0.0],
                [1.0, height, 1.0],
                material(shader, surface(tint, 0.1, 0.35, [0.0; 3])),
            ));
        }
    }

    // The ground, once per stage as well. A ground that were only in the lighting stage
    // would cast no shadow and would be missing from the depth the lighting pass tests
    // against, so it is drawn by all three like anything else.
    for (label, shader) in [
        ("cast", "shadowing"),
        ("depth", "depthing"),
        ("shade", "lighting"),
    ] {
        instances.push(instance(
            &format!("ground-{label}"),
            "ground",
            [0.0; 3],
            [0.0; 3],
            [1.0; 3],
            material(
                shader,
                // A smooth, dark, very slightly metallic surface. What stands in for
                // water without a reflection pass is a very smooth dielectric: it
                // reflects the sky at grazing angles and almost nothing looking straight
                // down, which is what a flat mirror of a sky looks like.
                surface_reflecting([0.018, 0.020, 0.026], 0.0, 0.06, [0.0; 3], 1.0),
            ),
        ));
    }

    Value::structure([
        ("name".to_owned(), Value::String("monoliths".to_owned())),
        (
            "camera".to_owned(),
            // Low, close, and cropped hard in. The ground reaches far past the frame in
            // every direction, so what decides the framing is the padding: at one the
            // camera would frame the whole plain and the monoliths would be specks on it.
            camera(
                [34.0, 7.0, 44.0],
                [-6.0, 12.0, -10.0],
                std::f64::consts::FRAC_PI_3,
                0.15,
            ),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![light(
                "sun",
                [-60.0, 30.0, 48.0],
                [1.0, 0.72, 0.45],
                4.0,
                90.0,
            )]),
        ),
        (
            "shaders".to_owned(),
            Value::List(
                mesh_shaders(SURFACE, "")
                    .into_iter()
                    .chain(std::iter::once(post_shader(GRADE)))
                    .collect(),
            ),
        ),
        (
            "meshes".to_owned(),
            Value::List(vec![
                mesh(
                    "monolith",
                    "Triangles",
                    shaded_point_schema(),
                    crate::gallery::geometry::sphere(20, 0.5),
                ),
                // The ground the monoliths stand in. It is a heightfield like any other,
                // with a height that does not vary, which is what makes it flat and what
                // makes it a mesh rather than a clear colour: a surface that can hold a
                // reflection has to be geometry, because a reflection is a view of it.
                mesh(
                    "ground",
                    "Triangles",
                    shaded_point_schema(),
                    heightfield(120, 340.0, |_, _| 0.0),
                ),
            ]),
        ),
        ("instances".to_owned(), Value::List(instances)),
    ])
}

/// What the lighting stage does to a surface after the light is applied.
///
/// # Invariants
///
/// The ground is wet, so what it shows is the sky rather than a colour of its own. A
/// reflection off a smooth surface follows the same Fresnel term the shading already
/// computed, so it is strongest where the surface is seen edge on and vanishes where it
/// is seen flat on — which is the whole of what makes a flat plane read as a liquid one.
const SURFACE: &str = r#"
    let eye = uniforms.projection.position;
    let from_eye = length(eye - in.world_position);
    let view = normalize(in.world_position - eye);
    let grazing = pow(1.0 - abs(dot(normal, view)), 3.0);
    // The name is not the lighting stage's: this is written into the same function, so a
    // name used twice is a shader that will not compile.
    let wet = vec3<f32>(0.40, 0.50, 0.68);
    // The sky is shown only as far as the row says this surface shows it. A wet floor and
    // a stone are drawn by the same pass with the same shader, so without this both would
    // be as reflective as the wettest thing in the scene.
    let amount = in.material.z;
    let haze = 1.0 - exp(-from_eye * 0.004);
    color = color + wet * grazing * amount * 1.6;
    color = mix(color, wet, clamp(haze * 0.9 * amount, 0.0, 0.96));
"#;

/// What the post stage does to the frame.
const GRADE: &str = r#"
    if depth >= 1.0 {
        let ray = ray_through(in.uv);
        let above = pow(clamp(ray.y, 0.0, 1.0), 0.5);
        let horizon = vec3<f32>(0.82, 0.66, 0.48);
        let zenith = vec3<f32>(0.10, 0.19, 0.42);
        let below = vec3<f32>(0.40, 0.34, 0.30);
        let sky = select(mix(horizon, below, clamp(-ray.y * 4.0, 0.0, 1.0)),
                         mix(horizon, zenith, above),
                         ray.y > 0.0);
        color = sky;
    }
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = clamp(mix(vec3<f32>(luma), color, 1.28), vec3<f32>(0.0), vec3<f32>(1.0));
    let centred = in.uv - vec2<f32>(0.5, 0.5);
    color = color * clamp(1.0 - dot(centred, centred) * 0.5, 0.0, 1.0);
"#;
