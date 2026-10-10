//! A walled yard cut into the floor of a valley, in the middle of a clear afternoon.
//!
//! # What the picture is
//!
//! A stone wall stands across the middle of the frame, with a gate in it and a beam over
//! the gate. Behind the wall is a yard: a paved surface, a shed under a tent roof, three
//! turned vessels standing against the shed, a rack of poles. Behind the yard the land
//! climbs in cut terraces to a massif that stands in one quarter of the valley and to
//! nothing else, and beyond the whole of it a far range lies in the afternoon's haze.
//!
//! The wall's coping is the one unbroken horizontal line in the frame, and it is what the
//! dark verticals of everything in front of it are read against. So the coping is laid
//! **wider** than the wall it caps: an overhang of a quarter of a metre is the shadow
//! line along the top of the wall, and a coping flush with its wall is a lid with no line
//! on it at all.
//!
//! # What it is built from
//!
//! Every height function and every solid form the gallery holds is called here, and the
//! scene is arranged so that each of them is doing the job its own documentation says it
//! exists for:
//!
//! - The ground is a [`Valley`] — a long fall, a range at its head, two lower walls, a far
//!   body — plus one more [`Ridge`] for the massif, which [`quarter`] confines to a single
//!   corner so the high ground is a **valley** and not a bowl. The massif's own high ground
//!   is multiplied by [`edge_taper`], so it dies to nothing before the terrain's mesh stops
//!   and the far range, which is drawn as its own mesh, closes the horizon behind it.
//! - The yard is a level platform: [`bench`] over the distance to a rectangle from
//!   [`box_distance`], so the bank rises out of the paving as one continuous surface whose
//!   normal the heightfield can be true to.
//! - The terraced band is [`terrace`], gated by [`slope`] measured over the heightfield's
//!   own cell, so it is cut where the land will hold a level and not where it would be a
//!   stripe on a cliff.
//! - The enclosure is a [`built::wall_ring`] with a gate left as a **gap** in the run, two
//!   piers either side of it, and a [`built::sweep`] beam over the opening that runs past
//!   both piers so its own end faces are buried inside them rather than coincident with
//!   theirs. A second [`built::wall_ring`] over the same corners is the coping, wider than
//!   the wall. A third is the shed, with a wide opening; a fourth is the fascia under the
//!   roof's eaves, which is what stops a roof from being a sheet of paper with its edge in
//!   the air. A fifth is a free-standing field wall out on the slope, its outer face
//!   decided by [`built::edge_normal`] rather than by the direction it happens to run.
//! - The roof is [`built::roof_ridge`] fed straight to a heightfield, so the crest, the
//!   eave's turn and the settle along the ridge are the generator's own arithmetic.
//! - The jar, the basket and the millstone are [`built::JAR`], [`built::BASKET`] and
//!   [`built::MILL`] turned by [`lathe`].
//!
//! # What the three cameras are
//!
//! One stands outside the gate, where the coping line and the lintel are the subject. One
//! stands in the yard with the paving under it, where the shed and the vessels are. One
//! stands across the valley on a long lens, where the enclosure is a shape in the land and
//! the terraces are the subject. Nothing about the scene changes between them but where the
//! photographer is standing and which lens is on the camera.

use definition::{PackedList, Value};
use scene::standard::Stage;

use crate::air::Atmosphere;
use crate::built::{self, BASKET, JAR, MILL, WallGap};
use crate::geometry::{
    Shaded, add, heightfield, lathe, normalized, plane_facing, scale, smoothstep, subtract,
};
use crate::kit::{
    camera_placed_on_lens, environment_light, image, instance, instances, light_with_radius, mesh,
    mesh_material, post_processes, screen_material, shaded_point_schema, surface,
    surface_reflecting,
};
use crate::land::{self, Ridge, Valley};
use crate::ridges;
use crate::shaders::{Projection, displaced_lighting_shader, mesh_shaders, post_shader};

// ---------------------------------------------------------------------------
// The ground
// ---------------------------------------------------------------------------

/// How far the terrain reaches from its centre, in world units.
const EXTENT: f64 = 150.0;

/// How many cells the terrain is divided into along each axis.
///
/// A riser is about half a terrace step of horizontal run, and a step is two and a bit
/// units, so the grid has to turn in about a unit for the riser to be a slope rather than a
/// torn edge. The cell size is also the `step` [`land::slope`] is measured over, and the
/// two have to be the same number or the gate that decides where terraces are cut is
/// measured over a different surface from the one being cut.
const DIVISIONS: usize = 300;

/// The terrain's own cell size, in world units.
const CELL: f64 = 2.0 * EXTENT / DIVISIONS as f64;

/// The heights the terraced band is cut between.
const BAND_LOW: f64 = 15.0;
const BAND_HIGH: f64 = 62.0;

/// The step of the terraces, and the fraction of each step held flat. The rest of the step
/// is the riser, which is the only place a riser can have a width at all.
const TERRACE_STEP: f64 = 2.2;
const TERRACE_FLAT: f64 = 0.46;

/// The slopes a terrace is cut on, as a gradient. A twelfth of a rise in a run is a gentle
/// field; a quarter is a bank, and a quarter of a rise in a run is a face.
const TERRACE_FLAT_SLOPE: [f64; 2] = [0.20, 0.46];
/// How far past the yard's own bank the farmed ground reaches before it stops being terraced.
///
/// The number is a width of field rather than a height, and it is what keeps the terraces off
/// the mountainside: without it the band alone cuts every slope in the landscape that stands
/// at the right altitude, and a hill cut into steps is a contour map drawn in three
/// dimensions.
const TERRACED_REACH: f64 = 21.0;

/// The corner the massif is confined to, and the distance its gate falls away over.
const QUARTER_CORNER: [f64; 2] = [-16.0, -12.0];
const QUARTER_FADE: f64 = 70.0;

/// Where the land's own high ground starts dying, and where it has died. The second is
/// inside the field, which is the whole of the point: a silhouette that ends at the mesh's
/// own edge is a cliff of cut earth standing against the sky.
const TAPER: [f64; 2] = [92.0, 146.0];

// ---------------------------------------------------------------------------
// The yard
// ---------------------------------------------------------------------------

/// The centre of the yard, and the half width of the platform the bench cuts into it.
const YARD: [f64; 2] = [0.0, 0.0];
const YARD_HALF: [f64; 2] = [14.0, 10.4];

/// How far the platform's bank eases out past its own edge.
const YARD_BANK: f64 = 5.6;

/// The enclosure: half width, the chamfer off each corner, and the two widths of the bands
/// it is built from. The coping is deliberately the wider of the two.
const RING_HALF: [f64; 2] = [11.0, 7.6];
const RING_CHAMFER: f64 = 2.4;
const WALL_THICKNESS: f64 = 0.56;
const COPING_THICKNESS: f64 = 1.06;
const WALL_RISE: f64 = 2.55;
const COPING_RISE: f64 = 0.30;

/// The number of faces a wall run is made of, so a run of points can be sized for the
/// number of runs about to go into it rather than grown.
const WALL_FACES: usize = 5;

/// The gate, as the fractions of the run it interrupts, and the half width between the two
/// piers that flank it. The run it falls in is the long southern side of the ring, which is
/// the side a camera on the open ground sees.
const GATE: [f64; 2] = [0.44, 0.56];
const GATE_HALF: [f64; 2] = [1.46, 0.0];

/// The far range's own mesh, as the half width it reaches and where its origin stands.
///
/// The origin is a long way beyond the terrain's edge rather than at it. Everything the
/// valley put up is now tapered to nothing before its own mesh stops, so what stands on the
/// horizon from the yard is this and nothing else, and it has to be far enough off to lie in
/// the haze rather than stand in it.
const FAR_EXTENT: f64 = 300.0;
const FAR_AT: [f64; 3] = [0.0, 0.0, -250.0];

/// The shed: where it stands in the yard, its half width, and the open half of its run.
///
/// It is square, because the roof over it is a [`heightfield`] and a heightfield is square:
/// a shed whose roof overhangs half a metre on one side and a metre on the other is a shed
/// with two different eaves, and the eave is the one line that says which tradition the
/// building is in.
const SHED: [f64; 2] = [-2.2, -2.6];
const SHED_HALF: [f64; 2] = [3.05, 3.05];
const SHED_CHAMFER: f64 = 0.7;
const SHED_RISE: f64 = 2.70;
const SHED_THICKNESS: f64 = 0.42;
const SHED_DOOR: [f64; 2] = [0.26, 0.74];

/// The roof over the shed: the half run from ridge to eave, the rise, and the height the
/// heightfield's own origin stands at.
///
/// The origin is the third number and it is the one that decides whether the roof is a
/// roof. A ridge profile is a **tent**: it is highest at the ridge and falls to the eave,
/// and the eave is the lowest point of it. Hang it too low and the eave finishes below the
/// top of the wall it caps, so the overhang is buried in the masonry and the roof reads as a
/// lid; hang it too high and there is a band of open air between the wall and the tiles
/// with the sky showing through it. This one puts the eave a hand's width clear of the
/// coping, which is what an overhang is for.
const ROOF_RUN: f64 = 3.35;
const ROOF_RISE: f64 = 1.45;
const ROOF_ORIGIN: f64 = 2.56;

