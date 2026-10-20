//! Flooded rice terraces under a morning sun: a Chinese mountain valley in spring.
//!
//! The scene is one heightfield, a village, and a stand of China firs, and what makes it
//! read as a photograph of a place rather than as terrain is four decisions, all of them
//! the scene's rather than the pipeline's:
//!
//! - **The contours are the subject.** A terrace is a hillside with its height quantised,
//!   so the same smooth noise that makes the mountain makes the paddy levels: each level
//!   follows a contour of the land it was cut into. The riser between levels is kept
//!   steep but smooth, because the heightfield's normals are taken from the height
//!   function itself and a hard step would shade as a torn edge.
//! - **Water is a mirror, not a colour.** A flooded paddy shows the sky. The flats the
//!   quantisation makes are shaded as water: a Fresnel mix towards a procedural sky
//!   evaluated along the reflected ray, with a rippled normal and lanes of wind, so the
//!   far field blazes where it mirrors the sun's horizon and the near paddies hold the
//!   blue of the zenith.
//! - **Depth is weather.** A valley morning is mist pooled in the low ground and ridges
//!   dissolving into haze. The post stage recovers each pixel's world position from the
//!   depth buffer and the projection matrix, fogs it with an exponential height fog that
//!   scatters towards the sun, and draws the far ridge lines itself where no geometry
//!   reached — the farthest mountains are weather, not geometry.
//! - **One sun, told once.** The sun's direction is a single rust value formatted into
//!   the light, the sky, the water and the fog, so the blaze in the paddies and the glow
//!   in the mist are the same sun and cannot drift apart.

use definition::Value;

use crate::dawn;
use crate::ridges;
use document::attributes::gallery_vertex_inputs;
use document::geometry::{
    cone, cuboid, fractal_noise, hash01, heightfield, prism, ridged_noise, sphere, value_noise,
};
use document::kit::{
    camera, environment_light, gallery_custom_schema, gallery_instance_schema, image, instance,
    instances, light_with_radius, mesh, mesh_material, post_processes, screen_material,
    shaded_point_schema, shader, surface,
};
use document::shaders::{Projection, lighting, mesh_shaders, post_shader};
use scene::standard::{Stage, light_pass_schema};

/// How far the valley reaches from its centre, in world units.
const EXTENT: f64 = 190.0;

/// How many cells the terrain is divided into along each axis.
///
/// A riser is about a fifth of a terrace step wide, and a step is under three units, so
/// the grid has to turn in well under a unit to shade a riser as a slope rather than as a
/// torn edge. Two hundred and twenty cells over the extent is one point every 1.7 units;
/// finer buys nothing the frame can resolve and the compile pays for it.
const DIVISIONS: usize = 220;

/// The sun's direction from the scene, normalised: low in the east-north-east, as an hour
/// after sunrise. This is the one fact every stage shares, and it is formatted into the
/// WGSL rather than written there twice.
const SUN_DIRECTION: [f64; 3] = [0.624, 0.240, -0.741];

/// Where the sun stands, far along its direction. The light is a point light with a
/// falloff, so its distance is part of how bright it is; this is far enough that the
/// falloff across the valley is nearly flat.
const SUN_POSITION: [f64; 3] = [268.3, 103.2, -318.6];

/// The height below which the valley floor's patchwork fields lie.
const FIELD_LEVEL: f64 = 14.0;

/// The height the terrace band starts at: above the fields.
const TERRACE_LOW: f64 = 10.0;

/// The height the terraces give way to forest at.
const TERRACE_HIGH: f64 = 66.0;

/// Where the village stands, on a knoll mid-slope on the eastern side.
const VILLAGE: [f64; 2] = [58.0, -6.0];

/// The time the scene carries: late enough that the mist has thinned off the water.
const TIME: f64 = 3.0;

/// Builds the terraced valley.
pub fn terraces() -> Value {
    let (materials, instances) = build_instances();
    terraced_valley(materials, instances)
}

