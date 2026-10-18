//! S2: the grid skeleton — the planned rhythm of a facade.
//!
//! A mass says what volume a building has; the grid says how that volume **reads**: where its storey
//! lines are, and how its wall is divided into bays. Both are authored, both are data in the
//! document, and neither is a rule that fires later — see D10 in
//! `.agents/proposals/agentic-building-generation.md`, which is why a rhythm here is a construction
//! move that writes stops into the document and never a live generator above it.
//!
//! # The one idea, here
//!
//! **A bay is a slot, not a place.** A grid is two lists of stops, so a cell is the space between two
//! consecutive stops of each, and it is named by its index in those lists — `(row, column)` — rather
//! than by its coordinates. That is what makes a re-split survivable: replacing a face's family
//! replaces the stops, and everything hanging in the grid by slot comes along, where a placement
//! keyed by position would be orphaned by the first edit. This is D10's rule, and this module is
//! where it is cashed.

use std::collections::BTreeMap;

use crate::mass::{AuthoredLoop, Mass, MassId, PlanNode};

/// How close two stops have to be, or a span has to match, before they are the same number.
///
/// # Invariants
///
/// A millimetre at building scale — the same kind of number, for the same reason, as the validator's
/// coverage tolerance: a rhythm laid out at metre scale carries `f32` rounding around `1e-5` m, an
/// authored stop is written to a millimetre, and the smallest real feature a facade has is a
/// centimetre or more. So this is four decades above the rounding and one below the smallest thing
/// anyone authors.
pub const GRID_TOLERANCE: f32 = 1.0e-3;

/// The most stops one rhythm may lay out on one face.
///
/// # Invariants
///
/// **A rhythm's stop count is a function of two authored numbers, not of a loop**, so without a bound
/// a caller can ask for an allocation by arithmetic: `Rhythmic { module: 1.0e-6 }` on an eleven-metre
/// face asks for eleven million stops, and `Even { count: 100_000 }` asks for a hundred thousand.
/// A facade has tens of bays and a hundred would be an unusual building, so this sits two orders above
/// anything an author writes and refuses beyond it rather than clamping — the rule everywhere else in
/// this crate. `symbios-shape`, the MIT CGA engine the design survey verified, bounds its own rhythm
/// expansion for the same reason ("max flattened split children (rhythm) 4 096").
pub const MAX_STOPS: usize = 4096;

/// Which of a wall ring's two loops a face is read off.
///
/// # Invariants
///
/// **A wall is two loops of the same shape, and this names which one a face is on.** A mass's wall
/// stands *inside* its outline and a courtyard's ring *outside* the void's loop, so the ring's two
/// faces are the loop the author drew and the loop a wall's thickness away from it. [`FaceRef`] holds
/// one value of this rather than deriving it from whether the mass has a courtyard, because the two
/// loops are two **different pieces of masonry** and an arc along one is not an arc along the other:
/// a mitre moves the offset edge's ends, so one ring's two faces do not even measure the same.
///
/// It is the word [`crate::expand`]'s ring walk already winds by, which is what keeps the expander and
/// the document from holding two answers to "which loop is this face on".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LoopSide {
    /// The loop the author drew: a mass's own outline, or a courtyard's void loop. Every face this
    /// crate named before the offset loop became addressable is on this one.
    Authored,
    /// The loop a wall's thickness away from the authored one: a mass's inner wall, or the wall a
    /// courtyard's ring shows **to the rooms**.
    ///
    /// Each of the two is one constructor — [`FaceRef::inner`] and [`FaceRef::court_rooms`] — so the
    /// variant itself is never spelled out by a caller.
    Offset,
}

impl LoopSide {
    /// Returns the number this side sorts after by: `0` for the loop the author drew and `1` for the
    /// loop a wall's thickness away.
    ///
    /// A rank rather than a cast, so a report that orders two faces of one mass reads *the loop the
    /// author drew first* whatever order the enum's own variants are declared in.
    pub fn rank(self) -> u64 {
        match self {
            Self::Authored => 0,
            Self::Offset => 1,
        }
    }
}

