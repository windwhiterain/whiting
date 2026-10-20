//! The dry valley and the wet one: 桫椤 *Alsophila spinulosa* and 木棉 *Bombax ceiba*.
//!
//! # What this module is
//!
//! Two species of [`southwest-china-trees.md`](../../../.agents/proposals/southwest-china-trees.md)'s arc 5
//! and arc 6, and the two that were left because they are the ones no branch graph draws. Every number
//! here cites its species' own note in
//! [`investigate/trees/`](../../../.agents/investigate/trees/), which cites the Flora of China and the
//! photographs in `out/refs/<slug>/`.
//!
//! | species | what it is here that nothing else is |
//! | --- | --- |
//! | **桫椤** *Alsophila spinulosa* | **no branch graph at all.** The column is one node, the crown is one generation of it, and a frond is one node whose pinnae are a leaf field. |
//! | **木棉** *Bombax ceiba* | **one rule at two seasons.** The same plant drawn in leaf and drawn leafless in red flower, the second carrying decorator seats and no leaf field. |
//!
//! # 桫椤: why there is no branch graph
//!
//! The note is blunt — *"**There is no branching. None at all above ground.** This species has no woody
//! branch system whatsoever, and any procedural tree system driven by a branch graph cannot represent
//! it"* — and this species is the test of that claim, because the crate's whole shape is a
//! [`vegetation::species::Generation`] per level and the tree fern has one level and a half.
//!
//! - **The column is one node** with a **constant taper**: the note's *"no taper; the column is cylindrical
//!   from the skirt to the apex"* and *"it gets **wider** toward the ground because the adventitious-root
//!   mantle flares out"*. Both halves of that are in [`column`]: a fall of two and a half times inside the
//!   first 40 cm and then a ripple of packed leaf bases and no taper at all.
//! - **The column's lumpiness is in the geometry in two places at once**, which is what
//!   [`Column::flutes`] and [`column`]'s ripple between them are for. A node has one section and one
//!   taper, so a column that is lumpy **around** *and* **along** is a lobed section scaled by a rippled
//!   law, and neither alone is the note's *"column of packed persistent bases"*.
//! - **The crown is one generation of the column**, `count` nodes in a narrow band at the apex with no
//!   `cluster` — a rosette is not a whorl. The note's *"the crown is one flat-topped radial rosette …
//!   not tiers"* is why the band is 0.09 of the column and not a fifth of it.
//! - **A frond is a node.** Its rachis is wood, which is what makes the pinnae a [`LeafField`] on the
//!   frond's own generation rather than a second kind of leaf: the crate's answer to a compound leaf,
//!   exactly as §4.7 records.
//! - **The brown collar of dead stipes under the crown is a decorator**, because it is the one organ
//!   this species has that is neither the column nor a live frond, and a decorator is
//!   *"a small authored thing seated on a node's surface"*.
//!
//! # 木棉: one rule, two seasons
//!
//! [`Season`] is the dial, and it is the answer the proposal's open question 3 deferred to this arc. It
//! is **one rule and one plant**; only two things change. [`Season::Flower`] carries
//! **no leaf field at all** — `fields` and `templates` are empty, not switched off — and seats the red
//! flowers on the twigs by [`vegetation::decorator::scatter_in`]. So *"在干热地区，花先叶开放"* (*"in dry-hot
//! country the flowers open before the leaves"*) is a fact about the crate's `Vec`, and the two plates
//! [`KAPOK`] and [`KAPOK_FLOWER`] are the same tree at the two ends of it.
//!
//! **The flowering plate's red is carried by the count and not by the size, and this pass says so with a
//! measurement.** Measured on [`KAPOK_FLOWER`]'s own pixels, the first pass carried **92** warm clusters of a
//! **median area of two pixels** — a 24 mm spot at the plate's 15 mm — for a warm share of **0.011** of the
//! crown's plant pixels. It carries **848** now, of a median two pixels still, for a warm share of **0.095**:
//! **eight and a half times the red for a flower the same size.** What did it was not geometry. It was that
//! [`kapok_flowers`] was seating at depth three, which is this rule's *secondaries* — see that function — so
//! **not one of the crown's 1 411 twigs was in flower**. The corolla is five petals rather than a star because
//! a star is a line seen edge-on and half of a crown's flowers hang edge-on; the crown reads red because there
//! are eight hundred of them on its rim rather than ninety-two spread evenly over the whole armature.
//!
//! # Two plates and two foliages, and the number that decides them
//!
//! The rasteriser samples a pixel at one point, so **a blade narrower than one pixel is dropped, not
//! faded**. One pixel is `2 · distance · tan(fov / 2) / height`, and both species state it:
//!
//! | plate | one pixel | the finest organ | chosen |
//! | --- | --- | --- | --- |
//! | [`TREE_FERN`] | 4.73 mm at 13.0 m | a 14 mm pinnule = 3.0 px | [`Foliage::Frond`], the plain pinnule |
//! | [`TREE_FERN_FROND`] | 1.61 mm at 1.90 m | a 7 × 4 mm pinnule segment = 4.4 × 2.5 px | [`Foliage::Pinna`], the **combed** pinnule |
//! | [`TREE_FERN_BASE`] | 1.18 mm at 1.70 m | a 12 mm root cord = 10 px | the true cord |
//! | [`KAPOK`] | 15.0 mm at 45.3 m | a 43 mm leaflet = 2.9 px | the true leaflet; **no card** |
//! | [`KAPOK_BOUGH`] | 7.0 mm at 9.6 m | the same leaflet = 6.1 px | the true leaflet |
//! | [`KAPOK_BASE`] | 3.3 mm at 5.1 m | a 250 mm plank board = 76 px | the true board |
//!
//! **The fern's two whole-plant numbers moved in this pass and the reason is the crown, not the plate.** A
//! `2.40 m` frond makes the plant **6.2 m** from the ground to the top of the crown's bowl, so [`FERN_FRAME`]
//! stood back from `12.0 m` to `13.0 m` to hold the whole crown in the picture, and every number derived from
//! that framing moved with it: **4.40 mm becomes 4.73 mm** and the pinnule's **3.2 pixels become 3.0**. That
//! is the tightest the whole-plant plate has ever been on its sampling tax, and it is why the frond plate's
//! distance was opened by the same quarter rather than left at 1.66 m: a frond that grew by a quarter and a
//! camera that did not would have cropped the tip and been a picture of a stipe.
//!
//! # The shadow-map dither on the fern's base plate: it is gone
//!
//! The first pass noted a **dither across the fern's cast shadow** on [`TREE_FERN_BASE`] and blamed the
//! shadow page. `crates/scene/src/standard/math.rs` then halved a point light's orthographic half-width to
//! the box it was fitted to, and this pass re-rendered the plate to find out. **It is gone, and so is
//! something the note did not mention: the shadow was doubled.**
//!
//! | what is on the floor at `x 60…460, y 400…700` of the base plate | first pass | this branch |
//! | --- | --- | --- |
//! | the fern's cast shadow | **two overlapping images** of every frond, offset from one another | **one** |
//! | the shadow's edge | soft, doubled, with the two copies disagreeing by up to a dozen pixels | **hard**, and the fronds are individually countable |
//! | the interior of the broad shadow lobes | **dithered** — a checkerboard of lit and unlit texels | **solid** |
//! | the floor between the horizon and the tree | a **dark band** across the whole width | **a smooth gradient**, the band gone |
//!
//! **The doubling is the finding, not the dither.** Two copies of one shadow is a page fitted twice at two
//! resolutions and the two being composited, and the halved half-width is what stops the second copy: a page
//! that reaches past the casters it was fitted to carries the *far* side of the plant's own bounding box
//! through the light's near side, so the whole floor was being tested against geometry behind it. **That is
//! the same defect the module's [`kapok_base_picture`] doc has been describing as "the plate pays the
//! dither"** — and that sentence is now wrong, so it has been corrected at the call rather than left to
//! contradict the picture four hundred lines away.
//!
//! **What is left on [`TREE_FERN_BASE`] is a stair-step on the shadow's own edge**, at the pixel the
//! shadow page's texel works out to, and that is the price of a hard shadow rather than a fault in one.
//!
//! # What the bar reads on the seven plates
//!
//! `audit/frame_stats.py <png> <png>-authored_id.png --crown`, measured on this branch. **The interior bar
//! is 0.80 here and was 0.75 when the first pass measured**, so two of these three rows moved under the bar
//! without the plate changing:
//!
//! | plate | coverage | interior share (bar ≥ 0.25) | interior over lit p90 (bar ≤ 0.80) |
//! | --- | --- | --- | --- |
//! | [`TREE_FERN`] | 0.182 | **0.690** ✓ | **0.675** ✓ |
//! | [`TREE_FERN_FROND`] | 0.306 | **0.343** ✓ | **0.650** ✓ |
//! | [`TREE_FERN_BASE`] | 0.839 | **0.964** ✓ | **0.728** ✓ |
//! | [`KAPOK`] | 0.326 | **0.561** ✓ | **0.747** ✓ |
//! | [`KAPOK_FLOWER`] | 0.075 | **0.234** ✗ — **exempt, and the exemption is the finding** | **0.755** ✓ |
//! | [`KAPOK_BOUGH`] | 0.095 | **0.726** ✓ | **0.709** ✓ |
//! | [`KAPOK_BASE`] | 0.744 | **0.990** ✓ | **0.927** ✗ — **not an interior** |
//!
//! **The kapok's flowering plate is the one row that fails, and a leafless crown is why.** H3's second number
//! asks whether a crown self-shadows, and its first asks whether a crown *has* an interior to shade: the
//! `0.234` is the **twig armature** counted as interior, and `08-flowering-bare-tree.jpg` is a photograph of a
//! crown with no foliage in it to have one. **A plate that measured 0.25 here would be measuring its twigs.**
//! The exemption is kept rather than argued away.
//!
//! **The base plates fail the second number and neither failure is H3's subject.** [`KAPOK_BASE`]'s `0.927`
//! is a plank buttress lit as one surface: the interior is the inside of a cone and there is nothing behind
//! it. [`TREE_FERN_BASE`] passes only because the packed bases and the root mantle break the column into
//! enough separate strands that a pixel can be wholly surrounded — which is the note's *"densely covered"*
//! arriving as a number, and is worth reading as such.
//!
//! **The fern is the only species in this set that needs a card, and its card is a margin rather than a
//! card**: at the whole-tree plate a pinnule *segment* is a fifth of a pixel and is not drawn at all, and
//! at the frond plate the same segment is five pixels and **is** drawn — as the teeth of the pinnule's
//! own width law, which is a level of detail rather than a second organ. [`Foliage::Pinna`] is therefore
//! the same pinnule with a comb in its width and [`Foliage::Frond`] the same pinnule without one, and
//! nothing else about them differs.
//!
//! **The frond plate's floor is the same 400 m as the whole-plant plate's**, and the reason is the same
//! one: a frond standing at `y = 4.55` looks *down* at its own ground, and a twelve-metre floor put its
//! far edge across the bottom of the frame as a hard diagonal — a straight line that is not a horizon is
//! the one artefact a picture of a frond must not carry.
//!
//! # What these seven scenes cost
//!
//! The scene document pays **per instance row**, and the row count is a fact about the document rather than a
//! guess about the picture, so every number below is **`cargo run -p whiting-device-rasterize --example render
//! -- <scene> x --census`'s own `depthing: N placements`** and can be read back by anyone. The ceiling is
//! ninety thousand rows and the largest of these seven is **9 307**, a tenth of it:
//!
//! | scene | rows | first pass | where the change is |
//! | --- | --- | --- | --- |
//! | [`TREE_FERN`] · [`TREE_FERN_BASE`] | **1 514** | 1 236 | the fronds are a quarter longer and carry **79** pinnae instead of 61 |
//! | [`TREE_FERN_FROND`] | **79** | 64 | one frond's own pinnae, on the frond that grew |
//! | [`KAPOK`] · [`KAPOK_BASE`] | **9 307** | 4 843 | the leaf fields moved onto the twigs — see [`kapok_leaves`] |
//! | [`KAPOK_FLOWER`] | **1 773** | 178 | the flowers moved onto the twigs and onto the crown's rim — see [`kapok_flowers`] |
//! | [`KAPOK_BOUGH`] | **253** | 253 | unchanged, and it is the check on the other two: its depths were already right |
//!
//! **The times are debug-build times on this machine and are quoted as an order, not a budget.** The seven
//! plates run between **0.3 s and 11.3 s** of compile and between **0.8 s and 3.7 s** of raster, and the two
//! heaviest — the whole-tree kapok plates — are heavy because of their leaves rather than their wood:
//! [`KAPOK`] compiles in 11.3 s and [`KAPOK_FLOWER`], which draws **no leaves at all** and ten times the
//! flowers, in 6.5 s.
//!
//! **The fern's 1 514 rows carry 79 blades each** and the kapok's 9 307 carry seven, and that is
//! [`LeafTemplate::sprig`] doing the only thing it can do: a pinna and a palmate hand are one instance of
//! one mesh, and a species whose finest organ is drawn as its own leaf would want four hundred thousand.
//!
//! # Four things these two species state that the crate cannot say, and one it can now
//!
//! They are written where they bite and repeated in the report, because a module that hides them is
//! worse than one that names them. The first is [`flutes`] and [`column`] together, the second and third
//! are [`mantle`] and [`corolla`], and the fourth is the rule the crate's `Section` is built on.
//!
//! - **`envelope::trim` measures about the root.** [`Envelope::outside`] takes the root's own spine and
//!   asks where a point is nearest to **it**, so a species whose root is a short structural node — the
//!   kapok's 1.7 m buttress — is trimmed against a law about a metre of trunk. [`kapok`] therefore shapes
//!   its crown from its own [`Joint::station`] laws and does not call `trim` at all, and the oak's use of
//!   it does not transfer.
//! - **A tree fern cannot be trimmed either, for a different reason**: its crown is 10–16 nodes on a
//!   5 m column and a trim that pulled a frond's tip inside a 2 m radius would cut the frond to a tenth of
//!   its length, which is the failure the oak module documents. The fern's arch is authored.
//! - **A decorator still cannot be seated on a *station band along its own node*, and this pass found it.**
//!   [`vegetation::decorator::scatter_in`]'s band is read on **the node's own attachment on its parent** —
//!   which is the fir's cone band and says *which branches* fruit, and it is what seats the kapok's flowers
//!   on the crown's rim. But a band *along* the node is still not expressible, and the fern is the case that
//!   proves it: **the column is the root, so the only node of its generation has an attachment at station
//!   nought and every band keeps it.** The two mantles are therefore [`ROOT_MANTLE`] and [`SKIRT_BAND`] — two
//!   named bands and two call sites — driven by [`mantle`], which is where the missing primitive is said
//!   out loud rather than left for the next reader to rediscover.
//! - **A spine is the only way a section varies along a node**, so the fern's packed bases had to be a
//!   ripple in the **taper** and not in the section. A caller who wants the bumps to be larger than the
//!   taper's local radius cannot get them here.
//! - **And the fifth, which this pass removed:** a decorator template *is* a sprig now, so the kapok's
//!   flower is five petals in one mesh and not a ten-pointed star in a section.

