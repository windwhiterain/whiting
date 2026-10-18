//! The law demos: four pictures that show what this crate's laws do, each beside the plant without them.
//!
//! # Why every scene here is a pair
//!
//! A law is a claim about a *difference*, so every scene in this module draws **one plant twice**: once as its rule
//! made it, and once under the law. One plant under a force is a plant; two plants, one of them not under it, is
//! evidence — and the difference is what the frame is for. The reports that cannot be a picture are measured where
//! they belong (`field_probe`, `obstacle_probe`, `leaf_probe`, `select_probe`), and the frames here are the half of
//! the claim that a table cannot make.
//!
//! | scene | shows |
//! | --- | --- |
//! | [`GARDEN_WALL`] | `Field::Obstacle` and `contact::collide`: a plant that grows beside a wall, and the same plant that does not know the wall is there |
//! | [`ASPIRING`] | `Field::Ancestor` with `LeafField::reference` and `LeafField::aim`: limbs that straighten along their parents, and leaves read in the trunk's frame and turned to the sky |
//! | [`THINNED`] | `thin`: a crowded clump, and the same clump at a spacing |
//! | [`BRUSHED`] | `Selector` and `paint`: one plant, and the same plant with two brush strokes on it |
//!
//! Run one with `cargo run -p whiting-device-rasterize --example render -- garden_wall out/wall`.

use definition::Value;

use vegetation::derive::{Species, bamboo, clump};
use vegetation::envelope::{Envelope, Outline, trim};
use vegetation::field::Field;
use vegetation::handle::Handle;
use vegetation::ops::{Change, Edit, Frame, Property, apply, paint};
use vegetation::selector::Selector;
use vegetation::thin::thin;

use crate::geometry::cuboid;
use crate::vegetation_picture::{
    Framing, Ground, Planting, plant_scene, planted_at, planting_handle,
};

/// The framing every pair is drawn in: two plants, near enough that the law's difference is the subject.
const PAIR: Framing = Framing {
    width: 1400,
    height: 1000,
    eye: [1.95, 1.30, 3.05],
    target: [0.0, 0.80, 0.00],
    field_of_view: 0.80,
    near: 0.05,
    far: 32.0,
    floor: 7.0,
    ground: Ground::Plane,
};

/// Where the two plants of a pair stand, and which way each faces: `x` negative is the one without the law.
const PAIR_PLACES: [([f32; 3], f32); 2] = [([-0.62, 0.0, 0.0], 0.0), ([0.62, 0.0, 0.0], 0.0)];

/// The wall scene's own framing: a grazing view along the wall, so that a bough lying on its face reads as a bough on
/// a face and not as a bough beside one.
const WALL_FRAME: Framing = Framing {
    width: 1400,
    height: 1000,
    eye: [2.45, 1.40, 1.75],
    target: [-0.05, 0.82, -0.55],
    field_of_view: 0.74,
    near: 0.05,
    far: 32.0,
    floor: 7.0,
    ground: Ground::Plane,
};
/// The name of the scene that grows a plant beside a wall.
pub const GARDEN_WALL: &str = "garden_wall";

/// Where the wall stands and how big it is.
///
/// It is **one metre thick and not a panel**: a bough that penetrates a thin slab is as close to its back face as to
/// its front one, so the occupancy's gradient pushes half of it each way and the picture is a plant cut in two. A thick
/// wall has one way out, and it is the way the camera is.
const WALL_AT: [f64; 3] = [0.0, 0.0, -0.62];
const WALL_SIZE: (f64, f64, f64) = (3.60, 1.55, 1.00);

/// Where the wall scene's two plants stand, turned so that their boughs reach **at** the wall: the pair's own places
/// face the camera, and a plant that cannot touch the wall cannot show what happens when it does.
const WALL_PLACES: [([f32; 3], f32); 2] = [
    ([-0.62, 0.0, 0.0], std::f32::consts::FRAC_PI_2),
    ([0.62, 0.0, 0.0], std::f32::consts::FRAC_PI_2),
];

