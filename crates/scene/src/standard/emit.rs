//! Turning a standard scene into the graphics and instance documents that draw it.
//!
//! # What the pipeline is
//!
//! One shadowing pass per light that was given a shadow page and per shadow casting instance,
//! then one depthing pass per instance, then **one lighting pass per instance**, then the
//! emission and post processes. The order of the emitted list is the pipeline: nothing in a
//! document says that a lighting pass runs after a shadowing pass, the order does.
//!
//! # Why the lighting pass is not per light
//!
//! It used to be, and the count was `lights * groups` for both stages. The lighting pass now
//! reads the cluster assignment the cpu produced and loops over one cluster's worth of lights
//! per fragment, so it is one pass per group however many lamps the scene has. The shadowing
//! pass still is one per light, because a render pass attaches exactly one depth image: a
//! light's page is rendered from that light's own eye, and two eyes are two passes whatever
//! the shading does. What changed there is that the number of them is bounded by the scene's
//! **pool** rather than by its light count, because a light the pool gave no page is not drawn
//! into anything and is lit unshadowed instead.
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

use definition::{FieldMap, PackedList, Path, Schema, SchemaKind, Value};

use graphics::binders::{FORMAT, HEIGHT, LAYER, LAYERS, RESOURCE, SIZE_BYTES, SOURCE, WIDTH};
use graphics::pass::{BINDERS, CLEAR_COLOR, LABEL, SHADER, TARGETS, UNIFORM};
use graphics::{derived_instance_schema, layout::packed_layout, verify_pass_inputs};

use crate::DeviceLimits;
use crate::lighting::{self as lighting_pool, ClusterLight, Clustering, LightPool};
use crate::lighting::{SPOT_CONES_ENTRY, SpotCone, spot_cone_schema, spot_cones, storage};
use crate::standard::camera::{Camera, Projection, matrix_projection, projection_value};
use crate::standard::debug::DebugChannel;
use crate::standard::frame::Frame;
use crate::standard::math::{
    Bounds, IDENTITY, Matrix, add, apply, dot, look_at, multiply, normalize, orthographic,
    perspective, projection_of, scale, subtract, transform_matrix,
};
use crate::standard::packing::{pack_points, pack_struct};
use crate::standard::pass_schemas::{
    CAMERA, CLUSTER_GRID, COLOR_FORMAT, COLOR_TARGET, DEPTH_BUFFER, DEPTH_FORMAT, DEPTH_RANGE,
    DIRECTIONAL, ENVIRONMENT, ENVIRONMENT_MAP, EXPOSURE, FORWARD, FRAME_SIZE, GLOBAL_COUNT,
    HALF_EXTENT, INSTANCE_DATA, POSITION, PRESENT_TARGET, PROJECTION, RIGHT, SHADOW_MAP_FORMAT,
    SHADOW_POOL, TIME, UP, VERTICES,
};
use crate::standard::scene_schema;
use crate::standard::scene_schema::RADIUS;
use crate::{CompileError, CompileNote};
use graphics::layout::components;
use graphics::pass::{BLEND, DEPTH, DEPTH_COMPARE, DEPTH_WRITE};

/// Field the model matrix of an instance row is carried under.
const MODEL: &str = "model";

/// Field of a point schema that says where the point is.
///
/// A point schema is free to declare whatever a mesh needs, so a mesh that carries a
/// normal and a texcoord as well as a position names the position, and the emitter reads
/// it by name rather than by position in the field order.
const POSITION_FIELD: &str = "position";

/// Field of an instance schema that says how a normal is carried through the transform.
const NORMAL_MATRIX: &str = "normal_matrix";

/// Returns whether a row schema declares a named field.
fn declares(schema: &Schema, name: &str) -> bool {
    matches!(&schema.kind, SchemaKind::Struct { fields } if fields.contains_key(name))
}

/// Returns the inverse transpose of a model matrix, as a column major four by four.
///
/// # Invariants
///
/// A model is an affine transform: a three by three linear part and a translation. The
/// translation has no effect on a normal, so only the linear part is inverted and the
/// result is written back as an affine matrix — a shader indexes it the same way it
/// indexes the model, and a three by three in sixteen floats would leave it guessing where
/// the padding was.
///
/// A model whose linear part is singular — a scale of zero on some axis — has no inverse,
/// and the normal matrix falls back to the model itself. A point on such an instance has
/// been flattened onto a plane or a line, so there is no normal to be right about.
fn normal_matrix(model: &Matrix) -> Matrix {
    // The three columns of the linear part, as rows of the matrix to invert.
    let a = [model[0], model[4], model[8]];
    let b = [model[1], model[5], model[9]];
    let c = [model[2], model[6], model[10]];
    let determinant = a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
        + a[2] * (b[0] * c[1] - b[1] * c[0]);
    if determinant.abs() < 1e-12 {
        return *model;
    }
    let reciprocal = 1.0 / determinant;
    // The inverse, by the adjugate, and transposed on the way out.
    let inverse = [
        (b[1] * c[2] - b[2] * c[1]) * reciprocal,
        (a[2] * c[1] - a[1] * c[2]) * reciprocal,
        (a[1] * b[2] - a[2] * b[1]) * reciprocal,
        (b[2] * c[0] - b[0] * c[2]) * reciprocal,
        (a[0] * c[2] - a[2] * c[0]) * reciprocal,
        (a[2] * b[0] - a[0] * b[2]) * reciprocal,
        (b[0] * c[1] - b[1] * c[0]) * reciprocal,
        (a[1] * c[0] - a[0] * c[1]) * reciprocal,
        (a[0] * b[1] - a[1] * b[0]) * reciprocal,
    ];
    [
        inverse[0], inverse[3], inverse[6], 0.0, inverse[1], inverse[4], inverse[7], 0.0,
        inverse[2], inverse[5], inverse[8], 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

/// How much wider than its box a projection is fitted.
pub const FRAMING_PADDING: f64 = 1.1;

/// Name of the instance entry holding the depth buffer every depthing pass writes.
pub const DEPTH_BUFFER_ENTRY: &str = "depth_buffer";

/// Name of the instance entry holding the image every lighting pass writes.
pub const COLOR_TARGET_ENTRY: &str = "color_target";

/// Name of the instance entry holding the image a post stage writes.
///
/// A post stage reads the shaded image and writes this one, because a pass that both reads
/// and writes one texture is a pass reading and writing the same pixels. The frame comes
/// back from here, so this is the last thing a scene draws into.
pub const PRESENT_TARGET_ENTRY: &str = "present_target";

/// Returns the entry name of the image one post process writes when it is not the last.
///
/// A chain of post processes cannot read and write one image, so every process but the last
/// writes one of its own and the next reads it.
fn post_target_entry(index: usize) -> String {
    format!("post_target_{index}")
}

/// Label of the mesh covering the frame, which a post stage draws.
pub const FULLSCREEN_MESH: &str = "fullscreen";

/// Label of the instance covering the frame.
pub const FULLSCREEN_INSTANCE: &str = "fullscreen";

/// Which stage of the pipeline a material belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Renders one light's depth map.
    Shadowing,
    /// Renders the depth of the scene.
    Depthing,
    /// Shades the scene.
    Lighting,
    /// Adds what a surface emits, once, rather than once per light.
    Emission,
    /// Draws over the whole frame once the scene is shaded.
    Post,
}

impl Stage {
    /// Returns the tag this stage is stored under in a scene document.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Shadowing => "ShadowingShader",
            Self::Depthing => "DepthShader",
            Self::Lighting => "LightShader",
            Self::Emission => "EmissionShader",
            Self::Post => "ScreenShader",
        }
    }

    /// Returns the name this stage's shader entry is suffixed with.
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Shadowing => "shadowing",
            Self::Depthing => "depthing",
            Self::Lighting => "lighting",
            Self::Emission => "emission",
            Self::Post => "post",
        }
    }

    /// Returns the prefix this stage's passes are labelled with.
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Shadowing => "shadow",
            Self::Depthing => "depth",
            Self::Lighting => "lighting",
            Self::Emission => "emission",
            Self::Post => "post",
        }
    }

    /// Returns the stage a bucket key names, or none when it names no mesh stage.
    ///
    /// The keys are the suffixes, because a bucket is a pass and the suffix is what a pass is
    /// called everywhere else: in a shader entry's name, in a pass's label, and in a
    /// diagnostic. `post` is not one of them, because a post process draws a fullscreen
    /// triangle rather than a scene instance and so has no instances to bucket.
    pub fn from_suffix(name: &str) -> Option<Self> {
        match name {
            "shadowing" => Some(Self::Shadowing),
            "depthing" => Some(Self::Depthing),
            "lighting" => Some(Self::Lighting),
            "emission" => Some(Self::Emission),
            _ => None,
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
    /// The mesh's points, packed.
    ///
    /// # Invariants
    ///
    /// This is packed whatever form the document wrote them in. A list of points is packed once
    /// here and never walked again, so a mesh is read, packed and thereafter indexed by stride —
    /// which is what makes the bounds of a scene and the packing of its buffers a fixed cost per
    /// point rather than a walk of a tree per point.
    points: PackedList,
    vertex_bytes: i64,
    vertex_entry: String,
}

/// One instance, read out of the scene document: a mesh, where it is, and what draws it.
struct InstanceRead {
    label: String,
    /// Where this instance was read from, for a message about it.
    ///
    /// # Invariants
    ///
    /// The path is kept rather than rebuilt where a note is written, because a note that
    /// names an instance has to be able to point at the row the author will edit, and the
    /// row is `instances` then a bucket then an index — a path assembled from a label would
    /// be a guess about where in the list the instance fell.
    path: Path,
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
    /// The images this instance's material names, and the fields it names them in.
    images: Vec<ImageBinding>,
}

/// One mesh material, read out of the scene document: a shader, and the data it reads.
///
/// # Invariants
///
/// A material names one shader, and that shader's stage is the only pass the material can be
/// drawn in. Which passes an instance actually appears in is the buckets it is listed under,
/// so a material is reusable across instances and an instance listed under two passes names
/// two materials.
struct MeshMaterialRead {
    label: String,
    /// The index of the shader this material is drawn with, resolved from its label.
    shader: usize,
    /// Where this material was read from, for a message about one of its fields.
    path: Path,
    /// The material's own data, for the shader's `custom_schema`.
    custom_data: Value,
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
    /// Which kind of light this is.
    ///
    /// The kind is read before the fields beside it, because a field is named by the kind
    /// that declares it, and a read result that had dropped the kind could not say which
    /// set of fields it had read. It is read rather than inferred from which fields were
    /// present, so a document naming a kind it cannot render is refused by name.
    kind: LightKind,
    label: String,
    /// Where a light that stands somewhere stands, and `None` for a kind that stands nowhere.
    ///
    /// # Invariants
    ///
    /// A field is here exactly when the kind declares it, and the kind is what says: an
    /// environment light names an image and no position, so there is nothing to read into
    /// this and a default of the origin would be a light that appears to stand in the middle
    /// of the scene. What is `None` is reached through [`LightRead::record_position`] and
    /// [`LightRead::shadow_projection`], which are the two places a record wants a number
    /// where a kind has none, and both say in words what they did.
    position: Option<[f64; 3]>,
    /// The unit vector the light travels along, and `None` for a kind that shines from a place
    /// rather than a direction.
    ///
    /// # Invariants
    ///
    /// A field is here exactly when the kind declares it, and it is **normalised where it is
    /// read** so that everything downstream gets a unit vector: the schema says the length does
    /// not matter, so a document may write `[0, -20, 0]` and get the same light as `[0, -1, 0]`.
    /// A spot light's cone carries the same direction as its own axis, and the two are read from
    /// one number rather than from two fields that would then be able to disagree.
    direction: Option<[f64; 3]>,
    colour: [f64; 3],
    /// The half width of the volume a light shadows, and `None` for a kind that casts none.
    shadow_extent: Option<f64>,
    /// How far the light reaches, when the document says.
    ///
    /// It is a field of the light because a light's reach is the light's own, and a scene
    /// that mixes a lantern with a sun has to be able to say which is which. A **point** light
    /// that says nothing is a **global** light, and a **spot** light that says nothing is
    /// answered by the reach of its own cone; a kind that takes no cluster slot has no reach
    /// to state. [`LightRead::reach`] resolves those three and [`LightRead::membership`] says
    /// which of them a light is.
    radius: Option<f64>,
    /// The cone the light shines in.
    ///
    /// # Invariants
    ///
    /// It is a cone for a spot light and [`SpotCone::whole_sphere`] for every other kind, so
    /// the run of cones has one record per light whatever kinds a scene mixes. A light with
    /// no cone is not one whose cone is absent: the record run is indexed by the same number
    /// as the light record run, and a hole in one of them is a fragment that has to test
    /// before it can read.
    cone: SpotCone,
}

impl LightRead {
    /// Returns how far this light reaches, and what answers for the kinds that do not say.
    ///
    /// # Invariants
    ///
    /// Three answers and no fourth, and the third is the one that used to be a wrong number:
    ///
    /// - A light that **named** a reach has said it, and the number is that number whatever
    ///   the kind is.
    /// - A **spot** light that may not have is answered by the reach of its own cone. The
    ///   volume that cone cuts out of a scene of half width `shadow_extent` ends in a disc,
    ///   and the distance from the light to the rim of that disc is what the cone reaches —
    ///   see [`SpotCone::reach`]. Answering it by something else would be a light whose
    ///   **culling** sphere is smaller than the cone it is drawing, so the light would be
    ///   dropped from the clusters at the edge of its own cone and the outer angle would
    ///   never be seen. A lamp standing higher above the floor than the box is wide hits that
    ///   on the first try, and what the author sees is a grid-aligned rectangle where the
    ///   cone's own circle should be.
    /// - A **point** light that named nothing is **global**, and `None` is what says so.
    ///   There is no default behind it, and there never should be: a light standing outside
    ///   the scene it lights is culled short of its own reach by any number the compiler
    ///   could pick, dropped from every cluster past that, and the boundary between lit and
    ///   unlit then comes out quantised to the **cluster grid** — a terminator drawn as a
    ///   staircase of screen-aligned rectangles, with nothing in the frame to say a number
    ///   was wrong. Saying nothing is the one answer that cannot be short of anything,
    ///   because it asks for no culling at all.
    fn reach(&self) -> Option<f64> {
        match (self.radius, self.kind) {
            (Some(reach), _) => Some(reach),
            (None, LightKind::Spot) => Some(self.cone.reach(self.shadow_extent.unwrap_or(0.0))),
            (None, _) => None,
        }
    }

    /// Returns how a shading pass reaches this light, which is one question with three answers.
    ///
    /// # Invariants
    ///
    /// The three are asked together rather than one predicate at a time, because they are one
    /// decision: a light is either walked into clusters, evaluated by every fragment, or
    /// neither — and the third is the case of the two kinds that stand nowhere, whose light
    /// the pass reads out of its own uniform and its own map rather than out of a record.
    /// Answering them separately is how a directional light ends up in the global run, and a
    /// directional light in the global run is a sun evaluated twice.
    ///
    /// - A **point** light that named no reach is [`Membership::Global`], and one that named
    ///   a reach is [`Membership::Clustered`] until that reach is found to contain the whole
    ///   scene, which is a **promotion**: a light that reaches everywhere has no culling left
    ///   to do, and the slot it holds in every cluster it reached buys nothing. What it does
    ///   to the picture is [`LightRead::covers_scene`]'s to say, and it is not nothing.
    /// - A **spot** light is always clustered. A cone is a light that does not reach the
    ///   whole scene, and one promoted on the strength of a radius would be a cone drawn past
    ///   the edge of the frame.
    /// - A **directional** and an **environment** light are [`Membership::Exempt`]: they
    ///   took no cluster slot before this rule existed and they take none now, and the
    ///   shading pass reaches them through the two uniform fields it already carried.
    fn membership(&self, bounds: &Bounds) -> Membership {
        if !self.kind.takes_cluster_slot() {
            return Membership::Exempt;
        }
        match self.reach() {
            None => Membership::Global,
            Some(reach) => {
                if self.covers_scene(reach, bounds) {
                    Membership::Global
                } else {
                    Membership::Clustered
                }
            }
        }
    }

    /// Returns whether a sphere of this reach around the light holds the whole scene.
    ///
    /// # Invariants
    ///
    /// The test is against the **farthest corner of what the scene draws**, and not against
    /// the box the camera frames or the far plane: a fragment exists only where geometry is,
    /// so the scene's own bounds are the whole of what a light can be wanted to reach.
    /// Comparing against the frustum instead would promote almost nothing — the boxes of the
    /// far slices stand a long way beyond the geometry — and comparing against less would
    /// promote a light that stops short of a corner the scene draws.
    ///
    /// **Promoting now saves a slot and buys nothing else, because a cluster's box contains
    /// its cell** (see `View::box_of`). A box that holds the cell and a sphere that holds the
    /// scene cannot fail their distance test, so a promoted light was never going to be
    /// dropped from any cluster holding a surface; the slots it gives up are the ones the far
    /// slices spend on empty space. What the rule really buys is immunity. A global light is
    /// evaluated for every fragment and does not depend on the cluster walk being right, which
    /// is what makes "name no reach" a good default: an author who forgets the number gets a
    /// light that is wrong everywhere and says so, rather than one that is subtly absent at
    /// the edge of the frame and says nothing.
    ///
    /// The reach itself is not applied to anything: a record carries the radius the document
    /// named and the shading loop has no test against it — the radius is a culling number, not
    /// a cut-off — so a promoted light reaches past what it named, to every fragment there is.
    /// That is the one thing about a promotion an author cannot see in their own document, and
    /// it is the honest reading of a light whose reach already holds the whole scene.
    fn covers_scene(&self, reach: f64, bounds: &Bounds) -> bool {
        let Some(position) = self.position else {
            return false;
        };
        let offset = subtract(position, bounds.centre());
        let corner = (0..3)
            .map(|axis| {
                let half = bounds.half_width(axis);
                (offset[axis].abs() + half).powi(2)
            })
            .sum::<f64>();
        reach >= corner.sqrt()
    }

    /// Returns the position this light's record carries.
    ///
    /// # Invariants
    ///
    /// A light that stands nowhere has no position, and the record has a slot for one
    /// whatever the kind is: the record's layout is the same for every light because the
    /// clustered loop indexes it by a light and not by a kind. The origin is written rather
    /// than a value that could be mistaken for a place, and nothing reads it: an exempt light
    /// is in no cluster list and in no global run, so no fragment ever loads its record.
    fn record_position(&self) -> [f64; 3] {
        self.position.unwrap_or([0.0; 3])
    }

    /// Returns the projection a fragment looks this light's page up through.
    ///
    /// A light that casts no shadow is the identity, which is the same answer the light
    /// record's own documentation gives for "a light of a kind that casts no shadow". It is
    /// the identity rather than a zero matrix because a zero matrix reads as a projection
    /// that collapses everything to a point, and a page index of minus one is already the
    /// sentinel that says this record is not to be looked through.
    fn shadow_projection(&self, bounds: &Bounds) -> Matrix {
        self.shadow_volume(bounds)
            .map_or(IDENTITY, |(matrix, _)| matrix)
    }

    /// Returns the projection and the eye a shadowing pass draws this light's page through, or
    /// nothing for a light that casts no shadow.
    ///
    /// # Invariants
    ///
    /// A light has a shadow volume or it has not, and the two fields that say so are read
    /// together rather than one after the other: a kind with a position and no volume would be
    /// a document whose two answers had come to disagree, and one check is the place that
    /// cannot.
    ///
    /// The **kind** chooses the shape, because the same two numbers are three different shapes:
    /// a point light's volume is a box looked at from where it stands, a spot light's is a
    /// cone, and a directional light's is a box looked at **along** a direction. Which is why
    /// the arm is chosen before either optional field is read — a light that has no position
    /// and a light that has no direction are different documents, and one `?` on each in turn
    /// would be a way of writing the match twice.
    fn shadow_volume(&self, bounds: &Bounds) -> Option<(Matrix, [f64; 3])> {
        let extent = self.shadow_extent?;
        match self.kind {
            LightKind::Directional => Some(directional_projection(self.direction?, extent, bounds)),
            _ => {
                let position = self.position?;
                Some(match self.kind {
                    LightKind::Spot => spot_projection(position, &self.cone, extent, bounds),
                    _ => light_projection(position, extent, bounds),
                })
            }
        }
    }
}

/// Which kind of light a scene names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LightKind {
    /// A light at a position, and nothing else.
    Point,
    /// A point light that shines inside a cone.
    Spot,
    /// A light from a direction rather than from a place.
    Directional,
    /// Light read from an image, which casts no shadow.
    Environment,
}

