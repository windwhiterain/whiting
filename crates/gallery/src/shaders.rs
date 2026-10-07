//! The shaders a gallery scene declares, and the parts of them that never change.
//!
//! A shader carries its own source, so a scene names a shader from here and the document
//! it produces is still self contained. That is what lets a scene be sent to a device that
//! cannot see the file it was written in.
//!
//! # Writing a shader against this pipeline
//!
//! Three layouts have to be matched, and only one of them is checked. The device parses a
//! shader back with `naga` and compares its uniform struct against the interface the
//! shader declared, so a uniform that disagrees is a render error naming both offsets
//! rather than a frame of the wrong numbers. The other two are not checked, so they are
//! stated here once rather than left to each shader:
//!
//! - **The vertex attributes.** A point schema sorted by name lays out locations from
//!   zero, and the instance row continues from where the point's ended. A row of
//!   `emissive`, `metallic`, `model`, `roughness`, `tint` sorts into that order, so the
//!   model matrix is read as four consecutive locations.
//! - **The bindings.** The uniform is binding zero, and every binding point follows it in
//!   name order, whether the pass binds it or renders into it. A point the pass renders into
//!   takes its number and emits no entry, and a vertex or instance buffer takes no number at
//!   all, so the numbering runs over the pass's points rather than over what it binds. The
//!   lighting stage's numbers are written out in [`lighting_bindings`] rather than counted in
//!   prose, and the post stage binds the shaded image at one and the depth at two.
//!
//! # The lighting stage is clustered
//!
//! It used to be one pass per light, which meant the pass count grew with the scene's lamp
//! count and the uniform had to hold a light. It is now one pass per group: a fragment
//! resolves which cluster of the view it is in from its own position, reads that cluster's
//! light list out of three storage buffers, and loops over it. Everything a per light pass
//! carried in its uniform is a field of a light's record now, which is what makes a thousand
//! lights a matter of buffer size rather than of passes.
//!
//! # What is outside that loop
//!
//! Three things, and only one of them is a light with a field of its own. A light that names
//! no reach, or a reach that holds the whole scene, is **global**: it is in no cluster list,
//! the compiler writes it first, and the fragment loops over the leading records outside the
//! cluster walk. The environment and the directional light are the other two, and they are
//! *stated* inputs of the pass rather than records, because neither is a lamp — one is an
//! image and one is a direction.

use std::borrow::Cow;

use definition::{Schema, SchemaKind, Value};

use scene::standard::{
    Stage, depth_pass_schema, emission_pass_schema, light_pass_schema, light_pass_schema_over,
    post_pass_schema, shadow_pass_schema,
};

use super::kit;

/// Declares a block of WGSL, and with it where the block was written.
///
/// # Invariants
///
/// - **The file is the invocation's own**, read by `file!()` as the macro expands, so it cannot
///   drift from the text it describes. Nothing counts lines and nothing is written down twice.
/// - **The line is the line after the one the macro was invoked on**, which is where a literal
///   opening on the next line puts the block's first line of WGSL. The literal must therefore
///   open on the line below the invocation, as it is written below and as `cargo fmt` writes it:
///   `line!()` reads the invocation and cannot see where the literal opens, so a literal opened
///   on the invocation's own line is reported one line high. Nothing in the expansion says which
///   of the two shapes it was given, and no rule can be right for both — two invocations holding
///   the same literal and starting on the same line put the block's text on different lines. The
///   arithmetic is stated for the one the formatter writes, and a test in
///   `tests/shader_provenance.rs` holds every declaration in this crate to it.
/// - **A block's text must not contain a line beginning [`graphics::provenance::MARKER`]**,
///   which is reserved and would be read as a declaration of its own.
///
/// ```ignore
/// const SHORE: Block = wgsl!(
///     r#"
///     color = color * (0.4 + 0.6 * shore_tint);
/// "#
/// );
/// ```
///
/// A block written as a plain `&str` says nothing about where it came from, and that is the
/// other half of the contract: see [`Block`].
#[macro_export]
macro_rules! wgsl {
    ($wgsl:literal) => {
        $crate::shaders::Block::declared(file!(), line!(), $wgsl)
    };
}

/// A block of WGSL a scene hands to a stage, and where it was written when it says.
///
/// # Why a block is a type rather than a string
///
/// A scene's WGSL is spliced into a template, and the string that reaches the device is a
/// module that exists nowhere on disk — so a parse error in it names a line no reader can
/// open. A block that knows the file and line it was written at is spliced with one comment
/// line above it saying so, and a parse error in it is reported against **that** file and line
/// rather than against the assembled string.
///
/// # Invariants
///
/// - **A block that declared no origin is spliced in unchanged, byte for byte.** The marker is
///   what a declaration buys, and a scene that writes a plain `&str` — which every scene did
///   before this type existed, and which is still accepted — gets the module it always got and
///   the error message it always got.
/// - **`spliced` is where the marker is written**, so every splice point names its block the
///   same way and no template has to know whether the block it was handed declared anything.
/// - **The origin is declared by the scene, never inferred.** The alternative — reading the
///   caller's line with `#[track_caller]` — names the line the scene *called the builder* on,
///   which is a different line from the one holding the WGSL, and is exactly the kind of
///   inferred rule that points somewhere that is not the mistake.
#[derive(Debug, Clone, Copy)]
pub struct Block<'text> {
    wgsl: &'text str,
    declared: Option<(&'static str, u32)>,
}

impl<'text> Block<'text> {
    /// Returns the block a [`wgsl!`] invocation declares.
    ///
    /// `invoked_at` is the line the macro was invoked on, and the block's **first** line of WGSL
    /// is the line after it: a literal opening on that next line puts the block's first line of
    /// text — an empty one — there, and every line of the block after it is one line on. That is
    /// the one number, and a literal opening on the invocation's own line is a line off it,
    /// because nothing in a `macro_rules!` expansion can see where the literal begins.
    ///
    /// A `const fn`, so a block is a `const` rather than a value built per scene: a block of
    /// WGSL is a constant, and a scene that wrote one out gets the same declaration it wrote
    /// before this type existed with an origin attached to it.
    pub const fn declared(file: &'static str, invoked_at: u32, wgsl: &'text str) -> Self {
        Self {
            wgsl,
            declared: Some((file, invoked_at)),
        }
    }

    /// Returns the WGSL as a module holds it: the block, and above it a marker naming where it
    /// came from — or the block alone, when nothing declared it.
    ///
    /// `name` is what the block is called at the splice point it is written into, which is
    /// where its meaning is: the same text is a `surface` in the lighting stage and a `grade`
    /// in the post one, and a reader told only "line 217" is told nothing.
    ///
    /// The marker carries the line the block's **first line of WGSL** is on, which is the line
    /// after the one [`declared`] was given: see the `wgsl!` doc for why that is one on.
    pub fn spliced(&self, name: &str) -> Cow<'text, str> {
        let Some((file, invoked_at)) = self.declared else {
            return Cow::Borrowed(self.wgsl);
        };
        Cow::Owned(graphics::provenance::mark(
            name,
            file,
            invoked_at + 1,
            self.wgsl,
        ))
    }

    /// Returns the block's own WGSL, with no marker above it.
    ///
    /// This is for the block a scene builds by composition rather than by writing out: the
    /// text of such a block is a `format!` of three others, and there is no line of any file
    /// that is the start of it, so declaring one would be a number nobody could check.
    pub fn text(&self) -> &'text str {
        self.wgsl
    }
}

impl<'text> From<&'text str> for Block<'text> {
    fn from(wgsl: &'text str) -> Self {
        Self {
            wgsl,
            declared: None,
        }
    }
}

impl<'text> From<&'text String> for Block<'text> {
    fn from(wgsl: &'text String) -> Self {
        Self::from(wgsl.as_str())
    }
}

/// The uniform the shadowing and depth stages read.
///
/// Only the projection and the time, because that is all a pass of those stages supplies.
/// The device parses this shader back with `naga` and compares its uniform struct against
/// the interface the shader declared, so declaring a field the pass does not supply is a
/// render error rather than a frame of the wrong numbers.
const PLACEMENT_UNIFORM: &str = r#"
struct Projection {
    position: vec3<f32>,
    view_projection: mat4x4<f32>,
};

struct Uniforms {
    projection: Projection,
    time: f32,
};
"#;

/// The uniform the shadowing stage reads: the placement above and the page it is drawing.
///
/// # Invariants
///
/// **The fields are in this order because the interface sorts them by name.** `projection`,
/// `time`, `view_page` is that order, and it is the order the device's own layout puts the bytes
/// in: `projection` is a nested struct of a three component vector and a matrix, so it runs to
/// eighty bytes, `time` lands at eighty, and the four component vector wants a sixteen byte
/// boundary and so lands at ninety six. [`PLACEMENT_UNIFORM`] is this with the last field absent,
/// which is why the two are written apart: a depthing pass that carried a page would be a depthing
/// pass with a page to draw into, and it has none.
const SHADOW_UNIFORM: &str = r#"
struct Projection {
    position: vec3<f32>,
    view_projection: mat4x4<f32>,
};

struct Uniforms {
    projection: Projection,
    time: f32,
    view_page: vec4<f32>,
};
"#;

/// The uniform the clustered lighting stage reads.
///
/// The fields are in the order the interface schema sorts them into, which is by name:
/// `cluster_grid`, `directional`, `environment`, `frame_size`, `global_count`, `projection`,
/// `time`. The order is not cosmetic — a three component vector is padded to sixteen bytes,
/// so a field written second starts at sixteen either way, and a struct written in any other
/// order puts the wrong thing there. The offsets are `cluster_grid` at 0, `directional` at 16
/// (ninety six bytes, so `environment` at 112), `frame_size` at 128, `global_count` at 136,
/// `projection` at 144 and `time` at 224, in a block of 240.
///
/// `global_count` is a plain `f32` and it lands in the four bytes `frame_size` already left
/// over: a two component vector is eight bytes and the next field wants sixteen, so the
/// padding was there before this field and nothing after it moves. That is a fact about the
/// names, not a coincidence to rely on — a field that sorted before `frame_size` would push
/// the whole tail of the struct along, and the device compares these offsets against the
/// layout its own interface schema computes, so a wrong one is a render error naming both.
///
/// **Nothing here names a clustered light.** The four per light fields this struct used to
/// carry — the light's position, its colour, its own projection and the size of its shadow
/// map's texel — are a field of the light's *record*, read from a storage buffer per light
/// inside the loop. That is the whole change: a uniform is sixteen kilobytes, which cannot
/// hold a thousand light records however they are packed.
///
/// The three lights it does carry are the three a fragment cannot reach any other way. The
/// first two are `environment` and `directional`: an image that casts no shadow and a
/// direction that casts one, each a colour with a fourth component saying whether the scene
/// named it, because a colour of zero is a perfectly good light to want and a shader that
/// inferred presence from the colour would be reading the wrong question.
///
/// The third is a **count and not a light**, and it is what a global point light does not
/// need a field for. A global light is in no cluster list, but unlike the other two it *is*
/// a light with a record, and the record run and the cone run beside it are the ones the
/// clustered loop already reads. The compiler writes the global lights first, so the loop
/// over `[0, global_count)` reaches them by index out of buffers that were bound anyway.
/// That is why the count is here rather than a second copy of a light: a global light that
/// cost a uniform field of its own would be a light a shader had to be written for.
const LIGHTING_UNIFORM: &str = r#"
struct Projection {
    position: vec3<f32>,
    view_projection: mat4x4<f32>,
};

/// The scene's directional light: what it is, which way it shines, and where its shadow is.
///
/// The page is the fourth component of the direction rather than a field of its own, so that
/// a fragment reads and tests it in one comparison exactly as it reads a record's own page.
struct Directional {
    colour: vec4<f32>,
    direction: vec4<f32>,
    shadow_projection: mat4x4<f32>,
};

struct Uniforms {
    cluster_grid: vec4<f32>,
    directional: Directional,
    environment: vec4<f32>,
    frame_size: vec2<f32>,
    global_count: f32,
    projection: Projection,
    time: f32,
};

/// One light, as the clustered loop reads it out of the record buffer.
///
/// # Why this is not the buffer's element type
///
/// The buffer is declared `array<f32>` and a record is read out of it by [`light_record`], which
/// writes the word offsets down. The reason is at the end of the same buffer: a light's **page
/// table** is appended to the record run, because the clustered pass has no fifth storage binding
/// to put it in, and an `array<LightRecord>` has a length the driver computes from the buffer's
/// size and stops at the last whole record — so the words past it are a different array by
/// WGSL's own arithmetic and nothing declared here could reach them. One flat run of words, read
/// through two accessors, is the only shape that reaches both halves of it.
///
/// The second reason is weaker but real: the struct-array form agreed with the compiler's packing
/// by a coincidence of alignment — a three component vector is sixteen byte aligned and twelve
/// bytes long, so the scalar after each of them was exactly the pad the next field wanted. Here
/// the offsets are said, and a record that grew would have to be edited in the one place that
/// says them.
///
/// `color` at word 0, `page` at 3, `position` at 4, `radius` at 7, the sixteen words of
/// `shadow_projection` at 8, and `virtual_page` at 24 — twenty eight words, a record of one
/// hundred and twelve bytes, a whole number of sixteen byte blocks.
struct LightRecord {
    color: vec3<f32>,
    page: f32,
    position: vec3<f32>,
    radius: f32,
    shadow_projection: mat4x4<f32>,
    virtual_page: vec4<f32>,
};

/// How many words one light record is, and where within it each field sits.
const LIGHT_RECORD_WORDS: u32 = 28u;