/// The wall's own occupancy: a signed distance to the slab it occupies, negative inside.
///
/// # Invariants
///
/// It is the **box's signed distance** and not a plane's: outside the slab it is the length of the positive part of
/// the offset from the box, inside it is the largest negative one, so a bough above the wall is pushed up rather than
/// back, and the arithmetic is the same at every point of the world.
fn wall_occupancy(point: [f32; 3]) -> f32 {
    let centre = [
        0.0,
        (WALL_SIZE.1 * 0.5) as f32,
        (WALL_AT[2] + WALL_SIZE.2 * 0.5) as f32,
    ];
    let half = [
        (WALL_SIZE.0 * 0.5) as f32,
        (WALL_SIZE.1 * 0.5) as f32,
        (WALL_SIZE.2 * 0.5) as f32,
    ];
    let offset = [
        (point[0] - centre[0]).abs() - half[0],
        (point[1] - centre[1]).abs() - half[1],
        (point[2] - centre[2]).abs() - half[2],
    ];
    let outside = [offset[0].max(0.0), offset[1].max(0.0), offset[2].max(0.0)];
    let reach =
        (outside[0] * outside[0] + outside[1] * outside[1] + outside[2] * outside[2]).sqrt();
    reach + offset[0].max(offset[1]).max(offset[2]).min(0.0)
}

/// Builds the wall scene: the same plant twice, and only the right-hand one knows the wall is there.
pub fn garden_wall() -> Value {
    let mut aware = bamboo(11);
    aware = aware.with_fields(vec![Field::obstacle(wall_occupancy, 1.0)]);
    let plantings = [
        Planting::new(bamboo(11)).at(planted_at(WALL_PLACES[0].0, WALL_PLACES[0].1)),
        Planting::new(aware).at(planted_at(WALL_PLACES[1].0, WALL_PLACES[1].1)),
    ];
    plant_scene(
        &plantings,
        &WALL_FRAME,
        GARDEN_WALL,
        &[crate::vegetation_picture::Prop {
            name: "wall".to_owned(),
            points: cuboid(WALL_SIZE.0, WALL_SIZE.1, WALL_SIZE.2),
            tint: [0.62, 0.58, 0.51],
            roughness: 0.92,
            at: WALL_AT,
        }],
    )
}

/// Resolves a wall scene's id: the left planting has no field and the right one is the plant that does.
pub fn garden_wall_handle_of(id: u32) -> Option<String> {
    planting_handle(&garden_wall_plantings(), id)
}

fn garden_wall_plantings() -> Vec<Planting> {
    let aware = bamboo(11).with_fields(vec![Field::obstacle(wall_occupancy, 1.0)]);
    vec![
        Planting::new(bamboo(11)).at(planted_at(WALL_PLACES[0].0, WALL_PLACES[0].1)),
        Planting::new(aware).at(planted_at(WALL_PLACES[1].0, WALL_PLACES[1].1)),
    ]
}

/// The name of the scene whose right-hand plant looks up its own ancestry.
pub const ASPIRING: &str = "aspiring";

/// The leaves a rule made, and the leaves the same rule makes when their frame is the trunk's and their midrib is
/// turned to the sky.
fn aspiring_species() -> Species {
    let mut species = bamboo(11).with_fields(vec![Field::ancestor(0.7)]);
    species.fields[0].reference = 2;
    // **A weight of one stands every midrib on the axis**, which is what a grass blade wants and a broadleaf does not:\n    // the frame shows the law at a little over a third, where a leaf leans toward the sky and still reads as a leaf.\n    species.fields[0].aim = shape::Scalar::constant(0.45);
    species
}

/// Builds the aspiring scene: `Field::Ancestor` on the right, with the two leaf laws that need a frame above the twig.
pub fn aspiring() -> Value {
    let plantings = aspiring_plantings();
    plant_scene(&plantings, &PAIR, ASPIRING, &[])
}

