//! Loading the documents a device holds, so a reference in one of them can be resolved.
//!
//! # Why the device loads at all
//!
//! A renderer reads a serialized document, and a serialized document can name data that lives in
//! another one. A reference is resolved by loading, so a document that shares its data reaches
//! it here and nowhere else: the reader is handed what the reference names, written out as an
//! ordinary document. That is also why the device cannot do this per entry — nothing smaller
//! than a document knows what its references resolve to.
//!
//! # Why the common case costs nothing extra
//!
//! A document that spells no reference is read directly, because loading it would copy the whole
//! graph only to write it back. Only a document that names something outside itself is loaded,
//! so a scene that keeps its data to itself is unaffected.

use std::cell::RefCell;
use std::collections::BTreeSet;

use definition::{
    Definition, DefinitionIdentifier, FileContent, FilePath, LoadError, Loaded, Origin, Registry,
    Store, Value, Wanted,
};

use crate::state::DeviceError;

/// The registry as a store, so a document that names another one can be loaded.
///
/// # Invariants
///
/// A reference keeps what it names alive, so definitions refer to each other in a graph rather
/// than in a cycle: a definition already being loaded is refused rather than loaded again, or two
/// definitions would hold each other's generation alive with nothing else reaching either. The
/// set of names in flight is the whole of that state, and it is interior because the load that
/// consults it is the load that is already running.
pub struct Stored<'registry> {
    registry: &'registry Registry,
    loading: RefCell<BTreeSet<String>>,
}

impl<'registry> Stored<'registry> {
    /// Creates a store over a registry.
    pub fn new(registry: &'registry Registry) -> Self {
        Self {
            registry,
            loading: RefCell::new(BTreeSet::new()),
        }
    }

    /// Returns a definition's document with every reference in it resolved and written out.
    pub fn value(&self, definition: &Definition) -> Result<Value, DeviceError> {
        let schema = self
            .registry
            .schema(&definition.schema_identifier)
            .map_err(DeviceError::from)?;
        if Loaded::spelled_reference(&definition.value, schema).is_none() {
            return Ok(definition.value.clone());
        }
        let identifier = &definition.definition_identifier;
        let loaded = self.load(identifier, &definition.value, schema)?;
        Ok(loaded.expanded()?)
    }

    /// Loads one definition against the schema it is registered with.
    fn load(
        &self,
        identifier: &DefinitionIdentifier,
        value: &Value,
        schema: &definition::Schema,
    ) -> Result<std::rc::Rc<Loaded>, LoadError> {
        let origin = Origin::Definition(identifier.clone());
        Ok(std::rc::Rc::new(Loaded::load_in(
            value, schema, &origin, self,
        )?))
    }
}

impl Store for Stored<'_> {
    fn definition(&self, name: &str) -> Result<std::rc::Rc<Loaded>, LoadError> {
        let identifier = DefinitionIdentifier::new(name);
        if !self.loading.borrow_mut().insert(name.to_owned()) {
            return Err(LoadError::Store {
                reason: format!("`{name}` is already being loaded"),
            });
        }
        let loaded = (|| {
            let definition =
                self.registry
                    .definition(&identifier)
                    .map_err(|error| LoadError::Store {
                        reason: error.to_string(),
                    })?;
            let schema = self
                .registry
                .schema(&definition.schema_identifier)
                .map_err(|error| LoadError::Store {
                    reason: error.to_string(),
                })?;
            self.load(&identifier, &definition.value, schema)
        })();
        self.loading.borrow_mut().remove(name);
        loaded
    }

    fn file(
        &self,
        _from: &Origin,
        path: &FilePath,
        _wanted: Wanted,
    ) -> Result<FileContent, LoadError> {
        Err(LoadError::Store {
            reason: format!(
                "this device reads no files, so it cannot resolve `{path}`: put the data in a \
                 definition instead"
            ),
        })
    }
}