/// One light's cone, as the clustered loop reads it out of the cone buffer.
///
/// # Why this is two vectors of four and not a direction and two angles
///
/// A `WGSL` `array<T>` in a storage buffer lays its elements out at
/// `roundUp(align_of_T, size_of_T)`, and a struct holding a vector is aligned to sixteen
/// bytes. A struct of a `vec3` and two `f32` is twenty bytes packed and **thirty two** to a
/// shader, so a shader reading the compiler's tightly packed run would read the second light's
/// direction as the first light's outer angle. Thirty two bytes is the smallest record that is
/// a whole number of sixteen byte blocks, and these two vectors are thirty two — which is the
/// same argument the light record above makes about its own ninety six.
struct SpotCone {
    axis: vec4<f32>,
    limits: vec4<f32>,
};
"#;

/// The emission stage: what a surface emits, added once rather than once per light.
///
/// # Why this is a stage of its own
///
/// The lighting stage used to run **once per light** and its passes added, so any term a
/// surface body added there was added once per light: an emissive surface was twice as bright
/// under two lights as under one, which is not what emitting means. It runs once now, so a
/// term added to the lighting stage is added once as well. The stage still exists, because
/// what a surface emits is *not lit*: a lamp is as bright under three lights as under one,
/// and the lighting stage's own ambient term is not that.
///
/// `emission` is the emitted colour, written against the varyings below and reading the
/// material's `custom_data` through them. It is a separate argument for the same reason the
/// lighting stage's `surface` is: what a surface emits is the scene's business and the fact
/// that it is added once is not.
///
/// `helpers` is module level `WGSL` the body needs — a sky, a mist, a noise — for the same
/// reason the lighting stage takes one. **A stage whose body cannot reach a shared function
/// cannot use one**, and a surface that emits the reflection of the same sky the frame's
/// background is drawn from has no other way to say that the two are one sky.
///
/// # Why this stage declares no resource beyond its uniform
///
/// The pass renders into the colour target and is tested against the depth buffer, so both
/// are its **targets** rather than its bindings: a backend forbids a pass to read what it
/// draws into, and a depth test is a pipeline state rather than a read a fragment performs.
/// The two points still take the binding numbers their names sort them to — that is the
/// device's rule and it applies to a point whatever the pass does with it — so the template
/// declares the uniform alone and leaves those numbers unread.
pub fn emission<'text>(
    vertex_inputs: &str,
    emission: impl Into<Block<'text>>,
    helpers: impl Into<Block<'text>>,
) -> String {
    let emission = emission.into();
    let helpers = helpers.into();
    let emission = emission.spliced("emission");
    let helpers = helpers.spliced("helpers");
    format!(
        r#"{PLACEMENT_UNIFORM}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

{helpers}
struct Vertex {{
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) texture: vec2<f32>,
    @location(3) emissive: vec3<f32>,
    @location(4) tint: vec3<f32>,
    @location(5) material: vec3<f32>,
}};

@vertex
fn vertex_main(
{vertex_inputs}) -> Vertex {{
    let model = mat4x4<f32>(model0, model1, model2, model3);
    let normals = mat4x4<f32>(normal_matrix0, normal_matrix1, normal_matrix2, normal_matrix3);
    let world = model * vec4<f32>(position, 1.0);
    var out: Vertex;
    out.position = uniforms.projection.view_projection * world;
    out.world_position = world.xyz;
    out.normal = normalize((normals * vec4<f32>(normal, 0.0)).xyz);
    out.texture = texture;
    out.emissive = emissive;
    out.tint = tint;
    out.material = vec3<f32>(metallic, roughness, reflect);
    return out;
}}

@fragment
fn fragment_main(in: Vertex) -> @location(0) vec4<f32> {{
    // What the surface emits, before anything is added to it. A body that writes nothing emits
    // the material's own `emissive`, which is what a scene that names an emission shader and
    // writes no body for it means.
    var emitted = in.emissive;
{emission}
    // An emission pass adds, so a surface that emits nothing leaves the frame it was drawn
    // over exactly as it found it.
    return vec4<f32>(emitted, 1.0);
}}
"#
    )
}

/// The shadowing stage: the point is placed, and the fragment writes depth through the
/// target itself, so the stage declares no fragment entry point.
///
/// `vertex_inputs` is the attribute list [`crate::attributes`] writes out from the
/// two schemas, rather than a transcription of it.
///
/// # Why the vertex stage remaps before it returns
///
/// A shadowing pass draws **one page** of a light's view, and the light's projection answers for
/// the whole of it. The pass's `viewport` says which rectangle of the atlas the page occupies,
/// which moves the whole cone's worth of geometry into that rectangle and leaves it no finer
/// there; what makes a page a *page* is the rectangle of the light's plane it records, and only a
/// multiply can say that. So `view_page` scales about the page's own centre in the light's normal
/// device coordinates, which puts the page's rectangle over the whole of the viewport's, and the
/// rasteriser's divide by `w` lands on the same result as a divide first and a remap after —
/// because the remap is affine in `x` and `y` and touches neither `z` nor `w`.
///
/// A light drawn into **one pool page** passes the whole plane, `[-1, -1, 2, 2]`, whose centre is
/// the origin and whose half extent is one, which is to say this multiply is the identity and the
/// pass places its points exactly as it did before the field existed. That is the reason one
/// template serves both shapes of pass rather than two.
pub fn shadowing(vertex_inputs: &str) -> String {
    format!(
        r#"{SHADOW_UNIFORM}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertex_main(
{vertex_inputs}) -> @builtin(position) vec4<f32> {{
    let model = mat4x4<f32>(model0, model1, model2, model3);
    let placed = uniforms.projection.view_projection * model * vec4<f32>(position, 1.0);
    // The page's own rectangle of the light's plane, stretched to fill the whole viewport. The
    // depth is the light's own and does not move: two pages of one light disagree about where a
    // point is in the plane and agree about how far it is.
    let centre = uniforms.view_page.xy + uniforms.view_page.zw * 0.5;
    let spread = vec2<f32>(2.0) / uniforms.view_page.zw;
    return vec4<f32>((placed.xy - centre * placed.w) * spread, placed.z, placed.w);
}}

@fragment
fn fragment_main() -> @location(0) vec4<f32> {{
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}}
"#
    )
}

/// The depth stage: the same placement, and no fragment stage at all.
///
/// `vertex_inputs` is the attribute list [`crate::attributes`] writes out, and it is
/// the same list the shadowing stage was given: a stage that places a point differently
/// places it differently, and nothing downstream can tell by how much.
pub fn depthing(vertex_inputs: &str) -> String {
    format!(
        r#"{PLACEMENT_UNIFORM}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertex_main(
{vertex_inputs}) -> @builtin(position) vec4<f32> {{
    let model = mat4x4<f32>(model0, model1, model2, model3);
    return uniforms.projection.view_projection * model * vec4<f32>(position, 1.0);
}}
"#
    )
}

/// The lighting stage, which shades by what the earlier passes wrote.
///
/// `helpers` is WGSL a scene needs beyond the shading itself — a noise, a curve — written
/// as functions. `WGSL` has no nested functions, so a scene's helpers cannot go in
/// `surface`, which is written into the body of one; they are declared beside it here.
///
/// `surface` is the shading of a surface, written against the varyings below. It is a
/// separate argument because what a surface looks like is the scene's business and how it
/// is lit is not: the shadow term, the falloff and the Fresnel are the same for every
/// surface, and a scene that rewrote them would get one of them subtly wrong.
///
/// `vertex_inputs` is the attribute list [`crate::attributes`] writes out from the
/// two schemas the stages and the mesh all declare, so a scene that changes its point schema
/// gets the attributes for it without writing them out.
pub fn lighting<'text>(
    vertex_inputs: &str,
    surface: impl Into<Block<'text>>,
    helpers: impl Into<Block<'text>>,
) -> String {
    lighting_with_normal(vertex_inputs, "", surface, helpers)
}

/// The lighting stage of a scene that displaces the normal it is lit by.
///
/// # Why the normal is displaced before the shading and not after
///
/// Everything the fragment stage computes — the Lambert term, the specular lobe, the Fresnel
/// term, the offset the shadow lookup is taken at — is a function of the shading normal, so a
/// body spliced after them can only add to a colour it did not get to influence. A water
/// surface with waves in it, a leaf that turns, a brushed metal and a coat of cloth are all
/// unreachable for that one reason: the normal arrives, is used, and is never offered again.
///
/// `displace` runs where the normal arrives and before anything reads it, and may reassign
/// `normal` — declared `var` for exactly that. An empty body changes nothing, so
/// [`lighting`] is this with an empty `displace`.
pub fn lighting_with_normal<'text>(
    vertex_inputs: &str,
    displace: impl Into<Block<'text>>,
    surface: impl Into<Block<'text>>,
    helpers: impl Into<Block<'text>>,
) -> String {
    lighting_stage(
        vertex_inputs,
        displace.into(),
        surface.into(),
        helpers.into(),
        lighting_bindings(false, &Extras::NONE.binding_points()),
        &Extras::NONE,
    )
}

/// Returns a lighting stage that writes the frame's extra outputs beside it.
///
/// # Invariants
///
/// **`Extras::NONE` is this with nothing declared**, and produces the same source the other
/// builder does — the same six varyings, the same assignments, and a bare `@location(0)
/// vec4<f32>` rather than a struct. Every gallery scene that declares no outputs therefore
/// compiles to the WGSL it compiled to before, which is what makes the additive property a
/// measurement: a scene's picture does not move because its source did not either.
pub fn lighting_declaring<'text>(
    vertex_inputs: &str,
    displace: impl Into<Block<'text>>,
    surface: impl Into<Block<'text>>,
    helpers: impl Into<Block<'text>>,
    extras: &Extras,
) -> String {
    lighting_stage(
        vertex_inputs,
        displace.into(),
        surface.into(),
        helpers.into(),
        lighting_bindings(false, &extras.binding_points()),
        extras,
    )
}

/// Where one declared output's value comes from.
///
/// # Invariants
///
/// **These are the only two ways a fragment can know a value**, and choosing between them is a
/// fact about the geometry rather than about the shader. A surface that *is* an authored element
/// carries its identity in its points, because two faces of one flank are two different answers
/// and only the geometry knows which. A surface that is *not* one — the plain a massif stands on,
/// the sky, a frame the scene put there for its own reasons — writes a constant, because there is
/// nothing per-vertex about it to read.
#[derive(Debug, Clone, PartialEq)]
pub enum Authored {
    /// Read from a point field, which the point schema then has to carry.
    Field {
        /// The point field, in the schema's own naming.
        name: String,
        /// How many components it is, which decides how the shader reads it.
        components: u32,
    },
    /// A constant, for geometry that is not an authored element.
    Constant(f64),
}

/// The extra fragment outputs a scene's own lighting shader writes beside the frame.
///
/// # Invariants
///
/// - **A scene that declares none gets `NONE`, and `NONE` reproduces the shader that existed
///   before this type did**: the same six varyings, the same assignments, and a fragment that
///   returns `@location(0) vec4<f32>` rather than a struct. Every other gallery scene's WGSL is
///   therefore the WGSL it was, which is what makes the additive property measurable rather than
///   argued — the file's sha256 does not move.
/// - **The names are the scene's own, taken from its `outputs`, in the declared locations'
///   order.** The varying, the vertex input and the fragment output all carry the same name, so a
///   shader that names one and not the other is a shader `naga` refuses rather than a picture
///   that is quietly wrong.
/// - **A fragment that writes more than the frame returns a struct**, and a fragment that writes
///   only the frame returns a bare `vec4<f32>`. Both are legal WGSL and both are checked the
///   same way by `check_outputs`; the struct is not a nicer shape, it is the only way a fragment
///   stage can name a second output at all.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Extras {
    /// The scene's declared outputs, in location order.
    outputs: Vec<Extra>,
}

/// One declared output and where its value comes from.
#[derive(Debug, Clone, PartialEq)]
struct Extra {
    name: String,
    location: u32,
    source: Authored,
}

/// One output's point field, borrowed from the [`Authored`] that named it.
///
/// It is its own type because the four places that read it — the schema, the varying, the
/// assignment and the read — all want the field's name and its width and neither wants to know
/// which of the two halves of [`Authored`] it came from.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PointField<'text> {
    field: &'text str,
    components: u32,
}

impl Extras {
    /// What a scene that declares no outputs gets.
    pub const NONE: Self = Self {
        outputs: Vec::new(),
    };

    /// Returns the outputs a scene declares, each paired with where its value comes from.
    ///
    /// # Invariants
    ///
    /// **The list is the scene's own `outputs` map in the declared locations' order, and it is
    /// the only ordering authority.** A varying numbered by one list and a fragment output
    /// numbered by the other would be a shader whose two halves describe different pictures, and
    /// the failure would be an image of somebody else's numbers rather than an error.
    pub fn new(entries: impl IntoIterator<Item = (String, u32, Authored)>) -> Self {
        Self {
            outputs: entries
                .into_iter()
                .map(|(name, location, source)| Extra {
                    name,
                    location,
                    source,
                })
                .collect(),
        }
    }

    /// Returns the binding points a fragment that writes these outputs has in its pass.
    ///
    /// # Invariants
    ///
    /// **The names come from [`scene::standard::output_point`], which is the same function the
    /// compiler creates the instance entry with.** A target is a binder the pass has and the
    /// shader reads nothing, exactly as `color_target` is, so a shader's interface has to
    /// declare it or the two halves of the document disagree — and the point name written in two
    /// places would be two things that can differ.
    pub fn binding_points(&self) -> Vec<String> {
        self.outputs
            .iter()
            .map(|extra| scene::standard::output_point(&extra.name))
            .collect()
    }

