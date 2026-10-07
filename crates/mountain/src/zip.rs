//! The whole triangulator: two polylines become the steps of a triangle strip.
//!
//! # Why there is no search here
//!
//! A belt is bounded by a crest and a valley floor, and the mesh between them is a strip of
//! triangles that walks both. Walking is O(vertices), it needs no two boundaries to agree on
//! a vertex count, and the strip's own boundary *is* the input polyline — which is why the
//! two valleys facing one ridge may subdivide it differently and still meet it exactly. A
//! triangulation search would make all three of those a property of the search rather than of
//! the construction. See [SPEC/mountain.md](../../../.agents/SPEC/mountain.md) R4.1.

use std::collections::BTreeSet;

use crate::math::Vec3;
use crate::tessellation::DiagonalRule;

/// How close two boundaries' progress must be before [`DiagonalRule::Alternate`] will
/// override the rule.
///
/// # Invariants
///
/// It is a **band and not a point**. Two arc-length progressions are equal to the last bit
/// only when the two boundaries are sampled identically, so a rule that fired on an exact tie
/// would be a rule that never fires on the flanks it exists for. A tenth of the boundary is
/// wide enough to catch a belt whose two sides were sampled to within a tenth of each other
/// and narrow enough that a flank which genuinely leans is still stepped by
/// [`DiagonalRule::Progress`].
const TIE_BAND: f32 = 0.1;

/// Which boundary the strip stepped along.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The strip advanced along the top boundary.
    Top,
    /// The strip advanced along the bottom boundary.
    Bottom,
}

/// Returns the steps that triangulate the strip between two polylines.
///
/// # Invariants
///
/// The steps consume both boundaries exactly once, so the triangles built from them cover the
/// strip and their outer edges are the two boundaries themselves. Every cell is reached, and
/// the strip never leaves either boundary behind by more than one vertex — that is what keeps
/// a belt from folding over itself when one boundary is sampled much more finely than the
/// other.
///
/// `forced` holds the step positions whose direction the author has overridden. An override
/// asks for the **other** step at that position, and is ignored where the other step is not
/// available, which is the only honest reading of an override on a boundary that has run out.
pub fn steps(
    top: &[Vec3],
    bottom: &[Vec3],
    rule: DiagonalRule,
    forced: &BTreeSet<usize>,
) -> Vec<Step> {
    let mut steps = Vec::with_capacity(top.len() + bottom.len());
    if top.is_empty() || bottom.is_empty() {
        return steps;
    }
    let top_span = cumulative(top);
    let bottom_span = cumulative(bottom);
    let (mut index, mut along) = (0usize, 0usize);
    let mut last: Option<Step> = None;

    while index + 1 < top.len() || along + 1 < bottom.len() {
        let can_top = index + 1 < top.len();
        let can_bottom = along + 1 < bottom.len();
        let step = if !can_bottom {
            Step::Top
        } else if !can_top {
            Step::Bottom
        } else {
            let top_progress = top_span[index] / top_span[top_span.len() - 1];
            let bottom_progress = bottom_span[along] / bottom_span[bottom_span.len() - 1];
            let behind = if top_progress < bottom_progress {
                Step::Top
            } else {
                Step::Bottom
            };
            let chosen = match rule {
                DiagonalRule::Progress => behind,
                DiagonalRule::Alternate => {
                    let tied = (top_progress - bottom_progress).abs() <= TIE_BAND;
                    match last {
                        Some(previous) if tied => match previous {
                            Step::Top => Step::Bottom,
                            Step::Bottom => Step::Top,
                        },
                        _ => behind,
                    }
                }
            };
            if forced.contains(&steps.len()) {
                match chosen {
                    Step::Top => Step::Bottom,
                    Step::Bottom => Step::Top,
                }
            } else {
                chosen
            }
        };
        match step {
            Step::Top => index += 1,
            Step::Bottom => along += 1,
        }
        last = Some(step);
        steps.push(step);
    }
    steps
}

/// Returns the cumulative arc length of a polyline in the plan.
fn cumulative(points: &[Vec3]) -> Vec<f32> {
    let mut span = Vec::with_capacity(points.len());
    let mut total = 0.0;
    span.push(0.0);
    for pair in points.windows(2) {
        total += crate::math::distance([pair[0][0], pair[0][2]], [pair[1][0], pair[1][2]]);
        span.push(total);
    }
    // A boundary with no length at all — a corner where both stations name one point — is a
    // single vertex, and dividing by its total would be dividing by zero. Every vertex is
    // then at progress one, so the other boundary is always the one behind and the strip
    // becomes the fan that closes the corner.
    if total <= f32::EPSILON {
        for value in span.iter_mut() {
            *value = 1.0;
        }
    }
    span
}
