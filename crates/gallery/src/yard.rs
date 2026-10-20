//! The yard scene: a site on a slope — two buildings, a walled courtyard with a gate, and the ground
//! the whole thing was seated on.
//!
//! # What this frame exists to show
//!
//! The shed scene proved a facade. This one proves the **site**: that a building on a slope sits on a
//! terrace cut to the highest ground under it, that a wall **follows** the ground it crosses instead of
//! standing on a plane, that a courtyard is several closed boxes meeting at their corners, and that a gate
//! is a gap rather than a hole — all of it authored in one document and seated by one function.
//!
//! # One law for the ground
//!
//! `ground` is called twice in this file and it matters that it is the same function: once by the
//! terrain mesh this scene draws, and once by [`seat`] writing the ground into the document. Two
//! different laws would draw a slope with buildings hovering over it or buried in it, and the picture
//! would still look plausible — the one failure a render cannot show and a shared function cannot have.
//!
//! Run it with `cargo run -p whiting-device-rasterize --example render -- yard out/yard`.

use definition::Value;
use scene::standard::Stage;

use building::derive::{self, RoofStyle, Starter};
use building::grid::{FaceRef, Rhythm};
use building::openings::{Inset, Opening};
use building::ops;
use building::parts::{Pin, Placement, Target, Template};
use building::site::seat;
use building::wall::{Wall, WallId};
use building::{Building, BuildingSkeleton, Element, Mass, MassId, PlanNode};

use crate::instancing::standing;
use building::identity::Identity;
use document::air::Atmosphere;
use document::building_identity::{
    building_extras, not_authored_extras, outputs_field, packed_where, stage_set,
};
use document::kit::{
    camera, flux_for_irradiance, frame, instance, instances, light_with_radius, mesh,
    mesh_material, offset_by, post_processes, screen_material, shaded_point_schema, sky, surface,
};
use document::shaders::{Projection, post_shader};

/// The ground the site stands on, in metres: it falls one unit in eleven as `x` rises.
///
/// # Invariants
///
/// **The terrain and the seat both call this**, so what the frame draws is what the document was seated
/// on. The camera looks up the slope, which is why the fall reads as depth rather than as a tilt.
fn ground(x: f64, _z: f64) -> f64 {
    0.16 * x
}

/// The two buildings, as starters: a shed and a smaller store up the slope.
const SHED: Starter = Starter {
    width: 5.0,
    depth: 8.0,
    height: 2.8,
    wall: 0.25,
    // A deep foundation, because this site is a slope: the terrace is cut to the highest ground under
    // each building, so its downhill edge stands the relief above the ground and a slab carries it down.
    slab: 1.20,
    overhang: 0.40,
    sink: 0.10,
    rise: 1.30,
    roof_thickness: 0.16,
    roof: RoofStyle::Gable,
    seed: 5,
};

const STORE: Starter = Starter {
    width: 4.0,
    depth: 5.0,
    height: 2.4,
    wall: 0.25,
    slab: 1.20,
    overhang: 0.35,
    sink: 0.10,
    rise: 1.10,
    roof_thickness: 0.16,
    roof: RoofStyle::Gable,
    seed: 9,
};

/// How far up the slope the store stands from the shed.
const STORE_OFFSET: f32 = 14.0;

/// Where the sun stands: high and to the front-left, so the courtyard's inside faces are lit and the
/// ground's slope shows as shading rather than as a silhouette.
const SUN: [f64; 3] = [-16.0, 30.0, 24.0];

/// How much irradiance the ground receives from that sun, in the scene's own units.
const SUN_IRRADIANCE: f64 = 3.2;

/// The image this scene is drawn into, in pixels.
///
/// # Why this frame is this shape
///
/// A yard is the broadest thing in this gallery: a thirteen-metre courtyard with a shed inside it and
/// a store fourteen metres further up the slope. The measured number says how broad. Reading every
/// id the frame carries at `FRAC_PI_3 * 1.15`, the silhouette stands `x 130..=826` by `y 343..=795`
/// of the pipeline's square — **f_w 0.6804 by f_h 0.4418**, a world aspect of **1.540 to one**.
///
/// So the frame takes those proportions, **1.55 to one** at 1264 by 816: 1,031,424 pixels against
/// the square's 1,048,576.
///
/// **The square it replaces was cutting it.** Its left column carried **198 pixels of building**,
/// and `wall 4, its left side, looking out` ran from x 0. A yard is seen from outside its own wall,
/// and the wall nearest the eye is the one that leaves the frame.
const FRAME_WIDTH: i64 = 1264;
const FRAME_HEIGHT: i64 = 816;

