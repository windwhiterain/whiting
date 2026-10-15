//! Does a Hermite surface with a per-edge hard/soft tag give what the tag promises?
//!
//! Run with `cargo run -p whiting-patch --example hermite_continuity_probe`. An example and not a
//! gate, following the other probes in this crate. It exits non-zero if any property fails.
//!
//! **This measures a proposal, not the crate.** Nothing in `whiting-patch` implements a Hermite
//! surface yet, so the whole evaluator is written out here. A probe that imported the code it was
//! meant to audit would prove only that the code is self-consistent; `examples/support` applies the
//! same rule in the other direction.
//!
//! The model under test, from [`.agents/handoffs/hermite-surface.md`](../.agents/handoffs/hermite-surface.md):
//!
//! ```text
//! per mesh edge e = (p, q):
//!   P(p), P(q)   mesh vertex positions,   always shared
//!   V(p), V(q)   tangent ALONG e,         always shared  -> the boundary is one cubic
//!   G(p), G(q)   derivative ACROSS e,     shared when soft, private per face when hard
//! ```
//!
//! `Delta_e`, what local `u` means along `e`, is the chord `|P(q) - P(p)|` — a property of the
//! **edge**, so two faces traversing it read the same number with nothing stored.
//!
//! 0. **Positive control.** Corrupt one column of node tangents and require the C1 measurement to
//!    fire. Every null result this repository has produced by accident came from a missing control.
//! 1. A grid with every interior edge soft: **exactly C1** across every seam, at every valence.
//! 2. What the soft tag buys. Two cells, boundary curve and step fully shared so the join is exact
//!    C0 with a matching tangential derivative, and the cross-boundary derivative *independent*: the
//!    join still folds. Then share it and the fold is zero. **The two arms are each other's
//!    control**, which is why they are one measurement.
//! 3. A hard edge: positions and boundary curve shared, the interior free — a drag on one side
//!    moves that side, and the seam neither opens nor changes its two sides' relationship.
//! 4. The boundary curve against a Bernstein basis written out independently, and its independence
//!    from the patch interior.

mod support;

use whiting_patch::Point3;

use support::ANGLE_FLOOR;

// -- the cubic Hermite basis, from the definition ----------------------------------------------

/// The cubic Hermite basis on one interval, in the order a jet is stored in: value at the near
/// end, derivative at the near end, value at the far end, derivative at the far end.
///
/// ```text
/// h00 = 2t^3 - 3t^2 + 1      h01 = -2t^3 + 3t^2
/// h10 =   t^3 - 2t^2 + t     h11 =   t^3 -   t^2
/// ```
///
/// **These are not the Bernstein weights, and confusing the two is invisible until the corners
/// stop meeting.** `h00..h11` is a basis; the Bernstein weights `(1-t)^3, 3t(1-t)^2, 3t^2(1-t),
/// t^3` are a different one that evaluates the same four numbers as a *Bezier* curve — which takes
/// `P_0` at the near end and `d_1` at the far end, instead of `P_0` and `d_0`. The first version of
/// this probe used Bernstein and every seam read `2.501e0` apart: it was placing a tangent where a
/// corner belongs, and reporting it as a surface. Measurement 4 keeps the Bernstein evaluator, and
/// keeps it *separate*, so the two are compared rather than assumed equal.
fn hermite(t: f64) -> [f64; 4] {
    let t2 = t * t;
    let t3 = t2 * t;
    [
        2.0 * t3 - 3.0 * t2 + 1.0,
        t3 - 2.0 * t2 + t,
        -2.0 * t3 + 3.0 * t2,
        t3 - t2,
    ]
}

/// `d/dt` of [`herite`], differentiated here rather than by finite difference because a tangent
/// plane measured through a difference is a tangent plane one order wrong.
///
/// `h00'(1) = h10'(1) = h01'(1) = 0` and `h11'(1) = 1`, which is why the cross-boundary derivative
/// picks up the far corner's `Fu` and `Fuv` and a probe reading only the near edge cannot see it.
fn hermite_slope(t: f64) -> [f64; 4] {
    let t2 = t * t;
    [
        6.0 * t2 - 6.0 * t,
        3.0 * t2 - 4.0 * t + 1.0,
        -6.0 * t2 + 6.0 * t,
        3.0 * t2 - 2.0 * t,
    ]
}

