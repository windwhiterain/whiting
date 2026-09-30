//! The device state: hot reload, incremental application, and the delta cross-check.

use std::path::Path;

use definition::{
    Definition, DefinitionIdentifier, Delta, Registry, RegistryError, SchemaIdentifier,
    ValidationError, Value, Version,
};
use graphics::GraphicsError;

use crate::resolve::Stored;
use scene::{CompileContext, CompileError, Compiler};
use thiserror::Error;

use crate::render::{RenderError, RenderedFrame, Renderer, render_with};

/// The hot-reloadable device.
pub struct Device {
    registry: Registry,
    compiler: Box<dyn Compiler>,
    renderer: Box<dyn Renderer>,
    /// Whether an emitted delta is checked against a full recompile before it is applied.
    ///
    /// This is on in debug and test builds and off in release, because the check costs a
    /// second full compile and a wrong delta is a development-time defect rather than a
    /// production concern. See [cross checking emitted deltas](crate#cross-checking).
    cross_check: bool,
}

/// What a reload did.
#[derive(Debug, Clone)]
pub struct ReloadReport {
    /// The graphics output after the reload.
    pub graphics_version: Version,
    /// The instance output after the reload.
    pub instance_version: Version,
    /// Whether the graphics output was created rather than advanced.
    pub graphics_created: bool,
    /// Whether the instance output was created rather than advanced.
    pub instance_created: bool,
    /// How many operations the emitted graphics delta carried.
    pub graphics_operations: usize,
    /// How many operations the emitted instance delta carried.
    pub instance_operations: usize,
    /// Whether the cross-check ran.
    pub cross_checked: bool,
}

impl Device {
    /// Creates a device over the given compiler and renderer.
    ///
    /// Cross checking follows the build profile: on in debug and test, off in release.
    pub fn new(compiler: Box<dyn Compiler>, renderer: Box<dyn Renderer>) -> Self {
        Self {
            registry: Registry::new(),
            compiler,
            renderer,
            cross_check: cfg!(debug_assertions),
        }
    }

    /// Forces cross checking on or off, overriding the build profile default.
    pub fn with_cross_check(mut self, cross_check: bool) -> Self {
        self.cross_check = cross_check;
        self
    }

    /// Borrows the registry, for callers that read state directly.
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Mutably borrows the registry.
    pub fn registry_mut(&mut self) -> &mut Registry {
        &mut self.registry
    }

    /// Borrows the compiler.
    pub fn compiler(&self) -> &dyn Compiler {
        self.compiler.as_ref()
    }

    /// Borrows the renderer.
    pub fn renderer(&self) -> &dyn Renderer {
        self.renderer.as_ref()
    }

    /// Registers the graphics and instance schemas this device's renderer expects.
    pub fn install_graphics_schemas(&mut self) {
        self.registry.put_schema(
            SchemaIdentifier::new(graphics::GRAPHICS_SCHEMA),
            graphics::graphics_schema(),
        );
        self.registry.put_schema(
            SchemaIdentifier::new(graphics::INSTANCE_SCHEMA),
            graphics::instance_schema(),
        );
    }

    /// Recompiles a scene definition and applies the result incrementally.
    ///
    /// When an output already exists, the compiler's emitted delta is applied to it and
    /// the change is recorded in that definition's history, so the reload is undoable.
    /// The delta is not re-diffed centrally; the compiler owns that.
    pub fn reload(&mut self, source: &DefinitionIdentifier) -> Result<ReloadReport, DeviceError> {
        let scene = self.registry.definition(source)?.clone();
        let graphics_identifier = self.compiler.graphics_identifier().clone();
        let instance_identifier = self.compiler.instance_identifier().clone();

        let previous_graphics = self.registry.definition(&graphics_identifier).ok().cloned();
        let previous_instance = self.registry.definition(&instance_identifier).ok().cloned();

        let output = self.compiler.compile(&CompileContext {
            scene: &scene,
            previous_graphics: previous_graphics.as_ref(),
            previous_instance: previous_instance.as_ref(),
            registry: &self.registry,
        })?;

        if self.cross_check {
            self.verify(
                &output.graphics,
                &output.graphics_delta,
                previous_graphics.as_ref(),
            )?;
            self.verify(
                &output.instance,
                &output.instance_delta,
                previous_instance.as_ref(),
            )?;
        }

        let graphics_operations = output.graphics_delta.operations.len();
        let instance_operations = output.instance_delta.operations.len();

        // The emitted documents start at the initial version, because they are a
        // description of the whole output rather than a history entry. When an output
        // already exists the delta is what advances it, and the delta apply owns the
        // version bookkeeping.
        let (graphics_version, graphics_created) = match previous_graphics {
            Some(_) => {
                self.registry.apply(&output.graphics_delta)?;
                (
                    self.registry.definition(&graphics_identifier)?.version,
                    false,
                )
            }
            None => {
                self.registry.insert_definition(output.graphics.clone())?;
                (output.graphics.version, true)
            }
        };

        let (instance_version, instance_created) = match previous_instance {
            Some(_) => {
                self.registry.apply(&output.instance_delta)?;
                (
                    self.registry.definition(&instance_identifier)?.version,
                    false,
                )
            }
            None => {
                self.registry.insert_definition(output.instance.clone())?;
                (output.instance.version, true)
            }
        };

        Ok(ReloadReport {
            graphics_version,
            instance_version,
            graphics_created,
            instance_created,
            graphics_operations,
            instance_operations,
            cross_checked: self.cross_check,
        })
    }

