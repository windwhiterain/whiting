//! Reading and writing device state to a file on demand.
//!
//! State is authoritative in memory. A snapshot is a point-in-time copy written when a
//! caller asks for one, not a log and not a watched file, so nothing here runs in the
//! background.

use std::fs;
use std::path::Path;

use definition::{Definition, Registry, Schema, Value};
use facet::Facet;

use crate::state::{Device, DeviceError};

/// A named schema inside a snapshot.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct NamedSchema {
    /// Name the schema is registered under.
    pub name: String,
    /// The schema document itself.
    pub schema: Schema,
}

/// A named definition inside a snapshot.
///
/// # Invariant
///
/// The version is restored as written. A snapshot therefore reinstates the exact state
/// the device was in, including what is undoable, because the version is what the next
/// delta must match.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct NamedDefinition {
    /// Name the definition is registered under.
    pub name: String,
    /// Name of the schema the value is verified against.
    pub schema: String,
    /// The version to restore.
    pub version: u64,
    /// The value itself.
    pub value: Value,
}

/// The on-disk form of the whole device state.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Snapshot {
    /// Every registered schema.
    pub schemas: Vec<NamedSchema>,
    /// Every registered definition, in name order.
    pub definitions: Vec<NamedDefinition>,
}

impl Snapshot {
    /// Captures the current registry contents.
    pub fn capture(registry: &Registry) -> Self {
        let schemas = registry
            .schema_identifiers()
            .into_iter()
            .filter_map(|name| {
                registry.schema(&name).ok().map(|schema| NamedSchema {
                    name: name.into_string(),
                    schema: schema.clone(),
                })
            })
            .collect();

        let definitions = registry
            .definition_identifiers()
            .into_iter()
            .filter_map(|name| {
                registry
                    .definition(&name)
                    .ok()
                    .map(|definition| NamedDefinition {
                        name: name.into_string(),
                        schema: definition.schema_identifier.as_str().to_owned(),
                        version: definition.version.get(),
                        value: definition.value.clone(),
                    })
            })
            .collect();

        Self {
            schemas,
            definitions,
        }
    }

    /// Rebuilds registry contents from a snapshot, replacing whatever was registered.
    pub fn restore(&self, registry: &mut Registry) {
        for named in &self.schemas {
            registry.put_schema(
                definition::SchemaIdentifier::new(named.name.clone()),
                named.schema.clone(),
            );
        }
        for named in &self.definitions {
            let mut definition = Definition::new(
                definition::DefinitionIdentifier::new(named.name.clone()),
                definition::SchemaIdentifier::new(named.schema.clone()),
                named.value.clone(),
            );
            definition.version = definition::Version::new(named.version);
            // A snapshot that names a definition already present replaces it, because a
            // restore is a whole-state operation rather than an append.
            if registry
                .definition(&definition.definition_identifier)
                .is_ok()
            {
                let _ = registry.remove_definition(&definition.definition_identifier);
            }
            let _ = registry.insert_definition(definition);
        }
    }
}

/// Writes the device state to `path` as pretty printed json.
pub fn write(device: &Device, path: &Path) -> Result<(), DeviceError> {
    let snapshot = Snapshot::capture(device.registry());
    let text =
        facet_json::to_string_pretty(&snapshot).map_err(|error| DeviceError::SnapshotWrite {
            path: path.display().to_string(),
            reason: error.to_string(),
        })?;
    fs::write(path, text).map_err(|error| DeviceError::SnapshotWrite {
        path: path.display().to_string(),
        reason: error.to_string(),
    })
}

/// Reads device state from `path`, replacing the registry contents.
pub fn read(registry: &mut Registry, path: &Path) -> Result<(), DeviceError> {
    let text = fs::read_to_string(path).map_err(|error| DeviceError::SnapshotRead {
        path: path.display().to_string(),
        reason: error.to_string(),
    })?;
    let snapshot: Snapshot =
        facet_json::from_str(&text).map_err(|error| DeviceError::SnapshotRead {
            path: path.display().to_string(),
            reason: error.to_string(),
        })?;
    snapshot.restore(registry);
    Ok(())
}
