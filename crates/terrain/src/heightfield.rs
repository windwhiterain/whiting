//! A height field over a rectangular lattice of the plan, and the mesh it is read off.
//!
//! # Invariants
//!
//! - **The lattice is square.** One plan spacing serves both axes, so the eight neighbour
//!   offsets of [`erode::grid`](crate::erode::grid) are one texel or `√2` of them and never a
//!   third number, and a slope read across the field is the same slope whichever axis it is
//!   read along. A lattice with two spacings would need the distance normalisation written
//!   against both of them, and every reader of the field would have to know which it had.
//! - **The plan coordinates are `(x, z)` and the elevation is `y`**, read from `mountain` and
//!   not re-decided here; see [`mountain::math`]. A field whose axes were named differently
//!   from the mesh's would be a field whose cells have to be matched to faces by hand.
//! - **A cell either has rock under it or does not**, and [`HeightField::is_inside`] says
//!   which. There is no third state and no sentinel height: the massif is a disc cut out of
//!   its own footprint, and a rectangle of lattice laid over a disc has cells that fall
//!   outside it. An outside cell is **frozen** — the solver never reads it and never writes
//!   it — and its height is the field's [`HeightField::base`], because a caller drawing the
//!   lattice needs *some* ground there and `base` is the one number that means something.
//!
//! # Why the conversion is exact rather than sampled
//!
//! [`mountain::facet::FacetMesh`] guarantees every face is exactly planar, so the height over
//! one face is the barycentric interpolation of its three corners and there is nothing left
//! to approximate. The conversion therefore has **no error term at all**: it is a change of
//! representation, from a triangle mesh over an arbitrary footprint to a lattice over a
//! rectangle, and the only thing it loses is the block faces' own straightness — which is
//! what [`.agents/SPEC/mountain.md`](../../../.agents/SPEC/mountain.md) R6 hands to a
//! downstream pipeline in the first place.

use mountain::math::Vec2;

/// A height field: elevation sampled on a rectangular lattice of the plan.
///
/// # Invariants
///
/// - **`heights` and `inside` are one entry per cell and are the same length**, `width *
///   height` of them, and a cell is at `row * width + column`. A field whose two buffers
///   disagreed in length would have a mask that runs out before the heights do, and the
///   failure would be a cell silently treated as rock rather than an error.
/// - **`cell` is positive and the same on both axes.** A non-positive spacing has no lattice
///   to index, and an anisotropic one breaks the square-neighbour relation every slope and
///   every receiver in [`crate::erode`] is written against.
/// - **`base` is the height of every cell with no rock under it**, and the solver treats
///   those cells as frozen rather than as ground at that height. It is a reading for a
///   caller drawing the lattice and not an input to the solve, which is why changing it
///   cannot move a single eroded height.
#[derive(Debug, Clone, PartialEq)]
pub struct HeightField {
    origin: Vec2,
    cell: f32,
    width: usize,
    height: usize,
    base: f32,
    heights: Vec<f32>,
    inside: Vec<bool>,
}

impl HeightField {
    /// Builds a field from its own buffers.
    ///
    /// # Invariants
    ///
    /// The two buffers are **checked rather than trusted**, because a caller that assembled
    /// them has no other way to learn that they disagree, and the symptom otherwise is a
    /// cell treated as rock because the mask ran out first. `cell` is checked for the same
    /// reason and against the same consequence: a non-positive spacing makes every neighbour
    /// lookup land on the cell itself, so every cell becomes a pit and the solve erodes
    /// nothing anywhere.
    pub fn of(
        origin: Vec2,
        cell: f32,
        width: usize,
        height: usize,
        base: f32,
        heights: Vec<f32>,
        inside: Vec<bool>,
    ) -> Result<Self, FieldError> {
        if cell <= 0.0 || cell.is_nan() {
            return Err(FieldError::NotASpacing { cell });
        }
        let cells = width
            .checked_mul(height)
            .ok_or(FieldError::CellCountOverflow)?;
        if heights.len() != cells || inside.len() != cells {
            return Err(FieldError::Buffers {
                cells,
                heights: heights.len(),
                inside: inside.len(),
            });
        }
        Ok(HeightField {
            origin,
            cell,
            width,
            height,
            base,
            heights,
            inside,
        })
    }