    /// Returns whether this shader writes anything beside the frame.
    pub fn is_empty(&self) -> bool {
        self.outputs.is_empty()
    }

    /// Returns the point schema a shader writing these outputs declares.
    ///
    /// # Invariants
    ///
    /// **The gallery's own point, with each output's field added to it.** A scene whose mesh
    /// carries an id per face needs a point schema with that field in it, and deriving both the
    /// layout and the shader's parameter list from the one schema is what makes them agree —
    /// nothing checks a vertex layout, so a shader naming a location the layout does not have
    /// reads somebody else's data rather than failing.
    pub fn point_schema(&self) -> Schema {
        let mut fields = match &kit::shaded_point_schema().kind {
            SchemaKind::Struct { fields } => fields.clone(),
            other => return Schema::new(other.clone()),
        };
        for extra in self.read_from_points() {
            fields.insert(
                extra.field.to_owned(),
                graphics::layout::components_schema(extra.components as usize),
            );
        }
        Schema::new(SchemaKind::Struct { fields })
    }

    /// Returns the outputs whose value is read from the points rather than written as a constant.
    ///
    /// # Invariants
    ///
    /// **A constant adds no field to the point schema, no attribute to the layout and no varying
    /// to the vertex stage.** That is what lets a scene whose second surface is a plain — which is
    /// most of them — declare the same outputs as the one whose surface is authored, without the
    /// plain's geometry carrying three numbers that are the same at every vertex.
    fn read_from_points(&self) -> Vec<PointField<'_>> {
        self.outputs
            .iter()
            .filter_map(|extra| match &extra.source {
                Authored::Field { name, components } => Some(PointField {
                    field: name.as_str(),
                    components: *components,
                }),
                Authored::Constant(_) => None,
            })
            .collect()
    }

    /// Returns the varyings the vertex stage carries and the fragment stage reads.
    ///
    /// The locations **continue from the shader's own six**, because those are the six the
    /// `Vertex` struct declares whatever this type is: a shader with outputs has the same
    /// world position, normal, texture, emissive, tint and material as one without.
    ///
    /// # Invariants
    ///
    /// **These are the varying namespace's numbers, not the scene's declared ones.** A varying's
    /// `@location` addresses an entry of the `Vertex` struct and has to be unique and unclaimed
    /// within it; a fragment output's addresses a colour attachment and is the number the scene
    /// declared. The two are different spaces and happen to run in the same order, so deriving
    /// one from the other would be right until a scene declared its locations out of order — at
    /// which point only one of the two is still what the compiler built.
    fn varyings(&self) -> String {
        let mut lines = String::new();
        for (index, field) in self.read_from_points().iter().enumerate() {
            lines.push_str(&format!(
                "    @location({}) {}: f32,\n",
                FIRST_EXTRA_LOCATION + index as u32,
                field.field
            ));
        }
        lines
    }

    /// Returns the assignments that carry each output's point field to its varying.
    fn assigns(&self) -> String {
        let mut lines = String::new();
        for field in self.read_from_points() {
            lines.push_str(&format!(
                "    out.{} = {};\n",
                field.field,
                Self::read(&field)
            ));
        }
        lines
    }

    /// Returns how one point field is read, given its component count.
    ///
    /// # Invariants
    ///
    /// **A field of one component is a bare `f32` and of more is assembled from consecutive
    /// attributes**, which is the same rule `crate::attributes` lays the vertex out by. Reading a
    /// one component field as a `vec3` would consume the two attributes after it and shift every
    /// one of them, which is a vertex reading somebody else's data rather than an error.
    fn read(field: &PointField<'_>) -> String {
        if field.components == 1 {
            field.field.to_owned()
        } else {
            let parts: Vec<String> = (0..field.components)
                .map(|component| format!("{}{component}", field.field))
                .collect();
            format!("vec{}<f32>({})", field.components, parts.join(", "))
        }
    }

    /// Returns the struct a fragment returns when it writes more than the frame, and the type
    /// the entry point is declared as.
    ///
    /// # Invariants
    ///
    /// **Each output is written at the `@location` the scene declared, never at its position in
    /// this list.** The compiler builds the pass's colour targets in the declared locations'
    /// order, so a declared location of two is the second attachment; deriving the location from
    /// the index would be right for every scene that happens to declare 1, 2, 3 in order and wrong
    /// for the first that does not — which is the failure `typed-outputs.md` records from an
    /// inferred rule that broke six gallery scenes at once.
    fn returns(&self) -> (String, String) {
        if self.is_empty() {
            return (String::new(), "@location(0) vec4<f32>".to_owned());
        }
        let mut declarations =
            String::from("struct Written {\n    @location(0) frame: vec4<f32>,\n");
        for extra in &self.outputs {
            declarations.push_str(&format!(
                "    @location({}) {}: f32,\n",
                extra.location, extra.name
            ));
        }
        declarations.push_str("};\n\n");
        (declarations, "Written".to_owned())
    }

    /// Returns the frame and every declared output, written into the struct a fragment returns.
    ///
    /// # Invariants
    ///
    /// **An output read from a point takes the varying the vertex stage carried it in, and an
    /// output written as a constant writes the constant.** Those are the two halves of
    /// [`Authored`], and the fragment reads the field's own name because the varying *is* that
    /// name: a fragment that mixed the two cases up would name a varying nothing writes, which
    /// `naga` rejects rather than draws — so both are written from the one value that decided
    /// which case they are, never from a name that could be either.
    fn writes(&self) -> String {
        if self.is_empty() {
            return String::new();
        }
        let mut lines =
            String::from("    var out: Written;\n    out.frame = vec4<f32>(color, 1.0);\n");
        for extra in &self.outputs {
            let value = match &extra.source {
                Authored::Constant(number) => format!("{number}"),
                Authored::Field { name, .. } => format!("in.{name}"),
            };
            lines.push_str(&format!("    out.{} = {};\n", extra.name, value));
        }
        lines
    }

    /// Returns what the fragment entry point returns, which is the struct when there is one.
    fn return_value(&self) -> String {
        if self.is_empty() {
            "vec4<f32>(color, 1.0)".to_owned()
        } else {
            "out".to_owned()
        }
    }
}

/// The `@location` a scene's first own output's varying takes, which continues the six every
/// lighting shader's `Vertex` struct declares.
const FIRST_EXTRA_LOCATION: u32 = 6;

/// Returns the binding declarations of the clustered lighting stage, numbered the way the
/// pass binds them.
///
/// # Invariants
///
/// The uniform is binding zero and the rest follow in the pass's **name order**, so the list
/// below is sorted by the binding point's name and the numbers are what that order works out
/// to. A number written in a shader and a number worked out by the device agreeing is the
/// only thing keeping this correct, and `check_bindings` reports a disagreement by name — but
/// it can only report one if there is a number to report, so the numbers are stated here once
/// rather than spread through the template.
///
/// The order is over the pass's binding **points**, not over the resources it binds, because
/// that is the rule the device numbers by: a point the pass renders into takes a number and
/// emits no binding entry. So `color_target` and `depth_buffer` are both in the list and
/// declare nothing. The pass renders into the colour target and a fragment is tested against
/// the depth buffer, and the fragment reads neither, so each takes the number its own name
/// sorts it to and no line of WGSL. Leaving a number out instead would put every binding after
/// it one lower than the device's, which is a frame of the wrong numbers rather than a
/// failure — and it is the mistake that left the lighting pass without a depth test at all,
/// because moving the depth buffer out of the bound resources and into the targets moved
/// `environment_map` into the slot the template had reserved for it.
///
/// An image is a texture **and** the sampler that reads it, at the position immediately after
/// its own, so adding one shifts everything below it. That is why the declarations are
/// generated from a flag rather than spliced into the source with a string replacement.
fn lighting_bindings(image: bool, outputs: &[String]) -> String {
    // Each entry is the access an address space needs written on `var` itself, and the type
    // that follows the name. A storage buffer is the one that needs the first, and needing
    // it is why a plain `var` is not enough for these three. **An empty type takes a number
    // and declares nothing** — that is what a target is.
    let mut resources: Vec<(&str, &str, &str)> = vec![
        ("cluster_light_counts", "<storage, read>", "array<f32>"),
        ("cluster_light_indices", "<storage, read>", "array<f32>"),
        ("color_target", "", ""),
        ("depth_buffer", "", ""),
    ];
    // The environment map is bound by every lighting pass, including one whose scene names no
    // `Environment` light — the pass then binds a one texel black map and the uniform's flag
    // is zero. It is bound unconditionally for the same reason the list is in name order: a
    // binding that came and went with the document would move every number after it, so a
    // shader would have to be written for one scene and not for another.
    resources.push(("environment_map", "", "texture_2d<f32>"));
    if image {
        resources.push(("image", "", "texture_2d<f32>"));
        resources.push(("image_sampler", "", "sampler"));
    }
    resources.push(("light_records", "<storage, read>", "array<f32>"));
    // A scene's own declared outputs are **targets the shader writes and never reads**, so they
    // take a number and emit no declaration — exactly what `color_target` above is.
    for point in outputs {
        resources.push((point.as_str(), "", ""));
    }
    // The atlas is bound by every lighting pass and read by only the fragments of a light that
    // was given a virtual grid, for the same reason the cone buffer below is: a binding point is
    // numbered by its name's position, so one that came and went with the document would move
    // `shadow_pool` and `spot_cones` off the numbers the shader declares them at.
    resources.push(("shadow_atlas", "", "texture_depth_2d_array"));
    resources.push(("shadow_pool", "", "texture_depth_2d_array"));
    // The cone buffer is bound by every lighting pass, including one whose scene names no
    // `Spot` light — the pass then binds a single whole sphere that no cluster points at.
    // Bound unconditionally for the same reason the environment map is above: a binding that
    // came and went with the document would move every number after it, so a shader would
    // have to be written for one scene and not for another. A scene with no spot light reads
    // nothing, so the bandwidth is zero and the only cost is a bound buffer.
    resources.push(("spot_cones", "<storage, read>", "array<SpotCone>"));

    // **Sorted by name, and the numbers read off the sort.** The device numbers a pass's binding
    // points by their names' order, so a list written in name order by hand is a number that has
    // to be maintained by editing it in the right place — and a scene's declared outputs sort
    // between `light_records` and `shadow_atlas`, so appending them where they were written
    // would put `shadow_atlas` one lower than the pass built it. That is a frame of the wrong
    // numbers rather than a failure, and it is the mistake that left the lighting pass without a
    // depth test at all. Sorting makes the number a function of the **name**, which is the thing
    // the device actually sorts by, and the list above no longer has to be kept in order.
    resources.sort_by(|left, right| left.0.cmp(right.0));

    let mut declarations = String::new();
    for (number, (source, access, ty)) in resources.iter().enumerate() {
        if !ty.is_empty() {
            declarations.push_str(&format!(
                "@group(0) @binding({}) var{access} {source}: {ty};\n",
                number + 1
            ));
        }
    }
    declarations
}

