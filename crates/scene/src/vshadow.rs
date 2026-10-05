//! The virtual shadow map: a light's depth recorded into sparse pages, at the precision its
//! casters asked for and only where the frame can see.
//!
//! # What this module answers
//!
//! A fixed shadow page is one projection fitted around one box, and every answer read from it is
//! at one precision: a coin and a wall standing in the same scene get the same texel size, a
//! receiver at the rim of the volume is shaded against as coarse a texel as one at the centre,
//! and geometry the camera cannot see spends pages that no fragment will read. This module makes
//! all three of those a decision rather than a consequence.
//!
//! It is asked one question and answers it completely: given each light's volume, the casters it
//! sees and the resolution each of them asked for, and given what the camera can see, which pages
//! exist, where in the atlas each one physically lives, and which casters are drawn into it. It
//! holds no GPU resource, reads no document and emits no pass — those belong to the compiler — and
//! the split is what lets the whole of it be tested as arithmetic.
//!
//! # Shape
//!
//! - [`LightVolume`] is one light's own view and projection, which is all the geometry a page grid
//!   needs: the grid lies over the light's normalised device coordinate plane, so it is an
//!   *angular* grid whose texels are equal angles rather than equal distances.
//! - [`Caster`] is one object that may block the light, with the density it asked its shadow to be
//!   recorded at and the handle its caller needs to draw it again.
//! - [`Frustum`] is what a camera can see, as eight corners. The allocator derives the six planes
//!   and never needs more.
//! - [`allocate`] returns an [`Allocation`]: the page table, the pages with their atlas slots and
//!   their casters, and the size the atlas has to be.
//!
//! # Invariants
//!
//! - **A page exists where the frame can read it and a caster covers it.** A page no camera
//!   frustum can touch is not allocated at all, and a caster outside every frustum is *not*
//!   dropped: it still draws into every kept page its projection covers, because its shadow can
//!   fall on a receiver the camera is looking at. Clipping is a property of the page, never of the
//!   caster.
//! - **The clipping test never drops a page that could matter.** It is a separating-plane test
//!   over the face normals of both volumes, which errs on the keeping side by construction: two
//!   convex volumes that overlap have no separating axis at all, so no plane this test tries can
//!   cut them apart. Pages may be over-allocated at grazing angles; a shadow is never lost.
//! - **Overflow is an answer, never a rounding.** A grid past [`MAX_PAGES_PER_SIDE`], a light
//!   whose pages do not fit its atlas layer, and a shadowing light with nothing to cast are each a
//!   [`Shortfall`] naming the light and the demand. The alternative is a scene that renders and
//!   quietly records its shadows at a fraction of the resolution it asked for.
//! - **A caster's chain of levels is never broken.** A caster is drawn into its own level and
//!   every coarser one, never into a page finer than the density it asked for, so the coarsest
//!   level a light has always holds it. The reading end is the other half of the same promise: a
//!   receiver walks the levels from finest to coarsest and a level that recorded **nothing** at
//!   its point does not answer, so a receiver that finds nothing at a finer level always finds
//!   something further down the walk. The two halves are one rule, and a page that merely exists
//!   is not the same as a page that recorded this point's caster.
//! - **The table is the only lookup a shader is given**, and it is laid out so that resolving a
//!   page is one mask test and one popcount. See [`table_words`] and [`Allocation::resolve`],
//!   which is that walk written in Rust so the two ends of the encoding are tested against each
//!   other.
//!
//! # Example
//!
//! ```
//! use whiting_scene::standard::math::{look_at, perspective};
//! use whiting_scene::vshadow::{
//!     Budget, Caster, LightPlan, LightVolume, MIN_PAGES_PER_SIDE, allocate, pages_at_level,
//! };
//!
//! // A spot light four units above a table, pointing down at it, with one coin-sized caster
//! // asking for its shadow to be recorded at five hundred texels per unit.
//! let volume = LightVolume {
//!     view: look_at([0.0, 4.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, -1.0]),
//!     projection: perspective(1.2, 1.0, 0.2, 4.2),
//! };
//! let plan = LightPlan {
//!     label: "over the table".to_owned(),
//!     volume,
//!     faces: 1,
//!     casters: vec![Caster {
//!         handle: 0,
//!         position: [0.4, 0.6, 0.0],
//!         radius: 0.05,
//!         density: 500.0,
//!     }],
//! };
//! let budget = Budget {
//!     atlas_side_pages: 16,
//!     clip: Vec::new(),
//! };
//!
//! let allocation = allocate(&[plan], &budget).expect("the light has a caster to size itself by");
//! let light = &allocation.lights[0];
//! assert!(light.pages_per_side >= MIN_PAGES_PER_SIDE, "a grid is at least four pages wide");
//! assert!(light.levels >= 2, "and more than one level deep, or the density has nowhere to go");
//! assert!(!allocation.pages.is_empty(), "the coin is covered somewhere");
//! // Every page of the light sits inside its own atlas layer, which is what the shader's one
//! // texture lookup relies on.
//! for page in &allocation.pages {
//!     assert!(page.slot < light.slots);
//!     assert!(page.page_x < pages_at_level(light.pages_per_side, page.level));
//! }
//! // And the table answers for the coin's own place on the floor at the finest level, which is
//! // the whole point of asking for a density in the first place.
//! let coin = allocation
//!     .resolve(&volume, 0, [0.4, 0.6, 0.0])
//!     .expect("the coin's own spot on the table is covered");
//! assert_eq!(coin.level, 0, "a caster that asked for the finest level is recorded there");
//! ```

use std::collections::HashMap;

use thiserror::Error;

use crate::standard::math::{Matrix, add, apply, cross, dot, normalize, scale, subtract};

