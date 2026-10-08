//! Far ranges painted into the sky, for a scene whose modelled horizon is too far gone to
//! read.
//!
//! # What this is for
//!
//! A range modelled as geometry and drawn as geometry runs into a wall well before the eye
//! can see it. The air closes over it in proportion to the distance to it, and once it is
//! most of the way closed the range is the air's own colour plus whatever fraction of its own
//! is left — and a range whose material was chosen to match the haze, which is the correct
//! choice, has no fraction left worth the name. What is left is a silhouette on a sky it is
//! the same colour as, which is not a distant mountain, it is a slightly wrong patch of sky.
//!
//! The way out is not to model the range nearer, because the distance is the subject. It is to
//! **stop asking the air to carry it.** These are painted into the sky in the post stage, after
//! the haze, so the air is not between them and the sky at all: they are inked rather than lit,
//! and a mark on a surface does not get hazed by the surface's own depth. A scene calls
//! [`RIDGES`] in the branch that draws the sky, and its modelled far ranges keep their proper
//! aerial perspective in front of these.
//!
//! # What it is not
//!
//! It is not a substitute for modelled land. A painted ridge has no normal, so it takes no
//! light, casts no shadow and carries no surface detail, and a camera that walks towards it
//! watches it stay exactly where it was. It is a backdrop and it says so: a scene that wants
//! ridges a photographer can stand among models them, and this is for the ones that are only
//! ever seen at a distance the air would not allow.
//!
//! Two scenes wanted this and one had it written twice, which is what [`RIDGES`] is: the
//! second scene to need a backdrop behind its far ground should add the string to its post
//! helpers and not write this again.
//!
//! # The two numbers a scene passes
//!
//! [`ridges`] takes a **scale** on the elevation of the layers and a **count** of them, because
//! how high a far range stands in the frame and how many of them there are are both facts about
//! the scene's camera and its own land, not about the idea. A scene whose camera looks **down**
//! on its subject from a height has its whole horizon high in the picture, and layers scaled for
//! a camera looking level sit above the top of the frame and come out as a band of colour across
//! the sky rather than as mountains on a skyline. `1.0` is a camera looking level; less is a
//! camera looking down.
//!
//! The **count** is how far the air closes. Each layer is a step of air between the eye and a
//! ridge, so three is enough for a horizon that stops at one and not enough for a valley that
//! has to recede through several — a deep horizon drawn too shallow reads as a single line with
//! a shadow on it, and reads as shallow however far away it is put.

/// The WGSL a post stage adds to its helpers to get [`ridges`](RIDGES).
///
/// # Invariants
///
/// - A ridge line is a function of the **ray's azimuth**, not of the pixel. A silhouette
///   written in screen space is a backdrop that does not hold still when the camera moves, and
///   a horizon that slides is the one failure a viewer names before they can say why.
/// - Each layer is inked **deeper** than the one behind it, because what separates the layers
///   is the air between them and nothing else. Three identical lines at three distances are
///   one line drawn three times.
/// - The **top** edge is the silhouette and the **foot** dissolves downward, because a ridge's
///   foot is standing in the mist rather than cut off by a rule. A hard bottom edge is the
///   same mistake as a hard top edge, and quieter, which is why it survives longer.
/// - A single crease **saturates** at its summit, and a saturated crease is a plateau, which
///   reads as a wall and not as a ridge. Two creases at incommensurate frequencies multiplied
///   is what keeps the summit a point.
pub const RIDGES: &str = r#"
/// Returns one dimensional value noise in zero to one, for the ridge lines.
///
/// # Invariants
///
/// The hash is of the lattice point, not of the coordinate, so a value is constant within a
/// cell and the smoothing is what turns the jumps into slopes — the same arrangement the rust
/// side's noise has, and for the same reason.
fn ridge_noise(value: f32) -> f32 {
    let cell = floor(value);
    let within = fract(value);
    let blend = within * within * (3.0 - 2.0 * within);
    let near = fract(sin(cell * 127.1) * 43758.5453);
    let far = fract(sin((cell + 1.0) * 127.1) * 43758.5453);
    return mix(near, far, blend);
}

