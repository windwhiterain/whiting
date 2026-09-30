//! Turning a standard scene into the graphics and instance documents that draw it.
//!
//! # What the pipeline is
//!
//! One shadowing pass per light and per shadow casting instance, then one depthing pass per
//! instance, then one lighting pass per instance and per light. The order of the emitted
//! list is the pipeline: nothing in a document says that a lighting pass runs after a
//! shadowing pass, the order does.
//!
//! # Why one instance per pass
//!
//! A pass binds exactly one shader, and a shader declares one interface. A pass that drew
//! several instances would therefore have to bind several shaders, which the pass model has
//! no room for. So a pass draws one instance, and a mesh drawn in three stages is three
//! instances of one mesh, which is why a mesh is geometry and an instance is a placement.
//!
//! # Invariants
//!
//! - Every emitted pass is verified against the interface its own shader declares before
//!   the emitter returns, so a compiler bug is a compile error rather than a bad frame.
//! - The `instance_schema` of the emitted graphics document is derived from the emitted
//!   declarations, so the two documents cannot drift apart silently.
//! - A vertex buffer belongs to a mesh and is shared by every instance that draws it; an
//!   instance buffer belongs to an instance and holds that instance's row alone.

use std::collections::BTreeMap;
use std::rc::Rc;

use definition::{Path, Schema, SchemaKind, Value};

use graphics::binders::{FORMAT, HEIGHT, RESOURCE, SIZE_BYTES, SOURCE, WIDTH};
use graphics::pass::{BINDERS, CLEAR_COLOR, LABEL, SHADER, TARGETS, UNIFORM};
use graphics::{derived_instance_schema, verify_pass_inputs};

use crate::CompileError;
use crate::standard::camera::{Camera, Projection, projection_value};
use crate::standard::math::{Bounds, Matrix, apply, projection_of, transform_matrix};
use crate::standard::packing::{pack_points, pack_struct};
use crate::standard::pass_schemas::{
    COLOR_FORMAT, COLOR_HEIGHT, COLOR_TARGET, COLOR_WIDTH, DEPTH_BUFFER, DEPTH_FORMAT,
    DEPTH_HEIGHT, DEPTH_WIDTH, INSTANCE_DATA, LIGHT_COLOR, LIGHT_POSITION, LIGHT_PROJECTION,
    LIGHT_SHADOW_TEXEL, PROJECTION, SHADOW_MAP, SHADOW_MAP_FORMAT, SHADOW_MAP_HEIGHT,
    SHADOW_MAP_WIDTH, TIME, VERTICES,
};
use crate::standard::scene_schema;

/// Field the model matrix of an instance row is carried under.
const MODEL: &str = "model";

/// How much wider than its box a projection is fitted.
pub const FRAMING_PADDING: f64 = 1.1;

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

    /// Returns the prefix this stage's passes are labelled with.
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Shadowing => "shadow",
            Self::Depthing => "depth",
            Self::Lighting => "lighting",
        }
    }
}

/// One mesh, read out of the scene document.
///
/// One mesh, read out of the scene document: geometry, and nothing about where it is drawn.
struct MeshRead {
    label: String,
    topology: Topology,
    point_schema: Rc<Schema>,
    points: Vec<Value>,
    vertex_bytes: i64,
    vertex_entry: String,
}

/// One instance, read out of the scene document: a mesh, where it is, and what draws it.
struct InstanceRead {
    label: String,
    /// The index of the mesh this instance draws, resolved from its label.
    mesh: usize,
    /// The index of the shader it is drawn by, resolved from the material's label.
    shader: usize,
    stage: Stage,
    /// The schema the instance row was packed with, which is the layout it is read with.
    instance_schema: Rc<Schema>,
    instance_bytes: Vec<u8>,
    /// The model matrix the instance row carries, which the transform composes to.
    model: Matrix,
}

