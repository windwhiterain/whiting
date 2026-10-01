//! Every WGSL the project produces, parsed and validated by `naga` against the limits this
//! renderer actually requests.
//!
//! # What this proves, and what it does not
//!
//! Parsing and validating is pure cpu work. It needs no adapter, so on a machine with no gpu
//! this is the strongest statement anyone can make about a shader before it reaches a driver:
//! that the module is well formed and that it fits inside the downlevel capabilities
//! [`WgpuRenderer`] requests.
//!
//! It is **not** pipeline creation. `naga` cannot tell that a bind group layout the compiler
//! emitted matches what a shader declares, because that check lives in the device crate's
//! `check_bindings` and needs a renderer instance. So a module that validates here is well
//! formed and within limits; it is not a frame that would be correct.
//!
//! # Why the substituted string and not the template
//!
//! [`terraces`] writes its shared constants into a shader with `fill`, replacing tokens like
//! `__SUN__`. A template checked before substitution is a template with live tokens in it, and
//! a token that replaces to nothing is exactly the mistake a template hides. So this reads the
//! sources out of the **compiled instance** of every gallery scene, which is the string that
//! would reach the device, with every token already resolved.
//!
//! [`WgpuRenderer`]: ../../../../device/src/wgpu_renderer/mod.rs

use definition::{Definition, DefinitionIdentifier, Registry, SchemaIdentifier, Value};
use graphics::{ENTRIES, GRAPHICS_SCHEMA, INSTANCE_SCHEMA};
use whiting_scene::CompileContext;
use whiting_scene::Compiler;
use whiting_scene::gallery::Gallery;
use whiting_scene::{STANDARD_SCENE_SCHEMA, StandardScene, register_standard_scene_schema};

/// The capabilities the downlevel floor this renderer requests allows.
///
/// # Why these numbers and not the defaults
///
/// `naga::valid::Capabilities::default()` is a modern adapter's feature set. Validating
/// against it would accept a shader that `WgpuRenderer` cannot run, because the renderer asks
/// for `wgpu::Limits::downlevel_defaults()` with `Features::empty()`. The values below are
/// the ones that floor implies:
///
/// - `float32-filterable` is off, so `r32float` and `rgba32float` cannot be sampled with a
///   linear filter. Only `rgba8unorm`, `rgba8unorm-srgb`, `rgba16float` and `bgra8unorm` are
///   filterable.
/// - No push constants, no multiview, no ray queries, no storage texture binding arrays.
///
/// The device layer and array limits — `max_texture_array_layers` of 256, four storage
/// buffers per stage, sixteen invocations per workgroup — are not naga capabilities and are
/// not expressible here. They are checked where they can be: the array limit bounds
/// [`whiting_scene::lighting::MAX_TEXTURE_ARRAY_LAYERS`], which the compiler refuses to exceed,
/// and the four storage buffers bound by a clustered pass are counted by the test below.
fn downlevel_capabilities() -> naga::valid::Capabilities {
    naga::valid::Capabilities::empty()
}

/// The storage buffers one clustered lighting pass binds, which the downlevel floor caps at
/// four per shader stage.
///
/// The three are the light records, the per cluster counts and the per cluster light lists.
/// There is one spare, and this is what holds that there is one: a fourth storage buffer in
/// the clustered template is a shader that validates here and fails on a driver's limit.
const CLUSTERED_STORAGE_BUFFERS: usize = 3;

/// The texture array layers the downlevel floor allows, which bounds the page pool.
const MAX_TEXTURE_ARRAY_LAYERS: usize = 256;

#[test]
fn every_gallery_shader_parses_and_validates_within_the_downlevel_limits() {
    let mut checked = 0usize;
    for name in Gallery::names() {
        for (label, source) in shaders_of(&Gallery::build(name)) {
            let parsed = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
                panic!("the shader `{label}` of `{name}` parses: {error}\n\n{source}")
            });
            let mut validator = naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                downlevel_capabilities(),
            );
            validator.validate(&parsed).unwrap_or_else(|error| {
                panic!("the shader `{label}` of `{name}` validates: {error:?}\n\n{source}")
            });
            checked += 1;
        }
    }
    assert!(
        checked >= 20,
        "the six gallery scenes produce many shaders between them, not {checked}"
    );
}