    /// Returns the plan position of the centre of one cell, without a lattice check.
    ///
    /// A column or row outside the lattice is a fact about a caller rather than about the
    /// field, and it is answered with a position past the edge rather than a panic so that a
    /// loop reading neighbours does not have to prove its own bounds first.
    pub fn plan(&self, column: usize, row: usize) -> Vec2 {
        [
            self.origin[0] + column as f32 * self.cell,
            self.origin[1] + row as f32 * self.cell,
        ]
    }

    /// Returns the index of one cell, or nothing when it is off the lattice.
    pub fn index(&self, column: usize, row: usize) -> Option<usize> {
        (column < self.width && row < self.height).then_some(row * self.width + column)
    }

    /// Returns the number of columns.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Returns the number of rows.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Returns how many cells the lattice has.
    pub fn cells(&self) -> usize {
        self.heights.len()
    }

    /// Returns the plan spacing of one cell, on both axes.
    pub fn cell(&self) -> f32 {
        self.cell
    }

    /// Returns the plan position of the centre of cell `(0, 0)`.
    pub fn origin(&self) -> Vec2 {
        self.origin
    }

    /// Returns the height carried by a cell with no rock under it.
    pub fn base(&self) -> f32 {
        self.base
    }

    /// Returns the lowest and highest elevation over the cells that have rock under them.
    ///
    /// # Invariants
    ///
    /// **A cell with no rock under it is not counted.** Its height is [`HeightField::base`],
    /// which the caller chose rather than the mesh, so letting it into the range would let a
    /// plain under a massif decide how tall the massif is. A field with no rock at all
    /// answers `(base, base)` rather than a pair of infinities, so a caller that reached
    /// this with nothing to measure gets a number it can print rather than a NaN it has to
    /// recognise.
    pub fn relief(&self) -> (f32, f32) {
        let mut low = f32::INFINITY;
        let mut high = f32::NEG_INFINITY;
        for (at, inside) in self.inside.iter().enumerate() {
            if *inside {
                let height = self.heights[at];
                low = low.min(height);
                high = high.max(height);
            }
        }
        if low > high {
            (self.base, self.base)
        } else {
            (low, high)
        }
    }

    /// Returns how many cells have rock under them.
    pub fn covered(&self) -> usize {
        self.inside.iter().filter(|inside| **inside).count()
    }

    /// Returns the field's heights, one per cell, in lattice order.
    pub fn heights(&self) -> &[f32] {
        &self.heights
    }

    /// Returns which cells have rock under them, one per cell, in lattice order.
    pub fn inside(&self) -> &[bool] {
        &self.inside
    }

    /// Returns the elevation of one cell, whatever is under it.
    ///
    /// It is a raw read of the buffer, so it answers [`HeightField::base`] for a cell with no
    /// rock under it. [`HeightField::sample`] is the read a caller positioning itself in the
    /// plan wants, and this is the one indexing by cell that wants.
    pub fn height_at(&self, at: usize) -> f32 {
        self.heights.get(at).copied().unwrap_or(self.base)
    }

    /// Returns whether one cell has rock under it, and is frozen when it has not.
    ///
    /// An index off the end of the lattice is reported as no rock, for the same reason
    /// [`HeightField::height_at`] answers `base` rather than panicking: the caller holds a
    /// fact about a field this was not read from, and a number is a better report of that
    /// than a backtrace.
    pub fn is_inside(&self, at: usize) -> bool {
        self.inside.get(at).copied().unwrap_or(false)
    }

    /// Returns the elevation at a plan position, interpolated between the four cells around
    /// it.
    ///
    /// A position with no rock under it within one cell of any is answered with
    /// [`HeightField::base`], which is the only height this type has for that case.
    pub fn sample(&self, plan: Vec2) -> f32 {
        self.sample_run(self.heights(), plan).unwrap_or(self.base)
    }

