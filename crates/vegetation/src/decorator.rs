//! Small authored things seated on a node's surface: a knot, a burl, a thorn, a fruit, a scar.
//!
//! # The one idea
//!
//! **A decorator is a template and three numbers, and no rotation is expressible.** Its template is a small
//! [`shape::Sweep`] authored in its own frame — base at the origin, axis along `+x` — and its seat is the same
//! `(station, slot, sink)` a joint is made of, so the frame it is drawn in is **the node's own surface frame taken
//! whole**: `x` outward, `y` along the parent, `z` their cross product. There is no angle to write down, which is
//! V3's sentence (*"a joint is one subtraction, not a transform"*) made a type: a rotation that cannot be
//! expressed cannot be got wrong.
//!
//! # Why a decorator's template is a sweep
//!
//! The same reason a leaf's is: a knot is a short tube that bulges, a thorn is a thin cone, a burl is a lumpy
//! stub — all of them a spine, a law that thickens it and a section. So a decorator template and a leaf template
//! are the same kind of object, and a scene that can draw one can draw the other: **`k` decorators of one template
//! are one mesh and `k` transforms**, exactly as `k` leaves are.
//!
//! # A template may be a sprig
//!
//! **One template is one instanced mesh, and a mesh may hold several bands** — the same sentence
//! [`crate::leaf::LeafTemplate`] states for blades, and the reason a five-petalled corolla otherwise has to be
//! authored as a ten-pointed star *section* and drawn as one cone. [`Template::sprig`] joins parts the way the
//! leaf's own sprig does, so the flower's petals are petals again: five small sweeps, each in its own frame
//! inside the template's. The parts ride inside the template's [`Sweep`] — [`shape::Part`]s beside its own band —
//! so [`Template::loft`] stays the single entry a scene calls and a template's `stations` and `around` still
//! name the **first** part's grid, which is the part a one-part template has always been.
//!
//! # Why a scatter may read a band
//!
//! [`scatter`] seats on every live node of one generation at one station *of that node*, so a species can say
//! "a cone at every twig tip" and nothing more. But a fir's cones are **erect and stand on the upper crown
//! only** — [`abies_delavayi.md`](../../../../.agents/investigate/trees/abies_delavayi.md): *"ERECT and
//! upright, standing on the upper crown"* — and a kapok flowers **at every tip of a bare crown** —
//! [`bombax_ceiba.md`](../../../../.agents/investigate/trees/bombax_ceiba.md): *"solitary, terminal or in the
//! leaf axils of branch tips"*. Both are statements about **where on its parent a node hangs**, which is the
//! one number a seat's own station is not. [`scatter_in`] is that number: a band of the parent's arc length,
//! and the same rule with the band `(0.0, 1.0)` is [`scatter`] itself.
//!
//! # What is not a decorator
//!
//! A tip cap. An open tube is a hole seen from below, and [`shape::Sweep::capped`] closes it with the sweep's own
//! end ring — a decorator would be a second, slower way to say the same thing, and one that could be left off.

use shape::math::clamp;
use shape::{Loft, Part, Sweep};

use crate::handle::Handle;
use crate::node::Attachment;
use crate::plant::{Placement, Plant};
use crate::rng::Rng;

/// A decorator's template: a small sweep in its own frame, and the dial it is drawn at.
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    /// The sweep, authored with its base at the origin and its axis along `+x`, which is outward from the surface
    /// it will sit on. A sprig's parts ride in `sweep.parts`, so this field is still the template's own first
    /// band and a one-part template is still exactly the sweep it was.
    pub sweep: Sweep,
    /// How many stations the template is sampled at: the first part's, and the whole of a one-part template's.
    pub stations: usize,
    /// How many slots around its section.
    pub around: usize,
}

impl Template {
    /// The template as the geometry a scene draws once and instances.
    ///
    /// # Invariants
    ///
    /// The loft is in the template's own frame, which is the space an instance's frame expects: a point at the
    /// template's origin lands at the seat's origin, and the template's `+x` lands along the surface's outward
    /// direction. A template that carries parts lofts the **join** of its own band and every part's, which is
    /// [`shape::Loft::joined`] and not a grid — a consumer reads its positions and triangles and never asks
    /// for a ring.
    pub fn loft(&self) -> Loft {
        self.sweep.sample(self.stations, self.around)
    }

