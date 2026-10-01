//! Compilers that turn a scene into graphics instructions, incrementally.
//!
//! A compiler is a pure function from a scene document to a graphics document and a
//! matching instance document. There is no scene type: a [`Scene`] is a [`Schema`]
//! document the compiler declares, and the scene itself is an ordinary [`Value`] that
//! the registry verifies against it.
//!
//! # The incremental contract
//!
//! A compiler emits its own [`Delta`] alongside its full outputs. It is not re-diffed
//! centrally. That places the burden on the compiler, and the contract is therefore
//! explicit: applying the emitted delta to the previous output must yield exactly the
//! emitted output.
//!
//! The cross-check lives in the device crate, which performs a full recompile in debug
//! and test builds and compares. In release the emitted delta is trusted, because paying
//! a full compile on every reload would defeat the point of compiling incrementally.
//!
//! # Example
//!
//! ```
//! use definition::{RegistryStore,Definition, DefinitionIdentifier, Registry, SchemaIdentifier, Value};
//! use whiting_scene::{CompileContext, Compiler, ReferenceScene, REFERENCE_SCENE_SCHEMA};
//! use graphics::{GRAPHICS_SCHEMA, INSTANCE_SCHEMA};
//!
//! let mut registry = Registry::new();
//! registry.put_schema(SchemaIdentifier::new(GRAPHICS_SCHEMA), graphics::graphics_schema());
//! registry.put_schema(SchemaIdentifier::new(INSTANCE_SCHEMA), graphics::instance_schema());
//!
//! let compiler = ReferenceScene::new(
//!     DefinitionIdentifier::new("scene.graphics"),
//!     DefinitionIdentifier::new("scene.instance"),
//! );
//! registry.put_schema(SchemaIdentifier::from(REFERENCE_SCENE_SCHEMA), compiler.scene_schema().clone());
//!
//! let scene = Definition::new(
//!     DefinitionIdentifier::new("scene.source"),
//!     SchemaIdentifier::from(REFERENCE_SCENE_SCHEMA),
//!     Value::structure([
//!         ("name".to_owned(), Value::String("backdrop".to_owned())),
//!         ("layers".to_owned(), Value::List(vec![Value::variant("Layer", [
//!             ("label".to_owned(), Value::String("sky".to_owned())),
//!             ("shader".to_owned(), Value::String("sky".to_owned())),
//!             ("clear_color".to_owned(), Value::List(
//!                 [0.1, 0.2, 0.3, 1.0].into_iter().map(Value::Float).collect(),
//!             )),
//!         ])])),
//!     ]),
//! );
//! scene.validate(&registry).expect("the scene validates");
//!
//! // A first compile has no previous output, so it emits empty deltas.
//! let first = compiler
//!     .compile(&CompileContext {
//!         scene: &scene,
//!         previous_graphics: None,
//!         previous_instance: None,
//!         registry: &registry,
//!         // The convention camera, which a scene naming no camera of its own is drawn with.
//!         camera: 0,
//!     })
//!     .expect("the scene compiles");
//! assert!(first.graphics_delta.is_empty());
//! assert_eq!(first.graphics.value.at(&definition::Path::root().key("passes")).map(|v| v.as_list().map(<[Value]>::len)), Some(Some(1)));
//!
//! // The contract: applying the emitted delta to the previous output reproduces it.
//! let mut replayed = first.graphics.clone();
//! let _ = first.graphics_delta.apply(&mut replayed, &registry);
//! assert_eq!(replayed.value, first.graphics.value);
//!
//! // A second compile against the first output emits a non-empty delta.
//! let second = compiler
//!     .compile(&CompileContext {
//!         scene: &scene,
//!         previous_graphics: Some(&first.graphics),
//!         previous_instance: Some(&first.instance),
//!         registry: &registry,
//!         camera: 0,
//!     })
//!     .expect("the scene compiles");
//! assert!(second.graphics_delta.is_empty(), "nothing changed, so nothing to emit");
//!
//! let mut replayed = first.graphics.clone();
//! replayed.version = first.graphics.version;
//! let applied = second.graphics_delta.apply(&mut replayed, &registry);
//! assert!(applied.is_ok(), "the emitted delta applies to the previous output");
//! assert_eq!(replayed.value, second.graphics.value);
//! ```