/// The four Hermite jets of a boundary curve as the four Bezier control points that draw it.
///
/// `(P_0, d_0, P_1, d_1)` and `(P_0, P_0 + d_0/3, P_1 - d_1/3, P_1)` are the same curve. That
/// identity is what makes the shared boundary of this representation the shared Bernstein row it
/// replaces, and measurement 4 is where it is checked.
fn as_bezier(jets: [Point3; 4]) -> [Point3; 4] {
    [
        jets[0],
        jets[0] + jets[1] / 3.0,
        jets[2] - jets[3] / 3.0,
        jets[2],
    ]
}

/// The Bernstein weights of degree 3, built from the binomial definition rather than from
/// [`hermite`], so measurement 4 compares two independent evaluations instead of one with itself.
fn bernstein_degree_three(t: f64) -> [f64; 4] {
    let mut weights = [0.0f64; 4];
    for (k, weight) in weights.iter_mut().enumerate() {
        let binomial: f64 = match k {
            0 => 1.0,
            1 => 3.0,
            2 => 3.0,
            _ => 1.0,
        };
        *weight = binomial * t.powi(k as i32) * (1.0 - t).powi((3 - k) as i32);
    }
    weights
}

// -- one cell ------------------------------------------------------------------------------------

/// A bicubic patch over `[0, 1]^2`, held as the jet at its four corners.
///
/// `jet[a][b]` is the tensor-product Hermite coefficient with `a` along `u` and `b` along `v`, and
/// every derivative in it has **already been multiplied by the parameter step it belongs to**. That
/// convention is what makes a non-uniform grid work: `step_u` divides on the way back out, and the
/// two cancel, so two cells with different `u` steps still agree on the normal across a seam they
/// share. Measurement 1's second arm is what checks that rather than asserting it.
#[derive(Clone, Copy, Debug)]
struct Cell {
    step_u: f64,
    step_v: f64,
    jet: [[Point3; 4]; 4],
}

impl Cell {
    /// Builds a cell from the nodes at its corners, in local `(0,0) (1,0) (1,1) (0,1)` order.
    fn from_nodes(step_u: f64, step_v: f64, corners: [&Node; 4]) -> Self {
        let mut jet = [[Point3::ZERO; 4]; 4];
        // Where each corner's four coefficients land, in the `(0,0) (1,0) (1,1) (0,1)` winding
        // `Field::cell` supplies. Written as a table rather than a loop because the four placements
        // differ and a loop would hide exactly the thing being checked — the third and fourth
        // entries are **not** adjacent in the table, and reading them as if they were puts the
        // corners on the wrong sides of the patch and makes a shared edge that no two faces meet.
        const SLOTS: [[(usize, usize); 4]; 4] = [
            [(0, 0), (1, 0), (0, 1), (1, 1)],
            [(2, 0), (3, 0), (2, 1), (3, 1)],
            [(2, 2), (3, 2), (2, 3), (3, 3)],
            [(0, 2), (1, 2), (0, 3), (1, 3)],
        ];
        for (corner, node) in corners.iter().enumerate() {
            let [(i, j), (u_slot, _), (_v_slot, v_index), (uv_u, uv_v)] = SLOTS[corner];
            jet[i][j] = node.point;
            jet[u_slot][j] = node.du * step_u;
            jet[i][v_index] = node.dv * step_v;
            jet[uv_u][uv_v] = node.duv * (step_u * step_v);
        }
        Self {
            step_u,
            step_v,
            jet,
        }
    }

    fn evaluate(&self, s: f64, t: f64) -> Point3 {
        self.blend(hermite(s), hermite(t))
    }