/// The lighting stage of the scene, with the bindings it draws with.
///
/// # Why the loop and not a pass per light
///
/// A pass is a pipeline, a bind group and a draw, and there is one uniform per pass. A uniform
/// is sixteen kilobytes under the limits this renderer requests, so a scene could hold a few
/// dozen lights and no more, however large the schema said its light was. A **storage buffer**
/// is not bounded that way, so the lights move into one and the pass reads the ones its own
/// fragment needs — which is a handful, because the cluster lists are what the culling was
/// for.
fn lighting_stage<'text>(
    vertex_inputs: &str,
    displace: Block<'text>,
    surface: Block<'text>,
    helpers: Block<'text>,
    bindings: String,
    extras: &Extras,
) -> String {
    let displace = displace.spliced("displace");
    let surface = surface.spliced("surface");
    let helpers = helpers.spliced("helpers");
    let vertex_varyings = extras.varyings();
    let vertex_assigns = extras.assigns();
    let (fragment_returns, fragment_type) = extras.returns();
    let fragment_writes = extras.writes();
    let fragment_return = extras.return_value();
    format!(
        r#"{LIGHTING_UNIFORM}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
{bindings}
/// Returns one light's record, read out of the flat run the page table shares a buffer with.
///
/// The offsets are written here rather than left to a struct declaration because the run is
/// **tightly packed four bytes per component**, and this is the one place that can say so: an
/// `array<LightRecord>` would be laid out by WGSL's own alignment rules and be right only while
/// the compiler's packing happened to need no padding that WGSL did not also want.
fn light_record(index: u32) -> LightRecord {{
    let at = index * LIGHT_RECORD_WORDS;
    let m = at + 8u;
    return LightRecord(
        vec3<f32>(light_records[at], light_records[at + 1u], light_records[at + 2u]),
        light_records[at + 3u],
        vec3<f32>(light_records[at + 4u], light_records[at + 5u], light_records[at + 6u]),
        light_records[at + 7u],
        mat4x4<f32>(
            vec4<f32>(light_records[m], light_records[m + 1u], light_records[m + 2u], light_records[m + 3u]),
            vec4<f32>(light_records[m + 4u], light_records[m + 5u], light_records[m + 6u], light_records[m + 7u]),
            vec4<f32>(light_records[m + 8u], light_records[m + 9u], light_records[m + 10u], light_records[m + 11u]),
            vec4<f32>(light_records[m + 12u], light_records[m + 13u], light_records[m + 14u], light_records[m + 15u]),
        ),
        vec4<f32>(
            light_records[at + 24u],
            light_records[at + 25u],
            light_records[at + 26u],
            light_records[at + 27u],
        ),
    );
}}

/// Returns which cluster of the grid a fragment is in.
///
/// # Why this costs three multiplies
///
/// The grid is divided evenly across the frame and evenly through the view's **depth**, and
/// those are the two divisions a fragment can index for free: its own position carries the
/// pixel it covers and the depth it was drawn at. Recovering a *distance* would need the near
/// and far planes and a divide, and WGSL has no matrix inverse to unproject with; the depth
/// is the number the compiler's own grid was measured in, because a projection puts the near
/// plane at zero and the far plane at one — which is the convention a framebuffer's depth
/// runs in, so the two agree by construction rather than by agreement.
///
/// Rows are counted from the **bottom** of the frame. A framebuffer row runs down the screen
/// and a grid row runs up the view, so counting one from the other would turn the image
/// upside down rather than merely misplace a shadow.
fn cluster_of(fragment: vec4<f32>) -> u32 {{
    // Clamped rather than truncated at one: a fragment on the last row or the last column
    // would otherwise index a cluster past the end of the grid, and the compiler's own
    // `slice_at` clamps for the same reason at the far plane.
    let across = clamp(fragment.x / uniforms.frame_size.x, 0.0, 0.99999);
    let up = clamp(1.0 - fragment.y / uniforms.frame_size.y, 0.0, 0.99999);
    let column = u32(across * uniforms.cluster_grid.x);
    let row = u32(up * uniforms.cluster_grid.y);
    let slice = u32(clamp(fragment.z, 0.0, 0.99999) * uniforms.cluster_grid.z);
    // Depth slices count fastest, then rows, then columns. The two cluster buffers are
    // written in that order, so a fragment that knows its own three indices needs no
    // arithmetic beyond these two products.
    return slice * u32(uniforms.cluster_grid.x * uniforms.cluster_grid.y)
        + row * u32(uniforms.cluster_grid.x)
        + column;
}}

/// Returns how lit a surface is by one light.
///
/// # Why the inputs rather than a record
///
/// A **directional** light is not in any cluster list, so no fragment ever loads a record of
/// it and a function taking one could not be called for it. The shadow lookup itself is the
/// same algorithm for both — one projection, one page, one comparison — so what it takes is
/// the three numbers that differ: which matrix to look the page up through, which page, and
/// which way the light is coming from. The record is how a clustered light supplies those, and
/// the uniform is how the exempt one does.
///
/// A light the pool gave **no page** is lit unshadowed, and that is the design rather than a
/// degraded mode: the pool is a scene's decision about how many shadows it wants to render
/// once, a light that lost the ranking is a light the scene chose to approximate, and the
/// alternative — giving it a neighbour's page — would be a wrong shadow rather than a missing
/// one. The page is minus one for exactly that case, and a page index of zero is a page, so
/// the two can be told apart by a single comparison.
///
/// Where there is a page, the surface's world position is carried through **the light's own**
/// projection and its depth there is compared against what that page recorded. A projection is
/// per light because a point light looks at the scene from where it is and a directional one
/// looks at it along its direction, and reading the page at the fragment's own screen
/// coordinate would be a different lookup and not a shadow.
///
/// The taps are taken by `textureLoad` rather than through a sampler because a binding
/// carries a texture and nothing else: there is no sampler to filter with, and a depth
/// compare has to be done by hand in any case.
fn shadowed(
    shadow_projection: mat4x4<f32>,
    page_index: f32,
    to_light: vec3<f32>,
    world_position: vec3<f32>,
    normal: vec3<f32>,
) -> f32 {{
    let page = i32(page_index);
    if page < 0 {{
        return 1.0;
    }}
    // The page's own size, not one carried in the uniform: the pool is a schema field, so a
    // scene that changed the page size changed this with it and there is nothing to keep in
    // step by hand. It is read before the push because the push is measured in these texels.
    let size = vec2<f32>(textureDimensions(shadow_pool));
    // A point behind the light has no shadow, and dividing by a negative w would fold it
    // back into the page from behind. This is the **perspective** case alone: a page built as
    // `orthographic × look_at` has a bottom row of exactly `(0, 0, 0, 1)`, so every point
    // keeps a `w` of one and this never fires. A point light and a directional one are both
    // drawn that way; only a spot light's cone gives its page a field of view, and only that
    // one takes this branch.
    //
    // The surface is projected once unpushed to measure the page's texel, and again pushed to
    // read through it. A single projection cannot do both, because the width of a texel is
    // itself a function of where on the page the point lands.
    let measured = shadow_projection * vec4<f32>(world_position, 1.0);
    if measured.w <= 0.0 {{
        return 1.0;
    }}
    let measured_ndc = measured.xyz / measured.w;
    // A surface is pushed away from the page by one texel **of that page**, and a world constant
    // cannot say how far that is: this pool's page is a fixed number of texels across a light
    // plane this size, and 1.6 units against it is 126 to 166 texels — not a bias but a walk
    // into a different part of the scene, after which the page answers about whatever stands
    // there rather than about the surface that asked.
    //
    // Displacing along the normal rather than along the light is what keeps the shadow attached:
    // a surface is pushed away from the map by the same amount everywhere, so a slope and a flat
    // both move by one page texel and neither detaches. How wide one texel is comes from the
    // light's own projection, measured along whichever world axis lies further across the light's
    // view — a step along the normal of a surface standing square to the lamp does not move in
    // the plane at all, and the reciprocal of no movement is not a distance. Where the page is
    // not square the **larger** of its two texel counts is the one divided by, so the push is
    // the finer of the two texels and never the coarser.
    let axis = select(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, 0.0, 1.0),
        abs(to_light.x) >= abs(to_light.z),
    );
    let across_world = normalize(cross(to_light, axis));
    let span = shadow_projection * vec4<f32>(across_world, 0.0);
    let per_unit = length((span.xy - measured_ndc.xy * span.w) / measured.w);
    let texel_world = 2.0 / (max(size.x, size.y) * max(per_unit, 1e-6));
    let from_light = shadow_projection
        * vec4<f32>(world_position + normal * (texel_world * PUSH_TEXELS), 1.0);
    if from_light.w <= 0.0 {{
        return 1.0;
    }}
    let ndc = from_light.xyz / from_light.w;
    // Clip space y runs up and texture space runs down.
    let uv = ndc.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) {{
        return 1.0;
    }}
    // A surface is its own depth and the recorded value is the depth of whatever was
    // nearest the light, so the comparison is biased. The bias scales with how obliquely
    // the light strikes the surface, because a surface's own rasterised faces disagree
    // with its true depth by more the further they lean away from the light, and one
    // constant is either too small for the leaning surfaces or detaches the shadow from
    // the flat ones. The normal offset above has already taken the large part of that
    // error, so what is left here is small.
    //
    // `to_light` is already the unit vector from the surface towards the light, which is
    // what both kinds supply: the loop negates the light's direction, and the directional
    // light negates the one it travels along.
    let slope = 1.0 - abs(dot(normal, to_light));
    let z = ndc.z - (0.0006 + 0.003 * slope);
    var lit = 0.0;
    for (var row = -1; row <= 1; row = row + 1) {{
        for (var column = -1; column <= 1; column = column + 1) {{
            // The offset is added as whole texels *after* the conversion, so the kernel
            // really is three by three. Adding it before, in units of a fraction of a texel,
            // would be swallowed by the truncation and two thirds of the taps would land on
            // the texel the centre tap already read.
            //
            // The last argument is the **mip level**, not the page: a depth texture's
            // `textureLoad` takes one, and a pool is a single level because it is rendered
            // into rather than uploaded, so zero is the only level there is. The page is the
            // argument before it.
            let coordinate = vec2<i32>(uv * size) + vec2<i32>(column, row);
            lit = lit + select(0.0, 1.0, z <= textureLoad(shadow_pool, coordinate, page, 0));
        }}
    }}
    return lit / 9.0;
}}

/// The side of one virtual shadow page, in texels.
///
/// It is written out here, rather than read from the texture, because a page's size is the one
/// fact the atlas's own dimensions do not carry: an atlas layer is `atlas_side` pages across, and
/// dividing its side by that would give the answer only for the pages that were allocated, which
/// is the question and not the constant. The same number is `PAGE_SIZE` in
/// `crates/scene/src/vshadow.rs`, and the walk below is that module's `Allocation::resolve`
/// translated; `crates/scene/tests/virtual_shadow_maps.rs` is where the two are checked against
/// each other.
const PAGE_TEXELS: u32 = 128u;

/// How many levels a light's table carries offsets for, which is the length of the second offset
/// array in it.
///
/// **This is a constant of the layout and not a number the record carries.** The offsets are read
/// before the walk knows how many levels this light has, and a light with fewer than the maximum
/// simply has offsets it never reads. It is `MAX_LEVELS` in `crates/scene/src/vshadow.rs`, and
/// `crates/scene/tests/spot_lights.rs` holds the two against each other.
const LEVELS_TABLE: u32 = 4u;

/// How many of the light's own plane texels a receiver is read off its own surface by.
///
/// One, because a texel is the smallest thing the recording can say: a point read a whole texel
/// off its own surface is not in the neighbourhood that recorded it, and a point read a
/// *hundred* texels off it — which is what a world constant is, on a grid this fine — is in a
/// different part of the scene entirely.
const PUSH_TEXELS: f32 = 1.0;

