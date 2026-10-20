//! What a compile says about a frame it believes is degenerate.
//!
//! # What these prove
//!
//! Three agents ran the same task against this framework and each of them put a camera inside
//! geometry: a camera in a pond got a water jar filling the frame, a ridge grew on the camera
//! side and two cameras sat inside a hundred metre hill, and one camera was repositioned four
//! times over twelve rounds without ever being right. Every one of those is a fact the compile
//! already holds at the moment it emits the frame, and this is where the three notes about it
//! are pinned:
//!
//! - **the eye is inside an instance's bounds** — named with the instance and how far in;
//! - **one instance holds the frame, between the eye and the target** — named with its share;
//! - **the target is nearer than the near plane** — named with both numbers.
//!
//! None of them refuses to compile, because a scene may legitimately want a camera inside a
//! volume. The policy is the page pool's, generalised: *a machine being too small is a fact
//! about this run; a wrong image on every machine is a fact about the document.* All three are
//! facts about this run, so all three are named and all three are drawn.
//!
//! The other half of the contract is here too, and it is the half that decides whether the
//! notes are ever read: **a frame that is fine says nothing at all.** A backdrop wall that
//! covers the whole frame, a ground reaching past it in every direction, a fitted camera and a
//! document that names no camera are all ordinary, and the document under test is the evidence
//! that none of them is reported.

use super::support::triple;
use definition::{
    Definition, DefinitionIdentifier, FieldMap, Path, Registry, SchemaIdentifier, Value,
};
/// The field a scene holds its instances under, which standard does not re export.
const INSTANCES: &str = "instances";
use whiting_document::kit::camera_placed;
use whiting_scene::standard::{CAMERAS, EYE, TARGET};
use whiting_scene::{
    CompileContext, Compiler, STANDARD_SCENE_SCHEMA, StandardScene, register_standard_scene_schema,
};

/// The example document: one mesh of four instances, which is the smallest document a test can
/// bury a camera in.
fn example_document() -> Value {
    whiting_scene::standard::example_standard_scene()
}

#[test]
fn an_eye_inside_an_instance_is_named_with_the_instance_and_how_far_in() {
    // The eye at 0.3 above the floor of a two unit cube, three tenths of the way in, which is
    // the dsf pond: a camera standing in the water rather than beside it.
    let scene = isolating(&with_a_cube(&example_document()), "quad");
    let scene = with_camera(&scene, &placed([0.4, -0.7, 0.4], [0.0, 0.0, -10.0]));

    let notes = camera_notes_of(&scene);
    assert_eq!(
        notes.len(),
        1,
        "one box holds the eye and it is named once: {notes:#?}"
    );
    let note = &notes[0];
    assert_eq!(
        note.path,
        Path::root().key(CAMERAS).index(0).key(EYE),
        "the note points at the field to change, which is the eye and not the instance"
    );
    assert!(
        note.message.contains("`quad`") && note.message.contains("0.300"),
        "it names the instance and how far in it is: {}",
        note.message
    );
    assert!(
        note.message.contains("draws it as written"),
        "and it says the compile did not refuse, because a camera inside a volume is a choice: \
         {}",
        note.message
    );
}

#[test]
fn one_instance_holding_the_frame_between_the_eye_and_the_target_is_named_with_its_share() {
    // A wall twenty units across, one unit thick, eight units in front of an eye looking at
    // something ten units away: it holds the whole frame and it is in front of the subject.
    let scene = isolating(&with_a_cube(&example_document()), "quad");
    let scene = with_transform(&scene, "quad", [0.0, 0.0, -8.0], [20.0, 20.0, 1.0]);
    let scene = with_camera(&scene, &placed([0.0, 0.0, 0.0], [0.0, 0.0, -10.0]));

    let notes = camera_notes_of(&scene);
    assert_eq!(notes.len(), 1, "one instance holds the frame: {notes:#?}");
    let note = &notes[0];
    assert_eq!(
        note.path,
        Path::root().key(INSTANCES).key("lighting").index(0),
        "a note about an instance points at the instance: a note that names a thing and points \
         at the camera sends the reader past it"
    );
    assert!(
        note.message.contains("`quad`")
            && note.message.contains("100.0%")
            && note.message.contains("between the eye"),
        "it names the instance, the share and why the share matters: {}",
        note.message
    );
    assert!(
        note.message.contains("upper bound"),
        "and it says the share is the box projected rather than the pixels it drew: {}",
        note.message
    );
}

