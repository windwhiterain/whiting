//! The conventional lit scene: its schema, the passes it needs, and a compiler.
//!
//! # The idea
//!
//! A scene says what is in it — some lights, some meshes — and nothing about how it is
//! drawn. The compiler works out the pipeline: a shadow map per light, a depth buffer,
//! then the shaded image, in that order, and emits the passes and the instance entries
//! that run it. Adding a light adds a pass; adding a mesh adds a pass; the two documents
//! carry declarations on one side and payloads on the other, so a reload of the scene
//! usually rewrites a handful of paths rather than the whole instruction list.
//!
//! # The three stages
//!
//! - `ShadowingShader` renders one light's depth map, and the mesh is a shadow caster.
//! - `DepthShader` renders the depth of the scene.
//! - `LightingShader` shades the mesh against the depth and the shadow maps.
//!
//! A material's shader names the stage, and a mesh is drawn in the stage it names. A
//! mesh that must appear in more than one stage is listed once per stage, because a
//! material is where the shader lives and a shader is where the stage lives.
//!
//! # What a shader has to declare
//!
//! A shader is the only thing that knows what it reads, so it declares the shape of the
//! pass it is bound to, of the instance data, of a point, and of its own custom data.
//! All four are checked before the compiler returns, so a shader that outgrows its mesh
//! is a compile error rather than a black triangle.
//!
//! # Example
//!
//! ```
//! use definition::{Definition, DefinitionIdentifier, Registry, SchemaIdentifier};
//! use graphics::{GRAPHICS_SCHEMA, INSTANCE_SCHEMA};
//! use whiting_scene::{
//!     CompileContext, Compiler, StandardScene, STANDARD_SCENE_SCHEMA,
//!     standard::example_standard_scene,
//! };
//!
//! let mut registry = Registry::new();
//! registry.put_schema(SchemaIdentifier::new(GRAPHICS_SCHEMA), graphics::graphics_schema());
//! registry.put_schema(SchemaIdentifier::new(INSTANCE_SCHEMA), graphics::instance_schema());
//! registry.put_schema(
//!     SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
//!     StandardScene::schema(),
//! );
//!
//! let compiler = StandardScene::new(
//!     DefinitionIdentifier::new("scene.graphics"),
//!     DefinitionIdentifier::new("scene.instance"),
//! );
//!
//! let scene = Definition::new(
//!     DefinitionIdentifier::new("scene.source"),
//!     SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
//!     example_standard_scene(),
//! );
//! scene.validate(&registry).expect("the scene validates");
//!
//! let compiled = compiler
//!     .compile(&CompileContext {
//!         scene: &scene,
//!         previous_graphics: None,
//!         previous_instance: None,
//!         registry: &registry,
//!     })
//!     .expect("the scene compiles");
//!
//! // The scene named one mesh per stage, so the pipeline is the three stages in order.
//! let passes = graphics::passes(&compiled.graphics.value).expect("the passes read");
//! let labels: Vec<&str> = passes.iter().map(|pass| pass.label().unwrap()).collect();
//! assert_eq!(labels, ["shadow/key/caster", "depth/occluder", "lighting/quad/key"]);
//!
//! // A pass carries declarations; the instance carries the payloads they name.
//! let quad_vertices = compiled
//!     .instance
//!     .value
//!     .at(&definition::Path::root().key("quad_vertices"))
//!     .expect("the vertex buffer is in the instance");
//! assert_eq!(quad_vertices.variant_tag(), Some("Buffer"));
//!
//! // The contract: applying the emitted delta to the previous output reproduces it.
//! assert!(compiled.graphics_delta.is_empty(), "a first compile has nothing to delta from");
//! let mut replayed = compiled.graphics.clone();
//! compiled
//!     .graphics_delta
//!     .apply(&mut replayed, &registry)
//!     .expect("the emitted delta applies");
//! assert_eq!(replayed.value, compiled.graphics.value);
//! ```

mod compiler;
mod emit;
mod example;
pub mod math;
mod packing;
mod pass_schemas;
mod scene_schema;

pub use compiler::{STANDARD_SCENE_SCHEMA, StandardScene, register_standard_scene_schema};
pub use emit::{COLOR_TARGET_ENTRY, DEPTH_BUFFER_ENTRY, Emitted, Stage, emit, shadow_map_entry};
pub use example::example_standard_scene;
pub use math::{Bounds, apply as project_point};
pub use pass_schemas::{
    COLOR_FORMAT, COLOR_HEIGHT, COLOR_TARGET, COLOR_WIDTH, DEPTH_BUFFER, DEPTH_FORMAT,
    DEPTH_HEIGHT, DEPTH_WIDTH, INSTANCE_DATA, LIGHT_COLOR, LIGHT_POSITION, PROJECTION, SHADOW_MAP,
    SHADOW_MAP_FORMAT, SHADOW_MAP_HEIGHT, SHADOW_MAP_WIDTH, VERTICES, depthing_pass_schema,
    lighting_pass_schema, projection_schema, shadowing_pass_schema,
};
pub use scene_schema::{
    default_custom_schema, default_instance_schema, default_point_schema, light_schema,
    material_schema, mesh_schema, shader_schema, standard_scene_schema,
};