/// The side of one page, in texels. Every page of every level is this many texels across; a level
/// is how much *world* a page covers, not how big it is.
pub const PAGE_SIZE: u32 = 128;

/// The smallest grid a light gets, in pages per side. Below four the levels run out before
/// adaptive precision has anywhere to go, and a grid this size costs sixteen pages of address
/// space, which nothing downstream notices.
pub const MIN_PAGES_PER_SIDE: u32 = 4;

/// How many levels of precision one grid carries, the finest being level zero.
pub const MAX_LEVELS: u32 = 4;

/// The largest grid a light may be given, in pages per side. This is an address-space ceiling
/// rather than a memory one: a grid of this side costs the table [`table_words`] lays out, which
/// is under twelve thousand words, and past it a page coordinate and an atlas side no longer fit
/// into the one header word a shader reads.
pub const MAX_PAGES_PER_SIDE: u32 = 512;

/// Header words of one light's table section: the grid side and the atlas side packed into one,
/// then the level count and the face count packed into one.
pub const TABLE_HEAD_WORDS: u32 = 2;

/// Words between the header and the first row of any level: one offset per level, so a shader
/// walks straight into a section without repeating this module's cumulative arithmetic.
pub const PREFIX_WORDS: u32 = MAX_LEVELS;

/// How many bits a header field is packed into.
const PACK: u32 = 16;

/// Packs two page counts into the one word a shader reads.
fn pack(low: u32, high: u32) -> u32 {
    low | (high << PACK)
}

/// The low half of a packed header word.
pub fn unpack_low(word: u32) -> u32 {
    word & ((1 << PACK) - 1)
}

/// The high half of a packed header word.
pub fn unpack_high(word: u32) -> u32 {
    word >> PACK
}

/// One object that may block a light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Caster {
    /// The caller's handle for it, echoed back in [`Page::casters`] so an emitter knows what to
    /// draw. This module never interprets it.
    pub handle: u32,
    /// Where it is, in world space.
    pub position: [f64; 3],
    /// How far its geometry reaches from there, in world units.
    pub radius: f64,
    /// How many texels per world unit it asked its shadow to be recorded at. Zero or less means
    /// the coarsest level only: it still casts, and it never drives a page finer than the root.
    pub density: f64,
}

/// One light's own space, which is everything a page grid is measured in.
///
/// # Invariants
///
/// The two matrices are the pair a projector is built from — the light's world-to-view and its
/// view-to-clip — rather than the single matrix a shader samples with, and the reason is that a
/// grid is measured in **view space**. A combined product has no slot holding the tangent of the
/// cone's half angle and none holding the depth range: multiplying a projection by a view smears
/// each of those numbers across a whole row. So each is read from the one matrix that states it
/// alone, and a world point reaches clip space through [`LightVolume::clip`], which applies the
/// two in the order a shader's single matrix already holds.
///
/// Both are read from the light's own projection rather than restated, so a volume cannot carry
/// one number about its cone and another about its depth range. The projection is a
/// [`crate::standard::math::perspective`], whose shape [`LightVolume::near_far`] inverts: a light
/// whose page is drawn through an orthographic projection has no angular grid and does not come
/// through here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightVolume {
    /// World space into the light's own view space, which looks down its negative z.
    pub view: Matrix,
    /// The light's view space into its clip space, which is the projection its pages are drawn
    /// through and the one a fragment's depth is compared against.
    pub projection: Matrix,
}

impl LightVolume {
    /// The tangent of the cone's half angle at x and at y, read out of the projection's own
    /// scales: a normalised coordinate is a view offset over the depth over this.
    pub fn half_tangent(&self) -> [f64; 2] {
        [1.0 / self.projection[0], 1.0 / self.projection[5]]
    }

    /// The nearest and furthest distance the grid reaches, read out of the depth mapping.
    ///
    /// A projection of this shape sends depth zero to the near plane and one to the far one, and
    /// inverting either of those two statements is all these two lines are. The two numbers live
    /// in the **third column** of a column major matrix — the row that produces a clip space z —
    /// which is not where a row major reader would look for them, and a matrix that is the product
    /// of a projection and a view has both of them smeared across a row, which is why
    /// [`LightVolume`] keeps the two apart.
    pub fn near_far(&self) -> (f64, f64) {
        let slope = self.projection[10];
        let offset = self.projection[14];
        (offset / slope, offset / (1.0 + slope))
    }

    /// The light's own right, up and backward directions, which are the rows of its view matrix.
    fn basis(&self) -> ([f64; 3], [f64; 3], [f64; 3]) {
        (
            [self.view[0], self.view[4], self.view[8]],
            [self.view[1], self.view[5], self.view[9]],
            [self.view[2], self.view[6], self.view[10]],
        )
    }

    /// Where the light is, which is its own view space origin in world space.
    ///
    /// The view matrix is `row · point + shift` per axis, and its three shifts are the eye's own
    /// coordinates along the three basis rows, so recovering the eye needs **all three**: reading
    /// only the first would place it at the point whose view space x is zero, which is a plane and
    /// not a point.
    fn eye(&self) -> [f64; 3] {
        let (right, up, back) = self.basis();
        add(
            add(scale(right, -self.view[12]), scale(up, -self.view[13])),
            scale(back, -self.view[14]),
        )
    }

    /// The view space of a world point.
    fn view_of(&self, point: [f64; 3]) -> [f64; 3] {
        let view = apply(&self.view, point);
        [view[0], view[1], view[2]]
    }

    /// The clip space of a world point, which is the point through the light's own projection.
    fn clip(&self, point: [f64; 3]) -> [f64; 4] {
        let view = self.view_of(point);
        apply(&self.projection, view)
    }

    /// The normalised device coordinate of a world point, or `None` when the point is at or behind
    /// the light and has no answer.
    fn ndc(&self, point: [f64; 3]) -> Option<[f64; 3]> {
        let clip = self.clip(point);
        (clip[3] > 0.0).then(|| [clip[0] / clip[3], clip[1] / clip[3], clip[2] / clip[3]])
    }