/// The full **vertical** angle this yard is drawn through, in radians.
///
/// # The arithmetic
///
/// ```text
/// tan(new / 2) = tan(1.15 * PI/3 / 2) * 0.4418 / 0.89
///              = 0.673576 * 0.496404 = 0.334405
/// new = 2 * atan(0.334405) = 0.65757 rad = FRAC_PI_3 * 0.6279
/// ```
///
/// **89 % and not 90 %**, because a site's walls and its gate leaf stand outside the buildings they
/// enclose.
///
/// **The eye did not move and the aim did, because the silhouette fits and is not in the middle.**
/// At this frame and this lens the site measures `x 18..=1137` by `y 137..=815`: **0.8860 of the width
/// and 0.8319 of the height**, with its centre **54 px left of and 68.5 px below** the frame's — which
/// is why the near yard wall sat on the last row while 137 rows of sky stood above it.
/// `54 / 631.5 * 22.1876 * 0.340840` = **0.6466 m** along `[0.71867, 0, 0.69542]` the other way and
/// `68.5 / 407.5 * 22.1876 * 0.340840` = **1.2711 m** of height put it in the middle with 69 rows of
/// margin on every side, **and the lens did not move at all**. Shortening the lens until the wall
/// fitted would have cost a twelfth of the fill to answer a question the measurement had answered.
const FIELD_OF_VIEW: f64 = std::f64::consts::FRAC_PI_3 * 0.6279;

/// The aim this yard looks from, and why it is not the building's own middle.
///
/// # Invariants
///
/// **A yard is seen from outside its own wall, and the wall nearest the eye is the one that decides
/// where the site sits in the frame.** The numbers under it are the ones measured above; what is worth
/// writing down is that the correction is to the aim alone, because the site's silhouette is smaller
/// than its frame in both directions and only its position was wrong.
const AIM: [f64; 3] = [1.53534, -0.0711, -0.44964];

/// The site's own outline: a courtyard round the shed, open at one gate.
const YARD: f32 = 6.5;

/// The gate's own width and height: the 3.2 m opening, and a leaf a little lower than the 1.9 m wall.
const GATE_WIDTH: f32 = 3.2;
const GATE_HEIGHT: f32 = 1.7;

/// The store's own mass **in this skeleton**, which is not the mass it was the id of in its own.
const STORE_MASS: MassId = MassId(1);

/// The wall the doorway is cut into: **edge 3** of the store's outline, which after the shift up the
/// slope reads `[(12,-2.5), (16,-2.5), (16,2.5), (12,2.5)]`, so edge 3 runs from `(12,2.5)` to
/// `(12,-2.5)` — the **west** face, and the one the eye at `[-13.0, 6.4, 15.5]` stands most squarely
/// outside of. `dot(normal, direction to the eye)` reads `0.871` here against `0.500` for the
/// store's own north face, so this wall is seen at 29 degrees off its normal and that one at 60.
///
/// # Why the store's and not the shed's, and why this is a doorway rather than a window
///
/// **The store is reached first, because it is the only one of the two the camera's own eye can see
/// through a hole in.** Its plate's top stands at `2.56` — the seat cuts a terrace to the highest
/// ground under the footprint, and `0.16 * 16 = 2.56` is that highest — and the eye is at `6.4`, so
/// the whole drop to the floor is **`3.84 m` at about `28 m` of horizontal run**: a depression of
/// `7.8` degrees. Every sight line from the eye to a point of that plate therefore crosses the
/// store's west wall between **`2.598` and `3.061`** — a band `0.46 m` tall standing `0.04` to `0.50`
/// above the floor itself — and `4.2 * (3.5 - x) / (11.5 - x)`, the house's own arithmetic, has
/// nothing to say here.
///
/// **A window cannot cover that band, because the band touches the wall's own base.** Its lower edge
/// is `0.038 m` above the ring's base annulus, so the sill has to be the ground line itself: any
/// sill inset at all puts the opening above every one of those crossings. Hence a doorway, `0.0`
/// from the bay's sill — which is what [`shed`](crate::shed)'s own door does — and the hole is
/// `2.0 m` tall on this wall's `2.4 m`. The shed by contrast answers a window, because its plate's
/// top stands at `0.4` under an eye at `6.4` and the same lines cross its wall a metre and more
/// above its floor.
///
/// **Nothing stands between that eye and that wall.** The courtyard's own north run is topped at
/// `2.167` and the sight line crosses `z = 6.5` at `y = 4.17`; the ray passes east of the shed
/// entirely. This is the measurement the constraint asks for — *place the opening so a pixel of the
/// plate's top is visible from the camera* — and it is the store's plate that this one opening shows.
const STORE_WEST_EDGE: usize = 3;

