//! A working reference compiler, and the schema of the scene it accepts.
//!
//! This is the pattern every compiler in the workspace follows: read a scene, emit the
//! complete graphics and instance documents, and emit the deltas that reproduce them
//! incrementally. The delta logic here is deliberately simple and exact rather than
//! clever, because its job is to be a correct example of the contract, not the fastest
//! possible implementation.

use std::collections::BTreeMap;

use definition::{
    Definition, DefinitionIdentifier, Path, Registry, Schema, SchemaIdentifier, SchemaKind, Value,
};
use graphics::binders::{FORMAT, HEIGHT, RESOURCE, SOURCE, WIDTH};
use graphics::pass::{BINDERS, CLEAR_COLOR, LABEL, SHADER, TARGETS, UNIFORM};
use graphics::{GRAPHICS_SCHEMA, INSTANCE_SCHEMA};

use crate::{
    CompileContext, CompileError, CompileOutput, Compiler, Scene, assemble, malformed_scene,
    scene_list, scene_text,
};

/// Name of the instance entry every layer of the reference scene renders into.
pub const SURFACE: &str = "surface";

/// Width in pixels of the surface the reference scene renders into.
pub const SURFACE_WIDTH: i64 = 512;

/// Height in pixels of the surface the reference scene renders into.
pub const SURFACE_HEIGHT: i64 = 512;

/// Format of the surface the reference scene renders into.
pub const SURFACE_FORMAT: &str = "rgba8unorm";

/// Name of the scene schema this compiler accepts.
pub const REFERENCE_SCENE_SCHEMA: &str = "whiting.scene.reference";

/// Compiles a layered scene into one rasterize pass per layer.
#[derive(Debug, Clone)]
pub struct ReferenceScene {
    schema: Scene,
    graphics_identifier: DefinitionIdentifier,
    instance_identifier: DefinitionIdentifier,
}

impl ReferenceScene {
    /// Creates a compiler writing to the given output definitions.
    pub fn new(
        graphics_identifier: DefinitionIdentifier,
        instance_identifier: DefinitionIdentifier,
    ) -> Self {
        Self {
            schema: Self::schema(),
            graphics_identifier,
            instance_identifier,
        }
    }

    /// Builds the scene schema: a name and a list of layers.
    pub fn schema() -> Scene {
        Schema::new(SchemaKind::structure([
            ("name".to_owned(), Schema::new(SchemaKind::String)),
            (
                "layers".to_owned(),
                Schema::new(SchemaKind::list(Schema::new(SchemaKind::enumeration([(
                    "Layer".to_owned(),
                    Schema::new(SchemaKind::structure([
                        ("label".to_owned(), Schema::new(SchemaKind::String)),
                        ("shader".to_owned(), Schema::new(SchemaKind::String)),
                        (
                            "clear_color".to_owned(),
                            Schema::new(SchemaKind::list(Schema::new(SchemaKind::Float))),
                        ),
                    ])),
                )]))))
                .with_documentation("One rasterize pass is emitted per layer, in order."),
            ),
        ]))
        .with_documentation("A layered 2d scene.")
    }

    fn passes_value(scene: &Value) -> Result<Vec<Value>, CompileError> {
        let layers = scene_list(scene, "layers")?;
        layers
            .iter()
            .map(|layer| {
                let path = Path::root().key("layers");
                let Some(fields) = layer.variant_fields() else {
                    return Err(malformed_scene(&path, "a layer", Some(layer)));
                };
                let text = |name: &str| -> Result<String, CompileError> {
                    match fields.get(name) {
                        Some(Value::String(value)) => Ok(value.clone()),
                        other => Err(malformed_scene(&path.key(name), "a string", other)),
                    }
                };
                let clear_color = match fields.get("clear_color") {
                    Some(Value::List(components)) => Value::List(components.clone()),
                    other => {
                        return Err(malformed_scene(
                            &path.key("clear_color"),
                            "a list of floats",
                            other,
                        ));
                    }
                };
                let label = text("label")?;
                Ok(Value::variant(
                    "Rasterize",
                    [
                        (LABEL.to_owned(), Value::String(label)),
                        // The clear colour is uniform data, so it is checked against the
                        // interface of the shader that reads it.
                        (
                            UNIFORM.to_owned(),
                            Value::structure([("clear_color".to_owned(), clear_color)]),
                        ),
                        (SHADER.to_owned(), Value::String(text("shader")?)),
                        (
                            BINDERS.to_owned(),
                            Value::dictionary([(
                                SURFACE.to_owned(),
                                Value::variant(
                                    "Resource",
                                    [(
                                        RESOURCE.to_owned(),
                                        Value::variant(
                                            "Texture",
                                            [
                                                (
                                                    SOURCE.to_owned(),
                                                    Value::String(SURFACE.to_owned()),
                                                ),
                                                (WIDTH.to_owned(), Value::Int(SURFACE_WIDTH)),
                                                (HEIGHT.to_owned(), Value::Int(SURFACE_HEIGHT)),
                                                (
                                                    FORMAT.to_owned(),
                                                    Value::String(SURFACE_FORMAT.to_owned()),
                                                ),
                                            ],
                                        ),
                                    )],
                                ),
                            )]),
                        ),
                        (
                            TARGETS.to_owned(),
                            Value::List(vec![Value::String(SURFACE.to_owned())]),
                        ),
                        (
                            CLEAR_COLOR.to_owned(),
                            Value::List(
                                [0.0, 0.0, 0.0, 0.0].into_iter().map(Value::Float).collect(),
                            ),
                        ),
                    ],
                ))
            })
            .collect()
    }

