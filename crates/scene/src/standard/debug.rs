//! The debug channels: the extra images the standard pipeline can be asked to draw.
//!
//! # What a channel is
//!
//! An image about the frame rather than of it. A picture says where things are on the screen;
//! a channel says what is *there*, so a caller that sees a dark band can ask which part of the
//! scene it is instead of authoring a second scene to find out.
//!
//! # Why a channel is the compiler's business and not a screen material
//!
//! Every channel is a fullscreen pass over a depth buffer that already exists, so an author
//! could write one as a post process. What they would write is the same twenty lines of ray
//! recovery and matrix arithmetic at every call site, with the y flip and the matrix row
//! convention to get wrong each time. A channel is that arithmetic written once, against the
//! same camera basis and the same recovery the post stage already uses.
//!
//! # Why the channel is not in the scene document
//!
//! A scene says what is in it. A channel is a question its **caller** is asking, and the
//! caller is usually a debugging loop rather than the author of the picture. So it travels as
//! an input to the compile, beside the camera, which is the other input a compile has that the
//! scene does not express by itself.
//!
//! # What a channel covers, which is a contract on the scene
//!
//! Every channel here is reconstructed from the **depth buffer**, so it covers exactly what the
//! scene's `depthing` bucket drew. A scene whose lit stages list an instance its `depthing`
//! bucket does not has pixels in its picture that no channel can name a position for, and a
//! scene with no depth at all produces an empty image — which the compiler says in a note
//! rather than leaving to be discovered. This is not a limitation of a channel: the depth buffer
//! is where a renderer knows what a pixel shows, and a document whose stages disagree about what
//! is on screen is already a document whose depth test is against the wrong surface.

use definition::Schema;

use super::pass_schemas::{
    CAMERA, DEPTH_BUFFER, PROJECTION, VERTICES, camera_schema, interface, projection_schema,
};
use graphics::binders::{buffer_binding_schema, texture_binding_schema};

/// One extra image the standard pipeline can be asked to render.
///
/// # Invariants
///
/// The set is closed, on the same grounds the light kinds are: a channel the compiler cannot
/// draw is refused by name rather than emitted as a pass that produces nothing. A closed set
/// is also what keeps the *meaning* of an image fixed: `Position` is the world position of the
/// nearest surface, and a caller that has to ask what a channel holds would be reading a
/// picture rather than a fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugChannel {
    /// The world position of the nearest surface each pixel shows, and none where it shows none.
    Position,
}

impl DebugChannel {
    /// Every channel, for a caller that has to name them all in a message or in a loop.
    pub const ALL: [Self; 1] = [Self::Position];

    /// Returns the channel a tag names, or none when no channel is called that.
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "position" => Some(Self::Position),
            _ => None,
        }
    }

    /// Returns the tag the channel is named by, which is how a caller asks for it.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Position => "position",
        }
    }

    /// Returns the name the image is declared under in the emitted graphics document.
    ///
    /// This is what a caller looks the image up by, and it is deliberately not the binding
    /// point: the point is where a pass happens to write, and the name is what the image *is*.
    pub const fn output(self) -> &'static str {
        match self {
            Self::Position => "position",
        }
    }

    /// Returns the binding point the image is written into, and the instance entry holding it.
    pub const fn point(self) -> &'static str {
        match self {
            Self::Position => "debug_position",
        }
    }

    /// Returns the format the image is drawn in.
    ///
    /// # Invariants
    ///
    /// A position is three floats and a target that held eight bits a channel would quantise it
    /// to a fraction of whatever the scene's extent happens to be, so the image would answer
    /// "which region" and not "where". `rgba16float` is blendable, is a format the device
    /// already accepts, and is read back as a float rather than as a byte.
    pub const fn format(self) -> &'static str {
        match self {
            Self::Position => "rgba16float",
        }
    }

    /// Returns the label the pass is emitted with.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Position => "debug/position",
        }
    }

    /// Returns the instance entry the pass's shader lives in.
    pub const fn shader_entry(self) -> &'static str {
        match self {
            Self::Position => "debug_position_shader",
        }
    }

    /// Returns the shader source the channel is drawn with.
    pub const fn source(self) -> &'static str {
        match self {
            Self::Position => POSITION_SOURCE,
        }
    }

    /// Builds the interface of the channel's pass, which its own shader is checked against.
    ///
    /// # Invariants
    ///
    /// The binders are exactly the three a channel needs and no more, because the binding
    /// numbers are derived from their **names**: adding a point renumbers the shader's own
    /// declarations, so the list and the source in this module have to move together. The
    /// channel's point sorts before `depth_buffer`, so the target takes binding one as a hole
    /// and the depth buffer is read at two, which is what [`POSITION_SOURCE`] declares.
    pub fn pass_schema(self) -> Schema {
        interface(
            "A debug pass, which draws one image about the frame rather than of it.",
            vec![
                (
                    self.point(),
                    texture_binding_schema().with_documentation(
                        "The image this pass renders into. It is a target, so it takes a \
                         binding number with no entry behind it.",
                    ),
                ),
                (
                    DEPTH_BUFFER,
                    texture_binding_schema().with_documentation(
                        "The depth of the scene, which is what a pixel's position is recovered \
                         from. Read with `textureLoad`, so it declares no sampler.",
                    ),
                ),
                (
                    VERTICES,
                    buffer_binding_schema()
                        .with_documentation("The fullscreen triangle this pass draws."),
                ),
            ],
            vec![
                (
                    CAMERA,
                    camera_schema().with_documentation(
                        "The camera this pass draws through, which is what a ray through a \
                         pixel is recovered from.",
                    ),
                ),
                (
                    PROJECTION,
                    projection_schema().with_documentation(
                        "The camera's own projection, whose last two rows are what a depth \
                         becomes a position through. It is the **whole** view-projection and \
                         not a perspective's coefficients, so the recovery holds for an \
                         orthographic camera too.",
                    ),
                ),
            ],
        )
    }
}