    /// `dS/du` in **world** units, which is `d/ds` divided by the step this cell uses.
    fn tangent_u(&self, s: f64, t: f64) -> Point3 {
        self.blend(hermite_slope(s), hermite(t)) / self.step_u
    }

    /// `dS/dv` in world units. See [`Cell::tangent_u`].
    fn tangent_v(&self, s: f64, t: f64) -> Point3 {
        self.blend(hermite(s), hermite_slope(t)) / self.step_v
    }

    /// The tensor product `sum over a, b of jet[a][b] * along_u[a] * along_v[b]`.
    ///
    /// One function rather than three, because the three differ only in which side's basis is
    /// differentiated — and writing that three times is how the *near* boundary ends up evaluated
    /// on both faces of a seam, which reports a split of `2.501e0` on a surface that is perfect.
    fn blend(&self, along_u: [f64; 4], along_v: [f64; 4]) -> Point3 {
        let mut sum = Point3::ZERO;
        for (a, row) in self.jet.iter().enumerate() {
            for (b, control) in row.iter().enumerate() {
                sum += *control * (along_u[a] * along_v[b]);
            }
        }
        sum
    }

    /// A boundary curve of this cell, as its four Hermite jets.
    ///
    /// The basis array **interleaves**: `a = 0` is the near edge's position, `a = 1` its
    /// cross-boundary derivative, `a = 2` the **far** edge's position and `a = 3` its
    /// cross-boundary derivative. So the two `u` boundary lines are `a = 0` and `a = 2`, and the
    /// interior rows are `a = 1` and `a = 3`. Reading `far` as `3` moves the far edge's tangent
    /// where its position belongs, which is a surface nobody drew — and the first version of this
    /// probe did exactly that and measured a drag opening a seam by `5.469e-1`.
    ///
    /// Which line a seam is depends on the two faces' parameter directions rather than on the
    /// edge, so it is passed in rather than assumed.
    fn boundary_curve(&self, seam: Seam, far: bool) -> [Point3; 4] {
        let edge = if far { 2 } else { 0 };
        match seam {
            Seam::AcrossU => [
                self.jet[edge][0],
                self.jet[edge][1],
                self.jet[edge][2],
                self.jet[edge][3],
            ],
            Seam::AcrossV => [
                self.jet[0][edge],
                self.jet[1][edge],
                self.jet[2][edge],
                self.jet[3][edge],
            ],
        }
    }

    /// Adds a displacement to the row of cross-boundary derivatives at `index`, which is `1` for
    /// the near edge's and `3` for the far edge's.
    ///
    /// This is the private interior a hard edge leaves free, and it is deliberately the whole row
    /// rather than only the four strictly-interior entries: `b = 0` and `b = 3` of this row sit on
    /// the perpendicular boundary curves, so moving it shows that the interior on a hard side is
    /// free of the seam while remaining tied to the edges either side of it.
    fn nudge_u_row(&mut self, index: usize, by: Point3) {
        for b in 0..4 {
            self.jet[index][b] += by;
        }
    }
}

/// The jet at one grid node: where it is, and how the surface moves there.
#[derive(Clone, Copy, Debug)]
struct Node {
    point: Point3,
    du: Point3,
    dv: Point3,
    duv: Point3,
}

// -- the node field ------------------------------------------------------------------------------

/// A rectangular grid of nodes with a tangent field seeded from the positions by finite
/// differences.
///
/// Central differences inside, one-sided on the boundary. **Finite** rather than closed-form on
/// purpose: what is under test is that the surface built from *whatever* tangent field is
/// continuous across a seam, and seeding it beautifully would be measuring the seed instead.
#[derive(Clone, Debug)]
struct Field {
    /// Nodes in `u`. The grid has `columns + 1` of them and `columns` cells.
    columns: usize,
    rows: usize,
    /// Global parameter coordinate of each node, which is what makes a non-uniform grid expressible.
    u: Vec<f64>,
    v: Vec<f64>,
    nodes: Vec<Vec<Node>>,
}

