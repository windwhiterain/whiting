//! The three belts [`crate::southwest_site`] does not stand in: the dry-hot valley, the subalpine
//! stand, and the whole slope in one frame.
//!
//! # What these three frames are
//!
//! [`crate::southwest_site`] drew the montane slope at two thousand metres and left the other three
//! belts of the same mountain unoccupied. These are those belts. **All three stand on
//! [`slope_height`], the law that module owns**, so the valley floor a kapok stands on is the same
//! heightfield the montane frame's tufts were seated on and the shoulder a fir stands on is the same
//! swell the montane camera looked across — one mountain, four heights, and E1's check is still a
//! comparison of a sidecar against a species' note.
//!
//! Each frame carries what a site frame carries and nothing a specimen frame carries: companions at a
//! coarser [`Detail`] and the stand foliage, a known-size object of stated dimensions, and a light that
//! is the belt's own.
//!
//! # The transect's compression, stated
//!
//! [`site_transect`] is a diagram that is also a picture, and the honest way to read it is as a section
//! drawn in perspective. **Two thousand four hundred metres of altitude are drawn across 43.1 m of
//! ground** — [`slope_height`] falls from **18.37 m** at the frame's highest planting (the fir
//! companion on the shoulder, `x = -72, z = 18`) to **-24.77 m** at its lowest (the fern in the valley
//! gully, `x = 68, z = 10`) — so one metre of ground carries **fifty-six metres** of altitude and the
//! frame's compression is **about fifty-six to one**. Nothing in the picture is to scale vertically
//! between the belts, and a reader who measures the ground and concludes that a fir grows a hundred
//! metres above a kapok is reading the picture's own rule rather than its error.
//!
//! **The second compression is the trees'.** Each species is drawn at the height its own modelling
//! table states — the kapok at twenty metres, the pine at sixteen, the spruce at thirty-four — because
//! the silhouettes are what this frame is finally judged on, and a silhouette drawn at its belt's
//! scale rather than its own is not a silhouette. So a belt reads by *what grows in it* and by the
//! tree's own height, and the ground's fifty-six-to-one compression is what keeps the four belts in
//! one frame at all.
//!
//! # What the two shared files this module appends to are
//!
//! `lib.rs` gains three build arms and `locate.rs` three [`document::naming::Naming`] rows, which is the
//! whole of the registration: the scene's own resolver names a planting and a node, and the row's
//! `note` is the one sentence an asker puts after it. **The transect's compression is stated in that
//! note as well as above**, because the row is the one piece of prose about a frame that reaches a
//! reader who never opens this file.

use definition::Value;
use vegetation::decorator::{Seat, scatter, scatter_in};
use vegetation::derive::{Species, tuft};
use vegetation::expand::Detail;
use vegetation::ground::seat;
use vegetation::handle::Handle;
use vegetation::plant::Placement;
use vegetation::rng::Rng;

use crate::southwest::{Foliage as PineFoliage, pine_palette, yunnan_pine};
use crate::southwest_flag::{dove_palette, dove_tree, giant_rhododendron, rhodo_palette};
use crate::southwest_oak::{alpine_oak, oak_palette};
use crate::southwest_site::slope_height;
use crate::southwest_spire::{
    Foliage as SpireFoliage, cangshan_fir, fir_palette, lijian_spruce, spruce_palette,
};
use crate::southwest_valley::{Foliage as FernFoliage, Season, kapok, tree_fern};
use crate::vegetation_picture::{
    Framing, Ground, Lighting, Palette, Planting, Prop, Skin, plant_scene_lit, planting_handle,
};
use document::air::Atmosphere;

// ---------------------------------------------------------------------------
// The three frames' names
// ---------------------------------------------------------------------------

/// The name the dry-hot valley site frame is built and rendered under.
pub const SITE_VALLEY: &str = "site_valley";

/// The name the subalpine site frame is built and rendered under.
pub const SITE_SUBALPINE: &str = "site_subalpine";

/// The name the whole-slope transect is built and rendered under.
pub const SITE_TRANSECT: &str = "site_transect";

// ---------------------------------------------------------------------------
// What the three frames share
// ---------------------------------------------------------------------------

/// The slope at `f32`, for seating: [`slope_height`] read at the precision a plant's base is stored
/// in, so a planting cannot stand where the floor the frame draws is not.
fn slope(at: [f32; 2]) -> f32 {
    slope_height(f64::from(at[0]), f64::from(at[1])) as f32
}

/// The detail a companion is drawn at: the hero's dial is the crate's default and a companion is the
/// same rule coarser, which is what [`Planting::with_detail`] is for.
const COMPANION: Detail = Detail {
    stations: 4,
    around: 8,
    spacing: 1.5,
};

/// The detail a tree in the transect's far belts is drawn at. The transect's smallest subject is a
/// fern a hundred and eighty metres off, and the transect carries fifteen plants, so the dial that
/// keeps the frame inside its row budget is one step coarser again than a stand's.
const DISTANT: Detail = Detail {
    stations: 3,
    around: 6,
    spacing: 3.0,
};

/// A planting of one species at a plan position, seated on the slope and turned by its own turn.
fn seated(species: Species, palette: Palette, at: [f32; 2], turn: f32, detail: Detail) -> Planting {
    Planting::new(species)
        .at(seat(
            Placement {
                origin: [at[0], 0.0, at[1]],
                x: [turn.cos(), 0.0, -turn.sin()],
                y: [0.0, 1.0, 0.0],
                z: [turn.sin(), 0.0, turn.cos()],
            },
            slope,
        ))
        .with_palette(palette)
        .with_detail(detail)
}

