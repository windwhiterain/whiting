//! A throwaway end-to-end check: build a scene document, emit it, and write the XML.
//!
//! This exists to be run once and deleted, not to be a test. It builds a document by hand
//! because the point is to exercise the emitter against a renderer, and a unit test could
//! not tell us whether the XML it produced is one Cycles accepts.
//!
//! The scene is arranged so that a single render distinguishes the paths that matter:
//!
//! - the left quad has an identity transform, so it exercises `<state>` without `<transform>`
//! - the right quad has a translation, a rotation and a scale, so it exercises the nesting
//! - the right quad is metallic and the left is not, so the two must not look alike
//! - the point light exercises the mapping of our clustered sphere light onto an area light
//! - the environment light gives the frame a floor value rather than black

use definition::Value;
use whiting_scene::cycles_export::{self, EmitOptions, LossyMode};

fn point(normal: [f64; 3], position: [f64; 3], uv: [f64; 2]) -> Value {
    let triple = |values: [f64; 3]| Value::List(values.into_iter().map(Value::Float).collect());
    Value::structure([
        ("normal".to_owned(), triple(normal)),
        ("position".to_owned(), triple(position)),
        (
            "uv".to_owned(),
            Value::List(uv.into_iter().map(Value::Float).collect()),
        ),
    ])
}

/// A quad standing in the plane the camera looks along, facing it.
///
/// # Which way the camera faces
///
/// The quad faces `+Z`, and the camera stands at negative Z. That is not a guess: a camera at
/// `translate="0, 0, -6"` with no rotation renders this quad, and the same document with the
/// camera at `translate="0, 0, 6"` renders only the world background. So the loader's camera
/// looks along its own `+Z` at identity, which is the same convention
/// [`cycles_export::scene`](scene::cycles_export) writes a camera's rotation for.
fn quad(centre: [f64; 3], half_width: f64, half_height: f64) -> Vec<Value> {
    let [cx, cy, cz] = centre;
    [
        (cx - half_width, cy - half_height, 0.0, 0.0f64),
        (cx + half_width, cy - half_height, 1.0, 0.0),
        (cx + half_width, cy + half_height, 1.0, 1.0),
        (cx - half_width, cy + half_height, 0.0, 1.0),
    ]
    .into_iter()
    .map(|(x, y, u, v)| point([0.0, 0.0, 1.0], [x, y, cz], [u, v]))
    .collect()
}

/// A mesh declaration wrapping the given points.
fn mesh(label: &str, points: Vec<Value>) -> Value {
    Value::structure([
        ("label".to_owned(), Value::String(label.to_owned())),
        (
            "topology".to_owned(),
            Value::variant("Triangles", Vec::<(String, Value)>::new()),
        ),
        (
            "point_schema".to_owned(),
            Value::structure([
                ("position".to_owned(), component_schema(3)),
                ("normal".to_owned(), component_schema(3)),
                ("uv".to_owned(), component_schema(2)),
            ]),
        ),
        ("points".to_owned(), Value::List(points)),
        (
            "indices".to_owned(),
            Value::List(
                [0i64, 1, 2, 0, 2, 3]
                    .into_iter()
                    .map(|index| Value::Int(index))
                    .collect(),
            ),
        ),
    ])
}

fn component_schema(count: i64) -> Value {
    Value::structure([
        (
            "type".to_owned(),
            Value::variant("Components", vec![("count".to_owned(), Value::Int(count))]),
        ),
        ("documentation".to_owned(), Value::String(String::new())),
    ])
}

/// An instance with the given placement.
fn instance(label: &str, mesh: &str, material: &str, transform: [[f64; 3]; 3]) -> Value {
    let (translation, rotation, scale) = (transform[0], transform[1], transform[2]);
    Value::structure([
        ("label".to_owned(), Value::String(label.to_owned())),
        ("mesh".to_owned(), Value::String(mesh.to_owned())),
        ("material".to_owned(), Value::String(material.to_owned())),
        (
            "transform".to_owned(),
            Value::structure([
                (
                    "translation".to_owned(),
                    Value::List(translation.into_iter().map(Value::Float).collect()),
                ),
                (
                    "rotation".to_owned(),
                    Value::List(rotation.into_iter().map(Value::Float).collect()),
                ),
                (
                    "scale".to_owned(),
                    Value::List(scale.into_iter().map(Value::Float).collect()),
                ),
            ]),
        ),
    ])
}

