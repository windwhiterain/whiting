//! Bernstein basis coefficients, shared by every net in the crate.
//!
//! Two nets over different domains — a triangle and a rectangle — need the same three things: the
//! binomial coefficients, the simplex's multinomial, and the one-dimensional basis weights. Only
//! the last differs in how it is assembled, and only because the domains differ.

/// `C(n, k)`, accumulated as a product so no factorial overflows.
///
/// Exact for every `n` whose binomial coefficients stay under `2^53`, which covers any degree a
/// hand-edited ladder reaches; degrees are small here because elevation is the only way to add a
/// control point.
pub fn binomial(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.0;
    }
    let k = k.min(n - k);
    let mut result = 1.0;
    for step in 1..=k {
        result = result * (n - k + step) as f64 / step as f64;
    }
    result
}

/// `n! / (i! j! k!)` for `i + j + k == n`, as two binomials so nothing overflows.
pub fn multinomial(n: usize, i: usize, j: usize) -> f64 {
    binomial(n, i) * binomial(n - i, j)
}

/// The `degree + 1` Bernstein weights at `t`: index `k` holds `C(degree, k) t^k (1-t)^(degree-k)`.
///
/// Built from running products rather than `powi`, so there is no division by `(1 - t)` to go
/// singular at the ends of the interval and no repeated exponentiation per term. The weights sum to
/// one for `t` in `[0, 1]`, which is what makes a net's support a partition of unity and what lets a
/// whole support be translated without a solve.
pub fn basis(degree: usize, t: f64) -> Vec<f64> {
    let mut powers_t = vec![1.0; degree + 1];
    let mut powers_complement = vec![1.0; degree + 1];
    for step in 1..=degree {
        powers_t[step] = powers_t[step - 1] * t;
        powers_complement[step] = powers_complement[step - 1] * (1.0 - t);
    }
    (0..=degree)
        .map(|k| binomial(degree, k) * powers_t[k] * powers_complement[degree - k])
        .collect()
}
