//! The site: one Hengduan slope, drawn at the height of each belt.
//!
//! # What this module is
//!
//! [`.agents/proposals/southwest-china-trees.md`](../../../.agents/proposals/southwest-china-trees.md)
//! §7: a hero specimen is a tree and an epic frame is a place. Every site frame here stands on **one**
//! heightfield — a ridge descending south-west into a valley, with a gully running along it — so the
//! four frames are one mountain seen at four heights rather than four inventions, and E1's check is a
//! comparison of the frame's sidecar against the species' notes rather than against a new ground per
//! scene.
//!
//! # What a site frame carries that a specimen frame does not
//!
//! Companions, at a coarser [`Detail`] and the spray foliage, seated on the slope by
//! [`vegetation::ground::seat`]; a standing stone of a stated height, because E2 wants a known size in
//! frame and a stone belongs in every one of these belts while a person does not; and a light that is
//! the belt's own — a hot noon in the valley, a raking morning on the montane slope, a cold high sun
//! over the subalpine stand.

use definition::Value;
use vegetation::derive::Species;
use vegetation::expand::Detail;
use vegetation::ground::seat;
use vegetation::plant::Placement;
use vegetation::rng::Rng;

use crate::air::Atmosphere;
use crate::southwest::{Foliage, pine_palette, yunnan_pine};
use crate::southwest_flag::{dove_palette, dove_tree};
use crate::southwest_oak::{alpine_oak, oak_palette};
use crate::vegetation_picture::{
    Framing, Ground, Lighting, Palette, Planting, Prop, Skin, plant_scene_lit, planting_handle,
};
use vegetation::derive::tuft;

/// The name the montane site frame is built and rendered under.
pub const SITE_MONTANE: &str = "site_montane";

/// The slope: a ridge falling south-west at about eleven degrees, a broad swell along it, a gully
/// running with it at `z` near eleven where the dove trees stand because that is where the moisture
/// is, a shoulder rising behind at `x` near minus fifty-five, and the floor of the valley dropping
/// away ahead past `x` seventy — so the frame has a behind, a here and a beyond, which a tilted plane
/// does not.
///
/// # Invariants
///
/// **One law, two readers.** The floor is drawn from this at `f64` and every planting is seated on it at
/// `f32`, and the two are the same function, so a tree cannot stand where the floor is not — the same
/// rule [`vegetation::ground`] states for a plant and its base.
pub fn slope_height(x: f64, z: f64) -> f64 {
    -0.20 * x + 0.8 * (x * 0.045).sin() * (z * 0.038).cos()
        - 1.6 * (-((z - 11.0) / 5.0).powi(2)).exp()
        + 6.0 * (-((x + 55.0) / 25.0).powi(2)).exp()
        - 14.0 / (1.0 + (-(x - 70.0) / 12.0).exp())
        + 0.35 * (x * 0.11).sin() * (z * 0.09).sin()
}

/// The slope at `f32`, for seating.
fn slope(p: [f32; 2]) -> f32 {
    slope_height(f64::from(p[0]), f64::from(p[1])) as f32
}

/// The detail a companion is drawn at: the hero's dial is the crate's default, and a companion on the
/// far ridge is the same rule coarser, which is what [`Planting::with_detail`] is for.
const COMPANION: Detail = Detail {
    stations: 4,
    around: 8,
    spacing: 1.5,
};

/// A planting of one species at a plan position, seated on the slope and turned by its own seed.
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

/// The montane slope's plantings: a hero pine, a hero oak in the draw, a hero dove tree in the gully,
/// and companions of all three at the coarser dial, placed by one seeded stream so the stand is the
/// same stand every time.
fn montane_plantings() -> Vec<Planting> {
    let mut rng = Rng::new(4_200);
    let mut plantings = Vec::new();
    plantings.push(seated(
        yunnan_pine(21, Foliage::Spray),
        pine_palette(),
        [0.0, 0.0],
        0.4,
        Detail::DEFAULT,
    ));
    plantings.push(seated(
        alpine_oak(31),
        oak_palette(),
        [-7.0, 6.0],
        2.2,
        Detail::DEFAULT,
    ));
    plantings.push(seated(
        dove_tree(41),
        dove_palette(),
        [3.0, 11.0],
        1.1,
        Detail::DEFAULT,
    ));
    // Three clumps, because a pine stand on a dry slope is clumps and not a grid: the centres are where
    // the soil is shallow, and the trees are where the seed landed.
    let centres: [[f32; 2]; 3] = [[-16.0, -10.0], [8.0, -24.0], [24.0, -6.0]];
    for index in 0..6 {
        let centre = centres[index % 3];
        let across = centre[0] + 5.0 * rng.signed();
        let deep = centre[1] + 5.0 * rng.signed();
        plantings.push(seated(
            yunnan_pine(100 + index as u64, Foliage::Stand),
            pine_palette(),
            [across, deep],
            rng.unit() * std::f32::consts::TAU,
            COMPANION,
        ));
    }
    // A far row on the shoulder behind, so the frame has a middle distance and the haze has something
    // to lie on.
    for index in 0..5 {
        let across = -38.0 - 6.0 * index as f32 + 4.0 * rng.signed();
        let deep = -14.0 + 10.0 * rng.signed();
        plantings.push(seated(
            yunnan_pine(400 + index as u64, Foliage::Stand),
            pine_palette(),
            [across, deep],
            rng.unit() * std::f32::consts::TAU,
            COMPANION,
        ));
    }
    // The ground cover: a montane slope at two thousand metres is grass and litter, not sand, and a
    // tuft is seven nodes and no leaves, so six hundred of them cost six hundred instance rows — the
    // cheapest mass in the crate. They stand in the foreground band only, because a tuft twenty metres
    // from the lens is two pixels and one at sixty is nothing.
    for index in 0..900 {
        // A third of the tufts stand in the near band between the lens and the stand, because a
        // foreground with nothing in it is a floor and not a ground.
        let near = index % 3 == 0;
        let across = if near {
            18.0 + 22.0 * rng.unit()
        } else {
            -4.0 + 30.0 * rng.unit()
        };
        let deep = if near {
            8.0 + 24.0 * rng.unit()
        } else {
            -14.0 + 24.0 * rng.unit()
        };
        plantings.push(
            Planting::new(tuft(9_000 + index as u64))
                .at(seat(
                    Placement {
                        origin: [across, 0.0, deep],
                        x: [1.0, 0.0, 0.0],
                        y: [0.0, 1.0, 0.0],
                        z: [0.0, 0.0, 1.0],
                    },
                    slope,
                ))
                .with_palette(Palette {
                    wood: Skin::new([0.40, 0.36, 0.18], 0.9),
                    leaves: Vec::new(),
                    seats: Vec::new(),
                }),
        );
    }
    plantings
}

