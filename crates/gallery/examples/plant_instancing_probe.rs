//! The plant slice's acceptance: a scene's instance transform stands a leaf's blade where the
//! vegetation crate says it is.
//!
//! Three routes to the same points, and the probe compares them as **sets** — a permutation of the
//! triangle order must not be able to hide a disagreement:
//!
//! 1. `placement.place(point)` — [`vegetation::plant::Placement`]'s own answer, the frame a leaf field
//!    composed from the root down;
//! 2. the same template point through
//!    [`transform_matrix`](whiting_scene::standard::math::transform_matrix) of the instance transform
//!    [`standing`] returns — what a scene actually draws;
//! 3. `placement.place(point) * scale` — the crate's stated rule for a template point in the world
//!    (`vegetation::leaf`'s module docs).
//!
//! **(1) against (2) is the contract this file exists for.** The rotation is the one conversion in the
//! slice that cannot fail loudly: a frame extracted for `Rz · Ry · Rx`, or with one sign flipped, is still
//! an orthonormal frame and still draws every leaf somewhere plausible. Only the round trip sees it.
//!
//! **(1) against (3) is a measured disagreement, and it is reported as one.** A scene's instance
//! transform scales first and translates last, so it gives `scale · R · p + origin` where the crate's
//! rule gives `scale · R · p + scale · origin`. The two differ by `(scale − 1) · origin` at **every**
//! point of **every** leaf, so that row reports how many leaves differ and how far the furthest
//! difference is from that prediction: agreeing with the prediction to the tolerance is what makes it
//! a measurement of the composition rather than a complaint about it. A leaf's origin is a point on its
//! parent's skin, so the scene transform is the one that stands it in the world — see
//! [`whiting_gallery::vegetation_instancing`].
//!
//! Two rows are controls: a frame whose `x` is world up, which takes the gimbal branch of the extraction
//! and has to agree, and the same leaves read in a deliberately wrong Euler order, which **must**
//! disagree.
//!
//! The last section is the other half of the claim the picture makes. **One blade, `n` placements, one
//! pass**: the emitter groups a stage's instances by shader, mesh and images, so every leaf of every twig
//! is one group with one instance buffer, and the row of the emitted document says how many rows that
//! buffer holds against how many leaves the scene placed.
//!
//! Run with `cargo run -p whiting-gallery --example plant_instancing_probe`.

use definition::{Definition, DefinitionIdentifier, Registry, SchemaIdentifier, Value};
use scene::standard::math::{apply, transform_matrix};
use scene::{CompileContext, Compiler, STANDARD_SCENE_SCHEMA, StandardScene};
use vegetation::derive::bamboo;
use vegetation::expand::{Detail, expand};
use vegetation::leaf::LeafInstance;
use vegetation::plant::Placement;

use whiting_gallery::vegetation_instancing::{euler, matrix, standing};
use whiting_gallery::vegetation_mesh;

/// The seed the picture draws, so the probe measures the plant the picture shows.
const SEED: u64 = 7;

/// How far two points may stand apart and still be called the same point.
///
/// A hundredth of a millimetre at the scale of a plant: three orders of magnitude above the rounding an
/// `f32` position and an `f64` Euler carry between them, and four below the `(scale − 1) · origin`
/// difference the scaled row has to separate from the rows that are supposed to agree.
const TOLERANCE: f64 = 1.0e-4;

/// What a row is supposed to find.
#[derive(Clone, Copy, PartialEq)]
enum Expect {
    /// Every point of one route has a point of the other within [`TOLERANCE`].
    Agree,
    /// At least one point has none within the tolerance.
    Differ,
    /// At least one point has none, **and** the furthest difference is this close to a named size: a
    /// wrong extraction that happened to land near the prediction is a second wrong extraction, and
    /// "it differed" would have called both of them right.
    DifferBy(f64),
}

/// One measurement: two routes to the same points, compared as sets.
struct Row {
    /// What the row measures, in the terms a reader of the plant would use.
    name: &'static str,
    /// How many points one route carried.
    points: usize,
    /// How many of them have no match within [`TOLERANCE`].
    differing: usize,
    /// The largest distance from a point of one route to the nearest point of the other.
    furthest: f64,
    /// What the row has to find.
    expect: Expect,
}

