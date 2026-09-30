//! The versioned document that a [`Delta`](crate::Delta) changes.

use facet::Facet;

use crate::error::ValidationError;
use crate::identifier::{DefinitionIdentifier, SchemaIdentifier};
use crate::path::Path;
use crate::registry::Registry;
use crate::schema::Schema;
use crate::value::Value;
use crate::version::Version;

/// A versioned data value that references a [`Schema`] by name.
///
/// The schema is referenced rather than embedded, so one schema is stored once no matter
/// how many definitions and deltas use it, and so a schema can be replaced at runtime
/// without rewriting the documents that depend on it.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Definition {
    /// Unique name of this definition within its registry.
    pub definition_identifier: DefinitionIdentifier,
    /// Counter incremented once per applied delta.
    pub version: Version,
    /// Name of the schema this value is verified against.
    pub schema_identifier: SchemaIdentifier,
    /// The data itself, as authored.
    pub value: Value,
}

impl Definition {
    /// Creates a definition at the initial version.
    pub fn new(
        definition_identifier: DefinitionIdentifier,
        schema_identifier: SchemaIdentifier,
        value: Value,
    ) -> Self {
        Self {
            definition_identifier,
            version: Version::INITIAL,
            schema_identifier,
            value,
        }
    }

    /// Verifies this definition against the schema its registry holds.
    pub fn validate(&self, registry: &Registry) -> Result<(), ValidationError> {
        registry.schema(&self.schema_identifier)?.validate(&self.value)
    }

    /// Returns the value this definition stores, with every schema default substituted.
    ///
    /// Validation runs against the authored value; defaults are for readers that need a
    /// fully populated tree, such as a renderer walking the document.
    pub fn defaulted_value(&self, registry: &Registry) -> Result<Value, ValidationError> {
        let schema: &Schema = registry.schema(&self.schema_identifier)?;
        Ok(self.value.defaulted(schema))
    }

    /// Borrows the value at `path`, reporting whether it resolves.
    pub fn at(&self, path: &Path) -> Option<&Value> {
        path.resolve(&self.value).ok()
    }
}
