//! Reading a graphics document: borrowed views over the dynamic value tree.
//!
//! These views parse the tree on demand. They are a read-side convenience only; the
//! document remains the single representation and no typed copy of it is ever stored.

use definition::{FieldMap, Path, Resolved, Value};

use crate::binders::Declaration;
use crate::output::{LOCATION_FIELD, POINT_FIELD, TYPE_FIELD};
use crate::pass::{BINDERS, CLEAR_COLOR, ENTRY_POINT, LABEL, SHADER, TARGETS, UNIFORM, WORKGROUP};
use crate::{GraphicsError, OUTPUTS_FIELD, PASSES_FIELD, malformed};

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

    /// Borrows the value at a path inside this pass.
    ///
    /// A pass is a struct of declarations, so a path into one always lands on a value the document
    /// owns and the borrow is a real one. A value that had to be decoded out of a packed run is
    /// not a pass field, so it is reported as absent rather than borrowed — which is what keeps
    /// every reader below returning `&'document` without a lifetime of its own.
    fn at(&self, path: &Path) -> Option<&'document Value> {
        match self.document.at(path) {
            Some(Resolved::Borrowed(value)) => Some(value),
            Some(Resolved::Owned(_)) | None => None,
        }
    }

    /// Borrows a string field of the pass.
    pub fn text(&self, field: &'static str) -> Result<&'document str, GraphicsError> {
        let path = self.path().key(field);
        match self.at(&path) {
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
        match self.at(&path) {
            Some(Value::List(elements)) => Ok(elements),
            other => Err(malformed(&path, field, "a list", other)),
        }
    }

    /// Borrows the uniform data of the pass.
    pub fn uniform(&self) -> Result<&'document Value, GraphicsError> {
        let path = self.path().key(UNIFORM);
        self.at(&path)
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
        self.at(&self.path().key(field))
    }

    /// Returns every binder declaration of the pass, keyed by binding point.
    pub fn binders(&self) -> Result<Vec<Declaration<'document>>, GraphicsError> {
        let path = self.path().key(BINDERS);
        let Some(Value::Dict(entries)) = self.at(&path) else {
            return Err(malformed(&path, BINDERS, "a dict", self.at(&path)));
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

/// One image a graphics document says a render produces.
///
/// # Invariants
///
/// - **The point is where the image is rendered, and the location is which fragment output
///   carries it.** They are two facts rather than one, and the second is stated rather than
///   derived: a pass numbers its colour targets from zero in attachment order, so a location
///   inferred from where a name sits in the map would move every name below it the moment one
///   was inserted — and the failure would be a wrong picture rather than an error.
/// - **The type is the spelling, borrowed.** It is carried as the text the document wrote and
///   is not resolved here, because the resolution has a consequence this reader cannot report:
///   a type no format can hold is a refusal, and a refusal has to name the location and the
///   shader that would have written it. That is the device's [`Output`] check, which parses the
///   spelling once and answers for all three fields at once.
/// - All three borrows come from the document tree. A declaration is a struct of scalars, so
///   they are real borrows and never values decoded out of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Output<'document> {
    /// The name the image is looked up under. The frame is [`crate::PRESENT_OUTPUT`].
    pub name: &'document str,
    /// The binding point of a pass's `targets` that the image is written into.
    pub point: &'document str,
    /// The `@location` of the writing shader's fragment output that carries the image.
    pub location: u32,
    /// The type of one texel, spelled as [`crate::TexelType`] spells it.
    pub ty: &'document str,
}

/// Reads the images a graphics document says it produces.
///
/// # Invariants
///
/// An **absent** field is an empty list rather than an error, and it means what every document
/// written before this field existed meant: the colour target of the last pass that wrote one is
/// the frame. That default belongs to the renderer, because which target is a colour target is a
/// question about the resolved payloads rather than about the document, so this reader answers
/// only what the document actually declares.
///
/// Nothing here checks that some pass renders into a declared point, or that the shader at the
/// other end writes it. A point no pass targets is a document asking for an image nothing
/// writes, and a location no shader fills is a target a render returns holding whatever was in
/// a texture nobody wrote. Both are reported by the device that was asked to produce the image,
/// where the passes and the shaders it did run are known.
pub fn outputs(graphics: &Value) -> Result<Vec<Output<'_>>, GraphicsError> {
    let path = Path::root().key(OUTPUTS_FIELD);
    // The output map is a field of the document's own root, so it is a real borrow of the tree
    // and never a value decoded out of a run. A root that carried a packed map could not answer
    // with borrowed names, and a name has to be borrowed to be handed back. The same holds one
    // level down: a declaration is a struct of scalars, so its fields are borrows of the same
    // tree and a declaration is never copied out to be read.
    let found = match graphics.at(&path) {
        None => return Ok(Vec::new()),
        Some(Resolved::Borrowed(value)) => value,
        Some(Resolved::Owned(_)) => {
            return Err(malformed(&path, OUTPUTS_FIELD, "a dict", None));
        }
    };
    let Some(entries) = found.as_dict() else {
        return Err(malformed(&path, OUTPUTS_FIELD, "a dict", Some(found)));
    };

    let mut outputs = Vec::with_capacity(entries.len());
    for (name, value) in entries {
        let at = path.key(name);
        let Some(fields) = value.as_struct() else {
            return Err(malformed(&at, name, "an output declaration", Some(value)));
        };
        let point = field(&at, fields, POINT_FIELD, "a binding point")?;
        let ty = field(&at, fields, TYPE_FIELD, "a texel type")?;
        let declared = fields.get(LOCATION_FIELD);
        let location = declared
            .and_then(Value::as_int)
            .and_then(|number| u32::try_from(number).ok())
            .ok_or_else(|| {
                malformed(
                    &at.key(LOCATION_FIELD),
                    LOCATION_FIELD,
                    "a non-negative location",
                    declared,
                )
            })?;
        outputs.push(Output {
            name,
            point,
            location,
            ty,
        });
    }
    Ok(outputs)
}

/// Borrows one string field of an output declaration, or reports which shape it is not.
fn field<'document>(
    at: &Path,
    fields: &'document FieldMap,
    name: &'static str,
    expected: &'static str,
) -> Result<&'document str, GraphicsError> {
    let found = fields.get(name);
    found
        .and_then(Value::as_str)
        .ok_or_else(|| malformed(&at.key(name), name, expected, found))
}

/// Reads every pass out of a graphics document, in execution order.
pub fn passes(graphics: &Value) -> Result<Vec<PassView<'_>>, GraphicsError> {
    let root = Path::root();
    let path = root.key(PASSES_FIELD);
    // A pass list is a list of declarations, never a run of numbers, so it is read as a tree.
    // Were it packed, `as_list` would refuse it rather than silently pass over the elements.
    let found = graphics.at(&path);
    let Some(entries) = found.as_deref().and_then(Value::as_list) else {
        return Err(malformed(&path, PASSES_FIELD, "a list", found.as_deref()));
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