/// Which face of a mass a grid belongs to: **one edge of one authored loop**.
///
/// # Invariants
///
/// The face's extent is the edge's own plan length horizontally and `[the lower of its two node
/// elevations, the mass's wall top]` vertically. A face is therefore a trapezoid when the ground
/// slopes under it, and the vertical extent is the **lowest** ground the wall stands on — the wall's
/// own top is level, so the shortest part of the face is the one that decides what fits.
///
/// **A mass holds two kinds of wall and a face names which one it is on.** `void` is `None` for the
/// mass's own outline and `Some` for one of its courtyards, so a rhythm — and everything a rhythm
/// carries, an opening and a placement — can be authored on a courtyard's wall exactly as it is on
/// the building's. This is [`AuthoredLoop`] in the form the document's own edits name things by:
/// an index rather than a type of thing, because a courtyard wall and a mass wall are the same kind
/// of face and differ only in which loop they were read off.
///
/// The key order puts `None` before `Some` and the authored loop before the offset one, which is what
/// makes every walk over a document's openings report a mass's own walls, then each courtyard's own
/// face, then each courtyard's face **as the rooms see it**, without comparing anything. Every face
/// this crate named before [`LoopSide`] existed is `Authored`, and it sorts where it always did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FaceRef {
    /// The mass the face belongs to.
    pub mass: MassId,
    /// Index of the loop edge.
    pub edge: usize,
    /// Which of the mass's voids this face is on, or `None` for the mass's own outline.
    pub void: Option<usize>,
    /// Which of that ring's two loops the edge is on.
    pub loop_side: LoopSide,
}

impl FaceRef {
    /// Builds a reference to one edge of a mass's own wall.
    pub fn new(mass: MassId, edge: usize) -> Self {
        Self {
            mass,
            edge,
            void: None,
            loop_side: LoopSide::Authored,
        }
    }

    /// Builds a reference to one edge of a courtyard's wall.
    ///
    /// The void is the loop the courtyard's ring is built **outward** from, so this edge is the face
    /// that looks *into* the courtyard; the ring's other face — the one the rooms see — is
    /// [`court_rooms`](Self::court_rooms).
    ///
    /// **A part pinned in one of these bays stands on the courtyard's side of that wall**, flush with
    /// the face this names and with its own depth running out of it into the courtyard — which is what
    /// `along × up = inward` requires here, and what `expand::part_frame` reads. See
    /// [`PartFrame`](crate::parts::PartFrame) for the frame a placement is stood in and why it is a
    /// rotation on this face as much as on a mass's.
    ///
    /// **A hole is still authored here and not on the far face**, because a hole is cut through both
    /// of the ring's faces from the loop the walk reads: naming the far one would be a second
    /// document for one hole, and the walk would never see it. That is
    /// [`Violation::OpeningOnAFarLoop`](crate::validate::Violation::OpeningOnAFarLoop).
    pub fn court(mass: MassId, void: usize, edge: usize) -> Self {
        Self {
            mass,
            edge,
            void: Some(void),
            loop_side: LoopSide::Authored,
        }
    }

    /// Builds a reference to one edge of a courtyard's wall **as the rooms see it**.
    ///
    /// This is the ring's **offset** loop, a wall's thickness outside the void's own: the face on the
    /// far side of the same masonry from [`court`](Self::court), and the one the expander names
    /// [`MassSurface::VoidInner`](crate::mesh::MassSurface::VoidInner). It is a face like any other
    /// and takes a rhythm, a bay and a part exactly as the near one does.
    ///
    /// **It measures differently, and that is the whole of why it needed its own name.** A mitre runs
    /// the offset edge's ends past the authored edge's at both corners — outward for a courtyard, so
    /// the rooms-facing wall is the *longer* of the two — so an arc length measured here is not an arc
    /// length there, and the two ends of this edge are two different pieces of masonry from the two
    /// ends of the near one. [`BuildingSkeleton::face_length`](crate::skeleton::BuildingSkeleton::face_length)
    /// reads the offset loop's own length for a face that names it, which is what makes a rhythm laid
    /// here span it exactly.
    ///
    /// **A part pinned in one of these bays stands in the rooms with its depth running into the ring**,
    /// which is the same rule the near face follows — `inward` is the walker's left, so `along × up`
    /// holds on both — and it is the only one of the four faces of a ring whose material is on the
    /// walker's left and whose depth therefore runs *into* the masonry.
    pub fn court_rooms(mass: MassId, void: usize, edge: usize) -> Self {
        Self {
            mass,
            edge,
            void: Some(void),
            loop_side: LoopSide::Offset,
        }
    }