/// The shed's roof edge, as the band of wall that caps it and the height it stands between.
///
/// The band is **wider than the roof it caps** and taller than the coping it hides, so the
/// overhang is where the eave's own shadow falls. It is a band of wall and not a second
/// sheet of roof because a downward facing face is the one thing a heightfield cannot be.
const EAVE_HALF: f64 = ROOF_RUN + 0.06;
const EAVE_BAND: [f64; 2] = [2.68, 2.84];
const EAVE_THICKNESS: f64 = 0.16;

/// The colonnade inside the yard: the near end of the row, and each pole's offset from it
/// with the height it stands at.
///
/// # Invariants
///
/// The row runs along the line the camera inside the yard is looking down and is held a
/// couple of metres to one side of it, and that offset is the whole of the difference
/// between a colonnade and a cage. A row the camera is standing among is a row of bars
/// across the picture, and every one of them is in front of the thing the frame is about;
/// the same row two metres to one side crosses from the edge of the picture towards its
/// middle, which is what a colonnade is for. The heights are not equal, because a row of
/// equal poles is a fence and a row of poles that are all a little different is carpentry.
const COLONNADE_AT: [f64; 2] = [7.2, 5.6];
const COLONNADE: [([f64; 2], f64); 4] = [
    ([-1.02, -4.10], 2.44),
    ([-2.10, -4.99], 2.62),
    ([-3.25, -5.95], 2.31),
    ([-4.48, -6.97], 2.53),
];

/// The free-standing field wall out on the slope, as the two points it stands between.
///
/// It is between the camera that stands outside the gate and the yard, and low, so that it
/// is the **near** thing in that frame: a wall of the same masonry as the one behind it, a
/// few metres away, running out of the side of the picture. A frame whose only wall is the
/// subject has no depth in it at all. One end is behind that camera and the other stands
/// inside the frame with a pole beside it, which is what says the wall goes on past the
/// picture rather than stopping in the middle of a field.
const FIELD_RUN: [[f64; 3]; 2] = [[29.0, 0.0, 24.5], [16.0, 0.0, 27.2]];

/// How far the sun stands in the frame's quarter.
///
/// A yard is a **horizontal** subject, and a sun high in the quarter lights every horizontal
/// surface in it equally: the paving and the yard floor take the light face on and come back
/// the same near-white, and a photograph of a flat slab under a flat light has no shape in it
/// at all. Raked lower, the paving takes the light at a grazing angle, the shed's own shadow
/// lies across it, and the wall's coping throws a line along the top of the band — which is
/// the one thing in the frame that says where the wall's height is. The hillside across the
/// valley goes dark, and that is the trade: a landscape lit from the side has a lit side and
/// a dark one, and a scene where everything is lit has neither.
const SUN_TOWARD: [f64; 3] = [0.300, 0.400, 0.690];

/// Where the sun stands: the yard's centre, four hundred units along its own bearing, so
/// that a point light stands in for it and the whole of the yard is within its reach.
const SUN_AT: [f64; 3] = [120.0, 264.0, 276.0];

/// Builds the scene.
pub fn foldyard() -> Value {
    let groups = groups();
    let (materials, placed) = build(&groups);
    Value::structure([
        ("name".to_owned(), Value::String("foldyard".to_owned())),
        (
            "frame".to_owned(),
            Value::structure([
                ("width".to_owned(), Value::Int(1536)),
                ("height".to_owned(), Value::Int(1024)),
                // A point light's shadow volume is one box for the whole scene, so a yard in
                // a three hundred unit landscape shadows through a page that has to cover all
                // of it. At the default thousand and twenty four, one texel of that page is
                // half a world unit; at two thousand and fifty six it is a quarter of one,
                // and the yard's own floor is a surface the sun strikes at forty degrees, so
                // the depth across a single texel of it is a third of a unit of world. That
                // is the measurement that puts a visible grid of light and dark squares
                // across the paving, and it is the whole of the case for the page's size
                // being a scene's decision: the grid is a function of how many texels the
                // yard's floor is given, not of anything the yard itself is doing.
                ("shadow_page_width".to_owned(), Value::Int(4096)),
                ("shadow_page_height".to_owned(), Value::Int(4096)),
            ]),
        ),
        ("cameras".to_owned(), Value::List(cameras())),
        (
            "lights".to_owned(),
            Value::List(vec![
                // The sun, as a point light standing a long way off along its own bearing,
                // which is the gallery's idiom for a sun over a landscape this size: the
                // standard shading attenuates by the inverse square law, and a valley three
                // hundred wide needs the sun far enough off that the falloff across it is a
                // gradient rather than a cliff. Its distance is part of how bright it is, so
                // what a scene states is a `power` and the number reads as a squared
                // distance — this one stands about 400 units off. See
                // `.agents/proposals/physical-light-units.md`.
                //
                // The radius is the distance to the far corner of the scene and no more, so
                // this light is **not** promoted to a global one: the compiler culls it out of
                // the clusters past the far range, and the yard — which is what the three
                // cameras are about — is in every cluster it is in at all. Naming a larger
                // radius would promote it and change no pixel while claiming to reach
                // something it does not.
                //
                // The shadow volume is a box of this half width centred on the whole of what
                // the scene draws, and the sun stands outside it: a volume wide enough to
                // hold the light is a volume the light is inside, and a projection from
                // inside its own volume is not a projection of anything. It is the whole of
                // the scene rather than the yard alone because the far range is in the
                // scene's bounds, and a volume that stopped short of it would shadow-map
                // nothing and shade it as though something stood in front of it.
                light_with_radius(
                    "sun",
                    SUN_AT,
                    [1.62, 1.36, 1.02],
                    12_396_223.1,
                    260.0,
                    760.0,
                ),
                // The sky's fill, from the opposite quarter and from above. Without it
                // every face the sun has turned away from is black, and black is not what
                // the inside of a wall run looks like at this hour. It is a **fill** and it
                // is held well under the sun: a fill that competes with the key is a second
                // key, and two keys from opposite quarters give a scene where every face is
                // lit and nothing has a lit side.
                light_with_radius(
                    "sky",
                    [-70.0, 150.0, -60.0],
                    [0.28, 0.34, 0.48],
                    197.4,
                    260.0,
                    700.0,
                ),
                // The air, as the environment every shadowed surface is lit by. It is the
                // same afternoon the sky above it is drawn from.
                //
                // It is a **dome**, and a dome has no direction in it: a horizontal face
                // sees the whole of it and a steep face sees almost none, so at any strength
                // that lights a shadow it also lights the paving flat. This is the ambient
                // that says the shade is blue and not black, and it says it at a quarter of
                // the sun's strength; the sun is what makes the shape.
                environment_light("air", "air", [1.0; 3], 0.26),
            ]),
        ),
        (
            "images".to_owned(),
            Value::List(vec![image(
                "air",
                192,
                96,
                air_pixels(192, 96),
                Projection::Latlong,
            )]),
        ),
        ("shaders".to_owned(), Value::List(shaders())),
        ("meshes".to_owned(), Value::List(meshes())),
        ("mesh_materials".to_owned(), Value::List(materials)),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("post", "post")]),
        ),
        ("post_processes".to_owned(), post_processes(&["post"])),
        ("instances".to_owned(), placed),
    ])
}

// ---------------------------------------------------------------------------
// The height of the land
// ---------------------------------------------------------------------------

/// The valley the yard stands in: a long fall up to a range at its head, two lower walls
/// closing its sides, a low body beyond its open end, and an undulation under the lot.
///
/// # Invariants
///
/// The two range seeds are 131 and 139 and the undulation's is 421, and no two of them
/// are equal: a crest line and the creasing across it that agree by accident is a mass
/// whose saddles land where its crests are, which reads as a range that has been smoothed.
fn valley() -> Valley {
    Valley {
        extent: EXTENT,
        fall_low: 3.0,
        fall_high: 25.0,
        fall_steepening: 1.28,
        range: Ridge {
            axis: [1.0, 0.18],
            along_scale: 0.0045,
            across_scale: [0.0072, 0.0104],
            height: 40.0,
            floor: 5.0,
            crease_floor: 0.30,
            profile_seed: 131,
            crease_seed: 139,
            octaves: 5,
        },
        // Measured from the open end, so the range gathers at the head of the fall and the
        // fall carries the eye to it rather than the two meeting at an angle.
        range_ramp: [180.0, 320.0],
        walls: [
            Ridge {
                axis: [0.15, 1.0],
                along_scale: 0.0072,
                across_scale: [0.018, 0.016],
                height: 19.0,
                floor: 3.0,
                crease_floor: 0.32,
                profile_seed: 211,
                crease_seed: 223,
                octaves: 3,
            },
            Ridge {
                axis: [-0.2, 1.0],
                along_scale: 0.0064,
                across_scale: [0.021, 0.014],
                height: 22.0,
                floor: 3.0,
                crease_floor: 0.30,
                profile_seed: 227,
                crease_seed: 229,
                octaves: 3,
            },
        ],
        // Across the valley, so the walls close the two sides of the fall and leave the
        // floor between them.
        wall_ramp: [74.0, 146.0],
        far_body: Ridge {
            axis: [1.0, 0.0],
            along_scale: 0.0050,
            across_scale: [0.010, 0.020],
            height: 11.0,
            floor: 0.0,
            crease_floor: 0.40,
            profile_seed: 307,
            crease_seed: 311,
            octaves: 3,
        },
        far_ramp: [120.0, 200.0],
        undulation: 3.0,
        undulation_scale: 0.019,
        // Faded out where the mass is already high, which is the detail the rest of the
        // structure depends on: three metres of wrinkle is nothing on a forty metre face.
        undulation_gate: [24.0, 70.0],
        undulation_octaves: 3,
        undulation_seed: 421,
    }
}

