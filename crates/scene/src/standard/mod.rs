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
//!     CompileContext, Compiler, MESH_SCHEMA, StandardScene, STANDARD_SCENE_SCHEMA,
//!     register_standard_scene_schema,
//!     standard::example_standard_scene,
//! };
//!
//! let mut registry = Registry::new();
//! registry.put_schema(SchemaIdentifier::new(GRAPHICS_SCHEMA), graphics::graphics_schema());
//! registry.put_schema(SchemaIdentifier::new(INSTANCE_SCHEMA), graphics::instance_schema());
//! // The scene schema, and the mesh schema a scene names a mesh under.
//! register_standard_scene_schema(&mut registry);
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
//!         // The convention camera, which a scene naming no camera of its own is drawn with.
//!         camera: 0,
//!     })
//!     .expect("the scene compiles");
//!
//! // The scene has instances in three stages, so the pipeline is the three stages in order.
//! // Instances that would be drawn by identical passes share one: the two lighting instances
//! // are one pass, so the label names the group rather than an instance.
//! let passes = graphics::passes(&compiled.graphics.value).expect("the passes read");
//! let labels: Vec<&str> = passes.iter().map(|pass| pass.label().unwrap()).collect();
//! assert_eq!(
//!     labels,
//!     [
//!         "shadow/key/shadowing/triangle",
//!         "depth/depthing/triangle",
//!         "lighting/key/lighting/triangle"
//!     ]
//! );
//!
//! // A pass draws as many instances as its instance buffer has rows, so the two lighting
//! // instances are one buffer of two rows rather than two buffers or two passes.
//! let shared = compiled
//!     .instance
//!     .value
//!     .at(&definition::Path::root().key("entries").key(&format!(
//!         "{labels_tail}_instance_data",
//!         labels_tail = "lighting/key/lighting/triangle"
//!     )))
//!     .expect("the lighting instances are in one entry");
//! let bytes = shared
//!     .variant_fields()
//!     .and_then(|fields| fields.get("data"))
//!     .and_then(definition::Value::as_bytes)
//!     .expect("a buffer carries its bytes")
//!     .len();
//! // Two rows, and each row is the model matrix plus the material's custom data: sixteen
//! // floats of transform and three of tint, which is what makes the tint per instance.
//! assert_eq!(bytes, 2 * (16 + 3) * 4);
//!
//! // A pass carries declarations; the instance carries the payloads they name, under the field
//! // that leaves room for the instance to say which graphics it belongs to. The mesh is
//! // authored once, so one vertex buffer is shared by every stage that draws it.
//! let triangle = compiled
//!     .instance
//!     .value
//!     .at(&definition::Path::root().key("entries").key("triangle_vertices"))
//!     .expect("the vertex buffer is in the instance");
//! assert_eq!(triangle.variant_tag(), Some("Buffer"));
//!
//! // The instance names the graphics it provides data to, which is the one edge that ties the
//! // pair together: a renderer refuses an instance rendered against anything else rather than
//! // resolving whatever entries happen to carry those names.
//! let declared = compiled
//!     .instance
//!     .value
//!     .as_field_map()
//!     .and_then(|fields| fields.get(graphics::REFERENCING_GRAPHICS))
//!     .and_then(definition::Value::as_str);
//! assert_eq!(declared, Some("scene.graphics"));
//!
//! // The contract: applying the emitted delta to the previous output reproduces it.
//! assert!(compiled.graphics_delta.is_empty(), "a first compile has nothing to delta from");
//! let mut replayed = compiled.graphics.clone();
//! compiled
//!     .graphics_delta
//!     .apply(&mut replayed, &registry)
//!     .expect("the emitted delta applies");
//! assert_eq!(replayed.value, compiled.graphics.value);
//!
//! // A scene may name a mesh instead of writing it out, so several scenes share one mesh
//! // rather than each holding a copy of its points. The compiler reads a scene that holds
//! // everything it names, so the named mesh compiles to the same pipeline.
//! let named_mesh = example_standard_scene()
//!     .at(&definition::Path::root().key("meshes").index(0))
//!     .expect("the example has a mesh")
//!     .as_value()
//!     .clone();
//! registry
//!     .insert_definition(Definition::new(
//!         DefinitionIdentifier::new("studio.mesh"),
//!         SchemaIdentifier::new(MESH_SCHEMA),
//!         named_mesh,
//!     ))
//!     .expect("the mesh definition is new");
//!
//! let mut naming = example_standard_scene();
//! if let Some(definition::Value::List(meshes)) = naming
//!     .as_field_map_mut()
//!     .and_then(|fields| fields.get_mut("meshes"))
//! {
//!     meshes[0] =
//!         definition::Reference::External(definition::Path::root().key("studio.mesh")).to_value();
//! }
//! let naming = Definition::new(
//!     DefinitionIdentifier::new("naming.source"),
//!     SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
//!     naming,
//! );
//! naming.validate(&registry).expect("a named mesh validates");
//! let named = compiler
//!     .compile(&CompileContext {
//!         scene: &naming,
//!         previous_graphics: None,
//!         previous_instance: None,
//!         registry: &registry,
//!         camera: 0,
//!     })
//!     .expect("a scene whose mesh is named compiles");
//! let named_labels: Vec<&str> = graphics::passes(&named.graphics.value)
//!     .expect("the passes read")
//!     .iter()
//!     .map(|pass| pass.label().unwrap())
//!     .collect();
//! assert_eq!(named_labels, labels);
//!
//! // And it still takes a delta, because a path that reaches no reference means the same thing
//! // written out as it does loaded, however much the document names.
//! registry
//!     .insert_definition(naming)
//!     .expect("the naming scene is new");
//! let mut operations = std::collections::BTreeMap::new();
//! operations.insert(
//!     // Instances are held in buckets named by the stage that draws them, and a bucket is a
//!     // list, so reaching one is a key and then an index.
//!     definition::Path::root()
//!         .key("instances")
//!         .key("lighting")
//!         .index(0)
//!         .key("transform")
//!         .key("translation"),
//!     definition::PrimitiveDelta::Replace(definition::Value::List(
//!         [3.0, 0.0, 0.0]
//!             .into_iter()
//!             .map(definition::Value::Float)
//!             .collect(),
//!     )),
//! );
//! registry
//!     .apply(&definition::Delta::new(
//!         DefinitionIdentifier::new("naming.source"),
//!         0u64.into(),
//!         operations,
//!     ))
//!     .expect("a scene that names a mesh takes a delta that reaches no reference");
//! ```