    /// Builds a reference to one edge of a mass's own wall **as its own rooms see it**.
    ///
    /// This is that ring's **offset** loop, a wall's thickness inside the outline the author drew: the
    /// other face of the very masonry [`new`](Self::new) names, read off the other of its two loops.
    /// It is a face like any other and takes a rhythm, a bay and a part exactly as the outer one
    /// does. A part pinned in one of these bays stands **in the mass's own rooms with its depth
    /// running away from the wall**, which is the rule [`court_rooms`](Self::court_rooms) follows on
    /// the far face of a courtyard's ring.
    ///
    /// **It measures differently from the outer face, and that is the whole of the difference between
    /// the two.** A mitre runs the offset edge's ends back past the authored edge's at both corners —
    /// *inwards* for a mass, where a courtyard's runs *outwards* — so **a mass's inner loop is the
    /// shorter of its two loops**, by one wall's thickness less at **each** end, while **a courtyard's
    /// rooms-facing loop is the longer one**. The two ends of this edge and the two ends of the outer
    /// edge are therefore two different pieces of masonry, and an arc length measured here is not an
    /// arc length there. [`BuildingSkeleton::face_length`](crate::skeleton::BuildingSkeleton::face_length)
    /// reads the offset loop's own length for a face that names it, which is what makes a rhythm laid
    /// here span it exactly.
    ///
    /// **The frame is still a rotation.** `inward` is the walker's left on every face of a ring, so
    /// `along × up` holds on all four and the determinant reads `+1` here as everywhere else; what
    /// changes from face to face is only which way the material lies. See
    /// [`PartFrame`](crate::parts::PartFrame) for the frame a placement is stood in.
    ///
    /// **And one thing is refused here, by the same rule and the same name as a courtyard's far
    /// face.** A hole is cut through both of a ring's faces from the loop the walk reads, so one
    /// authored on this loop would be a second rectangle for one hole with no second wall to put it
    /// in. That is [`Violation::OpeningOnAFarLoop`](crate::validate::Violation::OpeningOnAFarLoop),
    /// and it is the whole of what this face gives up: a rhythm, a bay, a pin and a part are not
    /// refused.
    pub fn inner(mass: MassId, edge: usize) -> Self {
        Self {
            mass,
            edge,
            void: None,
            loop_side: LoopSide::Offset,
        }
    }

    /// Returns whether this face is on a courtyard's wall rather than the mass's own.
    pub fn is_court(&self) -> bool {
        self.void.is_some()
    }

    /// Returns whether this face is on the loop a wall's thickness away from the one the author drew.
    pub fn is_offset(&self) -> bool {
        self.loop_side == LoopSide::Offset
    }

    /// Returns the authored loop this face's edge is read off, which is what a fault about it names.
    ///
    /// **Both loops of a ring answer with the loop the author drew**, because a ring is one wall and
    /// the fault is about the wall; which of its two faces a reader wants is [`loop_side`](Self::loop_side).
    pub fn loop_of(&self) -> AuthoredLoop {
        match self.void {
            Some(void) => AuthoredLoop::Void {
                mass: self.mass,
                void,
            },
            None => AuthoredLoop::Outline { mass: self.mass },
        }
    }
}