impl LightKind {
    /// Returns the tag this kind is stored under in a scene document.
    const fn tag(self) -> &'static str {
        match self {
            Self::Point => "Point",
            Self::Spot => "Spot",
            Self::Directional => "Directional",
            Self::Environment => "Environment",
        }
    }

    /// Returns whether a light of this kind takes a slot in a cluster.
    ///
    /// # Invariants
    ///
    /// A point light and a spot light stand somewhere and shine into a volume, so what they
    /// light is that volume and a cluster inside it wants them. A directional light and an
    /// environment light cover the whole view at every depth, so every cluster wants them
    /// equally and a cluster slot spent on one is a slot not spent on culling anything. They
    /// take no slot here and get a path of their own, which stage 3b builds.
    ///
    /// A point light and a spot light reach this compiler, and they are the two kinds that
    /// take a slot: a spot light is a point light with a cone, and it is culled by its reach
    /// exactly as one is, with the cone scaling what survives rather than replacing the
    /// culling. A directional light is the one kind here that is exempt and still refused.
    ///
    /// This is the question of the **kind** and not of the light: whether a point light
    /// actually takes a slot is asked by [`LightRead::membership`], which promotes a light
    /// that names no reach or a reach that holds the whole scene. A kind that could be
    /// promoted would make this answer false for it.
    const fn takes_cluster_slot(self) -> bool {
        matches!(self, Self::Point | Self::Spot)
    }

    /// Returns whether this compiler draws a light of this kind.
    ///
    /// # Invariants
    ///
    /// A point light stands somewhere and is drawn through a cluster list. A **spot** light
    /// does the same thing through the same list, and differs only in the cone the shading
    /// loop scales it by: it takes a cluster slot by exactly the rule a point light does, so
    /// there is one list and one loop rather than two.
    ///
    /// An environment light is an image, and it is drawn by the one ambient term every fragment
    /// computes beside that loop: it is exempt from clustering, so it takes no record a fragment
    /// would load and no page, and it is a **stated** input of the lighting pass rather than
    /// something the loop stumbles on.
    ///
    /// A **directional** light is a third way of being drawn, and the only one that is
    /// exempt from clustering and still shadows. It stands nowhere, so it is in no cluster
    /// list and no fragment has a path to a record; like the environment it is a **stated**
    /// input of the lighting pass, evaluated by every fragment once beside the loop. It
    /// differs from the environment in the one way that matters — it casts a shadow — so it
    /// carries a page and the environment does not.
    ///
    /// Every kind this stage knows is now drawn, which makes this predicate the statement of
    /// which light kinds exist rather than a list of the ones a pass is missing.
    ///
    /// It is still asked, and it is still the right place to ask, because it is the **gate**
    /// rather than a list: a fifth kind added to the enumeration above and not added here is
    /// refused by name before any of its fields are read, which is a document error rather than
    /// a light quietly drawn as a point. An always-true check on today's four kinds is what a
    /// gate that has not been needed yet looks like.
    const fn is_emitted(self) -> bool {
        matches!(
            self,
            Self::Point | Self::Spot | Self::Directional | Self::Environment
        )
    }
}

/// How a shading pass reaches one light, which is one question with three answers.
///
/// # Invariants
///
/// The three are exhaustive and disjoint. A light is walked into clusters, evaluated by every
/// fragment, or neither — and **neither** is the case of the two kinds that stand nowhere,
/// whose light the pass reads out of its own uniform and its own map. Folding the third into
/// the second is the mistake this type exists to prevent: an environment light read as a
/// global record would be a lamp at the origin, because a light with no position has a record
/// carrying the origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Membership {
    /// Assigned to the clusters its reach covers, and found there by a fragment.
    Clustered,
    /// In no cluster list, and evaluated by every fragment out of the record run.
    Global,
    /// In no cluster list and in no global run, because the pass carries it in its uniform.
    Exempt,
}