/// How many bays the store's walls carry, and which one the doorway stands in.
///
/// **Two bays, and the first** — the nearer half of a wall the eye approaches from `+z`. The lines
/// above reach the wall between `z = 0.37` and `z = 5.5` while bay 0 spans `z 0..=2.5`, so the whole
/// northern half of the crossing is inside it and none of it is cut off at either jamb.
const STORE_WALL_BAYS: usize = 2;
const DOOR_BAY: usize = 0;

/// The doorway's own insets: [`shed`](crate::shed)'s door, in metres — a jamb at each end, nothing
/// at the sill, `0.45 m` down from the head.
///
/// **The jambs are this wall's own thickness plus the `50 mm` [`shed`](crate::shed) leaves over its
/// own**: a wall's offset loop runs `wall ..= length - wall`, so a jamb narrower than the wall puts
/// the reveal on a corner the neighbouring wall still claims, and `expand` calls that `past_offset`.
fn store_door() -> Inset {
    let jamb = STORE.wall + 0.05;
    Inset::new(jamb, jamb, 0.0, 0.45)
}

/// The shed's own mass in this skeleton, which is the one [`site`] composes first.
const SHED_MASS: MassId = MassId(0);

/// The wall the window is cut into: **edge 3** of the shed's own outline, which `derive::starter`
/// builds centred on the origin as `[(-2.5,-4), (2.5,-4), (2.5,4), (-2.5,4)]`, so edge 3 runs from
/// `(-2.5, 4)` to `(-2.5, -4)` — the **west** face. It is the same face [`STORE_WEST_EDGE`] names
/// on the store and for the same reason: the eye at `[-13.0, 6.4, 15.5]` stands west of both
/// buildings.
///
/// # Why this wall takes a window and the store's takes a doorway
///
/// **The two plates are more than a metre apart on the same slope, and that is the whole
/// difference.** The seat cuts a terrace to the highest ground under each footprint, so the shed's
/// plate top stands at **`0.4`** — `0.16 * 2.5`, the shed's own east edge — where the store's stands
/// at **`2.56`**, and the eye is at `6.4` for both. A sight line from that eye to a point of the
/// shed's plate crosses the plane `x = -2.5` at
///
/// ```text
/// y = 6.4 - 6.0 * 10.5 / (x + 13.0)   ->  0.5395 at x = -2.25,   2.2689 at x = 2.25
/// ```
///
/// **a band `1.73 m` tall whose lower edge stands `0.14 m` above the floor**, where the store's band
/// is `0.46 m` tall with its lower edge `0.038 m` above the ring's base annulus. **A sill can be
/// `0.14 m` up this wall and still lie under every crossing on it; on the store's wall no sill inset
/// at all reaches.** So the shed answers a window and the store a doorway, and the geometry chooses
/// between them rather than the scene.
///
/// **Nothing stands in the way, and it was checked rather than assumed.** The same rays cross the
/// courtyard's own west run at `y 2.99 .. 3.86` against a run whose top is at most `0.86` — `1.9` of
/// wall over ground `-1.04` — and they cross `z = 6.5` at `y 2.9 .. 3.6` against that run's top,
/// which is lower still.
const SHED_WEST_EDGE: usize = 3;