    /// Several decorators in one template: a corolla, a whorl of cone scales, a bud — whatever a crown
    /// instances as one unit. Each part keeps its own frame inside the template's, so a sprig is authored as
    /// decorators placed where they grow and instanced as one mesh.
    ///
    /// # Invariants
    ///
    /// - **The first part is the template's own band and stands at the template's own frame**; the rest are
    ///   placed inside it. That is what keeps `stations` and `around` meaningful — they are the first part's
    ///   grid, which for a one-part template is the whole template — and it is the authoring a leaf's own
    ///   sprig already uses, where a palmate hand is a petiole at the origin and leaflets placed inside it.
    ///   A first part given any other frame is refused rather than quietly dropped, because the alternative
    ///   would be a template whose first part is drawn in a place its author did not ask for.
    /// - **The parts' frames compose the way placements do**: a part that is itself a sprig keeps both its
    ///   inner frames and the frame it is placed in, because a part carried inside a sprig is composed as
    ///   `at.then(&part.at)`. A part's own parts are carried with it rather than left behind, so a sprig of
    ///   sprigs draws each band once.
    /// - **A one-part sprig at the identity is the template it was made from**, vertex for vertex: the
    ///   template's own band is sampled exactly as before and nothing is joined to it.
    /// - **An empty list is refused**, because a template with no part has no mesh and [`Template::loft`]
    ///   would have nothing to sample.
    pub fn sprig(parts: Vec<(Template, Placement)>) -> Self {
        let mut carried: Vec<Part> = Vec::new();
        let mut head: Option<Template> = None;
        for (template, at) in parts {
            match head {
                None => {
                    assert!(
                        at == Placement::IDENTITY,
                        "a sprig's first part is the template's own band, so it stands at the template's frame"
                    );
                    head = Some(template);
                }
                Some(_) => carried.extend(template.carried(&at)),
            }
        }
        let mut head = head.expect("a sprig is authored with at least one part");
        // The head's own parts are already in its own coordinates and come first; every other part was
        // placed inside it as it was read.
        let own = std::mem::take(&mut head.sweep.parts);
        head.sweep.parts = [own, carried].concat();
        head
    }

    /// How many bands the template's loft draws: its own, plus one for every part it carries.
    pub fn parts(&self) -> usize {
        1 + self.sweep.parts.len()
    }

    /// This template's bands as a larger sprig carries them: its own band first at `at`, then each part it
    /// already carries with the two frames composed.
    ///
    /// A part is carried as a band of its own — its sweep stripped of its parts, which travel beside it — so
    /// a template that is itself a sprig contributes every one of its bands exactly once.
    fn carried(&self, at: &Placement) -> Vec<Part> {
        let Template {
            sweep,
            stations,
            around,
        } = self.clone();
        let mut bands = Vec::with_capacity(1 + sweep.parts.len());
        bands.push(Part {
            sweep: Sweep {
                parts: Vec::new(),
                ..sweep.clone()
            },
            stations,
            around,
            origin: at.origin,
            x: at.x,
            y: at.y,
            z: at.z,
        });
        for part in sweep.parts {
            let frame = at.then(&placement_of(&part));
            bands.push(Part {
                sweep: part.sweep,
                stations: part.stations,
                around: part.around,
                origin: frame.origin,
                x: frame.x,
                y: frame.y,
                z: frame.z,
            });
        }
        bands
    }
}

/// The frame a carried band stands in, read back as the placement it was placed by.
fn placement_of(part: &Part) -> Placement {
    Placement {
        origin: part.origin,
        x: part.x,
        y: part.y,
        z: part.z,
    }
}

/// One decorator: which template, which node, and the three numbers that seat it.
///
/// The numbers are [`Attachment`]'s own — a station along the node, a slot around it, and how far the seat sinks
/// into the surface — so a decorator is placed by the same vocabulary a branch is, and a reader who knows one
/// knows the other.
#[derive(Debug, Clone, PartialEq)]
pub struct Seat {
    /// The node whose surface the decorator sits on.
    pub node: Handle,
    /// Which template it instances.
    pub template: usize,
    /// Where along the node.
    pub station: f32,
    /// Where around the node's section.
    pub slot: f32,
    /// How far the seat sinks into the surface, so a knot is a knot in the bark rather than a bead on it.
    pub sink: f32,
    /// One uniform number multiplying the template's own size.
    pub scale: f32,
}