/// Returns how lit a surface is by a light whose shadow is recorded as **pages** rather than as
/// one pool image.
///
/// # What the walk is
///
/// The light's projection puts the surface somewhere in the light's normalised plane, and the
/// plane is a grid of pages whose finest side the compiler worked out from what its casters asked
/// for. A caster is drawn into **the one level its own density buys** and into no other, so a level
/// holds only the casters that asked for that level and no page answers about anything else. The
/// chain is therefore walked from the finest level down and **stops at the first level that shadows
/// this point**: a level that does not shadow it, whether because it has no page or because its
/// page holds no caster covering this point, is passed through rather than answered from.
///
/// # Why passing a level through is not the same as answering from it
///
/// A page a neighbour's footprint opened is **empty** where a coarser caster's shadow falls, and an
/// empty page reads as lit. Answering there — which is what stopping at the first page that exists
/// does — loses the coarser caster's shadow entirely, and the walk would never reach the level it
/// was actually drawn into. Reading through to it instead is what makes a caster's single level
/// enough. Neither half survives alone: a caster in every level from its own down would make the
/// first page answer, and a caster in one level needs a walk that keeps going past an empty one.
///
/// It is one mask test, one popcount and at most nine taps per level, four levels at most, and
/// the reason a table exists rather than a search. A fragment in shadow costs one level; a
/// fragment in none of them costs all of them, because only a shadow breaks the walk.
///
/// A page's slot is not its number in the grid but its rank among the pages of its own row, which
/// is what lets the popcount be the answer and the atlas be filled by whatever order the compiler
/// allocated pages in.
///
/// # Why a point with no page is lit rather than shadowed
///
/// The same rule the pool path holds to: no answer is not an answer of "blocked". A point outside
/// the light's plane, and a point whose every level came up empty, are both outside this light's
/// shadow, and the alternative is a dark region in the shape of a grid gap.
///
/// # Why the bias scales with the level and the taps do not
///
/// A page of level `k` covers `2^k` times as much of the world as a page of level zero in the same
/// number of texels, so the depth its geometry disagrees with the receiver by is `2^k` times as
/// large, and a constant bias is either nothing at the coarsest level or a detached shadow at the
/// finest. The kernel stays three by three: its taps are whole texels **of the page**, so a coarse
/// page's shadow edge is soft over as much world as a coarse page covers, which is the softness
/// the level was chosen to be able to afford.
///
/// The taps are clamped to their own page, which a pool page does not need and a slot in an atlas
/// does: the texel outside a page belongs to a different rectangle of the same light's plane, and
/// comparing a depth recorded there is a wrong answer rather than an absent one.
///
/// # Why the receiver is read one texel off its own surface
///
/// A surface is recorded in the page **as itself**, so a point compared against its own depth
/// without being moved first is a comparison of a depth with itself and reads as shadowed. The
/// pool path moves it, and this path was moving it too — in **world units**, which is the one
/// unit in which a shadow's own scale is never stated. A grid eight pages a side over a cone
/// seventeen units wide holds a thousand and twenty four texels across it, so a texel of that
/// plane on the floor is about a hundredth of a world unit, and the constant this path carried
/// was a push of **a hundred and thirty texels**.
///
/// A push that size does not soften a shadow; it moves the point being read so far off the
/// surface that the caster which should be between the lamp and that point is not there any
/// more. A wall shorter than the push casts nothing at all — not a soft shadow and not a
/// misplaced one, no shadow — and the height at which it stops is the number in the shader,
/// which is how that was found.
///
/// So the push is measured in the recording's own unit instead, and read out of the light's own
/// projection: that is the same matrix that placed the point, so it is the one that knows how
/// wide a texel is **where this fragment is**, and a light with a wider cone or a nearer page
/// gets a wider texel and a wider push without anything here knowing it. It is one texel rather
/// than a fraction of one, because the tap that has to be moved is the one that reads the
/// surface's own depth.
///
/// The push is in **level zero's** texels and does not follow the level, because the level is
/// not known until the page has been found and the page is found from this point. What a coarse
/// page needs beyond that is what the bias above is for, and it already scales: a level `k`
/// page's own error is `2^k` times as large and its bias is `2^k` times as large with it.
fn shadowed_pages(
    address: vec4<f32>,
    shadow_projection: mat4x4<f32>,
    to_light: vec3<f32>,
    world_position: vec3<f32>,
    normal: vec3<f32>,
) -> f32 {{
    let from_light = shadow_projection * vec4<f32>(world_position, 1.0);
    if from_light.w <= 0.0 {{
        return 1.0;
    }}
    let ndc = from_light.xyz / from_light.w;
    // Tested before the conversion rather than after, because a negative normalised coordinate
    // turned into an unsigned one is not a defined answer, and the plane's own edge is the
    // boundary of this light's shadow either way.
    if ndc.x < -1.0 || ndc.x > 1.0 || ndc.y < -1.0 || ndc.y > 1.0 {{
        return 1.0;
    }}
    let header = u32(address.x);
    let first = bitcast<u32>(light_records[header]);
    let second = bitcast<u32>(light_records[header + 1u]);
    let pages_per_side = first & 0xFFFFu;
    let atlas_side = max(first >> 16u, 1u);
    let levels = second & 0xFFFFu;
    // The whole grid, in texels of the light's plane: level zero's own resolution.
    let side = pages_per_side * PAGE_TEXELS;
    // How wide one texel of that plane is **in world units, measured here**: the plane is two of
    // its own normalised units across and `side` texels wide, and the light's own projection
    // says how far apart two world points one texel apart land in it. The direction measured
    // across is whichever of the two world axes lies further across the light's own view,
    // because a step along the normal of a surface standing square to the lamp does not move in
    // the plane at all, and the reciprocal of no movement is not a distance.
    let axis = select(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, 0.0, 1.0),
        abs(to_light.x) >= abs(to_light.z),
    );
    let across_world = normalize(cross(to_light, axis));
    // What one world unit along that direction is worth in the plane's own units: a point is
    // read as its normalised coordinate over its own `w`, so a world step arrives as its own
    // contribution less that coordinate times its share of `w`.
    let span = shadow_projection * vec4<f32>(across_world, 0.0);
    let per_unit = length((span.xy - ndc.xy * span.w) / from_light.w);
    let texel_world = 2.0 / (f32(side) * max(per_unit, 1e-6));
    // The point the shadow is read **at**, off the surface by the push above and by nothing
    // else. It is the one point the whole walk below reads, the page it names and the texel
    // within that page alike, because a walk that named the page from one point and the texel
    // from another would wrap its own page arithmetic at every boundary and read a texel up to
    // a page away from the one it chose.
    let pushed = shadow_projection
        * vec4<f32>(world_position + normal * (texel_world * PUSH_TEXELS), 1.0);
    if pushed.w <= 0.0 {{
        return 1.0;
    }}
    let at = pushed.xyz / pushed.w;
    // Tested before the conversion rather than after, because a negative normalised coordinate
    // turned into an unsigned one is not a defined answer, and the plane's own edge is the
    // boundary of this light's shadow either way.
    if at.x < -1.0 || at.x > 1.0 || at.y < -1.0 || at.y > 1.0 {{
        return 1.0;
    }}
    // Clip space y runs up and a texture's rows run down, which is the same fold the pool path
    // makes in its `uv`, and the same one the compiler makes when it lays pages out.
    let texel = vec2<u32>(
        u32((at.x * 0.5 + 0.5) * f32(side)),
        u32((0.5 - at.y * 0.5) * f32(side)),
    );
    if texel.x >= side || texel.y >= side {{
        return 1.0;
    }}
    for (var level = 0u; level < levels; level = level + 1u) {{
        let shift = PAGE_TEXELS << level;
        let across = max(pages_per_side >> level, 1u);
        let page_x = min(texel.x / shift, across - 1u);
        let page_y = min(texel.y / shift, across - 1u);
        // **Nothing at this level or below, and the walk is over.** The level above carries a mask
        // over this level's grid, one bit per page, set where this level and every coarser one are
        // empty over that page — so one bit test replaces reading the page and then, if it recorded
        // nothing, reading every level under it as well. The mask is a statement about *all* the
        // levels below and never about the one above, which is what makes stopping here safe: an
        // empty level says nothing about the levels beneath it, since a caster that asked for a
        // coarse level casts quite happily where a fine one does not.
        if level > 0u {{
            let sparse_section = bitcast<u32>(light_records[header + 2u + LEVELS_TABLE + level - 1u]);
            let sparse_index = page_y * across + page_x;
            let sparse_bits = bitcast<u32>(light_records[header + sparse_section + sparse_index / 32u]);
            if (sparse_bits & (1u << (sparse_index % 32u))) != 0u {{
                return 1.0;
            }}
        }}
        // One base slot, then the mask words of a row of `across` pages.
        let words = 1u + (across + 31u) / 32u;
        let section = bitcast<u32>(light_records[header + 2u + level]);
        let row = header + section + page_y * words;
        let bit = page_x % 32u;
        let column = page_x / 32u;
        let bits = bitcast<u32>(light_records[row + 1u + column]);
        if (bits & (1u << bit)) == 0u {{
            continue;
        }}
        var rank = countOneBits(bits & ((1u << bit) - 1u));
        for (var earlier = 0u; earlier < column; earlier = earlier + 1u) {{
            rank = rank + countOneBits(bitcast<u32>(light_records[row + 1u + earlier]));
        }}
        let slot = bitcast<u32>(light_records[row]) + rank;
        let origin = vec2<i32>(
            i32((slot % atlas_side) * PAGE_TEXELS),
            i32((slot / atlas_side) * PAGE_TEXELS),
        );
        let within = vec2<u32>((texel.x % shift) >> level, (texel.y % shift) >> level);
        let centre = origin + vec2<i32>(i32(within.x), i32(within.y));
        let slope = 1.0 - abs(dot(normal, to_light));
        let bias = (0.0006 + 0.003 * slope) * f32(1u << level);
        let layer = i32(address.y);
        var lit = 0.0;
        for (var r = -1; r <= 1; r = r + 1) {{
            for (var c = -1; c <= 1; c = c + 1) {{
                let tap = clamp(
                    centre + vec2<i32>(c, r),
                    origin,
                    origin + vec2<i32>(i32(PAGE_TEXELS) - 1),
                );
                lit = lit + select(0.0, 1.0, at.z - bias
                    <= textureLoad(shadow_atlas, tap, layer, 0));
            }}
        }}
        // **Once shadowed, stop.** A caster is drawn into the one level its density buys, so a
        // level that does not shadow this point holds no caster that does, and the next level down
        // is the only place the rest of them can be. A level with no page, and a level whose page
        // recorded nothing at this point, both read as nine lit taps and fall out of here by the
        // same test — which is why there is no separate emptiness check to keep in step with this
        // one.
        //
        // The fraction is this level's, not a sum over the levels: the first level that registers
        // a shadow is the one whose resolution the answer is stated at, and a coarse level is never
        // blended into a fine one.
        if lit < 9.0 {{
            return lit / 9.0;
        }}
    }}
    return 1.0;
}}

/// Returns how lit a surface is by one light, whichever of the two shapes its shadow has.
///
/// # Why the record decides and not the kind
///
/// A light's record carries **one** of the two addresses and minus one in the other, because a
/// kind that has moved to pages takes no pool page at all — the design replaces the fixed path
/// rather than keeping it as a fallback, and a light drawn twice would be a light shaded by the
/// sum of two shadows. So the test is on the address and not on the light's kind, which is what
/// lets a point light and a directional one move to pages later with no edit here at all: they
/// arrive carrying an address, and this reads it.
fn shadow_term(
    shadow_projection: mat4x4<f32>,
    page_index: f32,
    address: vec4<f32>,
    to_light: vec3<f32>,
    world_position: vec3<f32>,
    normal: vec3<f32>,
) -> f32 {{
    if address.x < 0.0 {{
        return shadowed(
            shadow_projection,
            page_index,
            to_light,
            world_position,
            normal,
        );
    }}
    return shadowed_pages(
        address,
        shadow_projection,
        to_light,
        world_position,
        normal,
    );
}}

/// Returns what one light adds to a surface, given which way it arrives and how bright it is.
///
/// # Why this is shared and not written out twice
///
/// The diffuse and specular terms below are the same for a clustered light and for the
/// directional one, and a copy of them would be a second place for the shading model to be
/// edited. The **only** thing that differs between the two callers is how the light was found:
/// a clustered light's brightness arrives already scaled by its falloff and its shadow, and
/// the directional one arrives unscaled, because a light that reaches every distance equally
/// has no falloff to apply.
///
/// `fresnel` is passed in rather than recomputed here because it depends only on the surface
/// and the view, and the cluster loop already works it out once per fragment.
fn light_contribution(
    to_light: vec3<f32>,
    radiance: vec3<f32>,
    normal: vec3<f32>,
    to_view: vec3<f32>,
    diffuse: vec3<f32>,
    fresnel: vec3<f32>,
    roughness: f32,
) -> vec3<f32> {{
    // The specular lobe, in two parts that answer different questions.
    //
    // `strength` is how much of the surface's reflectance comes back as a highlight rather
    // than as diffuse. That is a property of the material, and it is the only part of the
    // highlight the tint has a say in.
    //
    // `exponent` is the shape of the lobe: a rough surface spreads its highlight. Widening a
    // lobe is only half of what a microfacet distribution does, though — the other half is
    // that **the peak falls as the lobe widens**, because the same energy is spread thinner.
    // Left unnormalised, `pow` alone gives a searchlight rather than a material: a surface as
    // rough as a wall still shows a highlight at a mirror's brightness, and no material can
    // say otherwise. Dividing by the lobe's own width is what puts the roughness back into
    // the peak. The division is against the smoothest lobe this template builds, so a mirror
    // is exactly as bright here as it always was and only the roughness moves.
    let strength = mix(0.02, 1.0, (1.0 - roughness) * (1.0 - roughness));
    let exponent = mix(512.0, 4.0, roughness);
    // The lobe's own width, against the smoothest lobe this template builds, so that a mirror
    // is exactly as bright here as it was before the lobe was normalised.
    let lobe_width = sqrt(2.0 / (exponent + 2.0));
    let smooth_lobe_width = sqrt(2.0 / (514.0));

    let lambert = max(dot(normal, to_light), 0.0);
    let half_vector = normalize(to_light + to_view);
    let specular = pow(max(dot(normal, half_vector), 0.0), exponent)
        * strength * (smooth_lobe_width / lobe_width);

    return diffuse * lambert * radiance + fresnel * specular * radiance * 3.0;
}}

/// Returns what the scene's directional light adds to a surface.
///
/// # Why it is a stated input and not a member of a cluster list
///
/// It stands nowhere and reaches every depth equally, so it is in every cluster by definition
/// and listing it would spend the cap on a light the culling was never going to remove. There
/// is no list to be in, so it is evaluated here, once, outside the loop — which is the
/// correct cost: there is no culling to do and no light to choose between, and this is one
/// BRDF evaluation beside a loop that already runs.
///
/// # Why there is no falloff
///
/// A point light's brightness falls off with the square of its distance, and a directional
/// light has no distance: it reaches every distance equally, which is the whole of what makes
/// it the sun rather than a lamp. The attenuation the cluster loop applies is therefore
/// absent, and its absence is the light being drawn rather than an oversight.
///
/// The `colour.w` flag decides whether the scene named one, rather than the colour being
/// black: a scene may want a sun of no light at all, and inferring presence from the colour
/// would make that scene indistinguishable from one that named none.
fn directional_radiance(
    world_position: vec3<f32>,
    normal: vec3<f32>,
    to_view: vec3<f32>,
    diffuse: vec3<f32>,
    fresnel: vec3<f32>,
    roughness: f32,
) -> vec3<f32> {{
    if uniforms.directional.colour.w < 0.5 {{
        return vec3<f32>(0.0);
    }}
    // The uniform carries the direction the light **travels along**, which is the way it comes
    // **from**, so the vector towards it is the negative.
    let to_light = -normalize(uniforms.directional.direction.xyz);
    let lit = shadow_term(
        uniforms.directional.shadow_projection,
        uniforms.directional.direction.w,
        // A sun is still on the pool, so it carries no address at all and the test below routes
        // it to the one-page path. Its grid is the next stage of this migration.
        vec4<f32>(-1.0),
        to_light,
        world_position,
        normal,
    );
    return light_contribution(
        to_light,
        uniforms.directional.colour.rgb * lit,
        normal,
        to_view,
        diffuse,
        fresnel,
        roughness,
    );
}}

{helpers}
/// Returns how much of this light reaches a surface in the given direction.
///
/// `to_surface` is the unit vector **from the light towards the surface**, which is the
/// negative of the one the loop already has: the cone's axis is the way the light shines, and
/// a surface in front of the light is in front of it along that axis, not behind it.
///
/// # Why the limits are cosines
///
/// The number being compared is a dot product, so the two numbers compared against it are
/// cosines of the two half angles, and the falloff needs no inverse cosine to find the angle it
/// is at. The ramp between them is a straight line in the cosine, which is a curve in the angle
/// — and which curve it is belongs to the author, who picks it by where the two angles are.
/// A cone with the two cosines equal is the whole sphere, and that is what a light that is not
/// a spot light carries.
///
/// # Why the degenerate case is tested first
///
/// A cone with no range is a light with no cone, and it is recognised by the two limits being
/// equal rather than by either of them having a particular value. That is deliberate: the
/// whole sphere is written as a half turn, and whether the cosine of a half turn is exactly
/// minus one is a fact about rounding that this answer should not rest on. Tested second, the
/// single direction exactly opposite a point light would be *outside* a cone whose outer cosine
/// came out a rounding above minus one, and one unlit texel on a sphere is a bug rather than a
/// detail.
fn cone_attenuation(cone: SpotCone, to_surface: vec3<f32>) -> f32 {{
    let inner = cone.limits.x;
    let outer = cone.limits.y;
    if inner <= outer {{
        return 1.0;
    }}
    // The cosine of the angle between the light's axis and the surface. Beyond the outer
    // cosine the surface is outside the cone and gets nothing at all; short of the inner one it
    // is inside the full strength region and gets all of it.
    let facing = dot(to_surface, cone.axis.xyz);
    if facing <= outer {{
        return 0.0;
    }}
    if facing >= inner {{
        return 1.0;
    }}
    return (facing - outer) / (inner - outer);
}}

