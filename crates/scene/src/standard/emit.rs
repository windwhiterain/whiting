//! Turning a standard scene into the graphics and instance documents that draw it.
//!
//! # What the pipeline is
//!
//! One shadowing pass per light and per shadow casting mesh, then one depthing pass per
//! mesh, then one lighting pass per mesh and per light. The order of the emitted list is
//! the pipeline: nothing in a document says that a lighting pass runs after a shadowing
//! pass, the order does.
//!
//! # Why one mesh per pass
//!
//! A pass binds exactly one shader, and a shader declares one interface. A pass that drew
//! several meshes would therefore have to bind several shaders, which the pass model has
//! no room for. So a pass draws one mesh, and a mesh in three stages is three passes.
//! That is more passes than a merged pipeline needs, and it buys a binding model where
//! what a shader declares is exactly what the pass supplies.
//!
//! # Invariants
//!
//! - Every emitted pass is verified against the interface its own shader declares before
//!   the emitter returns, so a compiler bug is a compile error rather than a bad frame.
//! - The `instance_schema` of the emitted graphics document is derived from the emitted
//!   declarations, so the two documents cannot drift apart silently.

use std::collections::BTreeMap;
use std::rc::Rc;

use definition::{Path, Schema, SchemaKind, Value};

use graphics::binders::{FORMAT, HEIGHT, RESOURCE, SIZE_BYTES, SOURCE, WIDTH};
use graphics::pass::{BINDERS, CLEAR_COLOR, LABEL, SHADER, TARGETS, UNIFORM};
use graphics::{derived_instance_schema, verify_pass_inputs};

use crate::CompileError;
use crate::standard::math::{Bounds, Matrix, projection_of};
use crate::standard::packing::{pack_points, pack_struct};
use crate::standard::pass_schemas::{
    COLOR_FORMAT, COLOR_HEIGHT, COLOR_TARGET, COLOR_WIDTH, DEPTH_BUFFER, DEPTH_FORMAT,
    DEPTH_HEIGHT, DEPTH_WIDTH, INSTANCE_DATA, LIGHT_COLOR, LIGHT_POSITION, PROJECTION, SHADOW_MAP,
    SHADOW_MAP_FORMAT, SHADOW_MAP_HEIGHT, SHADOW_MAP_WIDTH, VERTICES,
};
use crate::standard::scene_schema;

/// How much wider than its box a projection is fitted.
const PADDING: f64 = 1.1;

/// Name of the instance entry holding the depth buffer every depthing pass writes.
pub const DEPTH_BUFFER_ENTRY: &str = "depth_buffer";

/// Name of the instance entry holding the image every lighting pass writes.
pub const COLOR_TARGET_ENTRY: &str = "color_target";

/// Which stage of the pipeline a material belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Renders one light's depth map.
    Shadowing,
    /// Renders the depth of the scene.
    Depthing,
    /// Shades the scene.
    Lighting,
}

impl Stage {
    /// Returns the tag this stage is stored under in a scene document.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Shadowing => "ShadowingShader",
            Self::Depthing => "DepthShader",
            Self::Lighting => "LightingShader",
        }
    }

    /// Returns the name this stage's shader entry is suffixed with.
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Shadowing => "shadowing",
            Self::Depthing => "depthing",
            Self::Lighting => "lighting",
        }
    }
}

/// One mesh, read out of the scene document.
///
/// This is the emitter's working state, not a document: the scene is a [`Value`], the
/// outputs are [`Value`]s, and this exists so the emitter can hold what it read while it
/// walks the stages in order.
struct MeshRead {
    label: String,
    stage: Stage,
    shader_entry: String,
    vertex_entry: String,
    vertex_bytes: i64,
    instance_entry: String,
    instance_bytes: i64,
    bounds: Bounds,
}

/// One light, read out of the scene document.
struct LightRead {
    label: String,
    position: [f64; 3],
    colour: [f64; 3],
    shadow_extent: f64,
}

/// The two documents a standard scene compiles to.
pub struct Emitted {
    /// The graphics document.
    pub graphics: Value,
    /// The instance document.
    pub instance: Value,
}

/// Which topology a mesh declares, which is how its points are read as primitives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Topology {
    /// Every three points are a triangle.
    Triangles,
    /// Every two points are a segment.
    Segments,
}

