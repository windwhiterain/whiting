//! A material's custom data naming an image, from the scene document to the emitted pair.
//!
//! # What these prove
//!
//! An image is written in the one slot a material has for its own data, and the compiler is what
//! turns it into a texture, a sampler and a binding point. Three things are checked, and each is
//! something that would be a silent wrong frame rather than an error:
//!
//! - the name is lifted out of the custom data, so the instance row carries the numbers and not
//!   the name — a name packed into a row of floats is garbage, not a compile failure;
//! - the image is emitted once, so two materials naming one image bind one texture rather than
//!   uploading the same pixels twice;
//! - the binding point is the custom field's own name, so the shader declares the point its own
//!   schema already names and no author writes a binding number.
//!
//! A name no image carries is the fourth, and it is checked as an error rather than as a texture
//! that reads black.

use std::collections::BTreeMap;

use definition::{Definition, DefinitionIdentifier, Registry, Schema, SchemaIdentifier, Value};
use graphics::binders::{DATA, SAMPLED};
use graphics::{BINDERS, ENTRIES, PASSES_FIELD};
use whiting_scene::standard::{
    IMAGES, MESH_MATERIALS, SHADERS, StandardScene, example_standard_scene,
    register_standard_scene_schema,
};
use whiting_scene::{CompileContext, Compiler};

/// The name the image is written under in a material's custom data, and therefore the binding
/// point it is bound at.
const IMAGE: &str = "image";

/// One texel of the image, as four bytes.
const TEXEL: [u8; 4] = [0x40, 0x80, 0xC0, 0xFF];

#[test]
fn a_named_image_becomes_a_texture_at_the_field_s_name() {
    let compiled = compiled(&scene(vec![material("quad", "lighting", Some("brick"))]));

    // The pass binds a texture at `image`, which is the field the material named it in.
    let binders = binders_of(&compiled.graphics, "lighting");
    assert!(
        binders.contains_key(IMAGE),
        "the pass binds the field's own name: {:?}",
        binders.keys().collect::<Vec<_>>()
    );

    // The pixels reached the instance document, once, under the image's own name.
    let entries = entries_of(&compiled.instance);
    let texture = entries
        .get("image_brick")
        .unwrap_or_else(|| panic!("the image is emitted: {:?}", entries.keys()));
    let fields = texture.variant_fields().expect("a texture is a variant");
    assert_eq!(
        fields.get("format").and_then(Value::as_str),
        Some("rgba8unorm")
    );
    assert_eq!(fields.get("width").and_then(Value::as_int), Some(1));
    assert_eq!(fields.get("height").and_then(Value::as_int), Some(1));
    assert_eq!(
        fields.get(DATA).and_then(Value::as_bytes),
        Some(&TEXEL[..]),
        "the pixels a scene declares are the pixels that are uploaded"
    );
    assert!(
        fields.contains_key(SAMPLED),
        "an image a scene declares is an image a shader samples"
    );
}

#[test]
fn the_name_is_not_packed_into_the_instance_row() {
    let compiled = compiled(&scene(vec![material("quad", "lighting", Some("brick"))]));

    let entries = entries_of(&compiled.instance);
    let (name, rows) = entries
        .iter()
        .find(|(name, _)| name.contains("lighting") && name.ends_with("instance_data"))
        .unwrap_or_else(|| {
            panic!(
                "the lighting row is emitted: {:?}",
                entries.keys().collect::<Vec<_>>()
            )
        });
    let schema = rows
        .variant_fields()
        .and_then(|fields| fields.get("schema"))
        .expect("a buffer carries the schema it was packed with");
    let schema = Schema::from_value(schema).expect("the packed schema reads");
    let definition::SchemaKind::Struct { fields } = &schema.kind else {
        panic!("a row schema is a struct");
    };
    assert!(
        !fields.contains_key(IMAGE),
        "a name is not a number and does not go in the row ({name}): {:?}",
        fields.keys().collect::<Vec<_>>()
    );
    // The numbers that were beside it did go in, which is what makes this a sort rather than a
    // whole field being dropped.
    assert!(
        fields.contains_key("tint"),
        "the numbers beside the name are still packed ({name}): {:?}",
        fields.keys().collect::<Vec<_>>()
    );
}

