//! The dawn over the rice terraces: one law, read twice.
//!
//! # Why this is a file and not a WGSL constant
//!
//! A sky written as a `WGSL` string is a sky only this engine can see. The tracer is handed a
//! **world** — an image, and a light that says how bright it is — and a scene whose sky lives
//! only in a shader has no world to hand it, so the two backends draw two different mornings
//! over one set of paddies. Measured on this scene before anything here: the rasterised frame
//! carried a pale dawn across its top and the traced one carried black, and the whole-frame
//! correlation was **-0.8264** — a picture whose bright part is the other picture's dark part.
//!
//! So the law is held here as **numbers**, and read out of them twice: as a WGSL function for
//! the stages that ask for a sky along a ray, and as a latlong map for the world. The two
//! readings share every field, which is the only reason they agree.
//!
//! # What this law is not
//!
//! It is not [`crate::air::Atmosphere`], and it is deliberately not an `Atmosphere` with more
//! fields. This sky's horizon **follows the sun around the compass** — the east burns and the
//! west holds the night's cool — and its glow is two lobes rather than one. Those are not
//! settings of the gallery's air; they are this valley's hour. `air.rs` grants a scene the right
//! to write its own law, and this is that right being used: the shared type keeps the sky it
//! can express, and a scene that wants another one does not push three single-scene knobs into
//! a type four other scenes read.
//!
//! # The weather is not here
//!
//! The stratus deck that drifts across this sky belongs to [`crate::terraces`] and is drawn by
//! the raster stage alone. It moves with the scene's clock, and a static map cannot carry a
//! thing that moves — so it stays in the stage that can move it, laid over whichever base the
//! stage is holding.

/// The sky over the terraces, as the numbers both readings are written from.
///
/// # Invariants
///
/// Every term has its own field and every field is read by **both** [`wgsl`](Self::wgsl) and
/// [`radiance`](Self::radiance). A field only one of them reads is a term the other backend
/// does not have, which is the defect this whole arrangement exists to remove.
#[derive(Debug, Clone, Copy)]
pub struct Sky {
    /// Where the sun stands, as a direction from the scene.
    sun: [f64; 3],
    /// The horizon's colour on the quarter away from the sun.
    horizon_away: [f64; 3],
    /// The horizon's colour on the sun's own bearing.
    horizon_toward: [f64; 3],
    /// How tightly the horizon's warmth gathers about the sun's bearing.
    bearing_reach: f64,
    /// The colour overhead.
    zenith: [f64; 3],
    /// What the air settles to below the horizon.
    ground: [f64; 3],
    /// How fast the ramp from horizon to zenith runs out.
    gradient_curve: f64,
    /// How fast the sky is replaced below the horizon.
    ground_rate: f64,
    /// The colour of the glow the sun lays along its bearing.
    glow: [f64; 3],
    /// How tightly each lobe of that glow gathers, outer first.
    glow_reach: [f64; 2],
    /// How bright each lobe of that glow is at its centre, matching `glow_reach`.
    glow_strength: [f64; 2],
    /// The colour of the sun's disc.
    disc: [f64; 3],
    /// How far the disc's radiance stands above one.
    disc_gain: f64,
    /// The cosines the disc's edge runs between.
    disc_edge: [f64; 2],
}

/// The dawn the rice terraces are drawn at, with the sun the scene already named.
///
/// The sun is **taken rather than owned**: the scene has one sun, and it is that same direction
/// in the light, the sky, the water and the fog. A second copy here would be a sun the map and
/// the lamp could disagree about, which is the one thing the scene's own comment says it cannot.
pub fn sky(sun: [f64; 3]) -> Sky {
    Sky {
        sun,
        horizon_away: [0.58, 0.63, 0.67],
        horizon_toward: [0.86, 0.66, 0.44],
        // Two, which is wide: the warmth is the whole horizon on the sun's side rather than a
        // band about it, and a paddy turned that way mirrors a burning sky rather than a streak.
        bearing_reach: 2.0,
        zenith: [0.11, 0.26, 0.58],
        ground: [0.42, 0.39, 0.34],
        gradient_curve: 0.42,
        ground_rate: 5.0,
        glow: [1.0, 0.52, 0.24],
        // Two lobes rather than one: a broad wash the sky is warm over, and a tight core that
        // makes the sun a thing in the sky instead of a bright quarter of it.
        glow_reach: [14.0, 90.0],
        glow_strength: [0.30, 2.2],
        disc: [1.0, 0.70, 0.42],
        disc_gain: 16.0,
        // A disc is a small solid angle, and the cosine across it stays within a few ten
        // thousandths of one, so the two edges are that close together.
        disc_edge: [0.99955, 0.99985],
    }
}