impl Seat {
    /// A decorator at a place on a node, with no sink and a scale of one.
    pub fn new(node: Handle, template: usize, station: f32, slot: f32) -> Self {
        Self {
            node,
            template,
            station,
            slot,
            sink: 0.0,
            scale: 1.0,
        }
    }

    /// The same seat sunk into its surface.
    pub fn sunk(mut self, sink: f32) -> Self {
        self.sink = sink;
        self
    }

    /// The same seat drawn larger or smaller.
    pub fn scaled(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// The attachment these numbers are.
    pub fn attachment(&self) -> Attachment {
        Attachment::new(self.station, self.slot).sunk(self.sink)
    }
}

/// A decorator as a scene draws it: where the template's own frame sits in the world, and how big it is.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    /// The node it sits on, which is the name a report gives it.
    pub node: Handle,
    /// Which template's mesh it instances.
    pub template: usize,
    /// The frame the template is drawn in.
    pub placement: Placement,
    /// The template's own size, multiplied by this.
    pub scale: f32,
}

/// Places every seat a plant can carry.
///
/// # Invariants
///
/// - **The frame is the node's own surface frame, moved into the world**: `plant.placement(node)` composed with
///   `plant.surface(node, seat.attachment())`, so a decorator sits on the skin the plant actually draws — nudged,
///   baked or bent by a field — and two numbers are the whole of its orientation.
/// - **A seat on a tombstone draws nothing**: the handle still resolves, so this is the one place a decorator can
///   be absent without an error, and it is the same rule the wood follows (`Plant::live`).
/// - **A seat whose template a caller did not supply is still placed**: the template number is the caller's to
///   resolve, and this function does not know how many they have. Resolving it is the scene's job, and a number
///   that names nothing is a scene that will not draw.
pub fn placed(plant: &Plant, seats: &[Seat]) -> Vec<Placed> {
    let mut placed = Vec::new();
    for seat in seats {
        let Ok(node) = plant.get(&seat.node) else {
            continue;
        };
        if node.pruned().is_some() {
            continue;
        }
        let Ok(frame) = plant.surface(&seat.node, seat.attachment()) else {
            continue;
        };
        let Ok(placement) = plant.placement(&seat.node) else {
            continue;
        };
        placed.push(Placed {
            node: seat.node.clone(),
            template: seat.template,
            placement: placement.then(&frame),
            scale: seat.scale,
        });
    }
    placed
}

/// The index a scatter's stream hangs off a node's path, so that a seat's numbers are a sibling of the
/// node's leaves' and never a draw in their stream.
///
/// # Invariants
///
/// No node's child ever carries this index: a child's index is its rank among its siblings, which is a
/// small number, and [`crate::rng::Rng::for_path`] folds the path into the seed one index at a time — so
/// appending it gives a stream that is the node's own stream's *sibling*, derived from the same seed and
/// path, and drawing from it moves no leaf and no branch.
const SCATTER_INDEX: u32 = u32::MAX;

/// A seat for every live node of one generation, at one station along it.
///
/// This is the rule that makes a cone a cone on *every* branch tip rather than a cone written out forty
/// times: a species names the generation that fruits, the template the fruit is, and where along the
/// branch it hangs, and the seed decides which branches fruit and where around them.
///
/// It is [`scatter_in`] with the whole node as its band, and it is written as that call rather than as a
/// second copy: a species that asks for the whole node cannot get a different seat than it got before,
/// because there is no second body to drift from the first.
///
/// # Invariants
///
/// - **The station is one number for the whole generation**, because a cone is not a law over a branch's
///   length: `1.0` is a cone at a branch's tip and a truss at a shoot's apex, which is where both actually
///   hang, and `0.0` is a knot where a limb leaves its parent.
/// - **The count and the slot are drawn from the node's own scatter stream, in that order**, one pair per
///   node, so two plants of one rule differ in which branches fruit and a change to one node's seed moves
///   no other node's seats and none of its leaves.
/// - **A count of `(0, 0)` fruits nowhere**, which is how a species says its cones are rare without saying
///   which branches are lucky, and a fixed count `(n, n)` draws nothing from the stream at all — the same
///   rule [`crate::species::grow`] states for a generation's children.
pub fn scatter(
    plant: &Plant,
    generation: usize,
    template: usize,
    station: f32,
    count: (u32, u32),
) -> Vec<Seat> {
    scatter_in(plant, generation, template, station, count, (0.0, 1.0))
}