impl Topology {
    /// Returns the tag this topology is stored under, in a scene and in a buffer.
    const fn tag(self) -> &'static str {
        match self {
            Self::Triangles => "Triangles",
            Self::Segments => "Segments",
        }
    }
}

/// Reads the topology of a mesh.
fn read_topology(mesh: &Value, path: &Path) -> Result<Topology, CompileError> {
    let topology_path = path.key(scene_schema::TOPOLOGY);
    match field(mesh, scene_schema::TOPOLOGY).and_then(Value::variant_tag) {
        Some("Triangles") => Ok(Topology::Triangles),
        Some("Segments") => Ok(Topology::Segments),
        Some(other) => Err(CompileError::UnknownTopology {
            path: topology_path,
            topology: other.to_owned(),
        }),
        None => Err(malformed(&topology_path, "a topology")),
    }
}

/// Reads a scene and emits the graphics and instance documents that draw it.
pub fn emit(scene: &Value) -> Result<Emitted, CompileError> {
    let lights = read_lights(scene)?;
    let (meshes, mut instance, bounds) = read_meshes(scene)?;
    let camera = camera_projection(&bounds);

    add_targets(&mut instance);
    for light in &lights {
        instance.insert(
            shadow_map_entry(&light.label),
            texture(SHADOW_MAP_WIDTH, SHADOW_MAP_HEIGHT, SHADOW_MAP_FORMAT),
        );
    }

    let mut passes = Vec::new();
    for light in &lights {
        for mesh in meshes.iter().filter(|mesh| mesh.stage == Stage::Shadowing) {
            passes.push(shadowing_pass(mesh, light, &bounds));
        }
    }
    for mesh in meshes.iter().filter(|mesh| mesh.stage == Stage::Depthing) {
        passes.push(depthing_pass(mesh, &camera));
    }
    for light in &lights {
        for mesh in meshes.iter().filter(|mesh| mesh.stage == Stage::Lighting) {
            passes.push(lighting_pass(mesh, light, &camera));
        }
    }

    let instance = Value::Dict(instance);
    let mut fields = BTreeMap::new();
    fields.insert(graphics::PASSES_FIELD.to_owned(), Value::List(passes));
    fields.insert(
        graphics::INSTANCE_SCHEMA_FIELD.to_owned(),
        derive_instance_schema(&Value::Struct(fields.clone()))?,
    );
    let graphics = Value::Struct(fields);

    // A compiler that emitted a pass its own shader rejects has emitted nothing useful,
    // so it is refused here rather than at render time.
    for pass in graphics::passes(&graphics).map_err(emission)? {
        verify_pass_inputs(&pass, &instance).map_err(emission)?;
    }

    Ok(Emitted { graphics, instance })
}

/// Reads the lights of the scene.
fn read_lights(scene: &Value) -> Result<Vec<LightRead>, CompileError> {
    let path = Path::root().key(scene_schema::LIGHTS);
    let lights = list_at(scene, &Path::root(), scene_schema::LIGHTS)?;
    let mut read = Vec::with_capacity(lights.len());
    for (index, light) in lights.iter().enumerate() {
        let path = path.index(index);
        let intensity = number(light, &path, "intensity")?;
        let colour = triple(light, &path, "color")?;
        let label = text(light, &path, "label")?.to_owned();
        if read.iter().any(|other: &LightRead| other.label == label) {
            return Err(CompileError::DuplicateLabel {
                kind: "light",
                label,
            });
        }
        read.push(LightRead {
            label,
            position: triple(light, &path, "position")?,
            shadow_extent: number(light, &path, "shadow_extent")?,
            colour: [
                colour[0] * intensity,
                colour[1] * intensity,
                colour[2] * intensity,
            ],
        });
    }
    Ok(read)
}

