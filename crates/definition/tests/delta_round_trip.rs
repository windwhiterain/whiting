//! Round-trip checks for the inverse-delta contract.
//!
//! Every assertion here guards the invariant the whole workspace rests on: applying a
//! delta and then applying the delta it returns must restore the original value exactly.

use std::collections::BTreeMap;

use whiting_definition::{
    Attributes, Definition, DefinitionIdentifier, Delta, DeltaError, Path, PrimitiveDelta,
    Registry, RegistryError, Schema, SchemaIdentifier, SchemaKind, Value, Version,
};

fn walk() -> DefinitionIdentifier {
    DefinitionIdentifier::new("walk")
}

fn step_path() -> Path {
    Path::root().key("steps")
}

fn label_path() -> Path {
    Path::root().key("label")
}

fn registry_with(steps: Vec<i64>) -> Registry {
    let mut registry = Registry::new();
    registry.put_schema(
        SchemaIdentifier::new("counter"),
        Schema::new(SchemaKind::structure([
            (
                "steps".to_owned(),
                Schema::new(SchemaKind::list(Schema::new(SchemaKind::Int))),
            ),
            ("label".to_owned(), Schema::new(SchemaKind::String)),
        ])),
    );
    registry
        .insert_definition(Definition::new(
            walk(),
            SchemaIdentifier::new("counter"),
            Value::structure([
                (
                    "steps".to_owned(),
                    Value::List(steps.into_iter().map(Value::Int).collect()),
                ),
                ("label".to_owned(), Value::String("walk".to_owned())),
            ]),
        ))
        .expect("the definition is new");
    registry
}

fn steps(registry: &Registry) -> Vec<i64> {
    registry
        .definition(&walk())
        .expect("the definition is registered")
        .at(&step_path())
        .expect("steps resolves")
        .as_list()
        .expect("steps is a list")
        .iter()
        .map(|value| value.as_int().expect("an int"))
        .collect()
}

fn delta(base_version: u64, path: Path, primitive: PrimitiveDelta) -> Delta {
    Delta::with_operation(walk(), Version::new(base_version), path, primitive)
}

#[test]
fn replace_is_undone_exactly() {
    let mut registry = registry_with(vec![1]);
    let before = steps(&registry);

    let forward = delta(
        0,
        step_path(),
        PrimitiveDelta::Replace(Value::List(vec![Value::Int(7), Value::Int(8)])),
    );
    registry.apply(&forward).expect("the delta applies");
    assert_eq!(steps(&registry), vec![7, 8]);

    registry.undo(&walk()).expect("the apply is undoable");
    assert_eq!(steps(&registry), before);
}

#[test]
fn indexed_append_inverts_to_exactly_the_inserted_range() {
    let mut registry = registry_with(vec![1, 4]);

    let forward = delta(
        0,
        step_path(),
        PrimitiveDelta::append_values(1, [Value::Int(2), Value::Int(3)]),
    );
    let inverse = registry.apply(&forward).expect("the delta applies");
    assert_eq!(steps(&registry), vec![1, 2, 3, 4]);

    assert_eq!(
        inverse.operations.get(&step_path()),
        Some(&PrimitiveDelta::remove_indices(1, 2))
    );

    registry.undo(&walk()).expect("undoable");
    assert_eq!(steps(&registry), vec![1, 4]);
}

#[test]
fn a_mid_list_removal_restores_position() {
    let mut registry = registry_with(vec![1, 2, 3, 4, 5]);

    let forward = delta(0, step_path(), PrimitiveDelta::remove_indices(1, 1));
    registry.apply(&forward).expect("the delta applies");
    assert_eq!(steps(&registry), vec![1, 3, 4, 5]);

    // The point of the exercise: undo restores the element at its original index rather
    // than appending it to the tail, which a tail-only append could not do.
    registry.undo(&walk()).expect("undoable");
    assert_eq!(steps(&registry), vec![1, 2, 3, 4, 5]);
}

#[test]
fn dict_entry_changes_invert_exactly() {
    let mut registry = Registry::new();
    registry.put_schema(
        SchemaIdentifier::new("settings"),
        Schema::new(SchemaKind::structure([(
            "options".to_owned(),
            Schema::new(SchemaKind::dictionary(Schema::new(SchemaKind::Int))),
        )])),
    );
    registry
        .insert_definition(Definition::new(
            walk(),
            SchemaIdentifier::new("settings"),
            Value::structure([(
                "options".to_owned(),
                Value::dictionary([("width".to_owned(), Value::Int(4))]),
            )]),
        ))
        .expect("the definition is new");
    let options_path = Path::root().key("options");

    let forward = delta(
        0,
        options_path.clone(),
        PrimitiveDelta::append_entries([("height".to_owned(), Value::Int(2))]),
    );
    let inverse = registry.apply(&forward).expect("the delta applies");
    assert_eq!(
        inverse.operations.get(&options_path),
        Some(&PrimitiveDelta::remove_keys(["height".to_owned()]))
    );

    registry.undo(&walk()).expect("undoable");
    // The resolved value is bound before it is read, because a path may answer with a value it
    // decoded rather than a borrow of the document.
    let resolved = registry
        .definition(&walk())
        .expect("registered")
        .at(&options_path)
        .expect("resolves");
    let options = resolved.as_dict().expect("a dict");
    assert_eq!(options.len(), 1);
    assert!(options.contains_key("width"));
}