    /// Interpolates any per-cell buffer at a plan position, over the same four cells
    /// [`HeightField::sample`] reads.
    ///
    /// # Invariants
    ///
    /// - **A cell with no rock under it takes no part**, whatever the buffer holds. A point
    ///   near the rim is interpolated between the cells that have rock and over the weights
    ///   they carry among themselves, so a run read near the boundary approaches the
    ///   boundary's own values instead of being pulled toward whatever sits outside it.
    /// - **A point with no rock within one cell of any has no answer**, and it is `None`
    ///   rather than a number. The buffers read here are not all heights — a displacement
    ///   field's neutral is zero and a distance-to-rim's is a small number — so a single
    ///   fallback would be a claim about one of them that the other two silently contradict.
    ///   Each caller states its own, which is what makes this a function rather than a
    ///   convention.
    /// - **A point on a cell's centre reads that cell**, because the four weights are one and
    ///   three zeros. A caller drawing the lattice at the lattice's own spacing therefore
    ///   reproduces the buffer it was given rather than a smoothed version of it, and the
    ///   two are the same surface only where they are read at the cells.
    /// - **`values` is one entry per cell.** A shorter buffer is read as zero past its end,
    ///   which is the neutral of every buffer here that is not a height.
    pub fn sample_run(&self, values: &[f32], plan: Vec2) -> Option<f32> {
        let column = (plan[0] - self.origin[0]) / self.cell;
        let row = (plan[1] - self.origin[1]) / self.cell;
        // **The far cell is one past the near one and not `ceil`**, because `floor` and `ceil` are the same
        // number on an exact lattice line and both shares then come out zero: the point's own cell is dropped
        // for having no weight and the far one for having no share, the loop's total stays `0.0`, and the answer
        // is the field's `base` instead of its height. Measured on a field filled from a closed-form hill: four
        // plan points with integer coordinates -- the square's own centre among them -- each answered `0.0`
        // while the buffer held `5.6`, and the scene then cut its terrace level from that zero.
        let (left, right) = (column.floor(), column.floor() + 1.0);
        let (near, far) = (row.floor(), row.floor() + 1.0);
        let mut weight = 0.0;
        let mut total = 0.0;
        for (column, share_column) in [(left, 1.0 - (column - left)), (right, column - left)] {
            // A lattice index is a `usize` and a float cast into one saturates at zero, so a
            // point left of the first column would otherwise borrow cell zero's height and
            // the field would extend itself out to the plane's negative quarter.
            if column < 0.0 {
                continue;
            }
            for (row, share_row) in [(near, 1.0 - (row - near)), (far, row - near)] {
                if row < 0.0 {
                    continue;
                }
                let share = share_column * share_row;
                if share <= 0.0 {
                    continue;
                }
                let Some(at) = self.index(column as usize, row as usize) else {
                    continue;
                };
                if !self.inside[at] {
                    continue;
                }
                weight += share * values.get(at).copied().unwrap_or(0.0);
                total += share;
            }
        }
        (total > 0.0).then_some(weight / total)
    }