/// Where an instance is, and how it is turned and sized.
#[derive(Debug, Clone, Copy)]
struct Transform {
    translation: [f64; 3],
    rotation: [f64; 3],
    scale: [f64; 3],
}

impl Transform {
    /// Returns the model matrix this transform composes to.
    fn matrix(self) -> Matrix {
        transform_matrix(self.translation, self.rotation, self.scale)
    }
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
///
/// # Invariants
///
/// A part is declared once and referred to by label: a mesh by every instance that draws
/// it, a shader by every material that names it. The emitted document keeps that shape, so
/// one mesh contributes one vertex buffer and one shader contributes one entry, however
/// many passes draw with them.
pub fn emit(scene: &Value) -> Result<Emitted, CompileError> {
    let lights = read_lights(scene)?;
    let shaders = read_shaders(scene)?;
    let (meshes, mut instance) = read_meshes(scene)?;
    let instances = read_instances(scene, &meshes, &shaders)?;
    // The camera frames what is drawn rather than what is authored, because an instance
    // may place its mesh anywhere and a mesh no instance names is not drawn at all.
    let bounds = bounds_of_instances(&meshes, &instances);
    let time = read_time(scene)?;
    let camera = Camera::read(scene, &bounds, COLOR_WIDTH as f64 / COLOR_HEIGHT as f64)?;
    let camera = projection_value(&camera.projection());

    add_targets(&mut instance);
    for light in &lights {
        insert_entry(
            &mut instance,
            shadow_map_entry(&light.label),
            texture(SHADOW_MAP_WIDTH, SHADOW_MAP_HEIGHT, SHADOW_MAP_FORMAT),
        )?;
    }
    for shader in &shaders {
        insert_entry(&mut instance, shader.label.clone(), shader_entry(shader))?;
    }

    // One pass per group, and one instance entry per group, because the instances in a group are
    // drawn together by one instanced draw rather than one draw apiece.
    let mut passes = Vec::new();
    for light in &lights {
        for group in groups(
            &instances,
            &meshes,
            &shaders,
            Stage::Shadowing,
            Some(&light.label),
        ) {
            insert_entry(&mut instance, group.entry(), group.payload())?;
            passes.push(shadowing_pass(&group, light, &bounds, time));
        }
    }
    for group in groups(&instances, &meshes, &shaders, Stage::Depthing, None) {
        insert_entry(&mut instance, group.entry(), group.payload())?;
        passes.push(depthing_pass(&group, &camera, time));
    }
    for light in &lights {
        for group in groups(
            &instances,
            &meshes,
            &shaders,
            Stage::Lighting,
            Some(&light.label),
        ) {
            insert_entry(&mut instance, group.entry(), group.payload())?;
            passes.push(lighting_pass(&group, light, &camera, &bounds, time));
        }
    }

    // An instance holds its payloads under a field rather than at its root, so that it can name
    // the graphics it belongs to. What names it is written by assembling, which is where the
    // identifier the output is stored under is known.
    let instance = Value::structure([(graphics::ENTRIES.to_owned(), Value::Dict(instance))]);
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

/// Reads the time a scene is drawn at, which defaults to a still.
///
/// # Invariants
///
/// Time is a field of the scene rather than a clock the renderer reads, so a render is a
/// pure function of the documents and this one number. A scene that names no time is a
/// still, which is what every scene was before time existed.
fn read_time(scene: &Value) -> Result<f64, CompileError> {
    let path = Path::root().key(scene_schema::TIME);
    match field(scene, scene_schema::TIME) {
        None => Ok(0.0),
        Some(value) => value
            .as_float()
            .or_else(|| value.as_int().map(|number| number as f64))
            .ok_or_else(|| CompileError::MalformedScene {
                path,
                expected: "a number of seconds",
                found: value.kind_name(),
            }),
    }
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

/// Reads the meshes of the scene and the instance entries their vertex buffers contribute.
fn read_meshes(scene: &Value) -> Result<(Vec<MeshRead>, BTreeMap<String, Value>), CompileError> {
    let path = Path::root().key(scene_schema::MESHES);
    let meshes = list_at(scene, &Path::root(), scene_schema::MESHES)?;

    let mut read: Vec<MeshRead> = Vec::with_capacity(meshes.len());
    let mut instance = BTreeMap::new();
    for (index, mesh) in meshes.iter().enumerate() {
        let mesh_path = path.index(index);
        let (found, entry) = read_mesh(mesh, &mesh_path)?;
        if read.iter().any(|other| other.label == found.label) {
            return Err(CompileError::DuplicateLabel {
                kind: "mesh",
                label: found.label,
            });
        }
        instance.insert(entry.0, entry.1);
        read.push(found);
    }
    Ok((read, instance))
}

/// Reads one mesh: the geometry an instance draws.
fn read_mesh(mesh: &Value, path: &Path) -> Result<(MeshRead, (String, Value)), CompileError> {
    let label = text(mesh, path, scene_schema::LABEL)?.to_owned();
    let point_schema = schema_at(mesh, path, scene_schema::POINT_SCHEMA)?;
    let points = list_at(mesh, path, scene_schema::POINTS)?;
    let points_path = path.key(scene_schema::POINTS);
    for (index, point) in points.iter().enumerate() {
        verify(
            point,
            &points_path.index(index),
            point_schema.as_ref(),
            "a point",
            "the mesh point schema",
        )?;
    }

    let vertex_bytes = pack_points(point_schema.as_ref(), points, &points_path)?;
    if vertex_bytes.is_empty() {
        return Err(CompileError::EmptyMesh { label });
    }
    let topology = read_topology(mesh, path)?;
    let vertex_entry = format!("{label}_{VERTICES}");
    let found = MeshRead {
        vertex_bytes: vertex_bytes.len() as i64,
        vertex_entry: vertex_entry.clone(),
        label,
        topology,
        point_schema,
        points: points.to_vec(),
    };

    // The buffer carries the schema it was packed with, so a renderer derives the vertex
    // layout from the point schema rather than it being declared a second time.
    let entry = (
        vertex_entry,
        buffer_payload(
            vertex_bytes,
            found.point_schema.to_value(),
            "vertex",
            topology.tag(),
        ),
    );
    Ok((found, entry))
}

/// Reads the instances of the scene, resolving each one's mesh and shader.
fn read_instances(
    scene: &Value,
    meshes: &[MeshRead],
    shaders: &[ShaderRead],
) -> Result<Vec<InstanceRead>, CompileError> {
    let path = Path::root().key(scene_schema::INSTANCES);
    let instances = list_at(scene, &Path::root(), scene_schema::INSTANCES)?;

    let mut read: Vec<InstanceRead> = Vec::with_capacity(instances.len());
    for (index, instance) in instances.iter().enumerate() {
        let found = read_instance(instance, &path.index(index), meshes, shaders)?;
        if read.iter().any(|other| other.label == found.label) {
            return Err(CompileError::DuplicateLabel {
                kind: "instance",
                label: found.label,
            });
        }
        read.push(found);
    }
    Ok(read)
}

/// Reads one instance and every schema it is verified against.
///
/// # Invariants
///
/// A point is verified against its mesh's own point schema and against the point schema of
/// every shader that draws that mesh. Verifying one and not the other is how a mesh ends up
/// carrying a field no shader asked for, or a shader that reads a field no point carries.
fn read_instance(
    instance: &Value,
    path: &Path,
    meshes: &[MeshRead],
    shaders: &[ShaderRead],
) -> Result<InstanceRead, CompileError> {
    let label = text(instance, path, scene_schema::LABEL)?.to_owned();
    let mesh_label = text(instance, path, scene_schema::MESH)?;
    let Some((mesh, found)) = meshes
        .iter()
        .enumerate()
        .find(|(_, mesh)| mesh.label == mesh_label)
    else {
        return Err(CompileError::UnknownMesh {
            path: path.key(scene_schema::MESH),
            mesh: mesh_label.to_owned(),
        });
    };

    let transform = read_transform(instance, path)?;
    let material_path = path.key(scene_schema::MATERIAL);
    let material = at(instance, path, scene_schema::MATERIAL)?;
    let shader_path = material_path.key(scene_schema::SHADER);
    let shader_label = at(material, &material_path, scene_schema::SHADER)?
        .as_str()
        .unwrap_or("<not a label>");
    let Some((shader, shader_read)) = shaders
        .iter()
        .enumerate()
        .find(|(_, shader)| shader.label == shader_label)
    else {
        return Err(CompileError::UnknownShader {
            path: shader_path,
            shader: shader_label.to_owned(),
        });
    };

    let points_path = Path::root()
        .key(scene_schema::MESHES)
        .index(mesh)
        .key(scene_schema::POINTS);
    for (index, point) in found.points.iter().enumerate() {
        verify(
            point,
            &points_path.index(index),
            shader_read.point_schema.as_ref(),
            "a point",
            "the shader point schema",
        )?;
    }

    let custom_data = at(material, &material_path, scene_schema::CUSTOM_DATA)?;
    let custom_path = material_path.key(scene_schema::CUSTOM_DATA);
    verify(
        custom_data,
        &custom_path,
        shader_read.custom_schema.as_ref(),
        "the custom data",
        "the shader custom schema",
    )?;

    // The instance row is where the instance is, plus the material's custom data. Both are per
    // instance, and once one pass draws several instances at once a per instance attribute is the
    // only place either can go: a uniform is read by every instance of the pass, so anything that
    // varies belongs here. The row is therefore read with the shader's instance schema and the
    // material's custom schema merged, and a field both name is an error rather than one of them
    // silently winning.
    let row_schema = merged_row_schema(
        shader_read.instance_schema.as_ref(),
        shader_read.custom_schema.as_ref(),
    )?;
    let model = transform.matrix();
    let mut fields = vec![(
        MODEL.to_owned(),
        Value::List(model.iter().copied().map(Value::Float).collect()),
    )];
    let Some(custom_fields) = custom_data.as_field_map() else {
        return Err(malformed(&custom_path, "the custom data as a struct"));
    };
    fields.extend(
        custom_fields
            .iter()
            .map(|(name, value)| (name.clone(), value.clone())),
    );
    let row = Value::structure(fields);
    let row_path = path.key(scene_schema::TRANSFORM);
    verify(
        &row,
        &row_path,
        &row_schema,
        "the instance row",
        "the shader instance schema",
    )?;
    let instance_bytes = pack_struct(&row_schema, &row, &row_path)?;

    Ok(InstanceRead {
        instance_schema: row_schema,
        instance_bytes,
        label,
        mesh,
        shader,
        stage: shader_read.stage,
        model,
    })
}

/// Returns the schema one instance row is read with: what the shader reads per instance, and the
/// custom fields the material carries.
///
/// # Invariants
///
/// A field named by both is an error rather than one of them winning, because the two schemas
/// describe one row and a name in both would be two values written to one place.
fn merged_row_schema(instance: &Schema, custom: &Schema) -> Result<Rc<Schema>, CompileError> {
    let (
        SchemaKind::Struct {
            fields: instance_fields,
        },
        SchemaKind::Struct { fields: custom },
    ) = (&instance.kind, &custom.kind)
    else {
        return Err(CompileError::Emission {
            reason: "an instance row and a material's custom data are both read from struct \
                     schemas"
                .to_owned(),
        });
    };
    let mut fields = instance_fields.clone();
    for (name, field) in custom {
        if fields.insert(name.clone(), field.clone()).is_some() {
            return Err(CompileError::Emission {
                reason: format!("the custom field `{name}` is also an instance field"),
            });
        }
    }
    Ok(Rc::new(Schema::new(SchemaKind::structure(fields))))
}

/// Reads where an instance is.
fn read_transform(instance: &Value, path: &Path) -> Result<Transform, CompileError> {
    let transform = at(instance, path, scene_schema::TRANSFORM)?;
    let transform_path = path.key(scene_schema::TRANSFORM);
    Ok(Transform {
        translation: triple(transform, &transform_path, "translation")?,
        rotation: triple(transform, &transform_path, "rotation")?,
        scale: triple(transform, &transform_path, "scale")?,
    })
}

/// Instances one pass can draw together.
///
/// # Why instances share a pass
///
/// A pass binds one vertex buffer and one instance buffer, and the instance buffer steps once per
/// row, so the renderer draws as many instances as the buffer has rows. Instances whose pass
/// would otherwise be identical therefore share one pass: the same stage, the same shader, the
/// same mesh, and the same light when the stage reads one. What differs between the rows of that
/// one buffer is the per-instance data, which is why the material carries it and why nothing about
/// it belongs in the uniform — a uniform is read by every instance the pass draws.
struct Group<'read> {
    /// The instances drawn together, in the order their rows are written.
    members: Vec<&'read InstanceRead>,
    /// The mesh they draw.
    mesh: &'read MeshRead,
    /// The label of the shader they draw with.
    shader: &'read str,
    /// The stage they are drawn in.
    stage: Stage,
    /// The light the stage reads, for the stages that read one.
    light: Option<&'read str>,
}

impl Group<'_> {
    /// Returns the name of the pass, which is also what its instance entry is named after.
    fn label(&self) -> String {
        match self.light {
            Some(light) => format!(
                "{}/{light}/{}/{}",
                self.stage.prefix(),
                self.shader,
                self.mesh.label
            ),
            None => format!(
                "{}/{}/{}",
                self.stage.prefix(),
                self.shader,
                self.mesh.label
            ),
        }
    }

    /// Returns the name of the instance entry the pass reads.
    fn entry(&self) -> String {
        format!("{}_instance_data", self.label())
    }

    /// Returns the rows of every member, in order, as the bytes one instance buffer carries.
    fn bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for member in &self.members {
            bytes.extend_from_slice(&member.instance_bytes);
        }
        bytes
    }