/// The standing stone: a lumpy sweep packed as a prop, because E2 wants a known size in frame and a
/// cuboid of one reads as a box someone left there. Its height is 1.2 m and the sidecar's prose says
/// so; a stone of it is waist to shoulder high, which is the comparison a reader makes.
///
/// A prop draws on the ground's stages and therefore carries the ground's point schema, which has no
/// `authored_id`: packing it with the authored layout is a compile error, and a stone is not an
/// authored element of any plant, which is exactly what the reserved zero says.
fn montane_stone() -> Prop {
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
    let template = vegetation::decorator::Template {
        sweep,
        stations: 8,
        around: 9,
    };
    let loft = template.loft();
    let schema = crate::building_identity::not_authored_extras().point_schema();
    let layout = graphics::layout::packed_layout(&schema)
        .expect("the ground's own point schema is a struct of components");
    let corners = crate::geometry::loft_corners(&loft);
    let mut points = definition::PackedBuilder::new(layout);
    points.reserve(corners.len());
    for corner in &corners {
        let written = points
            .field("normal", &corner.normal)
            .and_then(|points| points.field("position", &corner.position))
            .and_then(|points| points.field("texture", &corner.texture));
        written.expect("a generator writes whole elements of its own layout");
    }
    let at = [5.0, slope_height(5.0, -4.0), -4.0];
    Prop {
        name: "stone".to_owned(),
        points: points
            .finish()
            .expect("every generator writes whole elements of its own layout"),
        tint: [0.28, 0.27, 0.26],
        roughness: 0.95,
        at: [at[0], at[1] - 0.1, at[2]],
    }
}

/// A raking morning on the montane slope: the sun low in the east along the valley, a cool sky, and
/// enough haze to put the far ridge behind the near stand.
fn montane_lighting() -> Lighting {
    Lighting {
        key_position: [60.0, 26.0, -34.0],
        key_colour: [1.0, 0.93, 0.80],
        key_irradiance: 3.0,
        key_distance: 73.4,
        key_reach: 700.0,
        key_extent: 60.0,
        fill_position: [-30.0, 18.0, 26.0],
        fill_colour: [0.44, 0.52, 0.70],
        fill_irradiance: 0.5,
        fill_distance: 43.7,
        floor: Skin::new([0.30, 0.30, 0.17], 0.95),
        sky: Atmosphere::new()
            .zenith([0.09, 0.19, 0.42])
            .horizon([0.62, 0.58, 0.50])
            .ground([0.30, 0.28, 0.24])
            .density(0.0009)
            .scale_height(900.0),
        sky_radiance: 0.7,
        haze: 0.003,
    }
}

/// The montane frame: standing off down-slope at a person's eye height above the ground the eye stands
/// on, so the trees tower and the ridge line sits high in the frame. The eye's height is the slope's
/// own at its plan position plus 1.7 m, because a site frame's camera is a person and a person stands
/// on the ground, not at an absolute altitude.
fn montane_frame() -> Framing {
    Framing {
        width: 1600,
        height: 1000,
        eye: [38.0, slope_height(38.0, 30.0) + 1.7, 30.0],
        target: [0.0, 6.5, 2.0],
        field_of_view: 0.80,
        near: 0.3,
        far: 900.0,
        floor: 220.0,
        ground: Ground::Fielded {
            height: slope_height,
            divisions: 120,
        },
    }
}

/// Builds the montane site frame: three heroes, eleven companions, a hundred and twenty tufts, one
/// stone, one morning.
pub fn site_montane() -> Value {
    plant_scene_lit(
        &montane_plantings(),
        &montane_frame(),
        SITE_MONTANE,
        &montane_lighting(),
        &[montane_stone()],
    )
}

/// Resolves a site frame's `authored_id` into the planting and the handle it names.
pub fn site_montane_handle_of(id: u32) -> Option<String> {
    planting_handle(&montane_plantings(), id)
}