    /// The world point at a normalised coordinate and a distance along the light's own axis.
    ///
    /// This is the grid read the other way: a page is a rectangle of the light's normalised plane,
    /// and its corners at two distances are the eight corners of the volume of world it covers.
    fn corner(&self, ndc: [f64; 2], distance: f64) -> [f64; 3] {
        let (right, up, back) = self.basis();
        let [hx, hy] = self.half_tangent();
        add(
            add(
                self.eye(),
                add(
                    scale(right, ndc[0] * hx * distance),
                    scale(up, ndc[1] * hy * distance),
                ),
            ),
            scale(back, -distance),
        )
    }
}

/// What a camera can see, as the eight corners of its frustum.
///
/// The corners are ordered by the bit their index carries: the low bit is which side of the
/// horizontal it is on, the next the vertical, and the top bit near or far. [`Frustum::planes`]
/// derives the six faces from that order, so a set of corners written in another order is a
/// frustum with the wrong faces rather than one that fails.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frustum {
    /// The eight corners, in world space.
    pub corners: [[f64; 3]; 8],
}

/// The three corners of each face, in the bit order [`Frustum`] documents.
const FACES: [[usize; 3]; 6] = [
    [0, 2, 4],
    [1, 3, 5],
    [0, 1, 4],
    [2, 3, 6],
    [0, 1, 2],
    [4, 5, 6],
];

/// A plane whose inside faces the frustum's own centre.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Plane {
    normal: [f64; 3],
    offset: f64,
}

impl Plane {
    /// Builds the plane through three points, keeping whichever of its two normals points away
    /// from the reference point.
    ///
    /// The reference is the frustum's centroid, which is inside every frustum this module builds
    /// because a frustum is convex and its corners are symmetric about it. A degenerate face —
    /// three collinear corners, which a frustum with no field of view has — gives a zero normal
    /// and a plane that separates nothing, which is the right answer for a volume with no
    /// interior: it keeps pages rather than dropping shadows.
    fn through(a: [f64; 3], b: [f64; 3], c: [f64; 3], inside: [f64; 3]) -> Self {
        let normal = normalize(cross(subtract(b, a), subtract(c, a)));
        let offset = dot(normal, a);
        if dot(normal, inside) - offset < 0.0 {
            return Self {
                normal: [-normal[0], -normal[1], -normal[2]],
                offset: -offset,
            };
        }
        Self { normal, offset }
    }

    /// How far inside a point is, which is negative when it is outside.
    fn depth_of(&self, point: [f64; 3]) -> f64 {
        dot(self.normal, point) - self.offset
    }
}

impl Frustum {
    /// Builds a frustum from its eight corners.
    pub fn new(corners: [[f64; 3]; 8]) -> Self {
        Self { corners }
    }

    /// The centroid, which is the point every face is turned away from.
    fn centre(&self) -> [f64; 3] {
        let mut sum = [0.0; 3];
        for corner in &self.corners {
            sum = add(sum, *corner);
        }
        scale(sum, 1.0 / 8.0)
    }

    /// The six planes, each facing inwards.
    fn planes(&self) -> [Plane; 6] {
        let centre = self.centre();
        FACES.map(|[a, b, c]| {
            Plane::through(self.corners[a], self.corners[b], self.corners[c], centre)
        })
    }
}

/// Every frustum the frame is made of, with its planes derived once rather than per page.
#[derive(Debug, Clone, PartialEq)]
struct Clip {
    frusta: Vec<Frustum>,
    planes: Vec<[Plane; 6]>,
}

impl Clip {
    fn new(frusta: Vec<Frustum>) -> Self {
        let planes = frusta.iter().map(Frustum::planes).collect();
        Self { frusta, planes }
    }

    /// Whether anything was given to clip against, which is the same as nothing being clipped
    /// away.
    fn is_empty(&self) -> bool {
        self.frusta.is_empty()
    }

    /// Reports whether one polyhedron's corners all lie outside one plane of the other, which for
    /// two convex volumes means the two have no point in common.
    fn separates(plane: &Plane, points: &[[f64; 3]]) -> bool {
        points.iter().all(|point| plane.depth_of(*point) < 0.0)
    }
}

/// One light's plan: what it is, and what it sees.
#[derive(Debug, Clone, PartialEq)]
pub struct LightPlan {
    /// The name to put in a message about this light.
    pub label: String,
    /// The light's own space.
    pub volume: LightVolume,
    /// How many faces the light has. A spot has one; a point will have six, and the table carries
    /// the axis so that a face stays an offset rather than becoming a redesign.
    pub faces: u32,
    /// The objects that may block it.
    pub casters: Vec<Caster>,
}

/// What the allocation is bounded by.
#[derive(Debug, Clone, PartialEq)]
pub struct Budget {
    /// The largest atlas layer side the machine can build, in pages. A page is [`PAGE_SIZE`]
    /// texels, so this is the texture side ceiling divided by one.
    pub atlas_side_pages: u32,
    /// What the camera can see. Empty means every page of every light is spendable, which is what
    /// a scene with no camera in it asks for.
    pub clip: Vec<Frustum>,
}

/// A page that exists: where it physically lives and what is drawn into it.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    /// Which light's plan this page belongs to, as its index in the input.
    pub light: usize,
    /// How coarse it is: level zero is the finest, and each step doubles the world a texel covers.
    pub level: u32,
    /// Which face of the light, which is zero for the kinds that have one.
    pub face: u32,
    /// Where in this level's grid it is, in pages.
    pub page_x: u32,
    /// The other direction.
    pub page_y: u32,
    /// Which slot of the light's atlas layer holds it, counted across every level of that light.
    pub slot: u32,
    /// The page's window in the light's normalised plane: minimum x, minimum y, and the two
    /// extents. A vertex shader maps a caster's clip output through it so that the page is filled
    /// with what its own corner of the light sees.
    pub window: [f64; 4],
    /// Every caster that draws into it, by the handle its caller gave it, in ascending order.
    pub casters: Vec<u32>,
}

