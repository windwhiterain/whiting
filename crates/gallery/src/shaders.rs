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
//! - **The bindings.** The uniform is binding zero, and every bound resource follows it in
//!   name order. A texture a pass renders *into* is an attachment and is not bound, and
//!   neither are the two vertex buffers, so the numbering is over the rest. The lighting
//!   stage's numbers are written out in [`lighting_bindings`] rather than counted in prose,
//!   and the post stage binds the shaded image at one and the depth at two.
//!
//! # The lighting stage is clustered
//!
//! It used to be one pass per light, which meant the pass count grew with the scene's lamp
//! count and the uniform had to hold a light. It is now one pass per group: a fragment
//! resolves which cluster of the view it is in from its own position, reads that cluster's
//! light list out of three storage buffers, and loops over it. Everything a per light pass
//! carried in its uniform is a field of a light's record now, which is what makes a thousand
//! lights a matter of buffer size rather than of passes.

use definition::{Schema, SchemaKind, Value};

use scene::standard::{
    Stage, depth_pass_schema, emission_pass_schema, light_pass_schema, post_pass_schema,
    shadow_pass_schema,
};

use super::kit;

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

/// The uniform the clustered lighting stage reads.
///
/// The fields are in the order the interface schema sorts them into, which is by name:
/// `cluster_grid`, `frame_size`, `projection`, `time`. The order is not cosmetic — a three
/// component vector is padded to sixteen bytes, so a field written second starts at sixteen
/// either way, and a struct written in any other order puts the wrong thing there.
///
/// **Nothing here names a light.** The four per light fields this struct used to carry — the
/// light's position, its colour, its own projection and the size of its shadow map's texel —
/// are a field of the light's *record*, read from a storage buffer per light inside the
/// loop. That is the whole change: a uniform is sixteen kilobytes, which cannot hold a
/// thousand light records however they are packed.
const LIGHTING_UNIFORM: &str = r#"
struct Projection {
    position: vec3<f32>,
    view_projection: mat4x4<f32>,
};

struct Uniforms {
    cluster_grid: vec4<f32>,
    frame_size: vec2<f32>,
    projection: Projection,
    time: f32,
};

