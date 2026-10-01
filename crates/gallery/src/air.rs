//! The air a scene is drawn through, written once and handed to every stage.
//!
//! # Why this is one value and not a helper per stage
//!
//! A landscape is mostly air. The post stage draws the sky, the lighting stage hazes every
//! surface towards it, and a water surface reflects it — and those are three separate
//! `WGSL` strings written into three separate stages, so nothing stops them disagreeing.
//! When they do, the place they disagree is **the horizon**: the far water or the far field
//! meets the sky along a line, and the two colours either side of it are a fifth apart, and
//! the line reads as a bright seam. That is the commonest way a rendered landscape announces
//! itself, and it is not a subtle shading error — it is the picture telling you that it is a
//! picture.
//!
//! [`dunes`] had exactly that bug and named the two numbers in its own source: the lighting
//! body mixed distant surfaces towards `vec3(0.58, 0.62, 0.72)` under the comment *"the
//! colour a distant surface tends towards is the colour of the sky behind it"*, and the post
//! body drew the sky's horizon as `vec3(0.70, 0.66, 0.60)`. The comment was right and the
//! two values were not, and no amount of care at the call site fixes that, because the
//! defect is that there are two call sites.
//!
//! So the rule here is narrower than it first looks. **What must not disagree is the colour,
//! not the law.** Every outdoor scene has its own aerial perspective — a height falloff over
//! water, a flat distance wash over a dune field, nothing at all indoors — and forcing one law
//! on all of them would be a worse mistake than the seam. What every one of them has in common
//! is the colour the air settles at, and that is the one number this module owns.
//!
//! # What a scene writes
//!
//! ```text
//! let air = Atmosphere::new()
//!     .zenith([0.13, 0.27, 0.56])
//!     .horizon([0.70, 0.66, 0.60])
//!     .ground([0.44, 0.40, 0.36])
//!     .density(0.0006)
//!     .scale_height(90.0);
//! let helpers = air.wgsl();
//! ```
//!
//! and the same string goes to the lighting stage, the emission stage and the post stage. A
//! surface hazes towards [`air_colour`](AIR_COLOUR) in one and the sky reaches it in another,
//! and the seam is not avoided by a scene being careful — it is made impossible.
//!
//! A scene that wants its own law writes its own body and calls
//! [`air_closure`](AIR_CLOSURE) or nothing at all. The law below is the one that is right for
//! a scene whose subject is near the ground, which is most of them.

/// The air a scene is drawn through.
///
/// # Invariants
///
/// - **`horizon` is the colour of the air, and the sky is that colour at the horizon.** The
///   two are one number, so a surface hazed towards the air and a sky above it meet without
///   a seam. A scene that wants a sky warmer or cooler at the horizon than the air really is
///   describing a scene where the two differ, which is a decision rather than an oversight,
///   and the fields here are separate for exactly that: set them apart deliberately and the
///   seam comes back on purpose.
/// - **Everything is optional and everything has a default.** An atmosphere with no fields
///   set is a flat grey world, which is a legitimate thing to draw, and a scene that names
///   only a sun still gets a sky.
/// - **The sun is a direction, not a position.** It is what a sky is a function of, and a
///   scene that also wants it to *light* the scene needs a light for that — which is
///   [`kit::light`], and which is a separate decision this module deliberately does not
///   make for the scene.
#[derive(Debug, Clone, Copy)]
pub struct Atmosphere {
    zenith: [f64; 3],
    horizon: [f64; 3],
    ground: [f64; 3],
    sun: Option<[f64; 3]>,
    glow: Option<([f64; 3], f64, f64)>,
    disc: Option<([f64; 3], f64)>,
    density: f64,
    scale_height: f64,
}

/// The default gradient's tightness, which is what keeps a sky from being a backdrop.
///
/// A sky built as a straight ramp from the horizon to the zenith reads as a backdrop, and a
/// backdrop does not move when the camera tilts. Compressing the ramp towards the horizon
/// is what makes the gradient look like it belongs to a sphere.
const DEFAULT_CURVE: f64 = 0.55;

/// How wide the sun's disc is drawn, in the cosine of the angle from its centre.
///
/// A disc is a small solid angle, so the cosine of the angle between the ray and the sun is
/// within a few ten thousandths of one across the whole of it, and a `smoothstep` over a
/// range any wider draws a halo the size of a town. The width belongs here rather than in a
/// scene's number because it is a property of the disc and not of the hour.
const DISC_EDGE: f64 = 0.00004;