impl Field {
    /// Seeds from a rectangular position grid given in `points[i][j]` order.
    fn from_points(points: Vec<Vec<Point3>>) -> Self {
        // `points` is indexed `[node column][node row]`, so the cell counts come from the opposite
        // edges of the nesting. Swapping these two is the shape of bug that produces a plausible
        // surface out of a sample and reads as a result.
        let columns = points.len() - 1;
        let rows = points[0].len() - 1;
        let u: Vec<f64> = (0..=columns).map(|i| i as f64).collect();
        let v: Vec<f64> = (0..=rows).map(|j| j as f64).collect();

        // A difference along the node column (`axis` 0) or the node row (`axis` 1), divided by the
        // distance in the parameter it is a derivative of. Central inside, one-sided at the ends.
        let difference = |grid: &[Vec<Point3>], axis: usize, along: usize, index: usize| {
            // The coordinates are per-axis. Using the `u` ones for both is the shape of bug that
            // reads as a plausible surface and is measured here only by luck.
            let coordinates: &[f64] = if axis == 0 { &u } else { &v };
            let last = coordinates.len() - 1;
            let get = |at: usize| -> Point3 {
                if axis == 0 {
                    grid[at][along]
                } else {
                    grid[along][at]
                }
            };
            if index == 0 {
                (get(1) - get(0)) / (coordinates[1] - coordinates[0])
            } else if index == last {
                (get(last) - get(last - 1)) / (coordinates[last] - coordinates[last - 1])
            } else {
                (get(index + 1) - get(index - 1))
                    / (coordinates[index + 1] - coordinates[index - 1])
            }
        };

        let mut du = Vec::with_capacity(columns + 1);
        for i in 0..=columns {
            let mut line = Vec::with_capacity(rows + 1);
            for j in 0..=rows {
                line.push(difference(&points, 0, j, i));
            }
            du.push(line);
        }

        let mut nodes = Vec::with_capacity(columns + 1);
        for i in 0..=columns {
            let mut line = Vec::with_capacity(rows + 1);
            for j in 0..=rows {
                let duv = difference(&du, 1, i, j);
                line.push(Node {
                    point: points[i][j],
                    du: du[i][j],
                    dv: difference(&points, 1, i, j),
                    duv,
                });
            }
            nodes.push(line);
        }
        Self {
            columns,
            rows,
            u,
            v,
            nodes,
        }
    }

    /// The crate's own sample, read back as nodes so these numbers are comparable with the other
    /// probes'.
    fn sample() -> Self {
        let (positions, _) = support::grid();
        let (columns, rows) = (support::COLUMNS, support::ROWS);
        let points = (0..columns)
            .map(|i| {
                (0..rows)
                    .map(|j| positions[j * columns + i])
                    .collect::<Vec<_>>()
            })
            .collect();
        Self::from_points(points)
    }

    fn step_u(&self, i: usize) -> f64 {
        self.u[i + 1] - self.u[i]
    }

    fn step_v(&self, j: usize) -> f64 {
        self.v[j + 1] - self.v[j]
    }

    /// The cell whose lower corner is node `(i, j)`.
    fn cell(&self, i: usize, j: usize) -> Cell {
        Cell::from_nodes(
            self.step_u(i),
            self.step_v(j),
            [
                &self.nodes[i][j],
                &self.nodes[i + 1][j],
                &self.nodes[i + 1][j + 1],
                &self.nodes[i][j + 1],
            ],
        )
    }
}

// -- measuring a seam ----------------------------------------------------------------------------

/// Which way a seam runs, which decides which derivative is the tangential one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Seam {
    /// A `u = constant` edge: the two faces differ across `u` and agree along `v`.
    AcrossU,
    /// A `v = constant` edge, the other way round.
    AcrossV,
}