/// One face's authored rhythm: the level lines up it and the divisions along it.
///
/// # Invariants
///
/// - Both lists are **stops**, so a family of `n` stops makes `n - 1` bays and the last stop is the
///   far edge of the last one.
/// - **The stops span the face exactly**: `levels[0]` is the face's base, `levels.last()` is the wall
///   top, `divisions[0]` is `0.0` and `divisions.last()` is the edge's length. That is not a
///   convenience — it is what makes tiling structural rather than something a check has to infer:
///   adjacent bays share a stop, so there is no gap to leave and no overlap to measure.
/// - Both lists are **strictly increasing**, so no bay has zero width or zero height.
/// - An empty grid is legal and means "this face has no rhythm yet", which is a different statement
///   from a rhythm of no bays. [`Self::is_empty`] tells them apart by the document, not by a count.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FaceGrid {
    /// Elevations, strictly increasing, from the face's base to the wall top.
    pub levels: Vec<f32>,
    /// Arc-length stations along the edge, strictly increasing, from `0.0` to the edge's length.
    pub divisions: Vec<f32>,
}

impl FaceGrid {
    /// Builds a grid from its two stop lists.
    pub fn new(levels: Vec<f32>, divisions: Vec<f32>) -> Self {
        Self { levels, divisions }
    }

    /// Returns whether the grid has been authored at all.
    pub fn is_empty(&self) -> bool {
        self.levels.is_empty() && self.divisions.is_empty()
    }

    /// Returns how many storeys the levels make.
    pub fn rows(&self) -> usize {
        self.levels.len().saturating_sub(1)
    }

    /// Returns how many bays the divisions make across.
    pub fn columns(&self) -> usize {
        self.divisions.len().saturating_sub(1)
    }

    /// Returns how many bays the grid holds.
    pub fn bays(&self) -> usize {
        self.rows() * self.columns()
    }

    /// Returns one bay by its slot, or `None` when the slot is off the grid.
    ///
    /// # Invariants
    ///
    /// This is the lookup the whole slot-naming idea exists for, and it is `O(1)`: two indexed reads
    /// and one comparison each, with no search over the stops.
    pub fn bay(&self, face: FaceRef, row: usize, column: usize) -> Option<Bay> {
        if row >= self.rows() || column >= self.columns() {
            return None;
        }
        Some(Bay {
            face,
            row,
            column,
            low: self.levels[row],
            high: self.levels[row + 1],
            from: self.divisions[column],
            to: self.divisions[column + 1],
        })
    }

    /// Returns every bay, in slot order: row by row and column by column.
    ///
    /// The order is fixed and total, so two runs of anything that walks a grid — a probe, an
    /// expansion, an identity pass — see the same cells in the same sequence.
    pub fn slots(&self) -> Vec<(usize, usize)> {
        let mut slots = Vec::with_capacity(self.bays());
        for row in 0..self.rows() {
            for column in 0..self.columns() {
                slots.push((row, column));
            }
        }
        slots
    }
}

/// One cell of a face's grid.
///
/// # Invariants
///
/// The slot is the identity and the numbers are read off the grid, so a bay is always consistent with
/// the family it came from: there is no way to hold a bay whose extents disagree with its stops,
/// because a bay is not stored.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bay {
    /// The face it is a cell of.
    pub face: FaceRef,
    /// Which storey, counting from the face's base.
    pub row: usize,
    /// Which bay across, counting from the edge's start.
    pub column: usize,
    /// The elevation of its sill.
    pub low: f32,
    /// The elevation of its head.
    pub high: f32,
    /// Where it starts along the edge, in arc length.
    pub from: f32,
    /// Where it ends along the edge, in arc length.
    pub to: f32,
}

impl Bay {
    /// Returns how wide the bay is along its edge.
    pub fn width(&self) -> f32 {
        self.to - self.from
    }

    /// Returns how tall the bay is.
    pub fn height(&self) -> f32 {
        self.high - self.low
    }
}

