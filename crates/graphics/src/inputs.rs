//! Checking a pass's inputs against the interface its shader declares.
//!
//! A shader is the only thing that knows what it reads, so it carries the schema of
//! that data. The pass is what supplies it. This module is the one place that compares
//! the two, so a shader author writes a single flat schema of a `uniform` and a
//! `binders` block rather than two schemas and a rule about how they combine.
//!
//! # The interface document
//!
//! A shader's interface is a struct of a `uniform` and a `binders` block. The pass
//! writes its binding points as an open dict, because the set of points is the pass's
//! own business, while the interface writes them as a struct, because a shader needs to
//! name each one and a dict carries a single entry schema. This module is where that
//! difference is bridged, and it is the only place it is bridged:
//!
//! ```text
//! { uniform: <pass uniform>, binders: { <point>: <declaration>, .. } }
//! ```

use definition::{Schema, SchemaKind, Value};

use crate::GraphicsError;
use crate::binders::Declaration;
use crate::read::PassView;

/// Field of a shader interface holding the uniform data it reads.
const UNIFORM: &str = "uniform";

/// Field of a shader interface holding the binding points it reads.
const BINDERS: &str = "binders";

/// Verifies that a pass's `uniform` and `binders` satisfy the interface its shader
/// declares, and that every binder declaration resolves to an entry of the declared
/// kind.
///
/// # Invariants
///
/// - A shader with no `schema` is unchecked, so an instance stays editable before a
///   shader has declared anything. This is the only place a missing schema is allowed,
///   and it is allowed on purpose rather than as an oversight.
/// - A failure is a binding error rather than a validation error: the two documents
///   each validate against their own schema and still disagree with each other.
pub fn verify_pass_inputs(pass: &PassView<'_>, instance: &Value) -> Result<(), GraphicsError> {
    let label = pass.label().unwrap_or("<unlabelled>").to_owned();
    let declarations = pass.binders()?;
    for declaration in &declarations {
        declaration
            .bind(instance)
            .map_err(|error| name_pass(&label, error))?;
    }

    let Some(interface) = shader_interface(pass, instance)? else {
        return Ok(());
    };

    let inputs = Value::structure([
        (UNIFORM.to_owned(), pass.uniform()?.clone()),
        (BINDERS.to_owned(), bindings_value(&declarations)),
    ]);
    interface
        .validate(&inputs)
        .map_err(|error| GraphicsError::UnverifiedInputs {
            label: label.clone(),
            shader: pass.shader_binder().unwrap_or("<unlabelled>").to_owned(),
            reason: error.to_string(),
        })
}

/// Returns the schema of the uniform a pass supplies, as its shader declares it.
///
/// The interface is what a shader says it reads, so it is also the layout the uniform is
/// uploaded with: a renderer that packed a uniform without it would put the second field
/// where a tight packing leaves room rather than where the shader's own alignment puts it.
/// A shader that declares no interface has no declared layout, and `None` says so.
pub fn uniform_schema(
    pass: &PassView<'_>,
    instance: &Value,
) -> Result<Option<Schema>, GraphicsError> {
    let Some(interface) = shader_interface(pass, instance)? else {
        return Ok(None);
    };
    let SchemaKind::Struct { fields } = &interface.kind else {
        return Ok(None);
    };
    Ok(fields.get(UNIFORM).cloned())
}

/// Returns the interface the pass's shader declares, or `None` when it declares none.
pub fn shader_interface(
    pass: &PassView<'_>,
    instance: &Value,
) -> Result<Option<Schema>, GraphicsError> {
    let label = pass.label().unwrap_or("<unlabelled>").to_owned();
    let shader = pass
        .shader_binder()
        .map_err(|error| name_pass(&label, error))?;
    let binder = crate::resolve_for_pass(&label, instance, shader)?;
    let Some(declared) = binder.schema() else {
        return Ok(None);
    };
    declared
        .map(Some)
        .map_err(|error| GraphicsError::UnreadableSchema {
            label,
            shader: shader.to_owned(),
            reason: error.to_string(),
        })
}

/// Builds the value a shader interface is checked against, from the pass declarations.
fn bindings_value(declarations: &[Declaration<'_>]) -> Value {
    Value::structure(
        declarations
            .iter()
            .map(|declaration| (declaration.point().to_owned(), declaration.value().clone())),
    )
}

/// Restates an error so that it names the pass it came from.
fn name_pass(label: &str, error: GraphicsError) -> GraphicsError {
    match error {
        GraphicsError::Binding {
            binder_name,
            reason,
            ..
        } => GraphicsError::Binding {
            label: label.to_owned(),
            binder_name,
            reason,
        },
        other => other,
    }
}