use std::collections::BTreeMap;

use definition::{
    Definition, DefinitionIdentifier, Delta, Path, PrimitiveDelta, Registry, RegistryStore, Schema,
    Value, Version,
};
use thiserror::Error;

mod diff;
pub mod gallery;
pub mod lighting;
mod reference;
pub mod standard;

pub use reference::{REFERENCE_SCENE_SCHEMA, ReferenceScene, register_scene_schema};
pub use standard::{
    CAMERAS, MESH_SCHEMA, STANDARD_SCENE_SCHEMA, StandardScene, register_standard_scene_schema,
    standard_scene_schema,
};

/// Turns two emitted documents into a compile output.
///
/// Returns an instance document that names the graphics it provides data to.
///
/// # Invariants
///
/// An instance is a struct, so naming its graphics is a field of it. One that is not a struct
/// cannot name one, which is reported rather than passed on, because a renderer refuses an
/// instance that does not say which graphics it belongs to.
fn with_referencing_graphics(
    mut instance: Definition,
    graphics_identifier: &DefinitionIdentifier,
) -> Result<Definition, CompileError> {
    let Some(fields) = instance.value.as_field_map_mut() else {
        return Err(CompileError::Emission {
            reason: "the emitted instance document is not a struct, so it cannot name the \
                     graphics it provides data to"
                .to_owned(),
        });
    };
    fields.insert(
        graphics::REFERENCING_GRAPHICS.to_owned(),
        Value::String(graphics_identifier.to_string()),
    );
    Ok(instance)
}

/// The emitted documents are verified against their registered schemas here, so a
/// compiler bug is reported as a compile error rather than surfacing later at render
/// time, and the deltas are the diff against the previous output rather than a whole
/// document replace.
pub(crate) fn assemble(
    graphics_identifier: &DefinitionIdentifier,
    instance_identifier: &DefinitionIdentifier,
    graphics: Definition,
    instance: Definition,
    notes: Vec<CompileNote>,
    context: &CompileContext<'_>,
) -> Result<CompileOutput, CompileError> {
    // The instance names the graphics it provides data to, which is the one edge that ties a
    // pair together. It is written here rather than by the compiler because it is the
    // identifier the graphics output is stored under, which only assembling knows; a renderer
    // refuses a pair that disagrees.
    let instance = with_referencing_graphics(instance, graphics_identifier)?;
    let registry = context.registry;
    registry
        .schema(&graphics.schema_identifier)
        .map_err(|_| CompileError::UnregisteredSchema {
            schema: graphics.schema_identifier.to_string(),
        })?
        .validate(&graphics.value)?;
    registry
        .schema(&instance.schema_identifier)
        .map_err(|_| CompileError::UnregisteredSchema {
            schema: instance.schema_identifier.to_string(),
        })?
        .validate(&instance.value)?;

    // An emitted delta applies to the previous output, so it carries that output's
    // version, not the version of the freshly built document.
    let graphics_base = context
        .previous_graphics
        .map_or(Version::INITIAL, |previous| previous.version);
    let instance_base = context
        .previous_instance
        .map_or(Version::INITIAL, |previous| previous.version);
    let previous_graphics = context.previous_graphics.map(|previous| &previous.value);
    let previous_instance = context.previous_instance.map(|previous| &previous.value);
    let graphics_delta = Delta::new(
        graphics_identifier.clone(),
        graphics_base,
        diff::document_delta(previous_graphics, &graphics.value),
    );
    let instance_delta = Delta::new(
        instance_identifier.clone(),
        instance_base,
        diff::document_delta(previous_instance, &instance.value),
    );

    Ok(CompileOutput {
        graphics_delta,
        instance_delta,
        graphics,
        instance,
        notes,
    })
}
/// The schema describing the shape a [`Compiler`] accepts.
///
/// This is a schema document, not a rust type, so a compiler can declare a new scene
/// shape and have it take effect with no recompilation of this crate.
pub type Scene = Schema;