/// Returns what the light of one record adds to a surface.
///
/// # Why both loops call this rather than each carrying the arithmetic
///
/// A **clustered** light is found through its cluster's list and a **global** one through the
/// count the uniform carries, and the two differ in nothing else: same record, same cone, same
/// shadow lookup, same falloff, same two shading terms. Writing the body out twice would be a
/// second place for the shading model to be edited, which is the same argument
/// [`light_contribution`] makes for the diffuse and specular terms — and here the two copies
/// would be free to drift into disagreeing about what a light is.
///
/// The index is the whole difference between the callers, and it is an index into both runs
/// rather than a value: the record and the cone are packed in the same order, so a light's
/// cone is the same number away as the light itself.
fn record_contribution(
    index: u32,
    world_position: vec3<f32>,
    normal: vec3<f32>,
    to_view: vec3<f32>,
    diffuse: vec3<f32>,
    fresnel: vec3<f32>,
    roughness: f32,
) -> vec3<f32> {{
    let light = light_record(index);
    let cone = spot_cones[index];
    let to_light = light.position - world_position;
    let light_range = max(length(to_light), 0.0001);
    let direction = to_light / light_range;
    // The cone is an **extra** attenuation, not a replacement for the culling that put
    // this light in this cluster at all: a light outside the cone is already absent from
    // the list by its radius, and a light inside the radius but outside the cone is here
    // and scaled to nothing. A light that is not a spot light carries the whole sphere and
    // is scaled by exactly one, so a point light's arithmetic is unchanged — which is what
    // lets a global point light be evaluated by the same body as a clustered one.
    let lit = shadow_term(
        light.shadow_projection,
        light.page,
        light.virtual_page,
        direction,
        world_position,
        normal,
    ) * cone_attenuation(cone, -direction);

    // A point light falls off with the square of its distance. The constants are the
    // gentlest that still read as a falloff over a scene a couple of hundred units
    // across: a steeper one puts the light's own volume inside the scene, and every
    // surface near the light then reads as white no matter what its colour is.
    let attenuation = 1.0 / (1.0 + 0.0035 * light_range + 0.00002 * light_range * light_range);
    let radiance = light.color * attenuation * lit;

    return light_contribution(direction, radiance, normal, to_view, diffuse, fresnel, roughness);
}}

/// Returns the texel of the environment map a direction falls in, at one level.
///
/// # Why a two dimensional octahedral map
///
/// A cube map is the usual container for a direction, and this renderer's interface check
/// already has a shape for one — but it can only **recognise** a cube a shader declares.
/// Nothing here can build one: the device's texture shape answers `D2` or `D2Array` and there
/// is no third answer, so a cube would need a payload variant, a view descriptor, a layout
/// entry and a six face upload before a shader could name one, and a prefiltered specular
/// cube on top of that.
///
/// A two dimensional octahedral map folds the sphere into a square instead. That needs
/// nothing this device does not already do — one uploaded image is one flat texture with a
/// box filtered level chain behind it — and the chain is what makes roughness free: a rough
/// surface wants a blurrier environment, and a blurrier environment is the next level down.
///
/// The convention the map is authored to, which a scene needs to know to draw one: the
/// **centre** of the square is the world's `+z` direction, the **top** row is `+y`, and the
/// four edges are the seams the fold puts there.
///
/// # Why the fold is read at a texel and not through a sampler
///
/// The fold is a seam, and the two sides of it are as far apart in direction as the map is
/// wide. A filtered read across one blends a texel from each side, which is a direction
/// averaged with the direction almost opposite it, so the seam shows as a dark or a bright
/// line that no amount of resolution removes. `textureLoad` at a texel computed here takes
/// exactly one texel, so the only question left is whether that texel is inside the image.
///
/// Which is the second half of the bug this shape has to get right: a direction **on** the
/// square's edge gives a coordinate of exactly the map's size, and the first texel past the
/// end is a row of the next line of memory or a fault, depending on the backend. The answer
/// at a seam is continuous — the two texels either side of it face almost the same direction
/// — so clamping the coordinate to the last texel is not a rounding of the answer, it *is*
/// the answer.
fn octahedral(direction: vec3<f32>, level: i32) -> vec2<i32> {{
    // The octahedron's own projection, divided by the sum of the magnitudes so that the whole
    // sphere lands in the square rather than in a disc inscribed in it. The divisor is floored
    // away from zero because a zero direction has no direction to place, and the origin of the
    // square is a legal answer for it.
    var planar = direction.xy / max(
        abs(direction.x) + abs(direction.y) + abs(direction.z),
        1e-8,
    );
    // The half of the sphere behind the centre is mirrored back into the square, and that
    // mirror is the entire fold: four triangles folded onto the one the front covers. The sign
    // is taken as it is rather than as `sign`, because `sign(0.0)` is zero in WGSL and a zero
    // component has to land on the positive edge of the square, not on its middle.
    if direction.z < 0.0 {{
        let folded = vec2<f32>(1.0) - abs(planar.yx);
        planar = vec2<f32>(
            select(-folded.x, folded.x, planar.x >= 0.0),
            select(-folded.y, folded.y, planar.y >= 0.0),
        );
    }}
    // A square's `+y` is the map's top, and an image's rows run down, so the vertical half of
    // the mapping is flipped. The horizontal half is not: `+x` is the map's right, which is
    // where an image's columns run too.
    let uv = vec2<f32>(planar.x * 0.5 + 0.5, 0.5 - planar.y * 0.5);
    // The map's own size **at this level**, not the size of level zero. A coarse level of a
    // 256 texel map is four texels across, and indexing it with the level zero size reads four
    // times past the end of it.
    let extent = vec2<i32>(textureDimensions(environment_map, level));
    // Truncated rather than rounded, so a coordinate is the texel whose square contains it
    // rather than the nearest one, and clamped to the last texel on each axis because of the
    // seam. The coordinate is never negative — `uv` is a square — and the clamp says so rather
    // than leaving it to the arithmetic.
    return clamp(
        vec2<i32>(uv * vec2<f32>(extent)),
        vec2<i32>(0),
        extent - vec2<i32>(1),
    );
}}

/// Returns what a surface facing this normal receives from the environment.
///
/// # The environment replaces the stand-in, and the flag is what says so
///
/// There is **one** ambient term in this shader, and what feeds it is the scene's environment
/// when the scene names an `Environment` light and the two colour hemisphere when it does not.
/// Not both: a scene that named an environment and also had the hemisphere under it would be
/// counting an author's environment twice and adding a sky of this renderer's own that the
/// author had no way to turn off, which is exactly what the hemisphere was — a stand-in for
/// something the shader could not yet read.
///
/// The choice is made on `uniforms.environment.w` and on nothing else. A scene naming no
/// environment binds a one texel **black** map, so a shader that decided on the colour it
/// sampled could not tell "there is no environment" from "the environment is black", and a
/// frame's darkness would be a property of a texture rather than of a decision anybody made.
///
/// # Why the level is the roughness, and why it is clamped
///
/// A rough surface wants a blurrier environment, and the map's level chain is already blurred
/// — the device derives every level from the one above by a box filter, averaging in linear
/// light. So the level is the roughness scaled by the chain and clamped into it. The clamp is
/// what decides the question of whether a document can ask for something impossible: a
/// roughness of one maps to the **last** level rather than one past it, so the top of the
/// range is the top of the chain and no material is refused for wanting a blurer environment
/// than the map has.
///
/// The stand-in is returned unchanged, from the same two colours it has always been mixed
/// from, so a scene that names no environment light renders byte for byte what it rendered
/// before this stage. The tint and the metalness are applied by the caller and not here, and
/// that is deliberate: they are the material's own, and a term that stopped multiplying by
/// them would be a material that had stopped responding to its own colour.
fn environment_radiance(normal: vec3<f32>, roughness: f32) -> vec3<f32> {{
    if uniforms.environment.w < 0.5 {{
        return mix(
            vec3<f32>(0.014, 0.017, 0.026),
            vec3<f32>(0.055, 0.072, 0.110),
            normal.y * 0.5 + 0.5,
        );
    }}
    // How many levels the map **has**, read rather than written: the chain is the device's to
    // build and it is a function of the image's size, so a number written here would be right
    // for one map and past the end of another.
    let levels = i32(textureNumLevels(environment_map));
    let level = clamp(i32(roughness * f32(levels - 1)), 0, levels - 1);
    return textureLoad(environment_map, octahedral(normal, level), level).rgb
        * uniforms.environment.rgb;
}}

struct Vertex {{
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) texture: vec2<f32>,
    @location(3) emissive: vec3<f32>,
    @location(4) tint: vec3<f32>,
    @location(5) material: vec3<f32>,
{vertex_varyings}}};

@vertex
fn vertex_main(
{vertex_inputs}) -> Vertex {{
    let model = mat4x4<f32>(model0, model1, model2, model3);
    // A normal is carried through the inverse transpose of the model, which the emitter
    // fills in beside the model itself. A transform that scales unevenly tilts a normal
    // by exactly the amount it is uneven by, so an instance twenty times taller than it
    // is wide is lit wrongly by the model alone — and `WGSL` has no matrix inverse for a
    // shader to recover it with.
    let normals = mat4x4<f32>(normal_matrix0, normal_matrix1, normal_matrix2, normal_matrix3);
    let world = model * vec4<f32>(position, 1.0);
    var out: Vertex;
    out.position = uniforms.projection.view_projection * world;
    out.world_position = world.xyz;
    out.normal = normalize((normals * vec4<f32>(normal, 0.0)).xyz);
    out.texture = texture;
    out.emissive = emissive;
    out.tint = tint;
    out.material = vec3<f32>(metallic, roughness, reflect);
{vertex_assigns}    return out;
}}

{fragment_returns}@fragment
fn fragment_main(in: Vertex) -> {fragment_type} {{
    // The shading normal, and the one thing a scene may displace. Everything below is a
    // function of it, so displacing it here moves the diffuse term, the highlight, the
    // Fresnel term and the offset the shadow is looked up at together — which is what a
    // wave, a leaf or a piece of cloth needs, and what a body spliced after them could not
    // reach.
    var normal = normalize(in.normal);
{displace}
    let metallic = in.material.x;
    let roughness = clamp(in.material.y, 0.04, 1.0);
    let diffuse_color = in.tint * (1.0 - metallic);

    // Everything below that does not name a light is hoisted out of the loop, because it is
    // the same for every one of them: the view vector, the Fresnel term, and the roughness
    // the specular lobe is shaped by. Only the two terms that turn on the light's own
    // direction stay inside.
    let to_view = normalize(uniforms.projection.position - in.world_position);
    let normal_dot_view = max(dot(normal, to_view), 0.0);

    // A metal takes its colour from what it reflects and has no diffuse term; a dielectric
    // reflects about four percent at normal incidence, which is the Fresnel term below.
    let f0 = mix(vec3<f32>(0.04), in.tint, metallic);
    // The rise towards one at a grazing angle is a property of a **smooth** interface. A rough
    // surface is a field of microfacets tilted in every direction, so the reflectance it shows
    // edge on is not much above the one it shows head on — and Schlick's term, which saturates
    // at one whatever the roughness, hands it the whole difference anyway.
    //
    // That is what made a thin object seen edge on — a reed, a wire, a branch, a rope, a wet
    // edge, a horizon-lit rim — render as a white stick whatever tint it carried. At those
    // angles the specular reached a large share of the full light and the tint is not in the
    // specular to darken it, so the only lever a scene had was to scale the whole shaded
    // colour down, which is a scene reaching around the shading model to undo something the
    // model did. The ceiling below is `max(1 - roughness, f0)`: a smooth surface still reaches
    // one, and a fully rough dielectric settles at the flat four percent it physically is.
    let fresnel_ceiling = max(vec3<f32>(1.0) - roughness, f0);
    let fresnel = f0 + (fresnel_ceiling - f0) * pow(1.0 - normal_dot_view, 5.0);


    // The cluster this fragment is in, its light count, and the start of its row. The row is
    // the cap wide rather than the count, so no prefix sum of the counts is needed to find
    // it: a cluster's lights are the first `count` of a row that always starts in the same
    // place.
    let cluster = cluster_of(in.position);
    let count = u32(cluster_light_counts[cluster]);
    let row = cluster * u32(uniforms.cluster_grid.w);

    var color = vec3<f32>(0.0);
    for (var slot = 0u; slot < count; slot = slot + 1u) {{
        // One index into both runs: the record and the cone are packed in the same order, so
        // a light's cone is the same number away as the light itself.
        let index = u32(cluster_light_indices[row + slot]);
        color = color + record_contribution(
            index,
            in.world_position,
            normal,
            to_view,
            diffuse_color,
            fresnel,
            roughness,
        );
    }}

    // The **global** lights, evaluated by every fragment whether or not its cluster holds
    // them. They are the first `global_count` records of the run, so this loop walks indices
    // rather than a list: a global light is a light that names no reach, or names one that
    // holds the whole scene, and culling it would never drop it, so the cluster slot it was
    // given bought nothing. A light with a reach that does not reach is not here — it is in
    // the loop above, and the boundary between the two is where its radius ends, which is
    // what draws a terminator as a staircase of cluster-grid rectangles rather than as a
    // curve. The body is [`record_contribution`]'s, the same one the cluster loop uses, and
    // the record carries the same page and the same projection, so a global light still
    // casts its shadow.
    for (var index = 0u; index < u32(uniforms.global_count); index = index + 1u) {{
        color = color + record_contribution(
            index,
            in.world_position,
            normal,
            to_view,
            diffuse_color,
            fresnel,
            roughness,
        );
    }}

    // The directional light is added **once**, outside the loop, because it is not a member of
    // any cluster: there is no list to walk and no culling to do, and every fragment wants it
    // equally. The order against the ambient term below is the order of the two questions — a
    // surface is lit by a direction, and then by whatever reaches it where no light does — and
    // both are additions, so nothing depends on which came first.
    color = color + directional_radiance(
        in.world_position,
        normal,
        to_view,
        diffuse_color,
        fresnel,
        roughness,
    );

    // **The ambient term, and what feeds it.** It is added **once**, whatever the cluster
    // holds, because it is not one light's contribution: while the stage ran once per light
    // this term was inside the loop and a scene of three lights added it three times, which is
    // a bug the clustered pass happens to fix.
    //
    // `environment_radiance` is the whole of what is decided about the source, and the
    // decision is spelled out there: the scene's own environment map when it names an
    // `Environment` light, the two colour hemisphere that stood in for one when it does not,
    // and never both.
    //
    // The multiplication that follows is unchanged and is the reason it is here rather than
    // inside that function. The tint and the metalness are the **material's own**, the same
    // two the cluster loop uses, and a term that had stopped reaching them would be a
    // material that had stopped responding to its own colour — which is a regression nobody
    // asked for and the one thing this stage must not do.
    let sky = environment_radiance(normal, roughness);
    let ambient = sky * mix(vec3<f32>(1.0), in.tint, metallic);
    color = color + ambient * diffuse_color;
    // What the surface emits is not added here either, for the same reason: the lighting
    // stage runs once, so a term added here is a term added once, and emission is its own
    // stage.
{surface}
{fragment_writes}    return {fragment_return};
}}
"#
    )
}