/// A ground-cover tuft standing at a plan position in a skin of the frame's own.
///
/// # Invariants
///
/// - **A tuft is in the foreground band and nowhere else**, because a tuft twenty metres from the lens
///   is two pixels and one at sixty is nothing; the band each frame gives is the statement of how near
///   is near enough, and it is written at each frame rather than here because the three bands are three
///   different rectangles.
/// - **A tuft costs no instance rows at all.** Seven nodes with no leaf field is one baked mesh and
///   zero placements, so a foreground of nine hundred of them is nine hundred draws of free geometry:
///   the valley's seven hundred and the transect's nine hundred add nothing to the twenty thousand rows
///   the transect's pines carry, and cover is therefore a question of mesh points and not of the
///   budget that decides how many companions a frame can hold.
fn tufted(seed: u64, at: [f32; 2], tint: [f64; 3]) -> Planting {
    Planting::new(tuft(seed))
        .at(seat(
            Placement {
                origin: [at[0], 0.0, at[1]],
                x: [1.0, 0.0, 0.0],
                y: [0.0, 1.0, 0.0],
                z: [0.0, 0.0, 1.0],
            },
            slope,
        ))
        .with_palette(Palette {
            wood: Skin::new(tint, 0.97),
            leaves: Vec::new(),
            seats: Vec::new(),
        })
}

/// Packs a sweep as a prop: world geometry on the ground's own point schema, which carries no
/// `authored_id` because a stone, a log or a boulder is not an authored element of any plant.
///
/// # Invariants
///
/// **A prop is packed with the ground's schema and never with the authored layout**, because a prop
/// draws on the ground's three stages and the authored schema has a field the ground's does not; a
/// mismatch is a compile error rather than a silently dropped id, and the reserved zero is what a
/// prop's pixels are.
fn packed_prop(
    name: &str,
    sweep: shape::Sweep,
    stations: usize,
    around: usize,
    tint: [f64; 3],
    roughness: f64,
    at: [f64; 3],
) -> Prop {
    let loft = vegetation::decorator::Template {
        sweep,
        stations,
        around,
    }
    .loft();
    let schema = document::building_identity::not_authored_extras().point_schema();
    let layout = graphics::layout::packed_layout(&schema)
        .expect("the ground's own point schema is a struct of components");
    let corners = document::geometry::loft_corners(&loft);
    let mut points = definition::PackedBuilder::new(layout);
    points.reserve(corners.len());
    for corner in &corners {
        let written = points
            .field("normal", &corner.normal)
            .and_then(|points| points.field("position", &corner.position))
            .and_then(|points| points.field("texture", &corner.texture));
        written.expect("a generator writes whole elements of its own layout");
    }
    Prop {
        name: name.to_owned(),
        points: points
            .finish()
            .expect("every generator writes whole elements of its own layout"),
        tint,
        roughness,
        at,
    }
}

// ---------------------------------------------------------------------------
// The seat rules, restated from the species modules
// ---------------------------------------------------------------------------

/// The kapok's flower template index. [`southwest_valley::Season`] keeps its own constant private and
/// the flowers are its template nought, which is the number its flowering plate writes.
const KAPOK_FLOWER: usize = 0;

/// The kapok's flowers: **three to six on the outer fifth of every twig and nought to two on the outer
/// fifth of every secondary**.
///
/// The band is read on the node's attachment on its parent, so `(0.80, 1.0)` on the twig generation is
/// the rim of the crown's plate — which is where `08-flowering-bare-tree.jpg` masses the red and
/// nowhere else. The depths are four and three because a rule's `trunk` is depth nought.
fn kapok_flowers(species: &Species) -> Vec<Seat> {
    let mut seats = scatter_in(&species.plant, 4, KAPOK_FLOWER, 1.0, (3, 6), (0.80, 1.0));
    seats.extend(scatter_in(
        &species.plant,
        3,
        KAPOK_FLOWER,
        0.96,
        (0, 2),
        (0.80, 1.0),
    ));
    seats
}

/// The fir's cone template index, and the only decorator the species carries.
const FIR_CONE: usize = 0;

/// The fir's eight **erect** candlesticks on the leader's own bare top metre and a half.
///
/// # Invariants
///
/// **They are on the leader and on nothing else**, because they are the only cones in either conifer
/// that stand above the crown line: a cone on a tier's tip stands inside the crown however upright its
/// template is, and the leader's candle stops at `0.945` of the height so the last metre and a half of
/// bole is bare for these to stand against.
fn fir_leader_cones() -> Vec<Seat> {
    [
        (0.949, 0.12),
        (0.958, 0.63),
        (0.968, 0.35),
        (0.975, 0.87),
        (0.983, 0.41),
        (0.989, 0.72),
        (0.995, 0.26),
        (1.000, 0.94),
    ]
    .into_iter()
    .map(|(station, slot)| Seat::new(Handle::root(), FIR_CONE, station, slot))
    .collect()
}

/// The fir's cones: nought to two on every tier's tip and the leader's own candlesticks.
///
/// The station is past `0.9` on every call because the seat is on a node whose **tangent** turns up
/// only in its last tenth; a cone on a drooping node would hang, which is the spruce's arrangement and
/// would make the two species the same tree.
fn fir_cones(species: &Species) -> Vec<Seat> {
    let mut seats = scatter(&species.plant, 2, FIR_CONE, 0.96, (0, 2));
    seats.extend(scatter(&species.plant, 1, FIR_CONE, 0.99, (0, 2)));
    seats.extend(fir_leader_cones());
    seats
}

/// The spruce's cones: nought to one on every shoot of the second generation, because old cones persist
/// for years and the crown is studded rather than stippled.
fn spruce_cones(species: &Species) -> Vec<Seat> {
    scatter(&species.plant, 2, 0, 0.94, (0, 2))
}

/// The giant rhododendron's trusses: one on every shoot of the rosette generation, at its apex.
fn rhodo_trusses(species: &Species) -> Vec<Seat> {
    scatter(&species.plant, 3, 0, 1.0, (1, 1))
}

/// The dove tree's flower heads: one purple-brown ball on every spur of the fifth generation.
fn dove_heads(species: &Species) -> Vec<Seat> {
    scatter(&species.plant, 5, 0, 1.0, (1, 1))
}