/// The inputs of a compile.
pub struct CompileContext<'registry> {
    /// The current scene document.
    pub scene: &'registry Definition,
    /// The graphics output of the previous compile, or `None` on a first compile.
    pub previous_graphics: Option<&'registry Definition>,
    /// The instance output of the previous compile, or `None` on a first compile.
    pub previous_instance: Option<&'registry Definition>,
    /// The registry the scene and the outputs are resolved against.
    pub registry: &'registry Registry,
    /// Which of the scene's cameras this compile draws through.
    ///
    /// # Why the camera is an input rather than an output
    ///
    /// A scene may name several cameras, and every one of them is rendered. A camera decides
    /// the projection every pass draws through and, for a fitted camera, how far back the eye
    /// stands — so it changes the emitted documents rather than being something a renderer can
    /// choose afterwards. A compile is therefore a compile **for one camera**, and a scene with
    /// three cameras is compiled and rendered three times.
    ///
    /// An index no camera is at is the convention camera, so a scene that names none is
    /// compiled once and drawn by the stated convention as it always was.
    pub camera: usize,
}

impl<'registry> CompileContext<'registry> {
    /// Reads the scene: the document the compiler works from, with its references resolved.
    ///
    /// # Invariants
    ///
    /// A scene may name something that lives in another definition — a mesh shared by several
    /// scenes, say — so the document is written out before a compiler reads it. What a compiler
    /// sees is therefore always a scene that holds everything it names, however the author spread
    /// it out, and a compiler never has to know which parts were named rather than written.
    pub fn scene_value(&self) -> Result<Value, CompileError> {
        RegistryStore::new(self.registry)
            .value(self.scene)
            .map_err(|reason| CompileError::UnreadableScene {
                reason: reason.to_string(),
            })
    }
}

/// Something a compile wants the author of the documents to know about.
///
/// # Why this is not an error
///
/// An error stops the compile, and a compile that stops is a scene that is not drawn at all.
/// Some things a compiler learns are not reasons to refuse: a scene lit less than it was
/// drawn is still worth drawing, provided the author is told, because a warning nobody sees
/// is the same as no warning. A note is that middle ground — the compile finishes, and the
/// count of what it dropped travels with the output for a host to print.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileNote {
    /// The scene field the note is about, so it points at the number to change.
    pub path: Path,
    /// What happened, in a sentence.
    pub message: String,
}

/// The outputs of a compile.
pub struct CompileOutput {
    /// The complete new graphics document.
    pub graphics: Definition,
    /// The complete new instance document.
    pub instance: Definition,
    /// The compiler's own delta for the graphics output.
    ///
    /// # Invariant
    ///
    /// Applying this to `previous_graphics` must yield exactly `graphics`.
    pub graphics_delta: Delta,
    /// The compiler's own delta for the instance output.
    ///
    /// # Invariant
    ///
    /// Applying this to `previous_instance` must yield exactly `instance`.
    pub instance_delta: Delta,
    /// What the compiler wants the author to know about the output it produced.
    ///
    /// # Invariant
    ///
    /// A compile that dropped something says so here rather than in the documents: the
    /// documents hold what a shader reads, and a fact about what a shader cannot read is not
    /// theirs to carry. A host that ignores this field renders a scene that is quietly wrong,
    /// which is why a compiler that may drop something fills it.
    pub notes: Vec<CompileNote>,
}

