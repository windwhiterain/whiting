//! What a load needs from outside the document it is reading.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use crate::definition::Definition;
use crate::error::LoadError;
use crate::identifier::DefinitionIdentifier;
use crate::loaded::Loaded;
use crate::reference::FilePath;
use crate::registry::Registry;
use crate::value::Value;

/// What a document is, which is what a relative path inside it resolves against.
///
/// A relative file path is resolved against the document that spelled it rather than against
/// the registry, so a definition stored beside the assets it names keeps working when the
/// whole directory moves. A document a file held is a document in its own right, so a path
/// relative to it is relative to that file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// A definition the registry holds.
    Definition(DefinitionIdentifier),
    /// A file, which a document inside it is relative to.
    File(FilePath),
}

/// What a reference wants a file to be, which its slot's schema decides.
///
/// A slot of bytes wants the file itself, and a slot of anything else wants the document it
/// holds, because a GPU buffer cannot hold a path and a structured value cannot be assembled
/// from raw bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wanted {
    /// The file's own bytes.
    Asset,
    /// The document the file holds.
    Document,
}

/// What a file held.
#[derive(Debug, Clone)]
pub enum FileContent {
    /// The bytes of an asset.
    Bytes(Vec<u8>),
    /// The document a definition was written as.
    Document(Value),
}

/// Where a load finds what the document it is reading does not hold itself.
///
/// # Invariants
///
/// The store owns generations: it decides when one is loaded, what keeps it alive, and so
/// when it is freed. A reference keeps its target alive, so a definition already being loaded
/// must be refused rather than loaded again — two definitions naming each other would hold
/// two generations alive with nothing else reaching either, which is the one cycle
/// refcounting cannot free. Cycles *within* a generation are free and need no such rule.
pub trait Store {
    /// Returns the generation of a definition the registry holds, loading it if it is not.
    ///
    /// The name is the first segment of the path an external reference spelled.
    fn definition(&self, name: &str) -> Result<Rc<Loaded>, LoadError>;

    /// Returns what a file holds, resolving a relative path against the document that asks.
    fn file(
        &self,
        from: &Origin,
        path: &FilePath,
        wanted: Wanted,
    ) -> Result<FileContent, LoadError>;
}

/// A store over a registry, so a document that names another one can be read.
///
/// # Why the store is here rather than in a client
///
/// A compiler and a renderer both read a document that may name another one, and both have to
/// resolve it the same way: the same store, the same cycle rule, the same expansion. Only the
/// registry knows what a definition is registered under and which schema it is read with, and the
/// registry is here.
///
/// # Invariants
///
/// A reference keeps what it names alive, so definitions refer to each other in a graph rather than
/// in a cycle: a definition already being loaded is refused rather than loaded again, or two
/// definitions would hold each other's generation alive with nothing else reaching either. The set
/// of names in flight is the whole of that state, and it is interior because the load that consults
/// it is the load that is already running.
pub struct RegistryStore<'registry> {
    registry: &'registry Registry,
    loading: RefCell<BTreeSet<String>>,
}

impl<'registry> RegistryStore<'registry> {
    /// Creates a store over a registry.
    pub fn new(registry: &'registry Registry) -> Self {
        Self {
            registry,
            loading: RefCell::new(BTreeSet::new()),
        }
    }

    /// Returns a definition's document with every reference in it resolved and written out.
    ///
    /// # Invariants
    ///
    /// A document that spells no reference is returned as it is, because a reader of the serialized
    /// form can read it directly; only a document that names something outside itself pays for
    /// being loaded.
    pub fn value(&self, definition: &Definition) -> Result<Value, LoadError> {
        let schema = self
            .registry
            .schema(&definition.schema_identifier)
            .map_err(|error| LoadError::Store {
                reason: error.to_string(),
            })?;
        if Loaded::spelled_reference(&definition.value, schema).is_none() {
            return Ok(definition.value.clone());
        }
        let loaded = self.load(&definition.definition_identifier, &definition.value, schema)?;
        Ok(loaded.expanded()?)
    }

    /// Loads one definition against the schema it is registered with.
    fn load(
        &self,
        identifier: &DefinitionIdentifier,
        value: &Value,
        schema: &crate::schema::Schema,
    ) -> Result<Rc<Loaded>, LoadError> {
        let origin = Origin::Definition(identifier.clone());
        Ok(Rc::new(Loaded::load_in(value, schema, &origin, self)?))
    }
}

impl Store for RegistryStore<'_> {
    fn definition(&self, name: &str) -> Result<Rc<Loaded>, LoadError> {
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
                "a registry holds definitions rather than files, so `{path}` cannot be resolved: \
                 put the data in a definition instead"
            ),
        })
    }
}