/// Where one light's table rows live, which is the same three numbers for every row of it.
///
/// # Invariants
///
/// It carries the *shape* of a light's table section and nothing else, so it exists while the rows
/// are still being numbered: [`LightMap`] is what a finished allocation reports, and reading a
/// page's row out of the table needs an offset before there is a map to read it from.
#[derive(Debug, Clone, Copy)]
struct Grid {
    /// Where this light's section starts, in words of [`Allocation::table`].
    table_offset: usize,
    /// Pages per side of the finest grid.
    pages_per_side: u32,
    /// How many faces the light has.
    faces: u32,
}

impl Grid {
    /// The same section, read from a light's finished map.
    fn of(map: &LightMap) -> Self {
        Self {
            table_offset: map.table_offset,
            pages_per_side: map.pages_per_side,
            faces: map.faces,
        }
    }

    /// The word holding the first slot of one row.
    fn base(&self, level: u32, face: u32, row: u32) -> usize {
        self.table_offset + base_word(self.pages_per_side, level, self.faces, face, row) as usize
    }

    /// The first of the words holding one row's bitmask.
    fn mask(&self, level: u32, face: u32, row: u32) -> usize {
        self.table_offset + mask_word(self.pages_per_side, level, self.faces, face, row) as usize
    }
}

/// One light's grid, as its table header needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LightMap {
    /// Pages per side of the finest grid.
    pub pages_per_side: u32,
    /// How many levels of precision the grid carries.
    pub levels: u32,
    /// How many faces the light has.
    pub faces: u32,
    /// Where this light's table section starts, in words of [`Allocation::table`].
    pub table_offset: usize,
    /// How many slots of its atlas layer this light fills.
    pub slots: u32,
    /// The side of the grid of slots, in pages: the smallest that holds [`Self::slots`].
    pub atlas_side: u32,
}

/// What was decided: every page that exists, the table that maps them, and the atlas sized to
/// hold them.
#[derive(Debug, Clone, PartialEq)]
pub struct Allocation {
    /// One entry per light plan, in the order they were given.
    pub lights: Vec<LightMap>,
    /// Every page, ordered by light, then level, then row, then column. The order is the
    /// allocation's own rather than a courtesy: it is what makes one row's slots run
    /// consecutively, which is the whole reason a lookup is a popcount rather than a search.
    pub pages: Vec<Page>,
    /// The page table, as the words a shader reads. See [`Allocation::header`].
    pub table: Vec<u32>,
}

impl Allocation {
    /// Nothing allocated at all, which is what a scene with no shadowing light hands the emitter
    /// to skip.
    pub fn empty() -> Self {
        Self {
            lights: Vec::new(),
            pages: Vec::new(),
            table: Vec::new(),
        }
    }

    /// Reports whether no page exists, which is the only reason to leave a light out of the atlas.
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }

    /// The largest slot grid any one light needs, in pages.
    pub fn atlas_side_pages(&self) -> u32 {
        self.lights
            .iter()
            .map(|light| light.atlas_side)
            .max()
            .unwrap_or(1)
    }

    /// How many layers the atlas array has, which is one per light given a grid.
    pub fn atlas_layers(&self) -> usize {
        self.lights.len()
    }

    /// The atlas this allocation needs, or `None` when nothing was allocated and no texture has to
    /// be built at all.
    ///
    /// One array rather than one per level, because a page is the same number of texels whatever
    /// level it is on: the level says how much world those texels cover, and that is a fact about
    /// the window a pass draws through rather than about the texture it draws into.
    pub fn atlas(&self) -> Option<Atlas> {
        (!self.is_empty()).then(|| Atlas {
            side_pages: self.atlas_side_pages(),
            layers: self.atlas_layers() as u32,
        })
    }

    /// The header words of one light's table section, which is where a shader starts a walk.
    ///
    /// A light with no grid has no section, so this is `None` rather than a zeroed header: the
    /// answer for a light that was refused its grid is that its record says *no table*, and
    /// reading another light's rows out of a zero offset is the failure this avoids.
    pub fn header(&self, light: usize) -> Option<[u32; 2]> {
        let map = self.lights.get(light)?;
        Some([
            pack(map.pages_per_side, map.atlas_side),
            pack(map.levels, map.faces),
        ])
    }

    /// Writes one finished row's base slot into its table. The mask was set page by page as the
    /// row was walked, so this is the only half that needs the row to be over.
    fn flush_row(&mut self, grid: &Grid, row: (u32, u32, u32), first: u32) {
        let (level, face, row) = row;
        let at = grid.base(level, face, row);
        self.table[at] = first;
    }

    /// Which physical slot a mapped page of one light is, or `None` when the page has no slot.
    ///
    /// The row's base word says where the row starts and its mask says which columns of it exist,
    /// so the slot is the base plus how many set bits lie before this column. That is one test and
    /// one popcount per row, which is the whole of what a fragment does to find its shadow.
    fn slot_of(&self, grid: &Grid, level: u32, face: u32, row: u32, page_x: u32) -> Option<u32> {
        let pages = pages_at_level(grid.pages_per_side, level);
        if page_x >= pages || row >= pages {
            return None;
        }
        let masks = grid.mask(level, face, row);
        let bit = page_x % 32;
        let word = (page_x / 32) as usize;
        let bits = self.table[masks + word];
        if bits & (1 << bit) == 0 {
            return None;
        }
        let mut rank = (bits & ((1 << bit) - 1)).count_ones();
        for earlier in 0..word {
            rank += self.table[masks + earlier].count_ones();
        }
        Some(self.table[grid.base(level, face, row)] + rank)
    }

    /// Where a world point's shadow is read, mirroring the walk the table is laid out for.
    ///
    /// This is the shader's side of the encoding written out in Rust, and it exists so that the
    /// two ends of it are tested against each other rather than each against itself. The chain is
    /// walked from the finest level down and the first mapped page wins, which is what makes a
    /// caster drawn only at a coarse level still shade a receiver in a fine page's neighbourhood.
    pub fn resolve(&self, volume: &LightVolume, light: usize, point: [f64; 3]) -> Option<Lookup> {
        let map = self.lights.get(light)?;
        // A cube's face is the one thing this walk does not do yet, and the answer for a light with
        // more than one face would be the same slot whatever the direction, which is a wrong
        // shadow rather than a missing one. Only a spot light, which has one, is routed here.
        debug_assert_eq!(
            map.faces, 1,
            "a light with more than one face has no walk yet"
        );
        let grid = Grid::of(map);
        let ndc = volume.ndc(point)?;
        let side = map.pages_per_side * PAGE_SIZE;
        let texel = [
            (ndc[0] * 0.5 + 0.5) * f64::from(side),
            (0.5 - ndc[1] * 0.5) * f64::from(side),
        ];
        if !(-1.0..=1.0).contains(&ndc[0])
            || !(-1.0..=1.0).contains(&ndc[1])
            || !(0.0..f64::from(side)).contains(&texel[0])
            || !(0.0..f64::from(side)).contains(&texel[1])
        {
            return None;
        }
        for level in 0..map.levels {
            let shift = PAGE_SIZE << level;
            let page_x =
                (texel[0] as u32 / shift).min(pages_at_level(map.pages_per_side, level) - 1);
            let page_y =
                (texel[1] as u32 / shift).min(pages_at_level(map.pages_per_side, level) - 1);
            let Some(slot) = self.slot_of(&grid, level, 0, page_y, page_x) else {
                continue;
            };
            let in_page = [
                (texel[0] as u32 % shift) >> level,
                (texel[1] as u32 % shift) >> level,
            ];
            return Some(Lookup {
                level,
                slot,
                texel: [
                    (slot % map.atlas_side) * PAGE_SIZE + in_page[0],
                    (slot / map.atlas_side) * PAGE_SIZE + in_page[1],
                ],
                depth: ndc[2],
            });
        }
        None
    }
}

