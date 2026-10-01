//! The `StandardScene` compiler: a standard scene in, a graphics pipeline out.

use definition::{Definition, DefinitionIdentifier, Registry, SchemaIdentifier};
use graphics::{GRAPHICS_SCHEMA, INSTANCE_SCHEMA};

use crate::standard::DebugChannel;
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
    /// The extra images every compile of this compiler is asked to draw.
    ///
    /// # Invariants
    ///
    /// It is a field of the compiler rather than an argument of [`Compiler::compile`], because a
    /// caller that changed it between two compiles of one scene would emit two documents that
    /// disagree, and the device's cross check recompiles with the compiler it holds. A channel is
    /// a property of the *pipeline being compiled*, not of one moment of it.
    debug: Vec<DebugChannel>,
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
            debug: Vec::new(),
        }
    }

    /// Returns this compiler, asked to draw the given debug images beside the frame.
    ///
    /// # Invariants
    ///
    /// A channel is an image *about* the frame rather than of it: every one writes a target of
    /// its own, so asking for one cannot move a pixel of the picture, and the emitted documents
    /// say which image is the frame rather than leaving it to be whichever pass wrote last.
    pub fn with_debug(mut self, channels: impl IntoIterator<Item = DebugChannel>) -> Self {
        self.debug = channels.into_iter().collect();
        self
    }

    /// Returns the debug channels this compiler draws.
    pub fn debug(&self) -> &[DebugChannel] {
        &self.debug
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
        let scene = &context.scene_value()?;
        // The name is part of the schema even though the pipeline does not need it, so a
        // scene that does not carry one still fails at compile time.
        let _name = scene_text(scene, "name")?;

        let emitted = emit::emit(scene, context.camera, context.device_limits(), &self.debug)?;
        let output = assemble(
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
            // What the emitter could not do exactly — a cluster grid that had to drop lights —
            // travels with the output rather than being logged here, because a compile has no
            // console and a dropped light is a fact the author of the scene has to have.
            emitted.notes,
            context,
        );
        output
    }
}

/// Registers the scene schema this compiler accepts, so an agent can read it.
///
/// # Invariants
///
/// A scene may name a mesh or an image that lives in another definition, and that definition is
/// read with the mesh or image schema, so both are registered under their conventional
/// identifiers by one call: a client that registers only the scene schema would compile a scene
/// until the moment it named one, and then fail on a schema it never knew it had to provide.
pub fn register_standard_scene_schema(registry: &mut Registry) {
    registry.put_schema(
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        StandardScene::schema(),
    );
    registry.put_schema(
        SchemaIdentifier::new(crate::standard::MESH_SCHEMA),
        crate::standard::mesh_schema(),
    );
    registry.put_schema(
        SchemaIdentifier::new(crate::standard::IMAGE_SCHEMA),
        crate::standard::image_schema(),
    );
}