/// How many bays the shed's walls carry, and which one the window stands in.
///
/// **Three bays and the first** — [`shed`](crate::shed)'s own number, and the bay the eye
/// approaches. The face runs `8.0 m` from `z = 4` to `z = -4`, so bay 0 spans `z 1.333..=4` and the
/// `0.30 m` jambs leave the hole `z 1.633..=3.70`: at `pz = 0` the sight lines that reach the plate
/// cross this wall between `z = 0.36` and `z = 4.83`, and bay 0's window holds every crossing of
/// that band inside the wall's own run.
const SHED_BAYS: usize = 3;
const WINDOW_BAY: usize = 0;

/// The window's own insets: a jamb at each end, and [`shed`](crate::shed)'s sill and head unchanged.
///
/// **The sill and the head are `shed`'s numbers, `1.05 m` up and `0.55 m` down**, which on this
/// wall's `0.4 .. 3.2` extent reads a hole **`1.2 m` tall**. The jambs are this wall's own thickness
/// plus the same `50 mm` [`shed`](crate::shed) leaves over its own, for the reason [`store_door`]
/// gives: a wall's offset loop runs `wall ..= length - wall`, and a jamb narrower than the wall puts
/// the reveal on a corner the neighbouring wall still claims whole, which `expand` calls
/// `past_offset`. Here that reads a hole **`2.067 m` across**.
///
/// **The numbers are `shed`'s, and the geometry is met without moving them.** Swinging the grid
/// alone changes how much of the plate stands behind the hole, because the sill's height is what
/// selects a band of it: this window puts **`6.67 m²`** of the plate's `33.75 m²` where a reader can
/// see it, and `shed`'s own `1.05` is what puts that there.
fn shed_window() -> Inset {
    let jamb = SHED.wall + 0.05;
    Inset::new(jamb, jamb, 1.05, 0.55)
}

fn node(x: f32, z: f32) -> PlanNode {
    PlanNode::new([x, z], 0.0)
}

/// Returns the site's document: two derived buildings and a five-run courtyard with a gate, all in one
/// skeleton, none of it yet on the ground.
fn site() -> BuildingSkeleton {
    let shed = derive::starter(&SHED).skeleton;
    let mut store = derive::starter(&STORE).skeleton;
    // The second building moves up the slope: its mass, its plate, its roof and the roof's host all shift
    // together, because a roof that named the wrong mass would stand on the first building's walls, and a
    // plate left at the origin would lie under the shed instead of under the store it belongs to.
    for mass in &mut store.masses {
        for point in &mut mass.outline {
            point.position[0] += STORE_OFFSET;
        }
    }
    // **The plate's own footprint shifts with the mass**, which is the half of the store that the mass loop
    // above cannot reach: a slab carries plan positions and no elevations, so moving it is the same offset
    // on the same axis, and the seat still owns the two levels.
    for slab in &mut store.slabs {
        for point in &mut slab.outline {
            point[0] += STORE_OFFSET;
        }
    }
    for roof in &mut store.roofs {
        for eave in &mut roof.eaves {
            eave.position[0] += STORE_OFFSET;
        }
        for crest in &mut roof.crest {
            crest.position[0] += STORE_OFFSET;
        }
    }
    let mut skeleton = shed;
    skeleton.masses.push(store.masses.remove(0));
    let mut store_roof = store.roofs.remove(0);
    store_roof.host = MassId(1);
    skeleton.roofs.push(store_roof);
    // **The store's plate crosses with its mass, and is hosted by the mass it has become.** A mass is a
    // wall ring *and* a plate, so a store that carried only the ring across would be a building with no
    // floor in the volume model — the same thing a starter's plate suffers when its mass is taken and it
    // is not. The host is `MassId(1)`, the id the store's mass holds in this skeleton and not in its own.
    skeleton
        .slabs
        .push(store.slabs.remove(0).with_host(MassId(1)));

    // The courtyard: four sides and a gate gap in the south one. The gate is where no wall was authored.
    skeleton.walls = vec![
        Wall::new(node(-YARD, -YARD), node(YARD, -YARD), 0.35, 1.9),
        Wall::new(node(YARD, -YARD), node(YARD, YARD), 0.35, 1.9),
        Wall::new(node(YARD, YARD), node(1.6, YARD), 0.35, 1.9),
        Wall::new(node(-1.6, YARD), node(-YARD, YARD), 0.35, 1.9),
        Wall::new(node(-YARD, YARD), node(-YARD, -YARD), 0.35, 1.9),
    ];
    // **The gate is a part hung on a wall, not the gap itself.** The leaf is one template placed at the end
    // of the run that stops at the opening, and it closes the 3.2 m beyond it — the one shape a bay cannot
    // hold, which is why the placement target had to learn to be a wall run.
    let stops_at_the_gate = skeleton.walls[2].length();
    let leaf = ops::add_template(&mut skeleton, gate_leaf(GATE_WIDTH, GATE_HEIGHT, 0.08));
    ops::place_part(
        &mut skeleton,
        Placement::on_wall(leaf, WallId(2), Pin::new(stops_at_the_gate, 0.0)),
    )
    .expect("the run that stops at the gate is in the document");
    skeleton
}