/// The massif that closes the yard's far quarter, as a mass of its own.
///
/// It is a [`Ridge`] the same as the valley's, and it is added to the valley's height
/// rather than replacing any of it: the valley is the place and this is the hill at the
/// back of it.
fn massif() -> Ridge {
    Ridge {
        axis: [0.30, -1.0],
        along_scale: 0.0055,
        across_scale: [0.0112, 0.0088],
        height: 34.0,
        floor: 7.0,
        crease_floor: 0.34,
        profile_seed: 613,
        crease_seed: 617,
        octaves: 4,
    }
}

/// The land, before anything is cut into it.
///
/// # Invariants
///
/// The land is gated twice and for two different reasons. [`quarter`] decides **where** the
/// massif stands, and it is what keeps the high ground in one corner instead of ringing the
/// valley: high ground all the way round is a bowl, and a bowl stands its own wall between
/// a camera and its subject on every side. [`edge_taper`] decides **where the whole of it
/// stops**, and the taper is on the sum rather than on the massif alone, because the
/// valley's own range is the taller of the two masses: taper the massif and the range walks
/// to the field's own edge at full height, and the frame's horizon is a cut of raw earth
/// with the far range standing behind a wall instead of behind a crest.
fn natural(valley: &Valley, massif: &Ridge, x: f64, z: f64) -> f64 {
    let point = [x, z];
    land::edge_taper(point, TAPER[0], TAPER[1])
        * (valley.height_at(point)
            + massif.height_at(point) * land::quarter(point, QUARTER_CORNER, QUARTER_FADE))
}

/// The height the yard's paving stands at: the land under it, and a little above, so the
/// platform is a cut with a fill in the middle of it rather than a hole.
fn yard_level() -> f64 {
    natural(&valley(), &massif(), YARD[0], YARD[1]) + 0.35
}

/// How far the terrain's own platform sits **below** the yard's level.
///
/// # Invariants
///
/// The paving is a plane and the enclosure is a band of wall, and both are written at the
/// yard's level; the land under them is not. A land at the same height is a land sharing a
/// plane with both, and a shared plane is decided by the depth buffer rather than by the
/// scene: the paving breaks into hard-edged patches and the wall grows a jagged hem of
/// hillside along its foot. The paving is a hand's width above the yard's level and the land
/// is a hand's width below it, so the yard is a platform standing on the ground rather than
/// three surfaces at the same altitude.
const APRON_DROP: f64 = 0.14;

/// The height of the terrain, together with the share of the terrace cut that reached it.
///
/// # Invariants
///
/// The two come out of one function because they are one piece of arithmetic. A cut face
/// and the colour that says "this was cut" are a single decision, and a scene that answered
/// the second question somewhere else — a band of world heights, say — is answering it about
/// a different surface: the geometry is gated on a slope and on a region, so the colour has
/// to be gated on the same two, and the only place both are known is here.
///
/// The function is a pure function of its two arguments, and the gate that asks for the
/// slope asks for it through the same closure the height itself is read from, over the
/// heightfield's own cell: a slope measured over any other distance is a slope of a
/// different surface, and the one that disagrees is the one the eye finds first as a
/// hillside that is not lit like the slope it is.
///
/// The bench is cut **last**, so the yard is level whatever the terrace band did inside its
/// edge, and the cut is reported as nothing inside the bench's own easing — a platform the
/// bench has levelled is not terraced whatever the height under it was cut from.
fn ground_at(valley: &Valley, massif: &Ridge, level: f64, x: f64, z: f64) -> (f64, f64) {
    let land_at = move |px: f64, pz: f64| natural(valley, massif, px, pz);
    let ground = land_at(x, z);

    // The band the terraces belong in, by height, and the part of it that will hold a
    // level. The slope is only asked for inside the band, which is a fraction of the
    // field: it costs four more evaluations of the whole land, and a height function
    // that asks for it everywhere pays that on every point of the grid.
    let band = smoothstep(BAND_LOW, BAND_LOW + 10.0, ground)
        * (1.0 - smoothstep(BAND_HIGH - 14.0, BAND_HIGH, ground));

    // **The region the terraces are in**, and this is what a height band is not enough
    // without. A band selects every hillside in the landscape that happens to stand at the
    // right altitude, and a whole mountainside cut into `TERRACE_STEP` risers is not
    // terracing: it is a set of contour rings drawn round the hill, because each tread
    // follows a level and a smooth face has nothing but levels to cut. Terraces belong to
    // the farmed ground — the slope immediately below the yard's own bank and no further —
    // so the region is measured outward from the yard's rectangle and falls away over the
    // width of the field somebody would actually have cut.
    let farmed = 1.0
        - smoothstep(
            YARD_BANK + 3.0,
            YARD_BANK + TERRACED_REACH,
            land::box_distance([x, z], YARD_HALF),
        );

    let cut = if band * farmed > 0.002 {
        let steepness = land::slope(&land_at, [x, z], CELL);
        band * farmed * (1.0 - smoothstep(TERRACE_FLAT_SLOPE[0], TERRACE_FLAT_SLOPE[1], steepness))
    } else {
        0.0
    };
    let stepped = land::terrace(ground, TERRACE_STEP, TERRACE_FLAT);
    let terraced = ground + (stepped - ground) * cut;

    // The yard: a level platform, eased out of the slope over its own bank.
    let open = 1.0 - smoothstep(0.0, YARD_BANK, land::box_distance([x, z], YARD_HALF));
    let height = land::bench(
        terraced,
        level,
        land::box_distance([x, z], YARD_HALF),
        YARD_BANK,
    );
    (height, cut * open)
}

/// The height of the terrain on its own, for the callers that place a thing on it.
fn terrain() -> impl Fn(f64, f64) -> f64 {
    let valley = valley();
    let massif = massif();
    let level = yard_level() - APRON_DROP;
    move |x, z| ground_at(&valley, &massif, level, x, z).0
}

/// The ground as a heightfield whose points carry **the cut that reached them**.
///
/// # Invariants
///
/// This is [`heightfield`]'s own walk — the same grid, the same central difference over
/// one cell for the normal, the same winding — with one thing added: the cut goes out on
/// each point's texture coordinate, because the colour of a cut face is a fact about where
/// the cut happened, and no fragment can work that out from where it stands. A shader that
/// painted the terraces from the world height under the pixel painted a **contour of the
/// whole landscape** instead, which is a ring of pale earth following a level across a
/// hillside nothing ever cut.
///
/// The mask is the first channel and the second is left at zero, so the row is a value and
/// a gap rather than a pair of numbers that mean something. Only the ground's own lighting
/// shader reads it, and the ground is the only mesh drawn by that shader, so nothing else
/// in the frame can be holding a number that means a cut.
fn cut_heightfield(
    divisions: usize,
    extent: f64,
    ground: impl Fn(f64, f64) -> (f64, f64),
) -> PackedList {
    let step = 2.0 * extent / divisions as f64;
    let origin = -extent;
    let mut points = Shaded::with_capacity(divisions * divisions * 6);
    let at = |column: usize, row: usize| {
        let (x, z) = (origin + column as f64 * step, origin + row as f64 * step);
        let (dx, dz) = (step * 0.5, step * 0.5);
        let slope_x = (ground(x + dx, z).0 - ground(x - dx, z).0) / (2.0 * dx);
        let slope_z = (ground(x, z + dz).0 - ground(x, z - dz).0) / (2.0 * dz);
        let (height, cut) = ground(x, z);
        (
            [x, height, z],
            normalized([-slope_x, 1.0, -slope_z]),
            [cut, 0.0],
        )
    };
    for row in 0..divisions {
        for column in 0..divisions {
            let (p00, n00, t00) = at(column, row);
            let (p10, n10, t10) = at(column + 1, row);
            let (p01, n01, t01) = at(column, row + 1);
            let (p11, n11, t11) = at(column + 1, row + 1);
            points.push(p00, n00, t00);
            points.push(p10, n10, t10);
            points.push(p01, n01, t01);
            points.push(p10, n10, t10);
            points.push(p11, n11, t11);
            points.push(p01, n01, t01);
        }
    }
    points.finish()
}