#[test]
fn a_backdrop_behind_the_target_holds_the_frame_and_is_not_reported() {
    // The same wall, moved behind what the camera was aimed at. It still covers every pixel —
    // a still life is a backdrop and a backdrop fills the frame — and it is not what the frame
    // is *of*, so the note does not fire. This is the case that decides the bar: one instance
    // covering the frame is only a fault when it stands in front of the subject.
    let scene = isolating(&with_a_cube(&example_document()), "quad");
    let scene = with_transform(&scene, "quad", [0.0, 0.0, -20.0], [20.0, 20.0, 1.0]);
    let scene = with_camera(&scene, &placed([0.0, 0.0, 0.0], [0.0, 0.0, -2.0]));

    assert_eq!(
        camera_notes_of(&scene),
        Vec::new(),
        "a wall behind the subject is a backdrop, and a note about one is a note nobody reads"
    );
}

#[test]
fn a_target_nearer_than_the_near_plane_is_named_with_both_numbers() {
    // An eye ten units from the origin, a target two units in front of it, and a near plane
    // five units out: the camera was aimed at something the frame cannot hold.
    let scene = isolating(&with_a_cube(&example_document()), "quad");
    let scene = with_camera(
        &scene,
        &Value::structure([
            ("eye".to_owned(), triple([0.0, 0.0, 10.0])),
            ("target".to_owned(), triple([0.0, 0.0, 8.0])),
            (
                "field_of_view".to_owned(),
                Value::Float(std::f64::consts::FRAC_PI_3),
            ),
            ("projection".to_owned(), Value::variant("Perspective", [])),
            ("near".to_owned(), Value::Float(5.0)),
            ("far".to_owned(), Value::Float(100.0)),
        ]),
    );

    let notes = camera_notes_of(&scene);
    assert_eq!(notes.len(), 1, "one note, about the target: {notes:#?}");
    let note = &notes[0];
    assert_eq!(
        note.path,
        Path::root().key(CAMERAS).index(0).key(TARGET),
        "the target is what is wrong, so that is the field the note points at"
    );
    for named in ["2.000", "5.000"] {
        assert!(
            note.message.contains(named),
            "the note carries both numbers, the distance aimed at and the plane: {}",
            note.message
        );
    }
}

#[test]
fn a_scene_naming_no_camera_is_not_told_about_the_convention_one() {
    // The document names no camera, so this is the convention: pulled back until the whole
    // box fits, which puts the eye three half widths off the centre of the scene and outside
    // every box in it. A note here would point at a field the document does not have.
    assert_eq!(
        camera_notes_of(&example_document()),
        Vec::new(),
        "a camera nobody wrote is not a fault anybody can fix"
    );

    // The reason, pinned rather than asserted in prose: the convention eye is outside the box
    // the camera frames, so no instance's bounds can hold it.
    let bounds = whiting_scene::standard::Bounds::around([0.0, 0.0, 0.0], 4.0);
    let convention = whiting_scene::standard::Camera::read(None, &Path::root(), &bounds, 1.0)
        .expect("the convention camera reads")
        .projection();
    assert_eq!(
        bounds.depth_of(convention.eye),
        None,
        "the eye stands off the box it frames, which is why the notes say nothing here"
    );
    assert!(
        convention.distance_to([0.0, 0.0, 0.0]) > convention.near,
        "and the target it aims at is beyond the near plane, so even the third note is quiet"
    );
}