/// One light, as the clustered loop reads it out of the record buffer.
///
/// The offsets are the ones the compiler packs, and they need no padding written here: a
/// three component vector is sixteen byte aligned and twelve bytes long, so the single
/// `f32` after each of them is exactly the padding that puts the next field on its own
/// boundary. `color` at 0, `page` at 12, `position` at 16, `radius` at 28, and the matrix
/// at 32 — a whole record of ninety six bytes, a whole number of sixteen byte blocks.
struct LightRecord {
    color: vec3<f32>,
    page: f32,
    position: vec3<f32>,
    radius: f32,
    shadow_projection: mat4x4<f32>,
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
pub fn emission(vertex_inputs: &str, emission: &str, helpers: &str) -> String {
    format!(
        r#"{PLACEMENT_UNIFORM}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var depth_buffer: texture_depth_2d;

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
pub fn shadowing(vertex_inputs: &str) -> String {
    format!(
        r#"{PLACEMENT_UNIFORM}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertex_main(
{vertex_inputs}) -> @builtin(position) vec4<f32> {{
    let model = mat4x4<f32>(model0, model1, model2, model3);
    return uniforms.projection.view_projection * model * vec4<f32>(position, 1.0);
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
pub fn lighting(vertex_inputs: &str, surface: &str, helpers: &str) -> String {
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
pub fn lighting_with_normal(
    vertex_inputs: &str,
    displace: &str,
    surface: &str,
    helpers: &str,
) -> String {
    lighting_stage(vertex_inputs, displace, surface, helpers, lighting_bindings(false))
}

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
/// `depth_buffer` is in the list and declares nothing. The pass binds it because a fragment
/// is tested against it, and the fragment never reads it, so it takes a number and no line of
/// WGSL. Leaving the number out instead would put every binding after it one lower than the
/// device's, which is a frame of the wrong numbers rather than a failure.
///
/// An image is a texture **and** the sampler that reads it, at the position immediately after
/// its own, so adding one shifts everything below it. That is why the declarations are
/// generated from a flag rather than spliced into the source with a string replacement.
fn lighting_bindings(image: bool) -> String {
    // Each entry is the access an address space needs written on `var` itself, and the type
    // that follows the name. A storage buffer is the one that needs the first, and needing
    // it is why a plain `var` is not enough for these three.
    let mut resources: Vec<(&str, &str, &str)> = vec![
        ("cluster_light_counts", "<storage, read>", "array<f32>"),
        ("cluster_light_indices", "<storage, read>", "array<f32>"),
        // The depth buffer is next in name order, so it takes binding three. The pass binds
        // it because a fragment is tested against it and no fragment here reads it, so it has
        // no line of WGSL: the empty name is the number it consumes and nothing else.
        ("", "", ""),
    ];
    let mut number = 1;
    if image {
        resources.push(("image", "", "texture_2d<f32>"));
        resources.push(("image_sampler", "", "sampler"));
    }
    resources.push(("light_records", "<storage, read>", "array<LightRecord>"));
    resources.push(("shadow_pool", "", "texture_depth_2d_array"));

    let mut declarations = String::new();
    for (source, access, ty) in resources {
        if !source.is_empty() {
            declarations.push_str(&format!(
                "@group(0) @binding({number}) var{access} {source}: {ty};\n"
            ));
        }
        number += 1;
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
fn lighting_stage(
    vertex_inputs: &str,
    displace: &str,
    surface: &str,
    helpers: &str,
    bindings: String,
) -> String {
    format!(
        r#"{LIGHTING_UNIFORM}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
{bindings}
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
/// A light the pool gave **no page** is lit unshadowed, and that is the design rather than a
/// degraded mode: the pool is a scene's decision about how many shadows it wants to render
/// once, a light that lost the ranking is a light the scene chose to approximate, and the
/// alternative — giving it a neighbour's page — would be a wrong shadow rather than a missing
/// one. The record's page is minus one for exactly that case, and a page index of zero is a
/// page, so the two can be told apart by a single comparison.
///
/// Where there is a page, the surface's world position is carried through **the light's own**
/// projection and its depth there is compared against what that page recorded. A projection is
/// per light because a point light looks at the scene from where it is, and reading the page
/// at the fragment's own screen coordinate would be a different lookup and not a shadow.
///
/// The taps are taken by `textureLoad` rather than through a sampler because a binding
/// carries a texture and nothing else: there is no sampler to filter with, and a depth
/// compare has to be done by hand in any case.
fn shadowed(light: LightRecord, world_position: vec3<f32>, normal: vec3<f32>) -> f32 {{
    let page = i32(light.page);
    if page < 0 {{
        return 1.0;
    }}
    // A surface is pushed away from the page by a constant in world space. Displacing along
    // the normal rather than along the light is what keeps the shadow attached: a surface is
    // pushed away from the map by the same amount everywhere, so a slope and a flat both
    // move by one page texel and neither detaches.
    let lookup = world_position + normal * 1.6;
    let from_light = light.shadow_projection * vec4<f32>(lookup, 1.0);
    // A point behind the light has no shadow, and dividing by a negative w would fold it
    // back into the page from behind.
    if from_light.w <= 0.0 {{
        return 1.0;
    }}
    let ndc = from_light.xyz / from_light.w;
    // Clip space y runs up and texture space runs down.
    let uv = ndc.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) {{
        return 1.0;
    }}
    // The page's own size, not one carried in the uniform: the pool is a schema field, so a
    // scene that changed the page size changed this with it and there is nothing to keep in
    // step by hand.
    let size = vec2<f32>(textureDimensions(shadow_pool));
    // A surface is its own depth and the recorded value is the depth of whatever was
    // nearest the light, so the comparison is biased. The bias scales with how obliquely
    // the light strikes the surface, because a surface's own rasterised faces disagree
    // with its true depth by more the further they lean away from the light, and one
    // constant is either too small for the leaning surfaces or detaches the shadow from
    // the flat ones. The normal offset above has already taken the large part of that
    // error, so what is left here is small.
    let to_light = normalize(light.position - world_position);
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
    return out;
}}

@fragment
fn fragment_main(in: Vertex) -> @location(0) vec4<f32> {{
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
    // reflects about four percent at normal incidence, rising to one at grazing angles,
    // which is the Fresnel term below.
    let f0 = mix(vec3<f32>(0.04), in.tint, metallic);
    let fresnel = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - normal_dot_view, 5.0);
    // A rough surface spreads its highlight, so the exponent of the lobe follows the
    // roughness rather than being a constant.
    let exponent = mix(512.0, 4.0, roughness);
    let sharpness = mix(0.02, 1.0, (1.0 - roughness) * (1.0 - roughness));

    // The cluster this fragment is in, its light count, and the start of its row. The row is
    // the cap wide rather than the count, so no prefix sum of the counts is needed to find
    // it: a cluster's lights are the first `count` of a row that always starts in the same
    // place.
    let cluster = cluster_of(in.position);
    let count = u32(cluster_light_counts[cluster]);
    let row = cluster * u32(uniforms.cluster_grid.w);

    var color = vec3<f32>(0.0);
    for (var slot = 0u; slot < count; slot = slot + 1u) {{
        let light = light_records[u32(cluster_light_indices[row + slot])];
        let to_light = light.position - in.world_position;
        let light_range = max(length(to_light), 0.0001);
        let direction = to_light / light_range;
        let lit = shadowed(light, in.world_position, normal);

        // A point light falls off with the square of its distance. The constants are the
        // gentlest that still read as a falloff over a scene a couple of hundred units
        // across: a steeper one puts the light's own volume inside the scene, and every
        // surface near the light then reads as white no matter what its colour is.
        let attenuation = 1.0 / (1.0 + 0.0035 * light_range + 0.00002 * light_range * light_range);
        let radiance = light.color * attenuation * lit;

        let lambert = max(dot(normal, direction), 0.0);
        let half_vector = normalize(direction + to_view);
        let specular = pow(max(dot(normal, half_vector), 0.0), exponent) * sharpness;

        color = color + diffuse_color * lambert * radiance;
        color = color + fresnel * specular * radiance * 3.0;
    }}

    // A hemisphere of ambient light, brighter from above, which stands in for the
    // environment a surface would otherwise have nothing to reflect. It is added **once**,
    // whatever the cluster holds, because it is not one light's contribution: while the
    // stage ran once per light this term was inside the loop and a scene of three lights
    // added it three times, which is a bug the clustered pass happens to fix.
    let sky = mix(
        vec3<f32>(0.014, 0.017, 0.026),
        vec3<f32>(0.055, 0.072, 0.110),
        normal.y * 0.5 + 0.5,
    );
    let ambient = sky * mix(vec3<f32>(1.0), in.tint, metallic);
    color = color + ambient * diffuse_color;
    // What the surface emits is not added here either, for the same reason: the lighting
    // stage runs once, so a term added here is a term added once, and emission is its own
    // stage.
{surface}
    return vec4<f32>(color, 1.0);
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
pub fn textured(vertex_inputs: &str, helpers: &str) -> String {
    lighting_stage(
        vertex_inputs,
        "",
        "let pattern = textureSample(image, image_sampler, in.texture);\n    \
         color = color * (0.25 + 1.5 * pattern.rgb);",
        helpers,
        lighting_bindings(true),
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
pub fn post(grade: &str, helpers: &str) -> String {
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
pub fn mesh_shaders(surface: &str, helpers: &str) -> Vec<Value> {
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
pub fn mesh_shaders_with_normal(displace: &str, surface: &str, helpers: &str) -> Vec<Value> {
    let point = kit::shaded_point_schema();
    let inputs = crate::attributes::gallery_vertex_inputs(&point);
    vec![
        kit::shader(
            Stage::Shadowing,
            "shadowing",
            &shadowing(&inputs),
            shadow_pass_schema(),
            point.clone(),
            kit::gallery_instance_schema(),
            kit::gallery_custom_schema(),
        ),
        kit::shader(
            Stage::Depthing,
            "depthing",
            &depthing(&inputs),
            depth_pass_schema(),
            point.clone(),
            kit::gallery_instance_schema(),
            kit::gallery_custom_schema(),
        ),
        kit::shader(
            Stage::Lighting,
            "lighting",
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
            "emission",
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
pub fn emission_shader(label: &str, body: &str, helpers: &str) -> Value {
    let point = kit::shaded_point_schema();
    let inputs = crate::attributes::gallery_vertex_inputs(&point);
    kit::shader(
        Stage::Emission,
        label,
        &emission(&inputs, body, helpers),
        emission_pass_schema(),
        point,
        kit::gallery_instance_schema(),
        kit::gallery_custom_schema(),
    )
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
pub fn displaced_lighting_shader(
    label: &str,
    displace: &str,
    surface: &str,
    helpers: &str,
) -> Value {
    let point = kit::shaded_point_schema();
    let inputs = crate::attributes::gallery_vertex_inputs(&point);
    kit::shader(
        Stage::Lighting,
        label,
        &lighting_with_normal(&inputs, displace, surface, helpers),
        light_pass_schema(),
        point,
        kit::gallery_instance_schema(),
        kit::gallery_custom_schema(),
    )
}

/// Builds the post shader a scene declares, which grades the frame the scene produced.
///
/// `helpers` is handed to [`post`] unchanged: the WGSL the grade's arithmetic is written
/// against, declared at module level.
pub fn post_shader(grade: &str, helpers: &str) -> Value {
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
