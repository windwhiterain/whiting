//! Sparse, versioned changes to a [`Definition`].

use std::collections::BTreeMap;

use facet::Facet;

use crate::definition::Definition;
use crate::error::DeltaError;
use crate::identifier::DefinitionIdentifier;
use crate::loaded::Loaded;
use crate::path::Path;
use crate::registry::Registry;
use crate::value::Value;
use crate::version::Version;

/// A change to the single value a [`Path`] selects.
///
/// Three variants, as the spec names them: `Replace` swaps the value, and `Append` and
/// `Remove` add to or take from one. A value a list holds and a value a dict holds are
/// different shapes, so what each of those two acts on is a union of the shapes rather than
/// a variant apiece.
///
/// `Append` of values and `Remove` of indices each describe a contiguous range of a list,
/// which makes them exact mutual inverses. An arbitrary non-contiguous set of indices is
/// deliberately not representable, because no append primitive can invert it.
#[derive(Debug, Clone, PartialEq, Facet)]
#[repr(C)]
pub enum PrimitiveDelta {
    /// Replaces the value outright, structural values included.
    Replace(Value),
    /// Adds to a list or to a dict.
    Append(Appended),
    /// Takes from a list, a struct or a dict.
    Remove(Removed),
}

/// What a [`PrimitiveDelta::Append`] adds.
#[derive(Debug, Clone, PartialEq, Facet)]
#[repr(C)]
pub enum Appended {
    /// Inserts values before `index`, so `index == length` is a tail append.
    Values { index: usize, values: Vec<Value> },
    /// Adds entries to a dict, failing if a key is already present.
    Entries { entries: Vec<(String, Value)> },
}

/// What a [`PrimitiveDelta::Remove`] takes.
#[derive(Debug, Clone, PartialEq, Facet)]
#[repr(C)]
pub enum Removed {
    /// Removes `count` elements starting at `start`.
    Indices { start: usize, count: usize },
    /// Removes keys from a struct or a dict.
    Keys { keys: Vec<String> },
}

impl PrimitiveDelta {
    /// Builds a list insert.
    pub fn append_values(index: usize, values: impl IntoIterator<Item = Value>) -> Self {
        Self::Append(Appended::Values {
            index,
            values: values.into_iter().collect(),
        })
    }

    /// Builds a removal of a contiguous range of a list.
    pub fn remove_indices(start: usize, count: usize) -> Self {
        Self::Remove(Removed::Indices { start, count })
    }

    /// Builds a dict entry append.
    pub fn append_entries(entries: impl IntoIterator<Item = (String, Value)>) -> Self {
        Self::Append(Appended::Entries {
            entries: entries.into_iter().collect(),
        })
    }

    /// Builds a key removal.
    pub fn remove_keys(keys: impl IntoIterator<Item = String>) -> Self {
        Self::Remove(Removed::Keys {
            keys: keys.into_iter().collect(),
        })
    }
}

/// How to change the definition a primitive delta references.
///
/// Apply is atomic: a delta either applies completely and leaves the definition at
/// `version + 1`, or leaves it untouched. This is achieved by applying to a clone,
/// validating the clone against its schema, and only then committing.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Delta {
    /// The definition this delta applies to.
    pub definition_identifier: DefinitionIdentifier,
    /// The version this delta was computed against.
    pub base_version: Version,
    /// A sparse subset of the referenced definition, mapping each path to its change.
    pub operations: BTreeMap<Path, PrimitiveDelta>,
}

impl Delta {
    /// Creates a delta against the given definition version.
    pub fn new(
        definition_identifier: DefinitionIdentifier,
        base_version: Version,
        operations: BTreeMap<Path, PrimitiveDelta>,
    ) -> Self {
        Self {
            definition_identifier,
            base_version,
            operations,
        }
    }

    /// Creates a delta carrying a single operation.
    pub fn with_operation(
        definition_identifier: DefinitionIdentifier,
        base_version: Version,
        path: Path,
        primitive: PrimitiveDelta,
    ) -> Self {
        let mut operations = BTreeMap::new();
        operations.insert(path, primitive);
        Self::new(definition_identifier, base_version, operations)
    }