/// The shader a position image is drawn with.
///
/// # Invariants
///
/// - **The recovery is written against rows two and three of the whole view-projection**, as
///   the post stage's fog recovery is: `z_clip = row_z · world` and `w = row_w · world`, so
///   `world = eye + ray * t` makes `depth = z_clip / w` one linear equation in `t`. Reading
///   `m[2][2]` and `m[3][2]` as a perspective's coefficients instead reads every distance as
///   the far plane, and does it only for a perspective camera, which is the failure that looks
///   like a bug in the camera.
/// - **The ray comes from the camera's basis and not from the matrix**, because a matrix
///   inverse is not a thing `WGSL` has. The emitter hands the basis over for exactly this.
/// - **A pixel no surface covers is none.** The depth buffer is cleared to one, so the test is
///   the cheap one the post stage already makes. A background pixel given a position would tell
///   a caller that the sky is six hundred units away, which is a wrong answer rather than a
///   missing one.
/// - **The value is the position and not a fraction of it.** The image is a float target, so
///   there is nothing to decode and no scale a caller has to be told.
pub const POSITION_SOURCE: &str = r#"
struct Camera {
    forward: vec3<f32>,
    half_extent: vec2<f32>,
    position: vec3<f32>,
    right: vec3<f32>,
    up: vec3<f32>,
};

struct Projection {
    position: vec3<f32>,
    view_projection: mat4x4<f32>,
};

struct Uniforms {
    camera: Camera,
    projection: Projection,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(2) var depth_buffer: texture_depth_2d;

struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vertex_main(@location(0) position: vec3<f32>) -> Vertex {
    var out: Vertex;
    out.position = vec4<f32>(position.xy, 0.0, 1.0);
    // Clip space runs from minus one to one with y up; texture space runs from zero to one
    // with y down, so the two differ by a scale and a flip.
    out.uv = position.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    return out;
}

/// Returns the ray through a pixel of the frame.
///
/// A ray through a pixel is recovered from the camera's three axes and the half extent of the
/// frame, not from the projection matrix: recovering it from the matrix would mean inverting
/// it, and WGSL has no matrix inverse.
fn ray_through(uv: vec2<f32>) -> vec3<f32> {
    let offset = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    return normalize(
        uniforms.camera.forward
        + uniforms.camera.right * offset.x * uniforms.camera.half_extent.x
        + uniforms.camera.up * offset.y * uniforms.camera.half_extent.y,
    );
}

@fragment
fn fragment_main(in: Vertex) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(depth_buffer));
    let pixel = vec2<i32>(in.uv * size);
    let depth = textureLoad(depth_buffer, pixel, 0);
    // A depth of one is where nothing was drawn, because the depth buffer is cleared to the
    // far plane. Alpha is what marks it: a position of zero is a perfectly good position, so
    // the marker cannot be the position itself.
    if (depth >= 1.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let ray = ray_through(in.uv);
    let eye = uniforms.camera.position;
    let matrix = uniforms.projection.view_projection;
    let row_z = vec4<f32>(matrix[0][2], matrix[1][2], matrix[2][2], matrix[3][2]);
    let row_w = vec4<f32>(matrix[0][3], matrix[1][3], matrix[2][3], matrix[3][3]);
    let at_eye_z = dot(row_z.xyz, eye) + row_z.w;
    let at_eye_w = dot(row_w.xyz, eye) + row_w.w;
    let per_depth = dot(row_z.xyz - depth * row_w.xyz, ray);
    let t = (depth * at_eye_w - at_eye_z) / per_depth;
    return vec4<f32>(eye + ray * t, 1.0);
}
"#;