/// Where a point's shadow is read from, as [`Allocation::resolve`] answers it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lookup {
    /// The level the answer came from, which is what a bias has to be scaled by.
    pub level: u32,
    /// The atlas slot the page lives in.
    pub slot: u32,
    /// The texel of the atlas layer the point reads.
    pub texel: [u32; 2],
    /// The light's own depth of the point, in zero to one, which is what a recorded value is
    /// compared against.
    pub depth: f64,
}

/// The atlas one allocation needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Atlas {
    /// The side of the grid of slots, in pages.
    pub side_pages: u32,
    /// How many layers it has, which is one per light with a grid.
    pub layers: u32,
}

impl Atlas {
    /// The side of one layer, in texels.
    pub fn side_texels(&self) -> u32 {
        self.side_pages * PAGE_SIZE
    }
}

/// What could not be allocated, named by the light and the demand.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Shortfall {
    /// The grid a light's density and reach asked for is past the ceiling.
    #[error(
        "the light `{label}` asks for a shadow grid {wanted} pages per side and the ceiling is \
         {limit}. The density its casters asked for is finer than the light's own reach divided \
         by {limit} pages of {PAGE_SIZE} texels: lower a `shadow_density`, or shorten the light's \
         reach. Nothing here rounds the answer down, because a shadow recorded at a tenth of what \
         was asked for still renders, and reads as a choice rather than as a failure."
    )]
    Grid {
        label: String,
        wanted: u32,
        limit: u32,
    },
    /// The pages a light needed do not fit its atlas layer.
    #[error(
        "the light `{label}` needs {wanted} shadow pages at the density its casters asked for and \
         in the region the camera can see, and its atlas layer holds {limit}. The demand is pages \
         of {PAGE_SIZE} texels, so meeting it means a bigger atlas rather than a blurrier shadow."
    )]
    Atlas {
        label: String,
        wanted: u32,
        limit: u32,
    },
    /// A light that shadows has nothing to shadow with.
    #[error(
        "the light `{label}` casts shadows and has nothing to cast them with, so its grid has no \
         density to be sized by and no page to allocate. A light that shadows nothing should say \
         so rather than stand in the frame recording nothing."
    )]
    NoCasters { label: String },
}

/// How many pages one side of a level's grid has.
///
/// A level doubles the world a page covers, so its grid is half as many pages wide as the level
/// above it, and the last level is a grid of one page whatever the ceiling says.
pub fn pages_at_level(pages_per_side: u32, level: u32) -> u32 {
    (pages_per_side >> level).max(1)
}

/// How many levels one grid carries, which stops as soon as a further level would hold no more
/// pages than the level before it.
pub fn levels_for(pages_per_side: u32) -> u32 {
    let mut levels = 1_u32;
    while levels < MAX_LEVELS && (pages_per_side >> levels) >= 1 {
        levels += 1;
    }
    levels
}

/// Words one row of one level and face costs: the slot the row starts at, then one bit per page of
/// the row.
fn row_words(pages_per_side: u32, level: u32) -> u32 {
    1 + pages_at_level(pages_per_side, level).div_ceil(32)
}

/// Words one level's whole section costs, across every face it has.
fn level_words(pages_per_side: u32, level: u32, faces: u32) -> u32 {
    row_words(pages_per_side, level) * pages_at_level(pages_per_side, level) * faces.max(1)
}