/// Reads the meshes of the scene, verifying each against the schemas it declares, and
/// collects the instance entries they contribute and the bounds they cover.
fn read_meshes(
    scene: &Value,
) -> Result<(Vec<MeshRead>, BTreeMap<String, Value>, Bounds), CompileError> {
    let path = Path::root().key(scene_schema::MESHES);
    let meshes = list_at(scene, &Path::root(), scene_schema::MESHES)?;

    let mut read: Vec<MeshRead> = Vec::with_capacity(meshes.len());
    let mut instance = BTreeMap::new();
    let mut bounds: Option<Bounds> = None;
    for (index, mesh) in meshes.iter().enumerate() {
        let mesh_path = path.index(index);
        let (found, entries) = read_mesh(mesh, &mesh_path)?;
        bounds = Some(match bounds {
            None => found.bounds,
            Some(previous) => previous.union(found.bounds),
        });
        if read.iter().any(|other| other.label == found.label) {
            return Err(CompileError::DuplicateLabel {
                kind: "mesh",
                label: found.label,
            });
        }
        for (entry, value) in entries {
            instance.insert(entry, value);
        }
        read.push(found);
    }
    // A scene with no mesh has nothing to project, so the projection covers a unit box.
    Ok((
        read,
        instance,
        bounds.unwrap_or_else(|| Bounds::around([0.0; 3], 1.0)),
    ))
}

/// Reads one mesh and every schema it is verified against.
fn read_mesh(mesh: &Value, path: &Path) -> Result<(MeshRead, Vec<(String, Value)>), CompileError> {
    let label = text(mesh, path, scene_schema::LABEL)?.to_owned();
    let material_path = path.key(scene_schema::MATERIAL);
    let material = at(mesh, path, scene_schema::MATERIAL)?;
    let shader_path = material_path.key(scene_schema::SHADER);
    let shader_value = at(material, &material_path, scene_schema::SHADER)?;

    let point_schema = schema_at(mesh, path, scene_schema::POINT_SCHEMA)?;
    let shader = read_shader(shader_value, &shader_path)?;

    // A point must satisfy both the mesh's own shape and the shape its shader reads.
    // Verifying one and not the other is how a mesh ends up carrying a field no shader
    // asked for, or a shader that reads a field no point carries.
    let points = list_at(mesh, path, scene_schema::POINTS)?;
    let points_path = path.key(scene_schema::POINTS);
    for (index, point) in points.iter().enumerate() {
        let point_at = points_path.index(index);
        verify(
            point,
            &point_at,
            point_schema.as_ref(),
            "a point",
            "the mesh point schema",
        )?;
        verify(
            point,
            &point_at,
            shader.point_schema.as_ref(),
            "a point",
            "the shader point schema",
        )?;
    }

    let instance_data = at(mesh, path, scene_schema::INSTANCE_DATA)?;
    verify(
        instance_data,
        &path.key(scene_schema::INSTANCE_DATA),
        shader.instance_schema.as_ref(),
        "the instance data",
        "the shader instance schema",
    )?;

    let custom_data = at(material, &material_path, scene_schema::CUSTOM_DATA)?;
    verify(
        custom_data,
        &material_path.key(scene_schema::CUSTOM_DATA),
        shader.custom_schema.as_ref(),
        "the custom data",
        "the shader custom schema",
    )?;

    let vertex_bytes = pack_points(point_schema.as_ref(), points, &points_path)?;
    if vertex_bytes.is_empty() {
        return Err(CompileError::EmptyMesh { label });
    }
    let topology = read_topology(mesh, path)?;
    let instance_bytes = pack_struct(
        shader.instance_schema.as_ref(),
        instance_data,
        &path.key(scene_schema::INSTANCE_DATA),
    )?;

    let found = MeshRead {
        shader_entry: format!("{label}_{}", shader.stage.suffix()),
        vertex_entry: format!("{label}_{VERTICES}"),
        vertex_bytes: vertex_bytes.len() as i64,
        instance_entry: format!("{label}_{INSTANCE_DATA}"),
        instance_bytes: instance_bytes.len() as i64,
        bounds: bounds_of(points),
        label,
        stage: shader.stage,
    };

    // The buffers carry the schema they were packed with, so a renderer derives the
    // vertex layout from the point schema rather than it being declared a second time.
    let entries = vec![
        (
            found.vertex_entry.clone(),
            buffer_payload(
                vertex_bytes,
                point_schema.to_value(),
                "vertex",
                topology.tag(),
            ),
        ),
        (
            found.instance_entry.clone(),
            buffer_payload(
                instance_bytes,
                shader.instance_schema.to_value(),
                "instance",
                topology.tag(),
            ),
        ),
        (
            found.shader_entry.clone(),
            Value::variant(
                "Shader",
                [
                    (
                        graphics::binders::PATH.to_owned(),
                        Value::String(shader.path),
                    ),
                    (
                        graphics::binders::ENTRY_POINT.to_owned(),
                        Value::String("main".to_owned()),
                    ),
                    (
                        graphics::binders::SCHEMA.to_owned(),
                        shader.pass_schema.to_value(),
                    ),
                ],
            ),
        ),
    ];
    Ok((found, entries))
}