/// The height of the far range, which is its own mesh placed beyond the valley's.
///
/// The far range as a mass of its own: the same [`Ridge`] the valley's own range is, and not
/// a closed form along the line.
///
/// # Invariants
///
/// **This is a body and the closed form is for a line.** The range is five hundred and
/// thirty-six units across and sixty-odd tall, seen from two hundred and fifty, and every
/// part of it the eye can resolve is a **surface** with a shape: a face with a light side and
/// a dark one, a gully between two spurs. [`land::crest_profile`] is two smooth arches
/// multiplied
/// together, and a product of two arches has no crease in it anywhere, so over five hundred
/// units of it the range came out as two soft swells and a long even slope between them,
/// which reads as a painted backdrop rather than as rock. [`Ridge`] answers the same question
/// with a fractal along the axis and creasing across it, and the creasing is the half that
/// does the work here: it is what puts a saddle between two crests instead of one more
/// swell.
///
/// That both forms are in the module is not redundancy. `land::crest_profile`'s own
/// documentation says a mass wants the fractal, a skyline wants the closed form, and that
/// neither substitutes for the other. This range was on the wrong side of that line, and the
/// comment that put it there gave as its reason that a skyline is what a far range is, which
/// is true of a range seen end-on and not of one spread across half the horizon. It has no
/// caller now, and it keeps its place in the module for the skyline it is the right answer
/// for.
///
/// Four octaves and not five, and the reason is the mesh rather than the taste. The far
/// range's own grid is a hundred and fifty divisions over six hundred units, so its cell is
/// four units, and a feature has to be worth about **twice** that to be geometry at all: at
/// five octaves the finest wave along the axis is five and a half units, which is under one
/// and a half cells, and a ridge sampled at one and a half samples per cycle is not a ridge,
/// it is a moiré. Four octaves puts the finest at ten and a bit units, two and a half cells,
/// and the range is drawn by the grid it is actually given.
///
/// The line dies at both ends of its own mesh, before the mesh stops, which is the same
/// rule the terrain's own high ground is held to and for the same reason: a range whose
/// height runs out at the edge of the mesh that draws it is a cliff of cut earth on the
/// skyline.
///
/// The rise is what the range is **for**, and it is low on purpose. Every mass between here
/// and it is now tapered to nothing before the terrain's own mesh ends, so this is the only
/// thing standing on the horizon, and a range of a hundred and twenty units would not be a
/// far range: from the yard it would be a wall across the whole frame, and the only thing
/// it would hide is the sky.
///
/// The height is the one number here that was not chosen by reading the formula, because the
/// two forms do not have the same distribution and a ratio of one to the other means nothing.
/// Measured over the range's own hundred and fifty by a hundred and fifty of ground — the
/// part [`land::edge_taper`] leaves at full strength, and not the whole six hundred units of its
/// mesh — the closed form came out at a **peak of 61.2 and a mean of 14.9**, and the first
/// creased mass at a peak of 42.1 and a mean of 13.3: the peak fell by nearly a third and the
/// range all but disappeared behind the near hill, which is a regression dressed as an
/// improvement. [`crest_profile`] never returns below 0.45, so it was a solid body all the
/// way along, and a product of a fractal and a crease drops into its gullies instead. So
/// **two** constants move and not one: the height carries the peak back, to 59.7, and the
/// crease floor carries the mass back up out of the gullies, from 0.36 to 0.45, which is the
/// closed form's own floor arrived at from the other direction. The mean then lands at 20.1,
/// a third **above** what the closed form held, and that is the price of the creases rather
/// than a mistake in the setting: a mass with gullies in it is fuller between its crests than
/// a pair of smooth swells at the same height, and a far range that is only there to be
/// distant wants to be full. All of these are measured, not derived, and the closed form's
/// numbers are recorded here because the next person to retune this will otherwise repeat the
/// measurement rather than read it.
///
/// At this height it reads as what it is — a body of hills
/// lying in the afternoon's haze, close enough to the eye to have a shape and far enough
/// away that nothing in it has an edge.
fn far_body() -> Ridge {
    Ridge {
        axis: [1.0, 0.0],
        // k3's own mountain measured its profile along the axis at 0.012, and that is the
        // number that made the ridgelines there read as ridgelines. Held rather than
        // invented, because this is the same question asked of a wider field.
        along_scale: 0.0120,
        across_scale: [0.0160, 0.0110],
        // Both of the two this scene could not have got from the formula, and both are argued
        // above: `height` restores the peak the creases had taken away and `crease_floor`
        // keeps the gullies from swallowing the body between the crests.
        height: 100.0,
        floor: 6.0,
        crease_floor: 0.45,
        profile_seed: 503,
        crease_seed: 509,
        octaves: 4,
    }
}

/// The far range as the height of a mesh, with nothing cut into it: the mass, thinned to
/// nothing towards its own front and back so that it is a range and not a slab, and dying
/// before its mesh does.
///
/// # Invariants
///
/// The thinning is the composition and the mass is the subject, and they are separate on
/// purpose. The lens across the line is what decides the range is a **wall of hills and not
/// a ribbon standing on its edge**, and it has to keep deciding that whatever the mass inside
/// it is made of, so it multiplies the [`Ridge`] whole rather than reaching into it. The
/// hollow in front of the line is the same move one step on: a range that comes up level with
/// the ground in front of it has nothing to be far about, and a few units of dip is what says
/// there is distance here.
fn far_range() -> impl Fn(f64, f64) -> (f64, f64) {
    let body = far_body();
    move |x: f64, z: f64| {
        // The mass across the line, so the range is a wall of hills and not a ribbon
        // standing on its edge.
        let across = (1.0 - (z / 150.0).powi(2)).max(0.0);
        let rise = body.height_at([x, z]) * across.powf(1.35) - 10.0 * (1.0 - across);
        let height = rise * land::edge_taper([x, z], 150.0, 268.0);
        (height, 0.0)
    }
}

// ---------------------------------------------------------------------------
// The built forms
// ---------------------------------------------------------------------------

/// The corners of a chamfered rectangle, at one height, walked so that
/// [`built::edge_normal`] sends every run's outer face away from the middle.
///
/// # Invariants
///
/// The walk is the one the edge normal is defined against: the first run turns from the
/// east towards the south, so its normal points south-east, out of the corner rather than
/// into it. A ring walked the other way round is a wall with every face inside it, which
/// the rasterizer culls and nothing in the frame says why.
///
/// The corners are chamfered rather than square because a pair of runs meeting at a square
/// corner leaves two coincident end faces, and the rasterizer's choice between them is a
/// bright vertical line up the corner. The chamfer is a third run, and the pair of ends
/// close it.
fn ring_corners(centre: [f64; 2], half: [f64; 2], chamfer: f64, height: f64) -> Vec<[f64; 3]> {
    let (cx, cz) = (centre[0], centre[1]);
    let (hx, hz) = (half[0], half[1]);
    let plan = [
        [hx, hz - chamfer],
        [hx - chamfer, hz],
        [-(hx - chamfer), hz],
        [-hx, hz - chamfer],
        [-hx, -(hz - chamfer)],
        [-(hx - chamfer), -hz],
        [hx - chamfer, -hz],
        [hx, -(hz - chamfer)],
    ];
    plan.into_iter()
        .map(|[x, z]| [cx + x, height, cz + z])
        .collect()
}

/// The gate the yard's wall is interrupted by, as the fractions of the run it falls in.
///
/// # Invariants
///
/// A gate is a gap in the run and not a cut through it, so the two runs either side of it
/// are whole runs with their own end faces, and the faces a person standing in the opening
/// sees are the two the wall form already builds. The corner index is the one the walk above
/// puts the long southern run at.
const GATE_RUN: usize = 1;

/// The shed's open front, in the same terms as the gate.
const DOOR_RUN: usize = 1;

/// Appends a whole run of corners as wall, into a run of points that is already open.
///
/// # Invariants
///
/// This is [`built::wall_ring`]'s own walk written to **append** rather than to return, and
/// it is here because the yard's wall, its coping and its two piers are one wall as far as
/// the picture is concerned: they are one unbroken line of masonry, they share one colour,
/// and a scene that builds them as three lists pays a depthing pass and a lighting pass for
/// each. A [`definition::PackedList`] cannot be read back and rejoined, so the only way to
/// put several bands in one mesh is to walk the ring here and append each run.
///
/// Every run it appends is placed exactly as the generator places one: the same five faces,
/// the same outward normal from [`built::edge_normal`], the same two ends. A gap is a pair
/// of fractions on one run, and the wall either side of it is the same run the generator
/// would have written.
fn push_ring(
    points: &mut Shaded,
    corners: &[[f64; 3]],
    low: f64,
    high: f64,
    thickness: f64,
    gap: Option<(usize, f64, f64)>,
) {
    if corners.len() < 2 {
        return;
    }
    for (index, corner) in corners.iter().enumerate() {
        let from = *corner;
        let to = corners[(index + 1) % corners.len()];
        let outward = built::edge_normal(from, to);
        let stretches: Vec<(f64, f64)> = match gap {
            Some((edge, start, end)) if edge == index && end > start => {
                vec![(0.0, start.clamp(0.0, 1.0)), (end.clamp(0.0, 1.0), 1.0)]
            }
            _ => vec![(0.0, 1.0)],
        };
        for (start, stop) in stretches {
            if stop > start {
                built::push_wall_run(
                    points,
                    add(from, scale(subtract(to, from), start)),
                    add(from, scale(subtract(to, from), stop)),
                    outward,
                    low,
                    high,
                    thickness,
                );
            }
        }
    }
}

