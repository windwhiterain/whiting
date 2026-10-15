//! The massif after the water has been over it.
//!
//! # What this is
//!
//! [`SPEC/mountain.md`](../../../../.agents/SPEC/mountain.md) R6 hands the massif's block
//! faces to a downstream pipeline, and this is that pipeline: the mesh is read onto a lattice,
//! the lattice is eroded, and **the lattice is what the scene draws**.
//!
//! # Why the lattice and not the massif's own points, which was tried first
//!
//! The massif this scene draws has **one hundred and seventy four points**. The erosion field
//! has **six thousand** cells. Displacing those hundred and seventy four vertices by a
//! bilinearly read displacement therefore throws most of the solve away before the frame sees
//! it, and the first render showed it: a massif of sharp block faces, indistinguishable from
//! the same scene with no erosion in it at all. That is the measurement this module is built
//! around, and it is why the displacement is not where it first went.
//!
//! The identity survives the change. Each lattice cell names the **facet it was read from**,
//! which carries the belt it was expanded from, so a pixel still resolves to the crest an
//! author wrote — at the lattice's resolution rather than the facet's. That is a loss of
//! precision and not of the feature.
//!
//! # What the lattice's spacing is measured against
//!
//! **The silhouette, not the solve.** The eroded surface barely changes with a finer lattice —
//! the worst displacement is 108 units at twenty four and 87 at twelve — but the *drawing*
//! changes a great deal, because a cell with a corner outside the footprint is dropped whole
//! and the massif's outline is then the footprint quantised to the cell. See [`CELL`].
//!
//! # The rim, and the berm
//!
//! The massif's mesh ends at its footprint and the scene's plain begins there at the same
//! elevation, so anything the solve does at the rim opens a crack between two surfaces that
//! were built to meet. Measured by `erode_probe`, the solve does not cut the rim so much as
//! **pile a berm on it**: a mean of **+30 units** in the first cell and a half, decaying to
//! +17, +5 and then negative further in. That is the erosion answering an artificial
//! boundary — the massif is *cut off* there rather than ending — and it put the massif's foot
//! thirty units above a plain it is supposed to meet, throwing a black shadow of its own toe
//! across it.
//!
//! Two things fix it, and the second is the one that mattered:
//!
//! - the displacement is faded back to the massif's own over the outermost [`RIM_FADE_CELLS`]
//!   cells, so the two surfaces still meet;
//! - **the fade is applied in [`Erosion::surface`] and drawn from there.** It was written
//!   twice — once for the displaced mesh, once, wrongly, absent from the lattice — and the two
//!   drawings put the foot in two different places. There is now one surface and every caller
//!   reads it.

use definition::{PackedBuilder, PackedList};
use mountain::FacetMesh;
use terrain::{Books, ErodeParams, HeightField, Hydraulic, Provenance, erode_traced, rasterize};

use crate::identity::{ID_OUTPUT, Identity, U_OUTPUT, V_OUTPUT};

/// The lattice's plan spacing, in the mesh's own units.
///
/// # Invariants
///
/// **It is the geometry's resolution and not a sampling budget**, which is what changed when
/// the scene stopped drawing the massif's own hundred and seventy four points and started
/// drawing the lattice. A displacement read at the authored vertices can only be as fine as
/// those vertices and the solve is thrown away; a lattice *is* the drawing, so its cell is
/// how big the facets of this mountain are.
///
/// What it is measured against is then the **silhouette**, because that is the one thing a
/// lattice cannot hide: a cell with a corner outside the footprint is dropped whole, so the
/// massif's outline is the footprint quantised to the cell, and the steps are one cell long.
/// Twelve units is about three pixels of the massif in this scene's own frame, which reads as
/// rough ground where the mountain meets the plain rather than as the staircase it is. Six
/// units is about a pixel and costs four times the triangles; twenty four is about seven and
/// the steps are plainly visible.
///
/// **Six, lowered from twelve** (2026-10-08): three-pixel steps are exactly the texel artifact
/// the picture was judged to have — a rim and a flank quantised to a visible grid — and the
/// fourfold triangle cost is nothing to an offline renderer. The solve itself also reads finer:
/// a drainage line narrower than one cell of twelve cannot exist at all, which is half of why
/// the eroded flanks carried grid-aligned rills.
pub const CELL: f32 = 6.0;