    /// Returns whether this delta would change nothing.
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// Applies this delta and returns the inverse delta that undoes it.
    ///
    /// The returned inverse carries the post-apply version, because that is the state it
    /// applies to.
    ///
    /// # References are patched as a loaded generation
    ///
    /// A path is transparent, so a path that runs through a reference names the node the
    /// reference points at. The serialized form holds the spelling rather than the node, so a
    /// document that spells one has nothing for such a path to arrive at and is refused here:
    /// it is patched with [`Loaded::patch`](crate::Loaded::patch) instead. A delta may still
    /// introduce a reference, because writing a spelling is an ordinary write of a value and
    /// nothing traverses it; the deltas that follow belong on the loaded generation.
    ///
    /// ```
    /// use std::collections::BTreeMap;
    ///
    /// use whiting_definition::{
    ///     Definition, DefinitionIdentifier, Delta, DeltaError, Path, PrimitiveDelta, Reference,
    ///     Registry, RegistryError, Schema, SchemaIdentifier, SchemaKind, Value,
    /// };
    ///
    /// let slot = Schema::new(SchemaKind::Int).reference();
    /// let schema = Schema::new(SchemaKind::structure([
    ///     ("left".to_owned(), slot.clone()),
    ///     ("right".to_owned(), slot.clone()),
    /// ]));
    /// let mut registry = Registry::new();
    /// registry.put_schema(SchemaIdentifier::new("scene"), schema);
    ///
    /// let identifier = DefinitionIdentifier::new("scene");
    /// registry
    ///     .insert_definition(Definition::new(
    ///         identifier.clone(),
    ///         SchemaIdentifier::new("scene"),
    ///         Value::structure([
    ///             ("left".to_owned(), Value::Int(7)),
    ///             (
    ///                 "right".to_owned(),
    ///                 Reference::Internal(Path::root().key("left")).to_value(),
    ///             ),
    ///         ]),
    ///     ))
    ///     .expect("the definition is new");
    ///
    /// let mut operations = BTreeMap::new();
    /// operations.insert(
    ///     Path::root().key("left"),
    ///     PrimitiveDelta::Replace(Value::Int(9)),
    /// );
    /// let delta = Delta::new(identifier, 0u64.into(), operations);
    ///
    /// // The serialized form refuses it, and names the reference it found.
    /// assert!(matches!(
    ///     registry.apply(&delta),
    ///     Err(RegistryError::Delta(DeltaError::ReferenceInSerialized { .. }))
    /// ));
    /// ```
    pub fn apply(
        &self,
        definition: &mut Definition,
        registry: &Registry,
    ) -> Result<Delta, DeltaError> {
        if self.definition_identifier != definition.definition_identifier {
            return Err(DeltaError::WrongDefinition {
                expected: self.definition_identifier.clone(),
                actual: definition.definition_identifier.clone(),
            });
        }
        if self.base_version != definition.version {
            return Err(DeltaError::Conflict {
                expected: self.base_version,
                actual: definition.version,
            });
        }

        // A path is transparent, so a path that runs through a reference names the node the
        // reference points at, and only a loaded generation holds those nodes. A document that
        // already spells one is therefore patched as a loaded generation rather than here.
        //
        // A delta may still *introduce* a reference, because writing a spelling is an ordinary
        // write of a value and nothing traverses it. From then on the document spells one, so
        // the deltas that follow belong on the loaded generation.
        let schema = registry.schema(&definition.schema_identifier)?;
        if let Some(path) = Loaded::spelled_reference(&definition.value, schema) {
            return Err(DeltaError::ReferenceInSerialized { path });
        }

        // Applying to a clone is what makes a rejected delta leave nothing behind, so a
        // failure part way through a multi-operation delta cannot corrupt the definition.
        let mut next = definition.value.clone();
        let mut inverse_operations = BTreeMap::new();
        for (path, primitive) in &self.operations {
            apply_primitive(&mut next, path, primitive, &mut inverse_operations)?;
        }

        let next_version = definition.version.incremented();
        let mut candidate = definition.clone();
        candidate.value = next;
        candidate.version = next_version;
        candidate.validate(registry)?;

        *definition = candidate;
        Ok(Delta {
            definition_identifier: self.definition_identifier.clone(),
            base_version: next_version,
            operations: inverse_operations,
        })
    }
}

fn apply_primitive(
    root: &mut Value,
    path: &Path,
    primitive: &PrimitiveDelta,
    inverse_operations: &mut BTreeMap<Path, PrimitiveDelta>,
) -> Result<(), DeltaError> {
    let invalid = |reason: &str| DeltaError::InvalidOperation {
        path: path.clone(),
        reason: reason.to_owned(),
    };

    let target = path
        .resolve_mut(root)
        .map_err(|reason| DeltaError::UnresolvedPath {
            path: path.clone(),
            reason: reason.to_string(),
        })?;

    match primitive {
        PrimitiveDelta::Replace(new_value) => {
            let previous = std::mem::replace(target, new_value.clone());
            inverse_operations.insert(path.clone(), PrimitiveDelta::Replace(previous));
        }

        PrimitiveDelta::Append(Appended::Values { index, values }) => {
            let elements = target
                .as_list_mut()
                .ok_or_else(|| invalid("append needs a list"))?;
            if *index > elements.len() {
                return Err(invalid("append index is beyond the end of the list"));
            }
            for (offset, value) in values.iter().enumerate() {
                elements.insert(*index + offset, value.clone());
            }
            inverse_operations.insert(
                path.clone(),
                PrimitiveDelta::remove_indices(*index, values.len()),
            );
        }

        PrimitiveDelta::Remove(Removed::Indices { start, count }) => {
            let elements = target
                .as_list_mut()
                .ok_or_else(|| invalid("removal needs a list"))?;
            let end = start
                .checked_add(*count)
                .filter(|end| *end <= elements.len())
                .ok_or_else(|| invalid("removal range is out of bounds for the list"))?;
            let removed = elements.drain(*start..end).collect::<Vec<Value>>();
            inverse_operations.insert(path.clone(), PrimitiveDelta::append_values(*start, removed));
        }

        PrimitiveDelta::Append(Appended::Entries { entries }) => {
            let map = target
                .as_dict_mut()
                .ok_or_else(|| invalid("entry append needs a dict"))?;
            for (name, value) in entries {
                if map.insert(name.clone(), value.clone()).is_some() {
                    return Err(invalid("entry append found a key that is already present"));
                }
            }
            inverse_operations.insert(
                path.clone(),
                PrimitiveDelta::remove_keys(entries.iter().map(|(name, _)| name.clone())),
            );
        }

        PrimitiveDelta::Remove(Removed::Keys { keys }) => {
            let map = target
                .as_field_map_mut()
                .ok_or_else(|| invalid("key removal needs a struct or a dict"))?;
            let mut removed = Vec::with_capacity(keys.len());
            for name in keys {
                match map.remove(name) {
                    Some(value) => removed.push((name.clone(), value)),
                    None => return Err(invalid("key removal found a key that is absent")),
                }
            }
            inverse_operations.insert(path.clone(), PrimitiveDelta::append_entries(removed));
        }
    }

    Ok(())
}