/// Cuts the two openings this site carries: the store's doorway and the shed's window.
///
/// # Invariants
///
/// **It is called after [`seat`] and never before it.** A rhythm's rows are laid over the face's own
/// extent, and `face_extent` reads the **seated** elevations: the store's outline stands at `2.56`
/// once the ground has been written into it and at `0.0` before. A bay laid on the authored extent
/// would put the doorway `2.56 m` below the wall it is cut into — a hole in the ground outside the
/// building rather than a hole in its wall — and `shed` and `house`, which are never seated, are
/// the only reason the two call orders are not visibly different from the outside.
///
/// **One rhythm, one storey, over both buildings and every face**, so a re-split of either moves
/// both openings with it. Each opening is then **one slot of one of those faces**, authored the way
/// [`shed`](crate::shed) and [`house`](crate::house) author theirs: `Rhythm::Even` over every face
/// and `Opening` on one slot of one of them.
fn open_the_two_walls(skeleton: &mut BuildingSkeleton) {
    let rows = Rhythm::Even { count: 1 };
    for (mass, bays) in [(STORE_MASS, STORE_WALL_BAYS), (SHED_MASS, SHED_BAYS)] {
        let columns = Rhythm::Even { count: bays };
        for face in skeleton.faces_of(mass) {
            ops::split_face(skeleton, face, &columns, &rows);
        }
    }
    ops::add_opening(
        skeleton,
        Opening::new(
            FaceRef::new(STORE_MASS, STORE_WEST_EDGE),
            0,
            DOOR_BAY,
            store_door(),
        ),
    )
    .expect("the doorway names a bay of the grid");
    ops::add_opening(
        skeleton,
        Opening::new(
            FaceRef::new(SHED_MASS, SHED_WEST_EDGE),
            0,
            WINDOW_BAY,
            shed_window(),
        ),
    )
    .expect("the window names a bay of the grid");
}

/// Returns a gate leaf: a panel `width` across, `height` tall, `thickness` through, standing at its own
/// origin and running `+x`.
///
/// # Invariants
///
/// Its plan starts at the origin so the placement's frame — the run's end, on the wall's centre line — puts
/// its hinge there and the panel swings across the opening beyond. The panel is **centred on the wall's
/// thickness** by the frame's `inward`, so a gate in a 0.35 m wall does not stand proud of either face.
fn gate_leaf(width: f32, height: f32, thickness: f32) -> Template {
    let slab = 0.03;
    let outline = [
        [0.0, -thickness * 0.5],
        [width, -thickness * 0.5],
        [width, thickness * 0.5],
        [0.0, thickness * 0.5],
    ]
    .into_iter()
    .map(|corner| PlanNode::new(corner, slab))
    .collect();
    Template::new(
        "gate",
        vec![Mass::new(outline, height).with_wall(0.02).with_slab(slab)],
        Vec::new(),
    )
}

