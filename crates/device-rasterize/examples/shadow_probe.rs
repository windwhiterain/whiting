//! What a caster that asks for its own shadow resolution actually gets, measured on a real gpu.
//!
//! The virtual shadow map landed with its **default** calibrated to be the resolution the old pool
//! gave, and every frame that work produced is therefore a frame where nothing changed. That is
//! the compatibility claim, and it is the only claim any picture has tested. No scene in the
//! gallery declares `shadow_density` at all, so whether a caster that asks for eight times the
//! density gets a shadow eight times finer — while the wall beside it keeps the resolution it had
//! — is untested from the document to the pixel. This measures it.
//!
//! ```text
//! cargo run -p whiting-device --features wgpu --example shadow_probe
//! ```
//!
//! Every probe is a **controlled pair**: one variable moved, everything else held, and a number
//! that can come out the other way. A probe that cannot fail is not a probe, so each one below
//! says what would have to come back for it to have failed.
//!
//! # The scene
//!
//! One lamp, one floor, and whichever casters the probe names. The lamp, its cone and the first
//! camera are `spotlight`'s own numbers, because that scene was composed to make a cone legible
//! and there is no reason for a probe to invent a second one. The floor is the receiver every
//! measurement is taken on, and the casters are:
//!
//! - a **rod** a few hundredths of a unit across, whose shadow is a line a couple of pixels wide
//!   and which is therefore the only caster here whose resolution can be *seen* rather than
//!   inferred from a number;
//! - a **wall**, long and low, whose shadow is a band wide enough to measure and long enough to
//!   cross several page boundaries, which is where a leak between two pages would show;
//! - an **offstage** caster, standing where one camera cannot see it, for the claim that a caster
//!   outside the camera frustum still casts. Whether it does is **measured** from the `position`
//!   channel rather than assumed, and a camera that turns out to see it is reported as having
//!   tested nothing — which is a finding about the probe and not about the feature.
//!
//! A probe that wants one caster's shadow measured asks for that caster alone. Two casters in one
//! frame is the picture that shows what the design is for, and it is also the frame whose numbers
//! are hardest to attribute, so it comes after the isolated ones.
//!
//! # What is measured, and how each measure can be wrong
//!
//! - **The grid, the pages and the pass count** are read out of the compiled documents: the page
//!   table is decoded from the `light_records` run exactly as `crates/scene/tests/spot_lights.rs`
//!   decodes it, and the passes are counted from the emitted document. Nothing here reimplements
//!   the allocator's arithmetic, so a disagreement is a disagreement between the two halves of the
//!   emitter rather than with a second implementation of it.
//! - **The soft share** is the count of floor pixels lying strictly between a row's darkest and
//!   brightest floor, over the count that are shadowed at all. It is a ratio rather than a width
//!   because it needs no knowledge of where in the frame a given caster's shadow lands, which is
//!   what lets it survive a change to the scene. Each row is normalised against **its own** darkest
//!   and brightest, because the lamp falls off across the floor and a threshold shared by the whole
//!   frame would report the far end of the pool as shadowed. A row whose spread is too small to
//!   hold a shadow is left out rather than counted as a row of nothing.
//! - **The extent** is the dark pixels plus the soft ones, and it is the measure that says whether
//!   the chain of levels held together: the chain is supposed to change how **sharp** a shadow is
//!   and never whether there is one, so an extent that moves with density is a shadow appearing or
//!   disappearing rather than sharpening.
//! - **A leak** is a floor pixel that reads fully lit while six or more of its eight neighbours
//!   read fully shadowed, which is what a page boundary that lets the receiver through looks like
//!   and what nothing else in this scene produces. It is counted rather than judged, because a
//!   hairline is not something a person sees in a picture at this size.
//! - **Whether a caster is in frame** is asked of the `position` channel rather than guessed at from
//!   pixel coordinates, so a probe that failed to put its caster out of frame says so instead of
//!   quietly reporting a number about a setup that was never tested.

use definition::{Definition, DefinitionIdentifier, Registry, SchemaIdentifier, Value};
use device::{RenderedFrame, RenderedImage, Renderer};
use gallery::Gallery;
use gallery::geometry::{cuboid, plane_facing};
use gallery::kit::{
    camera_placed, instance, instances, mesh, mesh_material, post_processes, screen_material,
    shaded_point_schema, spot, surface,
};
use gallery::shaders::{mesh_shaders, post_shader};
use graphics::{ENTRIES, GRAPHICS_SCHEMA, INSTANCE_SCHEMA, passes};
use scene::lighting::{LIGHT_RECORDS_ENTRY, SHADOW_ATLAS_SIDE_PAGES};
use scene::standard::{SHADOW_DENSITY, Stage};
use scene::vshadow::{pages_at_level, unpack_high, unpack_low};
use scene::{
    CompileContext, CompileOutput, Compiler, DebugChannel, STANDARD_SCENE_SCHEMA, StandardScene,
};
use whiting_device_rasterize::rasterizer;

/// Where the frames go, which `out` already keeps out of the tree.
const OUTPUT: &str = "out/probe";

/// The lamp and its cone, as `spotlight` composed them.
///
/// They are copied rather than re-chosen because that scene's own documentation says why each
/// number is what it is, and a probe with a different cone would be measuring a different light.
const LAMP_POSITION: [f64; 3] = [-1.6, 8.5, 4.2];
const LAMP_DIRECTION: [f64; 3] = [0.18, -1.0, -0.36];
const LAMP_COLOUR: [f64; 3] = [1.0, 0.93, 0.80];
const LAMP_INTENSITY: f64 = 1.15;
const LAMP_INNER: f64 = 0.16;
const LAMP_OUTER: f64 = 0.56;
const LAMP_REACH: f64 = 9.5;

/// The camera every measurement is made against, and the three that ask a different question of
/// the same scene.
const EYE: [f64; 3] = [7.4, 4.6, 9.2];
const TARGET: [f64; 3] = [-0.4, 0.9, -1.0];
const FIELD_OF_VIEW: f64 = 0.92;
const NEAR: f64 = 3.0;
const FAR: f64 = 30.0;

/// How many cameras the scene declares, which is what a compile indexes them by.
const CAMERA_COUNT: usize = 4;

/// The floor, which is the receiver every measure is taken on, and how far it reaches.
const FLOOR_EXTENT: f64 = 9.0;
const FLOOR_TINT: [f64; 3] = [0.62, 0.60, 0.57];

/// What the post stage does to the frame, which is `spotlight`'s own grade.
const GRADE: &str = r#"
    color = color / (color + vec3<f32>(1.0));
    color = pow(color, vec3<f32>(0.4545));
"#;

/// The density the scene falls back on when an instance names none, in texels a world unit.
///
/// It is the pool's own page width over the width of the cone at the depth the grid is measured
/// at, which is what makes the finest level of a silent scene the page the pool gave. Every
/// absolute ladder in this file is written around it rather than around a round number, because a
/// round number is either below it — where the grid steps down a whole level and the comparison is
/// against something else — or a hair above it, where one step already quadruples the pyramid.
const DEFAULT_DENSITY: f64 = 58.07;

/// The density the rod asks for wherever one caster's resolution is compared against another's.
///
/// It is chosen rather than picked: it must be several levels above the wall for the comparison to
/// say anything, and it must be low enough that the pages **one** caster asks for still fit an
/// atlas layer. Four levels over a thirty two page grid is a thousand and a quarter pages at level
/// zero alone, and the margin that keeps the chain unbroken pads even a single caster's footprint
/// to five pages across, so a probe with two casters at this density needs a ceiling raised before
/// it will compile at all.
const FINE_DENSITY: f64 = 232.0;

/// The density the wall asks for: an eighth of the rod's, which is exactly three levels coarser.
const COARSE_DENSITY: f64 = 29.0;

/// How many pages a side one layer of the atlas may be, which the pipeline defaults to sixteen.
///
/// It is raised here because a probe whose finest density is **refused** measures the ceiling
/// rather than the resolution, and because the ladder's whole purpose is to walk up until something
/// gives. Sixteen holds two hundred and fifty six pages; a single caster at the coarsest level of a
/// thirty two page grid already asks for more than that, and the first run of this probe found that
/// out by refusing every density above a hundred and sixteen.
const ATLAS_SIDE_PAGES: i64 = 64;