impl Default for Atmosphere {
    /// Returns the air a scene that names no atmosphere is drawn through: a flat grey world.
    fn default() -> Self {
        Self {
            zenith: [0.10, 0.12, 0.16],
            horizon: [0.30, 0.32, 0.36],
            ground: [0.24, 0.23, 0.22],
            sun: None,
            glow: None,
            disc: None,
            density: 0.0,
            scale_height: 1.0,
        }
    }
}

impl Atmosphere {
    /// Returns an atmosphere with nothing set, which every field below replaces.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the colour overhead.
    pub fn zenith(mut self, colour: [f64; 3]) -> Self {
        self.zenith = colour;
        self
    }

    /// Sets the colour along the horizon, **and the colour the air itself is**.
    ///
    /// This is the field the whole module exists for: a surface hazed towards the air and a
    /// sky at the horizon are the same colour, and a scene that says so once cannot go wrong
    /// at the join.
    pub fn horizon(mut self, colour: [f64; 3]) -> Self {
        self.horizon = colour;
        self
    }

    /// Sets the colour below the horizon, which is more air rather than a sky.
    pub fn ground(mut self, colour: [f64; 3]) -> Self {
        self.ground = colour;
        self
    }

    /// Sets which way the sun is, and how far its bearing reaches round the compass.
    ///
    /// `reach` is the tightness of that burn: a large number is a band hugging the horizon
    /// and a small one is a glow filling a quarter of the sky. It is the difference between
    /// the hour after sunrise and an hour later, and it is a number rather than a choice of
    /// function because the shape is the same and only the width changes.
    pub fn sun(mut self, direction: [f64; 3], glow: [f64; 3], reach: f64, strength: f64) -> Self {
        let length = (direction[0].powi(2) + direction[1].powi(2) + direction[2].powi(2)).sqrt();
        if length > 1e-9 {
            self.sun = Some([
                direction[0] / length,
                direction[1] / length,
                direction[2] / length,
            ]);
            self.glow = Some((glow, reach, strength));
        }
        self
    }

    /// Sets the sun's disc: the colour it is and how large it is drawn.
    ///
    /// `sharpness` is the width of the disc's edge, in the cosine of the angle between the
    /// ray and the sun. A disc is a very small solid angle, so the number is very close to
    /// one and the difference between the inner and outer edge is in the fifth decimal — which
    /// is why this is a parameter rather than a constant, because the correct value depends
    /// on how wide the frame's field of view is and a scene framing a treeline wants a
    /// different disc from one framing a whole sky.
    pub fn disc(mut self, colour: [f64; 3], sharpness: f64) -> Self {
        self.disc = Some((colour, sharpness));
        self
    }

    /// Sets how thick the air is, per unit of distance.
    pub fn density(mut self, density: f64) -> Self {
        self.density = density;
        self
    }

    /// Sets the height the air thins out over, in world units.
    ///
    /// This is what puts mist on the ground rather than filling the world with it, and it is
    /// why a scene of water and a scene of a plain want different values: a lake wants a
    /// scale height of a few tens of units so the air lies on it, and a plain wants hundreds
    /// so the air is the same all the way up to the horizon.
    pub fn scale_height(mut self, scale_height: f64) -> Self {
        self.scale_height = scale_height;
        self
    }

