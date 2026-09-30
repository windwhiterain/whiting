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
    /// delta whose path *reaches* a reference is refused here and belongs on
    /// [`Loaded::patch`](crate::Loaded::patch) instead. The check is per path rather than per
    /// document, so a document may name something and still take a delta that touches none of what
    /// it named. A delta may also *introduce* a reference, because writing a spelling is an
    /// ordinary write of a value and nothing traverses it.
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
    /// // A path that reaches the reference is refused, and the error names the slot it reached.
    /// let mut at_the_reference = BTreeMap::new();
    /// at_the_reference.insert(
    ///     Path::root().key("right"),
    ///     PrimitiveDelta::Replace(Value::Int(9)),
    /// );
    /// assert!(matches!(
    ///     registry.apply(&Delta::new(identifier.clone(), 0u64.into(), at_the_reference)),
    ///     Err(RegistryError::Delta(DeltaError::ReferenceInSerialized { .. }))
    /// ));
    ///
    /// // A path that reaches none applies, and the reference goes on naming what it named: the
    /// // node it points at is the one that changed.
    /// let mut elsewhere = BTreeMap::new();
    /// elsewhere.insert(
    ///     Path::root().key("left"),
    ///     PrimitiveDelta::Replace(Value::Int(9)),
    /// );
    /// registry
    ///     .apply(&Delta::new(identifier, 0u64.into(), elsewhere))
    ///     .expect("a path that reaches no reference applies");
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

        // A path that runs through a reference names the node the reference points at, which the
        // serialized form does not hold: it holds the spelling. A delta that reaches one is
        // therefore refused, because applying it here would mean something different from applying
        // it to a loaded generation, which is where such a delta belongs. A path that meets none is
        // unambiguous here, which is what lets a document name something and still take a delta
        // that touches none of what it named.
        let schema = registry.schema(&definition.schema_identifier)?;
        for path in self.operations.keys() {
            if let Some(met) = Loaded::crossed_reference(&definition.value, schema, path) {
                return Err(DeltaError::ReferenceInSerialized { path: met });
            }
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

    // The edit is applied through `Path::update`, so a path that reaches into a packed list
    // decodes the one element it names, applies the operation, and writes it back. Editing a
    // single point of a run of a hundred thousand therefore costs one point.
    let outcome = path.update(root, |target| match primitive {
        PrimitiveDelta::Replace(new_value) => {
            let previous = std::mem::replace(target, new_value.clone());
            inverse_operations.insert(path.clone(), PrimitiveDelta::Replace(previous));
            Ok(())
        }

        PrimitiveDelta::Append(Appended::Values { index, values }) => {
            if !target.is_list() {
                return Err(invalid("append needs a list"));
            }
            if target.list_insert(*index, values.clone()).is_none() {
                return Err(invalid("append index is beyond the end of the list"));
            }
            inverse_operations.insert(
                path.clone(),
                PrimitiveDelta::remove_indices(*index, values.len()),
            );
            Ok(())
        }

        PrimitiveDelta::Remove(Removed::Indices { start, count }) => {
            if !target.is_list() {
                return Err(invalid("removal needs a list"));
            }
            let Some(removed) = target.list_remove(*start, *count) else {
                return Err(invalid("removal range is out of bounds for the list"));
            };
            inverse_operations.insert(path.clone(), PrimitiveDelta::append_values(*start, removed));
            Ok(())
        }

        PrimitiveDelta::Append(Appended::Entries { entries }) => {
            let Some(map) = target.as_dict_mut() else {
                return Err(invalid("entry append needs a dict"));
            };
            for (name, value) in entries {
                if map.insert(name.clone(), value.clone()).is_some() {
                    return Err(invalid("entry append found a key that is already present"));
                }
            }
            inverse_operations.insert(
                path.clone(),
                PrimitiveDelta::remove_keys(entries.iter().map(|(name, _)| name.clone())),
            );
            Ok(())
        }

        PrimitiveDelta::Remove(Removed::Keys { keys }) => {
            let Some(map) = target.as_field_map_mut() else {
                return Err(invalid("key removal needs a struct or a dict"));
            };
            let mut removed = Vec::with_capacity(keys.len());
            for name in keys {
                match map.remove(name) {
                    Some(value) => removed.push((name.clone(), value)),
                    None => return Err(invalid("key removal found a key that is absent")),
                }
            }
            inverse_operations.insert(path.clone(), PrimitiveDelta::append_entries(removed));
            Ok(())
        }
    });

    // The edit itself may have refused, in which case its error is the operation's error; a path
    // that does not resolve at all is reported as such.
    match outcome {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error),
        Err(reason) => Err(DeltaError::UnresolvedPath {
            path: path.clone(),
            reason: reason.to_string(),
        }),
    }
}