/// How many cells the displacement is faded over, from the rim inwards.
///
/// Three cells is about a sixth of the way up the massif's own toe, so the fade finishes
/// before the ground has begun to climb — which is what keeps it from reading as a bevel on
/// the mountain's edge rather than as the water stopping where the ground stops. It is
/// **not** long enough to be a general answer to the berm the solve raises at a free boundary
/// (measured at `+30` units in `erode_probe`, decaying over about six cells); it is a scene
/// constant and the measurement beside it is what says so.
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
    /// Which face of the massif each cell was read from, and where on it the cell stood.
    pub provenance: Provenance,
    /// The transport books, which are how a caller checks the mass line rather than
    /// assuming it. The scene draws none of them; the probe that chose the parameters does.
    pub books: Books,
    /// How far each cell stands from the rim, in cells, from
    /// [`HeightField::edge_distance`].
    pub margin: Vec<f32>,
    /// The surface the scene draws: the eroded field, faded back to the massif's own at the rim.
    ///
    /// # Invariants
    ///
    /// **It is the one place the fade is applied.** A caller asking what the picture shows gets
    /// this field and not the solve's own output, and a caller displacing the authored mesh
    /// gets `surface − field`. Written twice it was already wrong once: the lattice drew the raw
    /// solve and the displaced mesh drew the faded one, so the two put the massif's foot in two
    /// different places — and the raw one kept a **30 unit berm** right round the toe (measured
    /// by `erode_probe`), standing above a plain the scene levels to meet it and throwing a
    /// shadow across it.
    pub surface: HeightField,
}

impl Erosion {
    /// Reads a mesh onto a lattice and erodes it.
    pub fn of(mesh: &FacetMesh) -> Self {
        let read =
            rasterize(mesh, CELL).expect("the massif is far larger than the lattice's own spacing");
        let (eroded, books) = erode_traced(&ErodeParams::default(), &read.field, &Hydraulic)
            .expect("a field is a field");
        let margin = read.field.edge_distance();
        // The fade starts at the **first ring of cells**, not at zero: a vertex standing on
        // the rim has no cell under it, so the nearest cell with rock is one step in, and
        // measuring from zero left that vertex a third displaced.
        let fade: Vec<f32> = margin
            .iter()
            .map(|cells| ((*cells - 1.0) / RIM_FADE_CELLS).clamp(0.0, 1.0))
            .collect();
        let surface = read
            .field
            .with_surface(&eroded, &fade)
            .expect("a field is the same length as the run that fades it");
        Erosion {
            field: read.field,
            eroded,
            provenance: read.provenance,
            books,
            margin,
            surface,
        }
    }