/// How a face's stops are laid out.
///
/// # Invariants
///
/// **A rhythm is a function with a domain.** [`stops`](Rhythm::stops) answers `None` when it cannot
/// lay itself out on the extent it is handed, and the op that called it refuses rather than guessing
/// — the rule `mountain::derive_valleys` states for its own derivation ("the answer to 'the valleys
/// are wrong', never to 'the valleys are nearly right'") and the one this crate applies everywhere.
///
/// Randomness is deliberately absent: a rhythm that shuffled its own widths would be a generator
/// above the document, which is exactly what D10 rejects. A derived *starter* may choose a rhythm
/// from a published set with a seeded stream, and what it chooses is then ordinary stops.
#[derive(Debug, Clone, PartialEq)]
pub enum Rhythm {
    /// Equal parts: `count` bays of one width.
    Even {
        /// How many bays across.
        count: usize,
    },
    /// Authored stops, which must span the extent exactly. The end stops are the face's own, so a
    /// caller writes the interior ones and the two ends as the face has them.
    Absolute {
        /// The stops, strictly increasing, from `0.0` to the extent.
        stops: Vec<f32>,
    },
    /// A bookend at each end with whole modules between them.
    ///
    /// The remainder — what the modules do not account for — is either absorbed by **one declared**
    /// middle module or refused. It is declared because the alternative is a rule that silently
    /// stretches whichever bay it likes, and an author who cannot say *which* bay stretches cannot
    /// predict their own facade.
    Rhythmic {
        /// The width of the bay at each end.
        bookend: f32,
        /// The width of one module between them.
        module: f32,
        /// Whether the remainder is absorbed by the middle module rather than refused.
        flexible: bool,
    },
}

impl Rhythm {
    /// Returns the stops this rhythm lays out over an extent, or `None` when it cannot.
    ///
    /// # Invariants
    ///
    /// The answer always begins at `0.0` and ends at `extent`, and is always strictly increasing —
    /// those are the grid's own invariants and the reason this is the one place a family is born. An
    /// extent at or below [`GRID_TOLERANCE`] has no room for a bay and is refused here rather than by
    /// every caller.
    pub fn stops(&self, extent: f32) -> Option<Vec<f32>> {
        if extent <= GRID_TOLERANCE {
            return None;
        }
        match self {
            Self::Even { count } => {
                if *count == 0 || *count + 1 > MAX_STOPS {
                    return None;
                }
                let step = extent / *count as f32;
                Some((0..=*count).map(|index| step * index as f32).collect())
            }
            Self::Absolute { stops } => {
                if stops.len() < 2 || stops.len() > MAX_STOPS || !increasing(stops) {
                    return None;
                }
                let last = stops[stops.len() - 1];
                if stops[0].abs() > GRID_TOLERANCE || (last - extent).abs() > GRID_TOLERANCE {
                    return None;
                }
                Some(stops.clone())
            }
            Self::Rhythmic {
                bookend,
                module,
                flexible,
            } => {
                let (bookend, module) = (*bookend, *module);
                if bookend < 0.0 || module <= GRID_TOLERANCE {
                    return None;
                }
                let between = extent - 2.0 * bookend;
                if between <= GRID_TOLERANCE {
                    return None;
                }
                let whole = (between / module).floor();
                if whole < 1.0 {
                    return None;
                }
                let whole = whole as usize;
                if whole + 3 > MAX_STOPS {
                    return None;
                }
                let remainder = between - whole as f32 * module;
                if remainder > GRID_TOLERANCE && !*flexible {
                    return None;
                }
                let middle = whole / 2;
                let mut stops = vec![0.0, bookend];
                for index in 0..whole {
                    let width = if index == middle {
                        module + remainder
                    } else {
                        module
                    };
                    let next = stops[stops.len() - 1] + width;
                    stops.push(next);
                }
                stops.push(extent);
                Some(stops)
            }
        }
    }
}

/// Returns whether a list of stops is strictly increasing.
pub fn increasing(stops: &[f32]) -> bool {
    stops.windows(2).all(|pair| pair[1] > pair[0])
}

