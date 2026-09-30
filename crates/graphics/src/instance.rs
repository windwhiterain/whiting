//! The instance schema implied by the binders a graphics document declares.
//!
//! An instance is authored independently of the passes that read it, which is what
//! makes it easy to edit. The cost of that independence is that a plain open dict
//! accepts anything at all, so a mistyped entry is only discovered at render time.
//! Deriving the schema from the declarations closes that gap without either document
//! having to be derived from the other.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use definition::{Schema, SchemaKind, Value};

use crate::GraphicsError;
use crate::binders::BinderKind;
use crate::read::passes;

/// Builds the instance schema a graphics document implies.
///
/// # Invariants
///
/// An entry name bound by two passes must be declared with the same kind by both. A
/// disagreement fails the derivation rather than widening to `Any`, because a silent
/// widening would turn an author mistake into an unchecked binding.
pub fn derived_instance_schema(graphics: &Value) -> Result<Schema, GraphicsError> {
    let mut kinds: BTreeMap<&str, BinderKind> = BTreeMap::new();
    for pass in passes(graphics)? {
        for declaration in pass.binders()? {
            let kind = declaration.kind()?;
            let source = declaration.source()?;
            match kinds.entry(source) {
                Entry::Vacant(slot) => {
                    slot.insert(kind);
                }
                Entry::Occupied(slot) if *slot.get() == kind => {}
                Entry::Occupied(slot) => {
                    return Err(GraphicsError::BinderConflict {
                        entry: source.to_owned(),
                        first: slot.get().tag(),
                        second: kind.tag(),
                    });
                }
            }
        }
    }

    Ok(Schema::new(SchemaKind::structure([
        (
            crate::REFERENCING_GRAPHICS.to_owned(),
            Schema::new(SchemaKind::String)
                .with_documentation("Definition this instance provides data to."),
        ),
        (
            crate::ENTRIES.to_owned(),
            Schema::new(SchemaKind::dictionary(crate::binder_payload_schema()))
                .with_documentation("Instance data, keyed by the name a binder asks for."),
        ),
    ]))
    .with_documentation("Instance data, derived from the binders of the passes that read it."))
}