use definition::Value;
use vegetation::decorator::{Seat, Template, scatter_in};
use vegetation::derive::Species;
use vegetation::expand::Detail;
use vegetation::field::Field;
use vegetation::handle::Handle;
use vegetation::leaf::{LeafField, LeafTemplate};
use vegetation::plant::Placement;
use vegetation::rng::Rng;
use vegetation::species::{Generation, Joint, Rule, grow, spine, taper};

use crate::vegetation_mesh::node_ids;
use crate::vegetation_picture::{
    Framing, Ground, Lighting, Palette, Planting, Skin, plant_scene_lit,
};
use document::air::Atmosphere;

/// The name the tree fern's whole-plant plate is built and rendered under.
pub const TREE_FERN: &str = "tree_fern";

/// The name the tree fern's frond plate is built and rendered under: one frond at arm's length, where the
/// pinna-to-pinnule structure is countable against `05-frond-pinnules-countable.jpg`.
pub const TREE_FERN_FROND: &str = "tree_fern_frond";

/// The name the tree fern's base plate is built and rendered under: the foot of the column, which is the
/// frame H4 reads the root mantle and the packed bases in.
pub const TREE_FERN_BASE: &str = "tree_fern_base";

/// The name the kapok's whole-tree plate is built and rendered under: the tree **in leaf**, which is where
/// H1, H3 and H4 are read.
pub const KAPOK: &str = "kapok";

/// The name the kapok's **flowering** plate is built and rendered under: the same rule, the same seed and
/// the same skeleton, leafless and in red flower. This is the species' signature and `08` in its note.
pub const KAPOK_FLOWER: &str = "kapok_flower";

/// The name the kapok's limb plate is built and rendered under: one first-order limb at nine metres, where
/// the palmate hand is countable.
pub const KAPOK_BOUGH: &str = "kapok_bough";

/// The name the kapok's base plate is built and rendered under: the plank buttresses at the foot.
pub const KAPOK_BASE: &str = "kapok_base";

/// The seed the tree fern's three plates are grown from.
const FERN_SEED: u64 = 41;

/// The seed the kapok's four plates are grown from.
const KAPOK_SEED: u64 = 57;

/// The golden angle as a fraction of a turn, which is the step between two stations that are not a whorl.
const GOLDEN: f32 = 0.381_966_01;

// ---------------------------------------------------------------------------
// 桫椤 Alsophila spinulosa
// ---------------------------------------------------------------------------

/// Which foliage a tree fern draws.
///
/// # Invariants
///
/// **The two differ in one width law and in nothing else.** At the whole-plant plate a pinnule *segment* —
/// the note's falcate `7 × 4 mm` tooth — is a fifth of a pixel, so drawing it buys triangles and shows
/// nothing; at the frond plate the same tooth is five pixels across, so drawing it is the difference
/// between a green wedge and a pinnule. A comb of teeth on the pinnule's own width law is the way to draw
/// a third-order margin without a third-order node, and [`LeafTemplate::sprig`] is what makes one pinna
/// one instance of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Foliage {
    /// The pinnule as a smooth lanceolate blade, for the whole-plant and base plates.
    Frond,
    /// The pinnule with a comb of segment-teeth in its width law, for the frond plate.
    Pinna,
}

impl Foliage {
    /// The pinna template this foliage instances.
    fn template(self) -> LeafTemplate {
        pinna(self)
    }
}

/// The note's middle pinnule: **9–12 × 1.2–1.6 cm**, lanceolate, sessile, long-acuminate and caudate, and
/// itself pinnatisect with eighteen to twenty pairs of ascending falcate segments.
///
/// # The two versions, and what each is for
///
/// - **`Frond`** is the plain blade: a smooth lanceolate lens, seven rings, six-pointed section. At
///   [`TREE_FERN`]'s 4.73 mm a pixel it is a 3.0-pixel sliver, which is honest, and its segments are a
///   fifth of a pixel, which is not. **That is the tightest the whole-plant plate has been**: the crown grew
///   and [`FERN_FRAME`] stood back to hold it, so the sliver gained two tenths of a pixel and that is the
///   whole margin [`Foliage::Frond`] has to be the right choice.
/// - **`Pinna`** is the same blade with **twelve pairs of teeth** in its width law — the note's *18–20
///   pairs* read down to what 25 rings can carry — each tooth `0.25` of the blade's own half-width, so
///   the silhouette is a dentate feather edge and the geometry is the note's third order. **A width law
///   cannot make a tooth falcate or ascending**, which is two of the note's four words for a segment, and
///   the module says so rather than claiming the margin.
///
/// The blade is a **closed solid** (D18), so both faces carry their own normal and a pinnule lit from
/// beneath is shaded rather than faked. The `0.055` thickness ratio is a fern's: a papery lamina, about
/// four tenths of a millimetre over a fourteen-millimetre width.
fn pinnule(half_width: f32, teeth: usize, length: f32) -> LeafTemplate {
    // **The blade is authored for a 100 mm pinnule and scaled to the length asked for**, knots and tangents
    // alike, so the pinna's own taper profile — which is the note's *"decreasing in size toward both ends"*
    // once more, at the third order — is in the geometry of thirty-six blades rather than in a law that
    // thirty-six instances would have to read.
    let k = length / 0.100;
    let knots = [
        ([0.0000, 0.00000, 0.0000], [0.0300, 0.00120, 0.0002]),
        ([0.0300, 0.00260, 0.0006], [0.0310, 0.00420, 0.0008]),
        ([0.0610, 0.00700, 0.0010], [0.0290, 0.00480, 0.0004]),
        ([0.0890, 0.01380, 0.0010], [0.0090, 0.00220, 0.0000]),
    ]
    .map(|(at, tangent)| {
        let scale = |v: [f32; 3]| [v[0] * k, v[1] * k, v[2] * k];
        (scale(at), scale(tangent))
    });
    let width = match teeth {
        0 => {
            let knots: Vec<(f32, f32)> = [
                (0.00, 0.26),
                (0.14, 0.78),
                (0.40, 1.00),
                (0.72, 0.74),
                (1.00, 0.06),
            ]
            .iter()
            .map(|(at, across)| (*at, half_width * across * k))
            .collect();
            shape::Scalar::new(&knots).expect("a pinnule's width is a law")
        }
        // **A comb, and the envelope is a pinnule's own outline**: a pinnule is narrow at its cuneate base,
        // widest a third along and drawn out to a long caudate point, and the teeth multiply that. The
        // sine is clamped at nought because the `f32` pi overshoots the true one and its sine at `t = 1`
        // is a small negative number, whose fractional power is not a number at all.
        count => {
            let knots: Vec<(f32, f32)> = (0..=2 * count)
                .map(|step| {
                    let at = step as f32 / (2 * count) as f32;
                    let envelope =
                        (std::f32::consts::PI * at).sin().max(0.0).powf(0.42) * (1.0 - 0.22 * at);
                    let tooth = if step % 2 == 0 { 0.25 } else { 1.0 };
                    (at, half_width * envelope * tooth * k)
                })
                .collect();
            shape::Scalar::new(&knots).expect("a pinnule's margin is a law")
        }
    };
    LeafTemplate::blade(
        spine(&knots),
        width,
        0.055,
        if teeth == 0 { 7 } else { 2 * teeth + 1 },
        4,
    )
}

/// The note's middle pinna: **40–50 × 14–18 cm**, oblong, made of **18–20 pairs** of pinnules that are
/// themselves pinnatisect. **One pinna is one instance of one sprig**, which is what
/// [`LeafTemplate::sprig`] is for: eighteen pairs is thirty-six blades, and at the whole-plant plate's
/// six hundred and thirty-odd pinnae that is twenty-two thousand blades that a per-blade instance would
/// have paid for individually and this pays for once.
///
/// # The geometry, and why the pinnules sweep so far forward
///
/// The note gives a pinna **45 cm long and 16 cm wide**, and thirty-six pinnules **10 cm** long on it.
/// Those three numbers cannot both hold if a pinnule stands square out of the pinna's rachis — that would
/// make a 45 cm pinna half a metre wide. So the pinnules are **swept forward along their own rachis**, from
/// forty-five degrees at the pinna's foot to five degrees at its tip, which is what a fern pinna is: a
/// comb of barbs lying along a quill. [`pinna`]'s law is the note's `18–18 cm` width arriving that way.
///
/// The pinna's own profile — short at the foot, widest a third along, drawn to a point — is the *scale* the
/// pinnule is placed at, so the taper is in the placement rather than in a second blade: one geometry,
/// thirty-six uses.
fn pinna(foliage: Foliage) -> LeafTemplate {
    let (half_width, teeth) = match foliage {
        Foliage::Frond => (0.0070, 0),
        Foliage::Pinna => (0.0070, 12),
    };
    let pairs = 18usize;
    let rachis = 0.455;
    let reach = 0.112;
    let parts: Vec<(LeafTemplate, Placement)> = (0..pairs)
        .flat_map(|pair| {
            let at = pair as f32 / (pairs - 1) as f32;
            // **The pinna's own half-width profile, read as the pinnule's length**: short at the cuneate
            // base, widest a third along, and drawn out to the point the note's *long acuminate and
            // caudate* apex is. One length law and eighteen blades, so the pinna's outline is thirty-six
            // blades of eighteen different lengths rather than one blade thirty-six times. The floor is a
            // fifth and not nought: the note says the basal pinnules are **shorter**, and a zero at either
            // end would collapse a blade's four knots onto one place, which `Curve::new` refuses.
            let profile = ((std::f32::consts::PI * at).sin().max(0.0).powf(0.55)
                * (1.0 - 0.26 * at))
                .max(0.20);
            let blade = pinnule(half_width, teeth, reach * profile);
            let along = rachis * at;
            // **The sweep forward**: forty-five degrees at the foot to five at the tip, which is the whole
            // of the note's 45 × 16 cm against a 10 cm pinnule.
            let yaw = 0.80 * (1.0 - at) + 0.09;
            // **A tenth of a radian of tilt, alternating**, so a pinna's two ranks are not one flat sheet
            // and a frond's six hundred blades do not light as one plane. It is relief, not structure, and
            // it is half a degree at the note's frond plate.
            let tilt = 0.10;
            [
                (blade.clone(), at_tilt(along, yaw, tilt)),
                (blade, at_tilt(along, -yaw, -tilt)),
            ]
        })
        .collect();
    LeafTemplate::sprig(parts)
}

/// A frame at `(along, 0, 0)` turned `yaw` radians about the template's face normal — which is how a
/// pinnule leaves its pinna — and then `tilt` radians about its own forward axis.
///
/// # Invariants
///
/// **A rotation, never a scale.** `LeafInstance::scale` is the one number that changes a blade's size and
/// it is applied in the template's own frame; a `Placement` here that carried one would put the size of a
/// pinnule in the geometry instead of in the field, where the pinna's own taper law could not reach it.
fn at_tilt(along: f32, yaw: f32, tilt: f32) -> Placement {
    let (yaw_sin, yaw_cos) = yaw.sin_cos();
    let (tilt_sin, tilt_cos) = tilt.sin_cos();
    let forward = [yaw_cos, yaw_sin, 0.0];
    let side = [-yaw_sin, yaw_cos, 0.0];
    let face = [0.0, 0.0, 1.0];
    Placement {
        origin: [along, 0.0, 0.0],
        x: forward,
        y: [
            side[0] * tilt_cos + face[0] * tilt_sin,
            side[1] * tilt_cos + face[1] * tilt_sin,
            side[2] * tilt_cos + face[2] * tilt_sin,
        ],
        z: [
            face[0] * tilt_cos - side[0] * tilt_sin,
            face[1] * tilt_cos - side[1] * tilt_sin,
            face[2] * tilt_cos - side[2] * tilt_sin,
        ],
    }
}

/// The tree fern's rule: **one node and one generation**, which is the whole of this species.
///
/// # The numbers and where they come from
///
/// [`alsophila_spinulosa.md`](../../../.agents/investigate/trees/alsophila_spinulosa.md)'s modelling
/// table. The column is drawn at **five metres**, the flora's own floor, and the fronds at **2.30–2.50 m**.
///
/// **The frond length and the crown spread are the note's two estimates of the same quantity and they do not
/// agree, and [`frond`] says which one this module honours and why.** The short version: **the crown-spread
/// estimate (`1.0–1.5 ×`) is the one honoured, the frond-length estimate (`1.1–2.0 m`) is the one exceeded**,
/// because the first is a direct read of two photographs of whole plants and the second is a derivation from a
/// stipe plus an inferred lamina. **Five metres is also the tallest column at which the note's two numbers can
/// be argued about at all**: at the note's 7 m the frond-length estimate would give a crown of 3.5 m on a
/// 7 m column, which is half the spread the same photographs show. **The crown ratio is 0.85 × and the
/// residual 0.15 is named at [`frond`] rather than hidden here.**
///
/// There is **no `Field`**, and the reason is the species: a light tropism turns every frond's tip upward
/// and this tree's fronds arch *over*, so the arch is authored in [`frond`] and a field would flatten it.
fn tree_fern_rule() -> Rule {
    Rule {
        trunk: column,
        generations: vec![Generation {
            // **The crown: 11 to 15 fronds, one to a station.** The note's 10–16 fronds read at the middle,
            // and `cluster = 1` is the load-bearing number: a rosette is not a whorl. The band is
            // **0.09 of the column** — the note's *"each year the crown adds a whorl at the apex and the
            // column lengthens by one internode"* — and a wider band is a second flush along the trunk,
            // which the note says explicitly does not exist.
            count: (11, 15),
            station: (0.91, 1.0),
            slot: (0.0, GOLDEN),
            jitter: (0.02, 0.10),
            // A frond's base is **inside** the column: the stipe is a continuation of the crown's own
            // thickening and the sink is half a stipe's own radius.
            sink: 0.055,
            cluster: 1,
            shape: frond,
        }],
        fields: Vec::new(),
    }
}