/// Resolves an aspiring scene's id.
pub fn aspiring_handle_of(id: u32) -> Option<String> {
    planting_handle(&aspiring_plantings(), id)
}

fn aspiring_plantings() -> Vec<Planting> {
    vec![
        Planting::new(bamboo(11)).at(planted_at(PAIR_PLACES[0].0, PAIR_PLACES[0].1)),
        Planting::new(aspiring_species()).at(planted_at(PAIR_PLACES[1].0, PAIR_PLACES[1].1)),
    ]
}

/// The name of the scene whose right-hand clump has been thinned.
pub const THINNED: &str = "thinned";

/// The spacings the three clumps are thinned at, in metres: none, a quarter of the shrub, and most of it.
const THIN_SPACINGS: [f32; 3] = [0.0, 0.22, 0.40];

/// The three clumps of the thinning scene, and how far apart they stand.
const THIN_PLACES: [f32; 3] = [-1.05, 0.0, 1.05];

/// The framing a scene of shrubs is drawn in: two-thirds the distance of the pair, because a shrub is a third the
/// height of a culm and a law it obeys is invisible at a culm's distance.
const SHRUB_FRAME: Framing = Framing {
    width: 1400,
    height: 1000,
    eye: [0.0, 1.05, 3.05],
    target: [0.0, 0.52, 0.0],
    field_of_view: 0.78,
    near: 0.05,
    far: 32.0,
    floor: 7.0,
    ground: Ground::Plane,
};

/// Builds the thinned scene: **the same shrub three times, at three spacings**, so the law's one number is the frame.
///
/// # Invariants
///
/// The left clump is thinned at nought, which is the identity: the dial's own zero has to be the plant the rule made,
/// or the picture cannot say what the other two took away.
pub fn thinned() -> Value {
    let plantings = thinned_plantings();
    plant_scene(&plantings, &SHRUB_FRAME, THINNED, &[])
}

/// Resolves a thinned scene's id.
pub fn thinned_handle_of(id: u32) -> Option<String> {
    planting_handle(&thinned_plantings(), id)
}

fn thinned_plantings() -> Vec<Planting> {
    THIN_PLACES
        .iter()
        .zip(THIN_SPACINGS.iter())
        .map(|(place, spacing)| {
            let mut species = clump(31);
            thin(&mut species.plant, *spacing);
            Planting::new(species).at(planted_at([*place, 0.0, 0.0], 0.0))
        })
        .collect()
}
/// The name of the scene whose right-hand plant was edited by a brush rather than by a handle.
pub const BRUSHED: &str = "brushed";

/// Builds the brushed scene: two strokes, written by predicates rather than by handles.
///
/// # Invariants
///
/// The strokes are the two kinds a brush has — **a band** (`StationBand`, every bough that hangs high on the culm
/// thickens) and **a generation** (`Generation`, every branch lifts) — and both go through [`paint`], which is the
/// only bridge from a selector to an edit list. The picture is the difference, so the left plant is the same rule
/// with no stroke on it.
pub fn brushed() -> Value {
    let plantings = brushed_plantings();
    plant_scene(&plantings, &PAIR, BRUSHED, &[])
}

/// Resolves a brushed scene's id.
pub fn brushed_handle_of(id: u32) -> Option<String> {
    planting_handle(&brushed_plantings(), id)
}

fn brushed_plantings() -> Vec<Planting> {
    let mut brushed = bamboo(11);
    let mut edits = paint(
        &brushed.plant,
        &Selector::StationBand {
            from: 0.5,
            to: 0.86,
        },
        Change::Offset {
            property: Property::Radius,
            by: 0.009,
        },
    );
    edits.extend(paint(
        &brushed.plant,
        &Selector::Generation(2),
        Change::Shift {
            frame: Frame::Local,
            by: [0.0, 0.14, 0.0],
        },
    ));
    apply(&mut brushed.plant, &edits);
    vec![
        Planting::new(bamboo(11)).at(planted_at(PAIR_PLACES[0].0, PAIR_PLACES[0].1)),
        Planting::new(brushed).at(planted_at(PAIR_PLACES[1].0, PAIR_PLACES[1].1)),
    ]
}

