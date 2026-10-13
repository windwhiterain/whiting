//! Where a Cycles camera points, read off the document instead of off a render.
//!
//! # What this is for
//!
//! `a81b203` changed the rotation this emitter writes for a camera, to cancel a scale in world
//! space that the loader does not compute. Every camera in the gallery was turned away from its
//! subject by `2 * atan2(forward.x, forward.z)`, `sunlight` fell from drawing 59.597% of its frame
//! to 10.372%, and `foldyard` lost its courtyard. It was reverted in `afb0c0b`, and **110 tests
//! stayed green throughout** because `cycles_export` had none.
//!
//! The only thing that noticed was a person rendering eight scenes and reading the numbers. This
//! is that check without the rendering: it composes the emitted `translate`/`rotate`/`scale` the
//! way `src/app/cycles_xml.cpp:694-710` does, and asks whether the camera still points where the
//! scene document said.
//!
//! # Why it needs no Cycles
//!
//! The fault was a claim about how a third-party loader composes a transform, and that claim is
//! arithmetic. The loader right-multiplies — `tfm = tfm * translate`, then `tfm = tfm * rotate`,
//! then `tfm = tfm * scale` — so a transform written at the top of a scope is `T * R * S`, and a
//! camera's basis is the columns of `R * S`. A test that reproduces those three lines and
//! multiplies the `+Z` column out catches a wrong composition before a process is started.
//!
//! # Why forward and nothing else
//!
//! Two other properties of the same basis were measured against both the current export and the
//! regressed one, which differ in the camera's rotation and in nothing else:
//!
//! | | current export | the regression's |
//! |---|---|---|
//! | determinant of the basis | -1.0 | -1.0 — the same |
//! | `up` against world up | +0.9809 | +0.9809 — the same |
//! | forward against `normalize(target - eye)` | +0.0000 degrees | **+66.9472 degrees** |
//!
//! Two of the three held all the way through, and they are the two that look like the diagnosis:
//! the determinant is negative whenever a mirror is written at all, whatever the rotation says,
//! and `up` was never tipped — `a81b203` mirrored in `x`, which is a turn about the view axis
//! rather than a roll. **Only the forward identifies it**, and it needs the scene's own eye and
//! target, so it cannot be a property of the transform alone.
//!
//! See `.agents/proposals/the-cycles-emitter-has-no-tests.md`.

use definition::Value;
use whiting_scene::cycles_export::{EmitOptions, emit_to_string};
use whiting_scene::standard::CAMERAS;

/// The emitter's camera points where the scene document told it to.
///
/// Three placements rather than one, because a composition that is wrong in a way that always
/// turns the camera about its own view axis shows nothing on a camera aimed straight down an
/// axis: a mis-signed yaw is invisible until the camera is yawed. So one camera looks along an
/// axis, one is yawed hard to the left, and one is yawed right and pitched at the same time —
/// which is the composition order `T * R * S` actually has to get right.
#[test]
fn the_emitted_camera_points_at_the_target_the_scene_named() {
    for (eye, target) in [
        ([6.6, 3.1, 8.4], [-0.2, 0.7, -1.6]),
        ([0.0, 0.7, 2.6], [-4.0, 0.1, -8.0]),
        ([-3.0, 5.0, 2.0], [6.0, -1.0, -7.0]),
    ] {
        let document = emit_to_string(&scene_placing(eye, target), &EmitOptions::default()).expect(
            "a scene holding nothing but a camera emits: the loader reads a camera from a \
                 transform scope whether or not anything else is in the document",
        );

        let pointed_at = cycles_direction(normalised([
            target[0] - eye[0],
            target[1] - eye[1],
            target[2] - eye[2],
        ]));
        let points_at = loader_forward(&document);

        let angle = angle_between_degrees(points_at, pointed_at).to_degrees();
        assert!(
            angle < 1e-6,
            "a camera at {eye:?} aimed at {target:?} is written looking at {points_at:?}, which is \
             {angle} degrees from the direction the document named ({pointed_at:?}). The document \
             says:\n{}",
            camera_scope(&document),
        );
    }
}

/// Returns a scene holding nothing but one camera, placed rather than fitted.
///
/// Placed rather than fitted because a camera with no target is pulled back until the whole scene
/// fits, and the eye it ends up with is not the one written down. There is no scene to fit here,
/// so the numbers this test reasons about have to be the numbers it wrote.
///
/// The document carries no mesh, no material and no light. They are not stubbed out because the
/// camera's transform is written before any of them and reads none of them, and a scene that
/// carried a cube would be a second thing for this file to keep correct.
fn scene_placing(eye: [f64; 3], target: [f64; 3]) -> Value {
    Value::structure([(
        CAMERAS.to_owned(),
        Value::List(vec![Value::structure([
            ("eye".to_owned(), triple(eye)),
            ("target".to_owned(), triple(target)),
            (
                "field_of_view".to_owned(),
                Value::Float(std::f64::consts::FRAC_PI_3),
            ),
            ("projection".to_owned(), Value::variant("Perspective", [])),
            ("near".to_owned(), Value::Float(0.1)),
            ("far".to_owned(), Value::Float(1000.0)),
        ])]),
    )])
}

fn triple(vector: [f64; 3]) -> Value {
    Value::List(vector.into_iter().map(Value::Float).collect())
}