impl Row {
    /// Measures two routes to the same blade, leaf by leaf, as point sets.
    ///
    /// # Invariants
    ///
    /// - **The nearest point is looked for inside the same leaf, never across the whole plant.** Both
    ///   routes carry every leaf in one list, and two leaves of one twig stand within a centimetre of
    ///   each other, so a search across the whole plant lets one leaf's point find the neighbour that
    ///   happens to be near where it wrongly landed — and a wrongly scaled leaf then measures as correct.
    ///   That is the failure the pooled form had, and it is the reason this is per leaf.
    /// - **Nearest rather than paired.** The two routes hold the same corners in the same order, so a
    ///   paired comparison would give the same answer on this plant and a different one the moment a
    ///   scene emitted its mesh in another order — which is exactly the failure this measurement is here
    ///   to catch.
    fn measure(
        name: &'static str,
        one: &[Vec<[f64; 3]>],
        other: &[Vec<[f64; 3]>],
        expect: Expect,
    ) -> Self {
        assert_eq!(
            one.len(),
            other.len(),
            "both routes carry the same leaves, or one of them is missing one"
        );
        let (mut points, mut differing, mut furthest) = (0usize, 0usize, 0.0f64);
        for (mine, theirs) in one.iter().zip(other) {
            let nearest = |point: [f64; 3]| {
                theirs
                    .iter()
                    .map(|candidate| distance(point, *candidate))
                    .fold(f64::INFINITY, f64::min)
            };
            for point in mine {
                let away = nearest(*point);
                furthest = furthest.max(away);
                if away > TOLERANCE {
                    differing += 1;
                }
            }
            points += mine.len();
        }
        Self {
            name,
            points,
            differing,
            furthest,
            expect,
        }
    }

    /// Whether this row found what it was written to find.
    fn holds(&self) -> bool {
        match self.expect {
            Expect::Agree => self.differing == 0,
            Expect::Differ => self.differing > 0,
            Expect::DifferBy(predicted) => {
                self.differing > 0 && (self.furthest - predicted).abs() <= TOLERANCE
            }
        }
    }

    /// What the row had to find, as the table prints it.
    fn expected(&self) -> String {
        match self.expect {
            Expect::Agree => "agree".to_owned(),
            Expect::Differ => "differ".to_owned(),
            Expect::DifferBy(predicted) => format!("differ by {predicted:.6}"),
        }
    }
}