    /// Returns how many cells from the massif's rim each covered cell stands, in cells.
    ///
    /// # Invariants
    ///
    /// - **It is a chamfer distance, and the approximation is in the step costs**: one cell
    ///   across an edge neighbour and `√2` across a diagonal one, swept once forward and
    ///   once back. A chamfer under-reads a long diagonal by a few percent, which is why the
    ///   unit is **cells** rather than plan distance — a fade measured in three cells is
    ///   exact, and a distance measured in fifty is not used for anything here.
    /// - **A cell with no rock under it answers zero**, which is the point of the read: a
    ///   caller fading something out towards the boundary reads zero at the boundary and
    ///   something else everywhere inside it, without testing the mask separately.
    /// - **It is two passes over the lattice**, so it costs about one round of the solver
    ///   rather than a search per cell.
    pub fn edge_distance(&self) -> Vec<f32> {
        let mut distance = vec![0.0; self.cells()];
        for (at, cell) in distance.iter_mut().zip(&self.inside) {
            if *cell {
                *at = f32::INFINITY;
            }
        }
        for forward in [true, false] {
            for row in 0..self.height {
                let columns: Box<dyn Iterator<Item = usize>> = if forward {
                    Box::new(0..self.width)
                } else {
                    Box::new((0..self.width).rev())
                };
                for column in columns {
                    let Some(at) = self.index(column, row) else {
                        continue;
                    };
                    if !self.inside[at] {
                        continue;
                    }
                    // The four neighbours this sweep has already seen: up and to the left on
                    // the way down, down and to the right on the way back.
                    let seen: [(i32, i32); 4] = if forward {
                        [(-1, -1), (0, -1), (1, -1), (-1, 0)]
                    } else {
                        [(1, 1), (0, 1), (-1, 1), (1, 0)]
                    };
                    for (dx, dy) in seen {
                        let next = (column as i32 + dx, row as i32 + dy);
                        if next.0 < 0 || next.1 < 0 {
                            continue;
                        }
                        let Some(next) = self.index(next.0 as usize, next.1 as usize) else {
                            continue;
                        };
                        let step = if dx != 0 && dy != 0 {
                            std::f32::consts::SQRT_2
                        } else {
                            1.0
                        };
                        distance[at] = distance[at].min(distance[next] + step);
                    }
                }
            }
        }
        distance
    }

    /// Returns the same field with its heights replaced, keeping the lattice.
    pub(crate) fn with_heights(&self, heights: Vec<f32>) -> Result<Self, FieldError> {
        HeightField::of(
            self.origin,
            self.cell,
            self.width,
            self.height,
            self.base,
            heights,
            self.inside.clone(),
        )
    }

    /// Returns this field blended towards another, per cell, by a weight run.
    ///
    /// # Invariants
    ///
    /// - **A cell with no rock under it keeps this field's own height.** The weight of a cell
    ///   outside the massif is a number the caller chose, and the drawing it does with a
    ///   surface that is not there is its own business.
    /// - **`weights` is one entry per cell**, and a shorter run damps every cell that has one
    ///   against nothing, which is the same refusal to invent a weight as the lattice's own
    ///   index checks are.
    pub fn with_surface(&self, other: &HeightField, weights: &[f32]) -> Result<Self, FieldError> {
        let heights = self
            .heights
            .iter()
            .enumerate()
            .map(|(at, here)| {
                if !self.inside[at] {
                    return *here;
                }
                let weight = weights.get(at).copied().unwrap_or(0.0);
                *here + weight * (other.heights[at] - *here)
            })
            .collect();
        self.with_heights(heights)
    }

    /// Returns the field with every interior pit raised to the level its water would spill at.
    ///
    /// # Invariants
    ///
    /// - **Only pits rise, and never above their own spill level.** A cell's filled height is
    ///   the lowest level at which it connects to the rock's own boundary through rock — the
    ///   priority flood: the boundary stands at its own height and the level spreads inward,
    ///   climbing only where the rock lies below the level that reached it. Ground that already
    ///   drains is untouched, because its own height is the level that reaches it.
    /// - **It is one heap sweep over the covered cells**, `O(n log n)`, the order of one sorted
    ///   round of the solver rather than a search per pit.
    /// - **A cell with no rock under it keeps [`HeightField::base`]**, as everywhere else, and a
    ///   cell the flood never reaches — an inside component walled off from the boundary, which
    ///   the mask cannot express but an arithmetic fault could — keeps its own height rather
    ///   than being lifted to infinity.
    ///
    /// The solver leaves a handful of one-cell pits even with multiple-flow routing, and a pit
    /// drawn is a dot: this is the drawing's answer to them, applied by the caller that draws
    /// rather than inside the solve, so the transport books still describe the solve that ran.
    pub fn fill_pits(&self) -> HeightField {
        use std::collections::BinaryHeap;

        /// A heap entry: the level a cell was reached at, lowest first.
        #[derive(Clone, Copy)]
        struct Level(f32, usize);
        impl PartialEq for Level {
            fn eq(&self, other: &Self) -> bool {
                self.cmp(other) == std::cmp::Ordering::Equal
            }
        }
        impl Eq for Level {}
        impl Ord for Level {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                // Reversed on the level: the binary heap is a max-heap and the flood wants the
                // lowest level first. A total order including the index, because a NaN level
                // would otherwise leave the heap without a greatest element.
                other
                    .0
                    .total_cmp(&self.0)
                    .then_with(|| self.1.cmp(&other.1))
            }
        }
        impl PartialOrd for Level {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        /// The eight neighbours, edges before diagonals, in one table.
        const NEIGHBOURS: [(i32, i32); 8] = [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ];

