//! The store of schemas and definitions, and the entry point for every mutation.

use std::collections::BTreeMap;

use crate::definition::Definition;
use crate::delta::Delta;
use crate::error::{RegistryError, ValidationError};
use crate::history::History;
use crate::identifier::{DefinitionIdentifier, SchemaIdentifier};
use crate::schema::Schema;

/// The store of schemas and definitions.
///
/// Schemas and definitions are registered and replaced at runtime, so a schema change
/// takes effect with no recompilation.
#[derive(Debug, Default)]
pub struct Registry {
    schemas: BTreeMap<SchemaIdentifier, Schema>,
    definitions: BTreeMap<DefinitionIdentifier, Definition>,
    histories: BTreeMap<DefinitionIdentifier, History>,
}

impl Registry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a schema, replacing any schema already under that name.
    pub fn put_schema(&mut self, schema_identifier: SchemaIdentifier, schema: Schema) {
        self.schemas.insert(schema_identifier, schema);
    }

    /// Borrows a registered schema.
    pub fn schema(&self, schema_identifier: &SchemaIdentifier) -> Result<&Schema, ValidationError> {
        self.schemas
            .get(schema_identifier)
            .ok_or_else(|| ValidationError::UnknownSchema(schema_identifier.clone()))
    }

    /// Returns the names of every registered schema, in order.
    pub fn schema_identifiers(&self) -> Vec<SchemaIdentifier> {
        self.schemas.keys().cloned().collect()
    }

    /// Registers a definition, failing if one already exists under that name.
    pub fn insert_definition(&mut self, definition: Definition) -> Result<(), RegistryError> {
        if self
            .definitions
            .contains_key(&definition.definition_identifier)
        {
            return Err(RegistryError::DuplicateDefinition(
                definition.definition_identifier,
            ));
        }
        let definition_identifier = definition.definition_identifier.clone();
        self.definitions
            .insert(definition_identifier.clone(), definition);
        self.histories.insert(definition_identifier, History::new());
        Ok(())
    }

    /// Removes a definition and its history.
    pub fn remove_definition(
        &mut self,
        definition_identifier: &DefinitionIdentifier,
    ) -> Result<Definition, RegistryError> {
        self.histories.remove(definition_identifier);
        self.definitions
            .remove(definition_identifier)
            .ok_or_else(|| RegistryError::UnknownDefinition(definition_identifier.clone()))
    }

    /// Borrows a registered definition.
    pub fn definition(
        &self,
        definition_identifier: &DefinitionIdentifier,
    ) -> Result<&Definition, RegistryError> {
        self.definitions
            .get(definition_identifier)
            .ok_or_else(|| RegistryError::UnknownDefinition(definition_identifier.clone()))
    }

    /// Mutably borrows a registered definition, bypassing history.
    ///
    /// Prefer [`Registry::apply`], which records the inverse so the change stays undoable.
    pub fn definition_mut(
        &mut self,
        definition_identifier: &DefinitionIdentifier,
    ) -> Result<&mut Definition, RegistryError> {
        self.definitions
            .get_mut(definition_identifier)
            .ok_or_else(|| RegistryError::UnknownDefinition(definition_identifier.clone()))
    }

    /// Returns the names of every registered definition, in order.
    pub fn definition_identifiers(&self) -> Vec<DefinitionIdentifier> {
        self.definitions.keys().cloned().collect()
    }

    /// Borrows the history of a registered definition.
    pub fn history(&self, definition_identifier: &DefinitionIdentifier) -> Option<&History> {
        self.histories.get(definition_identifier)
    }

    /// Applies a delta and records its inverse, returning the inverse.
    ///
    /// A delta computed against a stale version is rejected as a conflict, so two agents
    /// can never silently clobber each other.
    pub fn apply(&mut self, delta: &Delta) -> Result<Delta, RegistryError> {
        // The definition is taken out for the duration of the apply so that `Delta::apply`
        // can read the registry for schema validation without borrowing it immutably while
        // the definition itself is held mutably. `Delta::apply` is atomic, so restoring an
        // untouched definition after a failure is always correct.
        let Some(mut definition) = self.definitions.remove(&delta.definition_identifier) else {
            return Err(RegistryError::UnknownDefinition(
                delta.definition_identifier.clone(),
            ));
        };
        let applied = delta.apply(&mut definition, self);
        self.definitions
            .insert(delta.definition_identifier.clone(), definition);
        let inverse = applied?;

        if let Some(history) = self.histories.get_mut(&delta.definition_identifier) {
            history.record(delta.clone(), inverse.clone());
        }
        Ok(inverse)
    }

    /// Undoes the newest applied delta of a definition.
    pub fn undo(
        &mut self,
        definition_identifier: &DefinitionIdentifier,
    ) -> Result<Delta, RegistryError> {
        let Some(mut definition) = self.definitions.remove(definition_identifier) else {
            return Err(RegistryError::UnknownDefinition(
                definition_identifier.clone(),
            ));
        };
        let Some(mut history) = self.histories.remove(definition_identifier) else {
            self.definitions
                .insert(definition_identifier.clone(), definition);
            return Err(RegistryError::UnknownDefinition(
                definition_identifier.clone(),
            ));
        };

        let result = history.undo(&mut definition, self);
        self.definitions
            .insert(definition_identifier.clone(), definition);
        self.histories
            .insert(definition_identifier.clone(), history);
        result.map_err(Into::into)
    }

    /// Redoes the newest undone delta of a definition.
    pub fn redo(
        &mut self,
        definition_identifier: &DefinitionIdentifier,
    ) -> Result<Delta, RegistryError> {
        let Some(mut definition) = self.definitions.remove(definition_identifier) else {
            return Err(RegistryError::UnknownDefinition(
                definition_identifier.clone(),
            ));
        };
        let Some(mut history) = self.histories.remove(definition_identifier) else {
            self.definitions
                .insert(definition_identifier.clone(), definition);
            return Err(RegistryError::UnknownDefinition(
                definition_identifier.clone(),
            ));
        };

        let result = history.redo(&mut definition, self);
        self.definitions
            .insert(definition_identifier.clone(), definition);
        self.histories
            .insert(definition_identifier.clone(), history);
        result.map_err(Into::into)
    }
}