/// The oak's acorns: nought to one on every shoot of the third generation.
fn oak_fruit(species: &Species) -> Vec<Seat> {
    scatter(&species.plant, 3, 0, 0.92, (0, 1))
}

/// The pine's cones: nought to one on every twig of the second generation.
fn pine_cones(species: &Species) -> Vec<Seat> {
    scatter(&species.plant, 2, 0, 0.95, (0, 1))
}

/// The fern's root mantle and its skirt of dead stipes, walked over two named bands of the column.
///
/// # Invariants
///
/// - **The walk is here rather than a call to `scatter_in`, because `scatter_in`'s band is read on the
///   node's attachment on its parent** and the fern's only node is the root, whose attachment is station
///   nought — a band on the column is the plant's own foot and nothing more. A mantle is a band *along*
///   a node, and the fern has no generation to band.
/// - **Each band carries its own template, written down rather than derived.** The root mantle is the
///   crown's own decorator nought and the skirt is nought and one, and the species module's own walk
///   passes the template as an argument; reading it out of the ring counter would put the skirt's
///   first two rings in the root's skin, which is a mistake that looks like nothing at forty metres.
fn fern_mantles(species: &Species) -> Vec<Seat> {
    let mut seats = Vec::new();
    for (template, band, stations, each) in [
        (0usize, (0.012f32, 0.452f32), 12u32, (12u32, 20u32)),
        (1, (0.680, 0.950), 7, (6, 11)),
    ] {
        let step = (band.1 - band.0) / (stations - 1) as f32;
        for station in 0..stations {
            seats.extend(scatter_in(
                &species.plant,
                0,
                template,
                band.0 + station as f32 * step,
                each,
                (0.0, 1.0),
            ));
        }
    }
    seats
}

// ---------------------------------------------------------------------------
// site_valley — the dry-hot valley at eight hundred metres
// ---------------------------------------------------------------------------

/// The kapok's surfaces on the valley floor: the same three tints its own flowering plate carries,
/// restated because the species module keeps its palette private and this frame may not edit it.
///
/// The wood is the note's grey bole, the two leaves are the note's *"loose, pale, open mass"*, and the
/// seat is a saturated red at a matte roughness: FOC's *"thick and fleshy, stellate-puberulent"* is a
/// matte petal, and `0.60` is the compromise between that note and a valley noon sun this strong.
fn valley_kapok_palette() -> Palette {
    Palette {
        wood: Skin::new([0.205, 0.198, 0.182], 0.88),
        leaves: vec![
            Skin::new([0.078, 0.118, 0.048], 0.62),
            Skin::new([0.098, 0.128, 0.056], 0.66),
        ],
        seats: vec![Skin::new([0.520, 0.060, 0.050], 0.60)],
    }
}

/// The fern's surfaces in the gully: a purple-black column under a mid-green crown under a warm brown
/// skirt, restated for the same reason as [`valley_kapok_palette`].
fn valley_fern_palette() -> Palette {
    Palette {
        wood: Skin::new([0.086, 0.068, 0.058], 0.80),
        leaves: vec![Skin::new([0.062, 0.115, 0.038], 0.72)],
        seats: vec![
            Skin::new([0.118, 0.088, 0.062], 0.86),
            Skin::new([0.150, 0.092, 0.048], 0.84),
        ],
    }
}

/// The dry-grass cover of a valley floor: straw, and paler and yellower than the montane frame's
/// olive.
const DRY_GRASS: [f64; 3] = [0.470, 0.415, 0.255];

/// Where the valley floor's tufts stand: **the rectangle between the camera and thirty metres ahead of
/// it**, which on this frame's plan is `x` from 66 to 96 and `z` from 0 to 30. Seven hundred of them,
/// because a valley floor at this camera's pitch is forty-five per cent of the picture and a bare
/// forty-five per cent is a dune.
const VALLEY_TUFTS: usize = 700;

/// The valley's plantings: **the hero kapok in flower on the open floor, four of its own kind in
/// flower around it, and three tree ferns in the gully**, which is the one damp line on a dry slope and
/// is where the note's 桫椤 stands — *"beside valley streams"* at **300–1600 m**, and this floor is at
/// eight hundred.
fn valley_plantings() -> Vec<Planting> {
    let mut rng = Rng::new(8_100);
    let mut plantings = Vec::new();
    // The hero: leafless, in red flower, and the valley's signature is that armature against a pale
    // hot sky rather than a crown of leaves. The flowers are the subject and are seated at the species'
    // own band, so the red masses on the rim of the plate and nowhere else.
    let hero = kapok(2_001, Season::Flower);
    plantings.push(
        seated(
            hero.clone(),
            valley_kapok_palette(),
            [74.0, 2.0],
            0.6,
            Detail::DEFAULT,
        )
        .with_seats(kapok_flowers(&hero)),
    );
    // Four companions, the same rule at the coarser dial, and **none nearer than twenty-six metres**,
    // which is the frame's own scale rule and not a taste: a twenty-metre kapok at twelve metres
    // subtends a hundred and twenty degrees and its crown runs out of the picture, so a companion
    // closer than a quarter of the way up the slope becomes the subject by standing in front of it.
    let companions: [[f32; 2]; 4] = [[56.0, 20.0], [94.0, -6.0], [68.0, -14.0], [88.0, -18.0]];
    for (index, at) in companions.iter().enumerate() {
        let species = kapok(2_100 + index as u64, Season::Flower);
        plantings.push(
            seated(
                species.clone(),
                valley_kapok_palette(),
                *at,
                rng.unit() * std::f32::consts::TAU,
                COMPANION,
            )
            .with_seats(kapok_flowers(&species)),
        );
    }
    // The gully runs along `z` near eleven, and the ferns stand in it and nowhere else: a fern is a
    // shade plant and this floor has no other shade. They are the nearest plants in the frame on
    // purpose — six metres of column and crown at sixteen metres is the only way a gully of six-metre
    // plants reads as a gully at all — and **none of them is on the hero's own bearing**, because a
    // dark fern column standing in front of a pale bole at the same screen position reads as one trunk
    // with two roots and the hero is lost inside it.
    for (index, at) in [[88.0f32, 14.0f32], [70.0, 12.0], [62.0, 11.0]]
        .into_iter()
        .enumerate()
    {
        let species = tree_fern(2_200 + index as u64, FernFoliage::Frond);
        plantings.push(
            seated(
                species.clone(),
                valley_fern_palette(),
                at,
                rng.unit() * std::f32::consts::TAU,
                COMPANION,
            )
            .with_seats(fern_mantles(&species)),
        );
    }
    // The cover last, so the ids an author names stay the small ones.
    for index in 0..VALLEY_TUFTS {
        let across = 66.0 + 30.0 * rng.unit();
        let deep = 0.0 + 30.0 * rng.unit();
        plantings.push(tufted(9_100 + index as u64, [across, deep], DRY_GRASS));
    }
    plantings
}