/// The whole of the picture invariant, as far as a compile can be asked about it: the notes
/// travel beside the documents and are not in them. A compile that raises one emits the same
/// passes, the same instance entries and the same delta it would have emitted silently, and
/// the harness's own manifest check is what closes the loop on the pixels.
#[test]
fn a_scene_that_raises_a_note_still_emits_the_pipeline_it_always_did() {
    let quiet = camera_compiled(&with_camera(
        &isolating(&with_a_cube(&example_document()), "quad"),
        &placed([0.0, 0.0, 20.0], [0.0, 0.0, -10.0]),
    ));
    let buried = camera_compiled(&with_camera(
        &isolating(&with_a_cube(&example_document()), "quad"),
        &placed([0.4, -0.7, 0.4], [0.0, 0.0, -10.0]),
    ));

    assert!(
        quiet.notes.is_empty(),
        "the eye is in open air here: {:#?}",
        quiet.notes
    );
    assert_eq!(
        buried.notes.len(),
        1,
        "and in the box there: {:#?}",
        buried.notes
    );
    // The pass list is the pipeline the scene describes, and the instance entries are what a
    // pass reads. Both are the same either way: a note is not a pass and not a payload, so it
    // cannot have moved a pass count or an entry name.
    let labels = |output: &whiting_scene::CompileOutput| {
        graphics::passes(&output.graphics.value)
            .expect("the passes read")
            .iter()
            .map(|pass| pass.label().expect("a pass is labelled").to_owned())
            .collect::<Vec<String>>()
    };
    assert_eq!(
        labels(&buried),
        labels(&quiet),
        "the same passes, in the same order"
    );
    let entries = |output: &whiting_scene::CompileOutput| {
        let resolved = output
            .instance
            .value
            .at(&Path::root().key(graphics::ENTRIES))
            .expect("the instance document carries its entries");
        resolved.as_value().as_dict().map(|entries| entries.len())
    };
    assert_eq!(
        entries(&buried),
        entries(&quiet),
        "and the same instance entries, which is what the passes read"
    );
}

/// Returns the notes a compile of this scene raises, which is the only thing these tests read.
/// Returns the notes a compile of this scene raises **about its camera**, which is what this
/// file is about.
///
/// # Why the list is filtered
///
/// A compile now also says what the scene's **size** costs — a falloff measured at the half
/// diagonal of the same box these notes are read off — and that note points at `instances`
/// rather than at a camera. Those two kinds are separate facts with separate paths, and this
/// file asserts exactly which camera notes a scene raises. Filtering here is what keeps "a
/// scene that buries nothing says nothing" a statement about cameras rather than about every
/// note the compiler now knows how to raise.
fn camera_notes_of(scene: &Value) -> Vec<whiting_scene::CompileNote> {
    camera_compiled(scene)
        .notes
        .into_iter()
        .filter(|note| note.path != Path::root().key("instances"))
        .collect()
}

/// Compiles a scene the way a host would, and returns what it emitted.
fn camera_compiled(scene: &Value) -> whiting_scene::CompileOutput {
    let mut registry = Registry::new();
    registry.put_schema(
        SchemaIdentifier::new(graphics::GRAPHICS_SCHEMA),
        graphics::graphics_schema(),
    );
    registry.put_schema(
        SchemaIdentifier::new(graphics::INSTANCE_SCHEMA),
        graphics::instance_schema(),
    );
    register_standard_scene_schema(&mut registry);
    let source = Definition::new(
        DefinitionIdentifier::new("camera-notes.source"),
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        scene.clone(),
    );
    source
        .validate(&registry)
        .expect("the scene validates against the schema");
    StandardScene::new(
        DefinitionIdentifier::new("camera-notes.graphics"),
        DefinitionIdentifier::new("camera-notes.instance"),
    )
    .compile(&CompileContext {
        scene: &source,
        previous_graphics: None,
        previous_instance: None,
        registry: &registry,
        camera: 0,
        // No machine behind this compile, so it is made against the floor: a note in this file
        // is a note about a camera rather than a count that asked past a ceiling.
        limits: None,
    })
    .expect("the scene compiles")
}

/// Returns a camera placed rather than fitted, with both depth planes written down — the only
/// kind a test can aim precisely, because a camera with no planes is pulled back until the whole
/// box fits and the eye it uses is not the one the document wrote.
fn placed(eye: [f64; 3], target: [f64; 3]) -> Value {
    camera_placed(eye, target, std::f64::consts::FRAC_PI_3, 0.1, 1000.0)
}

/// Returns the scene with its one camera written in place of whatever it had.
fn with_camera(scene: &Value, camera: &Value) -> Value {
    let mut scene = scene.clone();
    scene
        .as_field_map_mut()
        .expect("a scene is a struct")
        .insert(CAMERAS.to_owned(), Value::List(vec![camera.clone()]));
    scene
}