/// A shader as the scene document declares it.
struct ShaderRead {
    stage: Stage,
    path: String,
    pass_schema: Rc<Schema>,
    instance_schema: Rc<Schema>,
    point_schema: Rc<Schema>,
    custom_schema: Rc<Schema>,
}

/// Reads a shader document and every schema it declares.
fn read_shader(shader: &Value, path: &Path) -> Result<ShaderRead, CompileError> {
    if shader.variant_tag() == Some("ScreenShader") {
        return Err(CompileError::ScreenShader {
            path: path.clone(),
            reason: "a standard material is drawn by a mesh shader, in one of the three stages"
                .to_owned(),
        });
    }
    let stage = match shader.variant_tag() {
        Some("ShadowingShader") => Stage::Shadowing,
        Some("DepthShader") => Stage::Depthing,
        Some("LightingShader") => Stage::Lighting,
        Some(other) => {
            return Err(CompileError::UnknownStage {
                path: path.clone(),
                stage: other.to_owned(),
            });
        }
        None => return Err(malformed(path, "a mesh shader")),
    };

    let Some(fields) = shader.variant_fields() else {
        return Err(malformed(path, "a struct of schemas"));
    };
    let read = |name: &str| -> Result<Rc<Schema>, CompileError> {
        let field_path = path.key(name);
        let field = fields
            .get(name)
            .ok_or_else(|| malformed(&field_path, "a schema document"))?;
        Ok(Rc::new(read_schema(field, &field_path)?))
    };

    Ok(ShaderRead {
        stage,
        path: text(shader, path, scene_schema::PATH)?.to_owned(),
        pass_schema: read(scene_schema::PASS_SCHEMA)?,
        instance_schema: read(scene_schema::INSTANCE_SCHEMA)?,
        point_schema: read(scene_schema::POINT_SCHEMA_FIELD)?,
        custom_schema: read(scene_schema::CUSTOM_SCHEMA)?,
    })
}

/// Reads the `instance_schema` the emitted declarations imply.
fn derive_instance_schema(graphics: &Value) -> Result<Value, CompileError> {
    derived_instance_schema(graphics)
        .map(|schema| schema.to_value())
        .map_err(emission)
}

/// Adds the depth buffer and the shaded image to the instance.
fn add_targets(instance: &mut BTreeMap<String, Value>) {
    instance.insert(
        DEPTH_BUFFER_ENTRY.to_owned(),
        texture(DEPTH_WIDTH, DEPTH_HEIGHT, DEPTH_FORMAT),
    );
    instance.insert(
        COLOR_TARGET_ENTRY.to_owned(),
        texture(COLOR_WIDTH, COLOR_HEIGHT, COLOR_FORMAT),
    );
}

/// Returns the instance entry name holding one light's shadow map.
pub fn shadow_map_entry(light_label: &str) -> String {
    format!("{light_label}_{SHADOW_MAP}")
}

/// Builds the shadowing pass of one mesh for one light.
fn shadowing_pass(mesh: &MeshRead, light: &LightRead, bounds: &Bounds) -> Value {
    // The shadow map looks down the light's direction, at a box centred on the scene and
    // as wide as the light says it shines.
    let centre = bounds.centre();
    let eye = light.position;
    let projection = projection_value(
        &projection_of(
            &Bounds::around(centre, light.shadow_extent),
            eye,
            centre,
            PADDING,
        ),
        eye,
    );

    rasterize(
        &format!("shadow/{}/{}", light.label, mesh.label),
        &mesh.shader_entry,
        Value::structure([(PROJECTION.to_owned(), projection)]),
        mesh_bindings(mesh).chain([(
            SHADOW_MAP.to_owned(),
            texture_binding(
                &shadow_map_entry(&light.label),
                SHADOW_MAP_WIDTH,
                SHADOW_MAP_HEIGHT,
                SHADOW_MAP_FORMAT,
            ),
        )]),
        vec![SHADOW_MAP],
        DEPTH_CLEAR,
    )
}