    fn instance_value(passes: &[Value]) -> Value {
        let mut entries = BTreeMap::new();
        entries.insert(
            SURFACE.to_owned(),
            Value::variant(
                "Texture",
                [
                    (WIDTH.to_owned(), Value::Int(SURFACE_WIDTH)),
                    (HEIGHT.to_owned(), Value::Int(SURFACE_HEIGHT)),
                    (FORMAT.to_owned(), Value::String(SURFACE_FORMAT.to_owned())),
                ],
            ),
        );
        for pass in passes {
            let Some(fields) = pass.variant_fields() else {
                continue;
            };
            let Some(name) = fields.get(SHADER).and_then(Value::as_str) else {
                continue;
            };
            entries.insert(
                name.to_owned(),
                Value::variant(
                    "Shader",
                    [
                        (
                            graphics::binders::SOURCE.to_owned(),
                            Value::String(format!("// the reference shader for {name}")),
                        ),
                        (
                            graphics::binders::SCHEMA.to_owned(),
                            ReferenceScene::interface(),
                        ),
                    ],
                ),
            );
        }
        Value::Dict(entries)
    }

    /// The interface every reference shader declares.
    ///
    /// A reference layer reads a clear colour and renders into the surface, and nothing
    /// else. Writing the interface out here rather than leaving it out is the point of
    /// the example: the pass is checked against it at compile time.
    fn interface() -> Value {
        Schema::new(SchemaKind::structure([
            (
                UNIFORM.to_owned(),
                Schema::new(SchemaKind::structure([(
                    "clear_color".to_owned(),
                    Schema::new(SchemaKind::list(Schema::new(SchemaKind::Float))),
                )]))
                .with_documentation("Uniform data."),
            ),
            (
                BINDERS.to_owned(),
                Schema::new(SchemaKind::structure([(
                    SURFACE.to_owned(),
                    graphics::texture_binding_schema(),
                )]))
                .with_documentation("What the pass binds, keyed by binding point."),
            ),
        ]))
        .to_value()
    }
}

impl Compiler for ReferenceScene {
    fn scene_schema(&self) -> &Scene {
        &self.schema
    }

    fn graphics_identifier(&self) -> &DefinitionIdentifier {
        &self.graphics_identifier
    }

    fn instance_identifier(&self) -> &DefinitionIdentifier {
        &self.instance_identifier
    }

    fn compile(&self, context: &CompileContext<'_>) -> Result<CompileOutput, CompileError> {
        let scene = context.scene_value();
        // Reading the name is part of the contract even though the reference compiler does
        // not need it, so a malformed scene still fails at compile time.
        let _name = scene_text(scene, "name")?;

        let passes = ReferenceScene::passes_value(scene)?;
        let instance = ReferenceScene::instance_value(&passes);
        // The instance schema is derived from the emitted declarations rather than
        // authored, so the two documents cannot disagree about what an entry is.
        let declared = Value::structure([(
            graphics::PASSES_FIELD.to_owned(),
            Value::List(passes.clone()),
        )]);
        let instance_schema = graphics::derived_instance_schema(&declared)
            .map_err(|error| CompileError::Emission {
                reason: error.to_string(),
            })?
            .to_value();
        let graphics = Value::structure([
            (graphics::PASSES_FIELD.to_owned(), Value::List(passes)),
            (graphics::INSTANCE_SCHEMA_FIELD.to_owned(), instance_schema),
        ]);

        assemble(
            &self.graphics_identifier,
            &self.instance_identifier,
            Definition::new(
                self.graphics_identifier.clone(),
                SchemaIdentifier::new(GRAPHICS_SCHEMA),
                graphics,
            ),
            Definition::new(
                self.instance_identifier.clone(),
                SchemaIdentifier::new(INSTANCE_SCHEMA),
                instance,
            ),
            context,
        )
    }
}

/// Registers the scene schema this compiler accepts, so an agent can read it.
pub fn register_scene_schema(registry: &mut Registry) {
    registry.put_schema(
        definition::SchemaIdentifier::new(REFERENCE_SCENE_SCHEMA),
        ReferenceScene::schema(),
    );
}
