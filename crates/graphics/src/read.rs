//! Reading a graphics document: borrowed views over the dynamic value tree.
//!
//! These views parse the tree on demand. They are a read-side convenience only; the
//! document remains the single representation and no typed copy of it is ever stored.

use definition::{Path, Value};

use crate::{GraphicsError, PASSES_FIELD, malformed};

/// Which kind of pass a [`PassView`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassKind {
    /// Draws geometry into the bound targets.
    Rasterize,
    /// Runs a compute shader over the bound resources.
    Compute,
}

impl PassKind {
    /// Returns the variant tag this kind is stored under.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Rasterize => "Rasterize",
            Self::Compute => "Compute",
        }
    }
}

/// A borrowed view of one pass inside a graphics document.
#[derive(Debug, Clone)]
pub struct PassView<'document> {
    kind: PassKind,
    index: usize,
    document: &'document Value,
}

impl<'document> PassView<'document> {
    /// Returns which kind of pass this is.
    pub const fn kind(&self) -> PassKind {
        self.kind
    }

    /// Returns the position of this pass in the pass list, which is its execution order.
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the path of this pass, for diagnostics and for binder errors.
    pub fn path(&self) -> Path {
        Path::root().key(PASSES_FIELD).index(self.index)
    }

    /// Borrows a string field of the pass.
    pub fn text(&self, field: &'static str) -> Result<&'document str, GraphicsError> {
        let path = self.path().key(field);
        match self.document.at(&path) {
            Some(Value::String(text)) => Ok(text),
            other => Err(malformed(&path, field, "a string", other)),
        }
    }

    /// Returns the pass label, which identifies the pass in errors and in output.
    pub fn label(&self) -> Result<&'document str, GraphicsError> {
        self.text("label")
    }

    /// Borrows a list field of the pass.
    pub fn list(&self, field: &'static str) -> Result<&'document [Value], GraphicsError> {
        let path = self.path().key(field);
        match self.document.at(&path) {
            Some(Value::List(elements)) => Ok(elements),
            other => Err(malformed(&path, field, "a list", other)),
        }
    }

    /// Borrows an integer field of the pass.
    pub fn integer(&self, field: &'static str) -> Result<i64, GraphicsError> {
        let path = self.path().key(field);
        match self.document.at(&path) {
            Some(Value::Int(number)) => Ok(*number),
            other => Err(malformed(&path, field, "an int", other)),
        }
    }

    /// Returns the names of every shader this pass binds, in binding order.
    ///
    /// A rasterize pass binds a vertex then a fragment shader; a compute pass binds one.
    pub fn shader_binders(&self) -> Result<Vec<&'document str>, GraphicsError> {
        Ok(match self.kind {
            PassKind::Rasterize => vec![self.text("vertex_shader")?, self.text("fragment_shader")?],
            PassKind::Compute => vec![self.text("shader")?],
        })
    }

    /// Returns every binder name this pass needs, in binding order.
    pub fn binder_names(&self) -> Result<Vec<&'document str>, GraphicsError> {
        self.shader_binders()
    }
}

/// Reads every pass out of a graphics document, in execution order.
pub fn passes(graphics: &Value) -> Result<Vec<PassView<'_>>, GraphicsError> {
    let root = Path::root();
    let path = root.key(PASSES_FIELD);
    let Some(Value::List(entries)) = graphics.at(&path) else {
        return Err(malformed(&path, PASSES_FIELD, "a list", graphics.at(&path)));
    };

    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| view(graphics, index, entry))
        .collect()
}

fn view<'document>(
    document: &'document Value,
    index: usize,
    entry: &Value,
) -> Result<PassView<'document>, GraphicsError> {
    let path = Path::root().key(PASSES_FIELD).index(index);
    let Value::Enum { tag, .. } = entry else {
        return Err(malformed(&path, "pass", "an enum", Some(entry)));
    };
    let kind = match tag.as_str() {
        "Rasterize" => PassKind::Rasterize,
        "Compute" => PassKind::Compute,
        other => {
            return Err(GraphicsError::UnknownPass {
                index,
                tag: other.to_owned(),
            });
        }
    };
    Ok(PassView {
        kind,
        index,
        document,
    })
}