/// The yard's enclosure as **one** mesh: the wall, the coping laid over it, and the two piers
/// either side of the gate.
///
/// # Invariants
///
/// The coping is the same ring and the same gap as the wall and nearly twice the width, so
/// the overhang is the shadow line along the top of the wall and the gate is one opening
/// through both bands rather than two gaps at two heights.
///
/// The piers are rings of their own, as small chamfered squares whose inner face stands
/// inside the wall's own end, so the wall's run stops against a solid and neither pier's end
/// face is anywhere near a bright line. A pier is as wide as the coping above it, because a
/// gate pier the width of the wall it interrupts is a pair of ends and not a pier.
fn enclosure() -> PackedList {
    let level = yard_level();
    let corners = ring_corners(YARD, RING_HALF, RING_CHAMFER, level);
    let mut points = Shaded::with_capacity(40 * WALL_FACES * 6);
    push_ring(
        &mut points,
        &corners,
        level - 0.6,
        level + WALL_RISE,
        WALL_THICKNESS,
        Some((GATE_RUN, GATE[0], GATE[1])),
    );
    push_ring(
        &mut points,
        &corners,
        level + WALL_RISE,
        level + WALL_RISE + COPING_RISE,
        COPING_THICKNESS,
        Some((GATE_RUN, GATE[0], GATE[1])),
    );
    let half = COPING_THICKNESS * 0.5;
    for x in [GATE_HALF[0], -GATE_HALF[0]] {
        push_ring(
            &mut points,
            &ring_corners([x, YARD[1] + RING_HALF[1]], [half, half], 0.07, level),
            level - 0.6,
            level + WALL_RISE + 0.62,
            COPING_THICKNESS,
            None,
        );
    }
    points.finish()
}

/// The shed's roof edge, as **one** mesh: the band of wall that caps the eave, and the
/// soffit that closes the roof's underside.
///
/// # Invariants
///
/// A heightfield takes every normal from its own height function, so the tent of
/// [`built::roof_ridge`] has no face turned down at all: a camera below the eave line looks
/// straight through the tiles and out at the inside of the far slope, and the eave itself has
/// nothing to cap it. Two faces are needed and only one of them is a wall — the cap around
/// the edge is a band of masonry, and the ceiling under it is a surface that points down,
/// which is the one thing a heightfield cannot be.
///
/// The soffit is tucked a couple of hundredths of a unit **into** the shed's own wall rather
/// than stopping flush with its inner face. Two surfaces that meet exactly on a plane leave a
/// crack the width of a float, and a ceiling with a crack round it is a ceiling with a line
/// of daylight drawn round the top of every wall.
fn roof_edge() -> PackedList {
    let level = yard_level();
    let half = SHED_HALF[0] - SHED_THICKNESS * 0.5 + 0.02;
    let mut points = Shaded::with_capacity((WALL_FACES + 2) * 6);
    let corners = ring_corners(SHED, [EAVE_HALF, EAVE_HALF], 0.06, level);
    for index in 0..corners.len() {
        let from = corners[index];
        let to = corners[(index + 1) % corners.len()];
        built::push_wall_run(
            &mut points,
            from,
            to,
            built::edge_normal(from, to),
            level + EAVE_BAND[0],
            level + EAVE_BAND[1],
            EAVE_THICKNESS,
        );
    }
    // The ceiling: the same two triangles as a downward facing plane, wound so the surface
    // faces the floor rather than the tiles above it.
    let (a, b, c, d) = (
        [SHED[0] - half, level + EAVE_BAND[0], SHED[1] - half],
        [SHED[0] + half, level + EAVE_BAND[0], SHED[1] - half],
        [SHED[0] - half, level + EAVE_BAND[0], SHED[1] + half],
        [SHED[0] + half, level + EAVE_BAND[0], SHED[1] + half],
    );
    for corner in [a, c, b, b, c, d] {
        points.push(corner, [0.0, -1.0, 0.0], [0.0, 0.0]);
    }
    points.finish()
}

/// The yard's meshes: the ground, the paving, the enclosure, the lintel, the shed, its roof
/// and its closed edge, the field wall, the pole, and the three turned vessels.
fn meshes() -> Vec<Value> {
    let level = yard_level();
    let point = shaded_point_schema();
    let shed_corners = ring_corners(SHED, SHED_HALF, SHED_CHAMFER, level);
    let valley = valley();
    let massif = massif();
    let ground_level = level - APRON_DROP;

    // The beam over the gate. A sweep rather than a wall run because a lintel is a
    // horizontal member and every face of a wall run stands up whatever the ground does
    // under it.
    let lintel = built::sweep([-1.62, 0.0, 0.0], [1.62, 0.0, 0.0], 0.30, 0.30, 8);

    // A free-standing run on the slope, with its outer face decided by the edge normal of
    // the pair of points it stands on rather than by which way it happens to run. The band
    // is level whatever the ground does under it: the ends decide where it stands and the
    // band decides how tall it is.
    let field_ground = 0.5
        * (terrain()(FIELD_RUN[0][0], FIELD_RUN[0][2])
            + terrain()(FIELD_RUN[1][0], FIELD_RUN[1][2]));
    let field_wall = built::wall_run(
        FIELD_RUN[0],
        FIELD_RUN[1],
        built::edge_normal(FIELD_RUN[0], FIELD_RUN[1]),
        field_ground - 0.4,
        field_ground + 1.45,
        0.50,
    );

    vec![
        mesh(
            "ground",
            "Triangles",
            point.clone(),
            cut_heightfield(DIVISIONS, EXTENT, move |x, z| {
                ground_at(&valley, &massif, ground_level, x, z)
            }),
        ),
        // The far range is ground as well, and it is drawn by the ground's own shader for
        // the same reason: its colour is the colour of a hillside, and nothing was terraced
        // on it, so the cut it carries is nothing and the shader paints it as open land.
        mesh(
            "far-range",
            "Triangles",
            point.clone(),
            cut_heightfield(150, FAR_EXTENT, far_range()),
        ),
        // The yard's own surface. A plane rather than a patch of the heightfield, because
        // the ground under it is already level and a second copy of it as geometry would be
        // two surfaces z-fighting over the whole yard.
        mesh(
            "paving",
            "Triangles",
            point.clone(),
            plane_facing(1.0, 0.0, true),
        ),
        mesh("wall", "Triangles", point.clone(), enclosure()),
        mesh("lintel", "Triangles", point.clone(), lintel),
        mesh(
            "shed",
            "Triangles",
            point.clone(),
            built::wall_ring(
                &shed_corners,
                level - 0.4,
                level + SHED_RISE,
                SHED_THICKNESS,
                &[WallGap {
                    edge: DOOR_RUN,
                    from: SHED_DOOR[0],
                    to: SHED_DOOR[1],
                }],
            ),
        ),
        // The roof, fed straight to a heightfield, so the crest, the eave's turn and the
        // settle along the ridge are the generator's own arithmetic. The band and the
        // soffit under it are one mesh of their own, because they are one piece of the same
        // edge and neither of them is anything a heightfield can be.
        mesh(
            "roof",
            "Triangles",
            point.clone(),
            heightfield(48, ROOF_RUN, built::roof_ridge(ROOF_RUN, ROOF_RISE)),
        ),
        mesh("roof-edge", "Triangles", point.clone(), roof_edge()),
        mesh("field-wall", "Triangles", point.clone(), field_wall),
        // A pole, drawn once and placed several times: a colonnade, a stand and a leaning
        // prop are the same member at three different angles.
        mesh(
            "pole",
            "Triangles",
            point.clone(),
            built::sweep([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], 0.055, 0.036, 10),
        ),
        mesh("jar", "Triangles", point.clone(), lathe(&JAR, 24)),
        mesh("basket", "Triangles", point.clone(), lathe(&BASKET, 20)),
        mesh("mill", "Triangles", point.clone(), lathe(&MILL, 26)),
    ]
}

// ---------------------------------------------------------------------------
// Where everything stands
// ---------------------------------------------------------------------------

/// One placement of one mesh: where it is, how it is turned, how far it is scaled, and
/// whether it goes into the shadow map at all.
struct Place {
    at: [f64; 3],
    turn: [f64; 3],
    size: [f64; 3],
    casts: bool,
}

