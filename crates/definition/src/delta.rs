//! Sparse, versioned changes to a [`Definition`].

use std::collections::BTreeMap;

use facet::Facet;

use crate::definition::Definition;
use crate::error::DeltaError;
use crate::identifier::DefinitionIdentifier;
use crate::path::Path;
use crate::registry::Registry;
use crate::value::Value;
use crate::version::Version;

/// A change to the single value a [`Path`] selects.
///
/// `Append` and `RemoveIndices` each describe a contiguous range of a list, which makes
/// them exact mutual inverses. An arbitrary non-contiguous set of indices is deliberately
/// not representable, because no append primitive can invert it.
#[derive(Debug, Clone, PartialEq, Facet)]
#[repr(C)]
pub enum PrimitiveDelta {
    /// Replaces the value outright, structural values included.
    Replace(Value),
    /// Inserts values before `index`, so `index == length` is a tail append.
    Append { index: usize, values: Vec<Value> },
    /// Adds entries to a dict, failing if a key is already present.
    AppendEntries { entries: Vec<(String, Value)> },
    /// Removes `count` elements starting at `start`.
    RemoveIndices { start: usize, count: usize },
    /// Removes keys from a struct or a dict.
    RemoveKeys { keys: Vec<String> },
}

impl PrimitiveDelta {
    /// Builds a removal of a contiguous range of a list.
    pub fn remove_indices(start: usize, count: usize) -> Self {
        Self::RemoveIndices { start, count }
    }

    /// Builds a dict entry append.
    pub fn append_entries(entries: impl IntoIterator<Item = (String, Value)>) -> Self {
        Self::AppendEntries {
            entries: entries.into_iter().collect(),
        }
    }

    /// Builds a key removal.
    pub fn remove_keys(keys: impl IntoIterator<Item = String>) -> Self {
        Self::RemoveKeys {
            keys: keys.into_iter().collect(),
        }
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
    let invalid = |reason: &'static str| DeltaError::InvalidOperation {
        path: path.clone(),
        reason,
    };

    let target = path
        .resolve_mut(root)
        .map_err(|reason| DeltaError::UnresolvedPath {
            path: path.clone(),
            reason,
        })?;

    match primitive {
        PrimitiveDelta::Replace(new_value) => {
            let previous = std::mem::replace(target, new_value.clone());
            inverse_operations.insert(path.clone(), PrimitiveDelta::Replace(previous));
        }

        PrimitiveDelta::Append { index, values } => {
            let elements = target.as_list_mut().ok_or_else(|| invalid("append needs a list"))?;
            if *index > elements.len() {
                return Err(invalid("append index is beyond the end of the list"));
            }
            for (offset, value) in values.iter().enumerate() {
                elements.insert(*index + offset, value.clone());
            }
            inverse_operations.insert(
                path.clone(),
                PrimitiveDelta::RemoveIndices {
                    start: *index,
                    count: values.len(),
                },
            );
        }

        PrimitiveDelta::RemoveIndices { start, count } => {
            let elements = target.as_list_mut().ok_or_else(|| invalid("removal needs a list"))?;
            let end = start
                .checked_add(*count)
                .filter(|end| *end <= elements.len())
                .ok_or_else(|| invalid("removal range is out of bounds for the list"))?;
            let removed = elements.drain(*start..end).collect();
            inverse_operations.insert(
                path.clone(),
                PrimitiveDelta::Append {
                    index: *start,
                    values: removed,
                },
            );
        }

        PrimitiveDelta::AppendEntries { entries } => {
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
                PrimitiveDelta::RemoveKeys {
                    keys: entries.iter().map(|(name, _)| name.clone()).collect(),
                },
            );
        }

        PrimitiveDelta::RemoveKeys { keys } => {
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
            inverse_operations.insert(
                path.clone(),
                PrimitiveDelta::AppendEntries { entries: removed },
            );
        }
    }

    Ok(())
}
