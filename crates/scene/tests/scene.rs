//! The scene crate's tests, gathered in one binary: a separate file for each subject, one link.
//!
//! Each subject is named with the path it is read from, because the crate root of an
//! integration test owns the directory it sits in rather than a directory of its own name.

#[path = "scene/camera_notes.rs"]
mod camera_notes;
#[path = "scene/clustered_lights.rs"]
mod clustered_lights;
#[path = "scene/cycles_camera_forward.rs"]
mod cycles_camera_forward;
#[path = "scene/directional_lights.rs"]
mod directional_lights;
#[path = "scene/environment_light.rs"]
mod environment_light;
#[path = "scene/exposure.rs"]
mod exposure;
#[path = "scene/grade_channel.rs"]
mod grade_channel;
#[path = "scene/image_channel.rs"]
mod image_channel;
#[path = "scene/output_declarations.rs"]
mod output_declarations;
#[path = "scene/shadow_pages.rs"]
mod shadow_pages;
#[path = "scene/spot_lights.rs"]
mod spot_lights;
#[path = "scene/support.rs"]
mod support;
#[path = "scene/virtual_shadow_maps.rs"]
mod virtual_shadow_maps;
#[path = "scene/wgsl_validation.rs"]
mod wgsl_validation;