/// The lighting stage of a scene that samples an image.
///
/// # Where the sample goes
///
/// The lighting stage ends in the [`lighting`] template's `{surface}` splice point, which is
/// reached with the shaded colour computed and the interpolated point in scope. So this is the
/// same shader with one term added, rather than a second lighting stage that would drift from the
/// first: everything about how the surface is lit stays where it is, and the image decides what
/// the surface *is*.
///
/// The image is declared in the custom schema under the name `image`, so the compiler binds it at
/// that point and the shader reads it there. It is the one place a scene's document and a shader's
/// source meet, and it is a name rather than a number.
pub fn textured<'text>(vertex_inputs: &str, helpers: impl Into<Block<'text>>) -> String {
    lighting_stage(
        vertex_inputs,
        "".into(),
        "let pattern = textureSample(image, image_sampler, in.texture);\n    \
         color = color * (0.25 + 1.5 * pattern.rgb);"
            .into(),
        helpers.into(),
        lighting_bindings(true, &Extras::NONE.binding_points()),
        &Extras::NONE,
    )
}

/// Returns a tile pattern as pixels: four bytes a texel, top row first.
///
/// # Why the image is generated rather than loaded
///
/// Nothing in this framework reads the filesystem, so a texture is something a scene compiler
/// produces. A pattern a loop can write is a texture, which is the same reason a dune field's
/// points are generated: it is reviewable where a binary blob is not, and it travels with the
/// document instead of being looked up beside it.
///
/// The pattern is a grid of tiles with a mortar line between them, shaded so that no two
/// neighbouring tiles are the same colour. That is deliberate: a flat texture would prove only
/// that *a* texture was bound, and a pattern proves the texel the shader read is the texel that
/// was uploaded.
pub fn tile_pixels(size: i64) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            // Four checks a side. A finer grid is correct and unreadable: at the distance a floor
            // is drawn from, a small check falls below a texel per pixel and the level chain
            // averages it to a flat grey, which is the filter working and is also a picture with
            // nothing in it.
            let cell = size / 4;
            let (in_x, in_y) = (x % cell, y % cell);
            let mortar = in_x < 2 || in_y < 2;
            let (tile_x, tile_y) = (x / cell, y / cell);
            // Two greys far enough apart to be told in a lit frame, and a mortar line much
            // lighter than either. A surface multiplies this, so the tiles land as a grid of
            // light and dark squares with a bright seam rather than as a flat wash.
            let shade = if (tile_x + tile_y) % 2 == 0 { 210 } else { 120 };
            // A slight gradient across each tile, so a texel read one row off lands on a
            // different value rather than on the same flat colour.
            let within = ((in_x + in_y) as f64 / (2.0 * cell as f64) * 30.0) as u8;
            let value = if mortar { 240 } else { shade + within };
            pixels.extend_from_slice(&[
                value,
                (value as f64 * 0.85) as u8,
                (value as f64 * 0.7) as u8,
                255,
            ]);
        }
    }
    pixels
}

/// Returns the direction one texel of an octahedral map stands for.
///
/// # Invariants
///
/// This is the shader's fold **run backwards**, and it is written out here rather than shared
/// with it because the two live in different languages and because a generator that used the
/// shader's own arithmetic would agree with a fold that is wrong. A test that builds a map this
/// way and samples it with the shader proves the two are inverses, which is the only property
/// that matters and the one neither can check alone.
///
/// The overshoot is subtracted along each axis' own sign rather than reflected. Reflection is
/// what a reader would reach for, and it is the wrong one: it sends the point through the
/// origin and out the other side, which is why a decode written that way and an encode written
/// this way are not inverses.
fn octahedral_direction(column: i64, row: i64, size: i64) -> [f64; 3] {
    // The texel's place in the square, in the units the fold works in: minus one to one, with
    // the image's first row at the **top**, which is the mirror of the shader's own mapping and
    // is why the vertical half is negated here.
    let planar = [
        (column as f64 + 0.5) / size as f64 * 2.0 - 1.0,
        1.0 - (row as f64 + 0.5) / size as f64 * 2.0,
    ];
    // The front of the sphere is the triangle the square's interior covers, and its height
    // above the centre plane is what is left of the square after the two coordinates have
    // taken their share.
    let depth = 1.0 - planar[0].abs() - planar[1].abs();
    // Outside that triangle — the four corners of the square — the point is behind, and the
    // fold brings it back by pulling each coordinate in by however far past the edge it went.
    let overshoot = if depth < 0.0 { -depth } else { 0.0 };
    let folded = [
        planar[0]
            - if planar[0] >= 0.0 {
                overshoot
            } else {
                -overshoot
            },
        planar[1]
            - if planar[1] >= 0.0 {
                overshoot
            } else {
                -overshoot
            },
    ];
    let length = (folded[0] * folded[0] + folded[1] * folded[1] + depth * depth).sqrt();
    [folded[0] / length, folded[1] / length, depth / length]
}

/// Returns an octahedral environment map as pixels, four bytes a texel, top row first.
///
/// # What the map is
///
/// A **sky above and a ground below**, and nothing else: the value of a texel is a function of
/// the direction it stands for, and only of that direction's height, so the top row of the
/// square is as bright as it goes and the bottom row is as dark as it goes. That is the whole
/// point of generating it this way rather than drawing a gradient by hand — a hand drawn
/// gradient is bright at the top and dark at the bottom *as an image*, which is a claim about
/// rows, whereas this is a claim about directions, and the two differ by exactly the fold the
/// shader has to get right.
///
/// The middle row of the square is the seam, and it is the half of the map the other half is
/// folded onto, so it holds the horizon **twice**: once for the direction just above it and once
/// for the direction just below. A generator that could not produce that row could not be
/// testing the fold.
pub fn octahedral_pixels(size: i64) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for row in 0..size {
        for column in 0..size {
            let direction = octahedral_direction(column, row, size);
            // A number in zero to one from the direction's height, and nothing else: no noise,
            // no colour, no detail. A map with detail in it would make a blurrier level and a
            // sharper one differ for a reason that has nothing to do with which one was read.
            let height = (direction[1] * 0.5 + 0.5).clamp(0.0, 1.0);
            let value = (height * 255.0).round() as u8;
            pixels.extend_from_slice(&[value, value, value, 255]);
        }
    }
    pixels
}

/// The colours a [`sunset_pixels`] map is mixed from, as they appear in the map.
///
/// They are a function of the direction's **height** and not of where the sun is, which is what
/// makes a map a scene can write without a rotation: the horizon runs the whole way round the
/// square, so a warm band at the horizon is warm in every direction and needs nothing placed.
const ZENITH: [f64; 3] = [0.20, 0.30, 0.52];
const HORIZON: [f64; 3] = [0.92, 0.52, 0.26];
const GROUND: [f64; 3] = [0.16, 0.12, 0.12];

/// Returns a sunset environment map as pixels, four bytes a texel, top row first.
///
/// # What the map is, and why it is not a hemisphere
///
/// A cold dim zenith, a warm and much brighter band along the horizon, and a dark warm ground
/// below it. The band is the part that matters: it is **brighter than the zenith**, so the
/// brightest thing in this environment is not overhead. The two colour hemisphere it replaces
/// is darkest overhead and brightest at the zenith, so a frame lit by this map and a frame lit
/// by the stand-in differ in more than brightness — they differ in which way is bright, and a
/// surface tilted towards the horizon picks up more light than one tilted at the sky.
///
/// The colours are mixed by a curve over the direction's height rather than written per texel,
/// so the map is a function of directions and survives the fold: the four corners of the square
/// hold the horizon too, and they hold it because of the arithmetic rather than because anybody
/// drew a corner.
pub fn sunset_pixels(size: i64) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for row in 0..size {
        for column in 0..size {
            let height = octahedral_direction(column, row, size)[1];
            // Below the horizon is the ground, which is not black: a real one is lit by the sky
            // above it, and a black underside is the thing that makes a render look like it was
            // drawn rather than lit.
            let colour = if height < 0.0 {
                GROUND
            } else {
                // A band that peaks at the horizon and falls away towards the zenith. The curve
                // is a squared falloff rather than a linear one because a linear one puts half
                // the range at the zenith, which is the shape the hemisphere already has.
                let toward_horizon = (1.0 - height).powi(2);
                [
                    ZENITH[0] + (HORIZON[0] - ZENITH[0]) * toward_horizon,
                    ZENITH[1] + (HORIZON[1] - ZENITH[1]) * toward_horizon,
                    ZENITH[2] + (HORIZON[2] - ZENITH[2]) * toward_horizon,
                ]
            };
            pixels.extend_from_slice(&[
                (colour[0].clamp(0.0, 1.0) * 255.0).round() as u8,
                (colour[1].clamp(0.0, 1.0) * 255.0).round() as u8,
                (colour[2].clamp(0.0, 1.0) * 255.0).round() as u8,
                255,
            ]);
        }
    }
    pixels
}

/// The post stage, which draws over the whole frame.
/// The three points arrive already in clip space, so the vertex stage passes them
/// through: a post stage is not drawn through the camera's projection, it *is* the frame.
/// The fullscreen mesh is a position and nothing else, and the pass binds no instance row,
/// so the only attribute is at location zero.
///
/// `helpers` is WGSL a scene needs beyond the grade itself — a sky, a fog, a noise —
/// declared at module level beside `ray_through`, because `WGSL` has no nested functions
/// and `grade` is written into the body of one. It is the same arrangement the lighting
/// stage has, and for the same reason.
pub fn post<'text>(grade: impl Into<Block<'text>>, helpers: impl Into<Block<'text>>) -> String {
    let grade = grade.into();
    let helpers = helpers.into();
    let grade = grade.spliced("grade");
    let helpers = helpers.spliced("helpers");
    format!(
        r#"
struct Camera {{
    forward: vec3<f32>,
    half_extent: vec2<f32>,
    position: vec3<f32>,
    right: vec3<f32>,
    up: vec3<f32>,
}};

struct Projection {{
    position: vec3<f32>,
    view_projection: mat4x4<f32>,
}};

struct Uniforms {{
    camera: Camera,
    depth_range: vec2<f32>,
    projection: Projection,
    time: f32,
}};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var color_target: texture_2d<f32>;
@group(0) @binding(2) var depth_buffer: texture_depth_2d;

struct Vertex {{
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}};

@vertex
fn vertex_main(@location(0) position: vec3<f32>) -> Vertex {{
    var out: Vertex;
    out.position = vec4<f32>(position.xy, 0.0, 1.0);
    // Clip space runs from minus one to one with y up; texture space runs from zero to
    // one with y down, so the two differ by a scale and a flip.
    out.uv = position.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    return out;
}}

/// Returns the ray through a pixel of the frame.
///
/// # Invariants
///
/// A ray through a pixel is recovered from the camera's three axes and the half extent of
/// the frame, not from the projection matrix. Recovering it from the matrix would mean
/// inverting it, and `WGSL` has no matrix inverse — which is why the emitter hands a post
/// stage the basis at all.
///
/// The offset runs from minus one to one with **y up**, which is the way the basis is
/// defined. Texture space runs with y down, so the vertical component of the offset is
/// flipped; a sky built on the unflipped one is a sky with its zenith at the bottom of the
/// frame, which reads as a vignette rather than as a sky.
fn ray_through(uv: vec2<f32>) -> vec3<f32> {{
    let offset = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    return normalize(
        uniforms.camera.forward
        + uniforms.camera.right * offset.x * uniforms.camera.half_extent.x
        + uniforms.camera.up * offset.y * uniforms.camera.half_extent.y,
    );
}}

/// Returns how far from the camera a surface at this depth is, measured along the view.
///
/// # Invariants
///
/// A depth buffer holds a fraction of the range between the near and far planes, and that
/// fraction is linear in a divide rather than in the distance: `near * far` over
/// `far - depth * (far - near)`. The two planes are handed over rather than left to be
/// assumed, because a stage without them cannot divide at all — and `WGSL` cannot recover
/// them, for the same reason it cannot invert the matrix the basis stands in for.
///
/// The result is along the view axis rather than along the ray through the pixel, so a
/// caller walking through the air along a ray divides it by `dot(ray, forward)`.
fn distance_to(depth: f32) -> f32 {{
    let range = uniforms.depth_range;
    return range.x * range.y / (range.y - depth * (range.y - range.x));
}}

{helpers}
@fragment
fn fragment_main(in: Vertex) -> @location(0) vec4<f32> {{
    let size = vec2<f32>(textureDimensions(color_target));
    let pixel = vec2<i32>(in.uv * size);
    // The shaded image is read as colour only: its alpha carries whatever the shading
    // happened to leave there, and a grade applied to it is a grade of nothing.
    var color = textureLoad(color_target, pixel, 0).rgb;
    // A depth of one is where nothing was drawn, because the depth buffer is cleared to
    // the far plane. This is what lets a post stage be the sky: it draws over the whole
    // frame, and only the pixels the scene never reached are the ones left to it.
    let depth = textureLoad(depth_buffer, pixel, 0);
{grade}
    return vec4<f32>(color, 1.0);
}}
"#
    )
}