/// How many words one light's whole table section costs.
///
/// Every row of every level is laid out whether or not anything lands in it, because a shader
/// finds a row by arithmetic on its page coordinate and cannot know in advance whether the row was
/// wanted. The cost is `pages_per_side² / 32` words per level, which at the ceiling of
/// [`MAX_PAGES_PER_SIDE`] is under twelve thousand words for all four levels together — the price
/// of a lookup that is a mask test and a popcount rather than a search.
pub fn table_words(pages_per_side: u32, levels: u32, faces: u32) -> usize {
    let mut words = (TABLE_HEAD_WORDS + PREFIX_WORDS) as usize;
    for level in 0..levels {
        words += level_words(pages_per_side, level, faces) as usize;
    }
    words
}

/// Where one level's section starts, relative to its light's own header.
fn level_section(pages_per_side: u32, level: u32, faces: u32) -> u32 {
    let mut at = TABLE_HEAD_WORDS + PREFIX_WORDS;
    for earlier in 0..level {
        at += level_words(pages_per_side, earlier, faces);
    }
    at
}

/// The word holding the first slot of one row.
pub fn base_word(pages_per_side: u32, level: u32, faces: u32, face: u32, row: u32) -> u32 {
    let words = row_words(pages_per_side, level);
    level_section(pages_per_side, level, faces)
        + face * words * pages_at_level(pages_per_side, level)
        + row * words
}

/// The first of the words holding one row's bitmask.
pub fn mask_word(pages_per_side: u32, level: u32, faces: u32, face: u32, row: u32) -> u32 {
    base_word(pages_per_side, level, faces, face, row) + 1
}

/// The level a caster of this density is recorded at, given the grid the light was given.
///
/// # The constraint, stated as a resolution and not as a ratio
///
/// A level-`k` texel is the light's own plane divided by `pages_per_side` pages of `PAGE_SIZE`
/// texels and then doubled `k` times, so at the far plane — the widest the plane ever is, and
/// therefore the depth at which a texel is coarsest — it is
///
/// ```text
/// world_units(k) = plane_width * 2^k / (pages_per_side * PAGE_SIZE)
/// ```
///
/// A caster asking for `density` texels a world unit asked for a texel of `1 / density`, and is
/// recorded no coarser than it asked for when `world_units(k) <= 1 / density`. The level is the
/// **largest** one that still holds, because that is the cheapest recording that keeps the
/// promise: a level finer than this one would hand the caster a precision it declined.
///
/// The density the constraint is measured against is the grid's own, not the density the light
/// was asked for. `grid_side_for` rounds a whole number of pages up to a power of two, so the
/// built level zero is *finer* than `1 / finest_density_asked` and a ratio measured against the
/// ask would keep a caster a level above the one its own declaration pays for.
///
/// A density of zero or less is the **coarsest** level the grid has rather than the finest one:
/// such a caster still shadows, and it must not be the reason a page is allocated finer than
/// anything asked for. It is a sentinel rather than a number, and it is the one case the
/// constraint cannot evaluate.
pub fn caster_level(density: f64, plane_width: f64, pages_per_side: u32, levels: u32) -> u32 {
    let levels = levels.max(1);
    if density <= 0.0 {
        return levels - 1;
    }
    // The grid's own density at level zero, in texels a world unit, read off the grid rather than
    // off the demand that sized it. Level `k` halves that `k` times, and the level wanted is the
    // last one whose density the caster's own would not have wanted to be finer than.
    let finest = f64::from(pages_per_side) * f64::from(PAGE_SIZE) / plane_width.max(1e-9);
    let mut level = 0_u32;
    while level + 1 < levels && f64::from(1_u32 << (level + 1)) <= finest / density {
        level += 1;
    }
    level
}

/// The grid side that makes a texel at the far plane no coarser than one density asks for, or the
/// side it would have needed.
///
/// # What this guarantees, and where the guarantee is measured
///
/// The grid is angular, so a texel's world size is not one number: it is a fraction of the light's
/// plane, and that plane is wider the further the light reaches. At the **far** plane the grid
/// spans `2 × far × tan(half angle)` world units, which is the widest it ever gets, so a texel
/// fine enough there is finer than asked everywhere nearer. Sizing at any other depth would
/// promise a density the back of the volume cannot keep.
///
/// This is why a grid is measured against the cone's own width rather than against the distance it
/// reaches: a narrow cone and a wide one at the same distance need different grids, and one sized
/// by distance alone gives the narrow cone pages it cannot fill and the wide one texels coarser
/// than they were asked for.
fn grid_side_for(plane_width: f64, max_density: f64) -> Result<u32, u32> {
    let wanted = plane_width.max(0.0) * max_density.max(0.0);
    // The side a texel of the finest grid has to have to keep the promised density, reported when
    // the doubling runs out, so the message states the demand rather than the next power of two
    // past it.
    let needed = (wanted / f64::from(PAGE_SIZE))
        .ceil()
        .clamp(1.0, f64::from(u32::MAX)) as u32;
    let mut side = MIN_PAGES_PER_SIDE;
    while f64::from(side) * f64::from(PAGE_SIZE) < wanted {
        if side >= MAX_PAGES_PER_SIDE {
            return Err(needed);
        }
        side *= 2;
    }
    Ok(side)
}