/// Returns the elevation of a ridge line at an azimuth, which layer it is, and how high the
/// layers stand in this scene's frame.
///
/// # Invariants
///
/// The **base** is fixed and only the relief above it is scaled, because the base is the
/// distance at which the nearest layer is judged to stand and that is the one number here
/// that belongs to the idea rather than to the camera. Everything above it is how much the
/// range rises above that distance, and a camera looking down on a valley sees less of it.
fn ridge_at(azimuth: f32, layer: f32, scale: f32) -> vec2<f32> {
    // Creased noise, so the line is crests and saddles rather than swells: a smooth bump
    // the width of the frame is a volcano, and these are the far ranges. Two creases at
    // incommensurate frequencies, multiplied: a single crease saturates at its summit
    // and a saturated crease is a plateau, which reads as a wall, not a ridge.
    let swell = ridge_noise(azimuth * (1.6 + layer * 0.9) + layer * 17.0);
    let crest = 1.0 - abs(swell * 2.0 - 1.0);
    let second = 1.0 - abs(ridge_noise(azimuth * (3.7 + layer * 2.1) + layer * 41.0) * 2.0 - 1.0);
    let spur = ridge_noise(azimuth * (6.1 + layer * 2.3) + layer * 29.0);
    // The **massif** the peaks stand on, at a third of the crease's own frequency. Without it the
    // creases are all the same height and the skyline is a row of tents: every peak is the same
    // size and evenly spaced, and that regularity is the thing the eye names as *tents* before it
    // can say why. A range is peaks **on** something, and the something is what makes two peaks a
    // massif and six peaks a chain. It also lowers the whole skyline where the land behind is
    // low, which is what gives a horizon a shoulder to be seen over.
    let massif = 0.30 + 0.70 * ridge_noise(azimuth * (0.53 + layer * 0.31) + layer * 7.0);
    let elevation = 0.012
        + scale * (
            layer * 0.020
            + massif * crest * second * (0.030 + layer * 0.020)
            + spur * 0.008
        );
    return vec2<f32>(elevation, layer);
}

/// Draws the ridge lines into `sky` along `ray`, nearest last, `layers` deep, at the given scale.
///
/// # Invariants
///
/// **The depth is the caller's and not a constant.** How many ranges a horizon is made of is a
/// fact about the land in front of the eye, not about the idea of a painted ridge: a valley that
/// closes on a notch four ranges deep and a plateau that stops at the first are the same backdrop
/// drawn at two different depths, and a constant would force one of them to lie about either
/// how far away the horizon is or how far the air has closed over it. Nearer layers stand lower
/// and are inked deeper, so adding a layer at the back is the same operation as adding one at
/// the front and the order they are drawn in does the rest.
fn ridges(sky: vec3<f32>, ray: vec3<f32>, scale: f32, layers: i32) -> vec3<f32> {
    let azimuth = atan2(ray.x, -ray.z);
    var result = sky;
    // Farthest first, so the nearer line overwrites the farther where they cross. The
    // inks are the haze itself deepened a step per layer: nothing out there has a colour
    // of its own any more, only less air than the sky behind it.
    for (var layer = layers - 1; layer >= 0; layer = layer - 1) {
        let line = ridge_at(azimuth, f32(layer), scale);
        if ray.y < line.x && ray.y > -0.05 * scale {
            let depth_haze = 0.88 - line.y * 0.18;
            let ink = mix(
                vec3<f32>(0.44, 0.48, 0.58),
                vec3<f32>(0.34, 0.39, 0.50),
                line.y * 0.45,
            );
            // The top edge is the silhouette; the bottom fades into the valley's own
            // haze, because the ridge's foot is in the mist, not behind a hard line. The
            // foot's window scales with the crest, because a foot that stayed where it was
            // while the range came down would close over the range's own body and leave a
            // band of colour with no mountains in it.
            let top = 1.0 - smoothstep(line.x - 0.0035 * scale, line.x, ray.y);
            let foot = smoothstep(-0.05 * scale, -0.012 * scale, ray.y);
            result = mix(result, mix(result, ink, depth_haze), top * foot);
        }
    }
    return result;
}
"#;
