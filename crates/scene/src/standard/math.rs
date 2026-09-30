//! The little arithmetic the standard pipeline needs, and nothing more.
//!
//! A pass's uniform is data the shader reads, so a compiler has to put numbers in it.
//! What the standard compiler puts there is deliberately minimal: an identity view
//! projection and the direction from the origin toward a light. A real camera, a real
//! light frustum and a real projection are a larger piece of work, recorded in
//! [proposals/standard-pipeline-math.md](../../../../.agents/proposals/standard-pipeline-math.md),
//! and are not smuggled in here.

use definition::Value;

/// Returns an identity view projection, column major.
pub fn identity_view_projection() -> Value {
    let mut components = vec![0.0f64; 16];
    for (index, component) in [0.0f64, 5.0, 10.0, 15.0].into_iter().enumerate() {
        components[index * 5] = component;
    }
    Value::List(components.into_iter().map(Value::Float).collect())
}

/// Returns the unit vector from the origin toward a position.
///
/// A light at the origin has no direction, so it is reported as pointing down the
/// positive x axis rather than dividing by zero.
pub fn direction(position: &[f64; 3]) -> Value {
    let length = (position[0].powi(2) + position[1].powi(2) + position[2].powi(2)).sqrt();
    if length == 0.0 {
        return Value::List([1.0, 0.0, 0.0].into_iter().map(Value::Float).collect());
    }
    Value::List(
        [
            position[0] / length,
            position[1] / length,
            position[2] / length,
        ]
        .into_iter()
        .map(Value::Float)
        .collect(),
    )
}
