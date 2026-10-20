//! The graphics crate's tests, gathered in one binary: a separate file for each subject, one link.
//!
//! Each subject is named with the path it is read from, because the crate root of an
//! integration test owns the directory it sits in rather than a directory of its own name.

#[path = "graphics/pass_viewport.rs"]
mod pass_viewport;
#[path = "graphics/provenance.rs"]
mod provenance;
#[path = "graphics/texture_layers.rs"]
mod texture_layers;