/// The valley's fallen log: **4.6 m long, of stated dimensions, lying on ground that falls 1.7 m over
/// its own length**.
///
/// # Invariants
///
/// **The spine carries the fall rather than the prop carrying a tilt**, because a [`Prop`] is a
/// translation and not a rotation — its own note says a prop is not turned — so a log on a slope is a
/// log whose sweep descends with the ground. The radius is 0.26 m at the butt and 0.13 m at the tip,
/// and both ends are stated because a log's *length* is what a reader checks E2 against.
fn valley_log() -> Prop {
    let mut sweep = shape::Sweep::new(
        vegetation::species::spine(&[
            ([0.00, 0.00, 0.00], [1.00, -0.42, 0.02]),
            ([1.40, -0.60, 0.05], [1.00, -0.40, 0.02]),
            ([2.80, -1.22, -0.03], [1.00, -0.42, -0.03]),
            ([4.20, -1.73, 0.00], [1.00, -0.39, 0.02]),
        ]),
        shape::Section::circle(),
    );
    sweep.scale = vegetation::species::taper(&[
        (0.0, 0.260),
        (0.30, 0.242),
        (0.72, 0.208),
        (0.94, 0.160),
        (1.0, 0.090),
    ]);
    let at = [81.0, slope_height(81.0, 11.6) + 0.24, 11.6];
    packed_prop("fallen_log", sweep, 8, 12, [0.360, 0.272, 0.180], 0.95, at)
}

/// A hot noon on a valley floor at eight hundred metres.
///
/// # Invariants
///
/// - **The key is at `63.3` degrees of elevation and `55` degrees to the camera's left**, and the
///   second half of that pair is the one the first pass got wrong. A key at the right elevation and
///   behind the subject lights the far side of every bole and leaves the side the reader sees lit by
///   nothing but sky — which is why the first two passes drew **blue trunks** on a red floor under a
///   hot white sun. The sun here stands over the camera's own left shoulder.
/// - **The fill is the ground's own bounce and not the sky's**, warm and at `0.55`, because a valley
///   floor of red earth throws warm light up into the underside of every crown and that light is what
///   a vertical trunk is actually lit by. The blue the first pass's trunks wore was the fill's.
/// - **The zenith is a bleached cyan at `0.34, 0.44, 0.62` and not a deep blue**, and the horizon is
///   `0.86, 0.83, 0.76`, because a dry-hot valley at noon under the dry season's dust is not the
///   subalpine sky; `scale_height` is **3000 m**, which is the valley's thin air above the dust the
///   `haze` dial is standing for.
/// - **A light's reach is its cutoff and must clear the floor's own diagonal**: the floor reaches
///   220 m and its diagonal is 311 m, so the reach is **700 m**. The *distance* is where the irradiance
///   is stated, and it is the key's own length from the origin, **80.0 m**.
fn valley_lighting() -> Lighting {
    Lighting {
        key_position: [65.0, 41.6, 21.0],
        key_colour: [1.0, 0.97, 0.90],
        key_irradiance: 5.4,
        key_distance: 80.0,
        key_reach: 700.0,
        key_extent: 6.0,
        fill_position: [-30.0, 12.0, -18.0],
        fill_colour: [0.72, 0.66, 0.58],
        fill_irradiance: 0.55,
        fill_distance: 37.0,
        floor: Skin::new([0.268, 0.148, 0.088], 0.97),
        sky: Atmosphere::new()
            .zenith([0.34, 0.44, 0.62])
            .horizon([0.86, 0.83, 0.76])
            .ground([0.46, 0.36, 0.27])
            .density(0.0010)
            .scale_height(3000.0),
        sky_radiance: 0.42,
        haze: 0.010,
    }
}

/// The valley frame: standing on the floor the drop past `x` seventy has made, at a person's eye above
/// **the ground under those feet** and not at an absolute altitude, looking back up the slope at a
/// twenty-metre tree that stands four and a half metres above the camera on a floor eleven metres
/// below the montane frame's.
///
/// The field of view is `0.98` rad and that is arithmetic rather than taste: the hero's crown top sits
/// at `41.5` degrees above the eye at twenty-eight metres, the foreground starts at eight, and a
/// twenty-metre tree with sky above it and ground below it needs fifty-six degrees to hold.
fn valley_frame() -> Framing {
    Framing {
        width: 1600,
        height: 1000,
        eye: [88.0, slope_height(88.0, 26.0) + 1.7, 26.0],
        target: [74.0, -17.6, 2.0],
        field_of_view: 0.98,
        near: 0.3,
        far: 900.0,
        floor: 220.0,
        ground: Ground::Fielded {
            height: slope_height,
            divisions: 200,
        },
    }
}

/// Builds the dry-hot valley frame: a kapok in flower, four of its own kind, three ferns in the gully,
/// two hundred and eighty dry tufts, one fallen log, one noon.
pub fn site_valley() -> Value {
    plant_scene_lit(
        &valley_plantings(),
        &valley_frame(),
        SITE_VALLEY,
        &valley_lighting(),
        &[valley_log()],
    )
}

