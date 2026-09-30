//! Binders: the internal indirection from a pass to a named instance entry.
//!
//! A pass never carries instance data. It carries binder *names*, which are looked up in
//! the instance dict when the pass is built. Because the instance schema is authored
//! independently of those names, a missing or malformed entry is a binding error rather
//! than a schema validation failure, and is reported as such.

use definition::Value;

use crate::GraphicsError;

/// Which kind of resource a [`Binder`] names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinderKind {
    /// A GPU buffer.
    Buffer,
    /// A GPU texture.
    Texture,
    /// Uniform data.
    Uniform,
    /// A shader, addressed by path.
    Shader,
}

impl BinderKind {
    /// Returns the variant tag this kind is stored under.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Buffer => "Buffer",
            Self::Texture => "Texture",
            Self::Uniform => "Uniform",
            Self::Shader => "Shader",
        }
    }
}

/// A borrowed view of one bound instance entry.
#[derive(Debug, Clone)]
pub struct Binder<'instance> {
    name: &'instance str,
    value: &'instance Value,
}

impl<'instance> Binder<'instance> {
    /// Returns the binder name the pass referenced.
    pub const fn name(&self) -> &'instance str {
        self.name
    }

    /// Borrows the bound value.
    pub const fn value(&self) -> &'instance Value {
        self.value
    }

    /// Returns which kind of resource this is, or `None` when the entry is not a typed
    /// binder document.
    pub fn kind(&self) -> Option<BinderKind> {
        match self.value.variant_tag()? {
            "Resource" => match self
                .value
                .variant_fields()?
                .get("resource")
                .and_then(Value::variant_tag)
            {
                Some("Buffer") => Some(BinderKind::Buffer),
                Some("Texture") => Some(BinderKind::Texture),
                _ => None,
            },
            "Uniform" => Some(BinderKind::Uniform),
            "Shader" => Some(BinderKind::Shader),
            _ => None,
        }
    }

    /// Borrows a string field of a typed binder document.
    pub fn text(&self, field: &'static str) -> Option<&'instance str> {
        self.value.variant_fields()?.get(field)?.as_str()
    }
}

/// Resolves a binder name against an instance document.
///
/// # Why this is not a validation error
///
/// The instance schema is authored rather than derived from the binder names, so a
/// graphics document and its instance can each validate against their own schema and
/// still fail to bind. That outcome is reported as [`GraphicsError::Binding`], which is
/// deliberately a distinct variant from [`GraphicsError::Malformed`].
pub fn resolve_binder<'instance>(
    instance: &'instance Value,
    binder_name: &'instance str,
) -> Result<Binder<'instance>, GraphicsError> {
    let Some(entries) = instance.as_dict() else {
        return Err(GraphicsError::Binding {
            label: "<instance>".to_owned(),
            binder_name: binder_name.to_owned(),
            reason: "the instance is not a dict",
        });
    };
    let Some(value) = entries.get(binder_name) else {
        return Err(GraphicsError::Binding {
            label: "<instance>".to_owned(),
            binder_name: binder_name.to_owned(),
            reason: "no instance entry carries that name",
        });
    };
    Ok(Binder {
        name: binder_name,
        value,
    })
}

/// Resolves a binder name on behalf of a named pass, so the error names the pass.
pub fn resolve_for_pass<'instance>(
    pass_label: &str,
    instance: &'instance Value,
    binder_name: &'instance str,
) -> Result<Binder<'instance>, GraphicsError> {
    resolve_binder(instance, binder_name).map_err(|error| match error {
        GraphicsError::Binding {
            binder_name,
            reason,
            ..
        } => GraphicsError::Binding {
            label: pass_label.to_owned(),
            binder_name,
            reason,
        },
        other => other,
    })
}