/// Where a light record's virtual page address starts, in the words a shader counts.
///
/// It is an offset rather than a field because the record is packed as a run of floats, and the
/// same twenty four numbers are written out beside `light_record` in
/// `crates/gallery/src/shaders.rs`.
const VIRTUAL_PAGE_WORD: usize = 24;

/// A caster's shadow needs this many floor pixels in it before its brightness distribution means
/// anything, which is what keeps a shadow that fell outside the frame from being measured as an
/// empty one.
const MINIMUM_SHADOW_PIXELS: usize = 64;

/// The lit ring a shadow's contrast is measured against needs this many pixels before its
/// percentile means anything.
const MINIMUM_RING_PIXELS: usize = 64;

/// How far outside a shadow the lit reference is taken from, in world units.
///
/// A third of a unit is a few pixels of frame at this camera, which is enough ring to take a
/// percentile from and narrow enough that the lamp's falloff across it is not the thing being
/// measured.
const RING_MARGIN: f64 = 0.35;

/// A shadow whose lit reference and darkest interior differ by less than this is not in the frame
/// at all, and is left out of every ratio rather than counted as a shadow of nothing.
const MINIMUM_SHADOW_SPREAD: f32 = 0.04;

/// How dark a floor pixel has to be, against its own row, to be called a full shadow.
const DARK_EDGE: f32 = 0.2;

/// How lit it has to be to be called floor rather than shadow.
const LIT_EDGE: f32 = 0.8;

/// How many of a lit pixel's eight neighbours must be fully shadowed before the lit pixel is
/// called a leak. Six rather than a majority, because one soft neighbour is the penumbra of a
/// shadow that is entirely legitimate, and a leak is a discontinuity rather than a soft edge.
const LEAK_NEIGHBOURS: u32 = 6;

/// One caster the probe places, and the resolution it asks its own shadow to be recorded at.
#[derive(Clone, Copy)]
struct Caster {
    /// Names the mesh, the material and the three placements, so it is one word in a report.
    label: &'static str,
    /// Its size, as width, height and depth, standing on the floor at its own origin.
    size: [f64; 3],
    /// Where it stands.
    at: [f64; 3],
    /// What it is painted, which is what tells two casters apart in a frame lit by one lamp.
    tint: [f64; 3],
    /// Texels a world unit of its shadow is recorded at, or none for the scene's own default.
    density: Option<f64>,
}

/// # Why every caster below stands where `spotlight`'s own boxes stand
///
/// The lamp throws shadows away from the camera, so a caster hides the start of its own shadow, and
/// a wide caster standing where a thin one would stand hides **all** of it. The second run of this
/// probe put a wall nine tenths of a unit thick in the middle of the pool and measured a thousand
/// dark pixels in a frame with no shadow on it at all: the dark pixels were the far rim of the
/// cone, and the wall's own body was standing in front of its shadow.
///
/// `spotlight` is the one scene in the tree whose casters are known to cast visibly from **this**
/// camera, because its frame has been looked at. Its three boxes stand at `(0.6, 0, -0.6)`,
/// `(2.6, 0, -1.9)` and `(-1.1, 0, -2.4)`, and the rod and the wall here stand on two of those spots
/// at two thicknesses. The shadow's **shape** is still computed from the geometry rather than
/// guessed, by [`shadow_footprint`]; what is borrowed is only the standing.
const ROD: Caster = Caster {
    label: "rod",
    size: [0.30, 3.4, 1.1],
    at: [0.6, 0.0, -0.6],
    tint: [1.0, 0.12, 0.12],
    density: None,
};

/// The wall: the same standing as the rod and rather more than twice the thickness, which is the
/// comparison the design is for — a thin caster and a thick one under one lamp, each asking for
/// its own resolution.
const WALL: Caster = Caster {
    label: "wall",
    size: [1.6, 1.3, 1.6],
    at: [2.2, 0.0, -1.9],
    tint: [0.12, 1.0, 0.12],
    density: None,
};

/// A caster that exists only to **hold the grid**, standing far to the left where the frame does
/// not reach.
///
/// # Why the rod is not good enough for this, and what moving it found
///
/// The level ladder needs a caster that sets the grid and then stands out of the way, because the
/// grid is sized by the highest density in the cone. The rod did set the grid — and its padded
/// level-zero pages, five across, reach far enough to cover the wall's shadow as well. The four
/// points of the ladder came back **byte for byte identical** while their pass counts fell from
/// seven hundred and eighty nine to four hundred and seventy seven.
///
/// The reason was the reading rule rather than an arithmetic slip. A caster is drawn into **the one
/// level its density asks for** and into no other, so the wall — three levels coarser than this
/// holder — is nowhere in the level-zero pages the holder's footprint opened, and those pages are
/// empty across the whole of the wall's shadow. The walk went from finest to coarsest and stopped at
/// the first page that *existed*, read that emptiness as "nothing blocks the light", and returned,
/// so the wall's own page was allocated, drawn and paid for and then read by nobody. Passing a level
/// through instead of answering from it is what makes one layer per caster enough.
const HOLDER: Caster = Caster {
    label: "holder",
    size: [0.30, 3.4, 0.8],
    at: [-3.4, 0.0, 0.6],
    tint: [0.45, 0.45, 0.45],
    density: Some(FINE_DENSITY),
};

/// The offstage caster, asked to stand at the third of `spotlight`'s box spots so that the question
/// is whether its **caster** can leave a camera's frame while its shadow stays in it.
const OFFSTAGE: Caster = Caster {
    label: "offstage",
    size: [0.70, 2.6, 0.70],
    at: [-1.1, 0.0, -2.4],
    tint: [0.12, 0.12, 1.0],
    density: None,
};

/// One probe's answer: what the documents said and what the pixels said.
struct Measurement {
    /// What was varied, in the words a reader of the report needs.
    label: String,
    /// How many passes the compile emitted, which is the cost the feature is bought with.
    passes: usize,
    /// The shape of the light's grid, or none when the compile was refused.
    grid: Option<Grid>,
    /// The refusal, when there was one. A refusal is a result and not a failure of the probe.
    refused: Option<String>,
    /// How much of the floor carried a shadow at all, in pixels.
    shadow_pixels: u64,
    /// How much of that was a partial shadow, which is the measure of resolution.
    soft_pixels: u64,
    /// How many floor pixels read lit inside a shadow, which is what a leak between two pages is.
    leaks: u64,
    /// How much floor was measured, so a ratio beside it reads as a fraction of something.
    floor_pixels: u64,
    /// A short fingerprint of the pixels, so "the picture did not change" is measured rather than
    /// inferred from two soft shares agreeing.
    fingerprint: String,
    /// What each measurable caster's own share of the shadow was.
    ///
    /// The totals above are the sum over casters, which is the wrong number to compare when the
    /// question is whether *one* caster's shadow survived a change to a neighbour: a frame that
    /// gains a caster changes the total for reasons that have nothing to do with it.
    casters: Vec<CasterShare>,
    /// Whether the offstage caster was in this camera's frame, when the scene placed one.
    caster_visible: Option<bool>,
    /// The casters this frame could not measure, each with why.
    ///
    /// A caster with no shadow and a caster whose shadow could not be measured are different
    /// answers, and a table that prints `0` for both cannot tell a reader which it is giving them.
    skipped: Vec<String>,
}

/// One caster's own share of a frame's shadow, counted over its own footprint.
struct CasterShare {
    /// The caster's name, as the probe names it.
    label: String,
    /// How much of its own shadow carried a shadow at all, in pixels.
    shadow: u64,
    /// How much of that was a partial shadow rather than a full one.
    soft: u64,
}

/// The shape one light's virtual map was given, read out of the page table it answered with.
struct Grid {
    /// Pages a side at the finest level.
    per_side: u32,
    /// How many levels of it exist.
    levels: u32,
    /// How many slots a side the atlas it was cut from has.
    atlas_side: u32,
    /// How many pages were actually allocated, which is fewer than the grid holds.
    allocated: u32,
}

/// One rendered probe, as a measurement and the frame behind it.
struct Rendered {
    /// What was measured.
    measurement: Measurement,
    /// The frame the numbers were taken from, or none when the compile was refused and there was
    /// nothing to draw.
    frame: Option<RenderedFrame>,
}