#[test]
fn apply_is_atomic_when_a_later_operation_fails() {
    let mut registry = Registry::new();
    registry.put_schema(
        SchemaIdentifier::new("pair"),
        Schema::new(SchemaKind::structure([
            (
                "first".to_owned(),
                Schema::new(SchemaKind::list(Schema::new(SchemaKind::Int))),
            ),
            (
                "second".to_owned(),
                Schema::new(SchemaKind::list(Schema::new(SchemaKind::Int))),
            ),
        ])),
    );
    let value = Value::structure([
        (
            "first".to_owned(),
            Value::List(vec![Value::Int(1), Value::Int(2)]),
        ),
        (
            "second".to_owned(),
            Value::List(vec![Value::Int(1), Value::Int(2)]),
        ),
    ]);
    registry
        .insert_definition(Definition::new(
            walk(),
            SchemaIdentifier::new("pair"),
            value.clone(),
        ))
        .expect("the definition is new");

    let mut operations = BTreeMap::new();
    // Sorted order means this valid append runs before the failing removal below.
    operations.insert(
        Path::root().key("first"),
        PrimitiveDelta::append_values(2, [Value::Int(9)]),
    );
    operations.insert(
        Path::root().key("second"),
        PrimitiveDelta::remove_indices(1, 5),
    );
    let forward = Delta::new(walk(), Version::INITIAL, operations);

    registry
        .apply(&forward)
        .expect_err("the range is out of bounds");

    let definition = registry.definition(&walk()).expect("registered");
    assert_eq!(
        definition.value, value,
        "the earlier operation must be rolled back"
    );
    assert_eq!(
        definition.version,
        Version::INITIAL,
        "the version must not advance"
    );
}

#[test]
fn apply_is_atomic_when_the_result_fails_validation() {
    let mut registry = registry_with(vec![1]);
    let before = registry
        .definition(&walk())
        .expect("registered")
        .value
        .clone();

    let forward = delta(
        0,
        step_path(),
        // A list of strings does not satisfy a list of ints.
        PrimitiveDelta::Replace(Value::List(vec![Value::String("nope".to_owned())])),
    );
    registry
        .apply(&forward)
        .expect_err("validation rejects the result");

    let definition = registry.definition(&walk()).expect("registered");
    assert_eq!(definition.value, before);
    assert_eq!(definition.version, Version::INITIAL);
}

#[test]
fn a_stale_delta_is_rejected_as_a_conflict() {
    let mut registry = registry_with(vec![1]);
    let first = delta(
        0,
        label_path(),
        PrimitiveDelta::Replace(Value::String("stroll".to_owned())),
    );
    registry.apply(&first).expect("the first delta applies");

    let stale = delta(
        0,
        label_path(),
        PrimitiveDelta::Replace(Value::String("run".to_owned())),
    );
    let error = registry
        .apply(&stale)
        .expect_err("the version has moved on");
    assert!(
        matches!(error, RegistryError::Delta(DeltaError::Conflict { .. })),
        "expected a conflict, got {error}"
    );
}

#[test]
fn redo_replays_the_forward_delta() {
    let mut registry = registry_with(vec![1]);
    let forward = delta(0, step_path(), PrimitiveDelta::remove_indices(0, 1));
    registry.apply(&forward).expect("the delta applies");
    assert_eq!(steps(&registry), Vec::<i64>::new());

    registry.undo(&walk()).expect("undoable");
    registry.redo(&walk()).expect("redoable");
    assert_eq!(steps(&registry), Vec::<i64>::new());
}

#[test]
fn a_struct_is_closed() {
    let mut registry = Registry::new();
    registry.put_schema(
        SchemaIdentifier::new("closed"),
        Schema::new(SchemaKind::structure([(
            "known".to_owned(),
            Schema::new(SchemaKind::String),
        )])),
    );
    let schema = registry
        .schema(&SchemaIdentifier::new("closed"))
        .expect("registered")
        .clone();
    let value = Value::structure([
        ("known".to_owned(), Value::String("yes".to_owned())),
        ("surprise".to_owned(), Value::String("no".to_owned())),
    ]);
    assert!(
        schema.validate(&value).is_err(),
        "a field absent from the schema must be rejected"
    );
}

#[test]
fn a_declared_default_is_substituted_for_readers_only() {
    let mut registry = Registry::new();
    registry.put_schema(
        SchemaIdentifier::new("with_default"),
        Schema::new(SchemaKind::structure([
            ("name".to_owned(), Schema::new(SchemaKind::String)),
            (
                "speed".to_owned(),
                Schema::new(SchemaKind::Float)
                    .with_attributes(Attributes::new().with_default(Value::Float(1.0))),
            ),
        ])),
    );
    let definition = Definition::new(
        walk(),
        SchemaIdentifier::new("with_default"),
        Value::structure([("name".to_owned(), Value::String("mover".to_owned()))]),
    );
    definition
        .validate(&registry)
        .expect("a field with a default is not required");

    let defaulted = definition
        .defaulted_value(&registry)
        .expect("the schema resolves");
    assert_eq!(
        defaulted
            .as_struct()
            .and_then(|fields| fields.get("speed"))
            .and_then(Value::as_float),
        Some(1.0)
    );
    assert!(
        !definition
            .value
            .as_struct()
            .expect("a struct")
            .contains_key("speed"),
        "the authored value must not be rewritten"
    );
}
