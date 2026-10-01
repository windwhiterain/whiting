//! The interface check sees the difference between a flat texture and an array of them.
//!
//! # What this proves, and what it does not
//!
//! `texture_depth_2d` and `texture_depth_2d_array` are two WGSL types, and a `textureLoad`
//! on an array takes the layer as an argument a flat texture has nowhere to put. Before the
//! shape was compared, both collapsed to "a texture" on both sides of the comparison, so a
//! shader declaring an array where the pass bound a flat texture passed the check and the
//! failure arrived from the backend as a message about a view dimension naming neither
//! document.
//!
//! The comparison is a module and a list of layout entries, so it needs no adapter, and
//! these run on a machine with no gpu. What is **not** proved here is that a pipeline is
//! built from the entries: that is [`WgpuRenderer`]'s work and it needs a device.
//!
//! The ordinary case is the one that matters most, because every document and every shader
//! in the project today is a flat two dimensional texture. A flat pair must report nothing,
//! in both directions, or the check has broken every shader rather than guarded them.
//!
//! [`WgpuRenderer`]: ../../src/wgpu_renderer/mod.rs

#![cfg(feature = "wgpu")]

use wgpu::naga;
use whiting_device::check_resources;

/// The binding the texture is checked at. The uniform is binding zero, so the first bound
/// resource is one.
const BINDING: u32 = 1;

/// A module that reads one flat two dimensional depth texture.
const FLAT: &str = "@group(0) @binding(1) var page: texture_depth_2d;";

/// A module that reads an array of two dimensional depth textures.
const ARRAY: &str = "@group(0) @binding(1) var pages: texture_depth_2d_array;";

#[test]
fn a_flat_texture_and_a_flat_shader_agree() {
    check_resources(
        &module(FLAT),
        &[texture_entry(wgpu::TextureViewDimension::D2)],
        "shade",
    )
    .expect("this is every texture binding in the project that is not a pool");
}

#[test]
fn a_shader_reading_an_array_where_the_pass_binds_a_flat_texture_is_refused() {
    let error = check_resources(
        &module(ARRAY),
        &[texture_entry(wgpu::TextureViewDimension::D2)],
        "shade",
    )
    .expect_err("a shape the pass does not bind is a disagreement, not a later backend error");
    let message = error.to_string();
    for expected in [
        "shade",
        ARRAY_VARIABLE,
        "an array of two dimensional textures",
        "a two dimensional texture",
    ] {
        assert!(
            message.contains(expected),
            "the refusal names {expected:?}, so a reader knows which pass and which side: \
             {message}"
        );
    }
}

#[test]
fn a_shader_reading_a_flat_texture_where_the_pass_binds_an_array_is_refused_too() {
    let error = check_resources(
        &module(FLAT),
        &[texture_entry(wgpu::TextureViewDimension::D2Array)],
        "shade",
    )
    .expect_err("the disagreement is reported in whichever direction it is written");
    let message = error.to_string();
    assert!(
        message.contains("a two dimensional texture")
            && message.contains("an array of two dimensional textures"),
        "both sides are named: {message}"
    );
}

#[test]
fn an_array_and_an_array_agree() {
    check_resources(
        &module(ARRAY),
        &[texture_entry(wgpu::TextureViewDimension::D2Array)],
        "shade",
    )
    .expect("a pool read as a pool is the case the page scheme exists for");
}

/// The name the shader gives the array, which an error has to quote back.
const ARRAY_VARIABLE: &str = "pages";

/// Parses a module, which is pure work and needs nothing but the source.
fn module(source: &str) -> naga::Module {
    naga::front::wgsl::parse_str(source).expect("the module parses")
}

/// Builds the one layout entry a depth texture is bound at.
fn texture_entry(view_dimension: wgpu::TextureViewDimension) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: BINDING,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Depth,
            view_dimension,
            multisampled: false,
        },
        count: None,
    }
}
