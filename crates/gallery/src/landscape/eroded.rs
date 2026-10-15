//! The massif after the water has been over it.
//!
//! # What this is
//!
//! [`SPEC/mountain.md`](../../../../.agents/SPEC/mountain.md) R6 hands the massif's block
//! faces to a downstream pipeline, and this is that pipeline: the mesh is read onto a
//! lattice, the lattice is eroded, and the **displacement** between the two is applied back
//! to the mesh's own vertices.
//!
//! Applying it back rather than drawing the lattice is the whole of the decision, and it is
//! made here rather than in `terrain` because it is a statement about this scene. A lattice
//! drawn directly would throw away everything the scene is for — the mesh's faces carry the
//! belt they were expanded from, so a pixel still resolves to the crest an author wrote, and
//! a lattice has nowhere to carry that. Displacing keeps the authored skeleton as the shape
//! of the mountain and adds to it only what the water removed, which is what erosion is:
//! a **displacement field**, not a replacement for the thing it acts on.
//!
//! # What the lattice's spacing is measured against
//!
//! **The mesh, not the mountain.** A relief of nine hundred over a plan eighteen hundred
//! across says nothing about how much of an erosion pass the geometry can carry: this
//! massif's own faces are sixty units on a crest and twenty on a rib, so a lattice five
//! times finer than that would solve for gullies narrower than the triangles asked to draw
//! them, and the answer would be thrown away before it was ever seen. [`CELL`] is therefore
//! stated against [`mountain::Tessellation::step`] and sits between the two. Measured over
//! the three spacings, the eroded surface barely changes with a finer lattice — the worst
//! displacement is 108 units at twenty-four and 86 at twelve — while the cost quadruples.
//!
//! # The rim
//!
//! The massif's mesh ends at its footprint and the scene's plain begins there at the same
//! elevation, so anything the solve does at the rim opens a crack between two surfaces that
//! were built to meet. Measured, it moves up to seventy-two units, which against a plain
//! standing still is a step in the ground rather than a valley mouth. The displacement is
//! therefore faded to nothing over the outermost [`RIM_FADE_CELLS`] cells: the rim is a free
//! boundary, and a scene that levels its ground to meet the massif's needs the two to still
//! meet after the water has gone over.

use mountain::FacetMesh;
use terrain::{Books, ErodeParams, HeightField, Hydraulic, erode_traced, rasterize};

/// The lattice's plan spacing, in the mesh's own units.
///
/// **Between the mesh's crest step and its shortest edge.** See the module note for what it
/// is measured against; the numbers it produces are in `gallery`'s `erode_probe` example.
pub const CELL: f32 = 24.0;

/// How many cells the displacement is faded over, from the rim inwards.
///
/// Three cells is about one of the mesh's own crest steps, so the fade finishes before a
/// face has finished — which is what keeps it from reading as a bevel on the mountain's
/// edge rather than as the water stopping where the ground stops.
pub const RIM_FADE_CELLS: f32 = 3.0;

/// One massif, read onto a lattice and eroded, with both fields kept.
///
/// # Invariants
///
/// **The lattice, the erosion and the rim fade are one computation.** A scene that wanted the
/// solve and a probe that wanted to measure it must not be able to disagree about the
/// parameters or the spacing, so neither is a free function over either: both read this.
pub struct Erosion {
    /// The massif as it was read onto the lattice.
    pub field: HeightField,
    /// The same lattice after the solve.
    pub eroded: HeightField,
    /// The transport books, which are how a caller checks the mass line rather than
    /// assuming it. The scene draws none of them; the probe that chose the parameters does.
    pub books: Books,
    /// How far each cell stands from the rim, in cells, from
    /// [`HeightField::edge_distance`].
    pub margin: Vec<f32>,
}

impl Erosion {
    /// Reads a mesh onto a lattice and erodes it.
    pub fn of(mesh: &FacetMesh) -> Self {
        let field =
            rasterize(mesh, CELL).expect("the massif is far larger than the lattice's own spacing");
        let (eroded, books) =
            erode_traced(&ErodeParams::default(), &field, &Hydraulic).expect("a field is a field");
        let margin = field.edge_distance();
        Erosion {
            field,
            eroded,
            books,
            margin,
        }
    }

    /// Returns how far a point in the plan is moved, in the mesh's own units.
    ///
    /// # Invariants
    ///
    /// - **It is zero on the rim and reaches the full displacement [`RIM_FADE_CELLS`] cells
    ///   in,** linearly, which is a claim about the boundary and not about the rock: this
    ///   scene's plain is authored to meet the massif exactly, so a displacement that
    ///   survived to the rim would be a crack rather than an eroded edge.
    /// - **The fade starts at the first ring of cells rather than at the rim line**, because
    ///   a vertex standing *on* the rim has no cell under it and the nearest cell that has
    ///   rock is one step in. Measuring the fade from zero would leave that vertex at a
    ///   third of its displacement and put the seam back where it started.
    /// - **A point off the massif is not moved at all.** A displacement has no answer out
    ///   there, and its neutral is zero rather than any elevation the field happens to carry
    ///   — a vertex just outside the footprint would otherwise be lifted by the massif's own
    ///   base level, which is the largest number in the lattice.
    pub fn displacement(&self, plan: [f32; 2]) -> f32 {
        let delta: Vec<f32> = self
            .eroded
            .heights()
            .iter()
            .zip(self.field.heights())
            .map(|(after, before)| after - before)
            .collect();
        let Some(moved) = self.field.sample_run(&delta, plan) else {
            return 0.0;
        };
        let fade = ((self.field.sample_run(&self.margin, plan).unwrap_or(0.0) - 1.0)
            / RIM_FADE_CELLS)
            .clamp(0.0, 1.0);
        moved * fade
    }

    /// Returns the mesh with the displacement applied to its own vertices.
    ///
    /// # Invariants
    ///
    /// - **It moves the points and nothing else.** Every face's belt, block, stations and
    ///   ruggedness survive, so the id a pixel carries still names the authored element it
    ///   was authored as — the same rule [`crate::landscape::stood_at`] is written under.
    /// - **It moves elevation only.** The plan is where the massif is, and displacing it
    ///   would slide the rock sideways rather than take any of it away.
    pub fn applied(&self, mesh: &FacetMesh) -> FacetMesh {
        let mut moved = mesh.clone();
        for point in &mut moved.points {
            point[1] += self.displacement([point[0], point[2]]);
        }
        moved
    }
}
