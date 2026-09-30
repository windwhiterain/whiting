//! The `StandardScene` compiler: a standard scene in, a graphics pipeline out.

use definition::{Definition, DefinitionIdentifier, Registry, SchemaIdentifier};
use graphics::{GRAPHICS_SCHEMA, INSTANCE_SCHEMA};

use crate::standard::emit;
use crate::{CompileContext, CompileError, CompileOutput, Compiler, Scene, assemble, scene_text};

/// Name of the scene schema this compiler accepts.
pub const STANDARD_SCENE_SCHEMA: &str = "whiting.scene.standard";

/// Compiles the conventional lit scene into the three stage pipeline.
#[derive(Debug, Clone)]
pub struct StandardScene {
    schema: Scene,
    graphics_identifier: DefinitionIdentifier,
    instance_identifier: DefinitionIdentifier,
}

impl StandardScene {
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

    /// Builds the scene schema: a name, a list of lights and a list of meshes.
    pub fn schema() -> Scene {
        crate::standard::standard_scene_schema()
    }
}

impl Compiler for StandardScene {
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
        // The name is part of the schema even though the pipeline does not need it, so a
        // scene that does not carry one still fails at compile time.
        let _name = scene_text(scene, "name")?;

        let emitted = emit::emit(scene)?;
        assemble(
            &self.graphics_identifier,
            &self.instance_identifier,
            Definition::new(
                self.graphics_identifier.clone(),
                SchemaIdentifier::new(GRAPHICS_SCHEMA),
                emitted.graphics,
            ),
            Definition::new(
                self.instance_identifier.clone(),
                SchemaIdentifier::new(INSTANCE_SCHEMA),
                emitted.instance,
            ),
            context,
        )
    }
}

/// Registers the scene schema this compiler accepts, so an agent can read it.
pub fn register_standard_scene_schema(registry: &mut Registry) {
    registry.put_schema(
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        StandardScene::schema(),
    );
}