/// The two documents a standard scene compiles to.
pub struct Emitted {
    /// The graphics document.
    pub graphics: Value,
    /// The instance document.
    pub instance: Value,
    /// What the author is told about what the compilation could not do exactly.
    pub notes: Vec<CompileNote>,
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
///
/// `limits` is the machine the documents are being written for. It is a parameter rather than
/// a constant because a width, a page size and a page count are all numbers a document may ask
/// past what the hardware holds, and a compiler that answered for the hardware on its behalf
/// would be answering for a machine no reader of the output has.
pub fn emit(
    scene: &Value,
    camera_index: usize,
    limits: DeviceLimits,
    debug: &[DebugChannel],
) -> Result<Emitted, CompileError> {
    // The images come before the lights because an environment light names one of them by
    // label, and a label resolved against a list that had not been read is a label resolved
    // against nothing. Nothing else about the two reads depends on the other.
    let images = read_images(scene)?;
    let (lights, environment) = read_lights(scene, &images)?;
    // The environment a lighting pass is bound, whether the scene named one or not. It is
    // resolved here so that the image is moved rather than looked up a second time, and so
    // that the one branch deciding which map it is is the branch that also knows the flag.
    let environment = match environment {
        Some(environment) => EnvironmentMap {
            entry: image_entry(&environment.image.label),
            image: environment.image,
            colour: environment.colour,
            present: true,
        },
        None => EnvironmentMap::absent(),
    };
    let shaders = read_shaders(scene)?;
    let materials = read_materials(scene, &shaders)?;
    let (meshes, mut instance) = read_meshes(scene, &shaders)?;
    let instances = read_instances(scene, &meshes, &materials, &shaders, &images)?;
    // The camera frames what is drawn rather than what is authored, because an instance
    // may place its mesh anywhere and a mesh no instance names is not drawn at all. The
    // same walk leaves each instance's own box behind, which is what the notes about this
    // frame are read off — see `camera_notes`.
    let instance_bounds = bounds_of_instances(&meshes, &instances);
    let bounds = instance_bounds.scene;
    let time = read_time(scene)?;
    // What the post stage multiplies the frame by before its own grade, or nothing at all
    // for a scene that names none — which is the answer that keeps the field additive. See
    // `read_exposure` and `scene_schema::EXPOSURE` for what the number means.
    let exposure = read_exposure(scene)?;
    // The frame is read before the camera because the camera's field of view is widened by
    // the frame's aspect: a wide frame is wider rather than cropped, and which frame a
    // scene is drawn into is the scene's decision. Its notes come with it, so the note list
    // starts here rather than beside the cluster assignment further down.
    let (frame, mut notes) = Frame::read(scene, limits);
    // One camera of the scene, chosen by the compile. Every camera a scene names is rendered,
    // which means a compile is a compile for one of them: the projection every pass draws
    // through is a camera's, so the choice is made here rather than by a renderer afterwards.
    let cameras = at(scene, &Path::root(), scene_schema::CAMERAS).ok();
    let camera_path = Path::root().key(scene_schema::CAMERAS).index(camera_index);
    let chosen = cameras
        .and_then(Value::as_list)
        .and_then(|cameras| cameras.get(camera_index));
    let camera = Camera::read(chosen, &camera_path, &bounds, frame.aspect())?;
    let camera_basis = camera.projection();
    // What this frame is of, said out loud. Three runs of one task each put a camera inside
    // geometry and each found out by looking at a broken picture — a pond filling the frame
    // with a water jar, a hundred metre hill with the eye in it, a gatehouse nobody could see
    // past — and every one of those is a fact this function already holds: the eye is a
    // field, the target is a field, and the box every instance covers came out of the walk
    // above. A note is the whole of the answer, and none of these three refuses to compile,
    // because a scene may legitimately want a camera inside a volume. See `camera_notes`.
    //
    // **A camera the scene did not write says nothing at all.** `chosen` is `None` when the
    // scene names no camera, or names fewer than the compile was asked for, and the frame is
    // then the convention's: pulled back until the whole box fits, which is by construction
    // the camera least able to bury itself — its eye stands three half widths off the centre
    // of the box, and no instance's box reaches there. A note about a camera nobody wrote
    // points at a field that is not in the document and asks for a change nobody can make.
    if chosen.is_some() {
        camera_notes(
            &camera_basis,
            camera.target,
            &camera_path,
            &instances,
            &instance_bounds,
            &mut notes,
        );
    }
    // What the scene's own size does to a constant the document cannot set. **This one is
    // not about a camera**, so it is not behind the `chosen` gate above: a scene that names
    // no camera is still a scene with a size, and the falloff is a function of the size
    // whether or not anybody chose where to stand. It reads the box the walk above already
    // left behind, so it costs one square root.
    scale_notes(&bounds, &mut notes);
    let camera = projection_value(&camera_basis);

    // Which lights reach which part of the view, and the three buffers that say so. This is
    // here rather than in a pass because the instance is built before it is rendered: for a
    // given scene and camera the frustum, the lights and the boxes are all fixed, so the
    // assignment is a pure function of the documents and belongs beside the rest of the
    // compilation. The shadow page pool is read beside it because the pages are assigned from
    // the same ranking, in the same walk over the same lights.
    let clustering = Clustering::read(scene)?;
    let (light_pool, pool_notes) = lighting_pool::read_light_pool(scene, limits)?;
    notes.extend(pool_notes);
    // The light list, with the **global** lights first, and every other run built from that
    // one order. The order is not a preference: a shading pass reaches a global light by
    // index rather than through a cluster list, and the records, the cones, the cluster
    // entries and the shadow pages are all indexed by the same number, so writing the global
    // records first is what makes `global_count` a count rather than a list. The sort is
    // stable, so a scene's own order is kept within each group and one scene compiles to one
    // set of bytes.
    let mut ordered: Vec<(Membership, LightRead)> = lights
        .into_iter()
        .map(|light| {
            let membership = light.membership(&bounds);
            (membership, light)
        })
        .collect();
    ordered.sort_by_key(|(membership, _)| membership != &Membership::Global);
    let cluster_lights: Vec<ClusterLight> = ordered
        .iter()
        .map(|(membership, light)| ClusterLight {
            label: light.label.clone(),
            position: light.record_position(),
            colour: light.colour,
            // The radius a record carries is the one the document named, and **zero** for a
            // global light that named none. Zero is what the page ranking wants: a light that
            // reaches everything is at distance nothing from the view, so it outranks every
            // other light for a page. A light promoted for a named radius keeps it, so its
            // rank is the same number it had before it was promoted.
            radius: light.reach().unwrap_or(0.0),
            // The projection is computed here rather than in the lighting module, because
            // building one is a property of a view and the lighting module deliberately has
            // no view. It is the same matrix the shadowing pass draws its page through.
            shadow_projection: light.shadow_projection(&bounds),
            clustered: *membership == Membership::Clustered,
            global: *membership == Membership::Global,
            // Whether the light casts a shadow, which is asked apart from the projection above
            // because that field answers the same question for two different reasons: a light
            // the pool had no room for and a light that casts nothing both carry a matrix
            // nothing looks through, and the page ranking has to tell them apart.
            casts_shadow: light.shadow_extent.is_some(),
        })
        .collect();
    // The document's own light run, in the order the records above were written in. Every
    // later index into it — the shadowing pass per light, the directional light's page, the
    // cone run — is an index into this list, so the two cannot come apart.
    let lights: Vec<LightRead> = ordered.into_iter().map(|(_, light)| light).collect();
    let clusters = lighting_pool::assign(&clustering, &cluster_lights, &camera_basis, &light_pool)?;
    notes.extend(clusters.notes(&clustering, &Path::root().key(scene_schema::CLUSTERING)));
    notes.extend(lighting_pool::pool_notes(
        &cluster_lights,
        &light_pool,
        &clusters,
    ));

    add_targets(&mut instance, frame);
    // The environment's map is an **image**, so its instance entry is the entry that image
    // already has and a material naming the same image would name: one map, uploaded once,
    // however many things read it. It is therefore written only when it is not already there,
    // which is the one place in this function that inserts an entry that may already exist.
    insert_entry_once(
        &mut instance,
        environment.entry.clone(),
        environment_payload(&environment.image),
    );
    for (entry, payload) in clusters.entries() {
        insert_entry(&mut instance, entry.to_owned(), payload)?;
    }
    // The cone run is packed here rather than by the assignment, because a cone is a property of
    // the light and the assignment knows nothing about cones: what reaches which cluster, which
    // page was bought and which light was dropped are three other questions. What it shares with
    // the record run is its **order** — one record per light in the order the scene lists them —
    // and that is what lets a cluster index be an index into both.
    let cones = spot_cones(&lights.iter().map(|light| light.cone).collect::<Vec<_>>())?;
    insert_entry(
        &mut instance,
        SPOT_CONES_ENTRY.to_owned(),
        storage(cones.bytes(), &spot_cone_schema()),
    )?;
    insert_entry(
        &mut instance,
        SHADOW_POOL.to_owned(),
        texture_array(
            light_pool.page_width,
            light_pool.page_height,
            light_pool.pages as i64,
            SHADOW_MAP_FORMAT,
        ),
    )?;
    for shader in &shaders {
        insert_entry(&mut instance, shader.label.clone(), shader_entry(shader))?;
    }

    // One pass per group, and one instance entry per group, because the instances in a group are
    // drawn together by one instanced draw rather than one draw apiece.
    let mut passes = Vec::new();
    // A shadowing pass per light that was **given a page**. A light the pool turned away is
    // drawn into nothing, and drawing it into page zero would hand it a neighbour's shadow,
    // so it is lit unshadowed instead and there is no pass for it at all.
    for (index, light) in lights.iter().enumerate() {
        let Some(page) = clusters.page_of(index) else {
            continue;
        };
        for group in groups(
            &instances,
            &meshes,
            &shaders,
            Stage::Shadowing,
            Some(&light.label),
        ) {
            let images = declare_images(&mut instance, &group, &images)?;
            insert_entry(&mut instance, group.entry(), group.payload())?;
            passes.push(shadowing_pass(
                &group,
                light,
                &bounds,
                &light_pool,
                page,
                time,
                &images,
            ));
        }
    }
    for group in groups(&instances, &meshes, &shaders, Stage::Depthing, None) {
        let images = declare_images(&mut instance, &group, &images)?;
        insert_entry(&mut instance, group.entry(), group.payload())?;
        passes.push(depthing_pass(&group, &camera, frame, time, &images));
    }
    // The directional light as the pass carries it, found by the kind rather than by a separate
    // return from `read_lights`: the light is in the same run as every other one and in the same
    // order, so the page that was bought for it is the page of the same index.
    let directional = DirectionalLight::read(&lights, &clusters, &bounds);
    // The lighting stage names no light, so its groups carry none and its pass labels do not
    // move when a light is added. That is the whole of what the clustered pass bought.
    for group in groups(&instances, &meshes, &shaders, Stage::Lighting, None) {
        let images = declare_images(&mut instance, &group, &images)?;
        insert_entry(&mut instance, group.entry(), group.payload())?;
        passes.push(lighting_pass(
            &group,
            &camera,
            &clustering,
            &clusters,
            &cones,
            &light_pool,
            &environment,
            &directional,
            frame,
            time,
            &images,
        ));
    }
    // Emission runs once per instance rather than once per light, which is the whole reason it
    // is a stage: what a surface emits is not lit, so a pass per light would count it as many
    // times as there are lights. It adds into the shaded image, so the order against the
    // lighting passes does not change the sum; it sits after them so that the first pass to
    // write the target is a lighting pass, which is what clears it to unlit.
    for group in groups(&instances, &meshes, &shaders, Stage::Emission, None) {
        let images = declare_images(&mut instance, &group, &images)?;
        insert_entry(&mut instance, group.entry(), group.payload())?;
        passes.push(emission_pass(&group, &camera, frame, time, &images));
    }
    // The post processes are the last passes, in the order the scene lists them. Everything
    // each reads was written above it: the first reads the shaded image and the last writes
    // the frame, and each one between reads what the one before it wrote. A chain of two is
    // therefore two passes over the frame rather than two passes reading the same image.
    let post = read_post_processes(scene, &shaders, &images)?;
    // An exposure is a field the **post** uniform carries, so a scene that names one and runs
    // no post process has named a number nothing reads. That is said rather than refused and
    // rather than applied somewhere else: the frame of a scene with no grade is the shaded
    // image itself, and a multiplier the pipeline had nowhere to put would have to be either
    // dropped silently or folded into a light, which is the guessing this field exists to
    // replace. The path is the field, which is the one thing in the document to change.
    if let Some(exposure) = exposure.filter(|_| post.is_empty()) {
        notes.push(CompileNote {
            path: Path::root().key(scene_schema::EXPOSURE),
            message: format!(
                "this scene names an exposure of {exposure} but declares no post process, so \
                 nothing reads it: the frame is the shaded image the lighting and emission \
                 passes accumulated, and no grade multiplies it. This compile draws it as \
                 written"
            ),
        });
    }
    for (index, process) in post.iter().enumerate() {
        // A post shader that cannot read the exposure is **refused**, and this is the check
        // that makes the field mean one thing.
        //
        // A pass's uniform is uploaded with the layout its **own shader declares**
        // (`graphics::inputs::uniform_schema`), so a field the interface does not declare is
        // not a field the frame disagrees about: it is a field the upload drops. A scene that
        // named an exposure and got a clean compile and an unexposed frame is the exact shape
        // of failure this whole field exists to remove — a number that does not do what its
        // name says, in a transform the author can neither observe nor query — so the document
        // is refused with the two places to change rather than drawn and forgotten.
        if exposure.is_some()
            && !declares_uniform_field(&shaders[process.shader].pass_schema, EXPOSURE)
        {
            return Err(CompileError::Emission {
                reason: format!(
                    "at {}: this scene names an exposure, which the compiler stamps into the \
                     uniform of every post pass, but the post shader `{}` does not declare it. A \
                     pass's uniform is uploaded with the layout its own shader declares, so the \
                     number would be dropped and the frame drawn unexposed. Build that shader's \
                     `pass_schema` with `post_pass_schema_exposed()` and declare `exposure: f32` \
                     in its uniform, or take the field off the scene",
                    Path::root().key(scene_schema::EXPOSURE),
                    shaders[process.shader].label,
                ),
            });
        }
        let source = if index == 0 {
            COLOR_TARGET_ENTRY.to_owned()
        } else {
            post_target_entry(index - 1)
        };
        let target = if index + 1 == post.len() {
            PRESENT_TARGET_ENTRY.to_owned()
        } else {
            let entry = post_target_entry(index);
            instance.insert(
                entry.clone(),
                texture(frame.width, frame.height, COLOR_FORMAT),
            );
            entry
        };
        let binders = declare_read_images(&mut instance, &process.images, &images)?;
        let (pass, entry, vertex_entry) = post_pass(
            &shaders[process.shader],
            &camera_basis,
            frame,
            time,
            exposure,
            &source,
            &target,
            &binders,
        )?;
        insert_entry(&mut instance, vertex_entry, entry)?;
        passes.push(pass);
    }

    // The channels are asked for by the caller rather than by the scene, and they are emitted
    // **after** the post processes. That order is what the declared outputs below depend on: a
    // channel writes a target of its own, so it can sit anywhere, and putting it last would make
    // it the last colour pass — which is the frame under the convention this field replaces.
    // The frame is therefore named here rather than left to be whichever pass wrote last, and
    // naming it is what lets a channel be appended at all.
    let frame_target = if post.is_empty() {
        COLOR_TARGET_ENTRY
    } else {
        PRESENT_TARGET_ENTRY
    };
    // A channel that reconstructs a pixel's position reads it out of the depth buffer, so it
    // covers what the `depthing` bucket drew and nothing else. A scene that lists no instance
    // under `depthing` therefore gets a position image with nothing in it, and that is worth
    // saying: the image is empty because no depth was written, not because the scene draws
    // nothing, and a caller looking at a blank image has no way to tell those apart. The path is
    // the bucket, because that is the field to change.
    //
    // **The gate is on the channels that read depth rather than on there being one.** A `grade`
    // image is a copy of the shaded image and holds a full frame of a scene that draws nothing
    // at all, so this note would be false about it.
    if debug.iter().any(|channel| channel.reads_depth())
        && !instances
            .iter()
            .any(|instance| instance.stage == Stage::Depthing)
    {
        notes.push(CompileNote {
            path: Path::root()
                .key(scene_schema::INSTANCES)
                .key(Stage::Depthing.suffix()),
            message: "a debug position image is read out of the depth buffer, and this scene \
                      lists no instance under `depthing`: the image will be empty because nothing \
                      wrote a depth, not because the scene draws nothing"
                .to_owned(),
        });
    }
    passes.extend(debug_passes(debug, &mut instance, &camera_basis, frame)?);

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
    fields.insert(
        graphics::OUTPUTS_FIELD.to_owned(),
        debug_outputs(frame_target, debug),
    );
    let graphics = Value::Struct(fields);

    // A compiler that emitted a pass its own shader rejects has emitted nothing useful,
    // so it is refused here rather than at render time.
    for pass in graphics::passes(&graphics).map_err(emission)? {
        verify_pass_inputs(&pass, &instance).map_err(emission)?;
    }

    Ok(Emitted {
        graphics,
        instance,
        notes,
    })
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

/// The environment one lighting pass is shaded against, which is what an `Environment` light is.
///
/// # Invariants
///
/// It is returned beside the lights rather than found again in [`emit`], because resolving an
/// environment light's image is the one part of reading a light that can fail: the label has to
/// name an image the scene declares. A second lookup would be a second answer to a question that
/// has already been asked and answered, and the failure it would have to handle cannot happen.
struct Environment {
    /// The image the light names, copied out of the scene's images.
    ///
    /// It is **copied** rather than borrowed so that the map a pass binds and the map a uniform
    /// describes are one value, moved rather than looked up again. The copy is one image, once
    /// per compile.
    image: ImageRead,
    /// The light's colour scaled by its intensity, which is how bright the environment is.
    colour: [f64; 3],
}

/// What one lighting pass binds and declares for the environment.
///
/// # Invariants
///
/// This is never absent. The scene's own environment image when the scene names an
/// `Environment` light, and a one texel black image when it names none, because the binding
/// point is bound by **every** lighting pass: a binding that came and went with the document
/// would move every binding number after it, since the device numbers a pass's binders in name
/// order, and a shader would then have to be written for one scene and not for another.
///
/// `present` is the flag the shader tests, and it is written rather than inferred. A scene with
/// no environment light samples a black map, and a shader that could not tell that from an
/// environment that happens to be black would make the darkness of a frame a property of a
/// texture rather than of a decision anybody made.
struct EnvironmentMap {
    /// The image the map is uploaded from and the shader samples.
    image: ImageRead,
    /// The instance entry the map's pixels are uploaded from.
    ///
    /// For the scene's own environment this is the entry its **image** already has, so an
    /// environment map that is also a material's texture is one upload; see
    /// [`insert_entry_once`]. The absent map is not a scene image and is given a name of its
    /// own rather than one built from [`image_entry`], because an entry named after an image
    /// an author never declared is a name in the image namespace that means nothing.
    entry: String,
    /// The light's colour scaled by its intensity, or zero when the scene names no environment.
    colour: [f64; 3],
    /// Whether the scene named an `Environment` light at all.
    present: bool,
}

impl EnvironmentMap {
    /// Returns the map a scene naming no environment light is bound, which is black.
    fn absent() -> Self {
        Self {
            image: ImageRead {
                label: ABSENT_ENVIRONMENT_LABEL.to_owned(),
                width: 1,
                height: 1,
                format: COLOR_FORMAT.to_owned(),
                pixels: vec![0, 0, 0, 255],
            },
            entry: ABSENT_ENVIRONMENT_ENTRY.to_owned(),
            colour: [0.0; 3],
            present: false,
        }
    }

    /// Returns the four numbers the lighting pass's uniform carries.
    ///
    /// Three of them are the colour and the fourth is [`Self::present`]. A single four
    /// component vector rather than a colour and a flag beside it, because two numbers the
    /// shader could disagree about are one too many: the shader reads the vector and the flag
    /// is already in it.
    fn uniform(&self) -> Value {
        let present = if self.present { 1.0 } else { 0.0 };
        Value::List(
            [self.colour[0], self.colour[1], self.colour[2], present]
                .into_iter()
                .map(Value::Float)
                .collect(),
        )
    }
}

/// The scene's directional light as a lighting pass's uniform carries it.
///
/// # Invariants
///
/// There is exactly one of these per compile, present or not, because a lighting pass carries
/// one such field. A scene naming a second directional light is refused in [`read_lights`]
/// rather than one of them being dropped, which is the environment's rule applied to the
/// second exempt kind that has a uniform to collide in.
///
/// It is here rather than in a light record for the reason the environment is: the record is
/// read through a cluster list, and this light is in no list, so a fragment has no path to a
/// record to read. Every fragment evaluates it unconditionally, which is the correct cost —
/// there is no culling to do and no light to choose between, and it is one BRDF evaluation
/// beside a loop that already runs.
///
/// The record it is still given is dead weight, and it is kept on purpose: one light one
/// record is what makes a cluster index a plain index into an array. See
/// `.agents/proposals/environment-light-limits.md`.
struct DirectionalLight {
    /// The unit vector the light travels along, which is also the way a surface faces it.
    direction: [f64; 3],
    /// The light's colour scaled by its intensity.
    colour: [f64; 3],
    /// The orthographic matrix a fragment reads the light's page through, and the identity for
    /// a scene that names none — the same answer a light of a kind that casts no shadow gives
    /// anywhere else, and for the same reason.
    shadow_projection: Matrix,
    /// The page of the pool the light's shadow was rendered into, as [`lighting_pool::NO_PAGE`]
    /// for a light that was given none.
    ///
    /// It is a number in the direction's fourth component rather than a field of its own, so
    /// the page is read and tested in one comparison exactly as a record's own page is. A
    /// light the pool had no room for is lit unshadowed by design, and that design is stated
    /// once, beside [`shadowed`], rather than restated here for a second kind of light.
    page: f64,
    /// Whether the scene named a `Directional` light at all.
    present: bool,
}

impl DirectionalLight {
    /// Returns the scene's one directional light as a pass carries it, or the absent one.
    ///
    /// # Invariants
    ///
    /// Found by **kind** rather than taken from a second return, because the light is already
    /// in the run with every other one and the run is the thing whose order the cluster
    /// indices and the pages are both expressed in. Taking it from the same run is what makes
    /// the page found here the page the record of the same index carries, and a second list
    /// would be a second place for the two to come apart.
    ///
    /// A scene naming more than one is already refused in [`read_lights`], so the first found
    /// is the only one there is.
    fn read(lights: &[LightRead], clusters: &lighting_pool::Clusters, bounds: &Bounds) -> Self {
        let Some((index, light)) = lights
            .iter()
            .enumerate()
            .find(|(_, light)| light.kind == LightKind::Directional)
        else {
            return Self::absent();
        };
        // The projection is the same matrix the record of this index carries and the shadowing
        // pass drew its page through, and the page is that record's own page. Reading either
        // from somewhere else would be a second answer to a question with one.
        let page = clusters
            .page_of(index)
            .map_or(lighting_pool::NO_PAGE, |page| page as f64);
        Self {
            direction: light.direction.unwrap_or([0.0, -1.0, 0.0]),
            colour: light.colour,
            shadow_projection: light.shadow_projection(bounds),
            page,
            present: true,
        }
    }

    /// Returns the light a scene naming no directional light carries, which is the one that
    /// lights nothing.
    fn absent() -> Self {
        Self {
            direction: [0.0, -1.0, 0.0],
            colour: [0.0; 3],
            shadow_projection: IDENTITY,
            page: lighting_pool::NO_PAGE,
            present: false,
        }
    }

    /// Returns the three fields the lighting pass's uniform carries for this light.
    ///
    /// The fields are in the order the interface schema sorts them into, which is by name:
    /// `colour`, `direction`, `shadow_projection`. A nested struct rather than three fields of
    /// its own, because one light is one thing: the record run has one entry per light for
    /// exactly this reason, and a uniform reading as `directional_colour` beside
    /// `directional_projection` would be one light spelled three times.
    ///
    /// It is **written for every lighting pass**, present or not, because the uniform's layout
    /// is the same for every scene. A field that came and went with the document would put
    /// every field after it somewhere else — see [`EnvironmentMap`] for the same argument about
    /// the map.
    fn uniform(&self) -> Value {
        let present = if self.present { 1.0 } else { 0.0 };
        Value::structure([
            (
                "colour".to_owned(),
                Value::List(
                    [self.colour[0], self.colour[1], self.colour[2], present]
                        .into_iter()
                        .map(Value::Float)
                        .collect(),
                ),
            ),
            (
                "direction".to_owned(),
                Value::List(
                    [
                        self.direction[0],
                        self.direction[1],
                        self.direction[2],
                        self.page,
                    ]
                    .into_iter()
                    .map(Value::Float)
                    .collect(),
                ),
            ),
            (
                "shadow_projection".to_owned(),
                Value::List(
                    self.shadow_projection
                        .iter()
                        .copied()
                        .map(Value::Float)
                        .collect(),
                ),
            ),
        ])
    }
}

/// The label the absent environment's image is given, which nothing samples and nothing names.
///
/// It is a name rather than nothing so that the payload and the declaration are written from
/// one value; it is not a label an author could collide with, because no author writes one.
const ABSENT_ENVIRONMENT_LABEL: &str = "environment_absent";

/// The instance entry the absent environment's map is uploaded under.
///
/// # Invariants
///
/// It is **not** [`image_entry`] of anything. Every other entry that carries pixels is named
/// after the image it holds, because the entry *is* that image; the absent map has no image,
/// and a name in the `image_` namespace standing for a thing no author declared is a name a
/// reader of the instance would go looking for. Entry names are generated from labels and a
/// scene cannot author one, so a fixed name cannot collide.
const ABSENT_ENVIRONMENT_ENTRY: &str = "environment_absent_map";

/// Reads the lights of the scene, and the environment among them.
///
/// # Invariants
///
/// The label is read first, because every error below names the light, and an error that
/// cannot say which light it is about is an error an author has to search a document for.
///
/// The kind is read before the fields, and a kind this stage cannot render is refused there
/// rather than further down: a directional light declares no `position`, so reading the
/// fields first would report a missing field instead of the mistake that is actually there,
/// and a light quietly drawn as a point is a wrong image nobody sees until the render is
/// finished.
///
/// An environment light is read against the scene's **images**, which is why the images come
/// first: the one field it declares is a label into that list, and a label resolved against a
/// list that had not been read yet would be a label resolved against nothing.
///
/// The two kinds that are **stated inputs of the lighting pass** rather than members of a
/// cluster list — the environment and the directional light — are counted here, because one
/// pass carries one field of each. A second of either is refused by name, which is the same
/// refusal in the same place, and a light silently dropped to make room is a light an author
/// wrote and nobody sees.
fn read_lights(
    scene: &Value,
    images: &[ImageRead],
) -> Result<(Vec<LightRead>, Option<Environment>), CompileError> {
    let path = Path::root().key(scene_schema::LIGHTS);
    let lights = list_at(scene, &Path::root(), scene_schema::LIGHTS)?;
    let mut read = Vec::with_capacity(lights.len());
    // The label of the environment light already read, and the environment itself. One
    // environment is what a lighting pass carries, so a second one is a document naming two
    // things the pass has one binding for.
    let mut environment: Option<Environment> = None;
    // The label of the directional light already read, for the same reason the environment is
    // kept: one lighting pass carries one directional field, so a second is a document naming
    // two things the pass has one answer for. The label and not the light, because the light
    // itself is in `read` either way and the refusal has to name the one that was first.
    let mut directional: Option<String> = None;
    for (index, light) in lights.iter().enumerate() {
        let path = path.index(index);
        let label = text(light, &path, "label")?.to_owned();
        if read.iter().any(|other: &LightRead| other.label == label) {
            return Err(CompileError::DuplicateLabel {
                kind: "light",
                label,
            });
        }
        let kind = read_light_kind(light, &path, &label)?;
        if !kind.is_emitted() {
            return Err(CompileError::UnsupportedLightKind {
                path: path.clone(),
                label,
                kind: kind.tag().to_owned(),
            });
        }
        let intensity = number(light, &path, "intensity")?;
        let colour = triple(light, &path, "color")?;
        // Scaled here rather than in the reader of each term, so that a colour means the
        // brightness a surface is lit by everywhere it is used: a record, an environment, and
        // a note about which lights were dropped all compare the same number.
        let colour = [
            colour[0] * intensity,
            colour[1] * intensity,
            colour[2] * intensity,
        ];
        read.push(match kind {
            LightKind::Environment => {
                if let Some(first) = &environment {
                    return Err(CompileError::SecondEnvironmentLight {
                        path: path.clone(),
                        label,
                        first: first.image.label.clone(),
                    });
                }
                let found = read_environment_image(light, &path, &label, images)?;
                environment = Some(Environment {
                    image: found,
                    colour,
                });
                LightRead {
                    kind,
                    label,
                    position: None,
                    direction: None,
                    colour,
                    shadow_extent: None,
                    radius: None,
                    // An environment light is in no cluster list and in no global run, so no
                    // fragment ever loads its record or its cone. The whole sphere is written
                    // rather than nothing because the run has one record per light and this
                    // one is a light.
                    cone: SpotCone::whole_sphere(),
                }
            }
            LightKind::Directional => {
                if let Some(first) = &directional {
                    return Err(CompileError::SecondDirectionalLight {
                        path: path.clone(),
                        label,
                        first: first.clone(),
                    });
                }
                let direction = read_direction(light, &path, &label)?;
                directional = Some(label.clone());
                LightRead {
                    kind,
                    label,
                    position: None,
                    direction: Some(direction),
                    colour,
                    shadow_extent: Some(number(light, &path, "shadow_extent")?),
                    // A directional light reaches every distance equally, so it has no reach to
                    // cull by. It declares no `radius` of its own, and it is **exempt** rather
                    // than global: the lighting pass carries it in its own uniform, so putting
                    // its record in the global run would shade it twice. The one fact that
                    // says so is [`LightKind::takes_cluster_slot`] being false, and the
                    // membership it leads to is [`Membership::Exempt`].
                    radius: None,
                    cone: SpotCone::whole_sphere(),
                }
            }
            // `is_emitted` has already refused every other kind, so reaching this arm with
            // anything but a point or a spot light would mean the two answers above had come
            // to disagree, which is a bug in the compiler rather than in the document. The
            // direction is the one field the two kinds do not share with a directional light
            // alone, and it is read by the kind rather than beside the fields they do share: a
            // point light declares no `direction`, so a read that asked for one would report a
            // missing field on a document that is in no way missing one.
            _ => {
                // A point light that names no `radius` and a spot light that names none are
                // read the same way, because the schema now says the same thing about both:
                // the field is optional and what an absent one means is decided by the kind
                // rather than here. A point light that says nothing is a **global** light —
                // see [`LightRead::reach`] — and a spot that says nothing is culled by the
                // reach of its own cone. One optional read rather than two, so there is one
                // place that knows a document naming no radius is not a document missing a
                // field.
                let radius = optional_number(light, &path, RADIUS)?;
                if let Some(reach) = radius.filter(|reach| *reach <= 0.0) {
                    return Err(CompileError::EmptyLightRadius {
                        path: path.key(RADIUS),
                        label,
                        radius: reach,
                    });
                }
                let direction = if kind == LightKind::Spot {
                    Some(read_direction(light, &path, &label)?)
                } else {
                    None
                };
                let cone = if let Some(axis) = direction {
                    read_spot_cone(axis, light, &path, &label)?
                } else {
                    SpotCone::whole_sphere()
                };
                LightRead {
                    kind,
                    label,
                    position: Some(triple(light, &path, "position")?),
                    direction,
                    colour,
                    shadow_extent: Some(number(light, &path, "shadow_extent")?),
                    radius,
                    cone,
                }
            }
        });
    }
    Ok((read, environment))
}

/// Reads the unit direction the light travels along, refusing a vector that is not one.
///
/// # Invariants
///
/// The zero is refused rather than normalised because normalising it divides by nothing, and a
/// `NaN` that reaches a matrix is a page of nothing rendered with no error anywhere. This is
/// the one number about a light that is answered before it is packed, because the schema says
/// the length does not matter and a document may write `[0, -20, 0]` and mean the same light as
/// `[0, -1, 0]`.
///
/// A **spot** light and a **directional** light both declare `direction` and both read it here,
/// so the refusal and the normalisation are one answer rather than two copies that could drift.
fn read_direction(light: &Value, path: &Path, label: &str) -> Result<[f64; 3], CompileError> {
    let direction = triple(light, path, "direction")?;
    if dot(direction, direction) == 0.0 {
        return Err(CompileError::ZeroLightDirection {
            path: path.key("direction"),
            label: label.to_owned(),
        });
    }
    Ok(normalize(direction))
}

/// Reads the cone of a spot light, refusing a cone that could not be drawn.
///
/// # Invariants
///
/// The four refusals are all the same fact seen from four sides: **the two angles describe one
/// shape, so a document whose numbers do not describe a shape is a document whose two answers
/// disagree.** A cone whose outside is inside its own inside is inverted; a full strength
/// region below the axis is not inside the cone at all; a half angle of a right angle is the
/// whole sphere, which a point light already is and which the page a shadow is rendered
/// through cannot be drawn as; and a direction of zero is not a direction, and normalising it
/// divides by nothing.
///
/// `axis` is the already normalised direction, read by [`read_direction`] because a spot light
/// is not the only kind that declares one and the zero check is not the only refusal that
/// follows from it. The cone's axis and the light's own direction are therefore one number,
/// and a document cannot write a cone pointing one way and a light shining another.
///
/// The angles are carried as they are written, and the cosine the shader compares against is
/// taken where the run is packed: this is a refusal about a document and that is a decision
/// about bytes, and a refusal reached across two modules is a refusal that can be forgotten.
fn read_spot_cone(
    axis: [f64; 3],
    light: &Value,
    path: &Path,
    label: &str,
) -> Result<SpotCone, CompileError> {
    let inner_angle = number(light, path, "inner_angle")?;
    let outer_angle = number(light, path, "outer_angle")?;
    if outer_angle <= inner_angle {
        return Err(CompileError::InvertedSpotCone {
            path: path.key("outer_angle"),
            label: label.to_owned(),
            inner_angle,
            outer_angle,
        });
    }
    if inner_angle < 0.0 {
        return Err(CompileError::NegativeSpotAngle {
            path: path.key("inner_angle"),
            label: label.to_owned(),
            angle: inner_angle,
        });
    }
    if outer_angle >= MAX_SPOT_HALF_ANGLE {
        return Err(CompileError::SpotConeTooWide {
            path: path.key("outer_angle"),
            label: label.to_owned(),
            outer_angle,
            limit: MAX_SPOT_HALF_ANGLE,
        });
    }
    Ok(SpotCone {
        axis,
        inner_angle,
        outer_angle,
    })
}

/// The widest cone a spot light may be written with, as a half angle in radians.
///
/// # Invariants
///
/// A **quarter turn**, which is a right angle, and the limit is a hard one for two reasons
/// that happen to agree. Past a right angle the shape is no longer a cone: at a right angle it
/// is the whole sphere and past it the "outside" folds back inside, so a document naming one is
/// asking for a light that already exists under another name. And the page a spot light's
/// shadow is rendered through is a perspective frustum, whose field of view is `1 / tan(half)`
/// and therefore has no value at a right angle at all — `perspective` silently clamps there,
/// and a clamped page is a page that does not cover the cone it was asked to cover, which is a
/// shadow that is missing where nobody was told it would be.
///
/// This is **not** the bound the handoff named, which was a half turn. A half turn is a wider
/// shape than the shadow volume can be drawn as, so honouring it would mean compiling a
/// document whose shading is right and whose shadow is silently absent.
const MAX_SPOT_HALF_ANGLE: f64 = std::f64::consts::FRAC_PI_2;

/// Reads the image an environment light is read from, and refuses a label that names nothing.
///
/// # Invariants
///
/// The image must be one the scene **declares** and one that carries pixels. An environment
/// light whose image had no pixels would be a light the pass binds an empty texture for, and
/// an empty texture is a black frame rather than an error; the same refusal a material meets
/// for the same reason is the one it meets here.
///
/// The image is **returned** rather than its label, so the caller resolves the name once.
fn read_environment_image(
    light: &Value,
    path: &Path,
    label: &str,
    images: &[ImageRead],
) -> Result<ImageRead, CompileError> {
    let image = text(light, path, "image")?;
    let found = images
        .iter()
        .find(|found| found.label == image)
        .ok_or_else(|| CompileError::UnknownImage {
            path: path.key("image"),
            material: label.to_owned(),
            image: image.to_owned(),
        })?;
    if found.pixels.is_empty() {
        return Err(CompileError::ImageWithoutData {
            path: path.key("image"),
        });
    }
    Ok(found.clone())
}

/// Reads the kind of one light, which the schema writes as the light's variant tag.
///
/// # Invariants
///
/// A light is an enumeration whose tag is the kind, so a light that is a struct rather than
/// a variant names no kind at all. That is a validation error rather than a default: see
/// [`scene_schema::light_schema`] for why an offline renderer refuses the guess.
///
/// A tag this stage does not know is refused the same way a kind it knows is, because the
/// emitter has no way to draw either and the schema has already reported the tag as one it
/// does not declare.
fn read_light_kind(light: &Value, path: &Path, label: &str) -> Result<LightKind, CompileError> {
    match light.variant_tag() {
        Some("Point") => Ok(LightKind::Point),
        Some("Spot") => Ok(LightKind::Spot),
        Some("Directional") => Ok(LightKind::Directional),
        Some("Environment") => Ok(LightKind::Environment),
        Some(kind) => Err(CompileError::UnsupportedLightKind {
            path: path.clone(),
            label: label.to_owned(),
            kind: kind.to_owned(),
        }),
        None => Err(CompileError::MissingLightKind {
            path: path.clone(),
            label: label.to_owned(),
        }),
    }
}

/// Reads the meshes of the scene and the instance entries their vertex buffers contribute.
///
/// # Invariants
///
/// A point is verified here against its mesh's own point schema **and against every point
/// schema a shader in the scene declares**, which is the same set of checks
/// `read_instances` used to do one instance at a time. Doing them here rather than there is
/// the difference between checking a mesh's points once and checking them once per instance
/// that draws it: a field of a thousand identical objects is a thousand times the same
/// verification, and the answer is the same every time because it depends only on the points
/// and the schema.
///
/// The set of distinct schemas is small — one per shader a scene draws with — so a scene
/// verifies each mesh's points once per *shader* rather than once per *instance*.
/// One image, read out of the scene document: pixels a material samples, and nothing about
/// which material samples them.
///
/// # Invariants
///
/// An image is read without being checked for a label collision, because a label no material
/// names cannot collide with anything. `material_images` is where a name is resolved, and it
/// is where a duplicate is reported.
///
/// It is `Clone` because an environment light **takes** one rather than borrowing it: the map
/// a lighting pass binds and the map its uniform describes have to be the same value, and a
/// borrow would leave the pass holding a reference into the scene's own list.
#[derive(Clone)]
struct ImageRead {
    label: String,
    width: i64,
    height: i64,
    format: String,
    pixels: Vec<u8>,
}

/// Reads the images a scene declares.
fn read_images(scene: &Value) -> Result<Vec<ImageRead>, CompileError> {
    let path = Path::root().key(scene_schema::IMAGES);
    let Some(images) = field(scene, scene_schema::IMAGES) else {
        return Ok(Vec::new());
    };
    let Some(images) = images.as_list() else {
        return Err(malformed(&path, "a list"));
    };

    let mut read: Vec<ImageRead> = Vec::with_capacity(images.len());
    for (index, image) in images.iter().enumerate() {
        let path = path.index(index);
        let label = text(image, &path, scene_schema::LABEL)?.to_owned();
        let count = |name: &str| -> Result<i64, CompileError> {
            let field_path = path.key(name);
            field(image, name)
                .and_then(Value::as_int)
                .ok_or_else(|| malformed(&field_path, "a count of texels"))
        };
        let width = count(scene_schema::WIDTH)?;
        let height = count(scene_schema::HEIGHT)?;
        let format = text(image, &path, scene_schema::FORMAT)?.to_owned();
        // A scene image carries its own pixels, or names a definition that does. Either way it
        // is a texture the compiler can fill, and an image with nothing in it is caught where
        // it is named rather than here: a scene may declare one it never uses.
        let pixels = field(image, scene_schema::DATA)
            .and_then(Value::as_bytes)
            .ok_or_else(|| CompileError::ImageWithoutData {
                path: path.key(scene_schema::DATA),
            })?
            .to_vec();
        read.push(ImageRead {
            label,
            width,
            height,
            format,
            pixels,
        });
    }
    Ok(read)
}

/// One image a material's custom data names, and the field it was named in.
///
/// # Invariants
///
/// The field is what the compiler derives a binding point from, so two materials that name
/// different images in the same field cannot share a pass: the point would have to be two
/// sources at once. `groups` is where that is prevented, and this is what it compares.
#[derive(Clone, PartialEq, Eq)]
struct ImageBinding {
    field: String,
    label: String,
}

/// Returns the fields a shader's custom schema declares as images, in name order.
fn image_fields(custom: &Schema) -> Vec<String> {
    let SchemaKind::Struct { fields } = &custom.kind else {
        return Vec::new();
    };
    fields
        .iter()
        .filter(|(_, field)| graphics::is_image(field))
        .map(|(name, _)| name.clone())
        .collect()
}

/// Returns the custom data without the fields that name an image.
///
/// # Invariants
///
/// What is left is what gets packed into the instance row and what carries no name, so the two
/// can be told apart: an image is a name the compiler resolves, and everything else is a number
/// the row carries.
fn numbers_only(custom: &Value, images: &[String]) -> Value {
    let Some(fields) = custom.as_field_map() else {
        return custom.clone();
    };
    Value::structure(
        fields
            .iter()
            .filter(|(name, _)| !images.iter().any(|image| image == *name))
            .map(|(name, value)| (name.clone(), value.clone())),
    )
}

/// Resolves the images one material's custom data names.
fn material_images(
    custom_data: &Value,
    shader: &ShaderRead,
    material_label: &str,
    material_path: &Path,
    images: &[ImageRead],
) -> Result<Vec<ImageBinding>, CompileError> {
    let fields = image_fields(&shader.custom_schema);
    if fields.is_empty() {
        return Ok(Vec::new());
    }
    // A grade a shader reads nothing into has no custom data at all, which is the common case for
    // a screen material and is not an error: a shader that declares an image field and a material
    // that carries nothing names no image.
    let empty = definition::FieldMap::new();
    let custom = custom_data.as_field_map().unwrap_or(&empty);
    let mut bindings = Vec::new();
    for field in fields {
        // An image field is optional, so a material that names none is a material that draws
        // no image. That is every material until one asks for one.
        let Some(label) = custom.get(&field).and_then(Value::as_str) else {
            continue;
        };
        let Some(image) = images.iter().find(|image| image.label == label) else {
            return Err(CompileError::UnknownImage {
                path: material_path.key(scene_schema::CUSTOM_DATA).key(&field),
                material: material_label.to_owned(),
                image: label.to_owned(),
            });
        };
        if image.pixels.is_empty() {
            return Err(CompileError::ImageWithoutData {
                path: material_path.key(scene_schema::CUSTOM_DATA).key(&field),
            });
        }
        bindings.push(ImageBinding {
            field,
            label: label.to_owned(),
        });
    }
    Ok(bindings)
}

/// The name a texture entry for one image is written under.
///
/// An image is emitted once however many materials name it, so the entry is named after the
/// image rather than after the material. That is what makes two materials naming one image
/// share the one texture rather than uploading it twice.
fn image_entry(label: &str) -> String {
    format!("image_{label}")
}

/// The binding point an image field is bound at.
///
/// It is the field's own name, so a shader declares the point its custom schema already names
/// and no author writes a binding number anywhere.
fn image_point(field: &str) -> String {
    field.to_owned()
}

/// Builds the instance entry one image's pixels are uploaded from.
fn image_payload(image: &ImageRead) -> Value {
    texture_with_data(
        &image.format,
        image.width,
        image.height,
        &image.pixels,
        true,
    )
}

/// Builds the instance entry the environment map's pixels are uploaded from.
///
/// # Invariants
///
/// It is the same size, format and pixels as the image a material would upload, and it
/// declares **no `sampled` key** — which is a difference in the binding rather than in the
/// texture. A texture that says it is sampled takes a sampler at the binding immediately after
/// its own, and the environment map is read with `textureLoad` at a texel the shader computes,
/// so it has no sampler to take; asking for one pushes every binding number below it along by
/// one, and a shader written for the number without one is then reading somebody else's
/// resource.
///
/// A scene whose environment map is *also* a material's texture writes whichever payload came
/// first, because the entry is named after the image and one image is one entry. The
/// environment is written before any pass binds a material image, so the map is read with
/// `textureLoad`; a material that then samples the same texture finds no sampler, and the
/// interface check reports that by naming both sides rather than letting the frame draw.
fn environment_payload(image: &ImageRead) -> Value {
    texture_with_data(
        &image.format,
        image.width,
        image.height,
        &image.pixels,
        false,
    )
}

/// Builds the pass declaration one image is bound with.
fn image_binding(image: &ImageRead) -> Value {
    texture_binding(
        &image_entry(&image.label),
        image.width,
        image.height,
        &image.format,
    )
}

/// Builds a texture payload that carries its own pixels.
///
/// # Invariants
///
/// `sampled` is a unit key whose **presence** is the whole declaration, so it is written for
/// a texture a shader filters through a sampler and left out for one a shader reads at a texel
/// of its own choosing. It is a parameter rather than always written because format cannot
/// carry the distinction: the same `rgba8unorm` is a filtered surface texture in one pass and
/// the frame itself in the next, and the difference is one binding number. A texture a pass
/// renders into is the pipeline's own and is declared where it is emitted, not here.
fn texture_with_data(format: &str, width: i64, height: i64, pixels: &[u8], sampled: bool) -> Value {
    let mut fields = BTreeMap::new();
    fields.insert(graphics::binders::WIDTH.to_owned(), Value::Int(width));
    fields.insert(graphics::binders::HEIGHT.to_owned(), Value::Int(height));
    fields.insert(
        graphics::binders::FORMAT.to_owned(),
        Value::String(format.to_owned()),
    );
    fields.insert(
        graphics::binders::DATA.to_owned(),
        Value::Bytes(pixels.to_vec()),
    );
    if sampled {
        fields.insert(graphics::binders::SAMPLED.to_owned(), Value::Unit);
    }
    Value::Enum {
        tag: "Texture".to_owned(),
        fields,
    }
}

/// Returns a shader's pass schema with a texture bound at each field that names an image.
///
/// # Why the compiler writes this and not the author
///
/// A pass schema is a closed struct: a binder the schema does not declare is a binder the pass
/// may not carry, so a pass that binds an image has to declare it. That declaration is the
/// compiler's to write, because the field is where the author said what they wanted and the
/// binding point is what the compiler decided to do about it. A shader that had to name the point
/// itself would be naming a gpu resource, which is the one thing a scene does not do.
///
/// # Invariants
///
/// The point's name is the field's own name, and its schema is a texture declaration, because
/// that is what the pass emits there. A schema that already declares the point is left alone
/// rather than merged: a field is a field, and declaring it twice would be a schema that
/// disagrees with itself.
fn declared_with_images(pass: &Schema, custom: &Schema) -> Rc<Schema> {
    let images = image_fields(custom);
    if images.is_empty() {
        return Rc::new(pass.clone());
    }
    // A pass schema is the interface of a pass: a `uniform` and a `binders` block, and the
    // binder points are fields of the second. Adding one to the outer struct would declare a
    // field of the interface that no pass carries.
    let SchemaKind::Struct { fields } = &pass.kind else {
        return Rc::new(pass.clone());
    };
    let Some(binders) = fields.get(graphics::BINDERS) else {
        return Rc::new(pass.clone());
    };
    let SchemaKind::Struct {
        fields: binder_fields,
    } = &binders.kind
    else {
        return Rc::new(pass.clone());
    };
    let mut binder_fields = binder_fields.clone();
    for field in images {
        // Optional, because a material need not name an image: the field it would name one in is
        // optional in the custom schema, and a pass whose material named none binds nothing there.
        // A required binder would make every material of that shader carry an image.
        binder_fields.insert(
            field.clone(),
            graphics::texture_binding_schema()
                .optional()
                .with_documentation(
                    "The image this pass binds, at the point named after the custom field the \
                     material named it in. Absent when the material names no image.",
                ),
        );
    }
    let mut interface = fields.clone();
    interface.insert(
        graphics::BINDERS.to_owned(),
        Schema::new(SchemaKind::structure(binder_fields)),
    );
    Rc::new(Schema::new(SchemaKind::structure(interface)))
}

/// Reads the meshes a scene declares.
fn read_meshes(
    scene: &Value,
    shaders: &[ShaderRead],
) -> Result<(Vec<MeshRead>, BTreeMap<String, Value>), CompileError> {
    let path = Path::root().key(scene_schema::MESHES);
    let meshes = list_at(scene, &Path::root(), scene_schema::MESHES)?;
    let schemas = shader_point_schemas(shaders);

    let mut read: Vec<MeshRead> = Vec::with_capacity(meshes.len());
    let mut instance = BTreeMap::new();
    for (index, mesh) in meshes.iter().enumerate() {
        let mesh_path = path.index(index);
        let (found, entry) = read_mesh(mesh, &mesh_path, &schemas)?;
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

/// Returns the distinct point schemas the scene's mesh stages declare.
///
/// A screen shader is left out: it draws over the frame and reads no points, so its
/// interface is the unit schema, and verifying points against that would refuse every point
/// in the scene.
fn shader_point_schemas(shaders: &[ShaderRead]) -> Vec<Rc<Schema>> {
    let mut distinct: Vec<Rc<Schema>> = Vec::new();
    for shader in shaders {
        if shader.stage == Stage::Post {
            continue;
        }
        if !distinct.iter().any(|seen| **seen == *shader.point_schema) {
            distinct.push(Rc::clone(&shader.point_schema));
        }
    }
    distinct
}

/// Reads one mesh: the geometry an instance draws.
///
/// # Invariants
///
/// The points are verified against the mesh's own point schema **and against every distinct point
/// schema a shader in the scene declares**, and then packed once. Verification is the only pass
/// that looks at a point as a value, and it happens here, once, for the whole scene. Everything
/// downstream — the vertex buffer, the bounds, a diff against the previous frame — reads the run.
///
/// A run satisfies its schema by construction, so a *packed* point list does not re-verify at all:
/// `pack_points` is what checks that each element is a struct of the declared fields, and the
/// buffer is the bytes that check produced.
fn read_mesh(
    mesh: &Value,
    path: &Path,
    shader_schemas: &[Rc<Schema>],
) -> Result<(MeshRead, (String, Value)), CompileError> {
    let label = text(mesh, path, scene_schema::LABEL)?.to_owned();
    let point_schema = schema_at(mesh, path, scene_schema::POINT_SCHEMA)?;
    let points_path = path.key(scene_schema::POINTS);
    let points =
        field(mesh, scene_schema::POINTS).ok_or_else(|| malformed(&points_path, "a list"))?;

    // A packed list is already what a buffer wants, and it was checked when it was packed.
    if let Some(run) = points.as_packed() {
        let layout = packed_layout(point_schema.as_ref()).map_err(unpackable)?;
        if *run.layout() != layout {
            return Err(CompileError::Unpackable {
                path: points_path,
                reason: format!(
                    "the points are packed as {} but the mesh's point schema lays them out as {}",
                    run.layout().field_names().join(", "),
                    layout.field_names().join(", ")
                ),
            });
        }
        if run.is_empty() {
            return Err(CompileError::EmptyMesh { label });
        }
        return Ok(finish_mesh(
            label,
            point_schema,
            run.clone(),
            mesh,
            path,
            &points_path,
        ));
    }

    let points = points
        .as_list()
        .ok_or_else(|| malformed(&points_path, "a list"))?;
    for (index, point) in points.iter().enumerate() {
        verify(
            point,
            &points_path.index(index),
            point_schema.as_ref(),
            "a point",
            "the mesh point schema",
        )?;
        // A mesh is never described in a shape a shader that draws it rejects, and the
        // shape it is described in is the same whatever instance draws it.
        for schema in shader_schemas {
            if **schema == *point_schema {
                continue;
            }
            verify(
                point,
                &points_path.index(index),
                schema.as_ref(),
                "a point",
                "a shader point schema",
            )?;
        }
    }

    if points.is_empty() {
        return Err(CompileError::EmptyMesh { label });
    }
    // One pack, and the bytes it produces are both the run and the vertex buffer.
    let packed = pack_points(point_schema.as_ref(), points, &points_path)?;
    let run = PackedList::new(
        packed_layout(point_schema.as_ref()).map_err(unpackable)?,
        packed,
    )
    .map_err(|error| CompileError::Unpackable {
        path: points_path.clone(),
        reason: error.to_string(),
    })?;
    Ok(finish_mesh(
        label,
        point_schema,
        run,
        mesh,
        path,
        &points_path,
    ))
}

/// Finishes a mesh whose points are packed, whatever form the document wrote them in.
fn finish_mesh(
    label: String,
    point_schema: Rc<Schema>,
    points: PackedList,
    mesh: &Value,
    path: &Path,
    points_path: &Path,
) -> (MeshRead, (String, Value)) {
    let vertex_bytes = points.bytes().len() as i64;
    let topology = read_topology(mesh, path).expect("the topology was read before the points");
    let vertex_entry = format!("{label}_{VERTICES}");
    let found = MeshRead {
        vertex_bytes,
        vertex_entry: vertex_entry.clone(),
        label,
        topology,
        point_schema: point_schema.clone(),
        points,
    };

    // The buffer carries the schema it was packed with, so a renderer derives the vertex
    // layout from the point schema rather than it being declared a second time. The bytes are the
    // run's own, so this costs a copy and not a pack.
    let entry = (
        vertex_entry,
        buffer_payload(
            found.points.bytes().to_vec(),
            found.point_schema.to_value(),
            &["vertex"],
            topology.tag(),
        ),
    );
    let _ = points_path;
    (found, entry)
}

/// Reads the instances of the scene, which are listed in buckets, one per pass.
///
/// # Invariants
///
/// The pass an instance is drawn in is the bucket it is listed under. That is what makes the
/// stage a checkable fact: the material an instance names must have a shader of that bucket's
/// stage. Where the stage was whatever the material's shader happened to be, a material used
/// in the wrong pass could not be written down at all — and a plane drawn in the shadow pass
/// with a lighting shader could.
fn read_instances(
    scene: &Value,
    meshes: &[MeshRead],
    materials: &[MeshMaterialRead],
    shaders: &[ShaderRead],
    images: &[ImageRead],
) -> Result<Vec<InstanceRead>, CompileError> {
    let path = Path::root().key(scene_schema::INSTANCES);
    let instances = at(scene, &Path::root(), scene_schema::INSTANCES)?;
    let Some(buckets) = instances.as_dict() else {
        return Err(malformed(&path, "the instances as a map of pass to list"));
    };

    let mut read: Vec<InstanceRead> = Vec::new();
    for (name, bucket) in buckets {
        let bucket_path = path.key(name);
        let Some(stage) = Stage::from_suffix(name) else {
            return Err(CompileError::UnknownStage {
                path: bucket_path,
                stage: name.clone(),
            });
        };
        let Some(bucket) = bucket.as_list() else {
            return Err(malformed(&bucket_path, "a list of instances"));
        };
        for (index, instance) in bucket.iter().enumerate() {
            let found = read_instance(
                instance,
                &bucket_path.index(index),
                meshes,
                materials,
                shaders,
                images,
                stage,
            )?;
            if read.iter().any(|other| other.label == found.label) {
                return Err(CompileError::DuplicateLabel {
                    kind: "instance",
                    label: found.label,
                });
            }
            read.push(found);
        }
    }
    Ok(read)
}

/// Reads one instance and every schema it is verified against.
///
/// # Invariants
///
/// A point is verified against its mesh's own point schema and against the point schema of
/// the shader that draws it. Verifying one and not the other is how a mesh ends up carrying a
/// field no shader asked for, or a shader that reads a field no point carries.
fn read_instance(
    instance: &Value,
    path: &Path,
    meshes: &[MeshRead],
    materials: &[MeshMaterialRead],
    shaders: &[ShaderRead],
    images: &[ImageRead],
    stage: Stage,
) -> Result<InstanceRead, CompileError> {
    let label = text(instance, path, scene_schema::LABEL)?.to_owned();
    let mesh_label = text(instance, path, scene_schema::MESH)?;
    let Some((mesh, _found)) = meshes
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
    let material_label = text(instance, path, scene_schema::MATERIAL)?;
    let Some(material) = materials
        .iter()
        .find(|material| material.label == material_label)
    else {
        return Err(CompileError::UnknownMaterial {
            path: material_path,
            material: material_label.to_owned(),
        });
    };
    let shader_read = &shaders[material.shader];
    // The bucket is the pass and the material says how the instance is drawn in it, so the two
    // have to agree. A material whose shader belongs to another stage would be drawn by this
    // pass's pipeline with the wrong interface, which is a mismatch nothing else would report.
    if shader_read.stage != stage {
        return Err(CompileError::Emission {
            reason: format!(
                "at {material_path}: `{material_label}` names the {} shader `{}`, and this \
                 instance is listed under `{}`",
                shader_read.stage.tag(),
                shader_read.label,
                stage.suffix()
            ),
        });
    }

    // The points of the mesh this instance draws were verified against this shader's point
    // schema when the mesh was read, once for the whole scene rather than once per
    // instance. The check's answer depends only on the points and the schema, so doing it
    // again here would be the same answer the same number of times as there are instances.
    let custom_data = &material.custom_data;
    let custom_path = material.path.key(scene_schema::CUSTOM_DATA);
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
    //
    // A field that names an image is left out of both. It is a name the compiler resolves into a
    // texture rather than a number the row can carry, so the row is built from the custom data
    // without it and the image is bound beside the row instead of packed into it.
    let images = material_images(
        custom_data,
        shader_read,
        &material.label,
        &material.path,
        images,
    )?;
    let custom_data = &numbers_only(custom_data, &image_fields(&shader_read.custom_schema));
    let row_schema = merged_row_schema(
        shader_read.instance_schema.as_ref(),
        shader_read.custom_schema.as_ref(),
    )?;
    let model = transform.matrix();
    let mut fields = vec![(
        MODEL.to_owned(),
        Value::List(model.iter().copied().map(Value::Float).collect()),
    )];
    // A shader that declares a normal matrix is asking for the one that carries a normal
    // through this transform, and the emitter is the only place that knows the transform.
    // Filling it here rather than in a shader is what makes an unevenly scaled instance
    // lit correctly: a normal transformed by the model itself is wrong by exactly the
    // amount the scale is uneven by.
    if declares(&row_schema, NORMAL_MATRIX) {
        fields.push((
            NORMAL_MATRIX.to_owned(),
            Value::List(
                normal_matrix(&model)
                    .iter()
                    .copied()
                    .map(Value::Float)
                    .collect(),
            ),
        ));
    }
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
        path: path.clone(),
        mesh,
        shader: material.shader,
        stage,
        model,
        images,
    })
}

/// Returns the schema one instance row is read with: what the shader reads per instance, and the
/// custom fields the material carries.
///
/// # Invariants
///
/// A field named by both is an error rather than one of them winning, because the two schemas
/// describe one row and a name in both would be two values written to one place.
///
/// A field that names an image is left out. It is the one kind of custom value that is not a
/// number, so it cannot be packed into a row and is bound as a texture instead; leaving it in
/// would ask `pack_struct` for bytes no field of the row has.
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
        if graphics::is_image(field) {
            continue;
        }
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
    /// The images every member names, which is also what makes them one group.
    ///
    /// # Why this is part of the key
    ///
    /// A pass binds one texture at one binding point, so two instances that name different
    /// images in the same field cannot be drawn together: the point would have to be two
    /// sources at once. Instances whose images agree share a pass as they always did, and
    /// instances whose images differ are drawn by separate passes rather than by one pass
    /// that reads the wrong texture for half of them.
    images: Vec<ImageBinding>,
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
            &["instance"],
            self.mesh.topology.tag(),
        )
    }
}