impl Place {
    fn at(at: [f64; 3]) -> Self {
        Self {
            at,
            turn: [0.0; 3],
            size: [1.0; 3],
            casts: true,
        }
    }

    fn turned(mut self, turn: [f64; 3]) -> Self {
        self.turn = turn;
        self
    }

    fn sized(mut self, size: [f64; 3]) -> Self {
        self.size = size;
        self
    }

    /// Takes this placement out of the shadow map.
    fn unshadowed(mut self) -> Self {
        self.casts = false;
        self
    }
}

/// One group of geometry: a mesh, every placement of it, and the row the lighting stage
/// draws it with.
struct Group {
    mesh: &'static str,
    tint: [f64; 3],
    roughness: f64,
    reflect: f64,
    /// The lighting shader this group is drawn by, which is the scene's own for anything
    /// whose shading is a rule about one kind of surface rather than about all of them.
    shader: &'static str,
    places: Vec<Place>,
}

impl Group {
    fn new(mesh: &'static str, tint: [f64; 3], roughness: f64, reflect: f64) -> Self {
        Self {
            mesh,
            tint,
            roughness,
            reflect,
            shader: "lighting",
            places: Vec::new(),
        }
    }

    /// Draws this group by a shader of the scene's own rather than by the one every other
    /// group is drawn by.
    ///
    /// A pass draws one shader, so what differs between two kinds of surface is a shader and
    /// a row and not a pipeline. The depthing and shadowing passes are untouched: this is
    /// only which body the lighting stage runs, so a surface with its own body costs no pass
    /// of its own.
    fn drawn_by(mut self, shader: &'static str) -> Self {
        self.shader = shader;
        self
    }

    fn at(mut self, place: Place) -> Self {
        self.places.push(place);
        self
    }

    fn placing(mut self, places: Vec<Place>) -> Self {
        self.places.extend(places);
        self
    }
}

/// The groups the scene draws, in the order it draws them.
///
/// # Invariants
///
/// The yard is built first, so the frame reads the wall before it reads the land behind
/// it. The vessels and the millstone are not in the shadow map: a jar throws a shadow a
/// camera at twenty units cannot see, and the shadow map is the most expensive pass in the
/// scene per group in it. The far range is out of it for the same reason and a stronger
/// one — it is five hundred units from the sun's shadow extent's nearest edge, so it casts
/// nothing that the picture has room for.
fn groups() -> Vec<Group> {
    let level = yard_level();
    let ground = terrain();
    let paving = level + 0.03;

    let mut groups = vec![
        // The ground, and the far range with it, are drawn by the ground's own shader. The
        // body that shader runs is a rule about what the land is doing — a tread and a riser
        // are different earths, and one of them was cut — and a rule about the land does not
        // belong on a lime-washed wall or a turned jar.
        Group::new("ground", [0.232, 0.238, 0.152], 0.92, 0.0)
            .drawn_by("ground")
            .at(Place::at([0.0; 3])),
        Group::new("wall", [0.460, 0.428, 0.378], 0.95, 0.0).at(Place::at([0.0; 3])),
        // The paving is the one surface in the scene that is **not** a matte one, and it is worth
        // saying why it cannot be. A yard is looked across from about eye height, so the angle
        // between the eye and the paving is very nearly a grazing one, and a grazing angle is
        // where a Fresnel term goes to one whatever the surface is: a smooth horizontal plane at
        // that angle returns almost the whole sky, so the paving comes back the colour of the
        // daylight rather than the colour of the stone, and it comes back the same whatever the
        // sun is doing. Stone paving is not that, and a yard paved in it reads as a sheet of
        // sky lying on the ground. The roughness is high and the reflection is off.
        Group::new("paving", [0.300, 0.290, 0.272], 0.96, 0.0).at(Place::at([
            YARD[0], paving, YARD[1],
        ])
        .sized([
            RING_HALF[0] - WALL_THICKNESS * 0.5,
            1.0,
            RING_HALF[1] - WALL_THICKNESS * 0.5,
        ])),
        // The beam runs past both piers, so its own end faces are inside them and the two
        // boxes never stop on one plane.
        Group::new("lintel", [0.220, 0.150, 0.092], 0.88, 0.0).at(Place::at([
            0.0,
            level + 2.80,
            YARD[1] + RING_HALF[1],
        ])),
        Group::new("shed", [0.280, 0.240, 0.192], 0.94, 0.0).at(Place::at([0.0; 3])),
        Group::new("roof", [0.190, 0.176, 0.164], 0.72, 0.0).at(Place::at([
            SHED[0],
            level + ROOF_ORIGIN,
            SHED[1],
        ])),
        Group::new("roof-edge", [0.170, 0.146, 0.118], 0.90, 0.0).at(Place::at([0.0; 3])),
        Group::new("jar", [0.340, 0.132, 0.078], 0.62, 0.0)
            .at(Place::at([SHED[0] + 4.6, paving, SHED[1] + 3.4]).sized([1.15, 1.15, 1.15]))
            .at(Place::at([SHED[0] - 3.9, paving, SHED[1] + 3.2]).sized([0.86, 0.86, 0.86])),
        Group::new("basket", [0.300, 0.198, 0.090], 0.88, 0.0).at(Place::at([
            3.2,
            paving,
            YARD[1] + 5.8,
        ])
        .sized([1.25, 1.25, 1.25])
        .unshadowed()),
        Group::new("mill", [0.340, 0.330, 0.312], 0.82, 0.0).at(Place::at([
            -5.6,
            paving,
            YARD[1] + 4.6,
        ])
        .sized([1.3, 1.3, 1.3])
        .unshadowed()),
    ];

    // The poles. The colonnade is a **row** and it runs away from the camera that stands
    // inside the yard, along the line between the two places that frame it: the near end
    // sits a body's width to one side of that camera and the far end is beside the shed, so
    // the row crosses the picture from its edge towards its middle instead of standing
    // across it. The leaning pole is the same mesh turned about its own foot, which is why
    // the mesh's origin is at the bottom of the pole and not at its middle.
    let mut poles = vec![
        Place::at([SHED[0] - 5.2, paving, SHED[1] + 0.4])
            .turned([0.0, 0.0, -0.46])
            .sized([1.0, 3.6, 1.0]),
    ];
    for (offset, height) in COLONNADE {
        poles.push(
            Place::at([
                COLONNADE_AT[0] + offset[0],
                paving,
                COLONNADE_AT[1] + offset[1],
            ])
            .sized([1.0, height, 1.0]),
        );
    }
    for at in FIELD_RUN {
        poles.push(Place::at([at[0], ground(at[0], at[2]) - 0.2, at[2]]).sized([0.9, 2.9, 0.9]));
    }
    groups.push(Group::new("pole", [0.245, 0.170, 0.100], 0.90, 0.0).placing(poles));

    // The field wall, and the far range last, because it is the deepest thing in the frame
    // and the frame is read from the front back.
    groups.push(Group::new("field-wall", [0.440, 0.400, 0.340], 0.95, 0.0).at(Place::at([0.0; 3])));
    groups.push(
        Group::new("far-range", [0.300, 0.312, 0.330], 1.0, 0.0)
            .drawn_by("ground")
            .at(Place::at(FAR_AT).unshadowed()),
    );
    groups
}

/// Builds the materials and the instances of every group.
///
/// # Invariants
///
/// One group is one mesh and one material per stage, and the three lists are the same
/// placements under three labels: a material names one shader and a shader belongs to one
/// stage, and a label names one instance in the whole scene rather than one per pass. The
/// shadowing and depthing rows carry no colour at all — a shadow map and a depth buffer
/// are the pipeline's own textures — so every one of them is white.
fn build(groups: &[Group]) -> (Vec<Value>, Value) {
    let mut materials = Vec::new();
    let mut shadowing = Vec::new();
    let mut depthing = Vec::new();
    let mut lighting = Vec::new();

    for group in groups {
        let mut declared: Vec<(&str, String)> = Vec::new();
        for (pass, shader, row) in [
            (
                "shadowing",
                "shadowing",
                surface([1.0; 3], 0.0, 1.0, [0.0; 3]),
            ),
            (
                "depthing",
                "depthing",
                surface([1.0; 3], 0.0, 1.0, [0.0; 3]),
            ),
            (
                "lighting",
                group.shader,
                surface_reflecting(group.tint, 0.0, group.roughness, [0.0; 3], group.reflect),
            ),
        ] {
            let label = format!("{}-{pass}", group.mesh);
            materials.push(mesh_material(&label, shader, row));
            declared.push((pass, label));
        }
        for (index, place) in group.places.iter().enumerate() {
            for (pass, label) in &declared {
                let entry = instance(
                    format!("{}-{pass}-{index}", group.mesh),
                    group.mesh,
                    place.at,
                    place.turn,
                    place.size,
                    label,
                );
                match *pass {
                    // A placement that is out of the shadow map is out of it only: it is
                    // still drawn, and still casts whatever the map happens to hold.
                    "shadowing" if !place.casts => {}
                    "shadowing" => shadowing.push(entry),
                    "depthing" => depthing.push(entry),
                    _ => lighting.push(entry),
                }
            }
        }
    }

    (
        materials,
        instances(vec![
            (Stage::Shadowing, shadowing),
            (Stage::Depthing, depthing),
            (Stage::Lighting, lighting),
        ]),
    )
}