/// A seat for every live node of one generation **whose own attachment lies in `band`**, at one station
/// along it.
///
/// This is [`scatter`] with the one thing a seat cannot say: where on its parent the node it sits on hangs.
/// A fir's cones stand on the upper crown and nowhere else, and a kapok flowers at the tip of every twig of
/// a bare crown — the first is a statement about the trunk, the second about the twigs, and neither is a
/// statement about a station along a branch. So the band is the band's own two numbers: the fraction of the
/// **parent's** arc length between which the asked generation hangs, read on each node's own
/// [`Attachment::station`]. A node's children hang at their own attachment, which is the number the band
/// reads, so asking for the twigs in `(0.0, 1.0)` is asking for all of them and asking for the boughs in
/// `(0.6, 1.0)` is asking for the crown's own limbs.
///
/// # Invariants
///
/// - **The band is read on the node's own attachment as it is drawn** — the authored station plus its
///   [`crate::node::Nudges`] — because that is the number [`crate::plant::Plant::placement`] hangs the node
///   from: an edit that slides a bough up the trunk carries its cones with it, as it carries its leaves.
/// - **The root is inside every band that reaches the foot of the plant, and outside every other.** The root
///   has no parent, so its attachment is [`Attachment::root`]: station nought, which is where the plant
///   begins. The root therefore answers to the band the way a node hanging at its parent's foot answers to
///   it — `(0.0, 1.0)` keeps it, `(0.5, 1.0)` does not — and a band that claims the upper half of a plant
///   cannot keep a seat on the node that *is* the plant's bottom. Reading it any other way would make
///   [`scatter`] and `scatter_in`'s full band disagree, and that pair is required to be the same seats.
/// - **A node outside the band draws nothing at all**: the band's test comes before the node's stream is
///   opened, so narrowing a band cannot move a kept node's count or its slots, and cannot move any other
///   node's stream, which is that node's own path plus [`SCATTER_INDEX`].
/// - **The count and the slot are drawn in [`scatter`]'s order, from that same stream**, one pair per node,
///   so the seats of the nodes the band keeps are the seats [`scatter`] drew for them with the full band.
/// - **The band's two ends are clamped into `[0, 1]` and ordered**, the rule a leaf field's own band
///   follows: a band authored past the end of the plant is the band it names, and an inverted one is the
///   same band read the other way round rather than an empty one.
pub fn scatter_in(
    plant: &Plant,
    generation: usize,
    template: usize,
    station: f32,
    count: (u32, u32),
    band: (f32, f32),
) -> Vec<Seat> {
    let (low, high) = band_of(band);
    let mut seats = Vec::new();
    for (handle, node) in plant
        .live()
        .filter(|(handle, _)| handle.depth() == generation)
    {
        // **The band is the node's own attachment on its own parent**: where it hangs, not where a seat on
        // it would hang. The root's attachment is the zero of the type, so a band that reaches the foot keeps
        // it and a band of the upper crown does not.
        let attached = node.effective_attachment().station;
        if attached < low || attached > high {
            continue;
        }
        let mut path = handle.path().to_vec();
        path.push(SCATTER_INDEX);
        let mut rng = Rng::for_path(plant.seed(), &path);
        let drawn = if count.0 == count.1 {
            count.0
        } else {
            let drawn = rng.range(count.0 as f32, (count.1 + 1) as f32);
            (drawn as u32).min(count.1)
        };
        for _ in 0..drawn {
            let slot = rng.unit();
            seats.push(Seat::new(handle.clone(), template, station, slot));
        }
    }
    seats
}

/// The band's two ends, clamped into `[0, 1]` and put in order.
fn band_of(band: (f32, f32)) -> (f32, f32) {
    let low = clamp(band.0, 0.0, 1.0);
    let high = clamp(band.1, 0.0, 1.0);
    if low <= high {
        (low, high)
    } else {
        (high, low)
    }
}