    /// Returns the instance entry this group contributes.
    ///
    /// # Invariants
    ///
    /// Every member's row was packed with the schema of the shader the group draws with, so the
    /// rows share one layout and the first of them describes all of them.
    fn payload(&self) -> Value {
        buffer_payload(
            self.bytes(),
            self.members[0].instance_schema.to_value(),
            "instance",
            self.mesh.topology.tag(),
        )
    }
}

/// Groups the instances of one stage by the pass they can share.
fn groups<'read>(
    instances: &'read [InstanceRead],
    meshes: &'read [MeshRead],
    shaders: &'read [ShaderRead],
    stage: Stage,
    light: Option<&'read str>,
) -> Vec<Group<'read>> {
    let mut groups: Vec<Group<'read>> = Vec::new();
    for found in instances.iter().filter(|found| found.stage == stage) {
        let mesh = &meshes[found.mesh];
        let shader = shaders[found.shader].label.as_str();
        match groups
            .iter_mut()
            .find(|group| group.shader == shader && group.mesh.label == mesh.label)
        {
            Some(group) => group.members.push(found),
            None => groups.push(Group {
                members: vec![found],
                mesh,
                shader,
                stage,
                light,
            }),
        }
    }
    groups
}

/// A shader as the scene document declares it.
struct ShaderRead {
    label: String,
    stage: Stage,
    source: String,
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
        label: text(shader, path, scene_schema::LABEL)?.to_owned(),
        source: text(shader, path, scene_schema::SOURCE)?.to_owned(),
        pass_schema: read(scene_schema::PASS_SCHEMA)?,
        instance_schema: read(scene_schema::INSTANCE_SCHEMA)?,
        point_schema: read(scene_schema::POINT_SCHEMA_FIELD)?,
        custom_schema: read(scene_schema::CUSTOM_SCHEMA)?,
    })
}