        let cells = self.cells();
        let mut level = vec![f32::INFINITY; cells];
        let mut heap = BinaryHeap::new();
        for row in 0..self.height {
            for column in 0..self.width {
                let Some(at) = self.index(column, row) else {
                    continue;
                };
                if !self.inside[at] {
                    continue;
                }
                // The flood enters the rock where the rock meets its own boundary: a cell with
                // a neighbour off the lattice or with no rock under it.
                let open = NEIGHBOURS.iter().any(|(dx, dy)| {
                    let next = (column as i32 + dx, row as i32 + dy);
                    next.0 < 0
                        || next.1 < 0
                        || self
                            .index(next.0 as usize, next.1 as usize)
                            .is_none_or(|next| !self.inside[next])
                });
                if open {
                    level[at] = self.heights[at];
                    heap.push(Level(self.heights[at], at));
                }
            }
        }
        while let Some(Level(reached, at)) = heap.pop() {
            if reached > level[at] {
                // This cell was reached again, lower, after this stale entry was pushed.
                continue;
            }
            let column = at % self.width;
            let row = at / self.width;
            for (dx, dy) in NEIGHBOURS {
                let next_column = column as i32 + dx;
                let next_row = row as i32 + dy;
                if next_column < 0 || next_row < 0 {
                    continue;
                }
                let Some(next) = self.index(next_column as usize, next_row as usize) else {
                    continue;
                };
                if !self.inside[next] {
                    continue;
                }
                let spill = reached.max(self.heights[next]);
                if spill < level[next] {
                    level[next] = spill;
                    heap.push(Level(spill, next));
                }
            }
        }
        let heights = self
            .heights
            .iter()
            .enumerate()
            .map(|(at, here)| {
                if self.inside[at] && level[at] < f32::INFINITY {
                    level[at].max(*here)
                } else {
                    *here
                }
            })
            .collect();
        self.with_heights(heights)
            .expect("the lattice and mask are unchanged")
    }
}

/// Why a height field could not be built.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FieldError {
    /// The plan spacing asked for is not a length.
    NotASpacing {
        /// The spacing that was asked for.
        cell: f32,
    },
    /// The lattice has more cells than a `usize` can count.
    CellCountOverflow,
    /// The two buffers are not one entry per cell of the lattice.
    Buffers {
        /// How many cells the lattice has.
        cells: usize,
        /// How many heights were handed over.
        heights: usize,
        /// How many mask entries were handed over.
        inside: usize,
    },
}

impl std::fmt::Display for FieldError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldError::NotASpacing { cell } => {
                write!(
                    formatter,
                    "the plan spacing {cell} is not a positive length"
                )
            }
            FieldError::CellCountOverflow => {
                write!(
                    formatter,
                    "the lattice has more cells than a `usize` can count"
                )
            }
            FieldError::Buffers {
                cells,
                heights,
                inside,
            } => write!(
                formatter,
                "a {cells} cell lattice was given {heights} heights and {inside} mask entries"
            ),
        }
    }
}

impl std::error::Error for FieldError {}