/// Returns the `<transform>` element that opens the camera's scope.
///
/// The loader takes a camera's transform from the **enclosing** scope and never reads the
/// `<camera>` element's own transform attributes (`xml_read_camera` calls `set_matrix` with
/// `state.tfm`), so the camera is a sibling of the transform that scopes it. The nearest
/// `<transform` above the camera is therefore the one that decides where it looks.
fn camera_scope(document: &str) -> &str {
    let lines: Vec<&str> = document.lines().collect();
    let camera = lines
        .iter()
        .position(|line| line.contains("<camera "))
        .expect("the document holds a camera");
    lines[..camera]
        .iter()
        .rev()
        .find(|line| line.contains("<transform "))
        .expect("the camera is written inside a transform scope")
}

/// Returns the direction the loader will build the camera's own `+Z` into.
///
/// The three lines of `xml_read_transform` (`src/app/cycles_xml.cpp:694-710`), in the order it
/// executes them:
///
/// ```text
/// tfm = tfm * transform_translate(...)
/// tfm = tfm * transform_rotate(...)
/// tfm = tfm * transform_scale(...)
/// ```
///
/// So the scope's basis is `R * S` — the translation is leftmost and never touches it — and a
/// Cycles camera looks along its own `+Z`, which is the third column. A missing attribute is the
/// identity for that step, which is what the loader does with it.
fn loader_forward(document: &str) -> [f64; 3] {
    let scope = camera_scope(document);
    let rotation = attribute(scope, "rotate").expect("the emitter writes a rotation for a camera");
    let scale = attribute(scope, "scale").unwrap_or_else(|| vec![1.0; 3]);

    let angle = rotation[0];
    let axis = &rotation[1..4];
    let basis = multiply(&rotation_matrix(axis, angle), &diagonal(&scale));
    [basis[0][2], basis[1][2], basis[2][2]]
}

/// Returns one attribute of an element, read as a list of numbers.
///
/// The emitter writes `key="a, b, c"`, and the loader reads it back with a comma-separated
/// float list, so the numbers are read the way the loader would read them rather than by parsing
/// the document as XML — the emitter's own formatting is not what is under test here.
fn attribute(element: &str, name: &str) -> Option<Vec<f64>> {
    let written = format!("{name}=\"");
    let start = element.find(&written)? + written.len();
    let end = start + element[start..].find('"')?;
    Some(
        element[start..end]
            .split(',')
            .map(|number| {
                number.trim().parse::<f64>().unwrap_or_else(|_| {
                    panic!(
                        "{name} holds numbers, and this is not one: {}",
                        number.trim()
                    )
                })
            })
            .collect(),
    )
}

/// Returns the rotation of `degrees` about `axis`, by Rodrigues' formula.
///
/// Cycles takes the angle in **degrees** and the axis in whatever length it was written, so the
/// axis is normalised here rather than assumed unit. A right-handed turn is the convention the
/// loader builds `transform_rotate` from, and it is the one `axis_angle` writes towards.
fn rotation_matrix(axis: &[f64], degrees: f64) -> [[f64; 3]; 3] {
    let length = (axis[0].powi(2) + axis[1].powi(2) + axis[2].powi(2)).sqrt();
    let [x, y, z] = [axis[0] / length, axis[1] / length, axis[2] / length];
    let (sine, cosine) = degrees.to_radians().sin_cos();

    let mut matrix = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            let outer = [x, y, z][row] * [x, y, z][column] * (1.0 - cosine);
            let skew = match (row, column) {
                (0, 1) => -z,
                (0, 2) => y,
                (1, 0) => z,
                (1, 2) => -x,
                (2, 0) => -y,
                (2, 1) => x,
                _ => 0.0,
            };
            matrix[row][column] = if row == column { cosine } else { 0.0 } + sine * skew + outer;
        }
    }
    matrix
}

fn diagonal(scale: &[f64]) -> [[f64; 3]; 3] {
    let mut matrix = [[0.0; 3]; 3];
    for (axis, factor) in scale.iter().take(3).enumerate() {
        matrix[axis][axis] = *factor;
    }
    matrix
}

fn multiply(left: &[[f64; 3]; 3], right: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut product = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            product[row][column] = (0..3)
                .map(|term| left[row][term] * right[term][column])
                .sum();
        }
    }
    product
}

/// Returns one direction in the axes the document is written in.
///
/// This engine is y-up and Cycles is z-up, so a direction crosses the same basis change a point
/// does: `(x, y, z) -> (x, -z, y)`. Both sides of this comparison are put through it, so a
/// mismatch here can only be the emitter's and not the test's.
fn cycles_direction(direction: [f64; 3]) -> [f64; 3] {
    [direction[0], -direction[2], direction[1]]
}

fn normalised(vector: [f64; 3]) -> [f64; 3] {
    let length = (vector[0].powi(2) + vector[1].powi(2) + vector[2].powi(2)).sqrt();
    [vector[0] / length, vector[1] / length, vector[2] / length]
}

/// Returns the angle between two directions, which is the whole of the assertion.
///
/// An eye and a target are points and their difference is a direction, so both sides are compared
/// as unit vectors: how far apart the camera is from its subject is not an aiming question.
fn angle_between_degrees(one: [f64; 3], other: [f64; 3]) -> f64 {
    let (one, other) = (normalised(one), normalised(other));
    let cosine = one[0] * other[0] + one[1] * other[1] + one[2] * other[2];
    cosine.clamp(-1.0, 1.0).acos()
}