impl Sky {
    /// Returns the sky as a WGSL function of a ray, for the stages that ask for one.
    ///
    /// This is the **base** of the sky, before the stratus deck is laid over it. A stage that
    /// has no map — a surface shading a paddy — reads this; a stage that has one reads the map
    /// instead, so that its sky and the tracer's are the same image rather than the same law
    /// evaluated twice.
    pub fn wgsl(&self) -> String {
        let mut lobes = String::new();
        for lobe in 0..self.glow_reach.len() {
            if lobe > 0 {
                lobes.push_str(" + ");
            }
            lobes.push_str(&format!(
                "pow(to_sun, {}) * {}",
                number(self.glow_reach[lobe]),
                number(self.glow_strength[lobe])
            ));
        }
        format!(
            r#"
/// Returns the sky's own colour along a ray, before any weather is laid over it.
///
/// # Invariants
///
/// Every number below is [`Sky`](super::Sky)'s, formatted in rather than written here, so this
/// function and the world the tracer reads are one law rather than two that agree today.
fn sky_base(ray: vec3<f32>) -> vec3<f32> {{
    let sun = {sun};
    let rise = clamp(ray.y, -1.0, 1.0);
    // The horizon's warmth follows the sun around the compass: the east burns, the west holds
    // the night's cool a while longer, and a paddy mirrors whichever quarter it turns to.
    let flat_ray = ray.xz / max(length(ray.xz), 0.01);
    let flat_sun = sun.xz / length(sun.xz);
    let toward = dot(flat_ray, flat_sun) * 0.5 + 0.5;
    let horizon = mix({away}, {toward_colour}, pow(toward, {bearing}));
    let zenith = {zenith};
    let below = {ground};
    let above = pow(clamp(rise, 0.0, 1.0), {curve});
    var sky = select(
        mix(horizon, below, clamp(-rise * {rate}, 0.0, 1.0)),
        mix(horizon, zenith, above),
        rise > 0.0,
    );
    // The sun's glow, held close about the disk so the blaze in a paddy is an arc of light
    // and not a wash, and the disk itself, small and fierce.
    let to_sun = max(dot(ray, sun), 0.0);
    sky = sky + {glow} * ({lobes});
    sky = sky + {disc} * smoothstep({disc_inner}, {disc_outer}, to_sun) * {disc_gain};
    return sky;
}}
"#,
            sun = triple(self.sun),
            away = triple(self.horizon_away),
            toward_colour = triple(self.horizon_toward),
            bearing = number(self.bearing_reach),
            zenith = triple(self.zenith),
            ground = triple(self.ground),
            curve = number(self.gradient_curve),
            rate = number(self.ground_rate),
            glow = triple(self.glow),
            lobes = lobes,
            disc = triple(self.disc),
            disc_inner = number(self.disc_edge[0]),
            disc_outer = number(self.disc_edge[1]),
            disc_gain = number(self.disc_gain),
        )
    }

    /// Returns the same sky as a radiance along a ray.
    ///
    /// # Invariants
    ///
    /// Term for term with [`wgsl`](Self::wgsl), in the same order, from the same fields. The
    /// **branch is the same branch**: above the horizon the sky ramps to the zenith and below
    /// it is replaced by the ground colour, which is what stops the glow from leaking under the
    /// horizon and lighting the ground from below.
    pub fn radiance(&self, ray: [f64; 3]) -> [f64; 3] {
        let rise = ray[1].clamp(-1.0, 1.0);
        let flat = (ray[0] * ray[0] + ray[2] * ray[2]).sqrt();
        let flat_ray = if flat > 0.01 {
            [ray[0] / flat, ray[2] / flat]
        } else {
            [0.0, 0.0]
        };
        let sun_flat = (self.sun[0] * self.sun[0] + self.sun[2] * self.sun[2]).sqrt();
        let flat_sun = [self.sun[0] / sun_flat, self.sun[2] / sun_flat];
        let toward = (flat_ray[0] * flat_sun[0] + flat_ray[1] * flat_sun[1]) * 0.5 + 0.5;

        let horizon = mix3(
            self.horizon_away,
            self.horizon_toward,
            toward.powf(self.bearing_reach),
        );
        let mut sky = if rise > 0.0 {
            mix3(horizon, self.zenith, rise.powf(self.gradient_curve))
        } else {
            mix3(
                horizon,
                self.ground,
                (-rise * self.ground_rate).clamp(0.0, 1.0),
            )
        };

        let to_sun = (ray[0] * self.sun[0] + ray[1] * self.sun[1] + ray[2] * self.sun[2]).max(0.0);
        for (channel, value) in sky.iter_mut().enumerate() {
            let mut glow = 0.0;
            for lobe in 0..self.glow_reach.len() {
                glow += to_sun.powf(self.glow_reach[lobe]) * self.glow_strength[lobe];
            }
            *value += self.glow[channel] * glow;
            *value += self.disc[channel]
                * smoothstep(self.disc_edge[0], self.disc_edge[1], to_sun)
                * self.disc_gain;
        }
        sky
    }

    /// Returns this sky as a latlong map, the world a tracer is handed.
    pub fn pixels(&self, width: i64, height: i64) -> Vec<u8> {
        crate::shaders::environment_pixels_of(width, height, |ray| self.radiance(ray))
    }
}

/// Formats a number as a WGSL float literal, to the four places the scene's own tokens carry.
fn number(value: f64) -> String {
    format!("{value:.4}")
}

/// Formats a three component vector as a WGSL `vec3<f32>` literal.
fn triple(components: [f64; 3]) -> String {
    format!(
        "vec3<f32>({:.4}, {:.4}, {:.4})",
        components[0], components[1], components[2]
    )
}

/// Returns one colour mixed from one to another, the way `mix` does in the shader.
fn mix3(from: [f64; 3], to: [f64; 3], amount: f64) -> [f64; 3] {
    [
        from[0] + (to[0] - from[0]) * amount,
        from[1] + (to[1] - from[1]) * amount,
        from[2] + (to[2] - from[2]) * amount,
    ]
}

/// Returns zero below `edge0`, one above `edge1`, and a smooth ramp between.
fn smoothstep(edge0: f64, edge1: f64, value: f64) -> f64 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