/// Returns the plan length of one face's edge, or `None` when there is no such face.
///
/// `O(1)` in the mass's node count: it reads the two nodes the edge joins.
pub fn face_length(mass: &Mass, edge: usize) -> Option<f32> {
    loop_length(&mass.outline, edge)
}

/// Returns the vertical extent of one face: the lowest ground it stands on, and the wall top.
///
/// The floor of the range is the **lower** of the edge's two node elevations, because the wall top is
/// level and it is the shorter side of a trapezoidal face that decides what a storey can be. `None`
/// when there is no such edge.
pub fn face_extent(mass: &Mass, edge: usize) -> Option<(f32, f32)> {
    loop_extent(&mass.outline, edge, mass.top)
}

/// Returns the plan length of one edge of any authored loop, or `None` when the edge is not there.
///
/// # Invariants
///
/// **It takes the loop rather than the mass, which is what makes a courtyard's wall a face like any
/// other.** The mass's own helpers are this function over [`Mass::outline`], and they keep their
/// signature so a caller reading a mass's wall does not have to know a courtyard exists; a caller
/// reading a courtyard's hands this the void loop instead. `O(1)`: it reads the two nodes the edge
/// joins.
pub fn loop_length(nodes: &[PlanNode], edge: usize) -> Option<f32> {
    let count = nodes.len();
    if count < 3 || edge >= count {
        return None;
    }
    let from = nodes[edge].position;
    let to = nodes[(edge + 1) % count].position;
    Some(crate::math::distance(from, to))
}

/// Returns the vertical extent of one edge of any authored loop: the lowest ground it stands on, and
/// the wall top standing over it.
///
/// The floor of the range is the **lower** of the edge's two node elevations, for the reason
/// [`face_extent`] gives. `None` when the loop has no such edge.
pub fn loop_extent(nodes: &[PlanNode], edge: usize, top: f32) -> Option<(f32, f32)> {
    let count = nodes.len();
    if count < 3 || edge >= count {
        return None;
    }
    let from = nodes[edge].elevation;
    let to = nodes[(edge + 1) % count].elevation;
    Some((from.min(to), top))
}

/// The grids of a building, one per face that has been given a rhythm.
///
/// # Invariants
///
/// A map and not a list, and keyed by the face: a rhythm is looked up by the face an edit names, and
/// a face that has no grid is simply absent — which is a different state from a face whose grid has
/// no bays, exactly as [`FaceGrid::is_empty`] says. `BTreeMap` rather than a hash map because every
/// walk over the grids has to be reproducible, and an ordered map makes that a property of the type
/// instead of a promise.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Grids {
    faces: BTreeMap<FaceRef, FaceGrid>,
}

impl Grids {
    /// Builds an empty set of grids.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns one face's grid, or `None` when that face has no rhythm.
    pub fn get(&self, face: FaceRef) -> Option<&FaceGrid> {
        self.faces.get(&face)
    }

    /// Returns whether a face has a rhythm.
    pub fn holds(&self, face: FaceRef) -> bool {
        self.faces.contains_key(&face)
    }

    /// Writes a face's family, **replacing** whatever was there.
    ///
    /// # Invariants
    ///
    /// Replacement is the whole of the semantics: nothing merges with the family already present,
    /// because a derivation "has no way to know which parts of an edited valley the author meant to
    /// keep" (`mountain::derive_valleys`). The removed family is returned so a caller that wants an
    /// undo can keep it, and so a probe can measure that the replacement happened.
    pub fn replace(&mut self, face: FaceRef, grid: FaceGrid) -> Option<FaceGrid> {
        self.faces.insert(face, grid)
    }

    /// Removes a face's family and returns it.
    pub fn take(&mut self, face: FaceRef) -> Option<FaceGrid> {
        self.faces.remove(&face)
    }

    /// Returns every face that has a rhythm, in key order.
    pub fn faces(&self) -> Vec<FaceRef> {
        self.faces.keys().copied().collect()
    }

    /// Returns how many faces carry a rhythm.
    pub fn len(&self) -> usize {
        self.faces.len()
    }

    /// Returns whether no face carries a rhythm.
    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }
}