/// A material. Every parameter travels inside `custom_data`, because that is the only place the
/// document carries one.
fn material(label: &str, tint: [f64; 3], roughness: f64, metallic: f64) -> Value {
    Value::structure([
        ("label".to_owned(), Value::String(label.to_owned())),
        (
            "shader".to_owned(),
            Value::String("LightingShader".to_owned()),
        ),
        (
            "custom_data".to_owned(),
            Value::structure([
                (
                    "tint".to_owned(),
                    Value::List(tint.into_iter().map(Value::Float).collect()),
                ),
                ("roughness".to_owned(), Value::Float(roughness)),
                ("metallic".to_owned(), Value::Float(metallic)),
            ]),
        ),
    ])
}

fn camera(translation: [f64; 3], width: i64, height: i64) -> Value {
    Value::structure([
        (
            "transform".to_owned(),
            Value::structure([
                (
                    "translation".to_owned(),
                    Value::List(translation.into_iter().map(Value::Float).collect()),
                ),
                (
                    "rotation".to_owned(),
                    Value::List([0.0f64, 0.0, 0.0].into_iter().map(Value::Float).collect()),
                ),
                (
                    "scale".to_owned(),
                    Value::List([1.0f64, 1.0, 1.0].into_iter().map(Value::Float).collect()),
                ),
            ]),
        ),
        ("width".to_owned(), Value::Int(width)),
        ("height".to_owned(), Value::Int(height)),
    ])
}

fn main() {
    let scene = Value::structure([
        ("name".to_owned(), Value::String("emitter check".to_owned())),
        (
            "meshes".to_owned(),
            Value::List(vec![
                mesh("left", quad([-1.6, 0.0, 0.0], 1.0, 1.0)),
                mesh("right", quad([1.6, 0.0, 0.0], 1.0, 1.0)),
            ]),
        ),
        (
            "mesh_materials".to_owned(),
            Value::List(vec![
                material("matte", [0.8, 0.25, 0.2], 0.6, 0.0),
                material("metal", [0.75, 0.7, 0.6], 0.12, 1.0),
            ]),
        ),
        (
            "instances".to_owned(),
            Value::List(vec![
                // Identity, so the `<transform>` scope is skipped entirely.
                instance(
                    "the matte plane",
                    "left",
                    "matte",
                    [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]],
                ),
                // Every channel non-identity, so the scope is written in full.
                instance(
                    "the metal plane",
                    "right",
                    "metal",
                    [[0.0, 0.3, 0.0], [0.0, 0.0, -18.0], [0.8, 0.8, 0.8]],
                ),
            ]),
        ),
        (
            "cameras".to_owned(),
            Value::List(vec![camera([0.0, 0.0, -6.0], 640, 400)]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![
                // Travels forward and to the right, so both quads are lit at an angle.
                Value::variant(
                    "Directional",
                    vec![
                        ("label".to_owned(), Value::String("sun".to_owned())),
                        (
                            "direction".to_owned(),
                            Value::List(
                                [0.3f64, -0.7, 0.6].into_iter().map(Value::Float).collect(),
                            ),
                        ),
                        (
                            "color".to_owned(),
                            Value::List(
                                [1.0f64, 0.95, 0.85].into_iter().map(Value::Float).collect(),
                            ),
                        ),
                        ("irradiance".to_owned(), Value::Float(4.0)),
                    ],
                ),
                // Cycles' point light is a sphere source once it has a radius, which is the same
                // shape of thing ours is, so this one maps across directly and `radius` is not
                // written: ours is how far the light reaches, not how big it is.
                Value::variant(
                    "Point",
                    vec![
                        ("label".to_owned(), Value::String("key".to_owned())),
                        (
                            "position".to_owned(),
                            Value::List(
                                [2.0f64, 2.0, -2.0].into_iter().map(Value::Float).collect(),
                            ),
                        ),
                        (
                            "color".to_owned(),
                            Value::List([1.0f64, 0.9, 0.8].into_iter().map(Value::Float).collect()),
                        ),
                        ("power".to_owned(), Value::Float(120.0)),
                        ("radius".to_owned(), Value::Float(0.5)),
                    ],
                ),
                Value::variant(
                    "Environment",
                    vec![
                        ("label".to_owned(), Value::String("sky".to_owned())),
                        (
                            "color".to_owned(),
                            Value::List([0.3f64, 0.4, 0.6].into_iter().map(Value::Float).collect()),
                        ),
                        ("radiance".to_owned(), Value::Float(1.0)),
                    ],
                ),
            ]),
        ),
    ]);

    let options = EmitOptions {
        lossy: LossyMode::Reject,
        exposure: 1.0,
        seed: 7,
        denoise: false,
        max_bounce: 6,
    };

    match cycles_export::emit_to_file(&scene, std::path::Path::new("emitted.xml"), &options) {
        Ok(report) => {
            println!("emitted, losses: {report}");
            println!("---");
            print!("{}", std::fs::read_to_string("emitted.xml").unwrap());
        }
        Err(error) => {
            eprintln!("emit failed: {error}");
            std::process::exit(1);
        }
    }
}