/// The three cameras, each a different decision about where a photographer stands.
///
/// # Invariants
///
/// Every one of them is placed off the axis of whatever it is looking at. A frame taken
/// square on to a wall puts the wall's middle in the middle of the picture and gives the eye
/// nowhere to go; a frame taken square on to a **gate** is worse, because the opening is the
/// one thing in the picture with a symmetry of its own and it is then the frame's symmetry as
/// well. All three stand to one side, all three are looking across something at an angle, and
/// all three have their subject off the centre line.
fn cameras() -> Vec<Value> {
    let ground = terrain();
    let level = yard_level();
    // The height of the ground at a point, for an eye that stands on it.
    let eye = |x: f64, z: f64, height: f64| [x, ground(x, z) + height, z];

    vec![
        // Outside the gate, to the east of it and above the coping, looking back along the
        // wall's southern run. The wall therefore runs **away** from the eye rather than
        // across it, so the coping is a diagonal rising to the near corner, the gate is off
        // the centre line, and the near corner of the ring is the largest thing in the
        // frame. The field wall runs across the bottom of it a few metres in front.
        // Thirty five millimetres, which is the lens the wall wants: wide enough to hold the
        // massif behind it and the sky above that, and narrow enough that the coping line is
        // still one line rather than a sweep.
        //
        // The eye stands a little above the coping and well back from it, because a field
        // wall seven units off the lens is not a foreground detail — at eye height it is a
        // third of the picture, and raising the eye instead only trades it for empty ground,
        // because a wall the eye is above is a wall out of frame. Standing off at sixteen
        // puts the wall's coping in the bottom fifth, running away to the left, and holds
        // the near corner of the ring as the largest thing above it.
        camera_placed_on_lens(
            eye(28.0, 34.0, 3.4),
            [-2.0, level + 2.2, 3.5],
            35.0,
            36.0,
            0.0,
            0.4,
            2400.0,
        ),
        // In the yard, at the open corner, looking across the paving at the shed's open front
        // and the vessels against it. Twenty four millimetres, because a yard is a small room
        // and a lens that leaves the wall out of frame is a lens that has cropped the point.
        // The colonnade runs off the right of the picture and in towards the middle, so it
        // frames the shed rather than standing between the eye and it.
        camera_placed_on_lens(
            [COLONNADE_AT[0], paving_level() + 1.60, COLONNADE_AT[1]],
            [-2.2, level + 1.10, -2.2],
            24.0,
            36.0,
            0.0,
            0.3,
            2400.0,
        ),
        // Across the valley, and low. A long lens has a narrow field, and a narrow field
        // pointed at the ground from high up has no horizon in it at all: the depression to
        // the subject has to be **less** than the lens's own half angle or the sky is above
        // the top of the picture and the frame is a map. Seventy two millimetres could not
        // be asked for that at this distance, so this is fifty eight, and the eye is low
        // enough that the yard lands near the bottom of the picture with the terraced slope
        // behind it, the far range on the horizon and a band of sky above all three.
        camera_placed_on_lens(
            eye(58.0, 74.0, 24.0),
            [-4.3, level + 9.9, -13.4],
            58.0,
            36.0,
            0.0,
            0.6,
            2400.0,
        ),
    ]
}

/// The height of the yard's paving, which is the platform's level and a hand's width of it.
fn paving_level() -> f64 {
    yard_level() + 0.03
}

// ---------------------------------------------------------------------------
// The shaders
// ---------------------------------------------------------------------------

/// The five shaders the scene declares: the gallery's four mesh stages, the ground's own
/// lighting shader beside them, and the post.
///
/// # Invariants
///
/// The ground's body is a rule about **land** — a tread and a riser are different earths,
/// and one of them was cut — and it is declared as a lighting shader of the scene's own
/// rather than as the body every mesh in the scene is shaded by. Every other mesh is drawn
/// by the plain one, so nothing a lime-washed wall or a turned jar is painted by can tell
/// the ground's body that it is standing on a wall.
fn shaders() -> Vec<Value> {
    let mut set = mesh_shaders("", NOISE);
    set.push(displaced_lighting_shader(
        "ground",
        "",
        GROUND_SURFACE,
        NOISE,
    ));
    set.push(post_shader(
        GRADE,
        &(atmosphere().wgsl() + ridges::RIDGES + POST_HELPERS),
    ));
    set
}

/// The air the scene is drawn through, in one place.
///
/// # Invariants
///
/// The sun is named once, as the direction **towards** it, and the light is given the
/// same sun as the way its light travels. The two are the same fact written in the two
/// conventions the framework uses, and a scene that got one of them backwards is a sky
/// with its glow on the wrong side of the frame.
fn atmosphere() -> Atmosphere {
    Atmosphere::new()
        .zenith(ZENITH)
        .horizon(HORIZON)
        .ground(SKY_GROUND)
        .sun(SUN_TOWARD, [0.52, 0.34, 0.17], 110.0, 0.85)
        .disc([1.0, 0.90, 0.74], 0.99995)
        .density(0.0030)
        .scale_height(190.0)
}

/// The three colours the sky is made of, shared by the sky itself and by the environment
/// every shadowed surface is lit by.
///
/// # Invariants
///
/// The map the environment light samples and the sky the post stage draws are one sky, so
/// they are written down once and both are built from these. A scene whose ambient came
/// from a different afternoon than its sky is a frame where a yard floor in the wall's
/// shadow is one colour and the sky above it is another, and nothing in the picture says
/// which of the two the author meant.
const ZENITH: [f64; 3] = [0.105, 0.196, 0.420];
const HORIZON: [f64; 3] = [0.600, 0.588, 0.560];
const SKY_GROUND: [f64; 3] = [0.320, 0.296, 0.262];

/// The environment map, as the latlong image the lighting stage samples.
///
/// # Invariants
///
/// The map is built by **decoding** every texel to the direction it stands for rather than by
/// projecting directions onto it, and the decode is the only one of the two that is exact: a
/// generator that instead painted a gradient onto rows would be making a claim about rows, and
/// the reader inverts those rows into directions.
///
/// The horizon band and the sun's quarter are both added here rather than baked into the three
/// colours above, so the two halves of the sky — what is overhead and what is underfoot — stay
/// the single decision they are written down as.
fn air_pixels(width: usize, height: usize) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(width * height * 4);
    for row in 0..height {
        for column in 0..width {
            let direction = crate::shaders::latlong_direction(
                column as i64,
                row as i64,
                width as i64,
                height as i64,
            );
            // The sky itself: the horizon band between the zenith above and the ground
            // below, and a broad warm glow in the quarter the sun stands in, so that a
            // surface turned towards the sun's side of the sky is warm even where the sun
            // itself does not reach it.
            let sky = if direction[1] >= 0.0 {
                mix3(HORIZON, ZENITH, smoothstep(0.0, 0.55, direction[1]))
            } else {
                mix3(HORIZON, SKY_GROUND, smoothstep(0.0, 0.30, -direction[1]))
            };
            let towards = (direction[0] * SUN_TOWARD[0]
                + direction[1] * SUN_TOWARD[1]
                + direction[2] * SUN_TOWARD[2])
                .max(0.0)
                .powf(6.0);
            for channel in 0..3 {
                let lit = sky[channel] + towards * [0.46, 0.30, 0.15][channel];
                pixels.push((lit.min(1.0).max(0.0) * 255.0).round() as u8);
            }
            pixels.push(255);
        }
    }
    pixels
}

/// Returns one colour's three channels mixed from one colour to another.
fn mix3(from: [f64; 3], to: [f64; 3], amount: f64) -> [f64; 3] {
    [
        from[0] + (to[0] - from[0]) * amount,
        from[1] + (to[1] - from[1]) * amount,
        from[2] + (to[2] - from[2]) * amount,
    ]
}