/// Assembles the scene around the materials and the instances the placement produced.
fn terraced_valley(materials: Vec<Value>, instances: Value) -> Value {
    Value::structure([
        ("name".to_owned(), Value::String("terraces".to_owned())),
        ("time".to_owned(), Value::Float(TIME)),
        (
            "cameras".to_owned(),
            // Low on the south-west side, looking up the valley into the sun's quarter,
            // so the far paddies mirror the bright horizon and the near ones hold the
            // blue of the zenith. The framing crops the valley's rim out of frame: a
            // landscape whose edges show is a model, and one whose edges are cropped is
            // a place.
            //
            // The eye is **where the camera stands**, worked out against that cropping and
            // written down rather than left to be refitted. See `kit::camera`.
            Value::List(vec![camera(
                [-138.835897872, 90.014957447, 173.253846809],
                [10.0, 28.0, -50.0],
                0.88,
            )]),
        ),
        (
            // The valley's dawn, as the world a tracer is handed. The pixels are
            // [`dawn::Sky`]'s, term for term with the WGSL the mesh stage reads, and the
            // radiance beside it is one because the map is stored **at the sky's own scale**:
            // this sky's visible range runs from about a tenth to about nine tenths of one, and
            // pre-dividing it to make room for the sun's disc would spend the whole precision
            // the picture has on a disc that stands above the frame's top edge.
            //
            // **Two hundred and fifty six by one hundred and twenty-eight, the size the other
            // scenes use, and the size was measured rather than chosen.** This scene was first
            // written at `1024 × 512` on the reasoning that a latlong map spends its rows on
            // elevation and a coarse one must show steps. Both renders were measured against the
            // sky the `WGSL` law writes:
            //
            // | map | correlation with the law | sky band mean | worst step between rows |
            // |---|---|---|---|
            // | 256 × 128 | **+0.9803** | 195.8 185.6 183.1 | 0.27 |
            // | 1024 × 512 | **+0.9802** | 195.8 185.6 183.1 | 0.19 |
            //
            // **sixteen times the texels buy a thousandth of a correlation.** a latlong map at
            // this size carries this sky, and the earlier guess that it would not is recorded
            // here because it was made twice.
            "images".to_owned(),
            Value::List(vec![image(
                "sky",
                256,
                128,
                dawn::sky(SUN_DIRECTION).pixels(256, 128),
                Projection::Latlong,
            )]),
        ),
        (
            "lights".to_owned(),
            Value::List(vec![
                // The sun, low and warm. It stands in the frame's quarter rather than
                // behind the camera, because a paddy is a mirror and a mirror lit from
                // behind the viewer shows nothing.
                //
                // The radius is 692 to the far corner of the valley, so the sphere holds the
                // whole scene and the compiler promotes this light to a **global** one: in no
                // cluster list, evaluated by every fragment. Naming no radius would say the
                // same thing; the number is the scene's statement of the reach it means, and
                // a light naming less than that draws its own edge as a staircase quantised
                // to the cluster grid rather than as a terminator.
                light_with_radius(
                    "sun",
                    SUN_POSITION,
                    [1.0, 0.66, 0.40],
                    7_269_599.0,
                    330.0,
                    700.0,
                ),
                // The sky's fill, which used to be a lamp.
                //
                // It was there for an honest reason — *"a riser in its own shadow reads as
                // shadowed earth rather than as black"* — and for the reason the same sentence
                // names a few lines on: **a lamp cannot be a sky.** This valley is 380 units to
                // the far corner, a point light's brightness is the square of its distance, and
                // a lamp bright enough to lift the near risers leaves the far slopes black. The
                // defect the lamp was added to cure is the lamp.
                //
                // It is **replaced here, not joined**: the environment light reads the map
                // beside it, so the fill and the sky are one thing and cannot drift apart, and
                // the tracer draws its world from the same pixels this scene's post stage
                // draws its sky from. Before this, the rasterised frame carried the morning
                // and the traced one carried black, and the whole-frame correlation was
                // **-0.8264**.
                //
                // The radiance is **one**, and not a number fitted to a measured frame: the map
                // is stored at the sky's own scale, so one reads back the sky the WGSL writes.
                // Any other number would be scaling the picture rather than describing it.
                environment_light("sky", "sky", [1.0; 3], 1.0),
            ]),
        ),
        ("shaders".to_owned(), Value::List(build_shaders())),
        ("meshes".to_owned(), Value::List(build_meshes())),
        (
            "screen_materials".to_owned(),
            Value::List(vec![screen_material("post", "post")]),
        ),
        ("post_processes".to_owned(), post_processes(&["post"])),
        ("mesh_materials".to_owned(), Value::List(materials)),
        ("instances".to_owned(), instances),
    ])
}

/// The height of the land before the terraces are cut.
///
/// A valley that opens to the south: a long fall from the mountain at the back to the
/// fields in the foreground, walled on the east and west, with a knoll in the eastern
/// lee that the village stands on.
fn land_height(x: f64, z: f64) -> f64 {
    // The long fall, back to front. The curve steepens towards the back so the mountain
    // gathers the valley up rather than meeting it at an angle.
    let fall = (z + EXTENT) / (2.0 * EXTENT);
    let base = 4.0 + 58.0 * (1.0 - fall).powi(2);

    // The mountain mass at the back, built from creased noise rather than from a smooth
    // ramp: a ramp's knee seen from a low camera is a dome against the sky, and a dome
    // is a volcano. The creases run across the slope as well as down it, so what shows
    // against the sky is a range — crests and saddles at several depths — and never one
    // smooth silhouette. The crest's own height follows a one dimensional profile along
    // the ridge line, its second coordinate fixed.
    let ridge_profile = fractal_noise([(x + 60.0) * 0.017, 0.35], 5, 131);
    let ramp = smoothstep(-40.0, -EXTENT, z);
    let creased = ridged_noise([x * 0.021, z * 0.014], 5, 139);
    let mass = ramp * (22.0 + 78.0 * ridge_profile) * (0.30 + 0.90 * creased);

    // The side walls, kept as low creased hills: a smooth wall seen along its length is
    // a cone against the sky, which is the one shape this valley must not have, so the
    // walls hold the valley's sides and leave the skyline to the range.
    let wall = smoothstep(112.0, EXTENT, x.abs())
        * (8.0 + 22.0 * ridged_noise([z * 0.03, x * 0.02], 4, 47));

    // The knoll the village stands on.
    let (kx, kz) = (x - VILLAGE[0], z - VILLAGE[1]);
    let knoll = 9.0 * (-(kx * kx + kz * kz) / (2.0 * 30.0 * 30.0)).exp();

    // The undulation the contours follow. Kept out of the mountain's mass, where a
    // nine-metre wrinkle is nothing, and given to the field band, where it is the
    // difference between contours that rule straight and contours that wander.
    let undulation = (1.0 - smoothstep(40.0, 90.0, base + mass))
        * 9.0
        * fractal_noise([x * 0.016, z * 0.016], 3, 77);

    base + mass + wall + knoll + undulation
}

/// The height of the terrain: the land, quantised into levels where terraces belong.
///
/// # Invariants
///
/// The riser between levels is a smoothstep of the height fraction, so the surface is
/// C0 and the heightfield's finite-difference normals are true to it. A `floor` alone
/// would tear the surface at every level and shade every riser as a stripe of noise.
fn terrain_height(x: f64, z: f64) -> f64 {
    let land = land_height(x, z);

    // The terrace band: above the fields, below the treeline, and faded out where the
    // land is too steep to cut — no one terraces a cliff, and a contour stripe on a
    // cliff reads as snow.
    let slope = land_slope(x, z);
    let band = smoothstep(TERRACE_LOW, TERRACE_LOW + 6.0, land)
        * (1.0 - smoothstep(TERRACE_HIGH - 12.0, TERRACE_HIGH, land))
        * (1.0 - smoothstep(0.45, 0.75, slope));

    // The step height wanders a little, and the level's phase wanders more, which is
    // what makes one paddy wider than the next and the contours wander along the slope.
    let step = 2.55 * (0.85 + 0.3 * value_noise([x * 0.045, z * 0.045], 91));
    let phase = 1.6 * (value_noise([x * 0.028, z * 0.028], 57) - 0.5);
    let quantised = quantise(land + phase, step, 0.70) - phase;
    let terraced = land + (quantised - land) * band;

    // The valley floor's fields: the same idea with a step twice as tall and a riser
    // twice as soft, so the foreground reads as broad held fields rather than stairs.
    let fields =
        smoothstep(56.0, 96.0, z) * (1.0 - smoothstep(FIELD_LEVEL - 4.0, FIELD_LEVEL + 4.0, land));
    let fielded = quantise(terraced + phase, 5.2, 0.86) - phase;
    terraced + (fielded - terraced) * fields
}