    /// Checks that applying an emitted delta to the previous output reproduces the
    /// emitted output.
    fn verify(
        &self,
        emitted: &Definition,
        delta: &Delta,
        previous: Option<&Definition>,
    ) -> Result<(), DeviceError> {
        let Some(previous) = previous else {
            if delta.operations.is_empty() {
                return Ok(());
            }
            return Err(DeviceError::CrossCheck {
                output: emitted.definition_identifier.clone(),
                reason:
                    "a first compile emitted operations with no previous output to apply them to"
                        .to_owned(),
            });
        };

        let mut replayed = previous.clone();
        // A replayed delta is trusted internal state rather than an agent submission, so
        // it is re-stamped the same way the history replays are.
        let mut stamped = delta.clone();
        stamped.base_version = replayed.version;
        stamped
            .apply(&mut replayed, &self.registry)
            .map_err(|error| DeviceError::CrossCheck {
                output: emitted.definition_identifier.clone(),
                reason: match error {
                    definition::DeltaError::Validation(inner) => inner.to_string(),
                    other => other.to_string(),
                },
            })?;

        if replayed.value == emitted.value {
            return Ok(());
        }
        Err(DeviceError::CrossCheck {
            output: emitted.definition_identifier.clone(),
            reason: "applying the emitted delta does not reproduce the emitted output".to_owned(),
        })
    }

    /// Renders a stored graphics definition against the compiler's instance output.
    ///
    /// # Invariants
    ///
    /// An instance names the graphics it provides data to, and rendering it against anything
    /// else is refused rather than done: the binders address instance entries by name, so a
    /// mismatched pair would resolve to whatever happens to carry those names and produce
    /// pixels that look like a rendering of the wrong thing.
    pub fn render(
        &self,
        graphics_identifier: &DefinitionIdentifier,
    ) -> Result<RenderedFrame, DeviceError> {
        let graphics = self.registry.definition(graphics_identifier)?.clone();
        let instance_identifier = self.compiler.instance_identifier();
        let instance = self.registry.definition(instance_identifier).ok().cloned();
        let declared = instance
            .as_ref()
            .and_then(|instance| {
                instance
                    .value
                    .as_field_map()
                    .and_then(|fields| fields.get(graphics::REFERENCING_GRAPHICS))
                    .and_then(Value::as_str)
            })
            .ok_or_else(|| DeviceError::UnpairedInstance {
                instance: instance_identifier.clone(),
            })?;
        if declared != graphics_identifier.as_str() {
            return Err(DeviceError::InstancePairing {
                graphics: graphics_identifier.clone(),
                declared: declared.to_owned(),
                rendered: instance_identifier.clone(),
            });
        }

        // A document that names data in another definition can only be read once it is loaded,
        // so the renderer is handed what each reference resolves to rather than a spelling it
        // could not follow. A document that names nothing is handed over as it is.
        let stored = Stored::new(&self.registry);
        let graphics_value = stored.value(&graphics)?;
        let instance_value = match &instance {
            Some(instance) => stored.value(instance)?,
            None => Value::default(),
        };
        Ok(render_with(
            self.renderer.as_ref(),
            &graphics_value,
            &instance_value,
        )?)
    }

    /// Writes every schema and definition to a file.
    pub fn snapshot(&self, path: &Path) -> Result<(), DeviceError> {
        crate::snapshot::write(self, path)
    }

    /// Replaces the registry contents with what a snapshot file holds.
    pub fn restore(&mut self, path: &Path) -> Result<(), DeviceError> {
        crate::snapshot::read(&mut self.registry, path)
    }
}

/// A device operation failed.
#[derive(Debug, Error)]
pub enum DeviceError {
    #[error(transparent)]
    Registry(#[from] RegistryError),
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Compile(#[from] CompileError),
    #[error(transparent)]
    Render(#[from] RenderError),
    #[error(transparent)]
    Graphics(#[from] GraphicsError),
    #[error(transparent)]
    Schema(#[from] definition::SchemaError),
    #[error(transparent)]
    Load(#[from] definition::LoadError),
    #[error("the emitted delta for `{output}` failed the cross-check: {reason}")]
    CrossCheck {
        output: DefinitionIdentifier,
        reason: String,
    },
    #[error("the instance `{instance}` does not name the graphics it provides data to")]
    UnpairedInstance { instance: DefinitionIdentifier },
    #[error(
        "the instance `{rendered}` provides data to `{declared}`, and this render is of `{graphics}`"
    )]
    InstancePairing {
        graphics: DefinitionIdentifier,
        declared: String,
        rendered: DefinitionIdentifier,
    },
    #[error("the snapshot at {path} could not be read: {reason}")]
    SnapshotRead { path: String, reason: String },
    #[error("the snapshot at {path} could not be written: {reason}")]
    SnapshotWrite { path: String, reason: String },
    #[error("the request body could not be read: {0}")]
    Body(String),
}