    /// Returns the `WGSL` for this atmosphere, to hand to every stage that needs it.
    ///
    /// # Invariants
    ///
    /// The string is **self contained and stage independent**: it declares its own constants
    /// and its own functions, and reads nothing from the uniform, so the same string is valid
    /// in the lighting stage, the emission stage and the post stage without any of them
    /// having to supply anything. That is what lets a scene hand out one copy and be certain
    /// the three stages are describing the same air.
    ///
    /// The names are prefixed, because these are declared at module level beside a scene's
    /// own helpers and two functions with one name is a shader that will not compile.
    pub fn wgsl(&self) -> String {
        let triple = |colour: [f64; 3]| {
            format!(
                "vec3<f32>({:.6}, {:.6}, {:.6})",
                colour[0], colour[1], colour[2]
            )
        };
        let mut out = String::new();
        out.push_str(&format!(
            "const AIR_ZENITH: vec3<f32> = {};\n\
             const AIR_HORIZON: vec3<f32> = {};\n\
             const AIR_GROUND: vec3<f32> = {};\n\
             const AIR_CURVE: f32 = {DEFAULT_CURVE};\n\
             const AIR_DENSITY: f32 = {:.6};\n\
             const AIR_SCALE_HEIGHT: f32 = {:.6};\n",
            triple(self.zenith),
            triple(self.horizon),
            triple(self.ground),
            self.density,
            self.scale_height,
        ));
        if let Some(sun) = self.sun {
            out.push_str(&format!(
                "const SUN_DIRECTION: vec3<f32> = {};\n",
                triple(sun)
            ));
        }
        if let Some((colour, reach, strength)) = self.glow {
            out.push_str(&format!(
                "const SUN_GLOW_COLOUR: vec3<f32> = {};\n\
                 const SUN_GLOW_REACH: f32 = {reach:.4};\n\
                 const SUN_GLOW_STRENGTH: f32 = {strength:.4};\n",
                triple(colour),
            ));
        }
        if let Some((colour, sharpness)) = self.disc {
            out.push_str(&format!(
                "const SUN_DISC_COLOUR: vec3<f32> = {};\n\
                 const SUN_DISC: f32 = {sharpness:.8};\n\
                 const SUN_DISC_EDGE: f32 = {};\n",
                triple(colour),
                DISC_EDGE,
            ));
        }

        out.push_str(
            "\n\
             /// The colour of the air itself.\n\
             ///\n\
             /// This is the number a distant surface tends towards, and it is the same number\n\
             /// the sky reaches at its horizon. They are written once here rather than at the\n\
             /// two call sites because the two call sites are what disagree, and a comment\n\
             /// saying they should agree is not a constraint.\n\
             fn air_colour() -> vec3<f32> {\n\
             \x20   return AIR_HORIZON;\n\
             }\n\
             \n\
             /// The sky in a direction, as a function of the ray rather than of the pixel.\n\
             ///\n\
             /// A gradient taken off the screen coordinate is a backdrop: it stays put when\n\
             /// the camera moves, and a scene whose sky does not move with the camera is a\n\
             /// scene drawn in front of a photograph rather than in one.\n\
             fn air_sky(ray: vec3<f32>) -> vec3<f32> {\n\
             \x20   let above = pow(clamp(ray.y, 0.0, 1.0), AIR_CURVE);\n\
             \x20   var colour = mix(air_colour(), AIR_ZENITH, above);\n",
        );

        if self.sun.is_some() && self.glow.is_some() {
            out.push_str(
                "    // The warm band along the sun's bearing, hugging the horizon. This is the\n\
                 \x20   // brightest thing in most outdoor frames and it is only here, which is\n\
                 \x20   // what makes it read as a direction rather than as a bright patch.\n\
                 \x20   let toward = clamp(dot(ray, SUN_DIRECTION), 0.0, 1.0);\n\
                 \x20   colour = colour + SUN_GLOW_COLOUR * pow(toward, SUN_GLOW_REACH) * SUN_GLOW_STRENGTH;\n",
            );
        }
        if let Some((colour, _sharpness)) = self.disc {
            out.push_str(&format!(
                "    // The disc itself. It is added inside the glow rather than after it, so that\n\
                 \x20   // widening the glow does not widen the sun.\n\
                 \x20   let alignment = dot(ray, SUN_DIRECTION);\n\
                 \x20   colour = colour + {} * smoothstep(SUN_DISC, SUN_DISC + SUN_DISC_EDGE, alignment);\n",
                triple(colour),
            ));
        }
        out.push_str(
            "    // Below the horizon there is no sky, only more air. Stopping the ramp at the\n\
             \x20   // horizon instead would leave a hard line down the middle of a frame.\n\
             \x20   return select(\n\
             \x20       mix(air_colour(), AIR_GROUND, clamp(-ray.y * 4.0, 0.0, 1.0)),\n\
             \x20       colour,\n\
             \x20       ray.y > 0.0,\n\
             \x20   );\n\
             }\n\
             \n\
             /// How much of the view the air has closed between an eye and a point, from none\n\
             /// to one.\n\
             ///\n\
             /// The density is taken at the **midpoint** of the two, not at the surface, and\n\
             /// that is the whole of aerial perspective: a slope facing the camera is seen\n\
             /// through less air than the one behind it, so washing by distance alone flattens\n\
             /// a landscape, because the far side of a dune and its near side are the same\n\
             /// distance from the eye and not the same distance through the air.\n\
             fn air_closure(eye: vec3<f32>, point: vec3<f32>) -> f32 {\n\
             \x20   let distance = length(point - eye);\n\
             \x20   let height = max(0.5 * (eye.y + point.y), 0.0);\n\
             \x20   let thickness = exp(-height / AIR_SCALE_HEIGHT);\n\
             \x20   return clamp(1.0 - exp(-distance * AIR_DENSITY * thickness), 0.0, 1.0);\n\
             }\n",
        );
        if self.disc.is_some() {
            out.push_str("");
        }
        out
    }
}