/// Builds the depthing pass of one mesh.
fn depthing_pass(mesh: &MeshRead, camera: &Value) -> Value {
    rasterize(
        &format!("depth/{}", mesh.label),
        &mesh.shader_entry,
        Value::structure([(PROJECTION.to_owned(), camera.clone())]),
        mesh_bindings(mesh).chain([(
            DEPTH_BUFFER.to_owned(),
            texture_binding(DEPTH_BUFFER_ENTRY, DEPTH_WIDTH, DEPTH_HEIGHT, DEPTH_FORMAT),
        )]),
        vec![DEPTH_BUFFER],
        DEPTH_CLEAR,
    )
}

/// Builds the lighting pass of one mesh for one light.
fn lighting_pass(mesh: &MeshRead, light: &LightRead, camera: &Value) -> Value {
    rasterize(
        &format!("lighting/{}/{}", mesh.label, light.label),
        &mesh.shader_entry,
        Value::structure([
            (PROJECTION.to_owned(), camera.clone()),
            (
                LIGHT_POSITION.to_owned(),
                Value::List(light.position.into_iter().map(Value::Float).collect()),
            ),
            (
                LIGHT_COLOR.to_owned(),
                Value::List(light.colour.into_iter().map(Value::Float).collect()),
            ),
        ]),
        mesh_bindings(mesh).chain([
            (
                COLOR_TARGET.to_owned(),
                texture_binding(COLOR_TARGET_ENTRY, COLOR_WIDTH, COLOR_HEIGHT, COLOR_FORMAT),
            ),
            (
                DEPTH_BUFFER.to_owned(),
                texture_binding(DEPTH_BUFFER_ENTRY, DEPTH_WIDTH, DEPTH_HEIGHT, DEPTH_FORMAT),
            ),
            (
                SHADOW_MAP.to_owned(),
                texture_binding(
                    &shadow_map_entry(&light.label),
                    SHADOW_MAP_WIDTH,
                    SHADOW_MAP_HEIGHT,
                    SHADOW_MAP_FORMAT,
                ),
            ),
        ]),
        vec![COLOR_TARGET],
        COLOUR_CLEAR,
    )
}

/// Returns the projection the camera stages draw through.
///
/// # Invariants
///
/// The scene document has no camera, so this is a stated convention rather than an
/// author decision: the camera looks down the negative z axis from three box widths in
/// front of the scene, and the box it projects is the whole scene with a little padding
/// so geometry on the edge is not clipped by a rounding error. A scene that wants a
/// camera writes one, and this goes away.
fn camera_projection(bounds: &Bounds) -> Value {
    let eye = [
        bounds.centre()[0],
        bounds.centre()[1],
        bounds.centre()[2] + 3.0 * bounds.largest_half_width(),
    ];
    projection_value(&projection_of(bounds, eye, bounds.centre(), PADDING), eye)
}

/// Wraps a matrix and the point it was made from into a projection document.
fn projection_value(matrix: &Matrix, eye: [f64; 3]) -> Value {
    Value::structure([
        (
            "view_projection".to_owned(),
            Value::List(matrix.iter().copied().map(Value::Float).collect()),
        ),
        (
            "position".to_owned(),
            Value::List(eye.into_iter().map(Value::Float).collect()),
        ),
    ])
}

/// Returns the box the packed points of a mesh cover.
///
/// Every point of a mesh has the same shape, so the box is over the components of the
/// first three of them, which is the position a projection needs.
fn bounds_of(points: &[Value]) -> Bounds {
    let mut bounds = Bounds {
        minimum: [f64::INFINITY; 3],
        maximum: [f64::NEG_INFINITY; 3],
    };
    for point in points {
        for (axis, component) in point
            .as_struct()
            .into_iter()
            .flat_map(|fields| fields.values())
            .flat_map(number_components)
            .take(3)
            .enumerate()
        {
            bounds.minimum[axis] = bounds.minimum[axis].min(component);
            bounds.maximum[axis] = bounds.maximum[axis].max(component);
        }
    }
    if !bounds.minimum.iter().all(|value| value.is_finite()) {
        return Bounds::around([0.0; 3], 1.0);
    }
    bounds
}