/// The slope of the un-terraced land at a point, by central difference.
fn land_slope(x: f64, z: f64) -> f64 {
    let step = 2.0 * EXTENT / DIVISIONS as f64;
    let dx = land_height(x + step, z) - land_height(x - step, z);
    let dz = land_height(x, z + step) - land_height(x, z - step);
    (dx * dx + dz * dz).sqrt() / (2.0 * step)
}

/// Returns `value` quantised to steps of `step`, with the riser a smooth turn over the
/// top `1.0 - flat` of each step.
fn quantise(value: f64, step: f64, flat: f64) -> f64 {
    let q = value / step;
    let whole = q.floor();
    let part = q - whole;
    let riser = smoothstep(flat, 1.0, part);
    (whole + riser) * step
}

/// A smoothstep, matching the WGSL builtin's shape.
fn smoothstep(low: f64, high: f64, value: f64) -> f64 {
    let t = ((value - low) / (high - low)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The shaders the scene declares.
///
/// # Invariants
///
/// The two stages are given **different skies, on purpose**. The mesh stage has no map to read,
/// so the water is given the law — `sky_base` and the deck over it — and the post stage is given
/// the map and the deck alone. Neither is given both, because a stage that could draw its sky
/// either way is a stage that will one day draw the other one, and the two skies would then
/// differ by exactly the amount nobody measured.
///
/// The post stage's `Some(Projection::Latlong)` is what hands it the map: it is the declaration
/// that the stage reads `environment_sky`, and a `None` here would compile the grade perfectly
/// well against a black world.
fn build_shaders() -> Vec<Value> {
    let sky = dawn::sky(SUN_DIRECTION);
    let mut set = mesh_shaders(
        &fill(TERRAIN_SURFACE),
        &fill(&[NOISE_WGSL, &sky.wgsl(), CLOUD_WGSL, SKY_WGSL].concat()),
    );
    set.push(shader(
        Stage::Lighting,
        "lighting_props",
        // The same attribute list the three stages above were given: a second lighting
        // shader reads the same points and rows, and a list of its own is a second
        // transcription to keep in step with the first.
        &lighting(
            &gallery_vertex_inputs(&shaded_point_schema()),
            "",
            "",
            document::shaders::Projection::Latlong,
            &gallery_custom_schema(),
        ),
        light_pass_schema(),
        shaded_point_schema(),
        gallery_instance_schema(),
        gallery_custom_schema(),
    ));
    set.push(post_shader(
        &fill(GRADE),
        &fill(&[NOISE_WGSL, CLOUD_WGSL, ridges::RIDGES, FOG_WGSL].concat()),
        Some(Projection::Latlong),
    ));
    set
}

/// The meshes the scene draws: the terrain, the two cones of a China fir, a broadleaf
/// crown, and the unit wall and roof the village houses are scaled from.
fn build_meshes() -> Vec<Value> {
    vec![
        mesh(
            "terrain",
            "Triangles",
            shaded_point_schema(),
            heightfield(DIVISIONS, EXTENT, terrain_height),
        ),
        mesh(
            "fir_low",
            "Triangles",
            shaded_point_schema(),
            cone(9, 1.75, 2.6),
        ),
        mesh(
            "fir_high",
            "Triangles",
            shaded_point_schema(),
            cone(9, 1.15, 2.3),
        ),
        mesh("crown", "Triangles", shaded_point_schema(), sphere(10, 1.0)),
        mesh(
            "wall",
            "Triangles",
            shaded_point_schema(),
            cuboid(1.0, 1.0, 1.0),
        ),
        mesh(
            "roof",
            "Triangles",
            shaded_point_schema(),
            prism(1.0, 1.0, 1.0),
        ),
    ]
}

/// The materials a scene declares, and the instances drawn in each pass.
///
/// # Invariants
///
/// A material is declared once and named by the instances drawn with it. An instance is
/// listed under the passes that draw it, and since a material names one shader and a shader
/// belongs to one pass, an object drawn in three passes contributes three materials and
/// three entries, one per pass.
///
/// This is an accumulator rather than three loose lists because the two have to be built
/// together: every instance names a material, and a material that nothing names is a
/// declaration with no user.
struct Scene {
    materials: Vec<Value>,
    shadowing: Vec<Value>,
    depthing: Vec<Value>,
    lighting: Vec<Value>,
}

impl Scene {
    fn new() -> Self {
        Self {
            materials: Vec::new(),
            shadowing: Vec::new(),
            depthing: Vec::new(),
            lighting: Vec::new(),
        }
    }

    /// Declares a material and returns the label an instance names it by.
    fn declare(&mut self, label: &str, shader: &str, custom_data: Value) -> String {
        self.materials
            .push(mesh_material(label, shader, custom_data));
        label.to_owned()
    }

    /// Adds an instance to the bucket of the pass it is drawn in.
    ///
    /// The pass is the bucket key and the material's shader's stage, which the compiler
    /// checks agree. A prop's lighting shader is `lighting_props` while the pass is
    /// `lighting`, so the two are separate arguments rather than one name used twice.
    fn draw(&mut self, pass: &str, instance: Value) {
        match pass {
            "shadowing" => self.shadowing.push(instance),
            "depthing" => self.depthing.push(instance),
            _ => self.lighting.push(instance),
        }
    }

    /// Returns the material list and the instances bucketed by pass.
    fn build(self) -> (Vec<Value>, Value) {
        (
            self.materials,
            instances(vec![
                (Stage::Shadowing, self.shadowing),
                (Stage::Depthing, self.depthing),
                (Stage::Lighting, self.lighting),
            ]),
        )
    }
}

/// Builds every instance: the terrain three times, then the village and the firs three
/// times each, so the geometry is authored once and drawn by every stage.
fn build_instances() -> (Vec<Value>, Value) {
    let mut scene = Scene::new();
    for (label, pass, shader) in TERRAIN_STAGES {
        let material = scene.declare(
            &format!("terrain-{label}"),
            shader,
            // The terrain's albedo is decided per fragment by the surface; the row only
            // carries the light's own terms. A roughness near a half is wet earth's sheen
            // under the sky, and the water's sharp glint is the reflected sun the surface
            // adds, not this lobe.
            surface([1.0, 1.0, 1.0], 0.0, 0.55, [0.0; 3]),
        );
        scene.draw(
            pass,
            instance(
                format!("terrain-{label}"),
                "terrain",
                [0.0; 3],
                [0.0; 3],
                [1.0; 3],
                &material,
            ),
        );
    }

    for (index, house) in village().iter().enumerate() {
        place_house(&mut scene, index, house);
    }
    for (index, fir) in firs().iter().enumerate() {
        place_fir(&mut scene, index, fir);
    }
    for (index, crown) in broadleafs().iter().enumerate() {
        place_crown(&mut scene, index, crown);
    }
    scene.build()
}

/// One placement of a thing that stands on the terrain.
struct Placement {
    x: f64,
    z: f64,
    /// The height of the ground under it, from the terrain function itself.
    ground: f64,
    /// How much it is scaled by, and how far it is turned about the vertical.
    scale: f64,
    turn: f64,
    /// A per-instance hash, for tint variation.
    variety: f64,
}

/// A house of the village.
struct House {
    placement: Placement,
    /// Width, wall height and depth of the walls.
    walls: [f64; 3],
    /// Height of the gable.
    gable: f64,
}

/// The village: nine houses on and about the knoll, seated on the terrain.
///
/// The ring is deterministic, and the jitters are hashes of the index, so an unchanged
/// scene compiles to the same village every time.
fn village() -> Vec<House> {
    let mut houses = Vec::new();
    for index in 0..9i64 {
        let angle = index as f64 * 2.4 + hash01(index, 3, 5) * 0.9;
        let radius = 4.0 + 13.0 * hash01(index, 7, 11);
        let x = VILLAGE[0] + angle.cos() * radius;
        let z = VILLAGE[1] + angle.sin() * radius * 0.8;
        houses.push(House {
            placement: Placement {
                x,
                z,
                ground: terrain_height(x, z),
                scale: 1.0,
                // Aligned to the contour it stands on, with a jitter, the way a house is
                // set into a slope rather than dropped on it.
                turn: angle + 1.57 + (hash01(index, 13, 17) - 0.5) * 0.5,
                variety: hash01(index, 19, 23),
            },
            walls: [
                3.4 + 1.2 * hash01(index, 29, 31),
                2.4 + 0.6 * hash01(index, 37, 41),
                4.2 + 1.2 * hash01(index, 43, 47),
            ],
            gable: 1.3 + 0.5 * hash01(index, 53, 59),
        });
    }
    houses
}

/// Places one house: its walls and its roof, once per stage.
///
/// The roof is a unit prism scaled a half unit past the walls on each eave and sunk a
/// hand's width into the wall top, which is how a tiled roof sits on a farmhouse rather
/// than how a box sits on a box.
fn place_house(scene: &mut Scene, index: usize, house: &House) {
    let placed = &house.placement;
    // Sunk to the eaves' line of the first course: on a riser edge a house's downhill
    // corner would otherwise hover, and a farmhouse is built into its bank, not onto it.
    let base = placed.ground - 0.35;
    // Whitewash and rammed earth: most walls are lime-washed, a few are the earth they
    // are built of, and which is which is the house's own hash.
    let tint = if placed.variety < 0.3 {
        [0.55, 0.45, 0.33]
    } else {
        [0.76, 0.71, 0.62]
    };
    for (label, pass, shader) in PROP_STAGES {
        let wall = scene.declare(
            &format!("wall-{index}-{label}"),
            shader,
            prop_material(shader, tint, 0.75),
        );
        scene.draw(
            pass,
            instance(
                format!("wall-{index}-{label}"),
                "wall",
                [placed.x, base, placed.z],
                [0.0, placed.turn, 0.0],
                house.walls,
                &wall,
            ),
        );
        // The tile: a near-black grey-blue that warms where the sun catches it.
        let roof = scene.declare(
            &format!("roof-{index}-{label}"),
            shader,
            prop_material(shader, [0.10, 0.11, 0.14], 0.62),
        );
        scene.draw(
            pass,
            instance(
                format!("roof-{index}-{label}"),
                "roof",
                [placed.x, base + house.walls[1] - 0.10, placed.z],
                [0.0, placed.turn, 0.0],
                [house.walls[0] + 0.6, house.gable, house.walls[2] + 0.7],
                &roof,
            ),
        );
    }
}

/// The firs: a stand above the treeline, a guard around the village, and singles along
/// the terrace banks.
///
/// # Invariants
///
/// A fir stands on a bank or on the forest slope, never in a paddy: a candidate whose
/// ground is flat and low is water, and is rejected. The forest band accepts any slope a
/// tree could root on. Everything is a hash of the candidate's index, so the stand is the
/// same stand on every compile.
fn firs() -> Vec<Placement> {
    let mut firs = Vec::new();
    let mut index = 0i64;
    while firs.len() < 110 && index < 6000 {
        let x = (hash01(index, 3, 101) - 0.5) * 2.0 * (EXTENT - 8.0);
        let z = (hash01(index, 7, 103) - 0.5) * 2.0 * (EXTENT - 8.0);
        index += 1;

        let ground = terrain_height(x, z);
        let slope = terrain_slope(x, z);
        let village_range = ((x - VILLAGE[0]).powi(2) + (z - VILLAGE[1]).powi(2)).sqrt();
        if village_range < 14.0 {
            continue;
        }
        // Firs grow in stands, not in a sprinkle: a candidate outside a grove is passed
        // over wherever the grove noise is thin.
        let grove = fractal_noise([x * 0.03, z * 0.03], 3, 211) > 0.44;
        let forest = grove && ground > TERRACE_HIGH - 4.0 && ground < 128.0 && slope < 1.4;
        let bank = ground > TERRACE_LOW + 2.0
            && ground < TERRACE_HIGH - 6.0
            && slope > 0.05
            && slope < 0.9
            && hash01(index, 11, 107) < 0.28;
        let guard =
            ground > TERRACE_LOW + 2.0 && ground < 40.0 && village_range < 34.0 && slope < 1.0;
        if !(forest || bank || guard) {
            continue;
        }
        // The mountain stand grows the taller silhouettes: a fir against the sky on the
        // ridge's shoulder is what sells the range's scale.
        let high = ((ground - TERRACE_HIGH) / 50.0).clamp(0.0, 1.0);
        firs.push(Placement {
            x,
            z,
            ground,
            scale: (0.85 + 0.75 * hash01(index, 13, 109)) * (1.0 + 0.55 * high),
            turn: hash01(index, 17, 113) * std::f64::consts::TAU,
            variety: hash01(index, 19, 127),
        });
    }
    firs
}

/// The slope of the terrain at a point, from the same function the mesh is built from.
fn terrain_slope(x: f64, z: f64) -> f64 {
    let step = 2.0 * EXTENT / DIVISIONS as f64;
    let dx = terrain_height(x + step, z) - terrain_height(x - step, z);
    let dz = terrain_height(x, z + step) - terrain_height(x, z - step);
    (dx * dx + dz * dz).sqrt() / (2.0 * step)
}

/// The broadleaf trees: a few crowns among the houses and along the lower banks, where a
/// village keeps its pears and its camphor. A crown is a squashed sphere sat low, which
/// against the firs' cones is the difference between an orchard and a stand.
fn broadleafs() -> Vec<Placement> {
    let mut crowns = Vec::new();
    for index in 0..6i64 {
        let angle = hash01(index, 61, 67) * std::f64::consts::TAU;
        let radius = 11.0 + 9.0 * hash01(index, 71, 73);
        let x = VILLAGE[0] + angle.cos() * radius;
        let z = VILLAGE[1] + angle.sin() * radius;
        crowns.push(Placement {
            x,
            z,
            ground: terrain_height(x, z),
            scale: 1.1 + 0.7 * hash01(index, 79, 83),
            turn: 0.0,
            variety: hash01(index, 89, 97),
        });
    }
    crowns
}

/// Places one broadleaf crown, once per stage.
fn place_crown(scene: &mut Scene, index: usize, crown: &Placement) {
    let tint = [
        0.10 + 0.05 * crown.variety,
        0.20 + 0.06 * crown.variety,
        0.07,
    ];
    for (label, pass, shader) in PROP_STAGES {
        let material = scene.declare(
            &format!("crown-{index}-{label}"),
            shader,
            prop_material(shader, tint, 0.9),
        );
        scene.draw(
            pass,
            instance(
                format!("crown-{index}-{label}"),
                "crown",
                [crown.x, crown.ground + 1.4 * crown.scale, crown.z],
                [0.0, 0.0, 0.0],
                [crown.scale * 1.9, crown.scale * 1.5, crown.scale * 1.9],
                &material,
            ),
        );
    }
}

/// Places one fir: its two cones, once per stage.
///
/// A China fir is a broad lower skirt and a narrow upper one, so the tree is two cones
/// of one mesh pair and the pair is what every fir in the stand is drawn from.
fn place_fir(scene: &mut Scene, index: usize, fir: &Placement) {
    // The green deepens with altitude: the stand on the mountain is near-black against
    // the haze, and a garden fir by the village is a touch warmer.
    let depth = ((fir.ground - 20.0) / 90.0).clamp(0.0, 1.0);
    let tint = [
        0.055 + 0.035 * (1.0 - depth) + 0.02 * fir.variety,
        0.115 + 0.05 * (1.0 - depth),
        0.06 + 0.02 * (1.0 - depth),
    ];
    let base = fir.ground - 0.15;
    for (label, pass, shader) in PROP_STAGES {
        let material = scene.declare(
            &format!("fir-{index}-{label}"),
            shader,
            prop_material(shader, tint, 0.9),
        );
        scene.draw(
            pass,
            instance(
                format!("fir-low-{index}-{label}"),
                "fir_low",
                [fir.x, base, fir.z],
                [0.0, fir.turn, 0.0],
                [fir.scale; 3],
                &material,
            ),
        );
        scene.draw(
            pass,
            instance(
                format!("fir-high-{index}-{label}"),
                "fir_high",
                [fir.x, base + 1.85 * fir.scale, fir.z],
                [0.0, fir.turn + 0.4, 0.0],
                [fir.scale; 3],
                &material,
            ),
        );
    }
}

/// The instance labels and shaders of the three stages the terrain is drawn by.
const TERRAIN_STAGES: [(&str, &str, &str); 3] = [
    ("cast", "shadowing", "shadowing"),
    ("depth", "depthing", "depthing"),
    ("shade", "lighting", "lighting"),
];

/// The instance labels, passes and shaders of the three stages a prop is drawn by.
///
/// The pass and the shader are separate because a prop's lighting shader is
/// `lighting_props`: the bucket key is the pass, which is `lighting`.
const PROP_STAGES: [(&str, &str, &str); 3] = [
    ("cast", "shadowing", "shadowing"),
    ("depth", "depthing", "depthing"),
    ("shade", "lighting", "lighting_props"),
];

/// The material a prop is drawn with: plain in the two depth stages, tinted in the
/// lighting one.
fn prop_material(shader: &str, tint: [f64; 3], roughness: f64) -> Value {
    let tint = if shader == "lighting_props" {
        tint
    } else {
        [1.0, 1.0, 1.0]
    };
    surface(tint, 0.0, roughness, [0.0; 3])
}

/// Writes the shared constants into a WGSL source, in place of its tokens.
///
/// The tokens are the few facts the rust side and the WGSL side must agree on — the
/// sun's direction, the terrace band, the village — and formatting them in is what keeps
/// them single-sourced. A token that survives is a bug, and it fails loudly: the device
/// parses the shader and names the offending text.
fn fill(source: &str) -> String {
    source
        .replace("__SUN__", &vec3_wgsl(SUN_DIRECTION))
        .replace("__TERRACE_LOW__", &f32_wgsl(TERRACE_LOW))
        .replace("__TERRACE_HIGH__", &f32_wgsl(TERRACE_HIGH))
        .replace("__FIELD_LEVEL__", &f32_wgsl(FIELD_LEVEL))
        .replace("__VILLAGE__", &vec2_wgsl(VILLAGE))
}

/// Formats a number as a WGSL float literal.
fn f32_wgsl(value: f64) -> String {
    format!("{value:.4}")
}

/// Formats a two component vector as a WGSL `vec2<f32>` literal.
fn vec2_wgsl(vector: [f64; 2]) -> String {
    format!("vec2<f32>({:.4}, {:.4})", vector[0], vector[1])
}

/// Formats a three component vector as a WGSL `vec3<f32>` literal.
fn vec3_wgsl(vector: [f64; 3]) -> String {
    format!(
        "vec3<f32>({:.4}, {:.4}, {:.4})",
        vector[0], vector[1], vector[2]
    )
}

/// What the lighting stage does to the terrain after the light is applied.
///
/// # Invariants
///
/// The body is written into the lighting stage's fragment function, so it reads the
/// names the template has in scope — `color`, `normal`, `in` and `uniforms` — and
/// declares none of them twice. The per light names it used to name as well
/// (`lambert`, `radiance`, `ambient`) are gone: the template sums the lights of one
/// cluster and adds its ambient once, so a body spliced after them sees only the sum.
/// The pipeline's own colour is multiplied by the fragment's
/// albedo, which carries every zone the terrain has; the water's mirror is added after,
/// because a reflection is light the surface gathers, not albedo it holds.
const TERRAIN_SURFACE: &str = r#"
    let ground = in.world_position;
    let view_dir = normalize(ground - uniforms.projection.position);
    let up_flat = smoothstep(0.90, 0.975, normal.y);
    let steep = 1.0 - smoothstep(0.42, 0.72, normal.y);

    // The terrace band and the valley floor's fields, from the same heights the rust
    // side cut them at.
    let band = smoothstep(__TERRACE_LOW__, __TERRACE_LOW__ + 6.0, ground.y)
        * (1.0 - smoothstep(__TERRACE_HIGH__ - 12.0, __TERRACE_HIGH__, ground.y));
    let fields = smoothstep(56.0, 96.0, ground.z)
        * (1.0 - smoothstep(__FIELD_LEVEL__ - 4.0, __FIELD_LEVEL__ + 4.0, ground.y));

    // The valley floor's patchwork: a cell of eleven units, hashed to a crop. The
    // borders are the low banks between fields, a touch darker than whatever they hold.
    let cell = floor(ground.xz / 11.0);
    let field_kind = hash2(cell);
    var crop = vec3<f32>(0.20, 0.36, 0.09);
    if field_kind < 0.26 {
        crop = vec3<f32>(0.46, 0.38, 0.17);
    } else if field_kind < 0.44 {
        crop = vec3<f32>(0.29, 0.22, 0.14);
    } else if field_kind < 0.62 {
        crop = vec3<f32>(0.12, 0.27, 0.07);
    }
    let field_water = step(0.80, field_kind);
    let cell_edge = abs(fract(ground.xz / 11.0) - vec2<f32>(0.5, 0.5)) * 2.0;
    let bank_line = smoothstep(0.82, 0.96, max(cell_edge.x, cell_edge.y));
    crop = mix(crop, vec3<f32>(0.33, 0.27, 0.17), bank_line * 0.55);
    crop = crop * (0.85 + 0.3 * value_noise(ground.xz * 0.6));

    // The riser: mud and the straw of last year's stubble, banded by height so the
    // layers of a bank show, and tipped with grass where the bank meets the water's lip.
    let mud = 0.75 + 0.5 * value_noise(vec2<f32>((ground.x + ground.z) * 0.31, ground.y * 7.0));
    var riser = vec3<f32>(0.34, 0.27, 0.17) * mud;
    let lip = smoothstep(0.55, 0.9, normal.y) * (1.0 - up_flat);
    riser = mix(riser, vec3<f32>(0.22, 0.30, 0.10), lip * (0.4 + 0.4 * value_noise(ground.xz * 0.9)));

    // The forest and the rock above the band: canopy noise on the flats, strata on the
    // steeps, and the two mixed by slope the way a hillside decides it.
    let canopy = vec3<f32>(0.075, 0.135, 0.065)
        * (0.55 + 0.9 * fractal(ground.xz * 0.11));
    let strata = vec3<f32>(0.23, 0.22, 0.20)
        * (0.75 + 0.5 * value_noise(vec2<f32>((ground.x - ground.z) * 0.05, ground.y * 0.6)));
    let high = smoothstep(__TERRACE_HIGH__ - 14.0, __TERRACE_HIGH__ + 8.0, ground.y);
    let wild = mix(canopy, strata, clamp(steep * (0.5 + 0.5 * high) + high * 0.2, 0.0, 1.0));

    // The albedo the light is multiplied by: crop on the valley floor, earth on the
    // risers, forest above, and dark still water wherever a paddy lies. The riser's own
    // band is gated by the slope the rust side refused to terrace, so a natural steep
    // keeps its forest instead of wearing mud stripes.
    let cut = 1.0 - smoothstep(0.80, 0.91, normal.y);
    var albedo = mix(riser, wild, clamp(steep + high * (1.0 - fields), 0.0, 1.0));
    albedo = mix(albedo, crop, fields * up_flat);
    albedo = mix(albedo, riser, band * (1.0 - up_flat) * (1.0 - fields) * cut);

    // The water. A paddy is a terrace flat inside the band, a flooded cell on the
    // valley floor, or nothing; near the village the ground is dry, because a village
    // is not built in a pond. Some paddies hold young rice rather than open water, and
    // those keep a green cast under the sky they mirror.
    let village_far = smoothstep(16.0, 28.0, distance(ground.xz, __VILLAGE__));
    let paddy = band * up_flat * (1.0 - fields) * village_far;
    let flooded = fields * up_flat * field_water * village_far;
    let water = clamp(paddy + flooded, 0.0, 1.0);
    let planted = step(hash2(floor(ground.xz / 6.5) + vec2<f32>(7.0, 3.0)), 0.30);
    albedo = mix(albedo, vec3<f32>(0.035, 0.06, 0.055), water);
    albedo = mix(albedo, vec3<f32>(0.13, 0.20, 0.09), water * planted * 0.8);
    color = color * albedo;

    // The mirror. The normal is rippled by two drifting noises and calmed in lanes, so
    // the field has wind on it in patches the way still water does. What it reflects is
    // the sky function itself, sun included, so the paddies on the sun's quarter blaze.
    if water > 0.003 {
        let t = uniforms.time;
        let ripple = (value_noise(ground.xz * 1.9 + vec2<f32>(t * 0.31, t * 0.13)) - 0.5)
            + 0.5 * (value_noise(ground.xz * 4.3 - vec2<f32>(t * 0.19, t * 0.29)) - 0.5);
        let wind = 0.15 + 0.85 * value_noise(ground.xz * 0.033 + vec2<f32>(t * 0.045, 0.0));
        let water_normal = normalize(vec3<f32>(
            normal.x + ripple * 0.09 * wind,
            normal.y,
            normal.z + ripple * 0.07 * wind,
        ));
        let facing = max(dot(water_normal, -view_dir), 0.0);
        let mirror = 0.035 + 0.965 * pow(1.0 - facing, 5.0);
        let reflected = sky_colour(reflect(view_dir, water_normal));
        let open = 1.0 - planted * 0.75;
        color = mix(color, reflected * open, clamp(mirror * water * 1.02, 0.0, 0.96));
    }
"#;

/// The WGSL every noise function in the scene is written from.
///
/// # Invariants
///
/// The hash is of the lattice point, not of the coordinate, so a value is constant
/// within a cell and the smoothing is what turns the jumps into slopes — the same
/// arrangement the rust side's noise has, and for the same reason.
const NOISE_WGSL: &str = r#"
/// Returns a value in zero to one from a two dimensional coordinate.
fn hash2(point: vec2<f32>) -> f32 {
    return fract(sin(dot(point, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

/// Returns smooth value noise in zero to one.
fn value_noise(point: vec2<f32>) -> f32 {
    let cell = floor(point);
    let within = fract(point);
    let blend = within * within * (3.0 - 2.0 * within);
    let corner = hash2(cell);
    let east = hash2(cell + vec2<f32>(1.0, 0.0));
    let north = hash2(cell + vec2<f32>(0.0, 1.0));
    let far = hash2(cell + vec2<f32>(1.0, 1.0));
    return mix(mix(corner, east, blend.x), mix(north, far, blend.x), blend.y);
}

/// Returns noise summed over three octaves, in roughly zero to one.
fn fractal(point: vec2<f32>) -> f32 {
    var total = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    for (var octave = 0; octave < 3; octave = octave + 1) {
        total = total + amplitude * value_noise(point * frequency);
        frequency = frequency * 2.1;
        amplitude = amplitude * 0.5;
    }
    return total * 1.14;
}
"#;

/// The weather laid over a sky.
///
/// # Invariants
///
/// This is **not the sky**. The sky is [`dawn::Sky`], and a stage that has the environment map
/// reads its sky out of that. What is here is the one thing the map cannot carry: the stratus
/// deck drifts with the scene's clock, and a static image has no clock, so the deck belongs to
/// the stage that can still move it.
///
/// It takes the sky as an argument rather than evaluating one, so the stage that has a map and
/// the stage that has a law lay the **same** deck over either.
const CLOUD_WGSL: &str = r#"
/// Lays the stratus deck over a sky, and returns what is left of it.
///
/// A thin deck stretched along the wind and lit by the sun it half hides. It is mixed rather
/// than added, because a cloud **hides** what is behind it, and the part of the sun it does hide
/// is why a thick one goes grey on the side away from the light.
fn cloud_deck(ray: vec3<f32>, sky: vec3<f32>) -> vec3<f32> {
    if ray.y <= 0.015 {
        return sky;
    }
    let to_sun = max(dot(ray, __SUN__), 0.0);
    let sweep = vec2<f32>(ray.x, ray.z) / max(ray.y, 0.08);
    let streak = fractal(sweep * 0.22 + vec2<f32>(uniforms.time * 0.006, 0.0));
    let deck = smoothstep(0.48, 0.70, streak)
        * (1.0 - smoothstep(0.22, 0.55, ray.y))
        * smoothstep(0.015, 0.06, ray.y);
    let lit = pow(to_sun, 2.0);
    let cloud = mix(vec3<f32>(0.55, 0.53, 0.55), vec3<f32>(0.98, 0.72, 0.46), lit);
    return mix(sky, cloud, deck * 0.6);
}
"#;

/// The sky a paddy mirrors: its base, and the same deck the air is carrying.
///
/// # Invariants
///
/// This is the **only** place `sky_base` is reachable from. The post stage is handed the
/// environment map and reads its sky out of that, and it is not given this function at all —
/// a stage that could draw the sky either way would eventually draw it the other way, and the
/// two skies would differ by exactly the amount nobody measured.
const SKY_WGSL: &str = r#"
/// The sky a paddy mirrors: its base, and the same deck the air is carrying.
fn sky_colour(ray: vec3<f32>) -> vec3<f32> {
    return cloud_deck(ray, sky_base(ray));
}
"#;

// The far mountains this scene draws where no geometry reached are [`ridges::RIDGES`], and
// every rule behind them lives with the idea rather than here: a second scene wanted a
// backdrop behind a horizon the air had already closed over, and a backdrop written twice is
// two backdrops that will drift apart.

/// The valley's mist, as an exponential height fog.
///
/// # Invariants
///
/// The optical depth of an exponential height fog has a closed form along a ray, and the
/// closed form is what is used: stepping the fog would band where the mist is thin and
/// lie about where it is thick. The fog scatters towards the sun, so the mist over the
/// sun's quarter of the valley is warm and the mist behind is cool.
///
/// **`world_position` is not here.** this string used to declare it, and so did `foldyard`'s,
/// and the derivation that justified it — the one that says why reading a bare perspective's
/// own coefficients reads every distance as the far plane — is the reason it is now the
/// shared post stage's. A function two scenes each carry is a function the two can drift apart
/// on.
const FOG_WGSL: &str = r#"
/// Fogs `color` by the air between the camera and `world`, along `ray`.
///
/// # Invariants
///
/// The closed form is taken piecewise: a ray that dips below the ceiling integrates the
/// exponential down to the ceiling and the full density the rest of the way. Applying
/// the single exponential form to a dipping ray extrapolates the density's *growth*
/// without bound, and a few hundred units of it is a white frame rather than a mist.
fn fog(color: vec3<f32>, ray: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    let eye = uniforms.camera.position;
    let along = distance(eye, world);
    // The mist's ceiling wanders with the valley and drifts with the hour, so it pools
    // in the low ground and tears along the banks rather than lying flat.
    let ceiling = __FIELD_LEVEL__ + 6.0
        + 6.0 * fractal(world.xz * 0.014 + vec2<f32>(uniforms.time * 0.012, 0.0));
    // A morning mist, not a fog bank: three hundred units of visibility inside it, so
    // the valley floor reads through its own haze and only the distance drowns.
    let density = 0.0035;
    let falloff = 0.10;
    let above = max(eye.y - ceiling, 0.0);

    var optical = 0.0;
    if eye.y <= ceiling {
        // In the mist: the density is the ceiling's own all the way.
        optical = density * along;
    } else {
        let dip = falloff * ray.y;
        let cross = (ceiling - eye.y) / ray.y;
        if ray.y < -0.0005 && cross < along {
            // Down through the ceiling and on: the exponential to the crossing, the full
            // density after it.
            optical = density * exp(-falloff * above) * (1.0 - exp(-dip * cross)) / dip
                + density * (along - cross);
        } else if abs(dip) > 0.0005 {
            // Slanting but never reaching the ceiling: the closed form.
            optical = density * exp(-falloff * above) * (1.0 - exp(-dip * along)) / dip;
        } else {
            // Level: the limit of the form, which divides by the ray's rise.
            optical = density * exp(-falloff * above) * along;
        }
    }
    // The aerial perspective proper: even above the mist the air is not nothing.
    optical = optical + along * 0.00055;

    // What the fog adds is the sky's own haze, warmed only close about the sun: forward
    // scatter is why a misty valley blazes when you look into the light, and only then.
    let sunward = pow(max(dot(ray, __SUN__), 0.0), 5.0);
    let haze = mix(vec3<f32>(0.55, 0.62, 0.72), vec3<f32>(1.02, 0.66, 0.40), sunward);
    return mix(color, haze, clamp(1.0 - exp(-optical), 0.0, 1.0));
}
"#;

/// What the post stage does to the frame.
///
/// # Invariants
///
/// The sky and the ridges are drawn only where the depth buffer is still at its clear
/// value, because a depth of one is the set of pixels no geometry covered. The fog runs
/// on everything, geometry and sky alike, so the weather is one weather.
const GRADE: &str = r#"
    let ray = ray_through(in.uv);
    if depth >= 1.0 {
        // **The map, not a sky law.** This stage is handed the same image the tracer is handed,
        // so the sky it draws and the sky the tracer draws are one image rather than two
        // evaluations of one formula — and the moment they stop being one image, a radiance
        // fitted to a frame is the only thing left that could hide it. The stratus deck is laid
        // over the top because it is weather and not the world: it moves with the clock, and a
        // world is not a thing that moves.
        var sky = cloud_deck(ray, environment_sky(ray));
        // **Three** ranges, and not a receding series: this scene's horizon is the valley's own
        // mist, so the painted ranges are the one step of distance beyond it rather than the
        // several a valley looked into needs.
        sky = ridges(sky, ray, 1.0, 3);
        color = sky;
    }

    // The weather, on whatever the pixel shows. The world position is recovered from
    // the depth the prepass wrote, so the mist knows the ground it pools on.
    let world = world_position(in.uv, depth);
    color = fog(color, ray, world);

    // The grade. An ACES fit rather than a clamp, so the blaze in the paddies rolls off
    // to white instead of hitting a wall; gamma after, because the target is linear.
    color = color * 0.80;
    color = clamp(
        (color * (2.51 * color + vec3<f32>(0.03)))
            / (color * (2.43 * color + vec3<f32>(0.59)) + vec3<f32>(0.14)),
        vec3<f32>(0.0),
        vec3<f32>(1.0),
    );
    color = pow(color, vec3<f32>(0.4545));
    // The saturation the curve took out is put back, and a touch more for the crops.
    let luma = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = clamp(mix(vec3<f32>(luma), color, 1.26), vec3<f32>(0.0), vec3<f32>(1.0));
    // A gentle S about the middle, to separate a lit bank from a shadowed one.
    color = color * color * (3.0 - 2.0 * color) * 0.30 + color * 0.70;
    // The vignette, light enough to be a lens rather than a frame.
    let centred = in.uv - vec2<f32>(0.5, 0.5);
    color = color * clamp(1.0 - dot(centred, centred) * 0.30, 0.0, 1.0);
"#;