/// One floor pixel: how bright it is, and where on the floor it stands.
#[derive(Clone, Copy)]
struct FloorPixel {
    /// Its brightness, which is the one number a shadow is read by.
    luminance: f32,
    /// Where it stands on the floor, as `x` then `z`.
    at: (f32, f32),
}

/// One probe's floor, resolved once so that every measure reads the same pixels.
struct Floor {
    /// Every pixel of the frame, row major, and `None` where the frame shows something that is
    /// not floor.
    pixels: Vec<Option<FloorPixel>>,
    /// How many pixels a row holds.
    width: usize,
    /// How many pixels a frame is tall.
    height: usize,
}

impl Floor {
    /// Returns the floor pixel at this place, if the frame shows floor there.
    fn at(&self, x: i64, y: i64) -> Option<FloorPixel> {
        if x < 0 || y < 0 {
            return None;
        }
        self.pixels
            .get(y as usize * self.width + x as usize)
            .copied()
            .flatten()
    }

    /// Returns where the eight places around this one are, keeping only the ones the frame shows
    /// floor at.
    fn around(&self, x: i64, y: i64) -> Vec<(i64, i64)> {
        [
            (x - 1, y - 1),
            (x, y - 1),
            (x + 1, y - 1),
            (x - 1, y),
            (x + 1, y),
            (x - 1, y + 1),
            (x, y + 1),
            (x + 1, y + 1),
        ]
        .into_iter()
        .filter(|(column, row)| self.at(*column, *row).is_some())
        .collect()
    }
}

#[tokio::main]
async fn main() {
    if let Err(failure) = run().await {
        eprintln!("error: {failure}");
        std::process::exit(1);
    }
}

/// Runs every probe and prints what came back.
async fn run() -> Result<(), String> {
    let owned = rasterizer().await;
    let renderer: &dyn Renderer = &owned;

    let mut registry = Registry::new();
    registry.put_schema(
        SchemaIdentifier::new(GRAPHICS_SCHEMA),
        graphics::graphics_schema(),
    );
    registry.put_schema(
        SchemaIdentifier::new(INSTANCE_SCHEMA),
        graphics::instance_schema(),
    );
    scene::register_standard_scene_schema(&mut registry);

    // The `position` channel is asked for on **every** compile, because two of the probes ask
    // questions of the frame that are about where a surface is rather than how it is lit: the
    // floor a measurement is taken on, and whether a caster is in the camera's frame at all. It
    // costs one fullscreen pass and answers both without a guess at pixel coordinates.
    let compiler = StandardScene::new(
        DefinitionIdentifier::new("probe.graphics"),
        DefinitionIdentifier::new("probe.instance"),
    )
    .with_debug([DebugChannel::Position]);

    std::fs::create_dir_all(OUTPUT).map_err(|error| format!("{OUTPUT} cannot be made: {error}"))?;

    print!("{}", density_ladder(&compiler, &registry, &renderer));
    print!("{}", control_row(&compiler, &registry, &renderer));

    let mut results = Vec::new();
    for probe in [
        probe_coin_and_castle,
        probe_level_ladder,
        probe_budget_cliff,
        probe_page_seams,
        probe_camera_clipping,
        probe_caster_height,
        probe_level_walk,
    ] {
        for rendered in probe(&compiler, &registry, &renderer)? {
            if let Some(frame) = &rendered.frame {
                write_frame(&rendered.measurement.label, frame);
            }
            results.push(rendered.measurement);
        }
    }
    print!("{}", report(&results));

    Ok(())
}

/// Probe one: a coin and a castle under one lamp, each asking for its own resolution.
///
/// The frame that carries the claim is the last of the three, because a frame with two shadows in
/// it is the hardest to attribute and the numbers above it are each one caster's alone. **It
/// fails** if the rod's soft share does not fall when its density rises, or if the wall's soft
/// share moves at all when only the rod's density moved — which is the half of the design that
/// sizes one grid for the whole light, and the half nobody has looked at.
fn probe_coin_and_castle(
    compiler: &StandardScene,
    registry: &Registry,
    renderer: &dyn Renderer,
) -> Result<Vec<Rendered>, String> {
    let rod = Caster {
        density: Some(FINE_DENSITY),
        ..ROD
    };
    Ok(vec![
        render_one(
            compiler,
            registry,
            renderer,
            "probe1-rod-default",
            &[ROD],
            0,
        )?,
        render_one(compiler, registry, renderer, "probe1-rod-fine", &[rod], 0)?,
        render_one(
            compiler,
            registry,
            renderer,
            "probe1-castle",
            &[rod, WALL],
            0,
        )?,
    ])
}

/// Probe two: the same grid, and one caster walking down the levels.
///
/// The rod holds the top density, and the top density is what sizes the grid for the whole light,
/// so **the grid does not move** while the wall halves four times. That is the whole reason this
/// probe exists: an absolute ladder cannot separate the two mechanisms, because the grid side is a
/// power of two and three percent of density is a factor of four in pages.
///
/// Each point is drawn **twice**, from the square-on camera and from the low one, and that is not
/// belt and braces. At a thirty two page grid a page texel is already a hundredth of a world unit
/// at the finest level and a twelfth of one at the coarsest, which at this camera's forty pixels a
/// unit is anything from a fortieth of a pixel to a pixel and a half — so **the whole ladder can be
/// invisible square on**, and a probe that only looked there would report "the level does nothing"
/// from a frame too coarse to show it. The low camera stretches a texel along the receiver by
/// orders of magnitude and is the only one of the two that can tell a level from its neighbour.
///
/// The grid is held by a caster standing **eight pages away** rather than by one standing beside the
/// wall, for the reason [`HOLDER`] gives: a caster coarser than the holder is in **no** page the
/// holder opened, so the two things this probe exists to separate — which level a caster is recorded
/// at, and which level a receiver is answered from — are not the same question. **It fails** if the
/// four fingerprints are equal under both cameras, which would mean the level a caster is recorded at
/// does not reach its own shadow even with nothing else in range.
fn probe_level_ladder(
    compiler: &StandardScene,
    registry: &Registry,
    renderer: &dyn Renderer,
) -> Result<Vec<Rendered>, String> {
    let mut rendered = Vec::new();
    for wall in [
        FINE_DENSITY,
        FINE_DENSITY * 0.5,
        FINE_DENSITY * 0.25,
        COARSE_DENSITY,
    ] {
        let casters = [
            HOLDER,
            Caster {
                density: Some(wall),
                ..WALL
            },
        ];
        for (camera, view) in [(0, "square-on"), (1, "low")] {
            rendered.push(render_one(
                compiler,
                registry,
                renderer,
                &format!("probe2-wall-{}-{view}", wall as i64),
                &casters,
                camera,
            )?);
        }
    }
    Ok(rendered)
}

/// Probe three: what a density costs, rendered at three points of the ladder.
///
/// The whole ladder is compiled by [`density_ladder`] without being drawn, because a page count
/// does not need a frame and a refused compile costs nothing. **It fails** if the pages do not
/// rise with the density, or if a density the atlas cannot hold renders anyway rather than being
/// refused by name.
fn probe_budget_cliff(
    compiler: &StandardScene,
    registry: &Registry,
    renderer: &dyn Renderer,
) -> Result<Vec<Rendered>, String> {
    let mut rendered = Vec::new();
    for density in [5.0, COARSE_DENSITY, FINE_DENSITY, FINE_DENSITY * 2.0] {
        rendered.push(render_one(
            compiler,
            registry,
            renderer,
            &format!("probe3-rod-{}", density as i64),
            &[Caster {
                density: Some(density),
                ..ROD
            }],
            0,
        )?);
    }
    Ok(rendered)
}

/// Probe four: a shadow crossing page boundaries at a grazing angle.
///
/// The `viewport` that fills one page of the atlas and the clamp that keeps a PCF tap inside its
/// own page are both new, and a leak between two pages is a hairline that is invisible in a
/// picture and obvious in a count. The grazing camera stands low and close, which is what makes a
/// receiver's texels long and a page boundary most likely to be crossed by one receiver pixel's
/// worth of world. **It fails** if the grazing frame has lit pixels inside its shadow that the
/// square-on one does not.
fn probe_page_seams(
    compiler: &StandardScene,
    registry: &Registry,
    renderer: &dyn Renderer,
) -> Result<Vec<Rendered>, String> {
    Ok(vec![
        render_one(compiler, registry, renderer, "probe4-square-on", &[WALL], 0)?,
        render_one(compiler, registry, renderer, "probe4-grazing", &[WALL], 1)?,
    ])
}