/// Builds the yard scene.
pub fn yard() -> Value {
    let mut skeleton = site();
    let seated = seat(
        &mut skeleton,
        |plan| ground(plan[0] as f64, plan[1] as f64) as f32,
        0.0,
    );
    // **The scene refuses to draw a site it could not seat.** A mass the ground never answered for is
    // left at its authored elevation, which on this slope means floating or buried — and the frame is
    // exactly where that would hide. So the count is checked here rather than trusted.
    assert!(
        seated.len() == 2 && seated.walls == 5,
        "the site must be seated: {} masses and {} walls, not 2 and 5",
        seated.len(),
        seated.walls
    );
    // **After the seat and before the document**, because the store's bays are laid over the
    // elevations the ground wrote into it. See `open_the_two_walls`.
    open_the_two_walls(&mut skeleton);
    let document = Building::new(skeleton.clone());
    let built = document.build().expect("the seated site expands");
    // **The scene refuses to draw a site the crate's validator would not**, for the reason the seat's
    // own check above gives: the frame is exactly where a fault in the document would hide. A yard
    // that grows an opening grows the one thing most likely to fault, so the check is here rather
    // than in a probe that reports it.
    let violations = document.validate();
    assert!(
        violations.is_empty(),
        "the yard's own document must be clean, and it is not: {violations:?}"
    );

    // **Every mesh the frame draws carries its elements' ids** — the shells, the roofs, the walls and the gate —
    // so a pixel of this yard can be asked what it is. The gate is one template drawn once, so its mesh carries
    // the **template's** local ids and its material offsets them onto the row a bake would have written.
    let table = Identity::of(&built, 1);
    let shells = packed_where(&built, &table, |element| {
        matches!(element, Element::Mass(_, _))
    });
    let roofs = packed_where(&built, &table, |element| {
        matches!(element, Element::Roof(_, _))
    });
    // **The two 1.20 m foundations the slope needs**, the shed's and the store's carried across with its
    // mass: the shed's edge stands the relief above the ground, and a slab carries it down. The yard has
    // authored both since the store's plate crossed, and drew neither.
    let floors = packed_where(&built, &table, |element| {
        matches!(element, Element::Slab(_, _))
    });
    let stone = packed_where(&built, &table, |element| {
        matches!(element, Element::Wall(_, _))
    });
    // **The gate is drawn from its template, once, and stood by an instance.** A scene that filtered the
    // baked mesh for parts would draw every leaf the document holds; this one draws the one shape and the
    // one transform that puts it in the opening.
    let gate = skeleton
        .parts
        .template(0)
        .expect("the gate's template is in the document");
    // **The gate's offset, read from the baked table**: the least id the table gives an element of the gate's
    // placement, less one, so the template's local ids land on exactly the rows a bake would have written.
    let gate_at = (0..skeleton.parts.placements().len())
        .find(|index| {
            skeleton
                .parts
                .placement(*index)
                .is_some_and(|placement| placement.target == Target::Wall { wall: WallId(2) })
        })
        .expect("the gate is placed on the run that stops at the opening");
    let gate_offset = i64::from(
        table
            .rows()
            .iter()
            .filter(
                |row| matches!(row.element, Element::Part { placement: at, .. } if at == gate_at),
            )
            .map(|row| row.id)
            .min()
            .expect("the gate draws at least one surface"),
    ) - 1;
    let gate_local = Identity::of(&gate.mesh().expect("the gate expands on its own"), 1);
    let gate_panel = packed_where(
        &gate.mesh().expect("the gate expands on its own"),
        &gate_local,
        |_| true,
    );
    let gate_standing = standing(
        &building::expand::part_frame(&skeleton, gate_at)
            .expect("the wall offsets")
            .expect("the gate's slot is in the domain"),
    );

    // Weatherboard, sheet metal, dry-stone and earth. Four materials for four things the frame has to tell
    // apart: what a surface *is* stays the shader's business.
    let board = || surface([0.36, 0.27, 0.18], 0.0, 0.86, [0.0; 3]);
    let metal = || surface([0.34, 0.37, 0.40], 0.0, 0.38, [0.0; 3]);
    let rubble = || surface([0.66, 0.65, 0.60], 0.0, 0.92, [0.0; 3]);
    let earth = || surface([0.24, 0.25, 0.16], 0.0, 0.95, [0.0; 3]);
    // Rubble rather than board: a foundation this deep is masonry on a slope, and it is the one surface of the
    // two a viewer can see standing free of the ground.
    let plinth = || surface([0.50, 0.49, 0.45], 0.0, 0.93, [0.0; 3]);

    let mut materials = Vec::new();
    for (name, body) in [
        ("board", board as fn() -> Value),
        ("metal", metal),
        ("rubble", rubble),
        ("plinth", plinth),
    ] {
        materials.push(mesh_material(
            &format!("{name}-cast"),
            "authored_shadowing",
            body(),
        ));
        materials.push(mesh_material(
            &format!("{name}-depth"),
            "authored_depthing",
            body(),
        ));
        materials.push(mesh_material(
            &format!("{name}-shade"),
            "authored_lighting",
            body(),
        ));
    }
    for (stage, shader) in [
        ("cast", "authored_shadowing"),
        ("depth", "authored_depthing"),
        ("shade", "authored_lighting"),
    ] {
        materials.push(mesh_material(
            &format!("gate-{stage}"),
            shader,
            offset_by(board(), gate_offset as f64),
        ));
    }
    // The ground is not an element of the site, and it still writes the output — with the reserved zero.
    for (stage, shader) in [
        ("cast", "plain_shadowing"),
        ("depth", "plain_depthing"),
        ("shade", "plain_lighting"),
    ] {
        materials.push(mesh_material(&format!("ground-{stage}"), shader, earth()));
    }

    let placed = |stage: &str| {
        let mut rows = [
            ("shells", "board"),
            ("roofs", "metal"),
            ("stone", "rubble"),
            ("floors", "plinth"),
            ("ground", "ground"),
        ]
        .into_iter()
        .map(|(label, material)| {
            instance(
                format!("{label}-{stage}"),
                label,
                [0.0; 3],
                [0.0; 3],
                [1.0; 3],
                &format!("{material}-{stage}"),
            )
        })
        .collect::<Vec<_>>();
        rows.push(instance(
            format!("gate-{stage}"),
            "gate",
            gate_standing.0,
            gate_standing.1,
            gate_standing.2,
            // **The gate's own material**, which carries the offset. One material per placement is what makes
            // an instanced mesh's pixels answer the rows a bake would have written — and the row naming
            // `board-{stage}` instead was the third time this slice's material was named twice and once wrong.
            &format!("gate-{stage}"),
        ));
        rows
    };

    // **Two point schemas**: every authored mesh carries `authored_id`; the ground carries none and writes
    // the reserved zero from a constant.
    let extras = building_extras();
    let point = extras.point_schema();
    let plain = not_authored_extras();
    // **One air, handed to two renderers**: the map a world reads and the light a surface sees. Without the
    // light the shading's sky term is zero, which is why a sun-only scene's shadowed side is black.
    let (sky_image, sky_light) = sky(&atmosphere());

    Value::structure([
        ("name".to_owned(), Value::String("yard".to_owned())),
        // The one field here that is about the picture rather than what is in it: a yard is a broad
        // site and a broad site wants a broad frame. A scene that names no frame is drawn into the
        // pipeline's own square, so this is a decision rather than a default.
        ("frame".to_owned(), frame(FRAME_WIDTH, FRAME_HEIGHT)),
        (
            "cameras".to_owned(),
            // Up the slope and to one side, high enough to see over the yard wall into the courtyard and
            // low enough that the gate gap reads as a way in rather than a step in the wall. **The eye did
            // not move and the lens did not either**; the aim is what this scene's own frame asked for,
            // and the frame's shape is what let the site fill it.
            Value::List(vec![camera([-13.0, 6.4, 15.5], AIM, FIELD_OF_VIEW)]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![
                light_with_radius(
                    "sun",
                    SUN,
                    [1.0, 0.87, 0.70],
                    flux_for_irradiance(SUN_IRRADIANCE, distance_to_origin(SUN)),
                    16.0,
                    160.0,
                ),
                sky_light,
            ]),
        ),
        ("images".to_owned(), Value::List(vec![sky_image])),
        (
            "shaders".to_owned(),
            Value::List({
                let helpers = atmosphere().wgsl();
                let mut shaders = stage_set("authored", "", helpers.as_str(), &point, &extras);
                shaders.extend(stage_set(
                    "plain",
                    "",
                    helpers.as_str(),
                    &plain.point_schema(),
                    &plain,
                ));
                shaders.push(post_shader(GRADE, &helpers, Some(Projection::Latlong)));
                shaders
            }),
        ),
        // The one image this scene writes beside the frame: which element each pixel shows.
        ("outputs".to_owned(), outputs_field()),
        (
            "meshes".to_owned(),
            Value::List(vec![
                mesh("shells", "Triangles", point.clone(), shells),
                mesh("roofs", "Triangles", point.clone(), roofs),
                mesh("stone", "Triangles", point.clone(), stone),
                mesh("floors", "Triangles", point.clone(), floors),
                mesh("gate", "Triangles", point, gate_panel),
                mesh(
                    "ground",
                    "Triangles",
                    shaded_point_schema(),
                    crate::ground::mesh(&document, 160, 42.0, ground),
                ),
            ]),
        ),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("post", "post")]),
        ),
        ("post_processes".to_owned(), post_processes(&["post"])),
        ("mesh_materials".to_owned(), Value::List(materials)),
        (
            "instances".to_owned(),
            instances(vec![
                (Stage::Shadowing, placed("cast")),
                (Stage::Depthing, placed("depth")),
                (Stage::Lighting, placed("shade")),
            ]),
        ),
    ])
}