/// Returns the scene with one instance's transform written, by label.
fn with_transform(scene: &Value, label: &str, translation: [f64; 3], scale: [f64; 3]) -> Value {
    let mut scene = scene.clone();
    let instances = instances_of(&mut scene);
    let mut found = false;
    for bucket in instances.values_mut() {
        for instance in bucket.as_list_mut().expect("a bucket is a list") {
            let fields = instance
                .as_field_map_mut()
                .expect("an instance is a struct");
            if fields.get("label").and_then(Value::as_str) != Some(label) {
                continue;
            }
            found = true;
            let transform = fields
                .get_mut("transform")
                .and_then(Value::as_field_map_mut)
                .expect("an instance has a transform");
            transform.insert("translation".to_owned(), triple(translation));
            transform.insert("scale".to_owned(), triple(scale));
        }
    }
    assert!(found, "the scene has an instance labelled `{label}`");
    scene
}

/// Returns the scene with every instance but one moved a long way off.
///
/// The example's four instances overlap, so an eye inside one of their boxes would be inside
/// three; moving the other three a hundred units off leaves exactly one box to be inside, and
/// the test can then say which instance it meant.
fn isolating(scene: &Value, keep: &str) -> Value {
    let mut scene = scene.clone();
    for bucket in instances_of(&mut scene).values_mut() {
        for instance in bucket.as_list_mut().expect("a bucket is a list") {
            let fields = instance
                .as_field_map_mut()
                .expect("an instance is a struct");
            if fields.get("label").and_then(Value::as_str) == Some(keep) {
                continue;
            }
            let transform = fields
                .get_mut("transform")
                .and_then(Value::as_field_map_mut)
                .expect("an instance has a transform");
            let [x, y, z] = translation_of(transform);
            transform.insert("translation".to_owned(), triple([x + 100.0, y, z]));
        }
    }
    scene
}

/// Returns the translation a transform carries, read as three numbers.
fn translation_of(transform: &FieldMap) -> [f64; 3] {
    let mut at = [0.0; 3];
    if let Some(Value::List(components)) = transform.get("translation") {
        for (axis, component) in components.iter().take(3).enumerate() {
            at[axis] = component
                .as_float()
                .or_else(|| component.as_int().map(|number| number as f64))
                .unwrap_or(0.0);
        }
    }
    at
}

/// Returns the example scene with its one mesh's points replaced by a cube's.
///
/// A flat triangle has no thickness, so an eye inside its box is always exactly on one of its
/// faces and "how far in" is always nothing. A cube gives the number the note is for, and it is
/// four triangles rather than eight points so that the mesh is one a renderer could draw.
fn with_a_cube(scene: &Value) -> Value {
    const FACE: [[[f64; 3]; 3]; 4] = [
        [[-1.0, -1.0, 1.0], [1.0, -1.0, 1.0], [-1.0, 1.0, 1.0]],
        [[1.0, -1.0, -1.0], [-1.0, -1.0, -1.0], [1.0, 1.0, -1.0]],
        [[1.0, -1.0, 1.0], [1.0, -1.0, -1.0], [1.0, 1.0, 1.0]],
        [[-1.0, -1.0, -1.0], [-1.0, -1.0, 1.0], [-1.0, 1.0, -1.0]],
    ];
    let points = FACE
        .iter()
        .flatten()
        .map(|position| {
            Value::structure([
                ("normal".to_owned(), triple([0.0, 1.0, 0.0])),
                ("position".to_owned(), triple(*position)),
            ])
        })
        .collect();
    let mut scene = scene.clone();
    let mesh = scene
        .as_field_map_mut()
        .and_then(|fields| fields.get_mut("meshes"))
        .and_then(Value::as_list_mut)
        .and_then(|meshes| meshes.first_mut())
        .expect("the example has a mesh");
    mesh.as_field_map_mut()
        .expect("a mesh is a struct")
        .insert("points".to_owned(), Value::List(points));
    scene
}

/// Returns the `instances` field of a scene, ready to be edited in place.
fn instances_of(scene: &mut Value) -> &mut FieldMap {
    scene
        .as_field_map_mut()
        .and_then(|fields| fields.get_mut(INSTANCES))
        .and_then(Value::as_dict_mut)
        .expect("a scene names its instances")
}
