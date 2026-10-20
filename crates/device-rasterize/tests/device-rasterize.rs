//! The device-rasterize crate's tests, gathered in one binary: a separate file for each
//! subject, one link.
//!
//! Each subject is named with the path it is read from, because the crate root of an
//! integration test owns the directory it sits in rather than a directory of its own name.

#[path = "device-rasterize/clustered_shadow_render.rs"]
mod clustered_shadow_render;
#[path = "device-rasterize/environment_map_render.rs"]
mod environment_map_render;
#[path = "device-rasterize/multi_attachment_render.rs"]
mod multi_attachment_render;
#[path = "device-rasterize/r32float_attachment.rs"]
mod r32float_attachment;
#[path = "device-rasterize/support.rs"]
mod support;
#[path = "device-rasterize/texture_array_interface.rs"]
mod texture_array_interface;
#[path = "device-rasterize/texture_array_shape.rs"]
mod texture_array_shape;
#[path = "device-rasterize/texture_channel_render.rs"]
mod texture_channel_render;
#[path = "device-rasterize/typed_outputs.rs"]
mod typed_outputs;