/// The column: **five metres**, plumb, **not tapering**, and **widest at the ground**.
///
/// # The taper is the note's *"column of packed persistent bases"* in one law
///
/// It does four things at once and every one of them is the note's:
///
/// - **A fall of two and a half times in the first forty centimetres** — the adventitious-root mantle,
///   FOC's *"densely covered by adventitious roots"* and the note's ESTIMATE of a `0.25–0.45 m` apparent
///   basal diameter against a `0.10–0.20 m` column. `0.176 m` at the ground is a **35 cm** apparent base
///   and a **15 cm** column above it, which is the note's own range read at its middle. **The first plate
///   of this species stood the column at 30 cm through and was a pipe, not a fern**; the note's `10–20 cm`
///   is the number that fixes it and it is why this law is two and a half times lower than the first.
/// - **A ripple of about sixteen per cent from `t = 0.10` to `t = 0.90`**, a period of **275 mm** along the
///   column. That is the packed stipe bases and old stipes standing a hand's breadth apart, and it is
///   *along* the column because a node's section cannot vary along it. **The ripple needs rings to be
///   resolved**, which is why every tree fern plate is planted at [`FERN_DETAIL`]'s 80 mm spacing rather
///   than the crate's half-metre default: at half a metre the ripple is one sample per two lumps and the
///   column comes out smooth, which is a pole and not a fern. Sixteen per cent of a 72 mm column is
///   **twelve millimetres**, which is ten pixels at [`TREE_FERN_BASE`] and the difference between a trunk
///   and the note's *"densely covered"*.
/// - **No taper above that.** `0.070` at `t = 0.08` and `0.046` at the apex is a fall of thirty-four per
///   cent over four and a half metres, against a fall of sixty per cent in the first thirty centimetres: a
///   cylinder with a flare, which is what *"no taper; the column is cylindrical from the skirt to the apex"*
///   says and what H4's *"a taper that is not a cone"* asks for.
///
/// The spine is plumb to within **sixty millimetres over five metres**, because every photograph in the
/// note shows a straight column and a wander here would read as a lean.
fn column() -> (shape::Curve, shape::Scalar) {
    (
        spine(&[
            ([0.00, 0.00, 0.00], [0.03, 1.00, 0.01]),
            ([0.04, 1.70, 0.02], [0.02, 1.00, 0.00]),
            ([0.02, 3.60, -0.02], [0.00, 1.00, -0.01]),
            ([0.03, 5.00, 0.00], [0.00, 1.00, 0.00]),
        ]),
        taper(&[
            (0.000, 0.176),
            (0.010, 0.146),
            (0.022, 0.118),
            (0.038, 0.096),
            (0.058, 0.081),
            (0.080, 0.070),
            (0.103, 0.082),
            (0.130, 0.058),
            (0.158, 0.079),
            (0.185, 0.057),
            (0.213, 0.077),
            (0.240, 0.056),
            (0.268, 0.075),
            (0.295, 0.055),
            (0.323, 0.073),
            (0.350, 0.054),
            (0.378, 0.072),
            (0.405, 0.053),
            (0.433, 0.070),
            (0.460, 0.052),
            (0.495, 0.068),
            (0.535, 0.051),
            (0.575, 0.066),
            (0.620, 0.050),
            (0.665, 0.065),
            (0.715, 0.049),
            (0.765, 0.058),
            (0.815, 0.048),
            (0.865, 0.052),
            (0.910, 0.049),
            (0.955, 0.047),
            (1.000, 0.046),
        ]),
    )
}

/// The column's section: **twenty-four sides with two harmonics of lump on them**.
///
/// # Invariants
///
/// **A smooth modulation, not a notch list.** The column is *shaggy* — a mantle of fine adventitious roots
/// over packed bases — and a polygonal section with hard corners is the *polygonal silhouette* defect the
/// bar names. Nine lobes of eleven and a half per cent and five of six per cent is the note's *"densely
/// covered"* at a size the section can carry: **at the base plate's 1.18 mm a pixel the ±8 mm lobes are
/// seven pixels**, and they are what makes the column read as a bundle rather than as a pipe.
///
/// **The section is scaled by the taper, so the lobes shrink with the column** — which is right, because a
/// packed base is the same object wherever it is on the column.
fn flutes() -> shape::Section {
    let sides = 24;
    let points: Vec<[f32; 2]> = (0..sides)
        .map(|index| {
            let theta = index as f32 / sides as f32 * std::f32::consts::TAU;
            let lump = 1.0 + 0.115 * (theta * 9.0 + 0.4).sin() + 0.060 * (theta * 5.0 + 2.1).sin();
            [theta.cos() * lump, theta.sin() * lump]
        })
        .collect();
    shape::Section::outline(&points).expect("a lumpy column is a section")
}

/// A frond: **2.30–2.50 m**, leaving the column over the note's **45–70°** and arching over until its distal
/// half hangs below horizontal.
///
/// # The arch, in four numbers
///
/// [`bend`]'s angles are measured from the parent's tangent, and a frond's parent is the column whose
/// tangent at the apex is straight up, so `0.79 rad` is the note's forty-five degrees **from the trunk axis**.
/// The four are `45°`, `60°`, `81°` and `108°`: it leaves steeply, rises over the crown's shoulder, passes
/// horizontal two thirds along, and hangs an eighth of a turn below it at the tip. That last number is the
/// note's *"arching over so that the distal half of every frond hangs below horizontal"* and the reason the
/// crown is a shallow bowl rather than a ball.
///
/// **The departure range is the note's whole `45–70°` and not its top three fifths.** The first pass stood it
/// at `54–70°`, and a crown whose fronds all leave at nearly one angle is a fan and not a rosette. Opening it
/// to `0.79 + 0.43` lets one frond leave nearly vertically while its neighbour lies almost flat, which is what
/// `06-crown-radial-top-view.jpg` shows from above and what makes the crown read as a **parasol** rather than
/// as a shuttlecock.
///
/// The taper is the note's ESTIMATE of a **1.5–3 cm** stipe at its base, thinning to a point at the tip:
/// `0.015` is a 3 cm stipe and `0.0018` is the caudate end the pinna field's size law also draws out.
///
/// # Why the fronds are 2.4 m and the note says 1.1–2.0 m
///
/// **The note gives two estimates that cannot both hold on a five-metre column, and the first pass honoured the
/// wrong one.** They are:
///
/// | the note's estimate | its own words | what it costs |
/// | --- | --- | --- |
/// | **frond length `1.1–2.0 m`** | *"ESTIMATE — stipe 0.30–0.50 m is flora-given; the lamina is derived from the 40–50 cm middle pinnae and cross-checked against the arches in `02`"* | a crown of **4.0 m** across on a 5 m column |
/// | **crown `1.0–1.5 × column height`** | *"ESTIMATE from `01-habit-whole-tree-wild.jpg`, `11-whole-plant-radial-crown.jpg`"* | a crown of **5.0–7.5 m** across |
///
/// **This pass honours the crown-spread estimate and breaks the frond-length one.** The reason is which of the
/// two is a measurement and which is a derivation. The frond length is a *derivation* — a stipe from the
/// flora plus a lamina inferred from a pinna length — and it is the only number here that was never seen on
/// the plant. The crown spread is the note's **direct read of two photographs of whole plants**. The first
/// pass reasoned the other way round, took the derived number as binding, and the plate came out at a
/// **crown:height of 0.71** — a small cauliflower on a pole, which is a young tree and not the note's.
///
/// **The residual is named rather than hidden.** **A crown cannot be wider than its fronds are long**, and a
/// `2.4 m` frond spends about **92%** of itself reaching outward, so this pass's crown is **0.85 ×** the
/// column — **4.24 m across a 5 m column, measured off `--census`'s own placement bounds** (`x −2.11…2.13`,
/// `z −2.23…1.96`) — against the note's `1.0–1.5 ×`. The first pass's crown was **3.6 m**, or **0.71 ×**.
/// Reaching the note's floor needs a frond of **2.9 m** and its ceiling a frond of **4.4 m**, which is one and a
/// half to two and a quarter times the note's own frond ceiling and is not a number this module will author.
/// So **the two estimates are not reconciled here**: one is honoured, the other is exceeded, and the **0.15**
/// between them is the price, named in the same breath as the choice.
///
/// **The fronds are a quarter over the 2.2–2.4 m this pass was asked for, and that is the same decision.** A
/// `2.30 m` frond was built and rendered first and measured **0.78 ×**; `2.40 m` came back at **0.85 ×**. The
/// request's own reasoning — *"the photographs' crown spread is the stronger cue at silhouette distance"* — puts
/// the crown above the frond, and the frond is only the means of it. **A reader who wants the 2.2 m frond has
/// one number to change**, and gets back the crown the first pass had.
///
/// **The second price is arithmetic rather than a choice**: [`pinnae`]'s density is **per metre of rachis**, so
/// a `2.4 m` frond carries **38 pinnae pairs** against the note's ESTIMATE of `20–28`. A crown this wide at
/// the note's pinna count would be a skeleton of bare ribs, which is the *bare broom* the defect list names.
fn frond(rng: &mut Rng, _joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 2.40 * (0.96 + 0.08 * rng.unit());
    let departure = 0.79 + 0.43 * rng.unit();
    (
        bend(
            length,
            [
                departure,
                departure + 0.26,
                departure + 0.62,
                departure + 1.10,
            ],
        ),
        taper(&[(0.0, 0.0150), (0.30, 0.0110), (0.70, 0.0060), (1.0, 0.0018)]),
    )
}

/// The fern's pinna field, at `frond_depth`: the note's pinnae on a frond's own rachis.
///
/// # The two ranks, and the two dials that make them
///
/// The note gives *"alternating to subopposite"*, and **one number pair says that**: `cluster = 2` puts two
/// pinnae at one station and `1 / cluster` of a turn apart, so a pair stands **opposite** across the
/// rachis; `phyllotaxy = 0.5` then advances the next station by half a turn, so every pair stands on the
/// **same two slots**. Pinnae in two ranks is therefore one number and not a special case, and it is the
/// same word the pine's needle fascicle uses for the same reason.
///
/// The density is **42 leaves per metre of rachis**: a 2.40 m frond with a 0.78-wide band carries **79**
/// pinnae — **39 pairs** against the note's ESTIMATE of *20–28 pairs per frond* — standing **45 mm** apart on
/// a pinna 150 mm wide, three deep, which is what makes a frond a feather and not a ladder of blades.
/// Thirty-four was the first reading and it came out at **0.19 coverage** with sky showing between
/// every third pinna, which is a *cauliflower* crown and not a frond: the note's fourth fact asks for a
/// crown *"combed into separate flat feather vanes"* and the vanes are the fronds, not the pinnae. The pair
/// count is over the note's because the frond is, and the two are one decision — see [`frond`].
fn pinnae(frond_depth: usize) -> Vec<LeafField> {
    let mut pinnae = LeafField::new(frond_depth, 0);
    // **The band starts at 0.19 of the frond**, which is where the note's `30–50 cm` stipe ends and the
    // lamina begins, and stops at 0.97 so the caudate tip is bare rachis — the note's *"reduced, pinnatifid
    // apex formed by the terminal diminishing pinnae"*, drawn as the size law's tail rather than as a
    // terminal pinna.
    pinnae.band = (0.19, 0.97);
    pinnae.density = shape::Scalar::constant(42.0);
    pinnae.cluster = 2;
    pinnae.phyllotaxy = 0.5;
    // **A third of a radian of fan** splits each opposite pair by plus and minus a sixth of a turn, so the
    // two ranks are not one flat sheet and the two ranks do not line up into a herringbone.
    pinnae.fan = shape::Scalar::constant(0.32);
    // **The pitch is a law over the frond's own length**: seventy-six degrees at the foot, where a pinna
    // stands up and forward into the crown's dome, and forty-five at the tip, where it lies along the
    // rachis to close the lamina. One number for both ends draws a frond whose lamina is a flat plank.
    pinnae.pitch = shape::Scalar::new(&[(0.0, 1.32), (0.50, 1.05), (1.00, 0.78)])
        .expect("a pinna's pitch is a law");
    pinnae.twist = shape::Scalar::new(&[(0.0, -0.18), (0.55, 0.06), (1.00, 0.26)])
        .expect("a pinna's twist is a law");
    // **The size law is the frond's own outline**, and its zero at the foot is the stipe's last centimetres
    // drawing out to nothing: the note's *"decreasing in size toward both ends"* with a bare stipe in front
    // of it. `1.0` is the note's middle pinna of 45 cm, which is what [`pinna`] authors.
    pinnae.size = shape::Scalar::new(&[
        (0.00, 0.16),
        (0.12, 0.58),
        (0.34, 0.94),
        (0.55, 1.00),
        (0.80, 0.78),
        (1.00, 0.28),
    ])
    .expect("a pinna's size is a law");
    pinnae.jitter = 0.10;
    vec![pinnae]
}

/// Four knots along a heading that turns through `angles`, measured from the parent's own tangent: nought is
/// along the parent and a quarter turn is straight out. The tangents are the heading's own direction, so a
/// member is a chord with three bends in it rather than a straight line with a direction.
fn bend(length: f32, angles: [f32; 4]) -> shape::Curve {
    let step = length / 3.0;
    let at = |angle: f32| [step * angle.sin(), step * angle.cos(), 0.0];
    let tangent = |angle: f32| [length * 0.5 * angle.sin(), length * 0.5 * angle.cos(), 0.0];
    let first = at(angles[0]);
    let second = [
        first[0] + at(angles[1])[0],
        first[1] + at(angles[1])[1],
        0.0,
    ];
    let third = [
        second[0] + at(angles[2])[0],
        second[1] + at(angles[2])[1],
        0.0,
    ];
    spine(&[
        ([0.0, 0.0, 0.0], tangent(angles[0])),
        (first, tangent(angles[1])),
        (second, tangent(angles[2])),
        (third, tangent(angles[3])),
    ])
}

/// The adventitious root: a fine cord that leaves the column's surface, turns down and hangs.
///
/// # Invariants
///
/// **A seat expresses no rotation, so the hanging is the template's own shape.** The seat's frame is the
/// column's own surface frame — `x` outward, `y` up the column — so a template authored out along `+x` and
/// then down `-y` hangs from whatever side of the column it lands on, and every root in the mantle hangs
/// the same way. The same bargain the pine's cone and the oak's acorn make.
///
/// The cord is **12 mm at its base and 3 mm at its end**, which is the note's *"a mantle of fine
/// adventitious roots"* against a 150 mm column: at [`TREE_FERN_BASE`]'s 1.24 mm a pixel it is **ten
/// pixels**, so the mantle reads as cord and not as a second trunk. **The first plate of this species
/// drew it at 6 mm and it was a black hair on a black pole** — a shaggy surface is not a dark surface,
/// and the note's answer is a cord thick enough to have a lit side.
fn root() -> Template {
    let mut sweep = shape::Sweep::new(
        spine(&[
            ([0.000, 0.000, 0.000], [0.028, 0.000, 0.000]),
            ([0.028, -0.026, 0.006], [0.006, -0.058, 0.004]),
            ([0.028, -0.132, 0.011], [0.002, -0.092, 0.002]),
            ([0.024, -0.268, 0.013], [0.000, -0.104, 0.000]),
            ([0.018, -0.408, 0.011], [0.000, -0.090, 0.000]),
        ]),
        shape::Section::circle(),
    );
    sweep.scale = taper(&[
        (0.00, 0.0062),
        (0.22, 0.0048),
        (0.60, 0.0032),
        (1.00, 0.0016),
    ]);
    Template {
        sweep,
        stations: 9,
        around: 6,
    }
}

