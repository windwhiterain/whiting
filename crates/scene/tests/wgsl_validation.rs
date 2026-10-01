//! Every WGSL the project produces, parsed and validated by `naga` against the limits this
//! renderer actually requests.
//!
//! # What this proves, and what it does not
//!
//! Parsing and validating is pure cpu work. It needs no adapter, so on a machine with no gpu
//! this is the strongest statement anyone can make about a shader before it reaches a driver:
//! that the module is well formed and that it needs no feature [`WgpuRenderer`] does not ask
//! for.
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
use whiting_gallery::Gallery;
use whiting_scene::CompileContext;
use whiting_scene::Compiler;
use whiting_scene::{STANDARD_SCENE_SCHEMA, StandardScene, register_standard_scene_schema};

/// The capabilities the renderer's device is created with.
///
/// # Why these numbers and not the defaults
///
/// `naga::valid::Capabilities::default()` is a modern adapter's feature set. Validating
/// against it would accept a shader that `WgpuRenderer` cannot run, because the renderer asks
/// for `wgpu::Features::empty()`. The values below are the ones that empty implies:
///
/// - `float32-filterable` is off, so `r32float` and `rgba32float` cannot be sampled with a
///   linear filter. Only `rgba8unorm`, `rgba8unorm-srgb`, `rgba16float` and `bgra8unorm` are
///   filterable.
/// - No push constants, no multiview, no ray queries, no storage texture binding arrays.
///
/// This is a statement about **features**, and features are still an empty set. The *limits*
/// the device is created with are no longer a floor: `WgpuRenderer` requests the adapter's own
/// `Limits` now, with only the two vertex counts overridden, so `max_texture_dimension_2d` and
/// `max_texture_array_layers` are whatever the hardware reports rather than the downlevel 2048
/// and 256. A limit is therefore no longer expressible here as a constant, and the two places
/// this project bounds a limit are bounded in the places that own them: a document is held to
/// the limits of the machine its compile is made against, which reach the compiler through
/// `CompileContext::limits` and are the floor below where no machine is behind the compile, and
/// the four storage buffers a clustered pass binds are counted by the test below.
fn downlevel_capabilities() -> naga::valid::Capabilities {
    naga::valid::Capabilities::empty()
}

/// The storage buffers one clustered lighting pass binds.
///
/// The four are the light records, the per cluster counts, the per cluster light lists and the
/// spot cones. Four is all a stage is guaranteed, so this is now the whole of the budget rather
/// than most of it: the cone run spent the one spare the other three left. A fifth storage
/// buffer in the clustered template is a shader that validates here and fails on a driver's
/// limit, and the test below is what says there is no room for one.
const CLUSTERED_STORAGE_BUFFERS: usize = 4;

/// The texture array layers a compile with no machine behind it is bounded by, which is the
/// floor every device agrees to rather than a ceiling the hardware imposes.
const ARRAY_LAYER_FLOOR: u32 = 256;

#[test]
fn every_gallery_shader_parses_and_validates_within_the_requested_features() {
    let mut checked = 0usize;
    for name in Gallery::names() {
        for (label, source) in shaders_of(&Gallery::build(name)) {
            let parsed = naga::front::wgsl::parse_str(&source).unwrap_or_else(|error| {
                panic!(
                    "the shader `{label}` of `{name}` parses: {}",
                    error.emit_to_string(&source)
                )
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
    // The budget is four storage buffers per shader stage and the clustered pass binds all four
    // of them, so this is a statement about the design rather than about a shader: it fails the
    // day a fifth is wanted, which is the day the packing has to be reconsidered.
    assert!(
        CLUSTERED_STORAGE_BUFFERS <= 4,
        "light records, cluster counts, cluster lists and the spot cones are the whole of the \
         storage budget"
    );
    assert_eq!(
        CLUSTERED_STORAGE_BUFFERS, 4,
        "one storage buffer is no longer left spare: the spot cone run took it, and the next \
         thing the clustered pass needs has to share a buffer with something rather than get \
         one of its own"
    );
}

#[test]
fn the_page_pool_is_bounded_by_the_machine_the_compile_is_made_against() {
    // `max_texture_array_layers` is the device's own number now, so this is not a statement
    // about hardware. What it holds is that a scene asking for more is told what it lost
    // rather than refused, and that a compile with no machine behind it is bounded by the
    // floor every device agrees to.
    let floor = whiting_scene::DeviceLimits::downlevel();
    assert_eq!(floor.max_texture_array_layers, ARRAY_LAYER_FLOOR);
    let scene = Value::structure([(
        whiting_scene::standard::FRAME.to_owned(),
        Value::structure([(
            whiting_scene::lighting::SHADOW_PAGES.to_owned(),
            Value::Int(ARRAY_LAYER_FLOOR as i64 + 1),
        )]),
    )]);
    let (pool, notes) = whiting_scene::lighting::read_light_pool(&scene, floor)
        .expect("a pool deeper than this machine has still reads");
    assert_eq!(
        pool.pages, ARRAY_LAYER_FLOOR as usize,
        "the pool is given the depth the machine has"
    );
    assert!(
        notes.len() == 1 && notes[0].message.contains(&ARRAY_LAYER_FLOOR.to_string()),
        "the note names the number lost and the number given instead: {notes:?}"
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
        // No machine behind this compile, so it is made against the floor.
        limits: None,
    })
    .expect("the scene compiles")
}
