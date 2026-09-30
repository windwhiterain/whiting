//! Reading a graphics document: borrowed views over the dynamic value tree.
//!
//! These views parse the tree on demand. They are a read-side convenience only; the
//! document remains the single representation and no typed copy of it is ever stored.

use definition::{Path, Value};

use crate::binders::Declaration;
use crate::pass::{BINDERS, CLEAR_COLOR, ENTRY_POINT, LABEL, SHADER, TARGETS, UNIFORM, WORKGROUP};
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
        self.text(LABEL)
    }

    /// Borrows a list field of the pass.
    pub fn list(&self, field: &'static str) -> Result<&'document [Value], GraphicsError> {
        let path = self.path().key(field);
        match self.document.at(&path) {
            Some(Value::List(elements)) => Ok(elements),
            other => Err(malformed(&path, field, "a list", other)),
        }
    }

    /// Borrows the uniform data of the pass.
    pub fn uniform(&self) -> Result<&'document Value, GraphicsError> {
        let path = self.path().key(UNIFORM);
        self.document
            .at(&path)
            .ok_or_else(|| malformed(&path, UNIFORM, "a value", None))
    }

    /// Returns the name of the instance entry holding this pass's shader.
    pub fn shader_binder(&self) -> Result<&'document str, GraphicsError> {
        self.text(SHADER)
    }

    /// Borrows a field of the pass that may be absent.
    ///
    /// This is how an optional field is read: the field belongs to the variant's struct, so
    /// it is borrowed from the variant rather than from the pass, and a field that is not
    /// there is `None` rather than an error. A field the schema requires is still read with
    /// [`PassView::text`] or [`PassView::list`], which report a missing one.
    pub fn optional(&self, field: &'static str) -> Option<&'document Value> {
        self.document.at(&self.path().key(field))
    }

    /// Returns every binder declaration of the pass, keyed by binding point.
    pub fn binders(&self) -> Result<Vec<Declaration<'document>>, GraphicsError> {
        let path = self.path().key(BINDERS);
        let Some(Value::Dict(entries)) = self.document.at(&path) else {
            return Err(malformed(&path, BINDERS, "a dict", self.document.at(&path)));
        };
        Ok(entries
            .iter()
            .map(|(point, value)| Declaration {
                path: path.key(point),
                point,
                value,
            })
            .collect())
    }

    /// Returns the names of every instance entry this pass needs, in binding order.
    ///
    /// The shader is listed first, then the binding points in the order the document
    /// writes them, which is the order a renderer binds them in.
    pub fn binder_names(&self) -> Result<Vec<&'document str>, GraphicsError> {
        let mut names = vec![self.shader_binder()?];
        names.extend(
            self.binders()?
                .iter()
                .map(|declaration| declaration.source())
                .collect::<Result<Vec<&str>, GraphicsError>>()?,
        );
        Ok(names)
    }

    /// Returns the colour target formats of a rasterize pass, in attachment order.
    pub fn targets(&self) -> Result<&'document [Value], GraphicsError> {
        self.list(TARGETS)
    }

    /// Returns the clear colour of a rasterize pass.
    pub fn clear_color(&self) -> Result<&'document [Value], GraphicsError> {
        self.list(CLEAR_COLOR)
    }

    /// Returns the workgroup size and dispatch count of a compute pass.
    pub fn workgroup(&self) -> Result<&'document [Value], GraphicsError> {
        self.list(WORKGROUP)
    }

    /// Returns the entry point a compute pass invokes.
    pub fn entry_point(&self) -> Result<&'document str, GraphicsError> {
        self.text(ENTRY_POINT)
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