#[test]
fn two_materials_naming_one_image_upload_it_once() {
    let compiled = compiled(&scene(vec![
        material("quad", "lighting", Some("brick")),
        material("quad_again", "lighting", Some("brick")),
    ]));

    let entries = entries_of(&compiled.instance);
    let uploaded: Vec<&String> = entries
        .keys()
        .filter(|name| name.starts_with("image_"))
        .collect();
    assert_eq!(
        uploaded,
        vec!["image_brick"],
        "one image is one texture however many materials name it"
    );
}

#[test]
fn a_name_no_image_carries_is_an_error() {
    let failure = compile(&scene(vec![material("quad", "lighting", Some("nonesuch"))]))
        .err()
        .expect("an unknown image name is refused");
    let message = failure.to_string();
    assert!(
        message.contains("nonesuch") && message.contains("quad"),
        "the message names the material and the name: {message}"
    );
    assert!(
        message.contains("does not declare"),
        "and says the scene does not declare it: {message}"
    );
}

/// Compiles a scene that is expected to compile.
fn compiled(scene: &Value) -> Compiled {
    compile(scene).expect("the test scene compiles")
}

/// Compiles a scene, or returns why it would not.
fn compile(scene: &Value) -> Result<Compiled, whiting_scene::CompileError> {
    let mut registry = Registry::new();
    register_standard_scene_schema(&mut registry);
    // A compile verifies the documents it emitted against the schemas they claim, and an
    // emitted pair claims the graphics pair.
    registry.put_schema(
        SchemaIdentifier::new(graphics::GRAPHICS_SCHEMA),
        graphics::graphics_schema(),
    );
    registry.put_schema(
        SchemaIdentifier::new(graphics::INSTANCE_SCHEMA),
        graphics::instance_schema(),
    );
    let source = Definition::new(
        DefinitionIdentifier::new("image.source"),
        SchemaIdentifier::new(whiting_scene::STANDARD_SCENE_SCHEMA),
        scene.clone(),
    );
    source
        .validate(&registry)
        .expect("the test scene satisfies the scene schema");
    let compiler = StandardScene::new(
        DefinitionIdentifier::new("image.graphics"),
        DefinitionIdentifier::new("image.instance"),
    );
    let compiled = compiler.compile(&CompileContext {
        scene: &source,
        previous_graphics: None,
        previous_instance: None,
        registry: &registry,
        camera: 0,
    })?;
    Ok(Compiled {
        graphics: compiled.graphics.value,
        instance: compiled.instance.value,
    })
}

/// The two documents a compile emitted.
struct Compiled {
    graphics: Value,
    instance: Value,
}

/// Returns the binders of the pass whose label starts with `stage`.
fn binders_of(graphics: &Value, stage: &str) -> BTreeMap<String, Value> {
    let passes = graphics
        .as_field_map()
        .and_then(|fields| fields.get(PASSES_FIELD))
        .and_then(Value::as_list)
        .expect("the graphics document holds passes");
    let pass = passes
        .iter()
        .find(|pass| {
            pass.variant_fields()
                .and_then(|fields| fields.get("label"))
                .and_then(Value::as_str)
                .is_some_and(|label| label.starts_with(stage))
        })
        .unwrap_or_else(|| panic!("a `{stage}` pass was emitted"));
    pass.variant_fields()
        .and_then(|fields| fields.get(BINDERS))
        .and_then(Value::as_dict)
        .cloned()
        .expect("a pass carries its binders")
}

/// Returns the entries of the emitted instance document.
fn entries_of(instance: &Value) -> BTreeMap<String, Value> {
    instance
        .as_field_map()
        .and_then(|fields| fields.get(ENTRIES))
        .and_then(Value::as_dict)
        .cloned()
        .expect("an instance holds its entries under one field")
}