/// Resolves a valley frame's `authored_id` into the planting and the node it names.
pub fn site_valley_handle_of(id: u32) -> Option<String> {
    planting_handle(&valley_plantings(), id)
}

// ---------------------------------------------------------------------------
// site_subalpine — the closed stand at three thousand two hundred metres
// ---------------------------------------------------------------------------

/// Where the stand's tufts stand: the rectangle in front of the camera, `x` from -86 to -58 and `z`
/// from 8 to 34, which is the band between the lens and twenty-six metres of it.
const SUBALPINE_TUFTS: usize = 600;

/// The mossy floor's own cover: a dark moss green, and the darkest skin any of the three frames puts
/// on the ground.
const MOSS: [f64; 3] = [0.135, 0.165, 0.090];

/// The subalpine stand's plantings: **a spruce and a fir as co-heroes on the camera's own bearing, one
/// more of each beside the lens at twelve metres, six more between them and the heroes, and four
/// giant rhododendrons under the lot**.
///
/// # Invariants
///
/// - **The foliage is [`SpireFoliage::Plume`] on every planting, hero and companion alike.** The spire
///   module has no stand tier of its own and does not need one: `Plume` is already the ninth of the
///   needle's density, and a ninth is what makes a thirty-four-metre crown cost forty thousand rows
///   rather than three hundred and four.
/// - **A companion stands within half a frame-width of the lens on both sides.** That is what a closed
///   stand *is* — the first pass put its nearest companion at nineteen metres and drew a treeline on
///   an open hill with fifty-five per cent of the picture bare — and the two near boles are also what
///   keep the ground band from being the whole of the lower half.
fn subalpine_plantings() -> Vec<Planting> {
    let mut rng = Rng::new(8_200);
    let mut plantings = Vec::new();
    // The two co-heroes on the camera's own bearing, forty-eight and sixty-one metres out, at the
    // crate's own dial and each carrying its own species' cone rule. They are on the axis because the
    // fir's erect candlesticks are the frame's one thing that has to reach the sky, and a cone that is
    // thirty degrees off centre is a cone in a corner.
    let hero_spruce = lijian_spruce(3_001, SpireFoliage::Plume);
    plantings.push(
        seated(
            hero_spruce.clone(),
            spruce_palette(),
            [-108.0, 14.0],
            0.5,
            Detail::DEFAULT,
        )
        .with_seats(spruce_cones(&hero_spruce)),
    );
    let hero_fir = cangshan_fir(3_002, SpireFoliage::Plume);
    plantings.push(
        seated(
            hero_fir.clone(),
            fir_palette(),
            [-116.0, 0.0],
            2.6,
            Detail::DEFAULT,
        )
        .with_seats(fir_cones(&hero_fir)),
    );
    // The two near boles, one each side of the lens at twelve metres, whose crowns run out of the top
    // of the frame and whose roots stand in the foreground band.
    for (index, at) in [[-74.0f32, 31.0f32], [-70.0, 19.0]].into_iter().enumerate() {
        let species = lijian_spruce(3_090 + index as u64, SpireFoliage::Plume);
        plantings.push(
            seated(
                species.clone(),
                spruce_palette(),
                at,
                rng.unit() * std::f32::consts::TAU,
                COMPANION,
            )
            .with_seats(spruce_cones(&species)),
        );
    }
    // Six more between the lens and the heroes, so that the crowns interlock along the whole depth of
    // the frame rather than at one distance. Every one of them costs the same rows the hero does, and
    // this is the frame's whole row budget: ten companions is where a hundred and forty thousand
    // companion rows stop buying a denser picture.
    let spruce_companions: [[f32; 2]; 3] = [[-92.0, 26.0], [-80.0, 8.0], [-104.0, 4.0]];
    for (index, at) in spruce_companions.iter().enumerate() {
        let species = lijian_spruce(3_100 + index as u64, SpireFoliage::Plume);
        plantings.push(
            seated(
                species.clone(),
                spruce_palette(),
                *at,
                rng.unit() * std::f32::consts::TAU,
                COMPANION,
            )
            .with_seats(spruce_cones(&species)),
        );
    }
    let fir_companions: [[f32; 2]; 3] = [[-86.0, 16.0], [-98.0, 22.0], [-112.0, -12.0]];
    for (index, at) in fir_companions.iter().enumerate() {
        let species = cangshan_fir(3_200 + index as u64, SpireFoliage::Plume);
        plantings.push(
            seated(
                species.clone(),
                fir_palette(),
                *at,
                rng.unit() * std::f32::consts::TAU,
                COMPANION,
            )
            .with_seats(fir_cones(&species)),
        );
    }
    // The understory: 大树杜鹃 is a two-hundred-year-old tree of **2100–3200 m** on the Gaoligongshan
    // and it is the subalpine forest's own shrub layer, not a montane one. Four of them at the
    // coarser dial, **none of them on the camera's bearing** — the first pass put one dead ahead and a
    // glossy dark-green crown with a rose truss on it outshone both co-heroes, which is the
    // understory taking the frame from the canopy.
    for (index, at) in [
        [-74.0f32, 30.0f32],
        [-86.0, 32.0],
        [-98.0, 6.0],
        [-110.0, 24.0],
    ]
    .into_iter()
    .enumerate()
    {
        let species = giant_rhododendron(3_300 + index as u64);
        plantings.push(
            seated(
                species.clone(),
                rhodo_palette(),
                at,
                rng.unit() * std::f32::consts::TAU,
                COMPANION,
            )
            .with_seats(rhodo_trusses(&species)),
        );
    }
    for index in 0..SUBALPINE_TUFTS {
        let across = -86.0 + 28.0 * rng.unit();
        let deep = 8.0 + 26.0 * rng.unit();
        plantings.push(tufted(9_200 + index as u64, [across, deep], MOSS));
    }
    plantings
}