/// Probe five: pages follow the camera, and a shadow does not.
///
/// The same scene from four places: the one every other probe uses, one standing low, one looking
/// away from all of it, and one standing to the side. What has to hold is that a camera which sees
/// nothing allocates nothing, and that a caster outside the frame still casts into it. The second
/// half is reported rather than assumed, because the offstage caster's visibility is read out of
/// the `position` channel: a camera that sees it is a probe that tested nothing, and says so.
fn probe_camera_clipping(
    compiler: &StandardScene,
    registry: &Registry,
    renderer: &dyn Renderer,
) -> Result<Vec<Rendered>, String> {
    let casters = [OFFSTAGE, WALL];
    Ok(vec![
        render_one(compiler, registry, renderer, "probe5-camera-0", &casters, 0)?,
        render_one(
            compiler,
            registry,
            renderer,
            "probe5-camera-low",
            &casters,
            1,
        )?,
        render_one(
            compiler,
            registry,
            renderer,
            "probe5-camera-away",
            &casters,
            2,
        )?,
        render_one(
            compiler,
            registry,
            renderer,
            "probe5-camera-side",
            &casters,
            3,
        )?,
    ])
}

/// Compiles the whole absolute ladder and reports what each density cost, drawing none of them.
///
/// The refusals are the point of the sweep: the design's claim is that an atlas too small is a
/// compile error naming both numbers rather than a silently coarser shadow, and a claim about a
/// message is checked by compiling rather than by looking at a picture.
fn density_ladder(
    compiler: &StandardScene,
    registry: &Registry,
    renderer: &dyn Renderer,
) -> String {
    let mut report = String::from(
        r#"
# the density ladder, compiled and not drawn

The grid side is a power of two, so this is a staircase rather than a ramp: 58.07 is the
resolution the old pool gave, and the first step past it already quadruples the pyramid.

"#,
    );
    report.push_str(
        r#"  density   pages/side   levels   pages   atlas   passes   outcome
  ---------   ----------   ------   -----   -----   ------   -------
"#,
    );
    for density in [
        5.0,
        15.0,
        COARSE_DENSITY,
        DEFAULT_DENSITY,
        FINE_DENSITY * 0.5,
        FINE_DENSITY,
        FINE_DENSITY * 2.0,
        FINE_DENSITY * 4.0,
    ] {
        let scene = probe_scene(&[Caster {
            density: Some(density),
            ..ROD
        }]);
        let outcome = match compile(compiler, registry, renderer, &scene, 0) {
            Ok(compiled) => {
                let passes = count_passes(&compiled);
                match read_grid(&compiled) {
                    Some(grid) => format!(
                        "{:>9}   {:>10}   {:>6}   {:>5}   {:>5}   {:>6}   compiled",
                        density as i64,
                        grid.per_side,
                        grid.levels,
                        grid.allocated,
                        grid.atlas_side,
                        passes
                    ),
                    None => format!(
                        "{:>9}   {:>10}   {:>6}   {:>5}   {:>5}   {:>6}   no page reached this camera",
                        density as i64, "-", "-", "-", "-", passes
                    ),
                }
            }
            Err(refusal) => format!(
                "{:>9}   {:>10}   {:>6}   {:>5}   {:>5}   {:>6}   refused: {refusal}",
                density as i64, "-", "-", "-", "-", "-"
            ),
        };
        report.push_str(&format!("{outcome}\n"));
    }
    report
}

/// Returns what the gallery's own spot-lit scene allocates, as the control every number in this
/// file is read against.
///
/// `spotlight` is the one scene in the tree whose virtual shadow was measured before this work —
/// eighty five pages, an atlas ten to a side, and a refusal three percent finer — so a probe whose
/// numbers cannot be reconciled with it is measuring something other than what it says. It is
/// compiled and not drawn, because what is being reconciled is arithmetic.
fn control_row(compiler: &StandardScene, registry: &Registry, renderer: &dyn Renderer) -> String {
    let scene = Gallery::build("spotlight");
    let mut report = String::from(
        r#"
# the control: `spotlight`, the one scene whose virtual shadow was measured before this

"#,
    );
    match compile(compiler, registry, renderer, &scene, 0) {
        Ok(compiled) => match read_grid(&compiled) {
            Some(grid) => report.push_str(&format!(
                "  {} pages a side, {} levels, {} pages allocated, an atlas {} to a side, {} \
                 passes\n",
                grid.per_side,
                grid.levels,
                grid.allocated,
                grid.atlas_side,
                count_passes(&compiled)
            )),
            None => report.push_str("  it carries no page table at all\n"),
        },
        Err(refusal) => report.push_str(&format!("  it is refused: {refusal}\n")),
    }
    report
}

/// Probe six: the height at which a caster stops casting.
///
/// # The bug this was built for, and what is left of it
///
/// `shadowed_pages` samples the shadow at the receiver pushed along its own normal so that a
/// surface does not shadow itself, and the push used to be a constant in world space. A constant is
/// not a bias: at one and a half units it put the lookup point that far into the air, and **a caster
/// shorter than the push was not between the lamp and the point being sampled** — so its shadow was
/// not soft, or faint, or found at the wrong level. It was simply not there, and the frame said so
/// with a hard edge where a soft one belonged. The push is one texel of the recording now, measured
/// from the light's own projection, so no height is below it.
///
/// The ladder is kept because the measurement is the evidence: a 1.3-high caster casting a complete
/// hard-edged shadow is the fact that the push is a texel and not a world constant, and it is the
/// one row that would move first if the push ever became a constant again.
///
/// **It fails** if the frames are byte identical, which would mean the height no longer reaches the
/// recording at all.
fn probe_caster_height(
    compiler: &StandardScene,
    registry: &Registry,
    renderer: &dyn Renderer,
) -> Result<Vec<Rendered>, String> {
    let mut rendered = Vec::new();
    for height in [1.3, 1.7, 2.1, 3.4] {
        rendered.push(render_one(
            compiler,
            registry,
            renderer,
            &format!("probe6-wall-height-{}", (height * 10.0) as i64),
            &[Caster {
                size: [1.6, height, 1.6],
                at: WALL.at,
                ..WALL
            }],
            0,
        )?);
    }
    Ok(rendered)
}

/// The slab: a coarse caster large enough to measure, standing where [`WALL`] stands.
///
/// Wider and taller than the wall because the wall's own shadow is 48 pixels in frame and a
/// measurement needs 64. The height is the one probe six showed to be inside the cone, and the
/// width is what makes the footprint larger than that floor.
const SLAB: Caster = Caster {
    label: "slab",
    size: [2.2, 2.1, 2.2],
    at: [2.2, 0.0, -1.9],
    tint: [1.0, 0.55, 0.05],
    density: Some(COARSE_DENSITY),
};

/// Probe seven: a coarse caster whose shadow lands inside a finer caster's pages.
///
/// # What it separates
///
/// [`probe_level_ladder`] varies the density of a caster while the grid is held by another, and it
/// cannot tell a shadow that was lost from a shadow that was recorded coarsely. This one holds the
/// **slab** fixed — the same density, the same size, the same spot, in all three frames — and varies
/// only the neighbour standing next to it, so every difference between the frames is a difference in
/// what the neighbour did to the slab's shadow.
///
/// Three frames, and the middle one is the control that makes the outer two mean something:
///
/// - **alone.** Nothing sets the grid but the slab, so the slab is at level zero and the walk finds
///   its page on the first step. This is the reference: the shadow the slab is owed.
/// - **beside a coarse neighbour.** The neighbour asks for the slab's own density, so it is drawn
///   into the same pages the slab is, neither page is empty, and the slab's share must be the
///   reference's. A difference here would be the walk itself and nothing to do with the levels.
/// - **under a fine neighbour.** The neighbour asks for the finest density, so it opens pages the
///   slab is in no part of, and those pages are empty across the whole of the slab's shadow. The
///   walk reads them, finds nothing shadowing, and goes on to the level the slab is in.
///
/// **It fails** if the slab's own share is smaller under the fine neighbour than it is beside the
/// coarse one, and **it fails** if the coarse neighbour moved it at all. The first is a level that
/// answered about a caster it does not contain; the second is a walk that answers somewhere it
/// should not have.
fn probe_level_walk(
    compiler: &StandardScene,
    registry: &Registry,
    renderer: &dyn Renderer,
) -> Result<Vec<Rendered>, String> {
    let coarse_neighbour = Caster {
        label: "neighbour-coarse",
        density: Some(COARSE_DENSITY),
        ..HOLDER
    };
    Ok(vec![
        render_one(
            compiler,
            registry,
            renderer,
            "probe7-slab-alone",
            &[SLAB],
            0,
        )?,
        render_one(
            compiler,
            registry,
            renderer,
            "probe7-slab-beside-coarse",
            &[coarse_neighbour, SLAB],
            0,
        )?,
        render_one(
            compiler,
            registry,
            renderer,
            "probe7-slab-under-fine",
            &[HOLDER, SLAB],
            0,
        )?,
    ])
}