/// The rectangle of a caster's own shadow on the light's plane: minimum x, minimum y, and the two
/// extents.
///
/// # Why this is the silhouette rather than the projected box
///
/// A caster is given as a sphere, so the rays that touch it are a cone whose half angle is
/// `asin(radius / distance)` to the **nearest** surface rather than to the centre. Writing that
/// angle as a tangent and dividing by the plane's own tangent gives the normalised half extent
/// directly, which is exact for a sphere and conservative for whatever geometry its bounds stood
/// in for.
///
/// A sphere that reaches the eye, or stands behind it, has no silhouette on the plane. Both are
/// answered by the whole plane, which is the conservative reading: it costs a level of pages, and
/// the alternative is a caster that casts nothing.
fn footprint(volume: &LightVolume, caster: &Caster) -> [f64; 4] {
    let whole = [-1.0, -1.0, 2.0, 2.0];
    let clip = volume.clip(caster.position);
    if clip[3] <= 0.0 {
        return whole;
    }
    let [hx, hy] = volume.half_tangent();
    let distance = clip[3];
    let nearest = (distance - caster.radius).max(distance * 1e-3);
    let ratio = (caster.radius / nearest).min(0.9999);
    let tangent = ratio / (1.0 - ratio * ratio).sqrt();
    let half = [tangent / hx, tangent / hy];
    if half[0] >= 1.0 && half[1] >= 1.0 {
        return whole;
    }
    let centre = [clip[0] / clip[3], clip[1] / clip[3]];
    [
        centre[0] - half[0],
        centre[1] - half[1],
        half[0] * 2.0,
        half[1] * 2.0,
    ]
}

/// The window of one page, in the light's normalised plane.
fn window_of(pages_per_side: u32, level: u32, page_x: u32, page_y: u32) -> [f64; 4] {
    let side = f64::from(pages_per_side * PAGE_SIZE);
    let shift = f64::from(PAGE_SIZE << level);
    let left = f64::from(page_x) * shift;
    // The vertical axis of a texture runs the other way to the plane it is laid over, which is why
    // the window's minimum y is read off the page's *far* row.
    let far_row = (f64::from(page_y) + 1.0) * shift;
    let extent = shift * 2.0 / side;
    [
        left / side * 2.0 - 1.0,
        1.0 - far_row / side * 2.0,
        extent,
        extent,
    ]
}

/// The page range a footprint covers at one level, as an inclusive start and an exclusive end per
/// axis.
///
/// # Invariants
///
/// **A footprint that lies wholly outside the light's plane covers nothing**, which is the whole
/// point of intersecting before converting rather than clamping after: a caster standing to one
/// side of a narrow cone projects to a rectangle at normalised x of, say, nine, and clamping that
/// into the plane allocates the corner page of every level for a shadow the cone cannot cast. One
/// such caster per light is harmless; one per object in a scene with a dozen lamps is the atlas.
///
/// The one page of margin a silhouette's bounding sphere is padded by is asked **in the plane**, in
/// units of the level being covered, so that the margin is one page at every level rather than one
/// fine page that a coarse level rounds away from nothing.
fn cover(pages_per_side: u32, level: u32, rect: &[f64; 4]) -> (u32, u32, u32, u32) {
    let side = f64::from(pages_per_side * PAGE_SIZE);
    let shift = PAGE_SIZE << level;
    let pages = pages_at_level(pages_per_side, level);
    let margin = f64::from(shift) * 2.0 / side;
    let left = (rect[0] - margin).clamp(-1.0, 1.0);
    let right = (rect[0] + rect[2] + margin).clamp(-1.0, 1.0);
    let bottom = (rect[1] - margin).clamp(-1.0, 1.0);
    let top = (rect[1] + rect[3] + margin).clamp(-1.0, 1.0);
    if right <= left || top <= bottom {
        return (0, 0, 0, 0);
    }
    let column = |value: f64| {
        let texel = ((value * 0.5 + 0.5) * side).clamp(0.0, side - 1.0) as u32;
        texel / shift
    };
    let row = |value: f64| {
        let texel = ((0.5 - value * 0.5) * side).clamp(0.0, side - 1.0) as u32;
        texel / shift
    };
    // The vertical minimum of the rectangle is the *last* row of the grid, so the ends are taken
    // the other way round.
    (
        column(left).min(pages - 1),
        (column(right) + 1).min(pages),
        row(top).min(pages - 1),
        (row(bottom) + 1).min(pages),
    )
}

/// Reports whether a page's volume of world can touch anything a camera can see.
///
/// The test is tried in both directions: no plane of any camera frustum may have the whole page
/// outside it, and no plane of the page's own volume may have every camera corner outside it.
/// Either answer, for two convex volumes, means no point in common, and a page with no point in
/// common with anything visible shades nothing.
///
/// The page's six planes are its four window edges, which are planes through the light's eye, and
/// its near and far, which every page of a light shares. Each is evaluated as a linear form on the
/// camera's corner in the **light's view space** rather than by building the plane, because a side
/// plane through the eye is the same statement as an inequality on a normalised coordinate written
/// without the divide that a corner behind the light would make meaningless: `ndc_x` is
/// `point.x / (tangent × depth)`, and clearing the divide turns the comparison into
/// `point.x + limit × tangent × point.z ≥ 0`, which is a plane through the light's own eye and
/// holds for a corner behind it too.
fn kept(volume: &LightVolume, rect: &[f64; 4], clip: &Clip) -> bool {
    if clip.is_empty() {
        return true;
    }
    let (near, far) = volume.near_far();
    let [hx, hy] = volume.half_tangent();
    let [a, b, c, d] = [rect[0], rect[0] + rect[2], rect[1], rect[1] + rect[3]];
    let mut corners = Vec::with_capacity(8);
    for x in [rect[0], rect[0] + rect[2]] {
        for y in [rect[1], rect[1] + rect[3]] {
            for distance in [near, far] {
                corners.push(volume.corner([x, y], distance));
            }
        }
    }
    // One row per face of the page, holding the coefficients of the form that is at least zero on
    // the side the page is on. The four edges have no constant term because their planes run
    // through the eye, which is the view space origin; the two depth faces are the ones every page
    // of the light shares, and they are the only two that shift.
    let forms: [(f64, f64, f64, f64); 6] = [
        (1.0, 0.0, a * hx, 0.0),
        (-1.0, 0.0, -b * hx, 0.0),
        (0.0, 1.0, c * hy, 0.0),
        (0.0, -1.0, -d * hy, 0.0),
        (0.0, 0.0, -1.0, -near),
        (0.0, 0.0, 1.0, far),
    ];
    clip.frusta
        .iter()
        .zip(&clip.planes)
        .all(|(frustum, planes)| {
            let views: Vec<[f64; 3]> = frustum
                .corners
                .iter()
                .map(|corner| volume.view_of(*corner))
                .collect();
            let any_inside = |form: &(f64, f64, f64, f64)| {
                views
                    .iter()
                    .any(|p| form.0 * p[0] + form.1 * p[1] + form.2 * p[2] + form.3 >= 0.0)
            };
            !planes.iter().any(|plane| Clip::separates(plane, &corners))
                && forms.iter().all(any_inside)
        })
}