/// Writes the instance entry for each image a group draws with, once per image.
///
/// # Invariants
///
/// The entry is named after the image, so a second group naming the same image finds the entry
/// already there and does not upload it again. That is what makes two materials naming one
/// image bind one texture rather than two copies of it — and the write is *skipped* rather
/// than refused, because an image reached twice is the same image: one entry, one upload, and
/// no reason to call it a collision. See [`insert_entry_once`].
fn declare_images(
    instance: &mut BTreeMap<String, Value>,
    group: &Group<'_>,
    images: &[ImageRead],
) -> Result<Vec<(String, Value)>, CompileError> {
    declare_read_images(instance, &group.images, images)
}

/// The same, for any list of names: a group's, or one post process's.
///
/// A post process has no group to belong to — a grade runs once over the frame rather than once
/// per object — so the two callers share this rather than one of them owning it.
fn declare_read_images(
    instance: &mut BTreeMap<String, Value>,
    bindings: &[ImageBinding],
    images: &[ImageRead],
) -> Result<Vec<(String, Value)>, CompileError> {
    let mut declared = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let Some(image) = images.iter().find(|image| image.label == binding.label) else {
            // `material_images` resolved every label against this list, so a label that is not
            // in it cannot reach here.
            return Err(CompileError::Emission {
                reason: format!(
                    "the image `{}` was resolved and then not found",
                    binding.label
                ),
            });
        };
        insert_entry_once(instance, image_entry(&image.label), image_payload(image));
        declared.push((image_point(&binding.field), image_binding(image)));
    }
    Ok(declared)
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
        match groups.iter_mut().find(|group| {
            group.shader == shader && group.mesh.label == mesh.label && group.images == found.images
        }) {
            Some(group) => group.members.push(found),
            None => groups.push(Group {
                members: vec![found],
                mesh,
                shader,
                stage,
                light,
                images: found.images.clone(),
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
    let stage = match shader.variant_tag() {
        Some("ScreenShader") => Stage::Post,
        Some("ShadowingShader") => Stage::Shadowing,
        Some("DepthShader") => Stage::Depthing,
        Some("LightShader") => Stage::Lighting,
        Some("EmissionShader") => Stage::Emission,
        Some(other) => {
            return Err(CompileError::UnknownStage {
                path: path.clone(),
                stage: other.to_owned(),
            });
        }
        None => return Err(malformed(path, "a shader")),
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

    // A screen shader draws over the frame rather than over a mesh, so it reads neither
    // points nor an instance row. Both are declared as the unit schema, which is a schema
    // nothing satisfies, and neither is ever packed: the post pass draws a mesh of its own
    // with no instance data on it. Declaring a real interface here would be a claim the
    // pipeline does not keep.
    let nothing = || Rc::new(Schema::new(SchemaKind::Unit));
    let (point_schema, instance_schema) = if stage == Stage::Post {
        (nothing(), nothing())
    } else {
        (
            read(scene_schema::POINT_SCHEMA)?,
            read(scene_schema::INSTANCE_SCHEMA)?,
        )
    };

    let custom_schema = read(scene_schema::CUSTOM_SCHEMA)?;
    let pass_schema = read(scene_schema::PASS_SCHEMA)?;
    Ok(ShaderRead {
        stage,
        label: text(shader, path, scene_schema::LABEL)?.to_owned(),
        source: text(shader, path, scene_schema::SOURCE)?.to_owned(),
        // A field that names an image is a binder the pass carries, so the interface the shader
        // is checked against has to say so. See [`declared_with_images`].
        pass_schema: declared_with_images(&pass_schema, &custom_schema),
        instance_schema,
        point_schema,
        custom_schema,
    })
}

/// Reads the post processes a scene runs, in order, resolving each to its screen material.
///
/// # Invariants
///
/// A post process names a screen material by label, and the shader that material names must be
/// a screen shader: a mesh shader here would be a pass over the frame that reads points no
/// pass supplies.
fn read_post_processes(
    scene: &Value,
    shaders: &[ShaderRead],
    images: &[ImageRead],
) -> Result<Vec<ScreenRead>, CompileError> {
    let path = Path::root().key(scene_schema::POST_PROCESSES);
    // A scene with no post stage is a scene that grades nothing, which is a legitimate scene
    // rather than an error: an absent field and an empty list mean the same thing here.
    let Ok(processes) = list_at(scene, &Path::root(), scene_schema::POST_PROCESSES) else {
        return Ok(Vec::new());
    };
    if processes.is_empty() {
        return Ok(Vec::new());
    }
    let materials_path = Path::root().key(scene_schema::SCREEN_MATERIALS);
    let materials = list_at(scene, &Path::root(), scene_schema::SCREEN_MATERIALS)?;

    let mut resolved = Vec::with_capacity(processes.len());
    for (index, process) in processes.iter().enumerate() {
        let process_path = path.index(index);
        let Some(label) = process.as_str() else {
            return Err(malformed(&process_path, "a screen material label"));
        };
        let Some((material_index, material)) =
            materials.iter().enumerate().find(|(_, material)| {
                material
                    .as_field_map()
                    .and_then(|fields| fields.get(scene_schema::LABEL))
                    .and_then(Value::as_str)
                    == Some(label)
            })
        else {
            return Err(CompileError::Emission {
                reason: format!(
                    "at {process_path}: no screen material in this scene is labelled `{label}`"
                ),
            });
        };
        let material_path = materials_path.index(material_index);
        let shader_path = material_path.key(scene_schema::SHADER);
        let shader_label = text(material, &material_path, scene_schema::SHADER)?;
        let Some((shader, found)) = shaders
            .iter()
            .enumerate()
            .find(|(_, shader)| shader.label == shader_label)
        else {
            return Err(CompileError::UnknownShader {
                path: shader_path,
                shader: shader_label.to_owned(),
            });
        };
        if found.stage != Stage::Post {
            return Err(CompileError::Emission {
                reason: format!(
                    "at {material_path}: `{shader_label}` is a {} shader, and a screen material \
                     names a screen shader",
                    found.stage.tag()
                ),
            });
        }
        // A screen material's custom data is read the same way a mesh material's is: the fields
        // the shader declares as images are names the compiler resolves into textures. A grade
        // that carries nothing reads nothing, which is what every grade did until now.
        let custom_data = field(material, scene_schema::CUSTOM_DATA)
            .cloned()
            .unwrap_or(Value::Unit);
        let images = material_images(&custom_data, found, label, &material_path, images)?;
        resolved.push(ScreenRead { shader, images });
    }
    Ok(resolved)
}

/// One screen material as a post process uses it: the shader it runs, and the images it names.
///
/// # Invariants
///
/// A post process is not grouped the way an instance is, because a grade runs once over the frame
/// rather than once per object. So each process carries its own images rather than sharing a
/// group's, and two processes naming one image still upload it once, because the entry is named
/// after the image rather than after the material.
struct ScreenRead {
    shader: usize,
    images: Vec<ImageBinding>,
}

/// Returns whether a shader's declared interface carries a named field in its uniform.
///
/// # Invariants
///
/// The question is asked of the **interface** rather than of the uniform value, because the
/// interface is the layout: a field the interface does not declare is a field the upload drops
/// however carefully the emitter wrote it. See the call in [`emit`].
fn declares_uniform_field(interface: &Schema, field: &str) -> bool {
    let SchemaKind::Struct { fields } = &interface.kind else {
        return false;
    };
    let Some(SchemaKind::Struct { fields }) = fields.get(UNIFORM).map(|block| &block.kind) else {
        return false;
    };
    fields.contains_key(field)
}

/// Reads the exposure a scene names, which is `None` for a scene that names none.
///
/// # Why the absent case is `None` and not one
///
/// The post uniform is the **layout the pass is uploaded with**, so an `exposure` written into
/// every post uniform would be a field every scene's post shader would have to declare. One
/// that is written only when the document names it is what makes the field additive: a scene
/// that names no exposure emits the bytes it emitted before the field existed, and one that
/// names it opts in deliberately, in three places at once — see
/// [`scene_schema::EXPOSURE`].
fn read_exposure(scene: &Value) -> Result<Option<f64>, CompileError> {
    let path = Path::root().key(scene_schema::EXPOSURE);
    let Some(value) = field(scene, scene_schema::EXPOSURE) else {
        return Ok(None);
    };
    let exposure = value
        .as_float()
        .or_else(|| value.as_int().map(|number| number as f64))
        .ok_or_else(|| CompileError::MalformedScene {
            path: path.clone(),
            expected: "a linear multiplier, where 1 is no change",
            found: value.kind_name(),
        })?;
    // A negative exposure has no meaning as a multiplier of light and a zero one is a black
    // frame, so both are refused rather than clamped: a clamp would hand back a picture and
    // say nothing, and a document that names one of these numbers is a document that is wrong
    // on every machine. The page pool's line is *a count below one is refused*, and an
    // exposure of zero is a count below one. The finiteness test is written first because it
    // is also what catches a NaN, which every comparison below would answer `false` to.
    if !exposure.is_finite() || exposure <= 0.0 {
        return Err(CompileError::MalformedScene {
            path,
            expected: "a positive linear multiplier, where 1 is no change",
            found: "a number that is not",
        });
    }
    Ok(Some(exposure))
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

/// Reads the mesh materials of the scene, which every instance names by label.
///
/// A material is declared once and named by every instance drawn with it, so two objects
/// drawn the same way carry one copy of the material rather than one apiece.
fn read_materials(
    scene: &Value,
    shaders: &[ShaderRead],
) -> Result<Vec<MeshMaterialRead>, CompileError> {
    let path = Path::root().key(scene_schema::MESH_MATERIALS);
    let materials = list_at(scene, &Path::root(), scene_schema::MESH_MATERIALS)?;

    let mut read: Vec<MeshMaterialRead> = Vec::with_capacity(materials.len());
    for (index, material) in materials.iter().enumerate() {
        let material_path = path.index(index);
        let label = text(material, &material_path, scene_schema::LABEL)?.to_owned();
        if read.iter().any(|other| other.label == label) {
            return Err(CompileError::DuplicateLabel {
                kind: "material",
                label,
            });
        }
        let shader_path = material_path.key(scene_schema::SHADER);
        let shader_label = text(material, &material_path, scene_schema::SHADER)?;
        let Some((shader, _found)) = shaders
            .iter()
            .enumerate()
            .find(|(_, shader)| shader.label == shader_label)
        else {
            return Err(CompileError::UnknownShader {
                path: shader_path,
                shader: shader_label.to_owned(),
            });
        };
        read.push(MeshMaterialRead {
            shader,
            custom_data: at(material, &material_path, scene_schema::CUSTOM_DATA)?.clone(),
            label,
            path: material_path,
        });
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

/// Adds one instance entry **unless the name already holds one**.
///
/// # Invariants
///
/// An entry named after an *image* is reached once per reader, and two readers of one image
/// are not a collision: the entry is the image's, it carries the image's bytes, and the second
/// reader wanted exactly the entry the first one wrote. This is what lets two materials name
/// one image and lets an environment light and a material name the same one, which is the
/// ordinary case of a sky that is also a backdrop.
///
/// The entry it declines to write is the one already there, so a caller cannot get a different
/// picture by having written it first. Where two readers genuinely want the same name to hold
/// two different things, [`insert_entry`] still refuses, and that is where a real collision is
/// reported.
fn insert_entry_once(instance: &mut BTreeMap<String, Value>, entry: String, value: Value) {
    if !instance.contains_key(&entry) {
        instance.insert(entry, value);
    }
}

/// Reads the `instance_schema` the emitted declarations imply.
fn derive_instance_schema(graphics: &Value) -> Result<Value, CompileError> {
    derived_instance_schema(graphics)
        .map(|schema| schema.to_value())
        .map_err(emission)
}

/// Wraps a camera basis as the value a post stage reads its rays from.
///
/// # Invariants
///
/// The basis is the one the projection's matrix was built from, so a ray reconstructed from
/// these axes is the ray that matrix would have cast. A post stage that drew a sky from a
/// basis belonging to a different camera would place it correctly for one frame and
/// incorrectly for every other.
fn camera_value(projection: &Projection) -> Value {
    let triple =
        |components: [f64; 3]| Value::List(components.into_iter().map(Value::Float).collect());
    Value::structure([
        (FORWARD.to_owned(), triple(projection.forward)),
        (
            HALF_EXTENT.to_owned(),
            Value::List(
                projection
                    .half_extent
                    .into_iter()
                    .map(Value::Float)
                    .collect(),
            ),
        ),
        (POSITION.to_owned(), triple(projection.eye)),
        (RIGHT.to_owned(), triple(projection.right)),
        (UP.to_owned(), triple(projection.up)),
    ])
}

/// Adds the depth buffer and the shaded image to the instance.
///
/// # Invariants
///
/// The three images are the size of the frame, and the present target is the same size as
/// the image it is written from. A post stage reads the shaded image and the depth that
/// produced it and writes the present target, and a sample of one read at the coordinates
/// of another is a frame of the wrong picture rather than a wrong picture.
fn add_targets(instance: &mut BTreeMap<String, Value>, frame: Frame) {
    instance.insert(
        DEPTH_BUFFER_ENTRY.to_owned(),
        texture(frame.width, frame.height, DEPTH_FORMAT),
    );
    instance.insert(
        COLOR_TARGET_ENTRY.to_owned(),
        texture(frame.width, frame.height, COLOR_FORMAT),
    );
    instance.insert(
        PRESENT_TARGET_ENTRY.to_owned(),
        texture(frame.width, frame.height, COLOR_FORMAT),
    );
}

/// Builds the three points that cover the frame.
///
/// # Invariants
///
/// The three corners are given in clip space directly, so a post stage's vertex stage
/// passes them through rather than projecting them. The mesh exists only so that a post
/// stage is an ordinary pass with an ordinary vertex buffer: there is no special case in
/// the renderer for drawing over the frame, and a post stage that wanted a different
/// shape would author a different mesh.
fn fullscreen_points() -> Vec<Value> {
    [(-1.0, -1.0), (3.0, -1.0), (-1.0, 3.0)]
        .into_iter()
        .map(|(x, y)| {
            Value::structure([(
                "position".to_owned(),
                Value::List([x, y, 0.0].into_iter().map(Value::Float).collect()),
            )])
        })
        .collect()
}

/// Builds the point schema the fullscreen mesh declares, which is a position alone.
fn fullscreen_point_schema() -> Rc<Schema> {
    Rc::new(Schema::new(SchemaKind::structure([(
        "position".to_owned(),
        graphics::layout::components_schema(3),
    )])))
}

/// Builds the pass a post stage draws with, when the scene declares one.
///
/// A pass builder's arguments are the pass's inputs, and every one of these is a different
/// thing rather than two ways of saying one: the shader, what the pass draws through, how
/// large the images are, the two numbers the scene carries into the stage, and the pair of
/// images the pass moves between. The allowance is the same one `lighting_pass` and `rasterize`
/// take, for the same reason.
#[allow(clippy::too_many_arguments)]
fn post_pass(
    scene_shader: &ShaderRead,
    projection: &Projection,
    frame: Frame,
    time: f64,
    exposure: Option<f64>,
    source: &str,
    target: &str,
    images: &[(String, Value)],
) -> Result<(Value, Value, String), CompileError> {
    // The vertex buffer is packed once and entered into the instance, the same way a
    // mesh's is, because a post stage draws a mesh like any other pass does.
    let points = fullscreen_points();
    let point_schema = fullscreen_point_schema();
    let bytes = pack_points(point_schema.as_ref(), &points, &Path::root().key("points"))?;
    let vertex_entry = format!("{FULLSCREEN_MESH}_vertices");
    let size_bytes = bytes.len() as i64;
    let entry = buffer_payload(bytes, point_schema.to_value(), &["vertex"], "Triangles");

    let mut uniform = vec![
        (CAMERA.to_owned(), camera_value(projection)),
        (PROJECTION.to_owned(), projection_value(projection)),
        (TIME.to_owned(), Value::Float(time)),
        // A depth is a fraction of the range between the two planes, and turning it back
        // into a distance is a divide by the range. The two numbers travel with the pass
        // for the same reason the camera's basis does: the stage is not given them
        // otherwise and cannot recover them.
        (
            DEPTH_RANGE.to_owned(),
            Value::List(
                [projection.near, projection.far]
                    .into_iter()
                    .map(Value::Float)
                    .collect(),
            ),
        ),
    ];
    // The exposure is written **only** when the scene named one. The uniform is the layout
    // the pass is uploaded with, so an always-present field would be a field every post
    // shader would have to declare and every existing scene would have to change; this way
    // a scene that names none emits the documents it emitted before the field existed.
    if let Some(exposure) = exposure {
        uniform.push((EXPOSURE.to_owned(), Value::Float(exposure)));
    }
    let uniform = Value::structure(uniform);
    let pass = rasterize(
        &format!("post/{}", scene_shader.label),
        &scene_shader.label,
        uniform,
        // The images sit among the pass's own binders rather than after them, because a pass
        // binds its resources in **name order**: `color_target` and `depth_buffer` come first,
        // then the image and the sampler that reads it, then `present_target`, then `vertices`.
        // The point is a field of the interface like any other, so a screen shader reads it at
        // the number that order works out to, and `check_bindings` says so if it does not.
        images.iter().cloned().chain([
            (
                VERTICES.to_owned(),
                buffer_binding(&vertex_entry, size_bytes),
            ),
            (
                COLOR_TARGET.to_owned(),
                texture_binding(source, frame.width, frame.height, COLOR_FORMAT),
            ),
            (
                DEPTH_BUFFER.to_owned(),
                texture_binding(DEPTH_BUFFER_ENTRY, frame.width, frame.height, DEPTH_FORMAT),
            ),
            (
                PRESENT_TARGET.to_owned(),
                texture_binding(target, frame.width, frame.height, COLOR_FORMAT),
            ),
        ]),
        vec![PRESENT_TARGET],
        COLOUR_CLEAR,
        PipelineStateFields::default_state(),
    );
    Ok((pass, entry, vertex_entry))
}

/// Emits one pass per debug channel, and the instance entries they read.
///
/// # Invariants
///
/// - **Every channel writes a target of its own**, so asking for one cannot move a pixel of the
///   picture. A channel that tinted the frame in place would make the image being diagnosed a
///   different image, which is the one outcome worth more than the convenience it buys.
/// - **The fullscreen triangle is one entry for the whole pipeline.** A post stage already makes
///   `fullscreen_vertices`, from the same three points and the same point schema, so a channel's
///   entry is the *same buffer* rather than a second one under a colliding name — which is why it
///   is written only when it is not already there. The bytes are identical because the sources
///   are: a scene with no post stage gets one made here, and one with a post stage reuses the one
///   it has.
/// - **The image is the size of the frame**, because it is read at the coordinates of the pixel
///   it was written from, like every other image a post stage touches.
/// - **The pass carries no depth state.** It reads the depth buffer as a texture, and a pass that
///   both tested against a buffer and sampled it would be reading and writing one texture at once.
fn debug_passes(
    channels: &[DebugChannel],
    instance: &mut BTreeMap<String, Value>,
    camera: &Projection,
    frame: Frame,
) -> Result<Vec<Value>, CompileError> {
    if channels.is_empty() {
        return Ok(Vec::new());
    }

    let points = fullscreen_points();
    let point_schema = fullscreen_point_schema();
    let bytes = pack_points(point_schema.as_ref(), &points, &Path::root().key("points"))?;
    let vertex_entry = format!("{FULLSCREEN_MESH}_vertices");
    let vertex_bytes = bytes.len() as i64;
    let vertex_payload = buffer_payload(bytes, point_schema.to_value(), &["vertex"], "Triangles");

    let mut passes = Vec::with_capacity(channels.len());
    for &channel in channels {
        insert_entry_once(instance, vertex_entry.clone(), vertex_payload.clone());
        insert_entry(
            instance,
            channel.point().to_owned(),
            texture(frame.width, frame.height, channel.format()),
        )?;
        insert_entry(
            instance,
            channel.shader_entry().to_owned(),
            Value::variant(
                "Shader",
                [
                    (
                        SOURCE.to_owned(),
                        Value::String(channel.source().to_owned()),
                    ),
                    (
                        graphics::binders::SCHEMA.to_owned(),
                        channel.pass_schema().to_value(),
                    ),
                ],
            ),
        )?;

        passes.push(rasterize(
            channel.label(),
            channel.shader_entry(),
            Value::structure([
                (CAMERA.to_owned(), camera_value(camera)),
                (PROJECTION.to_owned(), projection_value(camera)),
            ]),
            {
                // The binders are the channel's own and not one list for all of them: a `grade`
                // pass copies the shaded image and never reads the depth, and a binder its
                // source cannot read would be a number in the layout that means nothing — and
                // worse, a name in the sorted order that moves the ones it does read. The list
                // a depth channel gets is the one it got before this, in the same order.
                let mut binders = vec![(
                    channel.point().to_owned(),
                    texture_binding(channel.point(), frame.width, frame.height, channel.format()),
                )];
                if channel.reads_depth() {
                    binders.push((
                        DEPTH_BUFFER.to_owned(),
                        texture_binding(
                            DEPTH_BUFFER_ENTRY,
                            frame.width,
                            frame.height,
                            DEPTH_FORMAT,
                        ),
                    ));
                } else {
                    binders.push((
                        COLOR_TARGET.to_owned(),
                        texture_binding(
                            COLOR_TARGET_ENTRY,
                            frame.width,
                            frame.height,
                            COLOR_FORMAT,
                        ),
                    ));
                }
                binders.push((
                    VERTICES.to_owned(),
                    buffer_binding(&vertex_entry, vertex_bytes),
                ));
                binders
            },
            vec![channel.point()],
            DEBUG_CLEAR,
            PipelineStateFields::default_state(),
        ));
    }
    Ok(passes)
}

/// Builds the images an emitted document says a render produces.
///
/// # Invariants
///
/// The frame is named here rather than left to a convention about pass order, and that is the
/// whole reason the field exists: a channel's pass is emitted after the post processes, so under
/// the old rule — the colour target of the last pass that drew into one — asking for a debug
/// image would silently stop the picture coming back. `frame_target` is the point that *would*
/// have been the frame under that rule, so a document that declares this is not a document that
/// changed its picture.
///
/// **Every entry is a full declaration** — point, location and type — rather than a bare point.
/// The three are written from one place so that the location the compiler built the pass at and
/// the type the compiler drew it in are the same two numbers a reader finds in the document, and
/// the device's check compares the third against the shader rather than against this function's
/// opinion. A built-in takes its declaration from the registry ([`DebugChannel::declaration`]),
/// so a new built-in cannot declare one thing and be drawn as another.
///
/// The frame's own type is `vec4<f32>` because that is what every shader in this pipeline
/// returns at location zero: the post stage's colour, and a lighting pass's shaded image when
/// there is no post stage at all. It is stated rather than left to the convention so that the
/// frame is checked by the same rule as every other output — a pipeline whose frame shader
/// stopped returning a `vec4` would be caught here rather than arriving as a picture.
fn debug_outputs(frame_target: &str, channels: &[DebugChannel]) -> Value {
    let mut outputs = FieldMap::new();
    outputs.insert(
        graphics::PRESENT_OUTPUT.to_owned(),
        graphics::OutputDeclaration {
            point: frame_target,
            // Every shader in this pipeline writes one colour target per pass at location zero,
            // and the frame is the first and only colour target of the pass that draws it.
            location: 0,
            ty: graphics::TexelType::Vec4Float.name(),
        }
        .to_value(),
    );
    for &channel in channels {
        outputs.insert(
            channel.output().to_owned(),
            channel.declaration().to_value(),
        );
    }
    Value::Dict(outputs)
}

/// Builds the shadowing pass of one group for one light, drawing into the page that light
/// was given.
///
/// # Invariants
///
/// The page is named on the **declaration** of the pool, which is the only place it can be
/// named: the entry itself is the whole pool and is shared by every shadowing pass and by the
/// lighting pass, so a payload naming a page would name it for the whole image. A device
/// refuses a layer the pool does not have rather than clamping it, because a pass that wrote
/// past the end would write page zero again and hand one light another's shadow.
///
/// The projection is the light's own, and it is the same matrix the light's record carries,
/// because a fragment finds the page by carrying its world position through that matrix. A
/// pair of functions that each built "the light's projection" would agree until one of them
/// was edited, and until then every shadow in the scene would be sampled at the wrong place.
fn shadowing_pass(
    group: &Group<'_>,
    light: &LightRead,
    bounds: &Bounds,
    pool: &LightPool,
    page: usize,
    time: f64,
    images: &[(String, Value)],
) -> Value {
    // A shadowing pass is only emitted for a light that was given a page, and a light that
    // casts no shadow is given none, so the volume is there by the time it is asked for. A
    // light with no volume is a page the pool was never going to hand it, so the two facts
    // cannot come apart here.
    let (light_matrix, light_eye) = light
        .shadow_volume(bounds)
        .expect("a shadowing pass is only emitted for a light with a shadow volume");
    rasterize(
        &group.label(),
        group.shader,
        Value::structure([
            (
                PROJECTION.to_owned(),
                matrix_projection(light_matrix, light_eye),
            ),
            (TIME.to_owned(), Value::Float(time)),
        ]),
        mesh_bindings(group)
            .chain([(
                SHADOW_POOL.to_owned(),
                texture_array_binding(
                    SHADOW_POOL,
                    pool.page_width,
                    pool.page_height,
                    pool.pages as i64,
                    SHADOW_MAP_FORMAT,
                    Some(page as i64),
                ),
            )])
            .chain(images.iter().cloned()),
        vec![SHADOW_POOL],
        DEPTH_CLEAR,
        PipelineStateFields::default_state(),
    )
}

/// Builds the depthing pass of one group.
fn depthing_pass(
    group: &Group<'_>,
    camera: &Value,
    frame: Frame,
    time: f64,
    images: &[(String, Value)],
) -> Value {
    rasterize(
        &group.label(),
        group.shader,
        Value::structure([
            (PROJECTION.to_owned(), camera.clone()),
            (TIME.to_owned(), Value::Float(time)),
        ]),
        mesh_bindings(group)
            .chain([(
                DEPTH_BUFFER.to_owned(),
                texture_binding(DEPTH_BUFFER_ENTRY, frame.width, frame.height, DEPTH_FORMAT),
            )])
            .chain(images.iter().cloned()),
        vec![DEPTH_BUFFER],
        DEPTH_CLEAR,
        PipelineStateFields::default_state(),
    )
}

/// Builds the lighting pass of one group, which shades every light of every cluster.
///
/// # Invariants
///
/// The pass names **no light**. It binds the three buffers the assignment packed and the pool,
/// and a fragment works out which cluster it is in from its own position and loops over that
/// cluster's list. Everything a per light pass used to carry in its uniform — the light's
/// position, its colour, its own projection, the size of its shadow map's texel — is a field
/// of the light's **record**, which is read per light from a storage buffer, so the uniform is
/// the same sixteen kilobytes whatever the scene's light count is.
///
/// That is the whole reason the record is a storage buffer and not the uniform: a uniform
/// cannot hold a thousand light records, and a scene that could only be drawn with a few dozen
/// lights was the limit this replaces.
///
/// The grid and the frame are carried so a fragment can resolve its own cluster. It could not
/// derive the extents from anything, and it derives its own three indices from them with a
/// multiply and a cast each — the depth being the one number a fragment already carries.
///
/// The cone run is bound **unconditionally**, by a scene that names no spot light as much as by
/// one that names a hundred. The device numbers a pass's binders in name order, so a binding
/// that came and went with the document would move every number after it and a shader would
/// have to be written for one scene and not for another — the same argument the environment
/// map is bound on, and the same answer it is given by binding a black one texel map. A scene
/// with no spot light writes one whole sphere that no cluster's light points at, so the cost is
/// one bound buffer nothing reads rather than a shader that cannot be written.
///
/// The depth buffer is named among the **targets**, which is the whole of what makes the
/// pass's `LessEqual` test exist. A fragment of a prepassed surface lands on the depth the
/// prepass recorded, so the test keeps it; a fragment of a surface behind one already drawn
/// is rejected, which is the occlusion the prepass was paid for. A resource a pass renders
/// into cannot be bound to it, so the point takes its number in name order and no line of
/// the shader reads it.
#[allow(clippy::too_many_arguments)]
fn lighting_pass(
    group: &Group<'_>,
    camera: &Value,
    clustering: &Clustering,
    clusters: &lighting_pool::Clusters,
    cones: &PackedList,
    pool: &LightPool,
    environment: &EnvironmentMap,
    directional: &DirectionalLight,
    frame: Frame,
    time: f64,
    images: &[(String, Value)],
) -> Value {
    rasterize(
        &group.label(),
        group.shader,
        Value::structure([
            (PROJECTION.to_owned(), camera.clone()),
            (
                CLUSTER_GRID.to_owned(),
                Value::List(
                    [
                        clustering.width_extents as f64,
                        clustering.height_extents as f64,
                        clustering.depth_extents as f64,
                        clustering.lights_per_cluster as f64,
                    ]
                    .into_iter()
                    .map(Value::Float)
                    .collect(),
                ),
            ),
            (
                ENVIRONMENT.to_owned(),
                // The field is written for every scene, present or not, because the uniform's
                // layout is the same for every scene and a field that came and went would put
                // everything after it somewhere else. What a scene with no environment light
                // writes is a zero colour and a zero flag, and the shader takes the stand-in
                // path on the flag rather than on the colour: see `EnvironmentMap`.
                environment.uniform(),
            ),
            (
                DIRECTIONAL.to_owned(),
                // Written on the same terms as the environment above and for the same reason:
                // the layout is the same for every scene, so a scene with no directional light
                // writes the absent one rather than nothing.
                directional.uniform(),
            ),
            (
                FRAME_SIZE.to_owned(),
                Value::List(
                    [frame.width as f64, frame.height as f64]
                        .into_iter()
                        .map(Value::Float)
                        .collect(),
                ),
            ),
            (
                GLOBAL_COUNT.to_owned(),
                // How many of the leading records are the global lights, and nothing else: a
                // fragment loops over `[0, global_count)` reading the record and the cone of
                // each. Zero for a scene whose lights are all clustered, which is the answer
                // the loop is shaped to take without running at all.
                Value::Float(clusters.globals as f64),
            ),
            (TIME.to_owned(), Value::Float(time)),
        ]),
        mesh_bindings(group)
            .chain([
                (
                    COLOR_TARGET.to_owned(),
                    texture_binding(COLOR_TARGET_ENTRY, frame.width, frame.height, COLOR_FORMAT),
                ),
                (
                    DEPTH_BUFFER.to_owned(),
                    texture_binding(DEPTH_BUFFER_ENTRY, frame.width, frame.height, DEPTH_FORMAT),
                ),
                (
                    SHADOW_POOL.to_owned(),
                    // No layer, and that is the point: this pass only *reads* the pool, and a
                    // shader indexes the layers itself with the page each record carries. A
                    // layer on a binding that is not one of the pass's targets is refused by
                    // the device rather than ignored, because it is what a pass draws into.
                    texture_array_binding(
                        SHADOW_POOL,
                        pool.page_width,
                        pool.page_height,
                        pool.pages as i64,
                        SHADOW_MAP_FORMAT,
                        None,
                    ),
                ),
            ])
            .chain(
                clusters
                    .declarations()
                    .into_iter()
                    .map(|(name, value)| (name.to_owned(), value)),
            )
            .chain([(
                SPOT_CONES_ENTRY.to_owned(),
                lighting_pool::declaration(SPOT_CONES_ENTRY, cones.bytes().len() as i64),
            )])
            .chain([environment_binding(environment)])
            .chain(images.iter().cloned()),
        // The depth buffer is named as a **target** and not only declared as a binder, which
        // is what makes this pass's own depth state real: a pipeline reads its depth state
        // off the depth attachment it was given, so a pass that declares `LessEqual` and is
        // given no attachment is built with no test at all and its occlusion silently stops
        // existing. The point stays in the numbering while it is a target, because the device
        // numbers a pass's binding points rather than its bound resources; see
        // `Bindings::binding_numbers`.
        vec![COLOR_TARGET, DEPTH_BUFFER],
        COLOUR_CLEAR,
        lighting_state(),
    )
}

/// Builds the binding point the environment map is read at, whatever the scene's environment is.
///
/// # Invariants
///
/// The map declares **no sampler**. It is read with `textureLoad` at a texel the shader
/// computes, which is what keeps a filtered read from bleeding across the octahedral fold's
/// seam, and a texture that declares no `sampled` takes exactly one binding number.
fn environment_binding(environment: &EnvironmentMap) -> (String, Value) {
    (
        ENVIRONMENT_MAP.to_owned(),
        texture_binding(
            &environment.entry,
            environment.image.width,
            environment.image.height,
            &environment.image.format,
        ),
    )
}

/// Builds the emission pass of one group.
///
/// # Invariants
///
/// The pass reads no light, which is what makes it emit once rather than once per light. It
/// adds into the shaded image and tests against the depth the earlier passes wrote, so an
/// emissive surface behind an opaque one is occluded — the same state the lighting passes run
/// with, minus the shadow map and the light's projection, which it has no use for. The depth
/// buffer is therefore named among its targets for the same reason it is named among the
/// lighting pass's: a depth state without a depth attachment is a state nothing keeps.
fn emission_pass(
    group: &Group<'_>,
    camera: &Value,
    frame: Frame,
    time: f64,
    images: &[(String, Value)],
) -> Value {
    rasterize(
        &group.label(),
        group.shader,
        Value::structure([
            (PROJECTION.to_owned(), camera.clone()),
            (TIME.to_owned(), Value::Float(time)),
        ]),
        mesh_bindings(group)
            .chain([
                (
                    COLOR_TARGET.to_owned(),
                    texture_binding(COLOR_TARGET_ENTRY, frame.width, frame.height, COLOR_FORMAT),
                ),
                (
                    DEPTH_BUFFER.to_owned(),
                    texture_binding(DEPTH_BUFFER_ENTRY, frame.width, frame.height, DEPTH_FORMAT),
                ),
            ])
            .chain(images.iter().cloned()),
        // Named as a target for the same reason the lighting pass names it: the depth state
        // this pass declares is honoured through the attachment or not at all, and a pass
        // that declares one and is given none is refused rather than drawn without it.
        vec![COLOR_TARGET, DEPTH_BUFFER],
        COLOUR_CLEAR,
        lighting_state(),
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
///
/// A light gets a matrix and an eye and nothing else: it is not a camera, and nothing
/// draws through it as though it were.
///
/// This is asked of a light that **stands somewhere**. An environment light stands nowhere,
/// casts no shadow and is given no page, so it never reaches here: [`LightRead::shadow_projection`]
/// answers for it with the identity rather than building a volume around a position it does
/// not have. A directional light — whose shadow volume is a box seen along a direction rather
/// than a box looked at from somewhere — is not a light this function can be asked about
/// either, and `read_lights` refuses it before this is reached.
fn light_projection(position: [f64; 3], shadow_extent: f64, bounds: &Bounds) -> (Matrix, [f64; 3]) {
    let centre = bounds.centre();
    (
        projection_of(
            &Bounds::around(centre, shadow_extent),
            position,
            centre,
            FRAMING_PADDING,
        ),
        position,
    )
}

/// The near plane of a spot light's page, as a fraction of its far plane.
///
/// # Invariants
///
/// **Not** the light itself. A page that starts at the light has a depth range of thousands to
/// one, and a depth comparison over such a range cannot be biased: the constant a shadow lookup
/// subtracts is in the page's own normalised depth, where one world unit near the far plane is
/// a millionth of it, so the bias that fixes acne near the light detaches every shadow from its
/// caster by tens of units near the far one. A fiftieth of the far plane is a range a bias can
/// be written for.
///
/// The cost of that choice is stated rather than hidden: a fragment nearer the light than this is
/// clipped out of the page, and the shader reads a fragment whose depth is **below** the page's
/// near plane as unshadowed, because its depth compares less than anything the page recorded.
/// So the sliver of scene right under a lamp has no shadow, which is a missing shadow rather
/// than one in the wrong place, and a cone's own attenuation has already zeroed everything
/// outside the cone besides.
const SPOT_NEAR_FRACTION: f64 = 0.05;

/// Returns the perspective projection a spot light's page is drawn through, and the eye it is
/// made from.
///
/// # Invariants
///
/// **The page is the narrower of the cone and the column of half width `shadow_extent`, reaching
/// from near the light to where the scene ends along the cone's axis.** That is the whole
/// decision, and it is made here rather than in the schema because it is a decision about which
/// box a projection covers, which is what this file answers.
///
/// It has to be one of them rather than the other, and the reason is which way each one fails.
///
/// - The cone is the light's own shape. Cutting it short to reach a `shadow_extent` the author
///   wrote narrow would leave the outer band of the cone with **no shadow at all**, and a
///   missing shadow is a shadow that is not there rather than one that is wrong.
/// - The column is how much of the scene the author said this light shadows. Ignoring it would
///   render the page from a light aimed at a scene smaller than the author asked for. A fragment
///   beyond the **far** plane is the dangerous case and not the near one: the shader returns
///   unshadowed for a fragment whose page coordinate falls outside the page, but a fragment
///   past the far plane still lands *inside* the page and is compared against a depth recorded
///   from nearer, so it reads as fully shadowed. The far plane is therefore **never** inside
///   the scene, whatever the extent says — which is why the extent narrows the angle and never
///   shortens the reach.
///
/// Taking the narrower half angle rather than the shorter of the two volumes is what makes both
/// of those hold at once: at every distance the page is as wide as the smaller of what the cone
/// would be and what the column would be, and it reaches the whole scene either way.
///
/// The far plane is the scene's **support interval** along the axis, which is the furthest the
/// box reaches in the direction the light shines, and not the axis-aligned box's own far face:
/// a box corner is the far end along an axis only when that axis points at the box, and a spot
/// light aimed across its own scene — which is the case that matters — is not.
///
/// The page is fitted to the whole drawn box, which over-covers when the box is much larger than
/// the pool of light. That is the same trade [`light_projection`] makes, and it is taken for
/// the same reason: an exact cone and box intersection is a harder answer than a scene whose
/// shadow page is a little larger than the shadow in it.
///
/// The aspect is one, so the page is a square that **circumscribes** the cone's disc at the far
/// plane rather than being inscribed in it. The corners of a page are the parts of it furthest
/// from the axis, so a page inscribed in the cone would render occluders that are outside the
/// cone into a page whose middle is empty.
fn spot_projection(
    position: [f64; 3],
    cone: &SpotCone,
    shadow_extent: f64,
    bounds: &Bounds,
) -> (Matrix, [f64; 3]) {
    // The furthest point of the box along the axis, which is the **sum** of each axis's own
    // furthest end: a point of the box is a choice on every axis, so the furthest point is the
    // one that takes the far end of all three at once. Taking the largest single term instead
    // is a page that stops inside the scene, and everything beyond it reads as fully shadowed.
    // Floored above a hand's width, because a light standing inside its own scene and aimed at
    // a wall it is almost touching still has a page.
    let far_plane = (0..3)
        .map(|axis| cone.axis[axis])
        .zip(0..3)
        .map(|(direction, axis)| {
            let low = direction * (bounds.minimum[axis] - position[axis]);
            let high = direction * (bounds.maximum[axis] - position[axis]);
            low.max(high)
        })
        .sum::<f64>()
        .max(1e-2);
    // The half angle the author's column subtends at that distance, which is the width it asks
    // for written as a shape. The `min` of the two is the whole decision.
    let half_angle = cone
        .outer_angle
        .min((shadow_extent.max(1e-3) / far_plane).atan())
        .max(1e-3);
    (
        multiply(
            &perspective(
                2.0 * half_angle,
                1.0,
                (far_plane * SPOT_NEAR_FRACTION).max(1e-3),
                far_plane,
            ),
            // The eye is the light and the target is a point along the cone's axis, so the
            // basis is built from the direction the light shines rather than from a point the
            // scene happens to have. A cone is symmetric about its axis, so which of the two
            // perpendicular directions becomes "up" cannot be wrong; that is also what makes a
            // light aimed straight up or straight down safe, where `look_at`'s up is parallel to
            // the direction and its right hand axis falls back to a unit x.
            &look_at(position, add(position, cone.axis), [0.0, 1.0, 0.0]),
        ),
        position,
    )
}

/// How far behind the scene a directional light's eye stands, as a multiple of the page's own
/// half width.
///
/// # Invariants
///
/// **This number does not change the page.** An orthographic projection's near and far planes
/// are distances from the eye rather than a divide, and both are passed here as offsets from
/// the eye, so moving the eye back moves the planes with it and the same geometry lands at the
/// same place on the page. The only thing the distance reaches is `look_at`'s rotation, and
/// that is the normalized `centre - eye`, which is the light's direction whatever the distance
/// is — so the rotation is the distance-independent half and the page is the other half, and
/// neither of them is this number.
///
/// It is still a number, because a light that stands somewhere is a thing a reader can reason
/// about and an eye at the centre of the scene is not one. It has to be **behind the whole of
/// the bounds**, since an eye in front of part of the scene puts that part behind it.
const DIRECTIONAL_EYE_WIDTHS: f64 = 8.0;

/// How much room a directional light's page leaves beyond the scene along its own direction,
/// as a fraction of the page's half width.
///
/// # Invariants
///
/// The near plane sits a little **before** the nearest point of the box and the far plane a
/// little **after** the furthest, because a plane exactly on a bound puts the surfaces that
/// make up that bound on the plane itself, where the rasteriser's own rounding can clip them
/// and the shadow at the very edge of the scene disappears.
///
/// The cost of a margin is depth range: every unit of it is a unit the page's twenty four bits
/// of depth are spread over, and the whole reason the planes are derived from the bounds rather
/// than from the light is a range no wider than the scene. Hence a fraction of a tenth rather
/// than a plane a page width out.
const DIRECTIONAL_DEPTH_PADDING: f64 = 0.1;

/// Returns the orthographic projection a directional light's page is drawn through, and the eye
/// it is made from.
///
/// # Invariants
///
/// **Orthographic, and fitted to the scene's bounds rather than to a box around a place**, and
/// those are two separate decisions that the handoff recorded as one.
///
/// Orthographic is the obvious half and needs no defence here, because **a point light's page
/// is already orthographic**: [`projection_of`] builds `orthographic × look_at`, so the
/// "point lights get frusta and suns get parallel projections" contrast is not a fact about
/// this renderer. The only kind whose page is a perspective frustum is a **spot** light, whose
/// cone is what gives it a field of view. The handoff named the orthographic page as what makes
/// a sun's shadow the same sharpness across the whole image rather than sharp near the camera
/// and soft far away — which is true, but it is a claim about the *camera's* projection, not
/// about the light's, and this function is not what makes it true.
///
/// What is left, and what this function actually decides:
///
/// - **The half width is `shadow_extent` and nothing else.** It is the same field a point
///   light's is and it means the same thing — how much of the scene this light shadows — so a
///   scene mixing a lantern and a sun says which is which with one author's number. Nothing
///   here widens it to the bounds, because a page wider than the scene is a page whose texels
///   are spread thinner than the author asked for; a scene whose own stage is wider than the
///   extent is an author who has said the sun does not shadow the edges of it, and the edges
///   come out unshadowed by that decision rather than by a bug.
/// - **The page is fitted to the bounds, not to the light.** A point light's page is a box of
///   half width `shadow_extent` centred on the light and reaching `distance + 2 × half_width`
///   deep, so a light standing inside its own scene has a page that barely reaches past it and
///   a scene wider than `shadow_extent` falls **outside** it, where a fragment reads as
///   unshadowed. A sun's page is centred on the scene and is exactly as deep as the scene is
///   along the light, so the whole stage is inside it however the author sized the extent.
/// - **The depth range is the scene's own extent along the light**, which is what keeps a
///   shadow's edges at one sharpness from the near wall to the far one. A page several times
///   deeper than the geometry in it spends its texels on empty space, and the same number of
///   texels over a larger area is a blurrier shadow.
///
/// # Why the near and far planes come from the box's support interval
///
/// A box is only as deep as a given direction sees it when that direction points at one of its
/// axes, and a sun across its own scene is not pointing at an axis. The interval is therefore
/// the box's **support interval along the light's own direction**: a sum over the three scene
/// axes of `|direction| × half width`, because a point of the box is a choice on every axis at
/// once and the furthest of them takes the far end of all three.
///
/// The cost of a near plane at the nearest point of the box is stated rather than hidden. A
/// fragment nearer the light than this is clipped out of the page, and the shader reads a
/// fragment whose depth is **below** the page's near plane as unshadowed, because it compares
/// less than anything the page recorded. Nothing in the scene is in that sliver — the plane is
/// derived from the box, so the nearest surface in the scene is at or beyond it — which is why
/// here that cost is one a reader does not pay, and it is the same sliver
/// [`SPOT_NEAR_FRACTION`] accepts for a cone.
fn directional_projection(
    direction: [f64; 3],
    shadow_extent: f64,
    bounds: &Bounds,
) -> (Matrix, [f64; 3]) {
    let centre = bounds.centre();
    let half_width = shadow_extent.max(1e-3);
    // The furthest the box reaches in each of the two directions along the light's own axis,
    // as a **sum over the three scene axes**. A point of the box is a choice on every axis at
    // once, so the furthest of them is the one that takes the far end of all three; taking the
    // largest single term instead gives a page that stops inside the scene, and everything
    // beyond it reads as fully shadowed.
    let reach: f64 = (0..3)
        .map(|axis| direction[axis].abs() * bounds.half_width(axis))
        .sum();
    // Far enough back that the whole box is in front of the eye even when the box is far wider
    // than the page, and floored above a hand's width so a scene drawn inside a fraction of a
    // unit still has an eye.
    let eye_distance = (DIRECTIONAL_EYE_WIDTHS * half_width).max(reach * 2.0 + half_width);
    let eye = subtract(centre, scale(direction, eye_distance));
    let margin = DIRECTIONAL_DEPTH_PADDING * half_width;
    (
        multiply(
            &orthographic(
                half_width,
                eye_distance - reach - margin,
                eye_distance + reach + margin,
            ),
            // `look_at`'s up is parallel to the light when the light shines straight up or
            // straight down, and its right hand axis then falls back to a unit x. Which of the
            // two perpendicular directions becomes "up" cannot be wrong for a box looked at
            // along an axis, so the same call a spot light's cone makes is the right one here.
            //
            // The **order** is `projection × view`, which is the order [`projection_of`] and
            // [`spot_projection`] both use and the order a column major matrix acting on a
            // column vector wants. Reversed, the orthographic scale lands in the view's
            // rotation and the scene is rotated into a page and clipped, which is a page of
            // nothing rather than a wrong shadow.
            &look_at(eye, centre, [0.0, 1.0, 0.0]),
        ),
        eye,
    )
}

/// The boxes the instances of a scene cover: the scene's own, and one box per instance.
struct InstanceBounds {
    /// The box over everything that is drawn, which is what the camera frames.
    scene: Bounds,
    /// What each instance covers, in the order the instances were read.
    ///
    /// # Invariants
    ///
    /// One entry per instance, and `None` for an instance whose mesh has no points to cover
    /// anything with — so the list indexes by instance rather than carrying a label of its
    /// own, and a note about the third instance cannot be about the second.
    each: Vec<Option<Bounds>>,
}

/// Returns the box the instanced meshes of a scene cover, and what each instance covers.
///
/// # Invariants
///
/// The box is over what is drawn rather than over what is authored: a mesh that no instance
/// names is not drawn, and an instance places its mesh wherever its transform says. A scene
/// with nothing drawn covers a unit box, so a projection is still well defined.
///
/// The per instance boxes come out of **the same walk**, which is the whole reason the notes
/// about a frame are cheap. Every point of every instance is read exactly once here, and a
/// second pass to recover the box one instance covers would be the most expensive thing a
/// compile could do — see [`compile-cost.md`](../../../../.agents/proposals/compile-cost.md),
/// where the same walk is the row that fell from forty three milliseconds to a strided read
/// when the points were packed.
///
/// The positions are read a component at a time out of the packed run, so the whole loop is a
/// strided walk of bytes with no map lookup and no per point allocation.
fn bounds_of_instances(meshes: &[MeshRead], instances: &[InstanceRead]) -> InstanceBounds {
    let mut scene: Option<Bounds> = None;
    let mut each = Vec::with_capacity(instances.len());
    for instance in instances {
        let mesh = &meshes[instance.mesh];
        let mut covered = Bounds {
            minimum: [f64::INFINITY; 3],
            maximum: [f64::NEG_INFINITY; 3],
        };
        for point in 0..mesh.points.len() {
            let world = apply(&instance.model, packed_point_position(mesh, point));
            for axis in 0..3 {
                covered.minimum[axis] = covered.minimum[axis].min(world[axis]);
                covered.maximum[axis] = covered.maximum[axis].max(world[axis]);
            }
        }
        let finite = covered.minimum.iter().all(|value| value.is_finite());
        if finite {
            scene = Some(match scene {
                None => covered,
                Some(previous) => previous.union(covered),
            });
        }
        each.push(finite.then_some(covered));
    }
    InstanceBounds {
        scene: scene.unwrap_or_else(|| Bounds::around([0.0; 3], 1.0)),
        each,
    }
}

/// The share of the frame one instance has to cover before the frame is called that
/// instance's.
///
/// # Invariants
///
/// Ninety percent rather than half, and for two reasons that point the same way. The share
/// is an over-estimate — a box turned away from the axes is counted by the axis aligned box
/// around it — so the bar has to sit above the degenerate case rather than on it. And the
/// condition is for a frame that **is** one object, not for a close one: a subject that
/// fills most of the frame is a composition somebody chose, and a note about every close-up
/// is a note nobody reads. This is the water jar, not the lantern beside it.
const OWNED_FRAME: f64 = 0.9;

/// Says what this camera is of, when the frame it draws is one a scene probably did not mean.
///
/// # Why none of these refuses to compile
///
/// A scene may legitimately want a camera inside a volume, inside foliage, inside a
/// building, or pointed at something nearer than its own near plane. The policy the shadow
/// page pool established — *"a count above what the machine has is bounded and named; a
/// count below one is refused"* — generalises to **a machine being too small is a fact about
/// this run, a wrong image on every machine is a fact about the document**, and all three of
/// these are facts about this run and this camera. So all three are named and all three are
/// drawn, which is the half of that sentence that matters: a note here distinguishes *this
/// picture is degenerate* from *this picture is fine and the number is just unusual*.
///
/// # What the path names
///
/// The field the note is about, which is the half that makes a note worth reading: the
/// camera's `eye` when the eye is what is wrong, its `target` when the target is, and **the
/// instance** when one instance is — a note that names an instance and points at the camera
/// sends the reader past the thing the message just told them about.
///
/// # What this reads
///
/// Nothing that is not already in hand. The eye, the target and the near plane are fields of
/// the camera, and each instance's box came out of the single walk in `bounds_of_instances`.
/// Nothing here re-reads the scene document, which is the most expensive thing a compile can
/// do.
fn camera_notes(
    projection: &Projection,
    target: [f64; 3],
    camera_path: &Path,
    instances: &[InstanceRead],
    bounds: &InstanceBounds,
    notes: &mut Vec<CompileNote>,
) {
    // The eye inside a box. **One note per box rather than one per instance**, because a mesh
    // drawn in three stages is three instances of one box and the reader is owed one fact and
    // not three: `terraces` draws its terrain as `terrain-depth`, `terrain-shade` and
    // `terrain-cast`, which are the same valley three times, and three sentences saying so is
    // the noise this is meant not to be. Every box is still named, in the order the document
    // lists them: two nested boxes are two facts, and moving out of the near one may leave the
    // eye inside the far one, which is a different fix.
    let mut buried: Vec<(f64, Bounds, Vec<&str>)> = Vec::new();
    for (instance, covered) in instances.iter().zip(&bounds.each) {
        let Some(covered) = covered else {
            continue;
        };
        let Some(depth) = covered.depth_of(projection.eye) else {
            continue;
        };
        // The depth is not compared as well as the box: the same box and the same eye give
        // the same depth by the same arithmetic, so the box alone is the identity.
        let same_box = |held: &Bounds| *held == *covered;
        match buried.iter_mut().find(|(_, held, _)| same_box(held)) {
            Some((_, _, labels)) => labels.push(&instance.label),
            None => buried.push((depth, *covered, vec![&instance.label])),
        }
    }
    for (depth, _, labels) in buried {
        // The agreement is the note's, not the format's: one instance draws one box and
        // three instances draw one box between them, and "which are the box that instances
        // draws" is a sentence nobody reads twice.
        let held = if labels.len() == 1 {
            "which is the box that instance draws"
        } else {
            "which are the boxes that instances draw"
        };
        notes.push(CompileNote {
            path: camera_path.key(scene_schema::EYE),
            message: format!(
                "the eye of this camera is {depth:.3} units inside the bounds of {}, {held}, \
                 so the frame is drawn from within that box rather than from outside it. This \
                 compile draws it as written",
                quoted(&labels),
            ),
        });
    }

    // One instance holding the frame, and standing between the eye and what the camera was
    // aimed at. **Both halves are load bearing.** A backdrop wall covers the whole frame in
    // any still life, and that is what a backdrop is for; the jar that filled dsf's frame was
    // not a background but the thing in front of the subject. Only the largest is named: a
    // frame is dominated by one thing or by nothing, and naming every instance over the bar
    // would be the same fact several times. A box the eye is inside is not measured at all —
    // the divide decides there, and the note above has already said where it is.
    let aimed_at = projection.distance_to(target);
    let mut owned: Option<(f64, &InstanceRead)> = None;
    for (instance, covered) in instances.iter().zip(&bounds.each) {
        let Some(share) = covered
            .as_ref()
            .and_then(|box_of| projection.frame_coverage(box_of))
            .filter(|share| *share >= OWNED_FRAME)
        else {
            continue;
        };
        let in_front = covered
            .as_ref()
            .and_then(|box_of| box_of.distance_from(projection.eye))
            .is_some_and(|away| away < aimed_at);
        if !in_front {
            continue;
        }
        let larger = match owned {
            Some((best, _)) => share > best,
            None => true,
        };
        if larger {
            owned = Some((share, instance));
        }
    }
    if let Some((share, instance)) = owned {
        notes.push(CompileNote {
            path: instance.path.clone(),
            message: format!(
                "`{}` covers {:.1}% of this frame and stands between the eye and what this \
                 camera was aimed at, so the picture is that one box and whatever is behind \
                 it. The share is the box projected to the frame rather than the pixels it \
                 drew, so it is an upper bound for anything turned away from the axes",
                instance.label,
                share * 100.0
            ),
        });
    }

    // The target behind the near plane, which is the one of the three that is about a number
    // rather than a place: the camera was aimed at something the frame cannot hold.
    if aimed_at < projection.near {
        notes.push(CompileNote {
            path: camera_path.key(scene_schema::TARGET),
            message: format!(
                "this camera's target is {aimed_at:.3} units in front of the eye, which is \
                 nearer than the near plane at {:.3}, so the target and everything between it \
                 and the eye is clipped away and the frame holds what is behind the target",
                projection.near
            ),
        });
    }
}

/// The linear term of the falloff the standard shading template applies to a point light.
///
/// # Invariants
///
/// **This is a statement about a constant of the template, and not a field of a scene.** The
/// standard pipeline has no light falloff of its own: a lighting pass runs the scene's own
/// `WGSL`, and the gallery's shading template multiplies a light's radiance by
/// `1 / (1 + 0.0035r + 0.00002r²)`. The number is recorded here because the note below is a
/// statement about **what that constant means at a scene's own size**, and a compiler cannot
/// make that statement without holding the number.
///
/// It is a copy, deliberately and visibly: the lighting stage is being rewritten elsewhere and
/// the constant lives in the per-light arithmetic that rewrite replaces, so this is the place
/// to delete when the falloff becomes a field a scene sets. `scale_falloff_matches_the_gallery`
/// in the tests holds the copy to the template, so a change to one without the other is a
/// failed test rather than a quietly wrong note.
const FALLOFF_LINEAR: f64 = 0.0035;

/// The quadratic term of that same falloff. See [`FALLOFF_LINEAR`].
const FALLOFF_QUADRATIC: f64 = 0.00002;

/// The scene the falloff above was tuned for, which is the scene whose gradient is unremarkable.
///
/// # Invariants
///
/// The template's own comment places its constants in "a scene a couple of hundred units
/// across", and two hundred is the number this note measures other scenes against. It is
/// written down rather than left in a sentence in another crate, because a bar with no
/// reference beside it is a bar somebody re-picks by feel.
const FALLOFF_TUNED_SPAN: f64 = 200.0;

/// Returns the fold the same constant gives a scene about [`FALLOFF_TUNED_SPAN`] units across.
///
/// The measure is the one [`scale_notes`] uses, so the reference and the number beside it are
/// the same quantity: a cube of that span rather than a distance of that length, because a
/// scene two hundred units across has a half diagonal of a hundred and seventy.
fn tuned_fold() -> f64 {
    falloff_fold(half_diagonal(&Bounds::around(
        [0.0; 3],
        FALLOFF_TUNED_SPAN / 2.0,
    )))
}

/// The fold beyond which a scene's size is reported, in units of attenuation.
///
/// # Why two, and what the gallery says about it
///
/// **Two is one stop**: a doubling of the brightness, which is the smallest difference in
/// light a photograph is named for. It is a bar with a meaning rather than a bar with a
/// number, and it happens to be the right one on the evidence.
///
/// Measured across the ten gallery scenes — the numbers are pinned by
/// `the_gallery_says_how_often_a_scene_is_reported`, which is where this table comes from:
///
/// ```text
/// scene       span   half diagonal   fold
/// studio          4              2.1   1.01
/// sunlight       17             12.2   1.05
/// spotlight      18             12.9   1.05
/// tiles          24             17.0   1.07
/// dunes         330            233.6   2.91
/// terraces      380            277.7   3.51
/// stilllife     700            356.4   4.79
/// lanterns      600            424.3   6.09
/// monoliths     680            480.9   7.31
/// tidewater    2800           1980.1  86.35
/// ```
///
/// The data has three clusters and two gaps, and the bar sits in the **first** one. Four
/// scenes are small enough that the constant does nothing at all — a fold of `1.05` is five
/// percent across the whole scene, and no picture has ever noticed five percent. The next
/// scene is `dunes` at `2.91`, and a bar anywhere between `1.07` and `2.91` gives the same
/// six scenes; two has a margin of 1.9 below the loudest quiet scene and 1.5 above the
/// quietest reported one.
///
/// **The second gap was left alone.** `tidewater` at `86.35` is the case the note exists for
/// — the scene a thousand units deep whose sun is attenuated by twenty — and a bar placed in
/// the gap above `monoliths` would report that one scene and nothing else, which is a bar
/// fitted to a single observation rather than chosen from the data.
///
/// **Six of ten firing is a lot, and it is the honest count.** The five scenes between the
/// gaps are outdoor scenes a third of a thousand units across, where the falloff is a real
/// part of the light; `stilllife`'s own file says it wants that, because a window lighting a
/// table is a gradient. A scene that wants its gradient is told the number and carries on,
/// which is what a note is for: this compile draws it as written, and the sentence says so.
const FALLOFF_FOLD_BAR: f64 = 2.0;

/// Returns the fraction of its radiance the template's falloff leaves a light at `reach`.
///
/// # Invariants
///
/// The same arithmetic the template's own `WGSL` does, so the number a note quotes is the
/// number the frame was drawn with. A distance of zero is left alone rather than treated as a
/// degenerate one: the law is continuous at zero and a light inside a surface is a scene's
/// own doing.
fn falloff(reach: f64) -> f64 {
    1.0 / (1.0 + FALLOFF_LINEAR * reach + FALLOFF_QUADRATIC * reach * reach)
}

/// Returns how far the template's falloff is from flat, at `reach`.
///
/// One is no falloff at all and anything above one is a brightness gradient the scene's own
/// distances produce: the reciprocal of [`falloff`], so it reads as the factor by which a
/// surface at that distance is dimmer than one at the light.
fn falloff_fold(reach: f64) -> f64 {
    1.0 / falloff(reach)
}

/// Returns the furthest a point at the centre of `box_of` is from one of its corners.
fn half_diagonal(box_of: &Bounds) -> f64 {
    (0..3)
        .map(|axis| box_of.half_width(axis).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// Says what this scene's size means for a constant of the pipeline the scene cannot set.
///
/// # Why this is a note and never an error
///
/// The page pool's policy — *"a count above what the machine has is bounded and named; a
/// count below one is refused"* — separates *a machine being too small, a fact about this
/// run* from *a wrong image on every machine, a fact about the document*. A scene is a
/// **legitimately** any size: a still life a hundred units across wants a falloff across its
/// table, and a lake two thousand eight hundred units across wants the same constant to be a
/// factor of eighty. Neither is a fault, and refusing either would be a compiler ruling on a
/// scene's subject. So the consequence is named and the frame is drawn.
///
/// # What the measure is, and why it is a box
///
/// The distance is from the **centre of the scene's box to its furthest corner** — the
/// half diagonal — because that is the only distance the document determines on its own: where
/// a scene's lights stand is a fact about its lights, and a light at one end of a lake is two
/// thousand units from the other. A light at the middle of the scene therefore has its
/// brightness set by the scene's own size before a single intensity is chosen, and the fold
/// is what that size costs.
///
/// **So this is a box, and a box is not a surface**, which is the same limitation the camera
/// notes have and the same reason their numbers are in their messages: the compiler knows the
/// box every instance covers and knows nothing about which part of it is visible from where
/// the camera stands. `tidewater`'s box is nearly three thousand units across, and its far
/// shore is a few hundred away, so the box says a fold of eighty-six and the picture may have
/// a fold of two. The number is quoted rather than the verdict, so a reader can see `86.4`
/// and disagree with it.
///
/// # What this reads
///
/// The box, which came out of the single walk in [`bounds_of_instances`]. Nothing here re-reads
/// the scene document, which is the most expensive thing a compile can do.
fn scale_notes(bounds: &Bounds, notes: &mut Vec<CompileNote>) {
    let span = (0..3)
        .map(|axis| bounds.half_width(axis) * 2.0)
        .fold(0.0f64, f64::max);
    let fold = falloff_fold(half_diagonal(bounds));
    if fold < FALLOFF_FOLD_BAR {
        return;
    }
    notes.push(CompileNote {
        // The path is the **instances**, because the transforms there are the only thing in
        // the document that sets how far apart anything is. A note that pointed at the camera
        // would send the reader after a field that has nothing to do with a falloff.
        path: Path::root().key(scene_schema::INSTANCES),
        message: format!(
            "this scene spans {span:.0} units across its largest axis, and the standard \
             shading template's light falloff leaves a point light {fold:.1} times dimmer at \
             the half diagonal of that box than at its centre — about {stops:.1} stops. The \
             constants are the gentlest that read as a falloff over a scene about \
             {FALLOFF_TUNED_SPAN:.0} units across, where the same fold is {reference:.1}, so at \
             this size the brightness across the scene comes from how far its own lights stand \
             from its own surfaces rather than from anything the document says. The distance is \
             a **box** and not a surface, and the compile knows no more than the box: this is \
             drawn as written",
            stops = fold.log2(),
            reference = tuned_fold(),
        ),
    });
}

/// Returns names as a list a sentence can carry: one name bare, two joined by *and*, and a
/// longer list with an *and* before the last.
fn quoted(labels: &[&str]) -> String {
    let quoted: Vec<String> = labels.iter().map(|label| format!("`{label}`")).collect();
    match quoted.split_last() {
        Some((last, [])) => last.clone(),
        Some((last, rest)) if rest.len() == 1 => format!("{} and {last}", rest[0]),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
        None => String::new(),
    }
}

/// Returns the position one packed point carries.
///
/// # Invariants
///
/// A point schema is free to declare whatever a mesh needs, so the position is the field
/// **named** `position` and not the first three numbers in field order. A schema whose fields
/// sort as `normal`, `position`, `texture` has its normal first, and reading the first three
/// numbers of such a point reads the normal — which is a unit vector, so every such mesh appears
/// to be one unit across and the camera frames a box around the origin rather than the scene.
///
/// A schema naming no `position` falls back to the first three components, which is the only
/// reasonable guess; the fallback is over the run's own field order, which is the schema's.
fn packed_point_position(mesh: &MeshRead, index: usize) -> [f64; 3] {
    if let Some(declared) = mesh.point_schema.struct_field(POSITION_FIELD) {
        let width = components(POSITION_FIELD, declared).unwrap_or(3).min(3);
        let mut position = [0.0f64; 3];
        for (axis, slot) in position.iter_mut().enumerate().take(width) {
            *slot = mesh
                .points
                .component(index, POSITION_FIELD, axis)
                .unwrap_or(0.0);
        }
        return position;
    }
    first_packed_components(mesh, index)
}

/// Returns the first three numbers a packed point carries, in layout order.
fn first_packed_components(mesh: &MeshRead, index: usize) -> [f64; 3] {
    let mut position = [0.0f64; 3];
    let mut at = 0;
    'fields: for (name, components) in mesh.points.layout().fields() {
        for component in 0..*components {
            if at == 3 {
                break 'fields;
            }
            position[at] = mesh.points.component(index, name, component).unwrap_or(0.0);
            at += 1;
        }
    }
    position
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

/// The colour a debug image is cleared to, which is no surface.
///
/// # Invariants
///
/// Alpha is the marker rather than a component of the value, so the clear has to say "nothing
/// here" in the same channel a fragment does. A debug pass draws the whole frame with a
/// fullscreen triangle, so every pixel is written and the clear is never seen — which is exactly
/// why it has to be right: the day a pass stops covering the frame is the day a background pixel
/// starts reporting the origin of the scene as a position.
const DEBUG_CLEAR: [f64; 4] = [0.0, 0.0, 0.0, 0.0];

/// Builds a rasterize pass with the shared fields every pass carries.
///
/// `depth` is the pass's depth state, or `None` for a pass that writes the depth it draws
/// and keeps the nearer fragment, which is what a pass into a depth map or a depth buffer
/// wants.
#[allow(clippy::too_many_arguments)]
fn rasterize(
    label: &str,
    shader_entry: &str,
    uniform: Value,
    binders: impl IntoIterator<Item = (String, Value)>,
    targets: impl IntoIterator<Item = &'static str>,
    clear: [f64; 4],
    state: PipelineStateFields,
) -> Value {
    let mut fields = vec![
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
    ];
    let (blend, depth) = state.into_fields();
    if let Some(blend) = blend {
        fields.push((BLEND.to_owned(), blend));
    }
    if let Some(depth) = depth {
        fields.push((DEPTH.to_owned(), depth));
    }
    Value::variant("Rasterize", fields)
}

/// The blend and depth a pass declares, or the absence of either.
///
/// A pass that names neither builds the pipeline it always built: it replaces what is
/// there, it records the depth of what it draws, and it keeps the nearer fragment.
struct PipelineStateFields {
    blend: Option<Value>,
    depth: Option<Value>,
}

impl PipelineStateFields {
    /// The state of a pass that writes a depth map or a depth buffer.
    fn default_state() -> Self {
        Self {
            blend: None,
            depth: None,
        }
    }

    /// Returns the two fields, for a pass that declares them.
    fn into_fields(self) -> (Option<Value>, Option<Value>) {
        (self.blend, self.depth)
    }
}

/// Builds the pipeline state of a lighting pass, and of the emission pass beside it.
///
/// # Invariants
///
/// - **The pass adds.** Every lighting pass of a scene draws into the same image, so a pass
///   that replaced what was there would leave only the last group standing. Adding is what
///   makes several passes into a sum rather than a choice, and the first pass to write the
///   target clears it to black, which added to nothing is the same as replaced. The emission
///   pass adds on the same grounds: what a surface emits is added to what was shaded.
/// - **The comparison is `LessEqual` and the pass does not write.** A depth prepass
///   records the depth of every opaque fragment, and a later pass drawing the *same*
///   geometry lands on exactly those depths — so a comparison of `Less` rejects every one
///   of them, and the pass draws nothing at all. `LessEqual` keeps the nearer fragment,
///   which is what a prepass exists to establish, and not writing keeps the prepass's
///   depths rather than replacing them with the same numbers.
/// - **The state is honoured only through a depth attachment.** A pipeline reads this state
///   off the depth texture its pass renders into, so a pass that declares it and names no
///   depth format among its targets is a declaration nothing keeps: it was built with no
///   test at all for as long as the lighting and emission passes named one target, and the
///   frames that came out of it read as a style rather than as a defect. Both passes name
///   the depth buffer as a target, and the renderer now refuses a depth state it cannot
///   attach; see `WgpuRenderer::run_rasterize`.
fn lighting_state() -> PipelineStateFields {
    let (blend, depth) = (
        Value::variant("Add", []),
        Value::structure([
            (DEPTH_WRITE.to_owned(), Value::Bool(false)),
            (DEPTH_COMPARE.to_owned(), Value::variant("LessEqual", [])),
        ]),
    );
    PipelineStateFields {
        blend: Some(blend),
        depth: Some(depth),
    }
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
///
/// # Invariants
///
/// The usage is a list because a buffer may need more than one word: a storage buffer a
/// shader writes declares `storage` and `storage_write` together, and a vertex layout is
/// built from the schema the bytes were packed with either way.
fn buffer_payload(bytes: Vec<u8>, schema: Value, usage: &[&str], topology: &str) -> Value {
    Value::variant(
        "Buffer",
        [
            (SIZE_BYTES.to_owned(), Value::Int(bytes.len() as i64)),
            (
                graphics::binders::USAGE.to_owned(),
                Value::List(
                    usage
                        .iter()
                        .map(|word| Value::String((*word).to_owned()))
                        .collect(),
                ),
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

/// Builds the payload of a pool of shadow pages: one entry, one array, every light's page.
///
/// # Invariants
///
/// The payload is a `TextureArray` rather than a `Texture` carrying a count, because those are
/// two `WGSL` types with a different `textureLoad` arity and a document that read `Texture`
/// while meaning an array is a document no shader can name. It carries no `data`: a pool is
/// written by shadowing passes, and filling a texture averages neighbouring texels and cannot
/// tell a layer boundary from a row, so an array with pixels is refused.
fn texture_array(width: i64, height: i64, layers: i64, format: &str) -> Value {
    Value::variant(
        "TextureArray",
        [
            (WIDTH.to_owned(), Value::Int(width)),
            (HEIGHT.to_owned(), Value::Int(height)),
            (LAYERS.to_owned(), Value::Int(layers)),
            (FORMAT.to_owned(), Value::String(format.to_owned())),
        ],
    )
}

/// Builds a declaration of a texture array, and of the layer a pass draws into.
///
/// # Invariants
///
/// `layer` is `Some` only for a pass that renders into the pool. A pass that only reads it
/// names none, because the page a fragment looks up is in the light's own record and a shader
/// cannot be told which light to shade at bind time.
fn texture_array_binding(
    source: &str,
    width: i64,
    height: i64,
    layers: i64,
    format: &str,
    layer: Option<i64>,
) -> Value {
    let mut resource = vec![
        (SOURCE.to_owned(), Value::String(source.to_owned())),
        (WIDTH.to_owned(), Value::Int(width)),
        (HEIGHT.to_owned(), Value::Int(height)),
        (LAYERS.to_owned(), Value::Int(layers)),
        (FORMAT.to_owned(), Value::String(format.to_owned())),
    ];
    if let Some(layer) = layer {
        resource.push((LAYER.to_owned(), Value::Int(layer)));
    }
    Value::variant(
        "Resource",
        [(
            RESOURCE.to_owned(),
            Value::variant("TextureArray", resource),
        )],
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

/// Borrows an optional number field, which is how a field the document may leave out is read.
fn optional_number(value: &Value, path: &Path, name: &str) -> Result<Option<f64>, CompileError> {
    match field(value, name) {
        None => Ok(None),
        Some(_) => number(value, path, name).map(Some),
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

/// Wraps a layout failure as a packing failure at a mesh's points.
fn unpackable(error: graphics::LayoutError) -> CompileError {
    CompileError::Unpackable {
        path: Path::root(),
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
