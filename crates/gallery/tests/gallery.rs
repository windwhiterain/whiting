//! The gallery crate's tests, gathered in one binary: a separate file for each subject, one link.
//!
//! Each subject is named with the path it is read from, because the crate root of an
//! integration test owns the directory it sits in rather than a directory of its own name.

#[path = "gallery/shader_bindings.rs"]
mod shader_bindings;
#[path = "gallery/shader_provenance.rs"]
mod shader_provenance;