/// Returns the numbers a point field contributes, whether it is one number or a list.
fn number_components(value: &Value) -> Vec<f64> {
    match value {
        Value::Float(number) => vec![*number],
        Value::Int(number) => vec![*number as f64],
        Value::List(components) => components.iter().flat_map(number_components).collect(),
        _ => Vec::new(),
    }
}

/// Returns the two binding points every pass of a mesh carries.
fn mesh_bindings(mesh: &MeshRead) -> impl Iterator<Item = (String, Value)> + use<'_> {
    [
        (
            VERTICES.to_owned(),
            buffer_binding(&mesh.vertex_entry, mesh.vertex_bytes),
        ),
        (
            INSTANCE_DATA.to_owned(),
            buffer_binding(&mesh.instance_entry, mesh.instance_bytes),
        ),
    ]
    .into_iter()
}

/// The colour a depth map is cleared to, which is as far away as the depth range goes.
const DEPTH_CLEAR: [f64; 4] = [1.0, 1.0, 1.0, 1.0];

/// The colour a shaded image is cleared to, which is unlit.
const COLOUR_CLEAR: [f64; 4] = [0.0, 0.0, 0.0, 1.0];

/// Builds a rasterize pass with the shared fields every pass carries.
fn rasterize(
    label: &str,
    shader_entry: &str,
    uniform: Value,
    binders: impl IntoIterator<Item = (String, Value)>,
    targets: impl IntoIterator<Item = &'static str>,
    clear: [f64; 4],
) -> Value {
    Value::variant(
        "Rasterize",
        [
            (LABEL.to_owned(), Value::String(label.to_owned())),
            (UNIFORM.to_owned(), uniform),
            (SHADER.to_owned(), Value::String(shader_entry.to_owned())),
            (BINDERS.to_owned(), Value::dictionary(binders)),
            (
                TARGETS.to_owned(),
                Value::List(
                    targets
                        .into_iter()
                        .map(|target| Value::String(target.to_owned()))
                        .collect(),
                ),
            ),
            (
                CLEAR_COLOR.to_owned(),
                Value::List(clear.into_iter().map(Value::Float).collect()),
            ),
        ],
    )
}

/// Builds a buffer declaration naming an instance entry.
fn buffer_binding(source: &str, size_bytes: i64) -> Value {
    Value::variant(
        "Resource",
        [(
            RESOURCE.to_owned(),
            Value::variant(
                "Buffer",
                [
                    (SOURCE.to_owned(), Value::String(source.to_owned())),
                    (SIZE_BYTES.to_owned(), Value::Int(size_bytes)),
                ],
            ),
        )],
    )
}

/// Builds a texture declaration naming an instance entry.
fn texture_binding(source: &str, width: i64, height: i64, format: &str) -> Value {
    Value::variant(
        "Resource",
        [(
            RESOURCE.to_owned(),
            Value::variant(
                "Texture",
                [
                    (SOURCE.to_owned(), Value::String(source.to_owned())),
                    (WIDTH.to_owned(), Value::Int(width)),
                    (HEIGHT.to_owned(), Value::Int(height)),
                    (FORMAT.to_owned(), Value::String(format.to_owned())),
                ],
            ),
        )],
    )
}

/// Builds a buffer payload, which carries the schema it was packed with.
fn buffer_payload(bytes: Vec<u8>, schema: Value, usage: &str, topology: &str) -> Value {
    Value::variant(
        "Buffer",
        [
            (SIZE_BYTES.to_owned(), Value::Int(bytes.len() as i64)),
            (
                graphics::binders::USAGE.to_owned(),
                Value::List(vec![Value::String(usage.to_owned())]),
            ),
            (graphics::binders::SCHEMA.to_owned(), schema),
            (
                graphics::binders::TOPOLOGY.to_owned(),
                Value::variant(topology, []),
            ),
            (graphics::binders::DATA.to_owned(), Value::Bytes(bytes)),
        ],
    )
}

/// Builds the texture payload of a target.
fn texture(width: i64, height: i64, format: &str) -> Value {
    Value::variant(
        "Texture",
        [
            (WIDTH.to_owned(), Value::Int(width)),
            (HEIGHT.to_owned(), Value::Int(height)),
            (FORMAT.to_owned(), Value::String(format.to_owned())),
        ],
    )
}