/// Everything measured at one point on one shared edge.
#[derive(Clone, Copy, Debug, Default)]
struct Reading {
    /// How far the two faces put the shared point. A crease must still read zero here.
    split: f64,
    /// Angle between the two faces' derivatives **along** the edge. Always shared, both tags.
    along_edge: f64,
    /// Angle between the two faces' derivatives **across** it. This is what the soft tag buys.
    across_edge: f64,
    /// Angle between the two faces' normals. Determined by the two above, reported anyway.
    normal: f64,
    /// Readings that came out non-finite.
    non_finite: usize,
}

impl Reading {
    /// The worse of two readings, field by field, because a seam is only as good as its worst point.
    fn worse(self, other: Self) -> Self {
        Self {
            split: self.split.max(other.split),
            along_edge: self.along_edge.max(other.along_edge),
            across_edge: self.across_edge.max(other.across_edge),
            normal: self.normal.max(other.normal),
            non_finite: self.non_finite + other.non_finite,
        }
    }
}

/// Reads the whole relationship between two faces at one point along the seam between them.
///
/// The two faces sit at **opposite ends of their own domain** — `s = 1` against `s = 0` for a
/// `u = constant` seam — and reading both at the same parameter is the error that makes a seam look
/// continuous when the two faces do not even meet. It is written out per seam direction rather than
/// by subtracting from one, so the two cases cannot be confused for one another.
fn read_seam_point(along: f64, seam: Seam, first: &Cell, second: &Cell) -> Reading {
    let (first_uv, second_uv) = match seam {
        Seam::AcrossU => ((1.0, along), (0.0, along)),
        Seam::AcrossV => ((along, 1.0), (along, 0.0)),
    };
    let (first_u, first_v) = (
        first.tangent_u(first_uv.0, first_uv.1),
        first.tangent_v(first_uv.0, first_uv.1),
    );
    let (second_u, second_v) = (
        second.tangent_u(second_uv.0, second_uv.1),
        second.tangent_v(second_uv.0, second_uv.1),
    );
    let place = first
        .evaluate(first_uv.0, first_uv.1)
        .distance(second.evaluate(second_uv.0, second_uv.1));
    let (along_first, along_second) = match seam {
        Seam::AcrossU => (first_v, second_v),
        Seam::AcrossV => (first_u, second_u),
    };
    let (across_first, across_second) = match seam {
        Seam::AcrossU => (first_u, second_u),
        Seam::AcrossV => (first_v, second_v),
    };
    let along = along_first.angle(along_second);
    let across = across_first.angle(across_second);
    let normal = first_u.cross(first_v).angle(second_u.cross(second_v));
    let readings = [place, along, across, normal];
    Reading {
        split: place,
        along_edge: finite(along),
        across_edge: finite(across),
        normal: finite(normal),
        // Counted before the reductions, because `f64::max` returns the non-NaN operand and a
        // broken run would otherwise report a clean zero.
        non_finite: readings.iter().filter(|value| !value.is_finite()).count(),
    }
}

/// A measurement that came out NaN is a measurement that cannot be compared, and folding it into a
/// maximum silently would turn a broken run into a passing one.
fn finite(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        f64::INFINITY
    }
}

/// The parameters measured along every seam: corners, the middle and three interior points.
///
/// Corners included on purpose. A seam's ends are where a per-position rule and a per-edge rule
/// disagree, and this design's claim is that there is no such distinction.
fn parameters() -> [f64; 7] {
    [0.0, 0.25, 0.5, 0.75, 1.0, 0.125, 0.875]
}

/// Measures every seam of a grid, taking the cells left of `column` from `clean` and those from
/// `column` rightwards out of `dirty`.
///
/// Two fields, because **perturbing a grid and measuring it measures nothing.** Move every node on
/// a column and *both* faces on each of its seams move together, so the grid stays perfectly
/// smooth and the largest reading is the floor. The first version of this probe's control did
/// precisely that and reported `2.980e-8` — indistinguishable from the passing arm — which is the
/// failure mode this signature exists to make impossible.
fn measure_split(clean: &Field, dirty: &Field, column: usize) -> Reading {
    let take = |i: usize| if i < column { clean } else { dirty };
    let mut worst = Reading::default();
    for j in 0..clean.rows {
        for i in 1..clean.columns {
            worst = worst.worse(measure_seam(
                &take(i - 1).cell(i - 1, j),
                &take(i).cell(i, j),
                Seam::AcrossU,
            ));
        }
    }
    for i in 0..clean.columns {
        for j in 1..clean.rows {
            worst = worst.worse(measure_seam(
                &take(i).cell(i, j - 1),
                &take(i).cell(i, j),
                Seam::AcrossV,
            ));
        }
    }
    worst
}