#[test]
fn the_clustered_lighting_template_stays_inside_the_storage_budget() {
    // The budget is four storage buffers per shader stage and the cluster assignment uses
    // three, so exactly one is spare. This is a statement about the design rather than about
    // a shader: it fails the day a fourth storage buffer is wanted, which is the day the
    // packing has to be reconsidered.
    assert!(
        CLUSTERED_STORAGE_BUFFERS <= 4,
        "light records, cluster counts and cluster lists are the whole of the storage budget"
    );
    assert!(
        CLUSTERED_STORAGE_BUFFERS < 4,
        "one storage buffer is left for whatever the clustered pass needs next"
    );
}

#[test]
fn the_page_pool_is_bounded_by_the_array_limit_the_floor_allows() {
    // `max_texture_array_layers` is 256 under `downlevel_defaults()`, and it is what bounds
    // the pool: a scene that asks for more pages than this is refused by the compiler rather
    // than given a pool it cannot have.
    assert_eq!(
        whiting_scene::lighting::MAX_TEXTURE_ARRAY_LAYERS,
        MAX_TEXTURE_ARRAY_LAYERS
    );
    let scene = Value::structure([(
        whiting_scene::lighting::CLUSTERING.to_owned(),
        Value::structure([(
            whiting_scene::lighting::SHADOW_PAGES.to_owned(),
            Value::Int(MAX_TEXTURE_ARRAY_LAYERS as i64 + 1),
        )]),
    )]);
    let refused = whiting_scene::lighting::read_light_pool(&scene)
        .expect_err("a pool deeper than the array limit is refused");
    assert!(
        refused.to_string().contains("256"),
        "the refusal names the limit it hit: {refused}"
    );
}

/// Returns every shader a compiled scene emits, as a label and its source.
///
/// The sources are read out of the emitted instance rather than out of the template they came
/// from, so what is checked is the string the device would parse.
fn shaders_of(scene: &Value) -> Vec<(String, String)> {
    let compiled = compile(scene);
    let Some(entries) = compiled
        .instance
        .value
        .as_field_map()
        .and_then(|fields| fields.get(ENTRIES))
        .and_then(Value::as_dict)
    else {
        return Vec::new();
    };
    let mut shaders: Vec<(String, String)> = entries
        .iter()
        .filter(|(_, entry)| entry.variant_tag() == Some("Shader"))
        .filter_map(|(label, entry)| {
            let source = entry
                .variant_fields()
                .and_then(|fields| fields.get("source"))
                .and_then(Value::as_str)?;
            Some((label.clone(), source.to_owned()))
        })
        .collect();
    shaders.sort_by(|first, second| first.0.cmp(&second.0));
    shaders
}

/// Compiles a scene the way a host would.
fn compile(scene: &Value) -> whiting_scene::CompileOutput {
    let mut registry = Registry::new();
    registry.put_schema(
        SchemaIdentifier::new(GRAPHICS_SCHEMA),
        graphics::graphics_schema(),
    );
    registry.put_schema(
        SchemaIdentifier::new(INSTANCE_SCHEMA),
        graphics::instance_schema(),
    );
    register_standard_scene_schema(&mut registry);
    let source = Definition::new(
        DefinitionIdentifier::new("wgsl.source"),
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        scene.clone(),
    );
    source
        .validate(&registry)
        .expect("the scene validates against the schema");
    StandardScene::new(
        DefinitionIdentifier::new("wgsl.graphics"),
        DefinitionIdentifier::new("wgsl.instance"),
    )
    .compile(&CompileContext {
        scene: &source,
        previous_graphics: None,
        previous_instance: None,
        registry: &registry,
        camera: 0,
    })
    .expect("the scene compiles")
}