/// Builds the scene one probe renders.
///
/// # Invariants
///
/// - **Every drawn thing is a caster, the floor included.** A receiver that is not drawn into the
///   shadow pages is a receiver whose shadow is missing wherever something stands between it and
///   the lamp.
/// - **Only the shadowing placement carries a density.** The same box is drawn three times, and
///   the number says how its shadow is *recorded* rather than how it is drawn, so a density on
///   either of the other two rows would be a field nothing reads.
/// - **The four cameras are declared in the order the probes index them**, and the first is the
///   one every other measurement is made against.
fn probe_scene(casters: &[Caster]) -> Value {
    let mut shaders = mesh_shaders("", "");
    shaders.push(post_shader(GRADE, ""));

    let mut materials = vec![
        mesh_material("caster", "shadowing", surface([1.0; 3], 0.0, 1.0, [0.0; 3])),
        mesh_material(
            "occluder",
            "depthing",
            surface([1.0; 3], 0.0, 1.0, [0.0; 3]),
        ),
        mesh_material("floor", "lighting", surface(FLOOR_TINT, 0.0, 1.0, [0.0; 3])),
    ];
    let mut meshes = vec![mesh(
        "floor",
        "Triangles",
        shaded_point_schema(),
        plane_facing(FLOOR_EXTENT, 0.0, true),
    )];
    for caster in casters {
        materials.push(mesh_material(
            caster.label,
            "lighting",
            surface(caster.tint, 0.0, 1.0, [0.0; 3]),
        ));
        meshes.push(mesh(
            caster.label,
            "Triangles",
            shaded_point_schema(),
            cuboid(caster.size[0], caster.size[1], caster.size[2]),
        ));
    }

    // The two stages that only *place* a mesh name one material for the whole scene, because a
    // caster's row carries nothing a shadow page or a depth buffer can use; the lighting stage
    // has no such question, because there every mesh is named after its own tinted material.
    //
    // **The floor is drawn as a caster at a density of zero, and that is the whole reason the
    // probes can measure anything.** A caster that names no density is given the scene's default,
    // and the default is high, so a floor eighteen units across would demand a grid sized for
    // eighteen units of shadow resolution — the receiver would size the map for every caster in
    // the scene. Zero is the documented way out: it puts the caster in the coarsest level only and
    // excludes it from the density the grid is sized against, so the floor still casts (a caster
    // that casts nothing is a receiver whose shadows are missing) while the casters are measured
    // against the resolution they asked for. The first run of this probe had the floor at the
    // default and every ladder point from 58 up was refused for pages the floor had asked for.
    let placements: Vec<(&str, [f64; 3])> = std::iter::once(("floor", [0.0; 3]))
        .chain(casters.iter().map(|caster| (caster.label, caster.at)))
        .collect();
    let density_of = |label: &str| -> Option<f64> {
        if label == "floor" {
            return Some(0.0);
        }
        casters
            .iter()
            .find(|caster| caster.label == label)
            .and_then(|caster| caster.density)
    };
    let placed = |stage: &str, label: &str, at: [f64; 3], material: &str| -> Value {
        instance(
            format!("{stage}-{label}"),
            label,
            at,
            [0.0; 3],
            [1.0; 3],
            material,
        )
    };

    Value::structure([
        ("name".to_owned(), Value::String("shadow-probe".to_owned())),
        // The atlas ceiling is raised well above the sixteen pages a side the pipeline asks for by
        // default, because a probe whose finest density is refused measures the ceiling rather than
        // the resolution. It is a **ceiling and not a size**: the atlas is still no bigger than the
        // pages a light was actually given, so raising it costs nothing in texture and is the one
        // number the design's own refusal message tells an author to raise.
        (
            "frame".to_owned(),
            Value::structure([(
                SHADOW_ATLAS_SIDE_PAGES.to_owned(),
                Value::Int(ATLAS_SIDE_PAGES),
            )]),
        ),
        ("cameras".to_owned(), Value::List(cameras())),
        (
            "lights".to_owned(),
            Value::List(vec![spot(
                "lamp",
                LAMP_POSITION,
                LAMP_DIRECTION,
                LAMP_COLOUR,
                LAMP_INTENSITY,
                LAMP_INNER,
                LAMP_OUTER,
                LAMP_REACH,
            )]),
        ),
        ("shaders".to_owned(), Value::List(shaders)),
        ("meshes".to_owned(), Value::List(meshes)),
        ("mesh_materials".to_owned(), Value::List(materials)),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("post", "post")]),
        ),
        ("post_processes".to_owned(), post_processes(&["post"])),
        (
            "instances".to_owned(),
            instances(vec![
                (
                    Stage::Shadowing,
                    placements
                        .iter()
                        .map(|(label, at)| {
                            at_density(placed("shadowing", label, *at, "caster"), density_of(label))
                        })
                        .collect(),
                ),
                (
                    Stage::Depthing,
                    placements
                        .iter()
                        .map(|(label, at)| placed("depthing", label, *at, "occluder"))
                        .collect(),
                ),
                (
                    Stage::Lighting,
                    placements
                        .iter()
                        .map(|(label, at)| placed("lighting", label, *at, label))
                        .collect(),
                ),
            ]),
        ),
    ])
}

/// Returns the four cameras, in the order the probes index them.
///
/// The second stands low and close, which lengthens a receiver's texels; the third looks up and
/// away from the whole scene, which is the case that has to allocate nothing; and the fourth stands
/// to the side, where the floor is seen nearly edge on and a caster can leave the frame while its
/// shadow stays in it.
fn cameras() -> Vec<Value> {
    let declared = vec![
        camera_placed(EYE, TARGET, FIELD_OF_VIEW, NEAR, FAR),
        camera_placed(
            [-6.2, 1.15, 7.4],
            [0.6, 0.05, -0.4],
            FIELD_OF_VIEW,
            NEAR,
            FAR,
        ),
        camera_placed(
            [EYE[0] + 6.0, EYE[1] + 14.0, EYE[2] + 12.0],
            [EYE[0] + 7.0, EYE[1] + 20.0, EYE[2] + 16.0],
            FIELD_OF_VIEW,
            NEAR,
            FAR,
        ),
        camera_placed([-8.6, 3.4, 1.2], [0.4, 0.2, -1.0], FIELD_OF_VIEW, NEAR, FAR),
    ];
    assert_eq!(
        declared.len(),
        CAMERA_COUNT,
        "a probe indexes these by number, and a camera added here without one added to the count \
         would be a camera no probe can reach"
    );
    declared
}

/// Returns the placement with the resolution its shadow is to be recorded at.
fn at_density(placed: Value, density: Option<f64>) -> Value {
    let Some(density) = density else {
        return placed;
    };
    let Value::Struct(mut fields) = placed else {
        panic!("a placement is a structure of a label, a mesh, a transform and a material");
    };
    fields.insert(SHADOW_DENSITY.to_owned(), Value::Float(density));
    Value::Struct(fields)
}