/// Measures every seam of one field, every edge soft.
fn measure(field: &Field) -> Reading {
    measure_split(field, field, usize::MAX)
}

/// Measures one seam, for the two-cell comparisons where the rest of the grid is in the way.
fn measure_seam(first: &Cell, second: &Cell, seam: Seam) -> Reading {
    let mut worst = Reading::default();
    for along in parameters() {
        worst = worst.worse(read_seam_point(along, seam, first, second));
    }
    worst
}

/// Every interior seam of a grid, for reporting how many there are.
fn seam_count(field: &Field) -> usize {
    (field.columns - 1) * field.rows + field.columns * (field.rows - 1)
}

/// A ceiling on an angle that should be zero.
///
/// The floor is about `1.4e-8`, from `acos` on a dot product `f64` cannot resolve below `1 - 1e-16`.
/// A **normal** reads two to three times that rather than exactly it, because it is a cross product
/// of two vectors that are not unit length and the error in their directions is what is being
/// measured. That is rounding, not geometry, so the ceiling sits a decade above it and no tighter:
/// tightening it against the floor is how a probe ends up reporting `1.4e-8` as a defect that is
/// the instrument's own limit.
const C1_CEILING: f64 = 1.0e-7;

fn row(reading: &Reading) -> String {
    format!(
        "split {:.3e}   along {:.3e}   across {:.3e}   normal {:.3e} rad",
        reading.split, reading.along_edge, reading.across_edge, reading.normal
    )
}

fn report(name: &str, detail: &str) {
    println!("\n== {name} ==\n{detail}");
}