impl CompileOutput {
    /// Builds a first-compile output, where there is no previous state to delta from.
    pub fn initial(graphics: Definition, instance: Definition) -> Self {
        let graphics_delta = Delta::new(
            graphics.definition_identifier.clone(),
            Version::new(0),
            BTreeMap::new(),
        );
        let instance_delta = Delta::new(
            instance.definition_identifier.clone(),
            Version::new(0),
            BTreeMap::new(),
        );
        Self {
            graphics,
            instance,
            graphics_delta,
            instance_delta,
            notes: Vec::new(),
        }
    }
}

/// Compiles a scene into a graphics document and a matching instance document.
///
/// The trait is `Send` and `Sync` because the device holds a compiler behind a shared
/// lock and serves it from several request threads.
pub trait Compiler: Send + Sync {
    /// The schema this compiler accepts, describing the shape of a scene.
    fn scene_schema(&self) -> &Scene;

    /// The definition the graphics output is stored under.
    ///
    /// The device needs this to find the previous output and to store the new one.
    fn graphics_identifier(&self) -> &DefinitionIdentifier;

    /// The definition the instance output is stored under.
    fn instance_identifier(&self) -> &DefinitionIdentifier;

    /// Compiles the scene, emitting the deltas that reproduce the outputs incrementally.
    fn compile(&self, context: &CompileContext<'_>) -> Result<CompileOutput, CompileError>;
}

/// A compile failed, or emitted a delta that does not reproduce its own output.
#[derive(Debug, Error)]
pub enum CompileError {
    #[error("the scene could not be read: {reason}")]
    UnreadableScene { reason: String },
    #[error("the scene field at {path} is not {expected}, found {found}")]
    MalformedScene {
        path: Path,
        expected: &'static str,
        found: &'static str,
    },
    #[error("a graphics binder name is empty")]
    EmptyBinderName,
    #[error("the previous output is missing but the compiler was asked to delta against it")]
    MissingPreviousOutput,
    #[error("the schema `{schema}` is not registered, so the emitted output cannot be checked")]
    UnregisteredSchema { schema: String },
    #[error("at {path}: the schema document could not be read: {reason}")]
    UnreadableSchema { path: Path, reason: String },
    #[error("at {path}: {subject} does not satisfy {against}: {reason}")]
    Verification {
        path: Path,
        subject: String,
        against: String,
        reason: String,
    },
    #[error("at {path}: the data cannot be packed into a buffer: {reason}")]
    Unpackable { path: Path, reason: String },
    #[error("at {path}: `{stage}` is not a stage a standard material is drawn in")]
    UnknownStage { path: Path, stage: String },
    #[error("at {path}: `{topology}` is not a topology a mesh is drawn as")]
    UnknownTopology { path: Path, topology: String },
    #[error(
        "at {path}: the light `{label}` names no kind, and a flat light is not a point one. \
         Say whether it is a Point, a Spot, a Directional or an Environment light."
    )]
    MissingLightKind { path: Path, label: String },
    #[error(
        "at {path}: the light `{label}` is a {kind} light, and only a Point light is emitted \
         at this stage. A light the renderer cannot draw is refused rather than drawn as a \
         point, which would be a wrong image nobody sees until the render finishes."
    )]
    UnsupportedLightKind {
        path: Path,
        label: String,
        kind: String,
    },
    #[error("at {path}: no mesh in this scene is labelled `{mesh}`")]
    UnknownMesh { path: Path, mesh: String },
    #[error("at {path}: no shader in this scene is labelled `{shader}`")]
    UnknownShader { path: Path, shader: String },
    #[error("at {path}: no mesh material in this scene is labelled `{material}`")]
    UnknownMaterial { path: Path, material: String },
    #[error("at {path}: `{material}` names the image `{image}`, which this scene does not declare")]
    UnknownImage {
        path: Path,
        material: String,
        image: String,
    },
    #[error(
        "at {path}: the image has no pixels, so there is nothing for a shader to sample. An \
         image with no `data` is one nothing can read."
    )]
    ImageWithoutData { path: Path },
    #[error(
        "the emitted documents both need an entry named `{entry}`, so one would hide the other"
    )]
    DuplicateEntry { entry: String },
    #[error("at {path}: a screen shader cannot be a standard material: {reason}")]
    ScreenShader { path: Path, reason: String },
    #[error("two {kind}s are labelled `{label}`, so their instance entries would collide")]
    DuplicateLabel { kind: &'static str, label: String },
    #[error(
        "at {path}: the clustering field `{field}` is {found}, and a grid of nothing is a \
         scene whose lights reach no cluster at all. Every extent and every cap must be at \
         least one."
    )]
    EmptyClustering {
        path: Path,
        field: String,
        found: i64,
    },
    #[error(
        "at {path}: the shadow page pool asks for {pages} pages, and the deepest texture array \
         this renderer requests is {limit}. A pool smaller than the one the document asked for \
         would lose lights silently, so the count is refused rather than truncated."
    )]
    PoolTooDeep {
        path: Path,
        pages: i64,
        limit: usize,
    },
    #[error(
        "at {path}: the light `{label}` has a radius of {radius}, and a light that reaches no \
         distance lights nothing at all. A radius is how far a light reaches, so it has to be \
         greater than zero."
    )]
    EmptyLightRadius {
        path: Path,
        label: String,
        radius: f64,
    },
    #[error("the mesh `{label}` has no points, so it has nothing to draw")]
    EmptyMesh { label: String },
    #[error("the emitted documents do not agree with each other: {reason}")]
    Emission { reason: String },
    #[error(
        "applying the emitted graphics delta does not reproduce the emitted graphics: {reason}"
    )]
    GraphicsDeltaMismatch { reason: &'static str },
    #[error(
        "applying the emitted instance delta does not reproduce the emitted instance: {reason}"
    )]
    InstanceDeltaMismatch { reason: &'static str },
    #[error(transparent)]
    Definition(#[from] definition::ValidationError),
}

/// Builds a delta that replaces `path` outright, for compilers that patch rather than
/// diff.
///
/// A whole-value replace is always a valid delta, which makes it a correct but coarse
/// fallback for a compiler that has not yet narrowed a change to a minimal primitive.
pub fn replace_at(path: Path, value: Value) -> BTreeMap<Path, PrimitiveDelta> {
    let mut operations = BTreeMap::new();
    operations.insert(path, PrimitiveDelta::Replace(value));
    operations
}

/// Reports a scene field that is not the shape the compiler requires.
pub(crate) fn malformed_scene(
    path: &Path,
    expected: &'static str,
    value: Option<&Value>,
) -> CompileError {
    CompileError::MalformedScene {
        path: path.clone(),
        expected,
        found: value.map_or("nothing", Value::kind_name),
    }
}

/// Reads a required string field of the scene.
pub(crate) fn scene_text<'value>(
    scene: &'value Value,
    field: &str,
) -> Result<&'value str, CompileError> {
    let path = Path::root().key(field);
    match scene.at(&path) {
        Some(definition::Resolved::Borrowed(Value::String(text))) => Ok(text),
        other => Err(malformed_scene(
            &path,
            "a string",
            other.as_ref().map(|found| found.as_value()),
        )),
    }
}

/// Reads a required list field of the scene.
///
/// # Invariants
///
/// A top level field of the scene is a list of declarations — lights, shaders, meshes, instances —
/// and is read as a tree. A mesh's *points* are a list of numbers and may be packed, but they are
/// read one stride at a time rather than through this, so nothing here ever has to expand a run.
pub(crate) fn scene_list<'value>(
    scene: &'value Value,
    field: &str,
) -> Result<&'value [Value], CompileError> {
    let path = Path::root().key(field);
    match scene.at(&path) {
        Some(definition::Resolved::Borrowed(Value::List(elements))) => Ok(elements),
        other => Err(malformed_scene(
            &path,
            "a list",
            other.as_ref().map(|found| found.as_value()),
        )),
    }
}
