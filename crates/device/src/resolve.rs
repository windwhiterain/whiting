//! Keeping what a render resolved, so a render does not resolve it again.
//!
//! # Why this is not the whole of resolving
//!
//! Resolving a document belongs to the [`Registry`] it is registered in: the same store, the same
//! cycle rule and the same expansion serve a compiler reading a scene and a renderer reading
//! graphics. What belongs here is the device's own concern, which is that a render happens far more
//! often than a definition changes.

use std::cell::RefCell;
use std::collections::BTreeMap;

use definition::{Definition, DefinitionIdentifier, Registry, RegistryStore, Value, Version};

use crate::state::DeviceError;

/// Documents already written out, keyed by the identifier they are stored under.
///
/// # Invariants
///
/// An entry is refreshed whenever the definition's version differs, because a version is what a
/// change to a definition produces. A definition changed *without* its version changing is not seen
/// here, so whoever changes one that way must call
/// [`Device::forget_resolved`](crate::Device::forget_resolved).
pub type Resolved = RefCell<BTreeMap<DefinitionIdentifier, (Version, Value)>>;

/// A store over a registry that keeps what it resolved.
pub struct Stored<'registry> {
    store: RegistryStore<'registry>,
    resolved: &'registry Resolved,
}

impl<'registry> Stored<'registry> {
    /// Creates a store over a registry, writing into a cache of resolved documents.
    pub fn new(registry: &'registry Registry, resolved: &'registry Resolved) -> Self {
        Self {
            store: RegistryStore::new(registry),
            resolved,
        }
    }

    /// Returns a definition's document with every reference in it resolved and written out.
    pub fn value(&self, definition: &Definition) -> Result<Value, DeviceError> {
        let identifier = &definition.definition_identifier;
        let cached = self.resolved.borrow().get(identifier).cloned();
        if let Some((version, value)) = cached {
            if version == definition.version {
                return Ok(value);
            }
        }
        let value = self.store.value(definition)?;
        self.resolved
            .borrow_mut()
            .insert(identifier.clone(), (definition.version, value.clone()));
        Ok(value)
    }
}