/// The air over the yard: a clear afternoon, and **the light the sun is not**.
///
/// The numbers are the ones this scene's own sky law carried — zenith, horizon, ground and the sun's
/// bloom — read out of it and handed to one `Atmosphere`, which bakes them into the map the shading reads
/// its sky term from. A yard whose shadows are black is a yard with no air, not a yard with the wrong
/// colours.
fn atmosphere() -> Atmosphere {
    Atmosphere::new()
        .zenith([0.14, 0.25, 0.47])
        .horizon([0.86, 0.73, 0.56])
        .ground([0.30, 0.27, 0.21])
        .sun([-16.0, 30.0, 24.0], [1.0, 0.86, 0.62], 8.0, 0.32)
        .density(0.0011)
        .scale_height(200.0)
}

/// Returns how far a light position stands from the scene's own middle.
fn distance_to_origin(position: [f64; 3]) -> f64 {
    (position[0] * position[0] + position[1] * position[1] + position[2] * position[2]).sqrt()
}

/// What the post stage does to the frame: the same shape as the shed's, with the air of a wider view.
const GRADE: &str = r#"
    // **The sky is read out of the scene's own environment map**, the same air the bounce reads, so the
    // background and the light cannot be two skies.
    if depth >= 1.0 {
        color = environment_sky(ray_through(in.uv));
    }
    if depth < 1.0 {
        let ray = ray_through(in.uv);
        let along_ray = length(world_position(in.uv, depth) - uniforms.camera.position);
        let air = clamp(1.0 - exp(-along_ray * 0.005), 0.0, 0.60);
        color = mix(
            color,
            environment_sky(vec3<f32>(ray.x, max(ray.y, -0.04), ray.z)),
            air,
        );
    }
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = clamp(mix(vec3<f32>(luma), color, 1.20), vec3<f32>(0.0), vec3<f32>(1.0));
    let centred = in.uv - vec2<f32>(0.5, 0.5);
    color = color * clamp(1.0 - dot(centred, centred) * 0.40, 0.0, 1.0);
"#;

/// Returns the yard's document **as the scene builds it**: the site, seated on the ground it crosses.
///
/// # Invariants
///
/// It is the same document the scene draws — the seat is part of the picture, so a caller naming what a frame
/// shows must name it after the seating rather than before.
pub fn document() -> Building {
    let mut skeleton = site();
    seat(
        &mut skeleton,
        |plan| ground(plan[0] as f64, plan[1] as f64) as f32,
        0.0,
    );
    // **The same two openings the frame draws**, for the same reason and in the same places: a
    // caller naming what this frame shows must name the seated document, not the document before it.
    open_the_two_walls(&mut skeleton);
    Building::new(skeleton)
}