/// Builds the three mesh shaders every gallery scene declares, and the point schema they
/// all read.
///
/// # Invariants
///
/// The attributes are derived from the point schema this passes, so the three stages and
/// the mesh a scene draws all read one layout, and a scene that declares a different point
/// schema gets the attributes for it without writing them out.
pub fn mesh_shaders<'text>(
    surface: impl Into<Block<'text>>,
    helpers: impl Into<Block<'text>>,
) -> Vec<Value> {
    mesh_shaders_with_normal("", surface, helpers)
}

/// The same three shaders, for a scene that displaces the normal it is lit by.
///
/// # Invariants
///
/// The four shaders are declared together, because the stages have to agree about the point
/// layout they all read and a scene that wrote one stage out by hand would have to keep that
/// agreement by hand. A scene that displaces a normal declares its own lighting shader
/// beside these rather than in place of them, and only the instances listed under `lighting`
/// with that material are drawn by it.
pub fn mesh_shaders_with_normal<'text>(
    displace: impl Into<Block<'text>>,
    surface: impl Into<Block<'text>>,
    helpers: impl Into<Block<'text>>,
) -> Vec<Value> {
    mesh_shaders_over("", displace, surface, helpers, kit::shaded_point_schema())
}

/// The same four shaders, for a mesh whose points carry more than the gallery's own three fields.
///
/// # Invariants
///
/// - **`prefix` names each shader `<prefix>_<stage>`, and an empty prefix gives the bare stage
///   name every existing scene already names.** A scene with two meshes needs two sets, because
///   the stages read the *point layout of the mesh they draw* — a plain drawn by a shader built
///   over a massif's wider point schema reads three attributes of somebody else's vertices, and
///   the failure is a mesh whose depth stage is looking at the wrong numbers rather than an
///   error. Two sets need two labels, and a shared name would be one entry silently
///   overwriting the other.
///
///   The prefix is empty for every caller that has one mesh, so nothing that exists changes
///   name.
///
/// - **All four are built from the one point schema, and the mesh declares that same one.** The
///   stages have to agree about the attribute locations they read, and they get that agreement
///   from a shared schema rather than from four shaders that happen to name the same fields.
/// - **Only the lighting stage's WGSL knows about the extra fields.** The shadowing and depthing
///   stages read position and nothing else, and their inputs are generated from the same schema,
///   so they skip the fields rather than being told they are there.
pub fn mesh_shaders_over<'text>(
    prefix: &str,
    displace: impl Into<Block<'text>>,
    surface: impl Into<Block<'text>>,
    helpers: impl Into<Block<'text>>,
    point: Schema,
) -> Vec<Value> {
    let inputs = crate::attributes::gallery_vertex_inputs(&point);
    // One block each, handed to every stage that takes it: a block declared once is the same
    // text in all of them, and converting per stage would clone the same WGSL four times.
    let displace = displace.into();
    let surface = surface.into();
    let helpers = helpers.into();
    let named = |stage: &'static str| {
        if prefix.is_empty() {
            stage.to_owned()
        } else {
            format!("{prefix}_{stage}")
        }
    };
    vec![
        kit::shader(
            Stage::Shadowing,
            &named("shadowing"),
            &shadowing(&inputs),
            shadow_pass_schema(),
            point.clone(),
            kit::gallery_instance_schema(),
            kit::gallery_custom_schema(),
        ),
        kit::shader(
            Stage::Depthing,
            &named("depthing"),
            &depthing(&inputs),
            depth_pass_schema(),
            point.clone(),
            kit::gallery_instance_schema(),
            kit::gallery_custom_schema(),
        ),
        kit::shader(
            Stage::Lighting,
            &named("lighting"),
            &lighting_with_normal(&inputs, displace, surface, helpers),
            light_pass_schema(),
            point.clone(),
            kit::gallery_instance_schema(),
            kit::gallery_custom_schema(),
        ),
        // The emission stage is declared beside the other three, so a scene that emits writes
        // no shader of its own and a scene that does not simply does not name this one. Naming
        // a shader is what puts an instance in a pass, so an emission shader nothing names
        // costs nothing.
        kit::shader(
            Stage::Emission,
            &named("emission"),
            &emission(&inputs, "", helpers),
            emission_pass_schema(),
            point,
            kit::gallery_instance_schema(),
            kit::gallery_custom_schema(),
        ),
    ]
}

/// Builds an emission shader with a body of the scene's own.
///
/// A scene whose surfaces emit a constant names [`mesh_shaders`]' `emission` and is done. A
/// scene whose emission varies across a surface — a falloff, a flicker, a pattern — needs a
/// body, and this is how it declares one. The body is written into the fragment stage's own
/// scope, where `emitted` is the colour the stage returns and `in` is the point and the row.
///
/// `helpers` is the module level `WGSL` the body is written against — a sky, a mist, a
/// noise — and it is what lets a body that reflects something be written against the same
/// copy of that something the rest of the scene uses.
pub fn emission_shader<'text>(
    label: &str,
    body: impl Into<Block<'text>>,
    helpers: impl Into<Block<'text>>,
) -> Value {
    emission_shader_with_schema(label, body, helpers, kit::gallery_custom_schema())
}

/// The same emission shader, over a custom schema of the scene's own.
///
/// See [`lighting_shader_with_schema`] for why the schema is a parameter rather than a
/// constant: an emission body that reads a field of the row needs the row to carry it, and a
/// body that does not is unaffected by where in the merge that field sits.
pub fn emission_shader_with_schema<'text>(
    label: &str,
    body: impl Into<Block<'text>>,
    helpers: impl Into<Block<'text>>,
    custom_schema: Schema,
) -> Value {
    let point = kit::shaded_point_schema();
    let inputs = crate::attributes::gallery_vertex_inputs_with(&point, &custom_schema);
    kit::shader(
        Stage::Emission,
        label,
        &emission(&inputs, body, helpers),
        emission_pass_schema(),
        point,
        kit::gallery_instance_schema(),
        custom_schema,
    )
}

/// Builds the two stage shaders that place a point rather than colour it, over a custom schema
/// of the scene's own.
///
/// # Why a placement shader declares a schema it never reads
///
/// A shadowing or depthing shader's fragment stage never reads the row — it writes depth — but
/// its `custom_schema` is what the row is verified against, and its **locations** come from the
/// same merge, so a scene whose materials carry a field of their own declares that field here
/// too. See [`lighting_shader_with_schema`].
///
/// This is the two of the four that [`mesh_shaders_with_normal`] declares. A scene supplying
/// its own lighting and emission bodies names those beside these rather than in place of them,
/// so that the three stages still agree about the point layout they all read.
pub fn placement_shaders_with_schema(label: &str, custom_schema: Schema) -> Vec<Value> {
    let point = kit::shaded_point_schema();
    let inputs = crate::attributes::gallery_vertex_inputs_with(&point, &custom_schema);
    vec![
        kit::shader(
            Stage::Shadowing,
            &format!("{label}_shadowing"),
            &shadowing(&inputs),
            shadow_pass_schema(),
            point.clone(),
            kit::gallery_instance_schema(),
            custom_schema.clone(),
        ),
        kit::shader(
            Stage::Depthing,
            &format!("{label}_depthing"),
            &depthing(&inputs),
            depth_pass_schema(),
            point,
            kit::gallery_instance_schema(),
            custom_schema,
        ),
    ]
}

/// Builds a lighting shader of a scene's own, which displaces the normal it is lit by.
///
/// # Why this is beside [`mesh_shaders`] rather than in place of it
///
/// A pass draws one shader, so a surface whose shading differs from the rest of the scene
/// needs a material that names a different one — and the two are then drawn by two passes
/// that share a point schema, a row schema and a vertex layout. That is the arrangement the
/// pipeline is built for: what differs between two surfaces is a shader and a row, not a
/// pipeline.
pub fn displaced_lighting_shader<'text>(
    label: &str,
    displace: impl Into<Block<'text>>,
    surface: impl Into<Block<'text>>,
    helpers: impl Into<Block<'text>>,
) -> Value {
    lighting_shader_with_schema(
        label,
        displace,
        surface,
        helpers,
        kit::gallery_custom_schema(),
    )
}

/// Builds a lighting shader over a custom schema of the scene's own.
///
/// # Why the schema is a parameter
///
/// A shader declares the shape of the material's `custom_data` it reads, and the row is
/// verified against it at compile time. A scene that adds a field to its own row — which is
/// exactly what a scene with more than a handful of surfaces has to do, because the
/// alternative is one shader per material — therefore has to declare it here, or every
/// material is refused by name.
///
/// This is the whole of what [`displaced_lighting_shader`] leaves out: the two differ in the
/// row they are verified against and in the **locations** that row lays out, and a list
/// written from the gallery's own schema would name the wrong attribute at every location
/// below a field the scene added. Nothing checks a vertex layout, so that is a draw that
/// reads a normal where a matrix is.
pub fn lighting_shader_with_schema<'text>(
    label: &str,
    displace: impl Into<Block<'text>>,
    surface: impl Into<Block<'text>>,
    helpers: impl Into<Block<'text>>,
    custom_schema: Schema,
) -> Value {
    lighting_shader_declaring(
        label,
        displace,
        surface,
        helpers,
        custom_schema,
        &Extras::NONE,
    )
}

/// Builds a lighting shader that also writes the frame's own extra outputs.
///
/// # Invariants
///
/// **The point schema is the one carrying the outputs' fields**, so `gallery_vertex_inputs_with`
/// reads them at the locations the shader's `vertex_main` names — derived from the same schema
/// rather than written beside it, which is what keeps a field that moved and a shader that reads
/// it from drifting apart. Nothing checks a vertex layout, so this is the one place the two can
/// be made to agree by construction.
pub fn lighting_shader_declaring<'text>(
    label: &str,
    displace: impl Into<Block<'text>>,
    surface: impl Into<Block<'text>>,
    helpers: impl Into<Block<'text>>,
    custom_schema: Schema,
    extras: &Extras,
) -> Value {
    let point = if extras.is_empty() {
        kit::shaded_point_schema()
    } else {
        extras.point_schema()
    };
    let inputs = crate::attributes::gallery_vertex_inputs_with(&point, &custom_schema);
    kit::shader(
        Stage::Lighting,
        label,
        &lighting_declaring(&inputs, displace, surface, helpers, extras),
        light_pass_schema_over(&extras.binding_points()),
        point,
        kit::gallery_instance_schema(),
        custom_schema,
    )
}

/// Builds the post shader a scene declares, which grades the frame the scene produced.
///
/// `helpers` is handed to [`post`] unchanged: the WGSL the grade's arithmetic is written
/// against, declared at module level.
pub fn post_shader<'text>(
    grade: impl Into<Block<'text>>,
    helpers: impl Into<Block<'text>>,
) -> Value {
    // A screen shader's variant carries a `stage` naming the stage it belongs to, which
    // the three mesh variants do not: they are named by their own tag, and a fourth one
    // that was not would leave a document with no way to say which stage a shader reads.
    Value::variant(
        Stage::Post.tag(),
        [
            (
                scene::standard::LABEL.to_owned(),
                Value::String("post".to_owned()),
            ),
            (
                scene::standard::SOURCE.to_owned(),
                Value::String(post(grade, helpers)),
            ),
            (
                scene::standard::PASS_SCHEMA.to_owned(),
                post_pass_schema().to_value(),
            ),
            // A post stage draws over the frame rather than over a mesh, so it declares no
            // point schema and no instance row. The emitter refuses a screen shader that
            // carries either, because a shader that declares an interface nothing supplies
            // is a shader whose layout nothing can check.
            (
                scene::standard::INSTANCE_SCHEMA.to_owned(),
                Schema::new(SchemaKind::Unit).to_value(),
            ),
            (
                scene::standard::CUSTOM_SCHEMA.to_owned(),
                Schema::new(SchemaKind::Unit).to_value(),
            ),
            ("stage".to_owned(), Value::String("post".to_owned())),
        ],
    )
}