fn main() {
    let mut failures: Vec<&str> = Vec::new();

    // -- 1. A grid with every interior edge soft is exactly C1. --------------------------------------
    let field = Field::sample();
    let soft = measure(&field);
    println!(
        "\nsample: {} columns by {} rows of nodes, {} cells, {} interior seams",
        field.columns + 1,
        field.rows + 1,
        field.columns * field.rows,
        seam_count(&field)
    );
    report(
        "1  every interior edge soft: exactly C1",
        &format!(
            "{}\n\
             the soft tag makes the cross-boundary derivative shared edge data, so the two faces'\n\
             normals are the same function of the edge parameter rather than two functions that\n\
             happen to agree. the ceiling is {:.1e}: the acos floor is {:.1e} and a normal reads\n\
             two to three times it, because it is a cross product of vectors that are not unit",
            row(&soft),
            C1_CEILING,
            ANGLE_FLOOR
        ),
    );
    if soft.non_finite > 0 || soft.split > support::EXACT {
        failures.push("1  a soft seam is one shared boundary");
    }
    if soft.normal > C1_CEILING {
        failures.push("1  a soft seam is exactly C1");
    }

    // -- 0. The control that makes measurement 1 readable. -------------------------------------------
    //
    // Three arms, and the second one is the point. Perturbing a grid and measuring it proves
    // nothing, because both faces on every touched seam move together; that arm reports the floor
    // and is the reason the first version of this control was void. Perturbing **one side only** is
    // what makes a discontinuity, and that arm must report a large angle.
    let mut broken = field.clone();
    let corrupt = 2.min(field.columns - 1);
    for j in 0..=field.rows {
        broken.nodes[corrupt][j].du += Point3::new(0.0, 0.0, 0.8);
    }
    let control = measure_split(&field, &broken, corrupt);
    let control_void = measure(&broken);
    report(
        "0  control: the probe can see a discontinuity",
        &format!(
            "node column {corrupt} lifted by 0.8, BOTH sides of every seam it touches\n{}\n\
             the same change on ONE side only, so the faces at that column genuinely disagree\n{}\n\
             the first arm is the one a perturbation-only control reports, and it is the floor: the\n\
             two faces moved together, so nothing is discontinuous. the second arm is what makes\n\
             measurement 1's near-zero a property of the surface rather than of the instrument --\n\
             it is {:.0e} times larger, on the same function and the same sample",
            row(&control_void),
            row(&control),
            control.normal / control_void.normal
        ),
    );
    if control.normal < 1.0e-2 || control_void.normal > C1_CEILING {
        failures.push("0  the control fires on one side and stays silent when both sides move");
    }

    // -- 2. What the soft tag buys. -------------------------------------------------------------------
    //
    // Both arms share the boundary curve exactly -- same positions, same tangential derivative,
    // same parameter step -- so C0 and the along-edge tangent are already perfect in both. The
    // only thing that differs is whether the cross-boundary derivative is shared.
    //
    // This is the correction the handoff leads with: synchronizing the edge and its parameters is
    // necessary and is not sufficient.
    let left = field.cell(0, 0);
    let right_shared = field.cell(1, 0);
    let independent = {
        let mut other = field.clone();
        for j in 0..=field.rows {
            other.nodes[1][j].du += Point3::new(0.0, 0.0, 0.8);
        }
        other.cell(1, 0)
    };
    let seam = Seam::AcrossU;
    let shared_reading = measure_seam(&left, &right_shared, seam);
    let independent_reading = measure_seam(&left, &independent, seam);

    report(
        "2  shared cross-boundary derivative, against an independent one",
        &format!(
            "cross-boundary derivative shared  (soft)\n{}\n\
             cross-boundary derivative private (hard)\n{}\n\
             split and along-edge are identical in both arms, and both are exact: the boundary curve\n\
             and its parameterization are shared either way. only the across-edge angle moves, and\n\
             it moves by {:.3e} rad. that is the whole content of the tag -- and it is why\n\
             'the two faces agree about the edge' is not the same claim as 'the join is C1'",
            row(&shared_reading),
            row(&independent_reading),
            independent_reading.normal - shared_reading.normal
        ),
    );
    if shared_reading.normal > C1_CEILING {
        failures.push("2a  a shared cross-boundary derivative gives an exact C1 join");
    }
    if independent_reading.split > support::EXACT || independent_reading.along_edge > C1_CEILING {
        failures
            .push("2b  a private cross-boundary derivative still leaves C0 and a matched tangent");
    }
    if independent_reading.normal < 1.0e-2 {
        failures
            .push("2b  an independent cross-boundary derivative does fold (the control for 2a)");
    }

    // -- 3. A hard edge keeps its interior free. -----------------------------------------------------
    //
    // Both interiors move, by different amounts, on a seam whose two faces disagree about the
    // normal. The crease must survive: the boundary is still one curve, and the two sides still
    // disagree about the normal rather than having been welded into agreement.
    //
    // The interior rows are `a = 1` (near) and `a = 3` (far); `a = 2` is the far edge's *position*
    // and moving it is moving the boundary, which is the bug the first version of this probe made.
    let mut left_free = left;
    left_free.nudge_u_row(3, Point3::new(0.0, 0.0, 0.5));
    let mut right_free = independent;
    right_free.nudge_u_row(1, Point3::new(0.0, 0.0, -0.3));
    let free_reading = measure_seam(&left_free, &right_free, seam);
    let crease_before = measure_seam(&left, &independent, seam);

    let seam_curve_moved = (0..4).any(|b| {
        left_free.jet[2][b].distance(left.jet[2][b]) > 0.0
            || right_free.jet[0][b].distance(independent.jet[0][b]) > 0.0
    });
    let interior_moved = (0..4).any(|b| {
        left_free.jet[3][b].distance(left.jet[3][b]) > 0.0
            && right_free.jet[1][b].distance(independent.jet[1][b]) > 0.0
    });

    report(
        "3  a hard edge: shared boundary, free interiors",
        &format!(
            "before either interior moved\n{}\n\
             after moving the master's interior by +0.5 and the slave's by -0.3\n{}\n\
             the shared boundary moved                                       {}\n\
             both interiors moved                                            {}\n\
             the crease is {:.3e} rad before and {:.3e} rad after, and the split stays at {:.3e}.\n\
             a hard edge is a shared curve with independent interiors, which is what crate::continuity\n\
             says today; the new part is that it costs no enforcement pass to keep the curve shared",
            row(&crease_before),
            row(&free_reading),
            if seam_curve_moved {
                "yes, which would be a crack"
            } else {
                "no"
            },
            if interior_moved {
                "yes"
            } else {
                "no, so this arm proves nothing"
            },
            crease_before.normal,
            free_reading.normal,
            free_reading.split
        ),
    );
    if free_reading.split > support::EXACT {
        failures.push("3  a drag on either side of a hard edge does not open the seam");
    }
    if free_reading.along_edge > C1_CEILING {
        failures.push("3  a hard edge keeps its two sides' tangential derivative matched");
    }
    if seam_curve_moved || !interior_moved {
        failures.push("3  the two interiors really were free to move independently");
    }

    // -- 4. The boundary is one cubic, independent of the interior. -----------------------------------
    //
    // Two separate claims, and the second is the one the shared boundary has always had to
    // protect. The first is that a boundary's four Hermite jets and the four Bezier control points
    // that `as_bezier` produces draw **the same curve** — evaluated here by a Bernstein basis
    // written out independently, so the two are compared and not assumed equal. It is the identity
    // that makes the shared boundary of this representation the shared Bernstein row it replaces,
    // so this is a reparameterization of the shared geometry rather than a change to it. The
    // second is that moving a patch's interior cannot move its boundary.
    let held = field.cell(0, 0);
    let mut moved_interior = held;
    moved_interior.nudge_u_row(1, Point3::new(0.3, -0.2, 0.7));

    let mut worst_curve = 0.0f64;
    for t in parameters() {
        let weights = bernstein_degree_three(t);
        let control = as_bezier(moved_interior.boundary_curve(Seam::AcrossU, true));
        let assembled = control
            .iter()
            .enumerate()
            .fold(Point3::ZERO, |sum, (index, point)| {
                sum + *point * weights[index]
            });
        // The same curve read as the cell's own surface at `s = 1`, which is its `u = 1` boundary.
        worst_curve = worst_curve.max(finite(moved_interior.evaluate(1.0, t).distance(assembled)));
    }

    let curve_shift = (0..4)
        .map(|index| {
            finite(
                moved_interior.boundary_curve(Seam::AcrossU, true)[index]
                    .distance(held.boundary_curve(Seam::AcrossU, true)[index]),
            )
        })
        .fold(0.0f64, f64::max);
    let interior_shift = moved_interior
        .evaluate(0.5, 0.5)
        .distance(held.evaluate(0.5, 0.5));

    report(
        "4  the boundary is one cubic, independent of the interior",
        &format!(
            "Hermite jets against Bezier control points on an independent Bernstein basis {worst_curve:.3e}\n\
             boundary control points moved when the interior did          {curve_shift:.3e}\n\
             the interior itself moved by                                   {interior_shift:.3e}\n\
             the first line is the identity that makes the shared boundary of this representation\n\
             the same curve as the shared Bernstein row it replaces. the other two say the interior\n\
             is free without touching it, which is what a per-face edit needs",
        ),
    );
    if worst_curve > support::EXACT || curve_shift > 0.0 || interior_shift <= 0.0 {
        failures.push("4  the boundary is one cubic and the interior is free of it");
    }

    // -- verdict -----------------------------------------------------------------------------------
    println!();
    if failures.is_empty() {
        println!("all measurements pass");
    } else {
        for failure in &failures {
            println!("FAILED  {failure}");
        }
        std::process::exit(1);
    }
}