/// The name of the scene whose trunk is a strongly curved spine.
pub const ARCHING: &str = "arching";

/// Builds the arching scene: **the trunk itself is a curve**, and every bough, branch and leaf derives from it.
///
/// # Invariants
///
/// The trunk is re-authored with [`Change::Spine`], which is the edit table's bake — the same tier a hand writes — so
/// nothing here is a new species and nothing is special-cased: the boughs keep their stations, the leaves keep their
/// `t`, and the whole plant follows because **a shape is its control points and every consumer derives from one
/// parameterisation**. The curve is an arch and not a lean because a lean is what the rule already draws (the culm's
/// own spine moves 0.19 m over 2.2 m); this one moves 1.35 m and comes back down, which no rule of the crate makes.
pub fn arching() -> Value {
    let mut species = bamboo(11);
    let arch = shape::Curve::new(vec![
        shape::Jet::new([0.0, 0.0, 0.0], [0.34, 0.52, 0.0]),
        shape::Jet::new([0.22, 0.72, 0.03], [0.40, 0.30, 0.06]),
        shape::Jet::new([0.76, 1.28, 0.12], [0.34, -0.10, 0.06]),
        shape::Jet::new([1.34, 1.20, 0.22], [0.22, -0.26, 0.04]),
    ])
    .expect("the arch's knots are distinct");
    apply(
        &mut species.plant,
        &[Edit::new(
            Handle::root(),
            Change::Spine { jets: arch.jets() },
        )],
    );
    plant_scene(
        &[Planting::new(species).at(planted_at([-0.62, 0.0, 0.0], 0.0))],
        &PAIR,
        ARCHING,
        &[],
    )
}

/// Resolves an arching scene's id.
pub fn arching_handle_of(id: u32) -> Option<String> {
    planting_handle(&arching_plantings(), id)
}

fn arching_plantings() -> Vec<Planting> {
    let mut species = bamboo(11);
    let arch = shape::Curve::new(vec![
        shape::Jet::new([0.0, 0.0, 0.0], [0.34, 0.52, 0.0]),
        shape::Jet::new([0.22, 0.72, 0.03], [0.40, 0.30, 0.06]),
        shape::Jet::new([0.76, 1.28, 0.12], [0.34, -0.10, 0.06]),
        shape::Jet::new([1.34, 1.20, 0.22], [0.22, -0.26, 0.04]),
    ])
    .expect("the arch's knots are distinct");
    apply(
        &mut species.plant,
        &[Edit::new(
            Handle::root(),
            Change::Spine { jets: arch.jets() },
        )],
    );
    vec![Planting::new(species).at(planted_at([-0.62, 0.0, 0.0], 0.0))]
}

/// The name of the scene whose crown is trimmed to a section rather than to a circle.
pub const PANELLED: &str = "panelled";

/// Where the panelled scene's three crowns stand: the pair's two places and one more to the right.
const PANEL_PLACES: [[f32; 3]; 3] = [[-0.95, 0.0, 0.0], [0.0, 0.0, 0.0], [0.95, 0.0, 0.0]];

/// The framing the panelled triptych is drawn in: three crowns need a wider frame than two.
const PANEL_FRAME: Framing = Framing {
    width: 1400,
    height: 1000,
    eye: [0.0, 1.55, 3.95],
    target: [0.0, 0.95, 0.0],
    field_of_view: 0.80,
    near: 0.05,
    far: 32.0,
    floor: 7.0,
    ground: Ground::Plane,
};
/// The radius law both crowns of the panelled scene are trimmed to.
const CROWN_RADIUS: [(f32, f32); 3] = [(0.0, 0.52), (0.5, 0.40), (1.0, 0.28)];