/// Compiles one probe scene, renders it, and measures both the documents and the pixels.
fn render_one(
    compiler: &StandardScene,
    registry: &Registry,
    renderer: &dyn Renderer,
    label: &str,
    casters: &[Caster],
    camera: usize,
) -> Result<Rendered, String> {
    let scene = probe_scene(casters);
    // A refusal is a **result**: the design's claim is that an atlas too small is refused by name
    // rather than quietly drawn coarser, and a probe that treated it as a crash of the harness
    // would stop at the first one and never report the message that is the claim.
    let compiled = match compile(compiler, registry, renderer, &scene, camera) {
        Ok(compiled) => compiled,
        Err(refusal) => {
            return Ok(Rendered {
                measurement: Measurement {
                    label: label.to_owned(),
                    passes: 0,
                    grid: None,
                    refused: Some(refusal),
                    shadow_pixels: 0,
                    soft_pixels: 0,
                    leaks: 0,
                    floor_pixels: 0,
                    fingerprint: "--------".to_owned(),
                    casters: Vec::new(),
                    caster_visible: None,
                    skipped: Vec::new(),
                },
                frame: None,
            });
        }
    };

    let frame = RenderedFrame {
        renderer: renderer.name(),
        images: renderer
            .render_outputs(&compiled.graphics.value, &compiled.instance.value)
            .map_err(|error| format!("{label}: the frame does not render: {error}"))?,
    };
    let present = frame
        .present()
        .ok_or_else(|| format!("{label}: the frame carries no image declared as the present"))?;
    let positions = frame
        .images
        .iter()
        .find(|image| image.name == DebugChannel::Position.output())
        .ok_or_else(|| {
            format!(
                "{label}: the position channel was asked for and is not in the frame, so no \
                 measure below knows what surface it is looking at"
            )
        })?;

    let floor = resolve_floor(present, positions, casters);
    let (shadow_pixels, soft_pixels, leaks, casters_measured, skipped) =
        measure_shadow(&floor, casters);

    Ok(Rendered {
        measurement: Measurement {
            label: label.to_owned(),
            passes: count_passes(&compiled),
            grid: read_grid(&compiled),
            refused: None,
            shadow_pixels,
            soft_pixels,
            leaks,
            floor_pixels: floor.pixels.iter().filter(|pixel| pixel.is_some()).count() as u64,
            fingerprint: frame_fingerprint(&frame),
            casters: casters_measured,
            caster_visible: casters
                .iter()
                .find(|caster| caster.label == OFFSTAGE.label)
                .map(|caster| is_visible(positions, caster)),
            skipped,
        },
        frame: Some(frame),
    })
}

/// Compiles a scene document against the machine behind the renderer.
fn compile(
    compiler: &StandardScene,
    registry: &Registry,
    renderer: &dyn Renderer,
    scene: &Value,
    camera: usize,
) -> Result<CompileOutput, String> {
    let source = Definition::new(
        DefinitionIdentifier::new("probe.source"),
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        scene.clone(),
    );
    source
        .validate(registry)
        .map_err(|error| format!("the scene does not satisfy the scene schema: {error}"))?;
    compiler
        .compile(&CompileContext {
            scene: &source,
            previous_graphics: None,
            previous_instance: None,
            registry,
            camera,
            limits: renderer.limits(),
        })
        .map_err(|error| error.to_string())
}

/// Counts the passes a compile emitted, which is the cost the feature is bought with.
fn count_passes(compiled: &CompileOutput) -> usize {
    passes(&compiled.graphics.value).map_or(0, |listed| listed.len())
}

/// Decodes the light's page table out of the compiled instance document.
///
/// This is the reader from `crates/scene/tests/spot_lights.rs`, walked exactly as a fragment walks
/// it: the header word, the per-level section offsets, and then a base slot plus `ceil(pages / 32)`
/// mask words per row, one bit per page.
fn read_grid(compiled: &CompileOutput) -> Option<Grid> {
    let records = packed_words(compiled, LIGHT_RECORDS_ENTRY);
    let header = *records.get(VIRTUAL_PAGE_WORD)?;
    // A light whose every page fell outside the camera carries the sentinel rather than a table
    // offset, and a sentinel walked as an offset reads somebody else's shadow.
    if header < 0.0 {
        return None;
    }
    let header = header as usize;
    let shape = word(&records, header);
    let kinds = word(&records, header + 1);
    let per_side = unpack_low(shape);
    let atlas_side = unpack_high(shape).max(1);
    let levels = unpack_low(kinds);

    let mut allocated = 0;
    for level in 0..levels {
        let section = word(&records, header + 2 + level as usize) as usize;
        let across = pages_at_level(per_side, level);
        let words = 1 + across.div_ceil(32) as usize;
        for row in 0..across {
            let start = header + section + row as usize * words;
            for mask in 1..words {
                allocated += word(&records, start + mask).count_ones();
            }
        }
    }
    Some(Grid {
        per_side,
        levels,
        atlas_side,
        allocated,
    })
}

/// Returns every float of a packed instance run, in the order the compiler wrote them.
fn packed_words(compiled: &CompileOutput, entry: &str) -> Vec<f32> {
    let bytes = compiled
        .instance
        .value
        .as_field_map()
        .and_then(|fields| fields.get(ENTRIES))
        .and_then(Value::as_dict)
        .and_then(|entries| entries.get(entry))
        .and_then(Value::variant_fields)
        .and_then(|fields| fields.get("data"))
        .and_then(Value::as_bytes)
        .unwrap_or_else(|| {
            panic!("{entry} is packed into the instance document, and it has no data field")
        });
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes(chunk.try_into().expect("a word is four bytes")))
        .collect()
}

/// Returns one word of a packed run, as the bit pattern the shader's own `bitcast` sees.
fn word(words: &[f32], at: usize) -> u32 {
    words
        .get(at)
        .unwrap_or_else(|| {
            panic!(
                "the run holds {} words and this table asked for the {at}th",
                words.len()
            )
        })
        .to_bits()
}

/// Resolves which pixels of the frame are floor, once, so every measure reads the same pixels.
///
/// The floor is named by the `position` channel rather than by a rectangle in the frame, because a
/// rectangle has to be re-guessed for every camera and would silently start measuring the sky. A
/// pixel standing on a caster's own footprint is left out: a caster's base is coincident with the
/// floor it stands on, and its own colour is not a shadow.
fn resolve_floor(present: &RenderedImage, positions: &RenderedImage, casters: &[Caster]) -> Floor {
    let width = present.width as usize;
    let height = present.height as usize;
    let mut pixels = vec![None; width * height];
    for y in 0..height {
        for x in 0..width {
            let (Ok(column), Ok(row)) = (u32::try_from(x), u32::try_from(y)) else {
                continue;
            };
            // The fourth component of a built-in channel is how it says no surface was drawn.
            let Some(where_it_is) = positions.texel(column, row) else {
                continue;
            };
            if where_it_is[3] < 0.5 || where_it_is[1].abs() > 1e-3 {
                continue;
            }
            // The channel reports world positions as `f32` and a caster is placed in `f64`, so the
            // comparison is made in the channel's own precision: narrowing the placement is the
            // honest direction, because a texel of the frame is already the smaller of the two.
            let under_a_caster = casters.iter().any(|caster| {
                (where_it_is[0] - caster.at[0] as f32).abs() < caster.size[0] as f32 * 0.5 + 0.05
                    && (where_it_is[2] - caster.at[2] as f32).abs()
                        < caster.size[2] as f32 * 0.5 + 0.05
            });
            if under_a_caster {
                continue;
            }
            pixels[y * width + x] = present.texel(column, row).map(|colour| FloorPixel {
                luminance: luminance(colour),
                at: (where_it_is[0], where_it_is[2]),
            });
        }
    }
    Floor {
        pixels,
        width,
        height,
    }
}

/// Returns the perceptual brightness of a colour, which is the one number a shadow is read by.
fn luminance(colour: [f32; 4]) -> f32 {
    0.2126 * colour[0] + 0.7152 * colour[1] + 0.0722 * colour[2]
}

/// Returns whether any visible pixel stands on the caster, which is how a probe says its own
/// setup did not test what it meant to test.
fn is_visible(positions: &RenderedImage, caster: &Caster) -> bool {
    for y in 0..positions.height {
        for x in 0..positions.width {
            let Some(where_it_is) = positions.texel(x, y) else {
                continue;
            };
            if where_it_is[3] < 0.5 {
                continue;
            }
            let inside = (where_it_is[0] - caster.at[0] as f32).abs() < caster.size[0] as f32 * 0.5
                && (where_it_is[2] - caster.at[2] as f32).abs() < caster.size[2] as f32 * 0.5
                && where_it_is[1] > -1e-3
                && where_it_is[1] < caster.size[1] as f32 + 1e-3;
            if inside {
                return true;
            }
        }
    }
    false
}