/// The subalpine stand's standing stone: **1.2 m**, the same known size the montane frame states, so
/// the two frames' E2 answers are the same object at the same height in two different belts.
///
/// It stands twenty-two metres off the lens and fourteen degrees left of the axis, because the first
/// pass put it at eight and the frame's own foreground at eight is the base of a thirty-four-metre
/// spruce — a stone at a tree's foot measures the tree and the reader never sees the stone.
fn subalpine_stone() -> Prop {
    let mut sweep = shape::Sweep::new(
        vegetation::species::spine(&[
            ([0.0, 0.0, 0.0], [0.05, 0.5, 0.02]),
            ([0.06, 0.55, 0.03], [0.02, 0.5, -0.02]),
            ([0.05, 1.05, 0.01], [0.0, 0.35, 0.0]),
        ]),
        shape::Section::circle(),
    );
    sweep.scale =
        vegetation::species::taper(&[(0.0, 0.42), (0.35, 0.34), (0.8, 0.22), (1.0, 0.06)]);
    let at = [-84.0, slope_height(-84.0, 28.9) - 0.10, 28.9];
    packed_prop("stone", sweep, 8, 9, [0.215, 0.225, 0.215], 0.96, at)
}

/// A cold high sun over the subalpine stand.
///
/// # Invariants
///
/// - **The key's elevation is stated from the stand and not from the origin**, and that is the whole
///   of what the first pass got wrong. A key at `[22, 58, -30]` is fifty-seven degrees above the
///   *origin* and **twenty-one degrees above a camera standing sixty metres away in `x`**, so the
///   first pass drew a high-sun frame at the elevation of a late afternoon and every shadow in it
///   lay the wrong way. The sun here is **fifty-five degrees above the stand** and `55` degrees in
///   azimuth from the camera's own bearing, which is what puts the near boles' lit flanks on the
///   camera's side and their shadow sides facing the lens.
/// - **The fill is a deep blue at `0.90`**, which is the frame's "deep blue shadow" stated as a
///   number: a shadow's colour is the sky's own and its strength is what puts blue in it. The first
///   pass's fill of `0.85` against a `0.75` sky radiance left the shadowed sides a neutral grey-brown;
///   the second pass's `1.20` put so much blue in the whole frame that the moss read as mint, and
///   `0.90` against a `0.45` sky is the ratio that keeps the shadow blue and the lit flank white.
/// - **The floor's albedo is `0.030` and not `0.115`**, which is the finding of two passes and is a
///   statement about this renderer's grade rather than about moss: the post stage lifts the low end
///   hard enough that a floor four times darker than a valley's red earth draws as the **paler** of
///   the two. "Mossy dark" is not a tint a frame gets by writing moss in the palette; it is the tint
///   that comes out dark under this grade.
/// - **The haze is `0.002`** against the valley frame's `0.010`, and those two numbers are the whole of
///   the difference between a humid gully at eight hundred metres and a clear ridge at three thousand
///   two hundred.
fn subalpine_lighting() -> Lighting {
    Lighting {
        key_position: [-82.8, 88.0, 62.5],
        key_colour: [1.00, 1.00, 1.00],
        key_irradiance: 4.2,
        key_distance: 136.0,
        key_reach: 700.0,
        key_extent: 12.0,
        fill_position: [-95.8, 38.0, -25.6],
        fill_colour: [0.26, 0.40, 0.82],
        fill_irradiance: 0.90,
        fill_distance: 106.2,
        floor: Skin::new([0.030, 0.038, 0.020], 0.98),
        sky: Atmosphere::new()
            .zenith([0.105, 0.195, 0.46])
            .horizon([0.60, 0.67, 0.76])
            .ground([0.16, 0.19, 0.20])
            .density(0.0007)
            .scale_height(800.0),
        sky_radiance: 0.45,
        haze: 0.002,
    }
}

/// The subalpine frame: standing **inside** the stand on the shoulder itself, at a person's eye above
/// the ground under those feet, with a wide lens — `0.90` rad, a fifty-two-degree horizontal — because
/// a closed canopy is photographed wide or not at all.
///
/// The eye is aimed at `18` degrees of elevation and that is the frame's balance: at `10` degrees the
/// spruce's crown top is thirty degrees below the top edge and the picture is a third sky; at `26`
/// the foreground starts twenty-six metres out and the ground band is half the picture.
fn subalpine_frame() -> Framing {
    Framing {
        width: 1600,
        height: 1000,
        eye: [-62.0, slope_height(-62.0, 28.0) + 1.7, 28.0],
        target: [-108.0, 35.05, 14.0],
        field_of_view: 0.90,
        near: 0.3,
        far: 900.0,
        floor: 220.0,
        ground: Ground::Fielded {
            height: slope_height,
            divisions: 300,
        },
    }
}

/// Builds the subalpine frame: two co-heroes, eight companions, four rhododendrons, six hundred mossy
/// tufts, one stone, one cold noon.
pub fn site_subalpine() -> Value {
    plant_scene_lit(
        &subalpine_plantings(),
        &subalpine_frame(),
        SITE_SUBALPINE,
        &subalpine_lighting(),
        &[subalpine_stone()],
    )
}

/// Resolves a subalpine frame's `authored_id` into the planting and the node it names.
pub fn site_subalpine_handle_of(id: u32) -> Option<String> {
    planting_handle(&subalpine_plantings(), id)
}

// ---------------------------------------------------------------------------
// site_transect — the whole slope, four belts, one frame
// ---------------------------------------------------------------------------

