//! A tiled floor: the one gallery scene whose surface detail comes from sampling an image.
//!
//! # What this scene is for
//!
//! Every other scene in the gallery draws its detail as geometry or as arithmetic in a fragment
//! stage. This one samples a texture, and the texture is generated here rather than loaded,
//! because nothing in this framework reads the filesystem: a shader carries its own source and an
//! image carries its own pixels.
//!
//! The scene is deliberately small. What it has to show is that a scene can generate an image,
//! name it in a material's custom data, and have the compiler bind it and the shader sample it —
//! so the floor is one flat heightfield and the only thing on it is a grid of tiles.

use definition::Value;

use crate::geometry::heightfield;
use crate::kit::{
    IMAGE_FIELD, camera_placed, gallery_custom_schema_with_image, gallery_instance_schema, image,
    instance, instances, light, mesh, mesh_material, post_processes, screen_material,
    shaded_point_schema, shader, surface,
};
use crate::shaders::{depthing, post_shader, shadowing, textured, tile_pixels};
use scene::standard::Stage;

/// The size of the generated texture, in texels a side.
const SIZE: i64 = 256;

/// The extent of the floor, in world units.
const EXTENT: f64 = 12.0;

/// Builds the scene.
pub fn tiles() -> Value {
    tiles_of(tile_pixels(SIZE))
}

/// Builds the scene with a given image in place of the generated one.
///
/// # Why the image is a parameter
///
/// The only thing that proves a scene samples an image is that two different images give two
/// different frames. A single render cannot tell a bound texture from one that was never bound
/// and reads as whatever the driver left there, because both are *a* picture. Two renders that
/// differ are the whole of the evidence, and this is how a caller gets the second one.
pub fn tiles_of(pixels: Vec<u8>) -> Value {
    let point = shaded_point_schema();
    let inputs = crate::attributes::gallery_vertex_inputs(&point);

    // The shader reads the image, so its custom schema declares one. `tint` is beside it because
    // every gallery shader reads the row and a material that carried only an image would be a
    // material whose row has nothing in it.
    let custom = gallery_custom_schema_with_image();
    let lighting_source = textured(&inputs, "");

    let shaders = Value::List(vec![
        shader(
            Stage::Shadowing,
            "shadowing",
            &shadowing(&inputs),
            scene::standard::shadow_pass_schema(),
            point.clone(),
            gallery_instance_schema(),
            custom.clone(),
        ),
        shader(
            Stage::Depthing,
            "depthing",
            &depthing(&inputs),
            scene::standard::depth_pass_schema(),
            point.clone(),
            gallery_instance_schema(),
            custom.clone(),
        ),
        shader(
            Stage::Lighting,
            "lighting",
            &lighting_source,
            scene::standard::light_pass_schema(),
            point.clone(),
            gallery_instance_schema(),
            custom.clone(),
        ),
        post_shader(GRADE, ""),
    ]);

    // A material names one shader and that shader's stage is the pass it can be drawn in, so a
    // floor drawn in three stages carries three materials. Only the lighting one names an image:
    // a shadow map and a depth buffer are the pipeline's own textures rather than the scene's.
    let materials = Value::List(vec![
        mesh_material(
            "caster",
            "shadowing",
            surface([1.0, 1.0, 1.0], 0.0, 1.0, [0.0; 3]),
        ),
        mesh_material(
            "occluder",
            "depthing",
            surface([1.0, 1.0, 1.0], 0.0, 1.0, [0.0; 3]),
        ),
        mesh_material("tiles", "lighting", surface_with_image("tiles")),
    ]);

    Value::structure([
        ("name".to_owned(), Value::String("tiles".to_owned())),
        (
            "cameras".to_owned(),
            // Close and low, looking across the floor rather than down at it. A tile is a sizeable
            // fraction of the frame, so a texel lands on several pixels and the pattern reads as a
            // pattern. From further back the floor is minified past a texel a pixel, and the level
            // chain correctly turns the tiles into an average — a picture with nothing in it.
            Value::List(vec![camera_placed(
                [0.0, 0.7, 2.6],
                [0.0, 0.1, -8.0],
                std::f64::consts::FRAC_PI_4,
                0.1,
                40.0,
            )]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![light(
                "sun",
                [-6.0, 12.0, 7.0],
                [1.0, 0.94, 0.86],
                3.0,
                16.0,
            )]),
        ),
        ("shaders".to_owned(), shaders),
        (
            "images".to_owned(),
            Value::List(vec![image("tiles", SIZE, SIZE, pixels)]),
        ),
        (
            "meshes".to_owned(),
            Value::List(vec![mesh(
                "floor",
                "Triangles",
                point,
                heightfield(48, EXTENT, |_, _| 0.0),
            )]),
        ),
        ("mesh_materials".to_owned(), materials),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("post", "post")]),
        ),
        ("post_processes".to_owned(), post_processes(&["post"])),
        (
            "instances".to_owned(),
            instances(vec![
                (
                    Stage::Shadowing,
                    vec![instance(
                        "caster", "floor", [0.0; 3], [0.0; 3], [1.0; 3], "caster",
                    )],
                ),
                (
                    Stage::Depthing,
                    vec![instance(
                        "occluder", "floor", [0.0; 3], [0.0; 3], [1.0; 3], "occluder",
                    )],
                ),
                (
                    Stage::Lighting,
                    vec![instance(
                        "floor", "floor", [0.0; 3], [0.0; 3], [1.0; 3], "tiles",
                    )],
                ),
            ]),
        ),
    ])
}

/// The custom data of the tiled material: a tint, and the image it names.
fn surface_with_image(label: &str) -> Value {
    let mut fields = surface([1.0, 1.0, 1.0], 0.0, 1.0, [0.0, 0.0, 0.0]);
    if let Value::Struct(fields) = &mut fields {
        fields.insert(IMAGE_FIELD.to_owned(), Value::String(label.to_owned()));
    }
    fields
}

/// What the post stage does to the frame.
const GRADE: &str = r#"
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
"#;