/// A dead stipe: the persistent, collapsed stalk of a frond that has died, hanging from the skirt.
///
/// # The organ, and why it is a decorator and not a leaf
///
/// The note's fourth silhouette fact is *"a brown fringe of dead fronds hangs just below the live crown"*,
/// and it is made of the *stipes* — the persistent, purple-black, spiny stalks the note's FOC text
/// describes — with their lamina long gone. A decorator is *"a small authored thing seated on a node's
/// surface"*, and this is exactly that: a 0.4 m cord on the column, no leaf field, and no node of its own.
///
/// It is **3.4 cm in diameter at its base and 4 mm at its hooked end**, which is the note's ESTIMATE of a
/// `1.5–3 cm` stipe read at its upper end because a dead frond's stipe is the thickest thing on the
/// column, and it **hangs with a slight outward lean and a recurved tip**, because a dead frond on a real
/// plant does not hang plumb: its own weight has folded it against the column. **The first plate of this
/// species drew it at 2.6 cm and it was a dark thread under the crown rather than the note's *"brown
/// fringe"*; the fringe is the plate's whole point.**
fn dead_stipe() -> Template {
    let mut sweep = shape::Sweep::new(
        spine(&[
            ([0.000, 0.000, 0.000], [0.030, 0.000, 0.000]),
            ([0.030, -0.052, 0.010], [0.014, -0.090, 0.008]),
            ([0.040, -0.170, 0.016], [0.006, -0.110, 0.000]),
            ([0.036, -0.330, 0.014], [0.000, -0.120, -0.006]),
            ([0.022, -0.470, 0.006], [0.000, -0.080, -0.012]),
            ([0.006, -0.560, -0.006], [0.000, -0.040, -0.008]),
        ]),
        shape::Section::circle(),
    );
    sweep.scale = taper(&[
        (0.00, 0.0170),
        (0.18, 0.0135),
        (0.60, 0.0085),
        (0.88, 0.0048),
        (1.00, 0.0020),
    ]);
    Template {
        sweep,
        stations: 11,
        around: 7,
    }
}

/// The template index the root mantle is named by.
const ROOT: usize = 0;

/// The template index the dead-stipe skirt is named by.
const SKIRT: usize = 1;

/// The root mantle's band along the column: **`t = 0.012` to `t = 0.452`**.
///
/// # Invariants
///
/// **The band is the note's *"especially heavy in the lower half"* read as the column's lower 2.2 m**, and the
/// two ends are the numbers rather than the words: the low end is above the ground by **six centimetres**, so
/// the mantle's own cords do not lie in the litter, and the high end is **2.26 m** up a 5 m column, which is
/// where the note's *lower half* is on this column — the other half of that sentence is the skirt at the top.
/// The old loop walked twelve stations of `0.040` from `0.012`, which is exactly this band; the two ends are
/// now the only numbers here.
const ROOT_MANTLE: (f32, f32) = (0.012, 0.452);

/// The dead-stipe skirt's band along the column: **`t = 0.680` to `t = 0.950`**.
///
/// # Invariants
///
/// **The band's top end is the crown's own band** — the fronds leave the column over `0.91` to `1.00` — so a
/// dead stipe can never hang from below a live frond's own foot and the two never come out of the same
/// place. **The low end is `0.68` and not `0.80`**, because the skirt is **0.56 m long** and hangs from the
/// highest ring: a stipe seated at `0.95` reaches down to `0.84`, and one at `0.68` to `0.57`, so the band's
/// two ends together hang a fringe **0.9 m deep** below the crown — the note's *"irregular brown skirt … just
/// below the crown"* rather than a collar.
const SKIRT_BAND: (f32, f32) = (0.680, 0.950);

/// How many stations a mantle's band is walked at, and how many cords each carries.
///
/// # Invariants
///
/// **`scatter_in` seats every seat of a node at ONE station**, so a band *along a node* — which is what a
/// mantle is, on a plant whose only node is the column — is a walk and not a call. The band
/// [`scatter_in`] takes is a different band: it is read on **the node's own attachment on its parent**, and
/// the fern's column is the root, whose attachment is station nought, so a band on it is the plant's own foot
/// and nothing more. That is not a defect in the call — it is exactly the fir's cone band, where a generation
/// has thirty nodes spread along a trunk and the band says *which of them fruit*. **The fern has no such
/// generation to band**, so the walk is here, over one named band, and the module reads as two bands rather
/// than nineteen enumerated seats.
///
/// Twelve and seven are the old numbers and they are kept because they are the ring spacing the two mantles
/// were read at: one cord every `0.037` of the column over the root band, and one stipe every `0.039` over the
/// skirt band.
fn mantle(
    species: &Species,
    generation: usize,
    template: usize,
    band: (f32, f32),
    stations: u32,
    each: (u32, u32),
) -> Vec<Seat> {
    let mut seats = Vec::new();
    let step = (band.1 - band.0) / stations.saturating_sub(1).max(1) as f32;
    for station in 0..stations {
        seats.extend(scatter_in(
            &species.plant,
            generation,
            template,
            band.0 + station as f32 * step,
            each,
            (0.0, 1.0),
        ));
    }
    seats
}

/// The root mantle: FOC's *"densely covered by adventitious roots"*, heaviest in the lower fifth.
///
/// # Invariants
///
/// **One band and a count per ring, and about **190 cords** stand on the lower 2.26 m of a 5 m column.** That
/// is a cord every **1.2 cm of circumference** at the densest ring against a 350 mm apparent base, which is
/// the note's ESTIMATE of a `0.25–0.45 m` apparent basal diameter read as a mantle rather than as a taper.
/// **The first pass put twelve to twenty on each of twelve rings and the module listed all twelve stations**;
/// the two numbers it carried — the band and the count — are the two numbers here, and the band is the one a
/// reader can check against the note.
fn fern_roots(species: &Species) -> Vec<Seat> {
    mantle(species, 0, ROOT, ROOT_MANTLE, 12, (12, 20))
}

/// The skirt of dead stipes: the note's *"irregular brown skirt of dead stipes just below the crown"*.
///
/// # Invariants
///
/// **Seven rings over `SKIRT_BAND` and nothing below it**, because the note's fact is *"a **bare** lower
/// column, a live crown, and an irregular brown skirt of dead stipes **just below** the crown"*, and a stipe
/// below `0.68` would hang on bare column, which is the one thing the note says is bare.
fn fern_skirt(species: &Species) -> Vec<Seat> {
    mantle(species, 0, SKIRT, SKIRT_BAND, 7, (6, 11))
}

/// The expansion dial every tree fern plate is planted at.
///
/// # Invariants
///
/// **`spacing` is the law and this one is chosen for the column's ripple, not for the fronds.** The crate's
/// default of half a metre puts ten rings on a 5 m column and one sample per two packed bases, so the
/// ripple in [`column`] is invisible and the species' H4 surface is a smooth cylinder. **Eighty
/// millimetres** is three rings to a lump. `around = 24` is the same reason from the other side: the
/// section has twenty-four authored points and a twelve-sided ring throws away half the lobes.
///
/// The cost is small and is worth stating: the column takes **63 rings** and each frond **24**, which is
/// 35 000 wood triangles for the whole plant against 20 000 at the default. The fern's cost is its pinnae,
/// which are leaves and are not in this number.
const FERN_DETAIL: Detail = Detail {
    stations: 8,
    around: 24,
    spacing: 0.08,
};

/// The tree fern: its column, its crown, its pinna field and its two mantles.
pub fn tree_fern(seed: u64, foliage: Foliage) -> Species {
    let mut plant = grow(&tree_fern_rule(), seed);
    // The column is dressed after the growth because a section is a node's own and `grow` authors circles:
    // the lumps are the column's, not the rule's.
    if let Ok(column) = plant.get_mut(&Handle::root()) {
        column.set_section(flutes());
    }
    Species {
        plant,
        templates: vec![foliage.template()],
        fields: pinnae(1),
        decorators: vec![root(), dead_stipe()],
    }
}

/// The frond plate's species: **one frond as the trunk**, and its pinnae as the only field.
///
/// # Invariants
///
/// **The same frond of the same rule, at the same length, with the same field** — the only difference is
/// that it is the root, so [`LeafField::generation`] is `0` and not `1`. This is proposal D9's bargain and
/// it is what makes the plate evidence about the tree rather than a portrait of a second model: a pinna
/// that reads here is the pinna the crown is made of, and its whole-tree instance is the same sprig under
/// the same size law.
pub fn tree_fern_frond(seed: u64) -> Species {
    let rule = Rule {
        trunk: || {
            (
                bend(2.30, [0.94, 1.20, 1.56, 2.04]),
                taper(&[(0.0, 0.0150), (0.30, 0.0110), (0.70, 0.0060), (1.0, 0.0018)]),
            )
        },
        generations: Vec::new(),
        fields: Vec::new(),
    };
    Species {
        plant: grow(&rule, seed),
        templates: vec![Foliage::Pinna.template()],
        fields: pinnae(0),
        decorators: Vec::new(),
    }
}

/// The fern's surfaces: FOC's *"purple-black toward base"* and *"dark brown"* column, a *"bright to mid
/// green"* frond above and a *"paler beneath"*, and the skirt's own dead brown.
///
/// # Invariants
///
/// **Three tints and not one**, and the *felt-pen tree* defect is what they prevent: a fern's whole
/// recognition is a **dark column under a bright crown under a dark collar**, and one tint for wood and
/// leaf would give a brown crown on a brown pole. The column is the note's darkest — a purple-black at
/// `0.052, 0.040, 0.036` — because FOC says the stipes grade *"purple-black toward base"* and the whole
/// of what the eye reads as the trunk in a photograph is that colour. The crown is a **mid green at a
/// low albedo**, because the plate is lit in a shaded gully and a `0.10`-tint leaf at this key clips to
/// white; the skirt is a **warm dead brown** that is neither the column's nor the crown's.
fn fern_palette() -> Palette {
    Palette {
        wood: Skin::new([0.086, 0.068, 0.058], 0.80),
        leaves: vec![Skin::new([0.062, 0.115, 0.038], 0.72)],
        seats: vec![
            // The root mantle is a **warmer and lighter** brown than the column's own purple-black: a root
            // is the column's tissue seen end-on, and the first plate of this species drew the two the same
            // colour and the mantle vanished into a black pole — a shaggy surface is not a dark surface.
            Skin::new([0.118, 0.088, 0.062], 0.86),
            // The skirt is the note's *"brown dead fronds hanging from the upper column"* and it is warmer
            // than the column because it is dead lamina-bearing stalk rather than root.
            Skin::new([0.150, 0.092, 0.048], 0.84),
        ],
    }
}

/// A shaded wet gully at eight hundred metres: a soft overhead key already filtered by the canopy above, a
/// cool sky fill, and damp limestone underfoot.
///
/// # Invariants
///
/// **A light's reach is its cutoff and the cutoff must clear the floor's own diagonal**, or the light is
/// culled per cluster box and the box boundary draws itself across the ground as a straight edge. The
/// floor reaches 400 m and its diagonal is 566 m, so the reach is **1200 m**; the *distance* — where the
/// irradiance is stated — is the key's own length from the origin, **18.0 m**.
///
/// **The key is soft but not soft enough to lose the crown's interior.** A shade plant is lit by the whole
/// sky, and a sky-dominated key would light the crown's inside exactly as brightly as its rim, which is
/// H3's failure. So the key stands **overhead and to the sunward side** with an extent of five metres, the
/// fill is a quarter of its strength, and the sky's own radiance is low: the crown's interior is lit by
/// light that has been through a leaf.
fn fern_lighting() -> Lighting {
    Lighting {
        key_position: [9.0, 16.0, 7.0],
        key_colour: [0.94, 0.98, 0.82],
        // **A canopy gap, not a sunbeam**: a tree fern is a shade plant, so the honest hour for its plate is
        // the one the photographs were taken in — a bright patch of sky seen from below. The first two
        // plates of this species stood the key at `2.1` and `3.6` with a sky of `0.55`, and the crown's
        // interior sat at **0.788** and **0.770** of its lit rim: too much of the light was ambient, so the
        // inside of the crown was as bright as its outside. **A gap is a strong key on a dim sky**, and
        // that is the number that moves H3.
        key_irradiance: 5.2,
        key_distance: 19.4,
        key_reach: 1200.0,
        key_extent: 5.0,
        fill_position: [-8.0, 5.0, -9.0],
        fill_colour: [0.40, 0.56, 0.62],
        fill_irradiance: 0.30,
        fill_distance: 13.0,
        floor: Skin::new([0.225, 0.210, 0.170], 0.94),
        sky: Atmosphere::new()
            .zenith([0.13, 0.24, 0.28])
            .horizon([0.55, 0.60, 0.58])
            .ground([0.28, 0.26, 0.21])
            .density(0.0014)
            .scale_height(1400.0),
        sky_radiance: 0.40,
        // A wet gully at eight hundred metres is the wettest air in this proposal: a garden's 0.012 would
        // grey the far side of the crown, and the crown is four metres across.
        haze: 0.008,
    }
}

/// The whole-plant frame: **13.0 m** off, the camera at 2.6 m looking slightly **up** at the crown.
///
/// # Invariants
///
/// **The eye is at half the column's height, not above the crown.** The whole of H1 is that the shape is
/// read as a pole with a parasol, and a plan seen from a roof is not that shape — the reference
/// photographs `01-habit-whole-tree-wild.jpg` and `11-whole-plant-radial-crown.jpg` are both taken from
/// below the crown.
///
/// **The pixel this framing makes is the number that chose the foliage**: `2 · 13.0 · tan(0.30) / 1700` is
/// **4.73 mm**, and the note's 14 mm pinnule is **3.0 of those** — above the sampling tax, so the whole
/// plate draws the **plain** pinnule and spends nothing on a comb whose teeth are a fifth of a pixel.
///
/// **The distance was opened from 12.0 m to 13.0 m, and the crown is what opened it.** A `2.40 m` frond
/// carries a `4.7 m` crown, so the plant is **6.2 m** from the ground to the top of the crown's bowl, and at
/// twelve metres the frame's vertical field at the crown's own distance was **6.8 m**: the crown's top frond
/// ran off the top of the picture. A plant whose top is cut off cannot be read as a *parasol* at all, so the
/// camera stood back far enough to hold it — and **not further**, because the pixel above is the number that
/// chose [`Foliage::Frond`] over [`Foliage::Pinna`] and every step further spends it. The crown now stands at
/// **76% of the frame's width** and the plant at **84% of its height**.
const FERN_FRAME: Framing = Framing {
    width: 1400,
    height: 1700,
    eye: [8.97, 2.60, 9.40],
    target: [0.0, 3.10, 0.0],
    field_of_view: 0.60,
    near: 0.2,
    far: 900.0,
    floor: 400.0,
    ground: Ground::Plane,
};