/// Where the transect's tufts stand: the near shoulder **between the lens and the montane belt**, `x`
/// from -62 to -18 and `z` from 60 to 88, which is four to thirty-two metres of the picture's own
/// foreground.
///
/// # Invariants
///
/// **Nine hundred of them in forty-four square metres, and not three hundred in three thousand six
/// hundred.** The first pass wrote the band as `z` 100 to 148 while the camera stood at `z` ninety
/// looking toward `-z`, so every tuft it had was behind the lens and the lower half of the plate was
/// bare tan; the second wrote `z` 34 to 86 at three hundred, which is one tuft to twelve square
/// metres and still reads as a dune. Cover is a density and not a count — and it is free, because
/// [`tufted`] writes no instance rows.
const TRANSECT_TUFTS: usize = 900;

/// The transect's plantings: **every species this crate has built, each in its own belt**, at the
/// dials a hundred-and-forty-metre subject can be drawn at.
///
/// # Invariants
///
/// - **A species appears once at its hero's dial and its companions at [`DISTANT`]**, because the frame
///   is a diagram first: fifteen plants at the crate's default would cost three hundred and fifty
///   thousand rows and the standing stone beside the lens would still be the only thing in the picture
///   whose size a reader could check.
/// - **The pine hero and all three of its companions are [`PineFoliage::Spray`]**, which is the one
///   species in the frame whose stand tier is switched off altogether. At `Stand` the pine's plumes
///   are three per cent of the needle's density, and eighty metres away that is a rounded grey-green
///   mass with no cone and no leader — this is the frame the ten silhouettes are judged in, and a pine
///   that reads as a broadleaf is a wrong answer and not a small one. It costs fifty thousand rows a
///   tree against eleven, which is a hundred and fifty-six thousand on the one species, and the frame
///   still compiles inside a minute.
///
/// # Why it is public
///
/// `examples/site_cost_probe.rs` measures the site by building these plantings and their expansions
/// as two phases, which [`site_transect`] cannot show because it builds the whole document in one
/// call. Nothing else outside this module has a reason to ask for them.
pub fn transect_plantings() -> Vec<Planting> {
    let mut rng = Rng::new(8_300);
    let mut plantings = Vec::new();

    // **The subalpine belt**, at the top of the frame's far left: a spruce and a fir and a companion
    // of each, with the rhododendrons under them.
    let fir = cangshan_fir(4_002, SpireFoliage::Plume);
    plantings.push(
        seated(fir.clone(), fir_palette(), [-74.0, 4.0], 1.4, DISTANT).with_seats(fir_cones(&fir)),
    );
    let fir_companion = cangshan_fir(4_003, SpireFoliage::Plume);
    plantings.push(
        seated(
            fir_companion.clone(),
            fir_palette(),
            [-72.0, 18.0],
            3.1,
            COMPANION,
        )
        .with_seats(fir_cones(&fir_companion)),
    );
    let spruce = lijian_spruce(4_001, SpireFoliage::Plume);
    plantings.push(
        seated(
            spruce.clone(),
            spruce_palette(),
            [-78.0, 14.0],
            0.9,
            DISTANT,
        )
        .with_seats(spruce_cones(&spruce)),
    );
    let spruce_companion = lijian_spruce(4_004, SpireFoliage::Plume);
    plantings.push(
        seated(
            spruce_companion.clone(),
            spruce_palette(),
            [-70.0, 24.0],
            4.2,
            COMPANION,
        )
        .with_seats(spruce_cones(&spruce_companion)),
    );
    for (index, at) in [[-66.0f32, 10.0f32], [-62.0, 18.0]].into_iter().enumerate() {
        let species = giant_rhododendron(4_100 + index as u64);
        plantings.push(
            seated(
                species.clone(),
                rhodo_palette(),
                at,
                rng.unit() * std::f32::consts::TAU,
                DISTANT,
            )
            .with_seats(rhodo_trusses(&species)),
        );
    }

    // **The montane belt**, in the middle distance: the pine stand on the sunny slope, the oak in the
    // draw and the dove tree at the gully's edge.
    let pine = yunnan_pine(4_101, PineFoliage::Spray);
    plantings.push(
        seated(pine.clone(), pine_palette(), [-20.0, 16.0], 0.3, COMPANION)
            .with_seats(pine_cones(&pine)),
    );
    for (index, at) in [[-8.0f32, 8.0f32], [-28.0, 22.0], [0.0, 20.0]]
        .into_iter()
        .enumerate()
    {
        let species = yunnan_pine(4_200 + index as u64, PineFoliage::Spray);
        plantings.push(
            seated(
                species.clone(),
                pine_palette(),
                at,
                rng.unit() * std::f32::consts::TAU,
                COMPANION,
            )
            .with_seats(pine_cones(&species)),
        );
    }
    let oak = alpine_oak(4_300);
    plantings.push(
        seated(oak.clone(), oak_palette(), [-14.0, 4.0], 2.0, DISTANT).with_seats(oak_fruit(&oak)),
    );
    let dove = dove_tree(4_400);
    plantings.push(
        seated(dove.clone(), dove_palette(), [2.0, 12.0], 5.0, DISTANT)
            .with_seats(dove_heads(&dove)),
    );

    // **The valley belt**, at the frame's right: the kapok in flower on the floor and a fern in the
    // gully beside it.
    let kapok_hero = kapok(4_500, Season::Flower);
    plantings.push(
        seated(
            kapok_hero.clone(),
            valley_kapok_palette(),
            [56.0, 6.0],
            1.1,
            DISTANT,
        )
        .with_seats(kapok_flowers(&kapok_hero)),
    );
    for (index, at) in [[48.0f32, 18.0f32], [62.0, 16.0]].into_iter().enumerate() {
        let species = kapok(4_600 + index as u64, Season::Flower);
        plantings.push(
            seated(
                species.clone(),
                valley_kapok_palette(),
                at,
                rng.unit() * std::f32::consts::TAU,
                COMPANION,
            )
            .with_seats(kapok_flowers(&species)),
        );
    }
    for (index, at) in [[58.0f32, 12.0f32], [68.0, 10.0]].into_iter().enumerate() {
        let species = tree_fern(4_700 + index as u64, FernFoliage::Frond);
        plantings.push(
            seated(
                species.clone(),
                valley_fern_palette(),
                at,
                rng.unit() * std::f32::consts::TAU,
                COMPANION,
            )
            .with_seats(fern_mantles(&species)),
        );
    }

    for index in 0..TRANSECT_TUFTS {
        let across = -62.0 + 44.0 * rng.unit();
        let deep = 60.0 + 28.0 * rng.unit();
        plantings.push(tufted(9_300 + index as u64, [across, deep], DRY_GRASS));
    }
    plantings
}