/// Reads the shaders of the scene, which every material names by label.
fn read_shaders(scene: &Value) -> Result<Vec<ShaderRead>, CompileError> {
    let path = Path::root().key(scene_schema::SHADERS);
    let shaders = list_at(scene, &Path::root(), scene_schema::SHADERS)?;

    let mut read: Vec<ShaderRead> = Vec::with_capacity(shaders.len());
    for (index, shader) in shaders.iter().enumerate() {
        let found = read_shader(shader, &path.index(index))?;
        if read.iter().any(|other| other.label == found.label) {
            return Err(CompileError::DuplicateLabel {
                kind: "shader",
                label: found.label,
            });
        }
        read.push(found);
    }
    Ok(read)
}

/// Builds the instance entry that holds one declared shader.
///
/// A shader is declared once and named by every material that draws with it, so its entry
/// carries the source and the interface rather than being copied per pass.
fn shader_entry(shader: &ShaderRead) -> Value {
    Value::variant(
        "Shader",
        [
            (
                graphics::binders::SOURCE.to_owned(),
                Value::String(shader.source.clone()),
            ),
            (
                graphics::binders::SCHEMA.to_owned(),
                shader.pass_schema.to_value(),
            ),
        ],
    )
}

/// Adds one instance entry, refusing a name another entry already holds.
///
/// Emitted entry names are generated from scene labels, so two parts whose labels collide
/// would silently overwrite one another and draw the wrong thing. A collision is a compile
/// error that names the entry.
fn insert_entry(
    instance: &mut BTreeMap<String, Value>,
    entry: String,
    value: Value,
) -> Result<(), CompileError> {
    if instance.contains_key(&entry) {
        return Err(CompileError::DuplicateEntry { entry });
    }
    instance.insert(entry, value);
    Ok(())
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

/// Builds the shadowing pass of one group for one light.
fn shadowing_pass(group: &Group<'_>, light: &LightRead, bounds: &Bounds, time: f64) -> Value {
    // The shadow map looks down the light's direction, at a box centred on the scene and
    // as wide as the light says it shines. The same projection is handed to the lighting
    // pass as `light_projection`, because that is how a surface is found in the map.
    let light_projection = light_projection(light, bounds);
    rasterize(
        &group.label(),
        group.shader,
        Value::structure([
            (PROJECTION.to_owned(), projection_value(&light_projection)),
            (TIME.to_owned(), Value::Float(time)),
        ]),
        mesh_bindings(group).chain([(
            SHADOW_MAP.to_owned(),
            texture_binding(
                &shadow_map_entry(light.label.as_str()),
                SHADOW_MAP_WIDTH,
                SHADOW_MAP_HEIGHT,
                SHADOW_MAP_FORMAT,
            ),
        )]),
        vec![SHADOW_MAP],
        DEPTH_CLEAR,
    )
}

/// Builds the depthing pass of one group.
fn depthing_pass(group: &Group<'_>, camera: &Value, time: f64) -> Value {
    rasterize(
        &group.label(),
        group.shader,
        Value::structure([
            (PROJECTION.to_owned(), camera.clone()),
            (TIME.to_owned(), Value::Float(time)),
        ]),
        mesh_bindings(group).chain([(
            DEPTH_BUFFER.to_owned(),
            texture_binding(DEPTH_BUFFER_ENTRY, DEPTH_WIDTH, DEPTH_HEIGHT, DEPTH_FORMAT),
        )]),
        vec![DEPTH_BUFFER],
        DEPTH_CLEAR,
    )
}

/// Builds the lighting pass of one group for one light.
///
/// # Invariants
///
/// The pass carries the **light's** projection as well as the camera's, and they are
/// different matrices. A fragment decides it is in shadow by carrying its world position
/// into the light's clip space and comparing the depth there against the shadow map; with
/// only the camera's matrix, there is no way to do that and the shadow map is a texture no
/// shader can meaningfully read.
fn lighting_pass(
    group: &Group<'_>,
    light: &LightRead,
    camera: &Value,
    bounds: &Bounds,
    time: f64,
) -> Value {
    rasterize(
        &group.label(),
        group.shader,
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
            (
                LIGHT_PROJECTION.to_owned(),
                Value::List(
                    light_projection(light, bounds)
                        .matrix
                        .iter()
                        .copied()
                        .map(Value::Float)
                        .collect(),
                ),
            ),
            (
                LIGHT_SHADOW_TEXEL.to_owned(),
                Value::List(
                    [
                        1.0 / SHADOW_MAP_WIDTH as f64,
                        1.0 / SHADOW_MAP_HEIGHT as f64,
                    ]
                    .into_iter()
                    .map(Value::Float)
                    .collect(),
                ),
            ),
            (TIME.to_owned(), Value::Float(time)),
        ]),
        mesh_bindings(group).chain([
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

/// Returns the projection a light looks at the scene through, and the eye it is made from.
///
/// # Invariants
///
/// The shadowing pass draws through this projection and the lighting pass is handed the
/// same matrix as `light_projection`, so the two are computed here once. A pair of
/// functions that each built "the light's projection" would agree until one of them was
/// edited, and until then a shadow would be sampled at the wrong place for every surface
/// in the scene.
fn light_projection(light: &LightRead, bounds: &Bounds) -> Projection {
    let centre = bounds.centre();
    Projection {
        matrix: projection_of(
            &Bounds::around(centre, light.shadow_extent),
            light.position,
            centre,
            FRAMING_PADDING,
        ),
        eye: light.position,
    }
}

/// Returns the box the instanced meshes of a scene cover.
///
/// # Invariants
///
/// The box is over what is drawn rather than over what is authored: a mesh that no instance
/// names is not drawn, and an instance places its mesh wherever its transform says. A scene
/// with nothing drawn covers a unit box, so a projection is still well defined.
fn bounds_of_instances(meshes: &[MeshRead], instances: &[InstanceRead]) -> Bounds {
    let mut bounds: Option<Bounds> = None;
    for instance in instances {
        let mesh = &meshes[instance.mesh];
        let mut covered = Bounds {
            minimum: [f64::INFINITY; 3],
            maximum: [f64::NEG_INFINITY; 3],
        };
        for point in &mesh.points {
            let world = apply(&instance.model, point_position(point));
            for axis in 0..3 {
                covered.minimum[axis] = covered.minimum[axis].min(world[axis]);
                covered.maximum[axis] = covered.maximum[axis].max(world[axis]);
            }
        }
        if covered.minimum.iter().all(|value| value.is_finite()) {
            bounds = Some(match bounds {
                None => covered,
                Some(previous) => previous.union(covered),
            });
        }
    }
    bounds.unwrap_or_else(|| Bounds::around([0.0; 3], 1.0))
}

/// Returns the position a point carries, which is the first three numbers it declares.
fn point_position(point: &Value) -> [f64; 3] {
    let mut position = [0.0f64; 3];
    for (axis, component) in point
        .as_struct()
        .into_iter()
        .flat_map(|fields| fields.values())
        .flat_map(number_components)
        .take(3)
        .enumerate()
    {
        position[axis] = component;
    }
    position
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

/// Returns the two binding points every pass of a group carries.
fn mesh_bindings<'group>(
    group: &'group Group<'_>,
) -> impl Iterator<Item = (String, Value)> + 'group {
    [
        (
            VERTICES.to_owned(),
            buffer_binding(&group.mesh.vertex_entry, group.mesh.vertex_bytes),
        ),
        (
            INSTANCE_DATA.to_owned(),
            buffer_binding(&group.entry(), group.bytes().len() as i64),
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