/// Returns a named field of a struct, a dict, or the fields of an enum variant.
fn field<'value>(value: &'value Value, name: &str) -> Option<&'value Value> {
    value
        .as_field_map()
        .or_else(|| value.variant_fields())
        .and_then(|fields| fields.get(name))
}

/// Borrows a named field, reporting the path it should have been at.
fn at<'value>(
    value: &'value Value,
    path: &Path,
    name: &str,
) -> Result<&'value Value, CompileError> {
    let field_path = path.key(name);
    field(value, name).ok_or_else(|| malformed(&field_path, "a value"))
}

/// Borrows a named list field.
fn list_at<'value>(
    value: &'value Value,
    path: &Path,
    name: &str,
) -> Result<&'value [Value], CompileError> {
    let field_path = path.key(name);
    match field(value, name) {
        Some(Value::List(elements)) => Ok(elements),
        Some(other) => Err(CompileError::MalformedScene {
            path: field_path,
            expected: "a list",
            found: other.kind_name(),
        }),
        None => Err(malformed(&field_path, "a list")),
    }
}

/// Borrows a required string field.
fn text<'value>(
    value: &'value Value,
    path: &Path,
    name: &str,
) -> Result<&'value str, CompileError> {
    let field_path = path.key(name);
    match field(value, name) {
        Some(Value::String(text)) => Ok(text),
        Some(other) => Err(CompileError::MalformedScene {
            path: field_path,
            expected: "a string",
            found: other.kind_name(),
        }),
        None => Err(malformed(&field_path, "a string")),
    }
}

/// Borrows a required number field.
fn number(value: &Value, path: &Path, name: &str) -> Result<f64, CompileError> {
    let field_path = path.key(name);
    match field(value, name) {
        Some(Value::Float(number)) => Ok(*number),
        Some(Value::Int(number)) => Ok(*number as f64),
        Some(other) => Err(CompileError::MalformedScene {
            path: field_path,
            expected: "a number",
            found: other.kind_name(),
        }),
        None => Err(malformed(&field_path, "a number")),
    }
}

/// Borrows a required three component vector field.
fn triple(value: &Value, path: &Path, name: &str) -> Result<[f64; 3], CompileError> {
    let field_path = path.key(name);
    let components = list_at(value, path, name)?;
    let mut numbers = Vec::with_capacity(components.len());
    for (index, component) in components.iter().enumerate() {
        numbers.push(match component {
            Value::Float(number) => *number,
            Value::Int(number) => *number as f64,
            other => {
                return Err(CompileError::MalformedScene {
                    path: field_path.index(index),
                    expected: "a number",
                    found: other.kind_name(),
                });
            }
        });
    }
    numbers
        .try_into()
        .map_err(|_| CompileError::MalformedScene {
            path: field_path,
            expected: "three components",
            found: "a different number of components",
        })
}

/// Reads the schema a document carries in a named field.
fn schema_at(value: &Value, path: &Path, name: &str) -> Result<Rc<Schema>, CompileError> {
    let field_path = path.key(name);
    Ok(Rc::new(read_schema(at(value, path, name)?, &field_path)?))
}

/// Reads a schema document.
fn read_schema(value: &Value, path: &Path) -> Result<Schema, CompileError> {
    Schema::from_value(value).map_err(|error| CompileError::UnreadableSchema {
        path: path.clone(),
        reason: error.to_string(),
    })
}

/// Verifies a value against a schema, naming both what was checked and what against.
fn verify(
    value: &Value,
    path: &Path,
    schema: &Schema,
    subject: &str,
    against: &str,
) -> Result<(), CompileError> {
    if matches!(schema.kind, SchemaKind::Any) {
        return Ok(());
    }
    schema
        .validate(value)
        .map_err(|error| CompileError::Verification {
            path: path.clone(),
            subject: subject.to_owned(),
            against: against.to_owned(),
            reason: error.to_string(),
        })
}

/// Wraps a graphics failure as an emission failure.
fn emission(error: graphics::GraphicsError) -> CompileError {
    CompileError::Emission {
        reason: error.to_string(),
    }
}

/// Reports a field of a document that is not the shape the compiler requires.
fn malformed(path: &Path, expected: &'static str) -> CompileError {
    CompileError::MalformedScene {
        path: path.clone(),
        expected,
        found: "nothing",
    }
}