/// Builds the example scene with an image list and the named materials.
///
/// The example already carries one instance in each mesh stage and a material drawing it, so the
/// only things added are the image, the materials that name it, and one more lighting instance so
/// that a group has something to share.
fn scene(materials: Vec<Value>) -> Value {
    let mut scene = example_standard_scene();
    let fields = scene
        .as_field_map_mut()
        .expect("the example scene is a struct");
    fields.insert(
        IMAGES.to_owned(),
        Value::List(vec![Value::structure([
            ("label".to_owned(), Value::String("brick".to_owned())),
            ("width".to_owned(), Value::Int(1)),
            ("height".to_owned(), Value::Int(1)),
            ("format".to_owned(), Value::String("rgba8unorm".to_owned())),
            ("data".to_owned(), Value::Bytes(TEXEL.to_vec())),
        ])]),
    );
    // A custom schema may name an image, and a material's data is verified against it, so the
    // example's lighting shader is given the field for a material to fill in.
    if let Some(shaders) = fields.get_mut(SHADERS).and_then(Value::as_list_mut) {
        for shader in shaders.iter_mut() {
            if shader.variant_tag() != Some("LightShader") {
                continue;
            }
            // A variant's fields have no mutable accessor, so the variant is rebuilt from the
            // fields it had with one of them replaced.
            let Some(tag) = shader.variant_tag().map(str::to_owned) else {
                continue;
            };
            let mut shader_fields = shader.variant_fields().cloned().unwrap_or_default();
            shader_fields.insert(
                "custom_schema".to_owned(),
                Schema::new(definition::SchemaKind::structure([
                    (
                        "tint".to_owned(),
                        Schema::new(definition::SchemaKind::list(Schema::new(
                            definition::SchemaKind::Float,
                        )))
                        .with_attributes(
                            definition::Attributes::new()
                                .with_extension("components", Value::Int(3)),
                        ),
                    ),
                    (IMAGE.to_owned(), graphics::image_field()),
                ]))
                .to_value(),
            );
            *shader = Value::Enum {
                tag,
                fields: shader_fields,
            };
        }
    }
    // The example carries a `quad` material drawing its lighting instance, and the material
    // this test is about is that one: it is replaced rather than added beside, so the scene
    // keeps one material per label.
    let mut all: Vec<Value> = fields
        .get(MESH_MATERIALS)
        .and_then(Value::as_list)
        .map(<[Value]>::to_vec)
        .unwrap_or_default()
        .into_iter()
        .filter(|material| {
            material
                .as_field_map()
                .and_then(|fields| fields.get("label"))
                .and_then(Value::as_str)
                != Some("quad")
        })
        .collect();
    // The example lists one lighting instance, drawing the material this test replaces. Every
    // material added gets an instance of its own, at the example's lighting instance's own
    // transform, so a group has several members and the sharing it does is what a scene does.
    let template = fields
        .get("instances")
        .and_then(Value::as_dict)
        .and_then(|buckets| buckets.get("lighting"))
        .and_then(Value::as_list)
        .and_then(|bucket| bucket.first())
        .cloned();
    // The example's lighting instance is replaced as well, so that every instance in the bucket
    // draws a material that names the image.
    if let Some(instances) = fields.get_mut("instances").and_then(Value::as_dict_mut) {
        if let Some(bucket) = instances.get_mut("lighting").and_then(Value::as_list_mut) {
            bucket.clear();
        }
    }
    if let Some(template) = template {
        for material in &materials {
            let label = material
                .as_field_map()
                .and_then(|fields| fields.get("label"))
                .and_then(Value::as_str)
                .expect("a material has a label");
            let mut instance = template.clone();
            if let Some(instance_fields) = instance.as_field_map_mut() {
                instance_fields.insert(
                    "label".to_owned(),
                    Value::String(format!("{label}_instance")),
                );
                instance_fields.insert("material".to_owned(), Value::String(label.to_owned()));
            }
            if let Some(instances) = fields.get_mut("instances").and_then(Value::as_dict_mut) {
                if let Some(bucket) = instances.get_mut("lighting").and_then(Value::as_list_mut) {
                    bucket.push(instance);
                }
            }
        }
    }
    all.extend(materials);
    fields.insert(MESH_MATERIALS.to_owned(), Value::List(all));
    scene
}

/// Builds one mesh material, optionally naming an image.
fn material(label: &str, shader: &str, image: Option<&str>) -> Value {
    let mut custom = definition::FieldMap::new();
    custom.insert(
        "tint".to_owned(),
        Value::List([1.0, 1.0, 1.0].into_iter().map(Value::Float).collect()),
    );
    if let Some(image) = image {
        custom.insert(IMAGE.to_owned(), Value::String(image.to_owned()));
    }
    Value::structure([
        ("label".to_owned(), Value::String(label.to_owned())),
        ("shader".to_owned(), Value::String(shader.to_owned())),
        ("custom_data".to_owned(), Value::Struct(custom)),
    ])
}