/// The four floor points one caster's shadow occupies, found by projecting the top of its box from
/// the lamp down onto the floor.
///
/// # Invariants
///
/// - **The top of the box is what casts.** A box is a straight prism, so the face the lamp sees is
///   its top, and projecting that face's four corners is the whole of the shadow's shape. Nothing
///   else about the box reaches the floor.
/// - **The ray goes past the caster, never back toward the lamp.** `t` is the lamp's height over the
///   drop from the lamp to the face, so it exceeds one exactly when the lamp is above the face —
///   which is the only case in which a box casts anything at all, and the only one a probe puts a
///   box in.
///
/// This is the whole reason the measurement below needs no threshold to find a shadow: the region
/// a caster shadows is arithmetic, and only the *softness* of what happens inside it is a question
/// about pixels.
fn shadow_footprint(caster: &Caster) -> [(f64, f64); 4] {
    let [width, height, depth] = caster.size;
    let centre_x = caster.at[0];
    let centre_z = caster.at[2];
    let corners = [
        (centre_x - width * 0.5, centre_z - depth * 0.5),
        (centre_x + width * 0.5, centre_z - depth * 0.5),
        (centre_x + width * 0.5, centre_z + depth * 0.5),
        (centre_x - width * 0.5, centre_z + depth * 0.5),
    ];
    let [lamp_x, lamp_y, lamp_z] = LAMP_POSITION;
    corners.map(|(corner_x, corner_z)| {
        let t = lamp_y / (lamp_y - height);
        (
            lamp_x + t * (corner_x - lamp_x),
            lamp_z + t * (corner_z - lamp_z),
        )
    })
}

/// Returns whether a floor point lies inside a convex quadrilateral.
///
/// Every corner is asked the same way and the answers must agree, which is the test for a convex
/// polygon: a point outside fails one side and inside passes all four. The corners come from a
/// projection of a rectangle, so the quadrilateral is convex by construction and no winding order
/// has to be trusted.
fn inside_quad(point: (f32, f32), quad: [(f64, f64); 4]) -> bool {
    let sides = (0..4).map(|index| {
        let (from_x, from_z) = quad[index];
        let (to_x, to_z) = quad[(index + 1) % 4];
        let edge_x = to_x - from_x;
        let edge_z = to_z - from_z;
        edge_x * (f64::from(point.1) - from_z) - edge_z * (f64::from(point.0) - from_x)
    });
    let mut sides = sides.collect::<Vec<f64>>();
    // A point exactly on the boundary answers zero for one side, and a sign test that insisted on
    // strictness would call the shadow's own edge unshadowed.
    sides.retain(|side| *side != 0.0);
    sides.iter().all(|side| *side > 0.0) || sides.iter().all(|side| *side < 0.0)
}

/// Returns the floor pixels lying just **outside** one caster's shadow, which is where the lit
/// reference for that shadow comes from.
///
/// # Why the reference cannot come from inside the shadow
///
/// A shadow is a dark region, so the brightest floor inside it is still shadowed, and a threshold
/// placed between the region's own darkest and brightest measures the lamp's falloff across the
/// band rather than the contrast of the edge — every run of the first version of this probe came
/// back reporting more than a hundred percent of the shadow as partial, which is what a
/// two-percent gradient between the fifth and ninety-fifth percentile of a fully shadowed region
/// looks like when it is asked to be an edge.
///
/// The floor immediately beside a shadow is the right reference and is nearly free of the falloff
/// that broke the other choice: it is the same surface at almost the same distance from the lamp.
/// The margin is a third of a world unit, which is a few pixels of frame at this camera and enough
/// of a ring to take a percentile from.
fn lit_reference(floor: &Floor, footprint: [(f64, f64); 4], margin: f64) -> Option<f32> {
    let mut ring: Vec<f32> = Vec::new();
    for y in 0..floor.height as i64 {
        for x in 0..floor.width as i64 {
            let Some(pixel) = floor.at(x, y) else {
                continue;
            };
            let distance = distance_to_quad(pixel.at, footprint);
            if distance > 0.0 && distance <= margin {
                ring.push(pixel.luminance);
            }
        }
    }
    if ring.len() < MINIMUM_RING_PIXELS {
        return None;
    }
    ring.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    Some(ring[ring.len() * 9 / 10])
}

/// Returns how far a floor point is from a convex quadrilateral, zero inside it.
fn distance_to_quad(point: (f32, f32), quad: [(f64, f64); 4]) -> f64 {
    let point = (f64::from(point.0), f64::from(point.1));
    (0..4)
        .map(|index| {
            let (from_x, from_z) = quad[index];
            let (to_x, to_z) = quad[(index + 1) % 4];
            let (edge_x, edge_z) = (to_x - from_x, to_z - from_z);
            let length = (edge_x * edge_x + edge_z * edge_z).sqrt();
            if length == 0.0 {
                return f64::INFINITY;
            }
            // How far along the edge the point's projection falls, clamped to the edge itself, so
            // a point beyond a corner is measured to that corner rather than to the line.
            let along = (((point.0 - from_x) * edge_x + (point.1 - from_z) * edge_z)
                / (length * length))
                .clamp(0.0, 1.0);
            let (near_x, near_z) = (from_x + along * edge_x, from_z + along * edge_z);
            ((point.0 - near_x).powi(2) + (point.1 - near_z).powi(2)).sqrt()
        })
        .fold(f64::INFINITY, f64::min)
}

/// Measures every caster's shadow on the floor: how much of it there is, how much of it is a
/// partial, and how much of it is lit.
///
/// Each caster is measured **inside its own shadow**, found by arithmetic rather than by a
/// threshold, and against the lit floor beside it. The two halves are what make the ratio mean
/// something: a shadow is a small region with a large internal contrast against its surroundings,
/// which is the one shape in which "how much of this is a partial" is a fact about resolution
/// rather than about the lamp.
fn measure_shadow(
    floor: &Floor,
    casters: &[Caster],
) -> (u64, u64, u64, Vec<CasterShare>, Vec<String>) {
    let mut shadow_pixels = 0;
    let mut soft_pixels = 0;
    let mut leaks = 0;
    let mut shares = Vec::new();
    let mut skipped = Vec::new();
    for caster in casters {
        let footprint = shadow_footprint(caster);
        if let Some(reason) = outside_the_cone(caster) {
            skipped.push(format!("{}: {}", caster.label, reason));
            continue;
        }
        let Some(lit) = lit_reference(floor, footprint, RING_MARGIN) else {
            skipped.push(format!(
                "{}: no lit floor beside its shadow to measure it against",
                caster.label
            ));
            continue;
        };
        let region: Vec<(i64, i64, f32)> = (0..floor.height as i64)
            .flat_map(|y| (0..floor.width as i64).map(move |x| (x, y)))
            .filter_map(|(x, y)| {
                let pixel = floor.at(x, y)?;
                inside_quad(pixel.at, footprint).then_some((x, y, pixel.luminance))
            })
            .collect();
        if region.len() < MINIMUM_SHADOW_PIXELS {
            skipped.push(format!(
                "{}: {} pixels of its shadow are in frame, and {} are needed to measure one",
                caster.label,
                region.len(),
                MINIMUM_SHADOW_PIXELS
            ));
            continue;
        }
        let mut ordered: Vec<f32> = region.iter().map(|(_, _, value)| *value).collect();
        ordered.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
        let darkest = ordered[ordered.len() / 20];
        if lit - darkest < MINIMUM_SHADOW_SPREAD {
            skipped.push(format!(
                "{}: its shadow is not in this frame ({} of brightness across it)",
                caster.label,
                lit - darkest
            ));
            continue;
        }
        let dark_edge = darkest + DARK_EDGE * (lit - darkest);
        let lit_edge = darkest + LIT_EDGE * (lit - darkest);
        let mut own_shadow = 0;
        let mut own_soft = 0;
        for (_, _, value) in &region {
            if *value <= dark_edge {
                shadow_pixels += 1;
                own_shadow += 1;
            } else if *value < lit_edge {
                soft_pixels += 1;
                own_soft += 1;
            }
        }
        shares.push(CasterShare {
            label: caster.label.to_owned(),
            shadow: own_shadow,
            soft: own_soft,
        });
        leaks += count_leaks(floor, &region, dark_edge, lit_edge);
    }
    (shadow_pixels, soft_pixels, leaks, shares, skipped)
}