/// The transect's standing stone: the same **1.2 m** stone the other two frames carry, and the only
/// object in the frame a reader can measure against — which is why it stands eighteen metres from the
/// lens and not beside a tree, where at a hundred metres a stone of it is eleven pixels.
fn transect_stone() -> Prop {
    let mut sweep = shape::Sweep::new(
        vegetation::species::spine(&[
            ([0.0, 0.0, 0.0], [0.05, 0.5, 0.02]),
            ([0.06, 0.55, 0.03], [0.02, 0.5, -0.02]),
            ([0.05, 1.05, 0.01], [0.0, 0.35, 0.0]),
        ]),
        shape::Section::circle(),
    );
    sweep.scale =
        vegetation::species::taper(&[(0.0, 0.42), (0.35, 0.34), (0.8, 0.22), (1.0, 0.06)]);
    let at = [-36.0, slope_height(-36.0, 72.0) - 0.10, 72.0];
    packed_prop("stone", sweep, 8, 9, [0.260, 0.230, 0.190], 0.96, at)
}

/// One low sun along the valley, which is the transect's own light: the proposal's table gives this
/// frame a low sun rather than any belt's own, because a frame that puts four belts in one picture
/// cannot be lit four ways.
///
/// # Invariants
///
/// - **The sun is at `22` degrees of elevation and its bearing is down the valley**, and the second
///   half of that is the half the first pass got wrong. A key at `[80, 26, 6]` is `17.9` degrees above
///   the *origin* and **nine degrees above the near ground seventy metres away**, because the slope
///   falls toward `+x` and so faces down-valley while the sun was almost due `-z` of it: the whole
///   first plate was a black silhouette with a lit horizon behind it. The sun here is `50` degrees from
///   the camera's own bearing, which is what a low afternoon light down a valley is.
/// - **A light's reach is its cutoff and must clear the floor's own diagonal**: the floor reaches
///   240 m and its diagonal is 339 m, so the reach is **700 m**. The *distance* is where the irradiance
///   is stated, and it is the key's own length from the origin, **129.9 m**.
/// - **The floor's albedo is `0.150` and not `0.230`,** because a dry mountain slope read pale enough
///   to be sand rather than the montane frame's own olive, and the two frames' floors are meant to be
///   the same mountain at two belts.
fn transect_lighting() -> Lighting {
    Lighting {
        key_position: [22.5, 46.3, 119.3],
        key_colour: [1.0, 0.90, 0.74],
        key_irradiance: 3.8,
        key_distance: 129.9,
        key_reach: 700.0,
        key_extent: 14.0,
        fill_position: [-60.0, 22.0, -20.0],
        fill_colour: [0.46, 0.55, 0.72],
        fill_irradiance: 0.60,
        fill_distance: 67.0,
        floor: Skin::new([0.150, 0.128, 0.082], 0.96),
        sky: Atmosphere::new()
            .zenith([0.14, 0.24, 0.46])
            .horizon([0.86, 0.72, 0.55])
            .ground([0.30, 0.26, 0.20])
            .density(0.0010)
            .scale_height(1400.0),
        sky_radiance: 0.60,
        haze: 0.005,
    }
}

/// The transect frame: standing **off the slope's side** at `z` ninety, on the shoulder the law carries
/// out that far, at a person's eye above the ground under those feet, with an eighty-two-degree
/// horizontal that holds the subalpine belt at `x` minus eighty-two and the valley floor at `x` seventy
/// in one picture.
///
/// # Invariants
///
/// - **The camera is off to the side and never below.** The only ground on this slope above thirty
///   metres is the shoulder crest, so a side view that is also a view from above has to stand on the
///   shoulder and look down-slope across it.
/// - **The axis is the middle of the span, not the middle of the map.** From this eye the fir at
///   `x` minus seventy-eight bears `243` degrees and the kapok at `x` fifty-six bears `319`, so the
///   axis is `284` degrees and the belts sit `41` and `35` degrees either side of it inside an
///   eighty-six-degree frame. The first pass aimed at the map's own middle, which put the subalpine
///   belt outside the frame and the montane belt dead centre — a section of a third of the mountain
///   with the two ends of it cropped.
/// - **The eye is level.** The slope's own band runs from `+1.5` degrees at the subalpine crest to
///   `-14.7` degrees at the valley floor; a level axis splits the picture into a ground half and a sky
///   half with the band across the middle, and any downward tilt from there buys sky by spending
///   foreground.
fn transect_frame() -> Framing {
    Framing {
        width: 1600,
        height: 1000,
        eye: [-40.0, slope_height(-40.0, 90.0) + 1.7, 90.0],
        target: [-8.6, 14.96, -36.1],
        field_of_view: 1.06,
        near: 0.3,
        far: 900.0,
        floor: 240.0,
        ground: Ground::Fielded {
            height: slope_height,
            divisions: 300,
        },
    }
}

/// Builds the transect: fifteen plants in four belts, three hundred tufts, one stone, one low sun, and
/// fifty-six to one of compression stated above.
pub fn site_transect() -> Value {
    plant_scene_lit(
        &transect_plantings(),
        &transect_frame(),
        SITE_TRANSECT,
        &transect_lighting(),
        &[transect_stone()],
    )
}

/// Resolves a transect frame's `authored_id` into the planting and the node it names.
pub fn site_transect_handle_of(id: u32) -> Option<String> {
    planting_handle(&transect_plantings(), id)
}
