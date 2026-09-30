//! Checking that a shader reads its uniform where its own interface says it is.
//!
//! # Why this check exists
//!
//! A pass supplies a uniform and a shader reads it, and the layout the two agree on is the
//! schema the shader declares: a schema sorts its fields by name, and the packer writes
//! them in that order at the alignment a backend gives them. A shader author who writes the
//! fields of their WGSL struct in another order, or with another type, gets no error from
//! anywhere: the pipeline is valid, the bind group matches, and the shader reads the wrong
//! bytes.
//!
//! The backend can be asked. A shader module parses back into the intermediate
//! representation the backend itself uses, and a struct in it carries the name and the
//! offset of every member. Comparing that against the schema turns a silent misread into a
//! render error that names the field and both offsets.
//!
//! # Invariants
//!
//! The comparison is exact and it recurses: every field of the schema must be a member of
//! the shader's struct under the same name, at the same offset, and every field that is a
//! struct must be a struct in the shader too. A shader that declares nothing at the uniform
//! binding is not checked, because there is nothing to compare against.

use definition::Schema;
use graphics::{Binder, layout::UniformField};
use wgpu::naga;

use super::WgpuRenderer;
use crate::render::RenderError;

/// The binding a pass's uniform is at, for every shader.
pub const UNIFORM_BINDING: u32 = 0;

/// A member of a shader's uniform struct, as the shader declares it.
struct Member {
    /// The name the member is declared under.
    name: String,
    /// The offset it starts at, according to the backend.
    offset: usize,
    /// The type it is, for recursing into a struct.
    ty: naga::Handle<naga::Type>,
}

impl WgpuRenderer {
    /// Checks that a shader's uniform struct reads the layout its interface declares.
    ///
    /// The shader is parsed rather than taken on trust, so this costs a parse per pass per
    /// render. A layout disagreement is a defect in the shader rather than in the frame, so
    /// the cost buys an error where there would otherwise be a wrong image.
    pub(super) fn check_uniform_layout(
        &self,
        binder: &Binder<'_>,
        schema: &Schema,
    ) -> Result<(), RenderError> {
        let (path, source) = self.shader_source(binder)?;
        let declared = graphics::layout::uniform_layout(schema).map_err(|error| {
            RenderError::Backend(format!(
                "the uniform interface `{path}` declares has no layout: {error}"
            ))
        })?;
        let module = naga::front::wgsl::parse_str(&source).map_err(|error| {
            RenderError::Backend(format!("`{path}` does not parse as wgsl: {error}"))
        })?;
        let found = uniform_members(&module);
        if found.is_empty() {
            return Ok(());
        }
        compare(&path, &declared, &found, &module)
    }
}

/// Returns the members of the uniform struct a shader declares.
fn uniform_members(module: &naga::Module) -> Vec<Member> {
    let Some((_, variable)) = module.global_variables.iter().find(|(_, variable)| {
        variable
            .binding
            .is_some_and(|binding| binding.group == 0 && binding.binding == UNIFORM_BINDING)
    }) else {
        return Vec::new();
    };
    members_of(module, variable.ty)
}

/// Returns the members of a type, or nothing when the type is not a struct.
fn members_of(module: &naga::Module, ty: naga::Handle<naga::Type>) -> Vec<Member> {
    let naga::TypeInner::Struct { members, .. } = &module.types[ty].inner else {
        return Vec::new();
    };
    members
        .iter()
        .map(|member| Member {
            name: member.name.clone().unwrap_or_default(),
            offset: member.offset as usize,
            ty: member.ty,
        })
        .collect()
}

/// Compares the layout a schema declares against the one a shader declares.
fn compare(
    path: &str,
    declared: &[UniformField],
    found: &[Member],
    module: &naga::Module,
) -> Result<(), RenderError> {
    if declared.len() != found.len() {
        let declared_names: Vec<&str> = declared.iter().map(|field| field.name.as_str()).collect();
        let found_names: Vec<&str> = found.iter().map(|member| member.name.as_str()).collect();
        return Err(RenderError::Backend(format!(
            "`{path}` does not read its uniform the way its interface declares it: the \
             interface has {declared_names:?} and the shader declares {found_names:?}"
        )));
    }

    for (field, member) in declared.iter().zip(found) {
        if field.name != member.name {
            return Err(disagree(
                path,
                &field.name,
                field.offset,
                &member.name,
                member.offset,
            ));
        }
        if field.offset != member.offset {
            return Err(disagree(
                path,
                &field.name,
                field.offset,
                &member.name,
                member.offset,
            ));
        }
        if !field.fields.is_empty() {
            let inner = members_of(module, member.ty);
            if inner.is_empty() {
                return Err(RenderError::Backend(format!(
                    "`{path}` reads `{}` as one value, but the interface declares it as a \
                     struct of {} fields",
                    field.name,
                    field.fields.len()
                )));
            }
            // A nested struct's members are compared at offsets relative to that struct,
            // which is what the layout reports for them and what the backend reports too.
            compare(path, &field.fields, &inner, module)?;
        }
    }
    Ok(())
}

/// Builds the error a disagreement between two layouts is reported as.
fn disagree(
    path: &str,
    declared_name: &str,
    declared_offset: usize,
    found_name: &str,
    found_offset: usize,
) -> RenderError {
    RenderError::Backend(format!(
        "`{path}` does not read its uniform the way its interface declares it: the \
         interface declares `{declared_name}` at offset {declared_offset}, and the shader \
         declares `{found_name}` at offset {found_offset}. A schema sorts its fields by \
         name, so the shader declares them in that order too."
    ))
}