/// The distance between two points.
fn distance(left: [f64; 3], right: [f64; 3]) -> f64 {
    (0..3)
        .map(|axis| (left[axis] - right[axis]).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// The width of a set of points on each axis: the box a caller has to place them in.
fn extent(points: &[[f64; 3]]) -> [f64; 3] {
    let mut low = [f64::INFINITY; 3];
    let mut high = [f64::NEG_INFINITY; 3];
    for point in points {
        for axis in 0..3 {
            low[axis] = low[axis].min(point[axis]);
            high[axis] = high[axis].max(point[axis]);
        }
    }
    (0..3)
        .map(|axis| high[axis] - low[axis])
        .collect::<Vec<_>>()
        .try_into()
        .unwrap_or([0.0; 3])
}

/// The blade's own points, read back out of the packed mesh the scene uploads.
///
/// The probe reads the **buffer** rather than the geometry it was packed from: the claim is about the
/// bytes a renderer draws, and reading the source would have proved a different one.
fn template_positions() -> Vec<[f64; 3]> {
    let points = vegetation_mesh::template_points(&bamboo(SEED).templates[0]);
    (0..points.len())
        .map(|index| {
            [
                points
                    .component(index, "position", 0)
                    .expect("a packed point holds its own position"),
                points
                    .component(index, "position", 1)
                    .expect("a packed point holds its own position"),
                points
                    .component(index, "position", 2)
                    .expect("a packed point holds its own position"),
            ]
        })
        .collect()
}

/// The blade's points placed by the frame itself, scaled by `factor`.
///
/// `factor` is the **whole** scale, not a multiplier of the leaf's own: the frame's contract is
/// `place(p) * scale`, so a row that wants the frame alone asks for a factor of one.
fn placed(leaf: &LeafInstance, template: &[[f64; 3]], factor: f64) -> Vec<[f64; 3]> {
    template
        .iter()
        .map(|point| {
            leaf.placement
                .place(point.map(|value| value as f32))
                .map(|value| f64::from(value) * factor)
        })
        .collect()
}

/// The blade's points through the scene's instance transform, scaled by `factor`.
fn drawn(leaf: &LeafInstance, template: &[[f64; 3]], factor: f64) -> Vec<[f64; 3]> {
    let (translation, rotation, _) = standing(leaf);
    let matrix = transform_matrix(translation, rotation, [factor; 3]);
    template
        .iter()
        .map(|point| {
            let mapped = apply(&matrix, *point);
            [mapped[0], mapped[1], mapped[2]]
        })
        .collect()
}

/// How far the transform's own columns stand from the frame's three axes.
///
/// The rotation alone: the scale is divided out, because a leaf's size is not its frame, and the
/// translation is not in a column at all.
fn axis_deviation(leaf: &LeafInstance) -> f64 {
    let matrix = matrix(leaf);
    let columns = [[0usize, 1, 2], [4, 5, 6], [8, 9, 10]];
    let frame = [leaf.placement.x, leaf.placement.y, leaf.placement.z];
    columns
        .iter()
        .zip(frame)
        .map(|(column, axis)| {
            let length = (0..3)
                .map(|row| matrix[column[row]] * matrix[column[row]])
                .sum::<f64>()
                .sqrt()
                .max(1.0e-12);
            distance(
                [
                    matrix[column[0]] / length,
                    matrix[column[1]] / length,
                    matrix[column[2]] / length,
                ],
                axis.map(f64::from),
            )
        })
        .fold(0.0, f64::max)
}

/// The distance `(scale − 1) · origin`: the whole of the difference between the crate's stated rule and
/// a scene transform, which scales before it translates.
fn offset_of(leaf: &LeafInstance) -> f64 {
    let scale = f64::from(leaf.scale) - 1.0;
    distance(
        leaf.placement.origin.map(|value| scale * f64::from(value)),
        [0.0; 3],
    )
}

fn main() {
    let species = bamboo(SEED);
    let expansion = expand(&species.plant, Detail::DEFAULT, &species.fields);
    let template = template_positions();
    let leaves: Vec<&LeafInstance> = expansion
        .leaves
        .iter()
        .flat_map(|group| group.leaves.iter())
        .collect();

    println!("# one bamboo, one blade, {} leaves", leaves.len());
    println!(
        "  {} nodes, {} wood solids, {} leaf groups, {} blade corners",
        species.plant.walk().count(),
        expansion.wood.len(),
        expansion.leaves.len(),
        template.len()
    );
    // The blade's own size, and the size it is drawn at, because a leaf field's numbers decide both and a
    // picture that cannot say how big its leaves are cannot be read for whether they are there.
    let blade = extent(&template);
    let widest = leaves
        .iter()
        .max_by(|left, right| left.scale.total_cmp(&right.scale))
        .expect("the field put at least one leaf on the plant");
    let leaf_box = extent(&drawn(widest, &template, f64::from(widest.scale)));
    println!(
        "  the blade is {:.3} m long in its own frame and is drawn at a scale of {:.3} to {:.3}, so this \
         plant's largest leaf is drawn in a box of {:.3} by {:.3} by {:.3} m",
        blade[0],
        leaves
            .iter()
            .map(|leaf| f64::from(leaf.scale))
            .fold(f64::INFINITY, f64::min),
        leaves
            .iter()
            .map(|leaf| f64::from(leaf.scale))
            .fold(f64::NEG_INFINITY, f64::max),
        leaf_box[0],
        leaf_box[1],
        leaf_box[2]
    );

    let mut rows: Vec<Row> = Vec::new();

    // The contract: the frame alone, which is the half the extraction owns. The scale is one on both
    // routes, so this row is about the position and the turns and about nothing else.
    rows.push(Row::measure(
        "every leaf: place(p) against transform(origin, euler, [1, 1, 1]) p",
        &leaves
            .iter()
            .map(|leaf| placed(leaf, &template, 1.0))
            .collect::<Vec<_>>(),
        &leaves
            .iter()
            .map(|leaf| drawn(leaf, &template, 1.0))
            .collect::<Vec<_>>(),
        Expect::Agree,
    ));

    // The crate's own rule, at the leaf's own scale, against a scene transform at the same scale.
    rows.push(Row::measure(
        "every leaf: place(p) * s against transform(origin, euler, [s, s, s]) p",
        &leaves
            .iter()
            .map(|leaf| placed(leaf, &template, f64::from(leaf.scale)))
            .collect::<Vec<_>>(),
        &leaves
            .iter()
            .map(|leaf| drawn(leaf, &template, f64::from(leaf.scale)))
            .collect::<Vec<_>>(),
        Expect::DifferBy(
            leaves
                .iter()
                .map(|leaf| offset_of(leaf))
                .fold(0.0, f64::max),
        ),
    ));

    // The gimbal case: a frame whose `x` is world up, which has no turn about `y` of its own.
    let gimbal = LeafInstance {
        index: 0,
        station: 0.5,
        slot: 0.5,
        placement: Placement {
            origin: [0.1, 1.6, -0.2],
            x: [0.0, 1.0, 0.0],
            y: [0.0, 0.0, 1.0],
            z: [1.0, 0.0, 0.0],
        },
        scale: 0.5,
    };
    rows.push(Row::measure(
        "gimbal: a placement whose x is world up",
        &[placed(&gimbal, &template, 1.0)],
        &[drawn(&gimbal, &template, 1.0)],
        Expect::Agree,
    ));

    // The control: the same leaves read with their first two turns transposed, which must disagree.
    rows.push(Row::measure(
        "control: the turns transposed, the leaf's rx and ry swapped",
        &leaves
            .iter()
            .map(|leaf| drawn(leaf, &template, 1.0))
            .collect::<Vec<_>>(),
        &leaves
            .iter()
            .map(|leaf| {
                let (translation, [rx, ry, rz], _) = standing(leaf);
                let wrong = transform_matrix(translation, [ry, rx, rz], [1.0; 3]);
                template
                    .iter()
                    .map(|point| {
                        let mapped = apply(&wrong, *point);
                        [mapped[0], mapped[1], mapped[2]]
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
        Expect::Differ,
    ));

    // The three axes, which is the contract stated directly: local `+x`, `+y`, `+z` onto the instance's.
    let deviations: Vec<f64> = leaves.iter().map(|leaf| axis_deviation(leaf)).collect();
    rows.push(Row {
        name: "every leaf: the transform's columns against the frame's own axes",
        points: leaves.len() * 3,
        differing: deviations
            .iter()
            .map(|deviation| usize::from(*deviation > TOLERANCE))
            .sum(),
        furthest: deviations.iter().copied().fold(0.0, f64::max),
        expect: Expect::Agree,
    });

    println!("\n# the conversion, leaf by leaf, as point sets");
    println!(
        "  {:<74} {:>9} {:>17} {:>12}  expected",
        "row", "tolerance", "differing / points", "furthest"
    );
    let mut held = true;
    for row in &rows {
        let verdict = if row.holds() { "ok" } else { "FAILED" };
        held &= row.holds();
        println!(
            "  {:<74} {:>9.0e} {:>8} /{:<7} {:>12.3e}  {:<18} {verdict}",
            row.name,
            TOLERANCE,
            row.differing,
            row.points,
            row.furthest,
            row.expected(),
        );
    }

    println!("\n# the turns, on the first leaf of two twigs");
    for leaf in [leaves[0], leaves[leaves.len() / 2]] {
        let (translation, rotation, scale) = standing(leaf);
        println!(
            "  leaf {} at ({:.4}, {:.4}, {:.4}), turned ({:.4}, {:.4}, {:.4}) rad, scaled {:.4}",
            leaf.index,
            translation[0],
            translation[1],
            translation[2],
            rotation[0],
            rotation[1],
            rotation[2],
            scale[0],
        );
    }
    // The gimbal branch is the one number a frame whose `x` is world up cannot take from the general
    // form, so it is printed rather than left in the row above: `rz` is zero there and the whole turn
    // is `rx`, and a reader who sees a zero in that slot knows which of the two branches ran.
    let turns = euler(&gimbal.placement);
    println!(
        "  the gimbal frame turns ({:.4}, {:.4}, {:.4}) rad, and its turn about z is zero: {}",
        turns[0],
        turns[1],
        turns[2],
        if turns[2] == 0.0 { "yes" } else { "NO" }
    );
    if turns[2] != 0.0 {
        eprintln!("error: a frame whose x is world up did not take the gimbal branch");
        std::process::exit(1);
    }

    if !held {
        eprintln!("error: a row did not find what it was written to find");
        std::process::exit(1);
    }

    if !passes(leaves.len()) {
        eprintln!("error: the leaves are not one pass over one instance buffer");
        std::process::exit(1);
    }

    println!("\n# the sidecar table a frame of this scene is read beside");
    print!("{}", whiting_gallery::vegetation_picture::sidecar());
}

/// Prints every pass the bamboo scene compiled to, with the instance buffer each one binds.
///
/// # Invariants
///
/// - **The rows are counted from the emitted buffer and divided by the size of a buffer that holds one
///   row.** The document writes no row count — a buffer is a byte range and a schema — so a probe that
///   re-derived the stride from the schema would be measuring its own arithmetic; taking the stride from a
///   single-placement buffer that was compiled by the same emitter measures the document's.
/// - **A leaf buffer's rows have to equal the number of leaves the scene placed.** That is the claim: one
///   blade, `n` placements, one buffer. A scene that gave every leaf its own mesh would show `n` buffers
///   of one row, which is the shape this is here to catch.
fn passes(leaves: usize) -> bool {
    let scene = whiting_gallery::vegetation_picture::bamboo_picture();
    let mut registry = Registry::new();
    registry.put_schema(
        SchemaIdentifier::new(graphics::GRAPHICS_SCHEMA),
        graphics::graphics_schema(),
    );
    registry.put_schema(
        SchemaIdentifier::new(graphics::INSTANCE_SCHEMA),
        graphics::instance_schema(),
    );
    scene::register_standard_scene_schema(&mut registry);
    let source = Definition::new(
        DefinitionIdentifier::new("gallery.source"),
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        scene,
    );
    source
        .validate(&registry)
        .expect("the bamboo scene satisfies the scene schema");
    let compiled = StandardScene::new(
        DefinitionIdentifier::new("gallery.graphics"),
        DefinitionIdentifier::new("gallery.instance"),
    )
    .compile(&CompileContext {
        scene: &source,
        previous_graphics: None,
        previous_instance: None,
        registry: &registry,
        camera: 0,
        limits: None,
    })
    .expect("the bamboo scene compiles");

    let entries = compiled
        .instance
        .value
        .as_field_map()
        .and_then(|document| document.get("entries"))
        .and_then(Value::as_field_map)
        .expect("the instance document holds its entries under `entries`");
    let bytes = |name: &str| -> i64 {
        entries
            .get(name)
            .and_then(|entry| entry.variant_fields())
            .and_then(|fields| fields.get(graphics::binders::SIZE_BYTES))
            .and_then(Value::as_int)
            .unwrap_or_else(|| panic!("the document carries an instance entry named {name}"))
    };
    let passes = graphics::passes(&compiled.graphics.value).expect("the emitted passes read");

    println!("\n# every pass the scene compiled to, and the instance buffer it binds");
    println!("  {:<44} {:>12}  instance buffer", "pass", "bytes");
    let mut single = 0i64;
    // Every pass over the blade, with the stage it is in and the size of the buffer it binds.
    let mut blades: Vec<(&str, i64)> = Vec::new();
    for pass in &passes {
        let label = pass.label().expect("a pass names itself");
        let bound = pass
            .binder_names()
            .expect("a pass lists its binders")
            .into_iter()
            .find(|name| name.ends_with("_instance_data"))
            .unwrap_or("none");
        let size = if bound == "none" { 0 } else { bytes(bound) };
        if size > 0 && single == 0 {
            single = size;
        }
        println!("  {label:<44} {size:>12}  {bound}");
        if let Some(stage) = label.split('/').next().filter(|_| label.contains("/leaf")) {
            blades.push((stage, size));
        }
    }
    let passes_over = |stage: &str| blades.iter().filter(|(held, _)| *held == stage).count();
    let rows_in = |stage: &str| {
        blades
            .iter()
            .find(|(held, _)| *held == stage)
            .map_or(0, |(_, size)| size / single.max(1))
    };
    let shadows = passes_over("shadow");
    let depths = passes_over("depth");
    let lighting = passes_over("lighting");
    println!(
        "  one placement is {single} bytes of instance row, and every blade buffer is {single} bytes a \
         row: {} leaves in {} row(s)",
        rows_in("lighting"),
        rows_in("lighting")
    );
    // **The claim is one pass per stage and not one pass per leaf.** A stage draws a group at a time and
    // the group's rows are the leaves, so a scene that gave every leaf its own mesh would show one buffer
    // per leaf and a shadowing pass per leaf per light.
    let held = lighting == 1
        && depths == 1
        && shadows == 2
        && rows_in("lighting") == leaves as i64
        && rows_in("depth") == leaves as i64
        && rows_in("shadow") == leaves as i64;
    println!(
        "  {leaves} leaves are drawn by {lighting} lighting pass, {depths} depth pass and {shadows} shadow \
         pass (one per light) over one blade mesh, each binding one buffer of {leaves} rows: {}",
        if held {
            "one pass and one instance buffer"
        } else {
            "NOT one pass and one instance buffer"
        }
    );
    held
}
