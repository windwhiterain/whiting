//! Which lights reach which parts of the view, and what is packed out of that.
//!
//! # What this is for
//!
//! One pass per light per group is what stops a scene holding thousands of lights: the pass
//! count is `O(lights * groups)`, and a thousand lights is a thousand shadow maps. A clustered
//! path replaces both with data. The view is divided into a grid of boxes, every box holds the
//! lights that reach it, and a shading pass that draws one box's worth of lights per fragment
//! costs one pass however many lights there are.
//!
//! # What is here and what is not
//!
//! The two halves are [`assignment`], which is the arithmetic, and [`buffers`], which is the
//! three packed runs and their schemas. The split is where the two concerns genuinely part:
//! nothing in the assignment knows how a record is laid out in bytes, and nothing in the
//! packing knows what a cluster is.
//!
//! This is a top level module rather than one inside the standard scene compiler, because
//! nothing in it is about the standard scene. It takes a [`assignment::ViewProjection`] rather
//! than the standard camera's `Projection`, and it owns the packed record schema rather than
//! borrowing the standard light's fields, so a compiler with a different camera or a different
//! light shape can use it. What stays in the standard scene is what is about the standard
//! scene: the camera itself, and the light document the author writes.
//!
//! # Invariants
//!
//! - The three buffers are read only. Nothing here writes a buffer, because there is no pass to
//!   write one and no dispatch to write it with, and `max_storage_buffers_per_shader_stage` is
//!   four: light records, cluster counts and cluster lists are all of the budget the clustered
//!   pass has, with one spare.
//! - Every light is listed in every cluster its volume reaches, up to the cap. A light that is
//!   not listed is a light no shading pass can find, so the cap is the only place one goes
//!   missing, and the cap reports what it missed.
//! - A kind that covers the whole view is listed nowhere and takes no cluster slot. A
//!   directional light and an environment light are in every box by definition, so listing them
//!   would spend the budget on lights the culling was never going to remove; they are in the
//!   record buffer and in no cluster list, which is what gives the shading pass its own path
//!   for them. A scene whose lights are only of those two kinds therefore has empty cluster
//!   lists, which is the correct answer rather than a missing one.

mod assignment;
mod buffers;

pub use assignment::{
    CLUSTERING, ClusterLight, Clustering, DEFAULT_DEPTH_EXTENTS, DEFAULT_HEIGHT_EXTENTS,
    DEFAULT_LIGHTS_PER_CLUSTER, DEFAULT_WIDTH_EXTENTS, DEPTH_EXTENTS, HEIGHT_EXTENTS,
    LIGHTS_PER_CLUSTER, Overflow, ViewProjection, WIDTH_EXTENTS, assign, contribution,
};
pub use buffers::{
    CLUSTER_COUNT_FIELD, CLUSTER_COUNTS_ENTRY, CLUSTER_INDICES_ENTRY, CLUSTER_LIGHT_FIELD,
    Clusters, LIGHT_COLOR_FIELD, LIGHT_PAGE_FIELD, LIGHT_POSITION_FIELD, LIGHT_RADIUS_FIELD,
    LIGHT_RECORDS_ENTRY, LightPool, MAX_TEXTURE_ARRAY_LAYERS, NO_PAGE, SHADOW_PAGE_HEIGHT,
    SHADOW_PAGE_WIDTH, SHADOW_PAGES, cluster_count_schema, cluster_list_schema,
    light_record_schema, pool_notes, read_light_pool,
};