/// The noise the ground's own surface is read from, as WGSL.
const NOISE: &str = r#"
/// Returns a value in zero to one from a lattice point.
fn ground_hash(point: vec2<f32>) -> f32 {
    return fract(sin(dot(point, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

/// Returns a smooth value in zero to one from a continuous coordinate.
fn ground_noise(point: vec2<f32>) -> f32 {
    let cell = floor(point);
    let within = fract(point);
    let blend = within * within * (3.0 - 2.0 * within);
    let near = mix(ground_hash(cell), ground_hash(cell + vec2<f32>(1.0, 0.0)), blend.x);
    let far = mix(ground_hash(cell + vec2<f32>(0.0, 1.0)), ground_hash(cell + vec2<f32>(1.0, 1.0)), blend.x);
    return mix(near, far, blend.y);
}
"#;

/// What the ground's own lighting shader does to the land after the light is applied.
///
/// # Invariants
///
/// The land is coloured by **what was done to it**, and the two things that were done are the
/// slope it was left at and the cut that reached it. A riser and the tread above it are
/// therefore different earths rather than one surface with a stripe on it, and a face too
/// steep to terrace is bare ground whatever it is next to.
///
/// The cut arrives on the **point**, in the first channel of the texture coordinate, and
/// that is the whole of the argument for it being there rather than worked out here. The
/// cut is gated on a slope and on a band of the land's own height, and a fragment given the
/// world height under the pixel can see the band and cannot see the slope. What it can see
/// instead is a contour: one colour for every hillside that happened to be at the right
/// altitude, which draws a ring of pale earth round the whole landscape following a level
/// across ground nothing was ever cut on. The number the rust side measured is the only one
/// that is about the same surface as the geometry.
const GROUND_SURFACE: &str = r#"
    let here = in.world_position;
    let up_flat = smoothstep(0.86, 0.97, normal.y);
    let steep = 1.0 - up_flat;
    let cut = in.texture.x;

    // The ground's own colour, written as a multiplier **about one** rather than as a colour
    // in its own right, so that the land material's tint is the albedo and this is only how
    // that albedo varies with what the land is doing.
    var earth = mix(vec3<f32>(0.94, 0.90, 0.86), vec3<f32>(1.08, 1.02, 0.80), up_flat);

    // A cut face is paler and drier than the grass above it, and a cut riser is paler still
    // than the tread it joins, because the riser is the face the tools were on.
    earth = mix(earth, vec3<f32>(1.22, 1.16, 1.00), cut * up_flat * 0.85);
    earth = mix(earth, vec3<f32>(1.14, 1.08, 0.97), cut * steep * 0.55);

    // Enough noise that a hillside three hundred units across is not one flat colour.
    earth = earth * (0.88 + 0.24 * ground_noise(here.xz * 0.09));
    color = color * in.tint * earth;
"#;

/// The post stage's own helpers: the world position a pixel shows, recovered from the depth
/// the prepass wrote.
const POST_HELPERS: &str = r#"
/// Returns the world position a pixel shows, given its depth buffer value.
///
/// # Invariants
///
/// The recovery is written against rows two and three of the whole view-projection, because
/// the pass carries the composition with the view and reading a bare perspective's own
/// coefficients out of it reads every distance as the far plane.
fn world_position(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ray = ray_through(uv);
    let eye = uniforms.camera.position;
    let matrix = uniforms.projection.view_projection;
    let row_z = vec4<f32>(matrix[0][2], matrix[1][2], matrix[2][2], matrix[3][2]);
    let row_w = vec4<f32>(matrix[0][3], matrix[1][3], matrix[2][3], matrix[3][3]);
    let at_eye_z = dot(row_z.xyz, eye) + row_z.w;
    let at_eye_w = dot(row_w.xyz, eye) + row_w.w;
    let per_depth = dot(row_z.xyz - depth * row_w.xyz, ray);
    let t = (depth * at_eye_w - at_eye_z) / per_depth;
    return eye + ray * min(t, 2400.0);
}

/// Returns a dither of about one 8-bit step, seeded on the pixel.
///
/// # Invariants
///
/// The picture is written to eight bits a channel, and a smooth gradient that crosses the
/// space between two of those steps does not fade into the next one — it sits on it, and
/// where the gradient is shallow enough that a whole hillside falls inside a step or two,
/// the steps are the picture. A hillside is exactly that surface: its shading changes
/// slowly and always in the direction of a contour, so the error the steps draw is drawn
/// in contour rings, and it reads as marbling in the land rather than as a fault in the
/// writing out. Nothing about the land is wrong; the frame is eight bits wide and the
/// land is not.
///
/// The offset is the **difference of two uniforms**, which is a triangular distribution
/// rather than a uniform one, and that is the whole of the reason to spend it: a uniform
/// dither of one step still leaves banding at half contrast, because it can only move a
/// pixel to either neighbour, whereas differencing two moves it to a level in between and
/// averages back to the true value. One step is the whole amplitude, and one is enough —
/// the error being removed is the quantisation itself.
fn dither(pixel: vec2<f32>) -> f32 {
    let first = hash(pixel);
    let second = hash(pixel + vec2<f32>(37.0, 11.0));
    return (first - second) * (1.0 / 255.0);
}

/// Returns a number in the unit interval that has no relation to its neighbour's.
fn hash(pixel: vec2<f32>) -> f32 {
    var seed = fract(vec3<f32>(pixel.x, pixel.y, pixel.x) * 0.1031);
    seed += vec3<f32>(dot(seed, vec3<f32>(seed.y, seed.z, seed.x) + 33.33));
    return fract((seed.x + seed.y) * seed.z);
}
"#;

/// What the post stage does to the frame.
///
/// # Invariants
///
/// The sky is drawn only where the depth buffer is still at its clear value, because a
/// depth of one is the set of pixels no geometry reached. The air is then closed over
/// whatever the pixel shows **and not over the sky**, which has already been drawn as the
/// air's own colour: hazing it a second time would flatten the gradient the sky exists to
/// have.
///
/// The one number this scene passes the painted ranges is [`RIDGE_SCALE`], and it is a fact
/// about the cameras rather than about the ranges: two of the three look **down** on the yard
/// from a height, which puts the whole horizon high in the picture, and layers scaled for a
/// camera looking level come out as a band of colour across the upper sky rather than as a
/// skyline standing on it.
const GRADE: &str = r#"
    /// How tall the painted ranges stand, as a fraction of the height the shared ridges draw
    /// them at for a camera looking level.
    ///
    /// A yard is a **horizontal** subject and all three of this scene's cameras were placed to
    /// read it as one: two stand above it and look down, and the third is on the far side of
    /// the valley looking down the fall. A camera looking down puts the horizon high in the
    /// picture, and a range scaled for a level camera therefore crests **above the modelled
    /// far range's own silhouette** — the relationship backwards, and a painted range
    /// standing taller than the real one behind it reads as a cloud bank rather than as
    /// distance. Half puts the topmost layer just under the far range's own crest, which is
    /// what leaves the painted ones showing only where the modelled silhouette dips: the
    /// skyline gains two more ranges behind it in the places the real one has a saddle.
    const RIDGE_SCALE: f32 = 0.50;

    /// How many painted ranges stand behind the modelled one, which is three: the yard is closed
    /// on three sides and the ranges that show are the ones showing through the gaps in it, and a
    /// fourth would only ever be a fourth step of ink behind two others.
    const RIDGE_LAYERS: i32 = 3;

    let ray = ray_through(in.uv);
    if depth >= 1.0 {
        // The sky, and behind it the ranges the air has already closed over.
        //
        // The modelled far range is a hundred and seventy units off and comes out within a
        // few levels of the sky it stands against — the closure there is over sixty percent
        // and its own material is a near-neutral grey chosen to match the haze, so the two
        // together leave a silhouette no wider than the quantisation. **So it is not asked
        // to carry the horizon.** `ridges` paints three layers into the sky itself, which is
        // drawn *after* the air, and where the modelled range's own silhouette dips they come
        // through above it: three ridges receding, each a step deeper than the one behind,
        // which is what a far range looks like and what one flat cut-out never does.
        color = ridges(air_sky(ray), ray, RIDGE_SCALE, RIDGE_LAYERS);
    } else {
        let world = world_position(in.uv, depth);
        color = mix(color, air_colour(), air_closure(uniforms.camera.position, world));
    }

    // The grade. An ACES fit rather than a clamp, so a lit face rolls off to white instead
    // of hitting a wall; gamma after, because the target is linear.
    //
    // The exposure is where this frame is exposed. The sun is a point light standing four
    // hundred units off, so its own falloff takes the radiance at the yard down to about a
    // fifth of what it is at the lamp, and a wall in full sun is then brighter than the
    // white the shaded image can hold. A photograph of a scene that bright is exposed down.
    color = color * 0.42;
    color = clamp(
        (color * (2.51 * color + vec3<f32>(0.03)))
            / (color * (2.43 * color + vec3<f32>(0.59)) + vec3<f32>(0.14)),
        vec3<f32>(0.0),
        vec3<f32>(1.0),
    );
    color = pow(color, vec3<f32>(0.4545));
    // A gentle S about the middle, to separate a lit face from a shadowed one.
    color = color * color * (3.0 - 2.0 * color) * 0.25 + color * 0.75;
    // The vignette, light enough to be a lens rather than a frame.
    let centred = in.uv - vec2<f32>(0.5, 0.5);
    color = color * clamp(1.0 - dot(centred, centred) * 0.24, 0.0, 1.0);

    // Last, so that nothing after it scales the offset down in the corners and leaves the
    // vignette banding on its own: this is the last arithmetic the frame does before it is
    // written out at eight bits, which is where the steps it removes are introduced.
    color = color + vec3<f32>(dither(in.position.xy));
"#;