/// Decides which pages exist.
///
/// Each light is sized from the density its own casters asked for, has each of them placed on the
/// level their density buys, covers the pages their projections span at every level from theirs
/// down to the coarsest, and drops the pages no camera can see. The rows that survive are numbered
/// into the light's atlas slots in the order a shader walks them, and every word of the table is
/// written from that numbering rather than from a second count of anything.
pub fn allocate(plans: &[LightPlan], budget: &Budget) -> Result<Allocation, Shortfall> {
    let clip = Clip::new(budget.clip.clone());
    let mut out = Allocation::empty();
    for (index, plan) in plans.iter().enumerate() {
        let faces = plan.faces.max(1);
        let live: Vec<&Caster> = plan
            .casters
            .iter()
            .filter(|caster| caster.radius > 0.0)
            .collect();
        if live.is_empty() {
            return Err(Shortfall::NoCasters {
                label: plan.label.clone(),
            });
        }
        let max_density = live
            .iter()
            .map(|caster| caster.density)
            .fold(0.0_f64, f64::max);
        let far = plan.volume.near_far().1;
        let width = 2.0 * far * plan.volume.half_tangent()[0];
        let pages_per_side =
            grid_side_for(width, max_density).map_err(|wanted| Shortfall::Grid {
                label: plan.label.clone(),
                wanted,
                limit: MAX_PAGES_PER_SIDE,
            })?;
        let levels = levels_for(pages_per_side);

        let mut wanted: HashMap<(u32, u32, u32, u32), Vec<u32>> = HashMap::new();
        for caster in &live {
            let own = caster_level(caster.density, width, pages_per_side, levels);
            let rect = footprint(&plan.volume, caster);
            for level in own..levels {
                let (x0, x1, y0, y1) = cover(pages_per_side, level, &rect);
                for page_y in y0..y1 {
                    for page_x in x0..x1 {
                        wanted
                            .entry((level, 0, page_y, page_x))
                            .or_default()
                            .push(caster.handle);
                    }
                }
            }
        }
        let mut wanted: Vec<(u32, u32, u32, u32, Vec<u32>)> = wanted
            .into_iter()
            .map(|((level, face, page_y, page_x), casters)| (level, face, page_y, page_x, casters))
            .collect();
        wanted.sort_by_key(|(level, face, page_y, page_x, _)| (*level, *face, *page_y, *page_x));
        wanted.retain(|(level, _, page_y, page_x, _)| {
            kept(
                &plan.volume,
                &window_of(pages_per_side, *level, *page_x, *page_y),
                &clip,
            )
        });

        let slots = wanted.len() as u32;
        let atlas_side = ceil_root(slots);
        let limit = budget.atlas_side_pages.max(1);
        if slots > limit * limit {
            return Err(Shortfall::Atlas {
                label: plan.label.clone(),
                wanted: slots,
                limit: limit * limit,
            });
        }

        let table_offset = out.table.len();
        let grid = Grid {
            table_offset,
            pages_per_side,
            faces,
        };
        out.table
            .resize(table_offset + table_words(pages_per_side, levels, faces), 0);
        out.table[table_offset] = pack(pages_per_side, atlas_side);
        out.table[table_offset + 1] = pack(levels, faces);
        for level in 0..levels {
            out.table[table_offset + (TABLE_HEAD_WORDS + level) as usize] =
                level_section(pages_per_side, level, faces);
        }

        let mut row: Option<(u32, u32, u32)> = None;
        let mut row_first = 0_u32;
        // The slot is the page's position in the order they were sorted into, which is what makes
        // one row's slots run consecutively and a lookup a popcount rather than a search.
        for (number, (level, face, page_y, page_x, mut casters)) in wanted.into_iter().enumerate() {
            let slot = number as u32;
            if row != Some((level, face, page_y)) {
                if let Some(done) = row {
                    out.flush_row(&grid, done, row_first);
                }
                row = Some((level, face, page_y));
                row_first = slot;
            }
            casters.sort();
            casters.dedup();
            out.pages.push(Page {
                light: index,
                level,
                face,
                page_x,
                page_y,
                slot,
                window: window_of(pages_per_side, level, page_x, page_y),
                casters,
            });
            out.table[grid.mask(level, face, page_y) + (page_x / 32) as usize] |=
                1 << (page_x % 32);
        }
        if let Some(done) = row {
            out.flush_row(&grid, done, row_first);
        }

        out.lights.push(LightMap {
            pages_per_side,
            levels,
            faces,
            table_offset,
            slots,
            atlas_side,
        });
    }
    Ok(out)
}

/// The smallest integer whose square is at least `count`, and at least one.
fn ceil_root(count: u32) -> u32 {
    let mut side = 1_u32;
    while side.saturating_mul(side) < count {
        side += 1;
    }
    side
}
