//! What a load needs from outside the document it is reading.

use std::rc::Rc;

use crate::error::LoadError;
use crate::identifier::DefinitionIdentifier;
use crate::loaded::Loaded;
use crate::reference::FilePath;
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