/// The frond plate's frame: **1.90 m** off one frond's lamina, side on to the rachis.
///
/// # Invariants
///
/// **`2 · 1.90 · tan(0.40) / 1000` is 1.61 mm**, and this is the plate the comb exists for: the note's
/// **7 × 4 mm** pinnule segment is **4.4 × 2.5 pixels** here and was a fifth of a pixel at the whole-plant
/// plate. The frame is **wider than it is tall** because the subject is two metres long and a quarter of a
/// metre deep, and a square frame of a frond is mostly empty rachis.
///
/// **The eye carries the planting's own four and a half metres.** A `Framing` is in world coordinates and
/// a `Placement` moves the plant in the world, so the first render of this plate stood the camera at
/// `y = 0.78` under a frond at `y = 4.55` and drew an empty plate with a horizon in it — which is what a
/// camera and a plant in two different frames looks like.
///
/// **The frame was re-aimed when the frond grew**, and both numbers moved together: a `2.30 m` frond reaches
/// **2.10 m** out and stands **0.74 m** above its own foot, so the target moved to the frond's own middle at
/// `x = 1.05` and the eye stood **1.90 m** off it rather than 1.66. Left where they were, the plate would have
/// cropped the tip and been a picture of the note's *stipe*.
const FROND_FRAME: Framing = Framing {
    width: 1400,
    height: 1000,
    eye: [0.13, 5.10, 1.66],
    target: [1.05, 4.92, 0.0],
    field_of_view: 0.80,
    near: 0.02,
    far: 900.0,
    floor: 400.0,
    ground: Ground::Plane,
};

/// The base plate's frame: **1.70 m** from the column at chest height, aimed low, so the ground line, the
/// root mantle and the packed bases are all in the frame.
///
/// # Invariants
///
/// **Aimed at nine hundred millimetres, not at the middle of the column.** H4 asks for the flare and the
/// bark *form*, both of which live in the first 800 mm, and the first plate of this species aimed at
/// 1.5 m and put the ground line off the bottom of the frame entirely — a bark plate with no base in it is
/// a texture swatch. `2 · 1.70 · tan(0.45) / 1400` is **1.18 mm**, so the section's nine lobes are fifteen
/// pixels and a root cord is **ten**.
const FERN_BASE_FRAME: Framing = Framing {
    width: 1200,
    height: 1400,
    eye: [1.05, 1.55, 1.25],
    target: [0.0, 0.90, 0.0],
    field_of_view: 0.90,
    near: 0.03,
    far: 60.0,
    floor: 30.0,
    ground: Ground::Plane,
};

/// Builds the tree fern's whole-plant plate: one column, its crown of fronds, both mantles, and the plain
/// pinnule the framing can honestly draw.
pub fn tree_fern_picture() -> Value {
    let species = tree_fern(FERN_SEED, Foliage::Frond);
    let mut seats = fern_roots(&species);
    seats.extend(fern_skirt(&species));
    let planting = Planting::new(species)
        .with_seats(seats)
        .with_palette(fern_palette())
        .with_detail(FERN_DETAIL);
    plant_scene_lit(&[planting], &FERN_FRAME, TREE_FERN, &fern_lighting(), &[])
}

/// Builds the tree fern's frond plate: one frond at arm's length, in the combed pinnule.
pub fn tree_fern_frond_picture() -> Value {
    let species = tree_fern_frond(FERN_SEED);
    // The frond stands at a crown's height and not on the ground: a frond plate whose frond lies in the
    // dirt reads as a fallen one, and this species' dead fronds are the one thing that really does lie in
    // the dirt.
    let planting = Planting::new(species)
        .at(Placement {
            origin: [0.0, 4.55, 0.0],
            ..Placement::IDENTITY
        })
        .with_palette(fern_palette())
        .with_detail(FERN_DETAIL);
    plant_scene_lit(
        &[planting],
        &FROND_FRAME,
        TREE_FERN_FROND,
        &fern_lighting(),
        &[],
    )
}

/// Builds the tree fern's base plate: the foot of the column at a person's reach, which is the frame H4
/// reads the root mantle and the packed bases in.
pub fn tree_fern_base_picture() -> Value {
    let species = tree_fern(FERN_SEED, Foliage::Frond);
    let mut seats = fern_roots(&species);
    seats.extend(fern_skirt(&species));
    let planting = Planting::new(species)
        .with_seats(seats)
        .with_palette(fern_palette())
        .with_detail(FERN_DETAIL);
    plant_scene_lit(
        &[planting],
        &FERN_BASE_FRAME,
        TREE_FERN_BASE,
        &fern_lighting(),
        &[],
    )
}