    /// Returns how far a point in the plan is moved, in the mesh's own units.
    ///
    /// # Invariants
    ///
    /// - **It is zero on the rim and reaches the full displacement [`RIM_FADE_CELLS`] cells
    ///   in,** which is a claim about the boundary and not about the rock: this scene's plain
    ///   is authored to meet the massif exactly, so a displacement that survived to the rim
    ///   would be a crack rather than an eroded edge. It is read off [`Erosion::surface`],
    ///   which is the same fade the lattice is drawn from.
    /// - **A point off the massif is not moved at all.** A displacement has no answer out
    ///   there, and its neutral is zero rather than any elevation the field happens to carry
    ///   — a vertex just outside the footprint would otherwise be lifted by the massif's own
    ///   base level, which is the largest number in the lattice.
    pub fn displacement(&self, plan: [f32; 2]) -> f32 {
        self.surface.sample(plan) - self.field.sample(plan)
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

    /// Returns the eroded field as a triangle mesh on the lattice's own vertices.
    ///
    /// # Invariants
    ///
    /// - **One vertex per lattice cell centre, and a cell is drawn only when all four of its
    ///   corners have rock under them.** A cell with a corner outside is dropped whole rather
    ///   than drawn short, because a triangle with one corner missing is a notch along the rim
    ///   — and the rim is the one place in this scene where the massif meets another surface.
    /// - **The heights are the eroded ones and the normals are their own gradient.** A normal
    ///   per cell would shade a smooth hillside as tiled facets, and this mesh has forty times
    ///   as many cells as the massif it came from had faces.
    /// - **The identity is the face each cell was read from**, so a pixel still names the
    ///   authored facet standing under it. The whole of that cell's share is written at each
    ///   of its corners, which makes them constant across the cell: a lattice cannot say
    ///   which facet a pixel is on at a resolution finer than the lattice, and interpolating
    ///   an id between two facets would name ids that belong to neither.
    /// - **The winding is [`crate::geometry::heightfield`]'s**, which is the winding this
    ///   pipeline already draws ground with and the one every other height field in the
    ///   gallery is drawn with.
    pub fn lattice(&self, mesh: &FacetMesh, identity: &Identity) -> PackedList {
        let schema = crate::identity::massif_extras().point_schema();
        // The layout is the schema's own, read through the same function the renderer reads
        // it with; writing the field order out here would be a second copy of the rule.
        let layout = graphics::layout::packed_layout(&schema)
            .expect("the massif's own point schema is a struct of components");
        let field = &self.surface;
        // The cell's four corners, in the winding `geometry::heightfield` writes them, so
        // this mesh is drawn by the same convention as every other ground in the gallery.
        let corners = |column: usize, row: usize| {
            [
                (column, row),
                (column + 1, row),
                (column + 1, row + 1),
                (column, row + 1),
            ]
        };
        let covered = |column: usize, row: usize| {
            field
                .index(column, row)
                .is_some_and(|at| field.is_inside(at))
        };
        // The texture slot carries the face's own ruggedness and steepness on the authored
        // mesh. A lattice cell has neither of its own, and the surface reads its steepness
        // off the normal it is drawn with rather than off a number the field no longer has.
        let texture = [0.0, 0.0];
        let mut points = PackedBuilder::new(layout);
        points.reserve(field.cells() * 6);
        for row in 0..field.height().saturating_sub(1) {
            for column in 0..field.width().saturating_sub(1) {
                if !corners(column, row).into_iter().all(|(c, r)| covered(c, r)) {
                    continue;
                }
                let cell = field
                    .index(column, row)
                    .expect("the cell is on the lattice");
                let face = &mesh.faces[self.provenance.faces[cell] as usize];
                let id = identity
                    .id_of(face.attributes.belt)
                    .expect("every cell's face was expanded from a belt the identity numbered")
                    as f64;
                // The face's own belt coordinate, interpolated to where this cell stood on
                // it, from the same three weights the cell's height was built from.
                let mut uv = [0.0_f32; 2];
                for corner in 0..3 {
                    let share = self.provenance.weights[cell][corner];
                    uv[0] += share * face.uv[corner][0];
                    uv[1] += share * face.uv[corner][1];
                }
                let uv = [f64::from(uv[0]), f64::from(uv[1])];
                // Two triangles per cell: the first takes the corner the other does not.
                for corner in [0usize, 3, 1, 1, 3, 2] {
                    let (column, row) = corners(column, row)[corner];
                    let at = field
                        .index(column, row)
                        .expect("the cell is on the lattice");
                    let position = [
                        field.origin()[0] as f64 + column as f64 * field.cell() as f64,
                        field.height_at(at) as f64,
                        field.origin()[1] as f64 + row as f64 * field.cell() as f64,
                    ];
                    let normal = self.normal_at(column, row);
                    let written = points
                        .field(ID_OUTPUT, &[id])
                        .and_then(|points| points.field(U_OUTPUT, &[uv[0]]))
                        .and_then(|points| points.field(V_OUTPUT, &[uv[1]]))
                        .and_then(|points| points.field("normal", &normal))
                        .and_then(|points| points.field("position", &position))
                        .and_then(|points| points.field("texture", &texture));
                    written.expect("a generator writes whole elements of its own layout");
                }
            }
        }
        points
            .finish()
            .expect("every generator writes whole elements of its own layout")
    }

    /// Returns one lattice vertex's normal, from the eroded field's own gradient.
    ///
    /// # Invariants
    ///
    /// **A vertex with a neighbour on one side only is a one-sided difference, and one with
    /// neither is flat.** The lattice's boundary has no centre difference to take — half its
    /// neighbours are outside the massif — and borrowing the plain's gradient there would
    /// tilt the rim towards whatever the plain happens to be doing.
    fn normal_at(&self, column: usize, row: usize) -> [f64; 3] {
        let field = &self.surface;
        let cell = field.cell() as f64;
        let read = |column: isize, row: isize| {
            usize::try_from(column)
                .ok()
                .zip(usize::try_from(row).ok())
                .and_then(|(column, row)| field.index(column, row))
                .filter(|at| field.is_inside(*at))
                .map(|at| field.height_at(at) as f64)
        };
        let column = column as isize;
        let row = row as isize;
        let here = read(column, row).unwrap_or(0.0);
        // The slope over a **two-cell span** where the field has one, falling back to the one-cell
        // span and then to the one side that exists. `None` on both sides leaves the axis level
        // rather than guessing.
        //
        // **Two cells, raised from one** (2026-10-08): a one-cell central difference reads the
        // solve's own micro-relief — the rills and pits hydraulic transport leaves at one to two
        // cells of wavelength — straight into the normal, and a normal per vertex then shades
        // every one of those cells as its own facet: the point artifact the eroded picture was
        // judged to have. A span of two cells averages over the micro-relief while leaving the
        // hillside slope, which lives at tens of cells, untouched.
        let slope = |dc: isize, dr: isize| match (
            read(column + 2 * dc, row + 2 * dr),
            read(column - 2 * dc, row - 2 * dr),
        ) {
            (Some(a), Some(b)) => (a - b) / (4.0 * cell),
            _ => match (read(column + dc, row + dr), read(column - dc, row - dr)) {
                (Some(a), Some(b)) => (a - b) / (2.0 * cell),
                // **One-sided reads keep the sign of the two-sided one**: the slope towards an
                // existing neighbour is that neighbour's height minus this one over the step to
                // it. The ahead case was written the other way round, which turned the rim's
                // west and north halves' normals downhill — a dark band along two sides of the
                // footprint, measured at up to 39 units of slope error per vertex.
                (Some(a), None) => (a - here) / cell,
                (None, Some(b)) => (here - b) / cell,
                (None, None) => 0.0,
            },
        };
        let slope_x = slope(1, 0);
        let slope_z = slope(0, 1);
        let normal = [-slope_x, 1.0, -slope_z];
        let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        if length > f64::EPSILON {
            [normal[0] / length, normal[1] / length, normal[2] / length]
        } else {
            [0.0, 1.0, 0.0]
        }
    }
}