/// Builds the panelled scene: the same radius law, drawn as a circle on the left and as a section on the right.
///
/// # Invariants
///
/// The two envelopes differ in **the section and in nothing else**, so the difference in the two plants is the section:
/// a crown the space allows to be wide one way and narrow the other — between two walls, under a soffit — and the trim
/// law reads it through one `reach` query per station.
pub fn panelled() -> Value {
    let plantings = panelled_plantings();
    plant_scene(&plantings, &PANEL_FRAME, PANELLED, &[])
}

/// Resolves a panelled scene's id.
pub fn panelled_handle_of(id: u32) -> Option<String> {
    planting_handle(&panelled_plantings(), id)
}

fn panelled_plantings() -> Vec<Planting> {
    // **Three crowns, one radius law**: the coarse circle, one section at every station, and a family of outlines —
    // round where the trunk is free and flattened where a soffit would be. The third is the surface the design asked
    // for, and the other two are the forms it refines.
    let mut round = bamboo(11);
    trim(&mut round.plant, &Envelope::radial(&CROWN_RADIUS));
    let mut sectioned = bamboo(11);
    trim(
        &mut sectioned.plant,
        &Envelope::radial(&CROWN_RADIUS).with_section(shape::Section::ellipse(0.30, 1.0)),
    );
    let mut lawed = bamboo(11);
    let mut envelope = Envelope::radial(&CROWN_RADIUS);
    envelope.shape = Outline::of(&[
        // One axis narrows and the other hardly moves, so whichever of the trunk frame's two axes the camera sees
        // across, the crown is visibly pinched toward the top — a family whose shape changes along the trunk has to
        // change where a frame can see it, or the picture is three identical crowns.
        (0.0, shape::Section::ellipse(0.95, 0.95)),
        (0.55, shape::Section::ellipse(0.60, 0.90)),
        (1.0, shape::Section::ellipse(0.24, 0.86)),
    ]);
    trim(&mut lawed.plant, &envelope);
    vec![
        Planting::new(round).at(planted_at(PANEL_PLACES[0], 0.0)),
        Planting::new(sectioned).at(planted_at(PANEL_PLACES[1], 0.0)),
        Planting::new(lawed).at(planted_at(PANEL_PLACES[2], 0.0)),
    ]
}

/// The name of the scene whose leaves are placed by two different fields.
pub const BANDS: &str = "bands";

/// Builds the bands scene: **the leaf field itself is the subject**, so the two plants differ only in its dials.
///
/// # Invariants
///
/// The right-hand field changes three dials and no geometry: its band is the top half of the bough instead of most of
/// it, its density **rises toward the tip** instead of being even, and its phyllotaxy is a quarter turn instead of the
/// golden angle — so the leaves gather at the ends and stand in four ranks. Nothing about the twigs moves, which is
/// what makes the picture a statement about the field.
pub fn bands() -> Value {
    let plantings = bands_plantings();
    plant_scene(&plantings, &PAIR, BANDS, &[])
}

/// Resolves a bands scene's id.
pub fn bands_handle_of(id: u32) -> Option<String> {
    planting_handle(&bands_plantings(), id)
}

fn bands_plantings() -> Vec<Planting> {
    let rule = bamboo(11);
    let mut riser = bamboo(11);
    let mut field = riser.fields[0].clone();
    field.band = (0.55, 0.98);
    field.density =
        shape::Scalar::new(&[(0.0, 4.0), (1.0, 70.0)]).expect("a rising density is a law");
    field.phyllotaxy = 0.25;
    field.size = shape::Scalar::new(&[(0.0, 0.20), (1.0, 0.55)]).expect("a growing leaf is a law");
    riser.fields[0] = field;
    vec![
        Planting::new(rule).at(planted_at(PAIR_PLACES[0].0, PAIR_PLACES[0].1)),
        Planting::new(riser).at(planted_at(PAIR_PLACES[1].0, PAIR_PLACES[1].1)),
    ]
}