pub mod camera;
mod compiler;
mod emit;
mod example;
pub mod frame;
pub mod math;
mod packing;
mod pass_schemas;
mod scene_schema;

pub use camera::{Camera, DEFAULT_FIELD_OF_VIEW, Projection, ProjectionKind, projection_value};
pub use compiler::{STANDARD_SCENE_SCHEMA, StandardScene, register_standard_scene_schema};
pub use emit::{
    COLOR_TARGET_ENTRY, DEPTH_BUFFER_ENTRY, Emitted, FULLSCREEN_INSTANCE, FULLSCREEN_MESH,
    PRESENT_TARGET_ENTRY, Stage, emit, shadow_map_entry,
};
pub use example::example_standard_scene;
pub use frame::Frame;
pub use math::{Bounds, apply as project_point, transform_matrix};
pub use pass_schemas::{
    COLOR_FORMAT, COLOR_TARGET, DEPTH_BUFFER, DEPTH_FORMAT, INSTANCE_DATA, LIGHT_COLOR,
    LIGHT_POSITION, LIGHT_PROJECTION, LIGHT_SHADOW_TEXEL, PRESENT_TARGET, PROJECTION, SHADOW_MAP,
    SHADOW_MAP_FORMAT, SHADOW_MAP_HEIGHT, SHADOW_MAP_WIDTH, TIME, VERTICES, depth_pass_schema,
    emission_pass_schema, light_pass_schema, post_pass_schema, projection_schema,
    shadow_pass_schema,
};
pub use scene_schema::{
    CAMERAS, CLUSTERING, CUSTOM_DATA, CUSTOM_SCHEMA, DATA, DEPTH_EXTENTS, EYE, FIELD_OF_VIEW,
    FORMAT, HEIGHT, HEIGHT_EXTENTS, IMAGE_SCHEMA, IMAGES, INSTANCE_SCHEMA, LABEL,
    LIGHTS_PER_CLUSTER, MATERIAL, MESH, MESH_MATERIALS, MESH_SCHEMA, PADDING, PASS_SCHEMA,
    POINT_SCHEMA, POINTS, RADIUS, SHADER, SHADERS, SOURCE, STAGE, TARGET, TOPOLOGY, TRANSFORM, UP,
    WIDTH, WIDTH_EXTENTS, camera_schema, clustering_schema, default_custom_schema,
    default_instance_schema, default_point_schema, image_schema, instance_schema, light_schema,
    mesh_material_schema, mesh_schema, shader_schema, standard_scene_schema, transform_schema,
};