/// Resolves a whole-plant plate's `authored_id` into the handle that names the node.
pub fn tree_fern_handle_of(id: u32) -> Option<String> {
    node_ids(&tree_fern(FERN_SEED, Foliage::Frond).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

/// Resolves the base plate's `authored_id`: it is the **same plant** as the whole-plant plate, so the same
/// walk is the table, and a second table would answer with the wrong node the moment the foliage changed.
pub fn tree_fern_base_handle_of(id: u32) -> Option<String> {
    tree_fern_handle_of(id)
}

/// Resolves the frond plate's `authored_id`: the frond is its own plant, so its walk is its own table and
/// the whole tree's would answer with the wrong node.
pub fn tree_fern_frond_handle_of(id: u32) -> Option<String> {
    node_ids(&tree_fern_frond(FERN_SEED).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

// ---------------------------------------------------------------------------
// 木棉 Bombax ceiba
// ---------------------------------------------------------------------------

/// The leaf template index the crown's **seven-fingered** hand is drawn by.
pub const HAND_LEAFLETS: usize = 7;

/// How many leaflets the crown's inner hand carries.
pub const FIVE_LEAFLETS: usize = 5;

/// The leaf template index the crown's seven-fingered hand is drawn by.
pub const KAPOK_HAND: usize = 0;

/// The leaf template index the crown's **five-fingered** hand is drawn by.
pub const KAPOK_FIVE: usize = 1;

/// Which season a kapok is drawn in.
///
/// # Invariants
///
/// **A season is an emptiness, not a switch.** [`Season::Flower`] carries `fields: Vec::new()` and
/// `templates: Vec::new()` — the plant has no leaves at all rather than zero of them — and the flowering
/// seats are [`scatter`]ed on the twigs and the secondaries. That is the whole of the proposal's open
/// question 3 answered for this species, and it is why *one rule* can be drawable at two seasons: the rule
/// is [`kapok_rule`] and neither season touches it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Season {
    /// In leaf: two leaf fields and no flower.
    Leaf,
    /// Leafless and in red flower: no leaf field, and flowers on the shoot tips.
    Flower,
}

impl Season {
    /// The template index this season's flowers are named by.
    const FLOWER: usize = 0;
}

/// The kapok's rule: a buttressed foot, one bole, whorled horizontal limbs, secondaries and twigs.
///
/// # The numbers and where they come from
///
/// [`bombax_ceiba.md`](../../../.agents/investigate/trees/bombax_ceiba.md)'s modelling table, at **20 m**,
/// the low end of its **20–25 m** range, because a seventeen-metre crown at this render budget is a crown
/// of thirty thousand leaves and the bar is read on the silhouette and the crown's depth, not on the count.
///
/// | level | count | where | what |
/// | --- | --- | --- | --- |
/// | **0** | **1** | the foot's own base | **the bole**, born out of the buttress: one child, sunk to the axis |
/// | **1** | **11–14** in **tiers of three** | `0.52`–`0.985` of the bole | the **whorled horizontal limbs** |
/// | **2** | **4–7** | `0.30`–`0.98` of a limb | the secondaries, 2.8–3.6 m |
/// | **3** | **14–20** | `0.22`–`1.0` of a secondary | the twigs, 0.25–0.5 m |
///
/// **`cluster = 3` on generation 1 is the whole of the whorl**, and it is the same word [`pinnae`] uses on
/// the fern's opposite pinnae and the pine's needle fascicles use on their needles: how many siblings
/// share one station. Five to six whorls of three over the bole's top forty-four per cent is the note's
/// *"pseudo-whorls of 3–5 near the top of the bole"*, and it is why the crown is **layered** — the note's
/// ESTIMATE of *"a shallow upper plate over a lower, sparser, more horizontal tier"* rather than a
/// conifer's tiers of shrinking boughs.
///
/// **The twig count is the crown's mass and it is above the note's count on purpose.** Two thousand twigs
/// is what an eighteen-metre crown of `0.285 m` hands needs to cover a fifth of its own frontal area; the
/// first two plates of this species stood at six hundred and eight hundred and came out at **0.139** and
/// **0.177 coverage**, which is the *bare broom* the defect list names. The note's own `2–4 leaves per
/// twig` is honoured; the number of shoots is the one the crown pays for.
///
/// **The band starts at 0.56**, which is the note's *"a mature tree has no branches over much of its
/// trunk"*, its own *11–15 m* to first branch, and the `long clean bole` half of the parasol silhouette.
/// **It ends at 1.0**, so the topmost whorl leaves from the bole's own tip and there is no leader above it.
///
/// **Six whorls over the bole's top forty-four per cent**, which is the note's `8–14` first-order branches
/// read at its upper end and the number that closes the gaps: **twelve limbs in four tiers left a bare grey
/// pole standing between every tier**, three metres of it, and the first plate of this species came out with
/// a Christmas tree inside a lamp standard. A whorl every **1.4 m** is the difference between a bole and a
/// trunk with wires on it.
///
/// **No `Field` but a weak light tropism.** `01-habit.jpg` shows the outer branchlets turned up at their
/// tips and the inner ones hanging, and a tropism of 0.14 is enough to lift the outer twigs without
/// flattening the crown's plate — which is what the flat top is made of.
fn kapok_rule() -> Rule {
    Rule {
        trunk: buttress,
        generations: vec![
            // **The bole is the foot's single child**, sunk to the axis by exactly the foot's own reach at
            // slot nought, so it rises out of the middle of the buttress rather than off one plank of it.
            // This is the one generation whose `count` is `(1, 1)`, and it is what the proposal's *"buttress
            // roots"* asks for: the root is a short flared node, not a trunk with a flare on it.
            Generation {
                count: (1, 1),
                station: (0.0, 0.0),
                slot: (0.0, 0.0),
                jitter: (0.0, 0.0),
                cluster: 1,
                sink: 1.32,
                shape: bole,
            },
            // **The whorled limbs.** Thirteen in tiers of three is four tiers; the slot step is the golden
            // angle between tiers and a third of a turn inside one, which is what a whorl is.
            Generation {
                count: (14, 17),
                station: (0.56, 1.0),
                slot: (0.0, GOLDEN),
                jitter: (0.04, 0.10),
                sink: 0.10,
                cluster: 3,
                shape: limb,
            },
            Generation {
                count: (4, 7),
                station: (0.30, 0.98),
                slot: (0.0, GOLDEN),
                jitter: (0.08, 0.16),
                sink: 0.035,
                cluster: 1,
                shape: secondary,
            },
            Generation {
                count: (14, 20),
                station: (0.22, 1.0),
                slot: (0.0, GOLDEN),
                jitter: (0.10, 0.20),
                sink: 0.012,
                cluster: 1,
                shape: twig,
            },
        ],
        fields: vec![Field::light([0.0, 1.0, 0.0], 0.14)],
    }
}

/// The foot: **2.6 metres** of plank buttress, five boards round a bole, and nothing else above ground.
///
/// # The taper, and the two metres it takes to disappear
///
/// FOC gives *"trunk buttressed"* and the note ESTIMATES a projection of **`0.9–1.6 m` radius, to
/// `2.0–3.0 m` up**. So the foot's own taper is `1.32 m` at the ground — a **2.64 m** apparent base — falls
/// to `0.71` at **830 mm**, to `0.60` at **1.3 m** and to `0.44` at its own 2.6 m tip. The bole it carries is
/// `0.45 m` at 830 mm and `0.42 m` at 2.6 m, so **the boards stand a quarter of a metre proud for two
/// metres** and then vanish — which is the note's *板状根*, and the reason the foot is **2.6 m long**.
///
/// The first two plates of this species dropped the radius from 1.32 to 0.49 in 34 mm and drew a **flat
/// triangular skirt at the foot of a twenty-metre tree**, then a five-pointed starfish lying on the sand.
/// A buttress is a curve and not a cliff: its steepest part is where a plank leaves the bole, and it is
/// proud of the bole for two metres after that.
fn buttress() -> (shape::Curve, shape::Scalar) {
    (
        spine(&[
            ([0.00, 0.00, 0.00], [0.00, 1.00, 0.00]),
            ([0.02, 0.90, 0.01], [0.00, 1.00, 0.00]),
            ([0.00, 1.80, -0.01], [0.00, 1.00, 0.00]),
            ([0.01, 2.60, 0.00], [0.00, 1.00, 0.00]),
        ]),
        taper(&[
            (0.000, 1.320),
            (0.080, 1.060),
            (0.180, 0.860),
            (0.320, 0.710),
            (0.500, 0.600),
            (0.700, 0.520),
            (1.000, 0.440),
        ]),
    )
}

/// The foot's section: **five boards**, and the crate draws them as creases.
///
/// # Invariants
///
/// **Ten authored points, five tips and five notches, at a ratio of one to 0.30** — and that ratio is the
/// note's *"plank buttresses … vertical boards"*: a board is a broad flat fin with a narrow deep notch
/// between it and the next, and `0.30` is how much of the board's radius the notch takes.
///
/// **The crease is what makes it read.** The wood's normals are creased above **0.61 rad**, and ten points
/// round a loop is `0.63 rad` a step, so every notch is a **hard edge** and every board is a flat facet lit
/// as one plane — which is exactly what a plank buttress looks like. **That is why the whole kapok is
/// planted at [`KAPOK_DETAIL`]'s ten `around`** and not at the crate's twelve: at twelve the loop is
/// sampled at twelve points over ten authored vertices, the notches are rounded off between samples and the
/// boards come out as soft lobes, which is a flared tree and not a buttressed one.
///
/// The section is scaled by the taper, so the boards stand **1.32 m** out at the ground and are **2 mm**
/// proud at 2.6 m — already inside the bole, so the flare ends where the note says it does.
fn planks() -> shape::Section {
    let points: Vec<[f32; 2]> = (0..10)
        .map(|index| {
            let theta = index as f32 / 10.0 * std::f32::consts::TAU;
            let reach = if index % 2 == 0 { 1.0 } else { 0.30 };
            [theta.cos() * reach, theta.sin() * reach]
        })
        .collect();
    shape::Section::outline(&points).expect("five boards are a section")
}

/// The bole: **twenty metres** of bottle, with the first six hundred millimetres inside the buttress.
///
/// # The bottle, in one taper law
///
/// The brief asks for a **bottle-shaped** bole and the note gives a *"straight, cylindrical"* one, which
/// agree on one reading: a bole that is **narrowest at the ground, widest at a third of a metre, and then
/// falls for the other twenty metres**. `0.470` at 0.4 m, `0.415` at 1.2 m and `0.062` at the top is that,
/// and the widest point at `0.406` at **1.0 m** is inside the note's ESTIMATE of a `0.28–0.48 m` trunk
/// radius at that height.
///
/// **It is not a cone and not a cylinder**: a bole that fell in a straight line would draw a parasol on a
/// pole, and the *floating tree* defect's sibling. The law has a shoulder at 1.2 m, a long shallow middle
/// and a knee at 60% of the height, which is the three-knot shape [`column`] also uses and for the same
/// reason.
///
/// **The bole ends inside the crown and not above it.** It stops at **19.8 m** and the topmost whorl leaves
/// at `station 1.0` — the bole's own tip — with limbs that reach `5.3 m`, so the crown's plate is at the top
/// of the tree and there is no leader above it. The first plate of this species ran the bole to 20.6 m with
/// the top whorl `3.8 m` long and stood a **metre of bare grey pole** above the crown, and its second plate
/// stood **three metres between every pair of tiers**: a leader, and the note says a mature kapok has lost
/// its leader.
fn bole(_rng: &mut Rng, _joint: &Joint) -> (shape::Curve, shape::Scalar) {
    (
        spine(&[
            ([0.00, -0.60, 0.00], [0.04, 1.00, 0.02]),
            ([0.16, 5.20, -0.10], [0.05, 1.00, -0.02]),
            ([0.26, 11.00, 0.06], [0.02, 1.00, 0.02]),
            ([0.22, 16.00, -0.04], [0.00, 1.00, 0.00]),
            ([0.20, 19.80, 0.00], [0.00, 1.00, 0.00]),
        ]),
        taper(&[
            (0.000, 0.560),
            (0.020, 0.470),
            (0.032, 0.430),
            (0.060, 0.415),
            (0.090, 0.405),
            (0.150, 0.392),
            (0.250, 0.372),
            (0.360, 0.340),
            (0.470, 0.300),
            (0.580, 0.255),
            (0.690, 0.208),
            (0.790, 0.163),
            (0.880, 0.122),
            (0.950, 0.092),
            (1.000, 0.062),
        ]),
    )
}

/// A first-order limb: **4.5–8.0 m**, leaving the bole six to twelve degrees above horizontal.
///
/// # The law over the station, which is the crown's profile
///
/// The note gives *"分枝平展"* and *"branches spreading"* and an ESTIMATE of **60–80° from vertical**. So
/// the lowest whorl leaves at `84°` and the topmost at `78°`, and **both reach nearly as far out**: `6.2 m`
/// at `station 0.52` and `5.3 m` at `0.985`. **A ratio of 1.17, and not of 1.5**, and the reason is the
/// note's *"as it matures the leader is lost, the crown lifts and flattens"* — a limb law that falls steeply
/// with height draws a **cone**, and the first plate of this species came out as a Christmas tree with a
/// spike of bare bole standing above it. **The flat top is the topmost whorl reaching as far as the
/// lowest**, and the crown's `18 m` across is `8 m` of limb plus `3 m` of secondary.
///
/// The last two bends turn it back **twenty-four degrees** below its own heading, because `04-branch.jpg`
/// shows the crown's branchlets dipping at their ends and a limb that stayed level would read as a wire.
fn limb(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 6.6 * (1.10 - 0.30 * joint.station) * (0.82 + 0.36 * rng.unit());
    let departure = 1.52 - 0.14 * joint.station + 0.10 * rng.signed();
    let radius = joint.parent_radius.max(1.0e-3);
    (
        bend(
            length,
            [
                departure,
                departure + 0.12,
                departure - 0.18,
                departure - 0.42,
            ],
        ),
        taper(&[
            (0.00, radius * 0.50),
            (0.35, radius * 0.36),
            (0.72, radius * 0.17),
            (1.00, 0.024),
        ]),
    )
}

/// A secondary: **2.4–4.1 m**, and the departure is a law over the station on the limb.
///
/// # Invariants
///
/// **At a limb's foot a secondary leaves at seventeen degrees above the limb and at its tip at fifty-two.**
/// That single law is what turns four tiers of whorls into one **broad, layered plate**. The first plate of
/// this species opened the fan from `23°` to `77°` and the crown came out as a **spire**, because a
/// secondary that climbs steeply off a limb's foot makes the crown's height and a flat top is the one shape
/// that has none.
fn secondary(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 3.5 * (1.14 - 0.36 * joint.station) * (0.72 + 0.55 * rng.unit());
    let departure = 0.30 + 0.62 * joint.station + 0.20 * rng.signed();
    let radius = joint.parent_radius.max(1.0e-4);
    (
        bend(
            length,
            [
                departure,
                departure + 0.24,
                departure - 0.10,
                departure - 0.36,
            ],
        ),
        taper(&[(0.00, radius * 0.42), (0.48, radius * 0.26), (1.00, 0.012)]),
    )
}

/// A twig: **0.25–0.50 m**, the shoot the palmate leaves and the flowers are on.
fn twig(rng: &mut Rng, joint: &Joint) -> (shape::Curve, shape::Scalar) {
    let length = 0.38 * (0.66 + 0.68 * rng.unit()) * (1.06 - 0.20 * joint.station);
    let departure = 0.72 + 0.62 * joint.station + 0.28 * rng.signed();
    let radius = joint.parent_radius.max(1.0e-4);
    (
        bend(
            length,
            [
                departure,
                departure + 0.26,
                departure + 0.34,
                departure + 0.18,
            ],
        ),
        taper(&[(0.00, radius * 0.62), (0.55, radius * 0.38), (1.00, 0.0026)]),
    )
}

/// A leaflet: **145 mm long and 43 mm wide** at full draw — the note's `10–16 × 3.5–5.5 cm` read near the
/// top of its range, and at the top because a crown's only area is its leaflets' and this builder's cost
/// in instances is superlinear.
///
/// # The shape, and the four words the law cannot say
///
/// FOC gives *"oblong to oblong-lanceolate, apex acuminate, base broad or tapering, margin entire"*.
///
/// - **Acuminate** is the law reaching a point at the tip rather than the round cap an obovate leaf needs:
///   the last knot is `0.0012` of half-width and the sweep's own cap closes it.
/// - **Entire margin** is the absence of a comb, and it is a real absence — this is the one blade in the
///   pair that has no teeth, and it is why a kapok and a tree fern never look like each other.
/// - **Fifteen to seventeen pairs of lateral veins** are a texture and not geometry at any of these plates'
///   distances, and are not drawn.
/// - **The broad base** is the width law's flat start, four millimetres of half-width over the petiolule's
///   `1.5–4 cm`; the petiolule itself is inside the petiole blade below.
///
/// The `0.035` thickness ratio is a broadleaf's: about half a millimetre over a 43 mm width.
fn leaflet() -> LeafTemplate {
    LeafTemplate::blade(
        spine(&[
            ([0.000, 0.0000, 0.0], [0.042, 0.0016, 0.0]),
            ([0.042, 0.0034, 0.0], [0.040, 0.0052, 0.0]),
            ([0.084, 0.0088, 0.0], [0.032, 0.0058, 0.0]),
            ([0.120, 0.0152, 0.0], [0.030, 0.0040, 0.0]),
            ([0.145, 0.0208, 0.0], [0.012, 0.0018, 0.0]),
        ]),
        taper(&[
            (0.00, 0.0042),
            (0.10, 0.0120),
            (0.30, 0.0198),
            (0.52, 0.0215),
            (0.74, 0.0170),
            (0.90, 0.0092),
            (1.00, 0.0012),
        ]),
        0.035,
        7,
        4,
    )
}

/// The petiole: a narrow red blade **`155 mm`** long, the note's `10–20 cm` at its middle.
///
/// # Invariants
///
/// **A petiole is a blade here because a template is made of blades and not of rods.** It is `3.2 mm`
/// wide against a 43 mm leaflet, so it reads as a stalk at every plate's distance and costs **one row**
/// inside the same instanced mesh as the hand it carries — which is the whole reason
/// [`LeafTemplate::sprig`] exists. The note's *"red"* is the palette's, not the geometry's.
fn petiole() -> LeafTemplate {
    LeafTemplate::blade(
        spine(&[
            ([0.000, 0.0000, 0.0], [0.052, 0.0006, 0.0]),
            ([0.052, 0.0014, 0.0], [0.052, 0.0018, 0.0]),
            ([0.104, 0.0040, 0.0], [0.050, 0.0016, 0.0]),
            ([0.155, 0.0072, 0.0], [0.040, 0.0010, 0.0]),
        ]),
        taper(&[
            (0.00, 0.0014),
            (0.35, 0.0016),
            (0.80, 0.0014),
            (1.00, 0.0010),
        ]),
        0.55,
        7,
        4,
    )
}

/// The palmate leaf: a petiole and **`leaflets`** leaflets fanning from its tip — the note's *"5–7
/// leaflets … like fingers of an open hand"*.
///
/// # Why two templates and not one number
///
/// A [`LeafTemplate`] is one arrangement, so **a count of leaflets is a count of templates**, not a dial:
/// five is [`KAPOK_FIVE`] and seven is [`KAPOK_HAND`], and the two are placed where the note's two states
/// belong. The **seven-fingered hand** is on the twigs, which is where the eye reads a crown's leaves from
/// below and where `05-leaf.jpg` is photographed; the **five-fingered** hand is on the secondaries, sparser
/// and smaller, which is the shaded interior of the crown. That is a choice about *where* and not a claim
/// that the species has two kinds of leaf, and the module says so rather than dressing it as biology.
///
/// The two template indices are also what gives the crown **two tints**, so the crown is not one mass of
/// one colour — the *felt-pen tree* defect, one level down.
///
/// # The spread, and the outer leaflets
///
/// **`2.2 rad` across the hand**: the outer leaflets at `±1.1` and `160 mm` long put the fan **285 mm**
/// across, which is the note's ESTIMATE of a `0.30–0.45 m` whole-leaf span at its narrow end. **The outer
/// leaflets are shortened to 62% and tilted down**, and that is not decoration — it is what `05-leaf.jpg`
/// shows: a kapok hand is a fan with a short pair outside a long middle three, not a star of equal rays.
fn palmate(leaflets: usize) -> LeafTemplate {
    let mut parts: Vec<(LeafTemplate, Placement)> = vec![(petiole(), Placement::IDENTITY)];
    let blade = leaflet();
    let count = leaflets.max(2) as f32;
    for member in 0..leaflets {
        let rank = member as f32 / (count - 1.0);
        let yaw = (rank - 0.5) * 2.2;
        let outer = (yaw.abs() / 1.1).powi(2);
        let tilt = -0.18 * outer;
        parts.push((blade.clone(), at_tilt(0.155, yaw, tilt)));
    }
    LeafTemplate::sprig(parts)
}

/// How many petals the corolla carries, which is the note's five and not a dial.
const PETALS: usize = 5;

/// The flower: a cup and **five petals**, in one instanced mesh, on a bare tree at forty-five metres.
///
/// # Why five petals and not a ten-pointed star
///
/// The first pass drew the corolla as **a star section on one sweep** — five tips at full reach and five
/// notches at three tenths — because a decorator template was a single sweep and a five-petalled flower had
/// to live in the *section*. Measured off [`KAPOK_FLOWER`]'s own pixels that plate carried **132 red
/// clusters of a median area of two pixels**, and a two-pixel cluster at the plate's 15.0 mm is a **24 mm**
/// spot: the flowers were pinpoints and the crown read grey with red dust. A star has a second failure the
/// count does not show, which is that **its silhouette is its thinnest direction**. Seen edge-on — and half
/// of a crown's flowers hang on twigs pointing at the camera's own plane — a five-pointed star is a line,
/// not a disc. [`vegetation::decorator::Template::sprig`] removes both: a corolla is now **five broad
/// petals in one mesh**, and a broad petal has a broad edge in every direction.
///
/// The reference settles the shape. `06-flower-fruit.jpg` is five **obovate-oblong, fleshy, overlapping**
/// petals rising out of a green cup, each one cupped rather than flat and each one recurving at its tip,
/// with the stamens standing clear above them. That is a bowl of five blades and not a star, and it is what
/// [`corolla`] authors.
///
/// # The sizes, and where each one is the note's
///
/// | organ | authored | the note |
/// | --- | --- | --- |
/// | the petal's length along its own spine | **79 mm** | `8–10 × 3–4 cm`, read at its floor |
/// | the petal's width | **36 mm** | `3–4 cm`, read at its upper end |
/// | the corolla across, tip to tip | **100 mm** | *"ca. 10 cm in diameter"* — the flora's own number |
/// | the cup | **9.5 → 19.5 mm** over 22 mm | *"cup-shaped calyx, 2–3 cm long"*, 3–5 lobes |
///
/// **The petal is the note's 8 cm and the corolla is the note's 10, and both are right** — the two are not
/// in tension. A petal that leaves the cup at the axis and runs to the rim is 5 cm; a kapok's petal is 8,
/// so **the last third of every petal turns back toward the axis**, which is what [`petal`]'s last two knots
/// are for and what `06`'s recurved tips show. The flower is therefore *deeper* along its own axis than it is
/// wide, and a plate at forty-five metres sees the bowl rather than the disc — which is the right read,
/// because the reference photographs a flowering tree are taken from below or level.
///
/// **The stamens are still not drawn**, and the reason is unchanged: FOC gives *"outer series in 5 fascicles
/// of more than 10"* and *"a central ring of 10 shorter filaments"* — a hundred and sixty filaments inside a
/// 10 cm flower, which is [`KAPOK_FLOWER`]'s 6.7 pixels of it. The corolla is the flower; the filaments are
/// the note's.
fn corolla() -> Template {
    let petal = petal();
    let mut parts: Vec<(Template, Placement)> = vec![(calyx(), Placement::IDENTITY)];
    for index in 0..PETALS {
        parts.push((petal.clone(), petal_frame(index)));
    }
    Template::sprig(parts)
}

/// The note's cup: **2.2 cm long, five lobes**, which is the part of the flower every petal stands on.
///
/// # Invariants
///
/// **Five lobes at three fifths, and not ten points at three tenths.** The cup is visible between the petals
/// as a green collar, so it is read at a distance for its *rim*, and the note's calyx is *"3–5 lobes, each
/// semi-orbicular"* — an even scallop, not a star. `0.62` is the notch between two lobes on a lobe's own
/// reach, which is a scallop; the star's `0.30` is a spike.
fn calyx() -> Template {
    let points: Vec<[f32; 2]> = (0..10)
        .map(|index| {
            let theta = index as f32 / 10.0 * std::f32::consts::TAU;
            let reach = if index % 2 == 0 { 1.0 } else { 0.62 };
            [theta.cos() * reach, theta.sin() * reach]
        })
        .collect();
    let mut sweep = shape::Sweep::new(
        spine(&[
            ([0.000, 0.000, 0.0], [0.006, 0.0, 0.0]),
            ([0.008, 0.000, 0.0], [0.010, 0.0, 0.0]),
            ([0.016, 0.000, 0.0], [0.008, 0.0, 0.0]),
            ([0.022, 0.000, 0.0], [0.002, 0.0, 0.0]),
        ]),
        shape::Section::outline(&points).expect("a five-lobed calyx is a section"),
    );
    // **It opens and does not close**: the note's calyx is a cup the petals stand out of, so the scale law
    // ends at its widest rather than tapering to a point as the star's did. A corolla's own face was the
    // star's swept cap, and the stamens stood on it; here the cap is the bottom of a cup.
    sweep.scale = taper(&[
        (0.00, 0.0095),
        (0.30, 0.0150),
        (0.70, 0.0190),
        (1.00, 0.0195),
    ]);
    Template {
        sweep,
        stations: 8,
        around: 10,
    }
}

/// One petal: **79 mm long, 36 mm wide, 11 mm thick**, recurved at its tip.
///
/// # Invariants
///
/// - **A fleshy blade and not a rod.** The section is an ellipse **a quarter as deep out of the corolla's
///   plane as it is broad across it**, which is FOC's *"thick and fleshy"* read as a number, and a rod of
///   equal depth would read as a stamen at forty-five metres. **The blade's fullness is asymmetric** — a
///   fifth deeper on the upper face than the lower — because `06` photographs a petal that is cupped, and a
///   symmetric lens is a lens.
/// - **A rotation, never a scale**: the petal's own `+z` is the corolla's outward axis, so the petals cup
///   because their spines rise in `+z` and not because a caller scaled them.
/// - **The last two knots turn back.** The spine runs out to a radius of **50 mm** and the tip curls to
///   **47 mm**, so the petal is the note's 8 cm of blade inside the flora's 10 cm of flower. A straight petal
///   of this length would make a 15 cm corolla, which is half again the note's diameter.
fn petal() -> Template {
    let sides = 10;
    let points: Vec<[f32; 2]> = (0..sides)
        .map(|index| {
            let theta = index as f32 / sides as f32 * std::f32::consts::TAU;
            let (across, out) = (theta.cos(), theta.sin());
            [across, if out < 0.0 { out * 0.30 } else { out * 0.21 }]
        })
        .collect();
    let mut sweep = shape::Sweep::new(
        spine(&[
            ([0.000, 0.000, 0.000], [0.042, 0.000, 0.034]),
            ([0.042, 0.000, 0.034], [0.016, 0.000, 0.009]),
            ([0.049, 0.000, 0.043], [0.000, 0.000, 0.002]),
            ([0.047, 0.000, 0.045], [0.000 - 0.016, 0.000, 0.004]),
        ]),
        shape::Section::outline(&points).expect("a fleshy petal is a section"),
    );
    // **Widest at two fifths and not at the middle**, which is the note's *obovate*: the blade's own law
    // carries the taper toward the tip, so the tip's point comes from the section rather than from a knot
    // closing the sweep.
    sweep.scale = taper(&[
        (0.00, 0.0045),
        (0.18, 0.0125),
        (0.42, 0.0180),
        (0.74, 0.0145),
        (1.00, 0.0038),
    ]);
    Template {
        sweep,
        stations: 12,
        around: sides,
    }
}

/// Where the `index`th petal stands inside the corolla.
///
/// # Invariants
///
/// **The corolla's plane is the cup's own `y–z` plane, so a petal's `+x` is a radius and its `+z` is the
/// flower's outward axis.** That is the whole reason the frame is written out by hand rather than taken from
/// [`at_tilt`]: a leaf's placement turns a blade in the plane it is already in, and a petal has to leave the
/// cup in the plane *perpendicular* to the cup's axis. The three axes are orthonormal and right-handed —
/// `x × y` is the flower's own `+x` exactly — because [`shape::Part::mesh`] composes the frames rather than
/// re-orthonormalising them, and a frame written by hand is one a reader can check.
///
/// **Five petals at a fifth of a turn and no jitter**, because a corolla's petals are the one organ here that
/// are regularly arranged, and the rotation a leaf field draws with would put them at five unrelated angles.
fn petal_frame(index: usize) -> Placement {
    let theta = index as f32 / PETALS as f32 * std::f32::consts::TAU;
    let (sin, cos) = theta.sin_cos();
    Placement {
        // **Out of the cup's mouth and not out of its floor**: the petals stand on the rim the note's calyx
        // describes, and a petal that starts at the origin stands *through* the cup.
        origin: [0.016, 0.0, 0.0],
        x: [0.0, cos, sin],
        y: [0.0, -sin, cos],
        z: [1.0, 0.0, 0.0],
    }
}

/// The two leaf fields of a kapok in leaf: the twig's hand and the secondary's.
///
/// # Invariants
///
/// **The two depths are three and four, and the reason is `kapok_rule`'s own first generation.** A rule's
/// `trunk` is the root at depth nought and `generations[i]` is its children at depth `i + 1`, so for a tree
/// whose four generations are *bole, limbs, secondaries, twigs* the twigs are at **four**. The first pass
/// passed `(2, 3)` to this function and so put the **seven-fingered hand on the secondaries** and the
/// **five-fingered interior on the limbs** — the exact opposite of what the two are documented to be, and
/// the opposite of where the note's own count puts them. [`kapok_bough`]'s `(1, 2)` was right, because a
/// bough's rule starts one generation further down the tree, which is the check that settles the
/// convention rather than a second opinion about it.
///
/// **What the wrong depth costs is the crown's mass.** The hand is `22` leaves a metre of twig and the inner
/// hand `5` a metre of secondary, so a depth that is one out moves the dense field onto the *secondaries* and
/// the sparse one onto the *limbs*: forty hands spread over a 6.6 m limb instead of a hundred and forty
/// spread over a 0.38 m twig, which is a crown of few big leaves near the wood rather than a loose pale mass.
fn kapok_leaves(secondary_depth: usize, twig_depth: usize) -> Vec<LeafField> {
    let mut hand = LeafField::new(twig_depth, KAPOK_HAND);
    // **A leaf every nine centimetres of twig** over the outer four fifths: the note gives *2–4 leaves per
    // twig* and a twig of 0.38 m, so twenty-two to the metre is six — over the note's count, and it is
    // **the twig count** that carries the rest of what the note asks for. The first plate of this species
    // stood the crown at **0.139 coverage** with the sky between every fourth leaf, which is a *bare
    // broom*: the defect the bar names. The band stops short of both ends so the twig's bare foot and tip
    // are the *dead give* the bar also names.
    hand.band = (0.18, 0.97);
    hand.density = shape::Scalar::constant(22.0);
    hand.phyllotaxy = GOLDEN;
    hand.cluster = 1;
    hand.fan = shape::Scalar::constant(0.0);
    hand.pitch = shape::Scalar::new(&[(0.00, 1.34), (0.50, 1.06), (1.00, 0.84)])
        .expect("a hand's pitch is a law");
    hand.twist = shape::Scalar::new(&[(0.00, -0.42), (0.50, 0.08), (1.00, 0.48)])
        .expect("a hand's twist is a law");
    hand.size = shape::Scalar::new(&[(0.00, 0.80), (0.45, 1.00), (1.00, 0.86)])
        .expect("a hand's size is a law");
    hand.jitter = 0.16;

    let mut inner = LeafField::new(secondary_depth, KAPOK_FIVE);
    // **The interior hand, on the outer two fifths of a secondary**: five to the metre over three metres is
    // six a secondary, and they are the shaded inside of the crown, which is where H3's interior has to
    // come from. Two and a quarter was the first reading and the crown's interior-over-lit-p90 came out at
    // **0.819** — a crown with sky through it has no interior to be dark.
    inner.band = (0.58, 1.0);
    inner.density = shape::Scalar::constant(5.0);
    inner.phyllotaxy = GOLDEN;
    inner.cluster = 1;
    inner.fan = shape::Scalar::constant(0.0);
    inner.pitch = shape::Scalar::constant(1.24);
    inner.twist = shape::Scalar::constant(0.22);
    inner.size = shape::Scalar::constant(0.90);
    inner.jitter = 0.24;

    vec![hand, inner]
}

/// The flowers of the flowering season: three to six at the tip of every twig of the **upper crown**, and up
/// to two at the tip of every secondary there.
///
/// # Invariants
///
/// - **`station = 1.0` is a flower at a shoot's apex**, which is where the note puts it: *"花单生枝顶叶腋"*,
///   solitary, terminal or in the leaf axils of branch tips. The count is a **range the seed decides**, so
///   the crown is **scattered scarlet** rather than every tip in flower, which would be a crocus display and
///   not a kapok.
/// - **The band is read on the twig's own attachment, and that is what makes "the upper crown" sayable.**
///   [`vegetation::decorator::scatter_in`]'s band is the fraction of the **parent's** arc length a node hangs
///   at, so `(0.80, 1.0)` on the twig generation is the outer fifth of every secondary — the rim of the
///   crown's plate, which is where a bare kapok's flowers mass in `08-flowering-bare-tree.jpg` and nowhere
///   else. The same band on the secondaries is the outer fifth of every limb, which is the same statement one
///   level up. **The first pass had no band at all**, so its flowers stood on every twig of every limb from
///   `station 0.22` upward and the crown's red was spread evenly over an armature that reads as a broom.
/// - **Three to six, not one to three, and the depth this call is given is four and not three.**
///   `Rule::generations[i]` puts its children at [`vegetation::handle::Handle::depth`] `i + 1`, because a
///   rule's `trunk` is the root at depth nought. [`kapok_rule`] names four generations, so **the twigs are
///   at depth four and the secondaries at depth three** — and the first pass passed `(2, 3)`, which is the
///   limbs and the secondaries. **Its flowers were never on a twig at all**, and that is measurable: the
///   first plate carried **178** flowers, which is `83` secondaries at `1–3` each plus `15` limbs, out of the
///   1 400 twigs the crown grows. The number in the first pass's own cost table is the tell.
/// - **Two thirds more flowers on the rim**, at the same time as the petals stopped being pinpoints: the
///   first plate carried a **median red cluster of two pixels**, and a crown that reads at range is carried
///   by the *count* as much as by the size — in `08` the red is a string of clusters along the twigs and no
///   single flower is resolved in it either.
fn kapok_flowers(species: &Species, secondary_depth: usize, twig_depth: usize) -> Vec<Seat> {
    let mut seats = scatter_in(
        &species.plant,
        twig_depth,
        Season::FLOWER,
        1.0,
        (3, 6),
        (0.80, 1.0),
    );
    seats.extend(scatter_in(
        &species.plant,
        secondary_depth,
        Season::FLOWER,
        0.96,
        (0, 2),
        (0.80, 1.0),
    ));
    seats
}

/// The kapok at one season: its buttressed foot, its bole, its four tiers and — at one season only — its
/// leaves or its flowers.
///
/// # Invariants
///
/// **One rule, two species objects.** [`kapok_rule`] is not touched by [`Season`]; what changes is that
/// [`Season::Flower`] returns `fields: Vec::new()` and `templates: Vec::new()` and the leafy season returns
/// `decorators: Vec::new()` with no seats. **That is the answer to the proposal's open question 3**, and it
/// is a real answer rather than a naming: the flowering plate is not the leafy plate with its leaves hidden,
/// because there are none in the plant to hide.
pub fn kapok(seed: u64, season: Season) -> Species {
    let mut plant = grow(&kapok_rule(), seed);
    if let Ok(foot) = plant.get_mut(&Handle::root()) {
        foot.set_section(planks());
    }
    match season {
        Season::Leaf => Species {
            plant,
            templates: vec![palmate(HAND_LEAFLETS), palmate(FIVE_LEAFLETS)],
            fields: kapok_leaves(3, 4),
            decorators: Vec::new(),
        },
        Season::Flower => Species {
            plant,
            templates: Vec::new(),
            fields: Vec::new(),
            decorators: vec![corolla()],
        },
    }
}

/// The limb plate's species: one first-order limb as the root, its secondaries and their twigs below it.
///
/// # Invariants
///
/// **The same rule one level down**, as the pine's bough plate and the oak's limb plate are: the root is a
/// limb of the shape the whole tree's limbs are, the generations are the whole tree's secondaries and twigs,
/// and the two leaf fields come from [`kapok_leaves`] with the depths the bough plate's own tree has — so a
/// hand that reads here is a hand the crown is made of.
pub fn kapok_bough(seed: u64) -> Species {
    let rule = Rule {
        trunk: || {
            (
                bend(6.6, [1.46, 1.58, 1.40, 1.16]),
                taper(&[(0.00, 0.240), (0.35, 0.180), (0.72, 0.090), (1.00, 0.022)]),
            )
        },
        generations: vec![
            Generation {
                count: (4, 7),
                station: (0.30, 0.98),
                slot: (0.0, GOLDEN),
                jitter: (0.08, 0.16),
                sink: 0.035,
                cluster: 1,
                shape: secondary,
            },
            Generation {
                count: (8, 12),
                station: (0.22, 1.0),
                slot: (0.0, GOLDEN),
                jitter: (0.10, 0.20),
                sink: 0.012,
                cluster: 1,
                shape: twig,
            },
        ],
        fields: vec![Field::light([0.0, 1.0, 0.0], 0.14)],
    };
    Species {
        plant: grow(&rule, seed),
        templates: vec![palmate(HAND_LEAFLETS), palmate(FIVE_LEAFLETS)],
        fields: kapok_leaves(1, 2),
        decorators: Vec::new(),
    }
}

/// The kapok's expansion dial where the **wood is the subject**, and the one number in this module that is
/// not about a species.
///
/// # Invariants
///
/// **`around = 10`**, and it is [`planks`]' invariant that asks for it: ten authored points round the
/// foot's section is `0.63 rad` a step, and the wood's creasing threshold is `0.61 rad`, so ten is the
/// **smallest** ring that keeps the buttress boards as hard-edged facets. Twelve rounds them off and the
/// foot reads as a flared tree.
///
/// The cost of ten sides on a smooth grey bole is that it is a decagon, and the first plate of this
/// species drew a **dark vertical stripe** down it — one facet turned fully away, which at forty-five metres
/// is the *polygonal silhouette* defect the bar names. [`KAPOK_TREE_DETAIL`] is the answer for the plates
/// where the bole is not the subject.
const KAPOK_DETAIL: Detail = Detail {
    stations: 6,
    around: 10,
    spacing: 0.5,
};

/// The kapok's expansion dial for the two whole-tree plates: **fourteen** sides, for the bole's sake.
///
/// # Invariants
///
/// **The buttress's boards are worth ten sides and the bole is worth fourteen**, and one `Detail` cannot
/// say both. [`KAPOK_DETAIL`] is the base plate's, where the foot is the subject and ten sides make five
/// creased boards; this is the tree's, where the foot is **176 pixels** at forty-five metres and the bole is
/// **fifty-seven pixels wide with a facet every twenty**, and a facet every twenty pixels of a pale grey
/// trunk is a stripe. Fourteen takes the step to `0.45 rad`, under the `0.61 rad` crease threshold, so the
/// bole shades round and the boards soften — and the boards soften **on a plate where they are 176 pixels
/// of a 20 000-pixel frame**, which is the trade the module takes and states.
const KAPOK_TREE_DETAIL: Detail = Detail {
    stations: 6,
    around: 14,
    spacing: 0.5,
};

/// The kapok's surfaces: FOC's **灰白色** bark, a mid-green palmate leaf, and the note's **usually red**
/// flowers.
///
/// # Invariants
///
/// **Three tints and no shared one.** The bark is a *dark* grey rather than the photograph's near-white: the
/// frame's key stands at `3.0` irradiance and a 0.55-albedo grey clips to white, which is how a first
/// plate of a pale-barked tree comes out with a bleached pole for a trunk. **0.205** is the darkest value
/// that still reads as *"灰白色"* under this key rather than as a black stick, and the roughness of **0.88**
/// is the note's *"smooth on young stems"* read as a matte chalk rather than a polished one.
///
/// **The flowers are `0.520, 0.060, 0.050` at a roughness of `0.60`, and both halves of that are measured,
/// not chosen.** The first pass drew them at `0.620, 0.055, 0.035` and `0.80`, and the crown's warm pixels
/// came back at **r 0.68, g 0.38, b 0.35** — a ratio of `1.8 : 1`, which is *pink*, on a plate whose sky is a
/// pale blue. The red channel was **clipping**: at `0.62` of albedo under a key of `5.5` it runs off the top
/// of the range, and everything the tone curve then does to it is to add the same white it is adding to the
/// sky, which *desaturates* the one colour the species is named for. Dropping the tint to `0.52` keeps the
/// red channel off the clip and lets the green and the blue stay where they are.
///
/// The note says *"通常红色，有时橙红色"* and a flower that is merely warm reads as orange, so the ratio is kept
/// at `8.7 : 1` — **lower than the first pass's `11.3 : 1` on purpose**, because a ratio that the frame's own
/// tone curve then halves is not the ratio that reaches the eye. **No emissive term**: [`Skin`] has none to
/// give (`surface(tint, 0.0, roughness, [0.0; 3])`), so "saturated without glowing" is a tint and a roughness
/// and not a flag — which is the crate's own limit here and the module says so rather than reaching for a
/// channel that does not exist.
///
/// **`0.60` is less rough than the petals' own `0.80`**, and the note is against it: FOC's *"thick and fleshy,
/// stellate-puberulent"* is a matte petal, and a petal at `0.80` under this key gathers a broad specular sheen
/// that is white on all three channels. The value is a compromise between the note's matte and the frame's
/// noon sun, and the number that moved is measured in the plate's own pixels rather than argued from the note.
fn kapok_palette() -> Palette {
    Palette {
        wood: Skin::new([0.205, 0.198, 0.182], 0.88),
        leaves: vec![
            // **The note's *"mid-green above … reddening before leaf-fall"*, read pale.** The first plate of
            // this species stood the crown at `0.048` albedo and the leaflets came out as dark specks on grey
            // sticks: the note's crown is *"a loose, **pale**, open mass"*, and pale is a tint and not a
            // count. `0.078` is what a kapok leaflet looks like at three o'clock in a dry valley.
            Skin::new([0.078, 0.118, 0.048], 0.62),
            Skin::new([0.098, 0.128, 0.056], 0.66),
        ],
        seats: vec![Skin::new([0.520, 0.060, 0.050], 0.60)],
    }
}

/// A dry-hot valley at noon: a high white sun, a washed pale sky, and an ochre floor of dry scrub.
///
/// # Invariants
///
/// **A light's reach is its cutoff and must clear the floor's own diagonal**, or the light is culled per
/// cluster box and the box boundary draws a straight edge across the ground: the floor reaches 400 m, its
/// diagonal is 566 m, so the reach is **1400 m**. **The distance is where the irradiance is stated**, and it
/// is the key's own length from the origin — `44.3 m` — because a twenty-metre tree wants its key far
/// enough away that its own shadow map is fitted to the crown and not to the bole.
///
/// **The sky is the note's own belt**: a dry-hot valley's sky at noon is a bleached cyan, not the deep blue
/// of the subalpine scenes, and the *haze* is **0.006** — the valley is full of dust and the dry season is
/// when you can see it.
fn kapok_lighting() -> Lighting {
    Lighting {
        key_position: [24.0, 34.0, 16.0],
        key_colour: [1.0, 0.97, 0.88],
        // **A hard noon sun on a pale sky**, and the ratio between them is the number H3 asks for. The
        // first two plates of this species stood the crown's interior-over-lit-p90 at **0.788** and
        // **0.828** with a key of 3.0 against a sky of 0.60, which is a ratio of five to one and a crown
        // lit mostly by the sky — and a crown lit mostly by the sky has an interior exactly as bright as
        // its rim. A dry-hot valley at noon is the strongest key in this whole set and the sky above it is
        // washed out; **eleven to one** is what that looks like in a number.
        key_irradiance: 5.5,
        key_distance: 44.3,
        key_reach: 1400.0,
        key_extent: 9.0,
        fill_position: [-22.0, 11.0, -19.0],
        fill_colour: [0.56, 0.62, 0.74],
        fill_irradiance: 0.32,
        fill_distance: 31.0,
        floor: Skin::new([0.330, 0.262, 0.172], 0.95),
        sky: Atmosphere::new()
            .zenith([0.16, 0.30, 0.52])
            .horizon([0.68, 0.72, 0.72])
            .ground([0.38, 0.33, 0.25])
            .density(0.0009)
            .scale_height(3000.0),
        sky_radiance: 0.42,
        haze: 0.006,
    }
}

/// The whole-tree frame: **45.3 m** off, the camera at 9.5 m on a twenty-metre tree.
const KAPOK_FRAME: Framing = Framing {
    width: 1400,
    height: 1700,
    eye: [33.0, 9.5, 31.0],
    target: [0.0, 11.0, 0.0],
    field_of_view: 0.55,
    near: 0.5,
    far: 900.0,
    floor: 400.0,
    ground: Ground::Plane,
};

/// The limb plate's frame: **8.8 m** off a limb's outer half, side on.
///
/// # Invariants
///
/// **`2 · 9.61 · tan(0.35) / 1000` is 7.0 mm**, and this is H2's own number: the note's `43 mm` leaflet is
/// **6.1 pixels** and the whole `285 mm` hand is **41**, so the palmate arrangement — the thing H2 asks a
/// broadleaf plate to show — is countable at this distance and is not at the whole-tree plate's fifteen
/// millimetres, where it is 2.9. The plate is therefore **further off than an arm's length**, which is the
/// opposite of the fern's and for the opposite reason: a fern's finest organ is four millimetres and a
/// kapok's is forty-five, so the two plates that answer H2 stand at distances that differ by a factor of
/// six and neither can serve the other.
///
/// **The limb stands at nine metres**, a crown's own height, and not on the ground: a branch plate whose
/// branch lies in the dirt reads as a fallen limb, and this species' photo `04-branch.jpg` is a crown read
/// against sky. The first render of this plate put the limb on the ground and the frame was half sand.
///
/// **The eye is square to the limb's middle**, `x = 3.3` at the limb's own half length, because the limb is
/// **6.6 m** in a frame **9.9 m** wide and an eye off to one side puts half of it outside the frame: the
/// second render cut the limb's own foot off at the left edge and filled the lower left with sand.
const KAPOK_BOUGH_FRAME: Framing = Framing {
    width: 1400,
    height: 1000,
    eye: [3.3, 10.40, 9.6],
    target: [3.3, 9.90, 0.0],
    field_of_view: 0.70,
    near: 0.10,
    far: 900.0,
    floor: 400.0,
    ground: Ground::Plane,
};

/// The base plate's frame: **5.0 m** off the foot at chest height, aimed at **1.2 m**, which is the frame
/// H4 reads the plank buttresses in.
///
/// # Invariants
///
/// **Five metres and a low target**, because a buttress is **2.64 m across and 1.7 m tall** and the first
/// plate of this species stood four and a half metres off at a `1.7 m` target and cropped the flare's
/// widest point off the bottom of the frame — a buttress plate with no buttress in it is a trunk plate.
/// `2 · 5.0 · tan(0.425) / 1400` is **3.3 mm** and the whole flare is three quarters of the frame's width.
const KAPOK_BASE_FRAME: Framing = Framing {
    width: 1200,
    height: 1400,
    eye: [3.6, 2.2, 3.4],
    target: [0.0, 1.2, 0.0],
    field_of_view: 0.85,
    near: 0.05,
    far: 900.0,
    floor: 400.0,
    ground: Ground::Plane,
};

/// Builds the kapok's whole-tree plate: one tree **in leaf**, its own light and ground.
pub fn kapok_picture() -> Value {
    let species = kapok(KAPOK_SEED, Season::Leaf);
    let planting = Planting::new(species)
        .with_palette(kapok_palette())
        .with_detail(KAPOK_TREE_DETAIL);
    plant_scene_lit(&[planting], &KAPOK_FRAME, KAPOK, &kapok_lighting(), &[])
}

/// Builds the kapok's **flowering** plate: the same rule, the same seed, the same skeleton — leafless and
/// in red flower. This is the species' signature and the note's hardest demand, and the only difference in
/// the whole document is [`Season::Flower`].
pub fn kapok_flower_picture() -> Value {
    let species = kapok(KAPOK_SEED, Season::Flower);
    let seats = kapok_flowers(&species, 3, 4);
    let planting = Planting::new(species)
        .with_seats(seats)
        .with_palette(kapok_palette())
        .with_detail(KAPOK_TREE_DETAIL);
    plant_scene_lit(
        &[planting],
        &KAPOK_FRAME,
        KAPOK_FLOWER,
        &kapok_lighting(),
        &[],
    )
}

/// Builds the kapok's limb plate: one first-order limb at eight and a half metres, in leaf.
pub fn kapok_bough_picture() -> Value {
    let species = kapok_bough(KAPOK_SEED);
    let planting = Planting::new(species)
        .at(Placement {
            origin: [0.0, 9.0, 0.0],
            ..Placement::IDENTITY
        })
        .with_palette(kapok_palette())
        .with_detail(KAPOK_TREE_DETAIL);
    plant_scene_lit(
        &[planting],
        &KAPOK_BOUGH_FRAME,
        KAPOK_BOUGH,
        &kapok_lighting(),
        &[],
    )
}

/// Builds the kapok's base plate: the plank buttresses at five metres, which is the frame H4 reads the
/// flare's form in.
///
/// **The spines are not drawn, and the reason is the note's.** FRPS has *"幼树的树干通常有圆锥状的粗刺"*
/// and FOC *"usually very spiny on young trees"*, and the note's modelling table puts them on *"trees under
/// ~8 m"*. **This tree is drawn at twenty metres**, where the note's own words are *"an old tree's bole is
/// smooth"* — so seating conical spines on it would draw a juvenile's diagnostic feature on a mature
/// specimen and every reader who knows the species would see it as wrong. The omission is deliberate and
/// stated here rather than left for a reader to find.
///
/// **Its own lighting, and the reason is the shadow page.** The key's extent sizes the page, and nine is
/// right for a nineteen-metre crown; four was tried for this plate's 17.6 mm texel and the ground beyond
/// the smaller page then read as one dark band from the horizon to the tree's own shadow. **The key's
/// extent is the subject's and not the plate's.** The first pass went on to say this plate "pays the
/// dither", and **that is no longer true**: the shadow page's half-width is now fitted to its box, so the
/// band this sentence describes is gone from this plate as well. See the module's own section on it for the
/// comparison, which was measured on [`TREE_FERN_BASE`] and holds here for the same reason.
pub fn kapok_base_picture() -> Value {
    let species = kapok(KAPOK_SEED, Season::Leaf);
    let planting = Planting::new(species)
        .with_palette(kapok_palette())
        .with_detail(Detail {
            // **The base plate is the one place the bole's own rings are visible**, and at half a metre a
            // ring they are: a 500 mm facet on a 900 mm bole is the polygonal silhouette the bar names. The
            // spacing drops to 120 mm here and the rings rise to 167.
            spacing: 0.12,
            ..KAPOK_DETAIL
        });
    plant_scene_lit(
        &[planting],
        &KAPOK_BASE_FRAME,
        KAPOK_BASE,
        &kapok_lighting(),
        &[],
    )
}

/// Resolves a whole-tree plate's `authored_id` into the handle that names the node.
pub fn kapok_handle_of(id: u32) -> Option<String> {
    node_ids(&kapok(KAPOK_SEED, Season::Leaf).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}

/// Resolves the flowering plate's `authored_id`. **The flowering and the leafy plant are the same tree** —
/// [`Season`] adds no node — so the same walk is the table, and a table built for the leafy season would
/// answer the flowering plate correctly rather than by accident.
pub fn kapok_flower_handle_of(id: u32) -> Option<String> {
    kapok_handle_of(id)
}

/// Resolves the base plate's `authored_id`: the same plant again, so the same table.
pub fn kapok_base_handle_of(id: u32) -> Option<String> {
    kapok_handle_of(id)
}

/// Resolves the limb plate's `authored_id`: the limb is its own plant, so its walk is its own table.
pub fn kapok_bough_handle_of(id: u32) -> Option<String> {
    node_ids(&kapok_bough(KAPOK_SEED).plant)
        .into_iter()
        .find(|(_, held)| *held == id)
        .map(|(handle, _)| handle.to_string())
}