/// Returns why a caster's shadow cannot be lit at all, which is a statement about the lamp rather
/// than about the frame.
///
/// # Why this is asked before anything is measured
///
/// A tall caster throws its shadow far from the lamp, and past the outer angle there is no light,
/// so there is no shadow to find however the frame is drawn. The first version of this harness
/// discovered that by accident: it reported **zero** shadow pixels for a caster whose shadow is
/// plainly visible, because the analytic region it measured was thirteen units from the lamp and
/// the harness read the absence as the answer. **A measurement that cannot distinguish "there is
/// nothing there" from "there is nothing to measure" is worse than no measurement**, and a zero in
/// this table is exactly that ambiguity.
fn outside_the_cone(caster: &Caster) -> Option<String> {
    let [lamp_x, lamp_y, lamp_z] = LAMP_POSITION;
    let t = lamp_y / (lamp_y - caster.size[1]);
    let centre = (
        lamp_x + t * (caster.at[0] - lamp_x),
        lamp_z + t * (caster.at[2] - lamp_z),
    );
    let from_lamp = [centre.0 - lamp_x, -lamp_y, centre.1 - lamp_z];
    let length = from_lamp.iter().map(|axis| axis * axis).sum::<f64>().sqrt();
    if length == 0.0 {
        return None;
    }
    let direction = LAMP_DIRECTION;
    let direction_length = direction.iter().map(|axis| axis * axis).sum::<f64>().sqrt();
    let cosine = from_lamp
        .iter()
        .zip(direction.iter())
        .map(|(axis, along)| axis * along)
        .sum::<f64>()
        / (length * direction_length);
    // An angle rather than a cosine, because the outer angle is the one number the scene states.
    let angle = cosine.clamp(-1.0, 1.0).acos();
    (angle > LAMP_OUTER).then(|| {
        format!(
            "its shadow lands {:.1}° from the lamp, past the {:.1}° the cone opens",
            angle.to_degrees(),
            LAMP_OUTER.to_degrees()
        )
    })
}

/// Returns how many lit pixels of one shadow are ringed by that same shadow, which is what a page
/// seam leaves behind.
///
/// A page boundary that lets the receiver through produces lit ground in the middle of a shadow, so
/// what is counted is a lit pixel whose neighbours are mostly fully shadowed **in the region being
/// measured** — not in the frame, because a lit pixel at the edge of the region has lit neighbours
/// by definition and would be counted as a leak by every shadow. The neighbourhood is two
/// dimensional, which is the whole reason this cannot be counted row by row: a seam runs across
/// rows, and a one dimensional test would find the lit pixel and none of its neighbours' state.
fn count_leaks(floor: &Floor, region: &[(i64, i64, f32)], dark_edge: f32, lit_edge: f32) -> u64 {
    let mut leaks = 0;
    for (x, y, value) in region {
        if *value <= lit_edge {
            continue;
        }
        let mut shadowed = 0;
        for (column, row) in floor.around(*x, *y) {
            if floor
                .at(column, row)
                .is_some_and(|neighbour| neighbour.luminance <= dark_edge)
            {
                shadowed += 1;
            }
        }
        if shadowed >= LEAK_NEIGHBOURS {
            leaks += 1;
        }
    }
    leaks
}

/// Returns a short fingerprint of the frame's pixels, as eight hexadecimal digits.
///
/// # Why this is FNV and not a cryptographic hash
///
/// The question is one of two: **did the picture change at all** when a density moved. That is a
/// change detector, and a sixty four bit FNV over four million pixels puts a collision between two
/// frames of one run far below anything a reader would act on. `render.rs` carries a full SHA-256
/// because it compares frames across runs and across machines; a probe comparing the frames of one
/// run does not need two hundred lines of FIPS 180-4 to answer the question, and the column is
/// here so that "the level ladder changed nothing" is a **measured** statement rather than one
/// inferred from two soft shares agreeing to the digit.
fn frame_fingerprint(frame: &RenderedFrame) -> String {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    const DIGITS: [u8; 16] = *b"0123456789abcdef";
    let Some(present) = frame.present() else {
        return "--------".to_owned();
    };
    let Some(pixels) = present.as_frame() else {
        return "--------".to_owned();
    };
    let mut hash = OFFSET;
    for byte in [pixels.width.to_le_bytes(), pixels.height.to_le_bytes()]
        .into_iter()
        .flatten()
        .chain(pixels.rgba.iter().copied())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(PRIME);
    }
    let mut out = String::with_capacity(8);
    for index in 0..8 {
        out.push(DIGITS[((hash >> (28 - index * 4)) & 0xF) as usize] as char);
    }
    out
}

/// Writes one frame beside the report.
fn write_frame(label: &str, frame: &RenderedFrame) {
    let path = format!("{OUTPUT}/{label}.png");
    match frame.to_png() {
        Ok(png) => {
            if let Err(error) = std::fs::write(&path, png) {
                eprintln!("note: {path} does not write: {error}");
            }
        }
        Err(error) => eprintln!("note: {label} does not encode: {error}"),
    }
}

/// Returns the report, one line per frame, with the columns a reader compares down.
fn report(results: &[Measurement]) -> String {
    let mut report = String::from(
        r#"
# what each frame measured

`soft` is the share of the shadow that is a partial rather than a full one, and it is the
resolution: a coarser page makes a wider edge, so a rod that asks for more density should show a
**smaller** number. `shadow` is the extent, and it is the one that must not move — the chain of
levels is meant to change how sharp a shadow is and never whether there is one. `leaks` is lit
floor ringed by shadow, which is what a page boundary that lets the receiver through leaves.

A caster this frame could not measure says so on the line below its own, because a caster with no
shadow and a caster whose shadow cannot be measured are different answers and a `0` that does not
say which one it is cannot be read.

"#,
    );
    report.push_str(
        r#"  frame                    passes   pages/side   pages   atlas   shadow px    soft   leaks   floor px   pixels
  ----------------------   ------   ----------   -----   -----   ---------   -----   -----   --------   --------
"#,
    );
    for measurement in results {
        let (per_side, pages, atlas) = match &measurement.grid {
            Some(grid) => (
                grid.per_side.to_string(),
                grid.allocated.to_string(),
                grid.atlas_side.to_string(),
            ),
            None => ("-".to_owned(), "-".to_owned(), "-".to_owned()),
        };
        let soft = if measurement.shadow_pixels == 0 {
            "-".to_owned()
        } else {
            format!(
                "{:.1}%",
                100.0 * measurement.soft_pixels as f64 / measurement.shadow_pixels as f64
            )
        };
        let offstage = match measurement.caster_visible {
            Some(true) => "   the offstage caster is in this frame, so it tested nothing",
            Some(false) => "   the offstage caster is out of this frame",
            None => "",
        };
        report.push_str(&format!(
            "  {:<22}   {:>6}   {:>10}   {:>5}   {:>5}   {:>9}   {:>5}   {:>5}   {:>8}   {}{offstage}\n",
            measurement.label,
            measurement.passes,
            per_side,
            pages,
            atlas,
            measurement.shadow_pixels,
            soft,
            measurement.leaks,
            measurement.floor_pixels,
            measurement.fingerprint,
        ));
        for reason in &measurement.skipped {
            report.push_str(&format!("{:<23}   not measured: {reason}\n", ""));
        }
        for share in &measurement.casters {
            report.push_str(&format!(
                "{:<23}   {}: {} px shadow, {} px soft\n",
                "", share.label, share.shadow, share.soft
            ));
        }
    }

    let refusals: Vec<&Measurement> = results
        .iter()
        .filter(|measurement| measurement.refused.is_some())
        .collect();
    if !refusals.is_empty() {
        report.push_str("\n# what was refused, and in what words\n\n");
        for measurement in refusals {
            report.push_str(&format!(
                "  {}: {}\n",
                measurement.label,
                measurement.refused.as_deref().unwrap_or("refused")
            ));
        }
    }
    report
}
