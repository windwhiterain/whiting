//! A working reference compiler, and the schema of the scene it accepts.
//!
//! This is the pattern every compiler in the workspace follows: read a scene, emit the
//! complete graphics and instance documents, and emit the deltas that reproduce them
//! incrementally. The delta logic here is deliberately simple and exact rather than
//! clever, because its job is to be a correct example of the contract, not the fastest
//! possible implementation.

use std::collections::BTreeMap;

use definition::{
    Definition, DefinitionIdentifier, Delta, Path, PrimitiveDelta, Registry, Schema, SchemaKind,
    Value, Version,
};
use graphics::{GRAPHICS_SCHEMA, INSTANCE_SCHEMA};

use crate::{
    CompileContext, CompileError, CompileOutput, Compiler, Scene, malformed_scene, scene_list,
    scene_text,
};

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
                Ok(Value::variant(
                    "Rasterize",
                    [
                        ("label".to_owned(), Value::String(text("label")?)),
                        // Both stages bind the same shader name; the renderer resolves the
                        // stage specific entry point from the bound shader.
                        ("vertex_shader".to_owned(), Value::String(text("shader")?)),
                        ("fragment_shader".to_owned(), Value::String(text("shader")?)),
                        (
                            "targets".to_owned(),
                            Value::List(vec![Value::String("rgba8unorm".to_owned())]),
                        ),
                        ("clear_color".to_owned(), clear_color),
                    ],
                ))
            })
            .collect()
    }

    fn instance_value(passes: &[Value]) -> Value {
        let entries = passes
            .iter()
            .filter_map(|pass| {
                let _tag = pass.variant_tag()?;
                let fields = pass.variant_fields()?;
                let name = fields.get("vertex_shader")?.as_str()?.to_owned();
                let shader = Value::variant(
                    "Shader",
                    [
                        (
                            "path".to_owned(),
                            Value::String(format!("shaders/{name}.wgsl")),
                        ),
                        ("entry_point".to_owned(), Value::String("main".to_owned())),
                    ],
                );
                Some((name, shader))
            })
            .collect();
        Value::Dict(entries)
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
        let graphics_value = Value::structure([
            ("passes".to_owned(), Value::List(passes.clone())),
            ("instance_schema".to_owned(), Value::Unit),
        ]);
        let instance_value = ReferenceScene::instance_value(&passes);

        let graphics = Definition::new(
            self.graphics_identifier.clone(),
            definition::SchemaIdentifier::new(GRAPHICS_SCHEMA),
            graphics_value.clone(),
        );
        let instance = Definition::new(
            self.instance_identifier.clone(),
            definition::SchemaIdentifier::new(INSTANCE_SCHEMA),
            instance_value,
        );

        // The emitted outputs are verified against their schemas here, so a compiler bug
        // is reported as a compile error rather than surfacing later at render time.
        context
            .registry
            .schema(&graphics.schema_identifier)
            .expect("the graphics schema is registered")
            .validate(&graphics.value)?;
        context
            .registry
            .schema(&instance.schema_identifier)
            .expect("the instance schema is registered")
            .validate(&instance.value)?;

        // The emitted delta applies to the previous output, so it carries that output's
        // version, not the version of the freshly built document.
        let graphics_base = context
            .previous_graphics
            .map_or(Version::INITIAL, |definition| definition.version);
        let instance_base = context
            .previous_instance
            .map_or(Version::INITIAL, |definition| definition.version);

        let previous_graphics = context
            .previous_graphics
            .map(|definition| &definition.value);
        let previous_instance = context
            .previous_instance
            .map(|definition| &definition.value);

        let graphics_delta = Delta::new(
            self.graphics_identifier.clone(),
            graphics_base,
            list_delta(
                &Path::root().key("passes"),
                previous_graphics.and_then(|value| value.at(&Path::root().key("passes"))),
                graphics_value.at(&Path::root().key("passes")),
            ),
        );
        let instance_delta = Delta::new(
            self.instance_identifier.clone(),
            instance_base,
            dict_delta(&Path::root(), previous_instance, Some(&instance.value)),
        );

        Ok(CompileOutput {
            graphics,
            instance,
            graphics_delta,
            instance_delta,
        })
    }
}

/// Emits the operations that turn `previous` into `next` for a list at `path`.
///
/// A whole-value replace is used when a list changes length, because the operations map
/// holds at most one primitive per path and a length change needs both a removal and an
/// append at the same path.
fn list_delta(
    path: &Path,
    previous: Option<&Value>,
    next: Option<&Value>,
) -> BTreeMap<Path, PrimitiveDelta> {
    let mut operations = BTreeMap::new();
    let (Some(Value::List(before)), Some(Value::List(after))) = (previous, next) else {
        return operations;
    };

    if before.len() != after.len() {
        operations.insert(
            path.clone(),
            PrimitiveDelta::Replace(Value::List(after.clone())),
        );
        return operations;
    }

    for (index, (old, new)) in before.iter().zip(after.iter()).enumerate() {
        if old != new {
            operations.insert(
                path.clone().index(index),
                PrimitiveDelta::Replace(new.clone()),
            );
        }
    }
    operations
}

/// Emits the operations that turn `previous` into `next` for a dict at `path`.
///
/// A dict that both gains and loses entries is replaced wholesale, for the same
/// one-primitive-per-path reason as [`list_delta`].
fn dict_delta(
    path: &Path,
    previous: Option<&Value>,
    next: Option<&Value>,
) -> BTreeMap<Path, PrimitiveDelta> {
    let mut operations = BTreeMap::new();
    let (Some(before), Some(after)) = (previous, next) else {
        return operations;
    };
    let (Some(before), Some(after)) = (before.as_dict(), after.as_dict()) else {
        return operations;
    };

    let removed: Vec<String> = before
        .keys()
        .filter(|name| !after.contains_key(*name))
        .cloned()
        .collect();
    let added: Vec<(String, Value)> = after
        .iter()
        .filter(|(name, _)| !before.contains_key(*name))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();

    if !removed.is_empty() && !added.is_empty() {
        operations.insert(
            path.clone(),
            PrimitiveDelta::Replace(Value::Dict(after.clone())),
        );
        return operations;
    }
    if !removed.is_empty() {
        operations.insert(path.clone(), PrimitiveDelta::RemoveKeys { keys: removed });
    }
    if !added.is_empty() {
        operations.insert(
            path.clone(),
            PrimitiveDelta::AppendEntries { entries: added },
        );
    }
    for (name, value) in after {
        if before.get(name).is_some_and(|old| old != value) {
            operations.insert(
                path.clone().key(name),
                PrimitiveDelta::Replace(value.clone()),
            );
        }
    }
    operations
}

/// Registers the scene schema this compiler accepts, so an agent can read it.
pub fn register_scene_schema(registry: &mut Registry) {
    registry.put_schema(
        definition::SchemaIdentifier::new(REFERENCE_SCENE_SCHEMA),
        ReferenceScene::schema(),
    );
}
