//! What a light's stated brightness means once it reaches a surface.
//!
//! # Why this file exists
//!
//! A scene states a lamp's **radiant flux** — `power`, in the units of
//! [`crate::standard::scene_schema`]. Radiance is what a surface sends onwards, and the two are
//! not the same number: a lamp's flux is spread over a sphere of area `4 pi d^2` to become an
//! irradiance, and a Lambertian surface turns an irradiance into a radiance by dividing by
//! `pi` again. The product of those two factors is the `4 pi^2` below.
//!
//! It used to be missing, and the two backends disagreed by exactly that much. Measured against
//! `cycles.exe` 5.3.0 on the `tiles` scene: a white floor lit by one point lamp of `power = 3000`
//! about fifteen units away came back at a peak of 71/255, and `peak * d^2 / cos` was constant
//! down the frame at `69`, which is `3000 / 43.5`. Physical single-bounce radiance for that lamp
//! is `cos * 3000 / (4 pi^2 * d^2)`, and `4 pi^2` is `39.5`. Cycles' `Light::strength` is radiant
//! flux, its image is scene-linear with no view transform, and so the whole of the discrepancy
//! was ours: a shading model that divided by `d^2` and then by nothing else.
//!
//! The scenes had been calibrated against *that* model, by writing `power` as roughly `d^2` so
//! that `power / d^2` came out at 1.0 on the subject. The rasterised pipeline's post stage ends
//! in a Reinhard curve, `(L / (L + 1))^0.4545`, which turns 1.0 into 186/255 and makes 1.0 read
//! as "correctly exposed". Cycles applies no such curve — its film has `exposure` and nothing
//! else — so the same document came back at 1.0, which is 1/255, and a black frame.
//!
//! # What was changed, and why the pictures did not move
//!
//! The normalisation went into the shading model **and** every scene's `power` was multiplied by
//! `4 pi^2`. The two cancel: `power / (4 pi^2 d^2)` with `power` raised by `4 pi^2` is
//! `power / d^2`, so the rasterised pipeline produces the same linear radiance it always did, and
//! the post stage sees the same input. What changed is that the number is now a physical one, so
//! a backend that knows the physics — a path tracer, or any renderer with a real BRDF — lands on
//! the same value without being told about this renderer's history.
//!
//! See `.agents/proposals/physical-light-units.md` for the decision to state these units at all,
//! and `.agents/investigate/cycles-light-units.md` for the measurements.
//!
//! # Invariants
//!
//! - **The value is written once here.** The gallery's lighting template and the example's copy of
//!   it are two renderings of the same shader, and a constant written into both is two constants
//!   that will disagree the first time one is edited.
//! - **It is a `f32`-exact decimal in WGSL and an `f64` here, and the two are equal.** The shader
//!   is given the digits below rather than an expression to evaluate, so a backend that reads the
//!   document's own reasoning is not reading a different rounding of it.

/// The factor that turns a lamp's radiant flux into the radiance it delivers to a
/// Lambertian surface: `1 / (4 pi^2)`.
///
/// # Why `4 pi^2` and not `pi`
///
/// Two separate divisions, from two separate places, and a document's `power` is upstream of
/// both. A lamp emitting flux `Phi` delivers irradiance `Phi / (4 pi d^2)` to a surface `d`
/// away facing it, because the flux is spread over a sphere. A surface with albedo `rho` facing
/// that irradiance returns radiance `rho * E / pi`, because a Lambertian lobe distributes over a
/// hemisphere and the cosine-weighted integral of the lobe is `pi`. Multiplying gives
/// `rho * Phi * cos / (4 pi^2 d^2)`.
///
/// # Why the shading model needed it and Cycles did not
///
/// Cycles' `PointLight::copy_to_kernel` (`src/scene/light.cpp`) already normalises: it divides
/// by the lamp's own area and by `pi`, so a lamp's `strength` there is radiant flux and no
/// conversion belongs in an exporter. This constant is therefore the *other* half of the same
/// statement — the half that belongs to a renderer whose BRDF omits the `1 / pi`.
pub const FLUX_TO_RADIANCE: f64 = 1.0 / (4.0 * std::f64::consts::PI * std::f64::consts::PI);

/// [`FLUX_TO_RADIANCE`] as the digits a WGSL `const` is given.
///
/// # Why the digits and not an expression
///
/// WGSL has `const`, so `const FLUX_TO_RADIANCE = 1.0 / (4.0 * PI * PI);` would compile and would
/// be the clearest statement of what the number is. It is written out anyway, for one reason: a
/// shader that is generated rather than written is something a person reads to answer "where did
/// this come from", and a value they have to evaluate is a value they cannot check against
/// [`FLUX_TO_RADIANCE`] at a glance. `0.025330295910584444` is `1 / (4 pi^2)` to the last bit of
/// an `f64`, and the `f32` the shader holds is the nearest one to it.
pub const FLUX_TO_RADIANCE_DIGITS: &str = "0.025330295910584444";

/// Multiplies a stated radiant flux by [`FLUX_TO_RADIANCE`].
///
/// # Invariants
///
/// A scene's `power` is a flux and never a radiance, so this is the only way one becomes the
/// other. A caller that has already converted must not call it again, and there is nothing in the
/// number that can tell the two apart — which is why it is a function with a name rather than a
/// constant multiplied at a call site.
pub fn radiance_from_flux(power: f64) -> f64 {
    power * FLUX_TO_RADIANCE
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shader is given digits, and they have to be *this* constant.
    ///
    /// A WGSL `const` is a literal in a generated string, so nothing checks it against the Rust
    /// side except this. If the two drift, the rasterised pipeline and the shader the docs
    /// describe disagree by a factor nobody would see in a picture.
    #[test]
    fn the_shader_digits_are_the_constant() {
        let written: f64 = FLUX_TO_RADIANCE_DIGITS
            .parse()
            .expect("the digits are a number");
        assert_eq!(written, FLUX_TO_RADIANCE);
    }

    /// `1 / (4 pi^2)` and the two factors it is the product of, checked separately.
    ///
    /// The derivation in the module documentation is three divisions and a multiplication, and
    /// each of them can be got wrong in a way that a wrong constant still satisfies.
    #[test]
    fn the_constant_is_the_product_of_the_sphere_and_the_hemisphere() {
        let pi = std::f64::consts::PI;
        assert!((FLUX_TO_RADIANCE - 1.0 / (4.0 * pi * pi)).abs() < 1e-15);
        assert!((FLUX_TO_RADIANCE - (1.0 / (4.0 * pi)) * (1.0 / pi)).abs() < 1e-15);
    }

    /// A flux and the radiance it delivers, at the distance the law is stated at.
    ///
    /// One hundred watts at ten units, facing it square on, is `100 / (4 pi^2 * 100)`, which is
    /// `1 / (4 pi^2)` again. The point of the case is that the distance cancels: a lamp four
    /// times further away and sixteen times brighter lands on the same radiance, which is the
    /// inverse square law and the reason a scene states a power rather than a brightness.
    #[test]
    fn radiance_falls_with_the_square_of_the_distance() {
        let near = radiance_from_flux(100.0) / 10.0_f64.powi(2);
        let far = radiance_from_flux(1600.0) / 40.0_f64.powi(2);
        assert!((near - far).abs() < 1e-12);
    }
}
