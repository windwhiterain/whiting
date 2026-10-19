//! Which lights reach which part of the view, from the scene document to the three buffers.
//!
//! # What these prove
//!
//! The assignment is the one part of this effort with no adapter behind it, so it is also the
//! one part that can be checked properly. The cases here are the ones that are easy to get
//! quietly wrong:
//!
//! - a light that reaches no cluster, one that reaches every cluster, and a cluster no light
//!   reaches. An empty result is a correct result: a kind that covers the whole view is in no
//!   cluster by design, so a scene whose lights are only those kinds has empty lists and that
//!   is not a missing assignment.
//! - a light whose volume straddles a slice boundary, and the depth distribution at both ends
//!   of the frustum. The slices divide depth rather than distance, which is only visible as a
//!   difference in where a given distance lands, so it is asserted as one.
//! - a cluster over its cap, where the ranking has to keep the highest contributing lights and
//!   not the first ones to arrive. A light that is first in the document order and dim is
//!   dropped in favour of a brighter one behind it, which is the case a truncation gets wrong.
//! - a grid the scene says nothing about, which has to read as the documented default. A
//!   default that is wrong is a number an author silently inherits.
//!
//! The last is the regression bar for the stage: an all `Point` scene emits the passes it
//! emitted before, with the same buffer sizes, gains three entries that nothing binds yet, and
//! since 2026-10-16 carries a post stage as well — so its pass list has four entries where it
//! had three, and its name says so.

use definition::{Definition, DefinitionIdentifier, Path, Registry, SchemaIdentifier, Value};
use graphics::{ENTRIES, GRAPHICS_SCHEMA, INSTANCE_SCHEMA};
use whiting_gallery::Gallery;
use whiting_scene::CompileContext;
use whiting_scene::CompileOutput;
use whiting_scene::Compiler;
use whiting_scene::lighting::{
    CLUSTER_COUNTS_ENTRY, CLUSTER_INDICES_ENTRY, ClusterLight, Clustering, Clusters,
    LIGHT_RECORDS_ENTRY, LightPool, NO_PAGE, assign,
};
use whiting_scene::standard::{
    Bounds, CLUSTERING, Camera, DEPTH_EXTENTS, LIGHTS_PER_CLUSTER, Projection, WIDTH_EXTENTS,
    example_standard_scene, register_standard_scene_schema,
};
use whiting_scene::{CompileError, STANDARD_SCENE_SCHEMA, StandardScene};

/// A box the clusters are tested against, chosen so its numbers can be read off by hand.
const WIDE: f64 = 16.0;

#[test]
fn an_all_point_scene_emits_one_pass_per_stage_and_a_post_stage() {
    let compiled = compiled(&example_standard_scene());

    let labels: Vec<&str> = graphics::passes(&compiled.graphics.value)
        .expect("the passes read")
        .iter()
        .map(|pass| pass.label().expect("a pass is labelled"))
        .collect();
    // The lighting pass no longer names a light, so its label lost the `key/` that the one
    // light of this scene used to contribute. The shadowing pass still does name one, because
    // it is still one per light that was given a page.
    //
    // **This list gained its fourth entry on 2026-10-16 and so did this name.** The example
    // scene used to declare no `post_processes`, which made it the one document in the tree
    // whose frame was the lighting pass's shaded image — and since `0257706` that image is
    // `rgba16float`, so the frame could not be read back as an eight bit rgba image. The
    // example now grades, and `post/post` is that pass. A test still called
    // `an_all_point_scene_still_emits_the_passes_it_emitted_before` would be claiming a
    // history it no longer has, sitting over an assertion that it emits one more.
    assert_eq!(
        labels,
        [
            "shadow/key/shadowing/triangle",
            "depth/depthing/triangle",
            "lighting/lighting/triangle",
            "post/post",
        ],
        "one pass per mesh stage, one post pass, and none of them named twice over"
    );

    // Every buffer that existed before this stage, at the size it had.
    for (entry, size) in [
        ("triangle_vertices", 72),
        ("shadow/key/shadowing/triangle_instance_data", 76),
        ("depth/depthing/triangle_instance_data", 76),
        ("lighting/lighting/triangle_instance_data", 152),
    ] {
        assert_eq!(
            buffer_bytes(&compiled.instance, entry),
            size as i64,
            "the entry `{entry}` is the size it was"
        );
    }

    // And the three this stage adds: one record of twenty eight floats for the one light —
    // colour, page, position, radius, the light's own projection and the address of its virtual
    // shadow pages — one count per cluster of the default grid, and one row of the default cap per
    // cluster.
    //
    // The record grew by one `vec4` when a light's shadow became a grid rather than a page, and it
    // grew by four rather than by two because a record has to stay a whole number of sixteen byte
    // blocks: the two numbers a virtual lookup needs are eight bytes with eight more of pad beside
    // them whether they are wanted or not.
    let clusters = Clustering::convention().cluster_count() as i64;
    let stride = whiting_scene::lighting::light_record_stride() as i64;
    assert_eq!(
        stride * 4,
        112,
        "the record is twenty eight floats, and the stride is stated rather than written down \
         so the assertion and the packing read one number"
    );
    for (entry, size) in [
        (LIGHT_RECORDS_ENTRY, stride * 4),
        (CLUSTER_COUNTS_ENTRY, clusters * 4),
        (CLUSTER_INDICES_ENTRY, clusters * 64 * 4),
    ] {
        assert_eq!(
            buffer_bytes(&compiled.instance, entry),
            size,
            "the entry `{entry}` is the size the grid and the cap imply"
        );
    }

    assert!(
        compiled.notes.is_empty(),
        "a light per cluster is a scene that dropped nothing, and the post stage it now declares \
         names no exposure, so there is no field-nothing-reads note either: {:?}",
        compiled.notes
    );
}

/// How many passes a scene emits of each stage, which is the whole of what a pass count is.
///
/// # Invariants
///
/// The **lighting** number is the scene's groups of *lit* geometry and nothing else: it is what
/// the clustered pass bought, and it is what the acceptance bar in `shadow_pages.rs` holds
/// still while a light is added.
///
/// The **shadowing** number is the lights the pool bought pages for, times the groups of
/// *shadow casting* geometry, so it is bounded by the pool rather than by the scene's light
/// count: a scene with more lights than pages draws only as many shadow maps as it has pages,
/// and lights the rest unshadowed.
///
/// **The lighting count is not the depthing count**, and the difference is real rather than
/// rounding: a mesh may be drawn in the depthing stage and not in the lighting one, as an
/// occluder is. `tidewater` is the case in the gallery — four groups in the depth prepass and
/// three lit — so a claim that the two are equal is true of twenty-one of the twenty-three
/// scenes the table below holds and wrong about the other two: `hermite_massif`, whose backdrop
/// is shaded and never drawn into the depth prepass, and `tidewater`.
///
/// # What the count is a count of
///
/// **A mesh, not an element and not a building — and a placement, not only a mesh.** Every one of
/// the seven scenes the building arc added draws the same four meshes — `walls`, `roof`, `floors`,
/// `ground` — and their depthing and lighting counts are that number; `shed` draws two more because
/// its `door` and `canopy` are parts standing as their own meshes, and `terrace` is four and not
/// six because its three units' roofs are one mesh named `roofs`. So a mass with five openings in
/// it is still one pass, a wing attached to a host is still one pass, and a slab packed into the
/// mesh that was already there is still one pass.
///
/// **The two scenes that joined this table last are the other half of that rule, and they are a
/// control for each other.** `workshop` and `yard` each draw **six** meshes and emit six groups per
/// mesh stage: `shells`, `roofs`, `floors` and `ground` as everywhere, plus one mesh per part
/// template — `shutters` and `bench` for the workshop, `gate` for the yard. The workshop's five
/// shutters are five placements of **one** mesh and are one pass, while the yard's `stone` — its
/// masonry, packed from `Element::Wall` — is the only reason it is six rather than five, because it
/// is the only scene in this group that draws its walls apart from the masses they enclose.
///
/// **Which is why three changes in that arc are invisible here and were measured not to be
/// defects.** `882c0d8` gave seven scenes a `floors` mesh and the count rose by exactly one, once
/// per stage; `2011be5` cut the ground out from under every plate and changed *which triangles*
/// the one `ground` mesh holds rather than how many meshes there are, so nothing moved; `ed8c83f`
/// cut a window into `house` and a doorway into `yard`, and an opening is a hole in the `walls`
/// mesh rather than a mesh of its own, so again nothing moved. `identity` is the control for the
/// first of those: it gained no `floors` mesh, because `packed()` is a catch-all that already
/// carried `Element::Slab`, and its row is two meshes rather than three.
#[derive(Debug, PartialEq, Eq)]
struct Stages {
    shadowing: usize,
    depthing: usize,
    lighting: usize,
    other: usize,
}

impl Stages {
    fn of(scene: &Value) -> Self {
        let compiled = compiled(scene);
        let passes = graphics::passes(&compiled.graphics.value).expect("the passes read");
        let count = |prefix: &str| {
            passes
                .iter()
                .filter(|pass| {
                    pass.label()
                        .expect("a pass is labelled")
                        .starts_with(prefix)
                })
                .count()
        };
        let stages = (
            count("shadow/"),
            count("depth/"),
            count("lighting/"),
            passes.len(),
        );
        Self {
            shadowing: stages.0,
            depthing: stages.1,
            lighting: stages.2,
            // Everything else: the emission stage and the post processes, neither of which
            // reads a light and neither of which this stage touched.
            other: stages.3 - stages.0 - stages.1 - stages.2,
        }
    }

    /// The passes of the three mesh stages, which is what the clustered swap produced or kept.
    ///
    /// **This is what `total()` used to be, and the name says why it changed on 2026-10-16.** A
    /// whole-scene total stopped meaning "what the swap did to this scene" the moment `studio`
    /// gained a post stage, which was a decision about its frame and had nothing to do with
    /// lights. The mesh stages are the ones the swap replaced, so they are the ones to hold
    /// still.
    fn mesh_total(&self) -> usize {
        self.shadowing + self.depthing + self.lighting
    }
}

#[test]
fn every_gallery_scene_still_emits_the_pass_count_it_emitted_before() {
    // The counts move, and this is where the movement is pinned. Each is a function of the
    // scene's **geometry** — how many groups of mesh — and, for the shadowing stage, of how
    // many pages the pool bought; none is a function of the scene's light count.
    // The list is `Gallery::names()` rather than a list written out here, because a list
    // written out here is a list that goes stale: `tidewater` landed in the gallery and this
    // test carried on covering the six scenes it was written for. A scene added to the
    // gallery now has to be added to the table and cannot be forgotten.
    //
    // **That is what happened, and it took the arc the building crate ran to notice.** The seven
    // rows for `hall`, `hall_large`, `house`, `identity`, `shed`, `terrace` and `wing` are the
    // first record this test has ever held for those scenes, so the name above promises a "before"
    // that for them is today: what these rows buy is the *next* change being visible, and the
    // guard proved it works by staying red for every one of those scenes until this table caught
    // up. Every row that was here before measures exactly as it was written, which is the other
    // half of the finding and the reason nothing in it had to be rewritten.
    //
    // **The two rows for `workshop` and `yard` are the same finding one arc later, and it is why
    // they are here rather than somewhere else.** They were the last two scenes of the nine that
    // `Gallery::build` enumerated and `Gallery::names` did not, so no pass-count change in either
    // was visible here at all — the gate was red for a whole arc over seven scenes and then green
    // over a table that still could not see the last two. The two lists are the same list twice
    // over, so a scene added to one without the other is a scene no suite walks.
    let counted: Vec<(String, Stages)> = Gallery::names()
        .iter()
        .map(|name| ((*name).to_owned(), Stages::of(&Gallery::build(name))))
        .collect();
    let expected: Vec<(&str, Stages)> = vec![
        (
            "dunes",
            Stages {
                shadowing: 1,
                depthing: 1,
                lighting: 1,
                other: 1,
            },
        ),
        // `foldyard` is the widest scene in the gallery — thirteen meshes, one pass each for
        // the depthing and lighting stages — and its shadowing count is larger than either,
        // because a caster is a pass for **each** light that casts one, and two of its three
        // lights cast. The counts are measured rather than predicted, and they say the thing
        // this table exists to say: they are a function of the scene's **geometry** and not of
        // its light count in any other way. This scene has three lights and its depthing and
        // lighting counts are the thirteen another scene with one light and thirteen meshes
        // would have.
        //
        // The enclosure is why thirteen and not sixteen. Its runs were at first separate
        // meshes, one per wall band, and composing each run into a single mesh through
        // `built::push_wall_run` took three of them out without changing a vertex. The
        // shadowing count fell with them, which is the point of recording it: fewer meshes is
        // fewer passes in every stage that draws per mesh, and none of it is visible.
        (
            "foldyard",
            Stages {
                shadowing: 20,
                depthing: 13,
                lighting: 13,
                other: 1,
            },
        ),
        (
            // **This row was `nurbs` until 2026-10-15, and renaming it rather than adding a row is
            // the point.** The scene is the same picture built on the other representation: one
            // patch sheet with a crease down it. Keeping both would be two scenes doing one job,
            // and this file's own comment below is about exactly that failure — a scene added
            // without a row here does not fail loudly, it fails as a list that differs by one entry.
            //
            // The counts are unchanged by the swap and that is worth noticing: the old scene
            // emitted one caster group, one mesh, one light and one post, and so does the new one.
            // What changed is that the crease is now made by one call and the seams beside it are
            // exactly C1 rather than measurably smooth, and **the document it compiles to is the
            // same size**.
            "hermite",
            Stages {
                shadowing: 1,
                depthing: 1,
                lighting: 1,
                other: 1,
            },
        ),
        (
            // The massif drawn on its Hermite surface, with **a backdrop standing behind it**.
            //
            // The backdrop is not decoration. This scene renders against Cycles' own film, which is
            // black, so a hole in the massif and a triangle shaded almost black are the same colour
            // and two hypotheses were disproved by exactly that. The backdrop is emissive, so a hole
            // now reads as a hole: every dark patch in the picture stayed dark when the backdrop
            // appeared, which is what said the artifacts are folded surface and not gaps in the
            // mesh.
            //
            // Two lighting passes and one caster group, because the backdrop casts nothing and is
            // drawn once.
            "hermite_massif",
            Stages {
                shadowing: 1,
                depthing: 1,
                lighting: 2,
                other: 1,
            },
        ),
        (
            // **Four groups and one light, and the four are the same four every building scene in
            // this table draws**: the walls, the roof, the plate and the ground. The shadowing
            // count is the same four because the scene declares exactly one `light_with_radius`
            // named `sun` and one page draws every caster once — the rule this file's header
            // states — so a second lamp would multiply all three mesh counts by the caster set and
            // not by one.
            //
            // The plate is the one change in this scene's history that a pass count could see:
            // `882c0d8` gave it a `floors` mesh and the row went from three groups to four, because
            // before that the plate was packed into a mesh that was already there and nothing drew
            // it. Everything else the building arc did to this scene was invisible here — see
            // `house`, which had all three of them.
            "hall",
            Stages {
                shadowing: 4,
                depthing: 4,
                lighting: 4,
                other: 1,
            },
        ),
        (
            // The same four meshes at a larger footprint, from the same style and the same code.
            //
            // **It is measured rather than copied.** `document_at` is handed only the footprint and
            // the storey count, so the count follows the mesh set rather than the size — which is
            // the claim this pair exists to make, and a copy of the row above would not have made
            // it.
            "hall_large",
            Stages {
                shadowing: 4,
                depthing: 4,
                lighting: 4,
                other: 1,
            },
        ),
        (
            // **Five groups, and the compound is three buildings and a wall.** Its three ranges are
            // three masses, three roofs, the plates under them and the one screen wall, packed into
            // five meshes — and the covering's 652 roll segments ride in the roofs' own mesh, so sixty
            // metres of tiles cost no pass at all. Measured, not predicted.
            "compound",
            Stages {
                shadowing: 5,
                depthing: 5,
                lighting: 5,
                other: 1,
            },
        ),
        (
            // The same four meshes and the same one sun, and **this is the row the building arc's
            // invisible changes are measured against**, because `house` is the one scene that drew
            // the plate (`882c0d8`, three groups to four), had the ground cut out from under it
            // (`2011be5`, no move) and grew a window (`ed8c83f`, no move).
            //
            // The window is an opening cut **into** the `walls` mesh, so it is rows in the id table
            // — `scene_identity_probe` went 845 to 852 when it landed — and not a pass. The ground
            // cut changed which triangles the one `ground` mesh holds rather than how many meshes
            // there are. Neither is a stale number and neither is a defect in the stage: a mesh is
            // what a pass is counted in, and both changes were made inside a mesh that was already
            // being drawn.
            "house",
            Stages {
                shadowing: 4,
                depthing: 4,
                lighting: 4,
                other: 1,
            },
        ),
        (
            // **Two meshes and no plate mesh**, and this is the row that says a pass counts a mesh
            // and not an element. The scene draws its whole building — two storeys, a plate, a
            // doorway, five windows, a shutter and a courtyard wall — as **one** mesh named
            // `building`, beside one `ground`.
            //
            // It is the control for `882c0d8`. The other scenes were each given a `floors` mesh
            // because they pack through `packed_where`, whose predicate names one element kind at a
            // time, while `identity` packs through `packed()`, a catch-all that **already carried
            // `Element::Slab`** before that commit asked for one. Its plate was drawn all along and
            // this row never moved by one, because the plate was never a mesh of its own — the same
            // reason the ground cut and the openings moved nothing anywhere.
            "identity",
            Stages {
                shadowing: 2,
                depthing: 2,
                lighting: 2,
                other: 1,
            },
        ),
        (
            // One sun, one point light and three groups: the plain and the **two ranges** it runs
            // between, each drawn in two stages. Three shadowing passes rather than one per light:
            // the plain is a flat sheet that cannot occlude itself, so it is left out of the cast
            // and spends none of the shadow page's extent on ground that has nothing to say.
            //
            // The count moved from two to three because the massif is **stood twice**, once for
            // each wall of the valley, and a placement names a mesh by name: two ranges drawn from
            // one geometry are still two meshes and still two passes in every stage. The rise is
            // the scene's own shape and not a change in how a pass is counted.
            "landscape",
            Stages {
                shadowing: 2,
                depthing: 3,
                lighting: 3,
                other: 1,
            },
        ),
        (
            "lanterns",
            Stages {
                shadowing: 9,
                depthing: 3,
                lighting: 3,
                other: 2,
            },
        ),
        (
            "monoliths",
            Stages {
                shadowing: 2,
                depthing: 2,
                lighting: 2,
                other: 1,
            },
        ),
        (
            // **Six groups, and the two beyond the common four are parts.** `walls`, `roof`,
            // `floors` and `ground` are the set every building scene in this table draws; `door` and
            // `canopy` are this scene's own — a leaf and a plate hung on the head of the doorway's
            // bay. Each is authored as its own mesh, so each is one pass in every stage.
            //
            // The plate took this row from five to six in `882c0d8`. The door and the canopy were
            // here before that commit, from the facade phase, and they are the reason the row is six
            // rather than four. The canopy is hung on the doorway's **socket** rather than pinned
            // to numbers, so moving the head moves it by exactly the same amount — and neither
            // move changes what the frame contains, which is why the count is a plain six.
            "shed",
            Stages {
                shadowing: 6,
                depthing: 6,
                lighting: 6,
                other: 1,
            },
        ),
        (
            // **Five groups, and the fifth is the one this scene exists for.** The four every
            // building scene draws are the walls, the roof, the plate and the ground; `stilt` adds
            // **the posts**, which are the only class of solid in the gallery that stands under a
            // floor and outside a wall ring. Measured, not predicted: the row reads the count a
            // scene with five meshes and one light emits, five in every mesh stage, and the shadowing
            // count is five rather than five times a caster set because one page draws every caster
            // once.
            "stilt",
            Stages {
                shadowing: 5,
                depthing: 5,
                lighting: 5,
                other: 1,
            },
        ),
        (
            // **Four groups, and the street is five houses that touch.** A frontage's plots become
            // masses, and five masses, five roofs, five floors and the ground are four meshes — the
            // same four a single house costs, because the row is what a mass list is for. The tiles
            // ride in the roofs' own mesh and cost nothing, as they do in the compound.
            "street",
            Stages {
                shadowing: 4,
                depthing: 4,
                lighting: 4,
                other: 1,
            },
        ),
        (
            // Four groups and three buildings, and **the plural in `roofs` is the whole of it**:
            // the terrace is three units that touch, and their roofs are packed into one mesh, so
            // the count is four and not six. Packing two things into one mesh is a saving this
            // count can see, while cutting a hole in a mesh is not — the two halves of the rule
            // this file's table is built to hold still.
            //
            // The plate took the row from three to four in `882c0d8`. This scene and `wing` are the
            // two that author their plate over the footprint rather than over the inner loop, which
            // is what lets `2011be5`'s ground cut reach the outer face exactly here.
            "terrace",
            Stages {
                shadowing: 4,
                depthing: 4,
                lighting: 4,
                other: 1,
            },
        ),
        (
            // Four groups and an L, and **the wing costs nothing**: it is attached to the host mass
            // rather than stood beside it, so `walls` holds the host and its wing and `roof` holds
            // both roofs. The scene's own naming of the two arms is in the id table rather than in
            // the pass list — the mesh-not-element rule `identity` records.
            //
            // The plate took the row from three to four in `882c0d8`, on the same terms as every
            // other scene in this group and for the same single reason.
            "wing",
            Stages {
                shadowing: 4,
                depthing: 4,
                lighting: 4,
                other: 1,
            },
        ),
        (
            // **Six groups, and the two beyond the common four are parts.** `shells`, `roofs`,
            // `floors` and `ground` are the set every building scene in this table draws, and this
            // scene adds **one mesh per part template**: `shutters` and `bench`.
            //
            // **The five shutters are one pass and not five, and that is the whole of this row.**
            // Each of the facade phase's five window bays holds a placement, and each placement
            // names its own material, because the offset that makes one shutter's pixels answer
            // its own elements lives on the row the instance names — but the emitter groups by
            // shader, mesh and images, and all five name the same three shaders over the same
            // `shutters` mesh, so they land in **one pass over one instance buffer**. `shed` is
            // the control and reads the other way: its `door` and `canopy` are two placements of
            // two *different* meshes, which is why it is six as well. **A pass counts a
            // shader-and-mesh pair, not a placement and not an element** — the rule `terrace` shows
            // from the other side, where three roofs are one mesh.
            //
            // The bench is the inner-face scene's part and costs exactly the one mesh it is: one
            // placement of one template, standing on `FaceRef::inner`, drawn in the same three
            // stages as the shutters it stands beside and inside.
            //
            // The shadowing count is the same six, and **the pool is the bound that holds it
            // there**: the scene declares exactly one `light_with_radius` named `sun`, which buys
            // **one page rather than a grid**, so the rule this file's header states gives one
            // shadowing pass per caster group — and all six cast, the bench standing inside the
            // building included. `spotlight` is that rule against eighty five pages and one caster
            // group, and it is the only row in this table where the light rather than the
            // geometry sets the number.
            //
            // Nothing else about the scene shows here: the six openings cut into its front are
            // holes in the one `shells` mesh, so `scene_identity_probe` moved when they landed and
            // this row did not.
            "workshop",
            Stages {
                shadowing: 6,
                depthing: 6,
                lighting: 6,
                other: 1,
            },
        ),
        (
            // **Six groups, and two of the six are this scene's own.** `shells`, `roofs`, `floors`
            // and `ground` are the common four; the yard adds **`stone`**, the site's masonry
            // packed from `Element::Wall`, and one `gate`.
            //
            // `stone` is why the yard is six and no building scene beside it is. It is the only
            // scene in this table that draws its walls as a mesh of their own: the site's five
            // seated wall runs are a separate element from the two masses they enclose, so they
            // pack separately, where `shed`'s doorway wall is part of the one `walls` mesh and
            // costs nothing. **A pass counts a mesh and not a role**, and the yard is the row that
            // says a wall is a role too. `workshop` reaches the same six a different way — a
            // second part mesh rather than a second kind of surface.
            //
            // The gate is the facade phase's part, hung on the run that stops at the opening, and
            // it costs **one group for one placement of one mesh** — `shed`'s rule again. The
            // shadowing count is the same six because the scene declares one
            // `light_with_radius` named `sun` and it buys one page: one shadowing pass per caster
            // group, and all six cast, the masonry and the gate included.
            //
            // **And the row is a control for `ed8c83f`**, which cut this scene's doorway: the
            // opening is a hole in a mesh that was already being drawn, so the identity table went
            // 104 to 108 and this row stayed at six. `house` records the same pair of moves on its
            // own window, and these two scenes are the only rows in this table that had one.
            "yard",
            Stages {
                shadowing: 6,
                depthing: 6,
                lighting: 6,
                other: 1,
            },
        ),
        (
            // **Four groups, and a 一颗印 is four ranges that touch.** Three two-storey ranges and a
            // gate house round a three-metre court: four masses, four roofs, four intermediate floors
            // and the ground, packed into four meshes — the same four a single house costs, and the
            // same four the compound pays for three buildings.
            "yikeyin",
            Stages {
                shadowing: 4,
                depthing: 4,
                lighting: 4,
                other: 1,
            },
        ),
        (
            // **The same document from inside its court**: one expansion, two cameras, so the passes are
            // the passes — and the row is here because the scene list is ordered by name and the count is
            // per scene rather than per document.
            "yikeyin_court",
            Stages {
                shadowing: 4,
                depthing: 4,
                lighting: 4,
                other: 1,
            },
        ),
        (
            // The one gallery scene whose light is a spot, so the one whose shadow is now a
            // **grid of pages** rather than one page — and the count is what that costs. Its lamp
            // reaches a cone seventeen and a half world units across at the far plane, which at
            // the pool's own resolution is a grid eight pages per side, and the whole pyramid of
            // four levels is eighty five pages. Every one of them emits a pass per caster group
            // that covers it, so eighty five pages and four groups come to two hundred and
            // forty two, against four when the light was one page.
            //
            // It is recorded here rather than in a comment on the emitter because it is the cost
            // of the design and not a bug in this scene: the alternative to a page per region is
            // one texel size for a coin and a wall standing in the same cone, which is the thing
            // the virtual map exists to refuse. A scene that wants the old count asks for it by
            // naming a lower `shadow_density`, and the pages come down with it.
            //
            // It was 242, and 179 until a caster stopped being drawn into every level from its own
            // down. `SPEC/render/virtual_shadow.md` puts each instance in the one layer its
            // required resolution buys and nothing else, so the copies at the coarser levels are
            // gone and with them the passes that drew them. A caster copied four ways is recorded
            // four times over and its finest copy answers for a receiver standing far from it.
            "spotlight",
            Stages {
                shadowing: 123,
                depthing: 4,
                lighting: 4,
                other: 1,
            },
        ),
        (
            // Eleven lights and eight groups. The shadowing count is the one that moved: a
            // light with a page costs a pass per group, which is what a scene pays for keeping
            // a window's softness as a grid of point lights.
            "stilllife",
            Stages {
                shadowing: 88,
                depthing: 8,
                lighting: 8,
                other: 1,
            },
        ),
        (
            "studio",
            // `other` is one and not zero since 2026-10-16, and the change is the example's
            // post stage rather than anything about lights: the scene declares one
            // `post_processes` entry so that its frame is a display image rather than the
            // `rgba16float` shaded image. See `an_all_point_scene_emits_one_pass_per_stage_and_a_post_stage`
            // and `.agents/investigate/a-scene-with-no-post-stage-has-no-frame-to-read.md`.
            Stages {
                shadowing: 1,
                depthing: 1,
                lighting: 1,
                other: 1,
            },
        ),
        (
            "sunlight",
            // A floor and four boxes, and one light. A shadowing pass is one per
            // (light, shader, mesh), so a scene of five drawn meshes under one sun emits five
            // of them — **and the sun counts**. It takes no cluster slot and is in no list, so
            // a light that were wrongly given no page would emit **no** shadowing pass here at
            // all, and the scene would render with no shadow in it and nothing to say so.
            Stages {
                shadowing: 5,
                depthing: 5,
                lighting: 5,
                other: 1,
            },
        ),
        (
            "terraces",
            Stages {
                // 12 to 6, and the derivation is the scene's: the valley had two lamps, both
                // global, and each drew a shadow map per tile of the grid. The sky is no longer
                // a lamp — it is the environment the scene declares — and one lamp over the same
                // grid is six maps. `dunes` and `landscape` lost their rows the same way.
                shadowing: 6,
                depthing: 6,
                lighting: 6,
                other: 1,
            },
        ),
        (
            "tidewater",
            Stages {
                shadowing: 6,
                depthing: 4,
                lighting: 3,
                other: 2,
            },
        ),
        (
            "tiles",
            Stages {
                shadowing: 1,
                depthing: 1,
                lighting: 1,
                other: 1,
            },
        ),
    ];
    assert_eq!(
        counted,
        expected
            .into_iter()
            .map(|(name, stages)| (name.to_owned(), stages))
            .collect::<Vec<(String, Stages)>>(),
        "the passes of each gallery scene by stage, after the clustered pass replaced the \
         per light ones"
    );
}

/// The totals the same scenes emitted before, which is the claim the stage above is replacing.
///
/// Kept as a plain list so the move is legible in one place. `dunes` 6 to 5, `lanterns` 23 to
/// 17, `monoliths` 7 to 7, `studio` 3 to 3, `terraces` 31 to 25, `tidewater` 18 to 15, `tiles` 4
/// to 4. Three of the seven did not move at all, which is the honest result: a scene whose
/// groups are few and whose lights are few gains nothing from the swap, and a scene with the
/// most of both is where the count actually moved.
///
/// **The comparison is on the mesh stages alone, and that is a change of 2026-10-16.** `studio`
/// emits four passes now and emitted three then, because it gained a post stage — a decision
/// about what its frame is, with nothing to do with how lights are assigned. Comparing the whole
/// total would have meant either editing a recorded history that is true or raising the bound,
/// and both would have made this assert something other than what its name says. The clustered
/// swap touched the mesh stages; the post stage did not, so the mesh stages are what this holds
/// still.
#[test]
fn the_totals_shrank_where_the_scene_had_more_lights_than_groups() {
    let before: Vec<(&str, usize)> = vec![
        ("dunes", 6),
        ("lanterns", 23),
        ("monoliths", 7),
        ("studio", 3),
        ("terraces", 31),
        ("tidewater", 18),
        ("tiles", 4),
    ];
    for (name, was) in before {
        let stages = Stages::of(&Gallery::build(name));
        let mesh = stages.mesh_total();
        assert!(
            mesh <= was,
            "{name} emitted {was} mesh passes and emits {mesh}, so the clustered pass never \
             added one. Its {other} emission and post passes are not counted here and never \
             were: the swap this test replaced did not touch them",
            other = stages.other,
        );
    }
}

#[test]
fn a_grid_the_scene_says_nothing_about_is_the_documented_one() {
    let clustering = Clustering::read(&Value::structure([])).expect("a scene with no grid reads");
    assert_eq!(clustering.width_extents, 16);
    assert_eq!(clustering.height_extents, 9);
    assert_eq!(clustering.depth_extents, 24);
    assert_eq!(clustering.lights_per_cluster, 64);
    assert_eq!(clustering.cluster_count(), 3456);

    // One extent on its own, and the rest of the grid every scene gets.
    let written = Value::structure([(
        CLUSTERING.to_owned(),
        Value::structure([(WIDTH_EXTENTS.to_owned(), Value::Int(4))]),
    )]);
    let clustering = Clustering::read(&written).expect("a written extent reads");
    assert_eq!(
        (clustering.width_extents, clustering.height_extents),
        (4, 9)
    );
}

#[test]
fn a_grid_of_nothing_is_refused_rather_than_read_as_no_clusters() {
    let scene = Value::structure([(
        CLUSTERING.to_owned(),
        Value::structure([(DEPTH_EXTENTS.to_owned(), Value::Int(0))]),
    )]);
    let refused = Clustering::read(&scene).expect_err("zero slices is refused");
    assert!(
        matches!(refused, CompileError::EmptyClustering { found: 0, .. }),
        "the refusal names the count: {refused}"
    );

    let cap = Value::structure([(
        CLUSTERING.to_owned(),
        Value::structure([(LIGHTS_PER_CLUSTER.to_owned(), Value::Int(0))]),
    )]);
    assert!(
        matches!(
            Clustering::read(&cap),
            Err(CompileError::EmptyClustering { .. })
        ),
        "a cap of nothing keeps no light at all, which is also refused"
    );
}

#[test]
fn a_light_behind_the_eye_is_recorded_and_reaches_no_cluster() {
    let grid = grid(2, 2, 2, 4);
    // The parallel camera stands at z twelve, so a light at z twenty is eight units behind it.
    let behind = light("behind", [0.0, 0.0, 20.0], 1.0);
    let clusters = assign(&grid, &[behind], &parallel(), &pool()).expect("the assignment runs");

    assert_eq!(clusters.records.len(), 1, "the light has a record");
    assert_eq!(count_of(&clusters, 0), 0.0, "and no cluster holds it");
    assert!(
        clusters.counts.bytes().iter().all(|byte| *byte == 0),
        "every cluster is empty, which is a correct answer rather than a missing one"
    );
    assert!(clusters.overflows.is_empty());
}

#[test]
fn a_light_covering_the_view_is_in_every_cluster() {
    let grid = grid(2, 2, 2, 4);
    let everything = light("everything", [0.0, 0.0, 0.0], 1000.0);
    let clusters = assign(&grid, &[everything], &parallel(), &pool()).expect("the assignment runs");

    assert_eq!(clusters.counts.len(), 8);
    for cluster in 0..8 {
        assert_eq!(count_of(&clusters, cluster), 1.0, "cluster {cluster}");
        assert_eq!(light_at(&clusters, cluster, 0), 0.0);
    }
}

#[test]
fn a_light_off_to_one_side_reaches_no_cluster() {
    let grid = grid(2, 2, 2, 4);
    // A thousand units to the side of a frame eight and a bit wide, at a distance in the middle
    // of the frustum: the failure here is lateral, not one of depth.
    let aside = light("aside", [1000.0, 0.0, -2.0], 1.0);
    let clusters = assign(&grid, &[aside], &parallel(), &pool()).expect("the assignment runs");

    assert_eq!(clusters.records.len(), 1);
    assert_eq!(clusters.counts.len(), 8);
    assert!(
        clusters.counts.bytes().iter().all(|byte| *byte == 0),
        "no cluster of the frame is anywhere near it"
    );
}

#[test]
fn a_volume_straddling_a_slice_boundary_is_in_both_slices() {
    // A parallel view divides depth and distance alike, so its boundaries can be read off: the
    // convention camera frames a box of half width four, stands three box radii back, and
    // therefore has a near plane at five and a far plane at twenty, which four slices divide
    // at 5.0, 8.75, 12.5, 16.25 and 20.0. A light two and a half units across the 12.5
    // boundary is in the slice before it and the slice after, and in neither of the others.
    let grid = grid(1, 1, 4, 4);
    let across = light("across", parallel_at(12.5), 2.5);
    let clusters = assign(&grid, &[across], &parallel(), &pool()).expect("the assignment runs");

    let reached: Vec<usize> = (0..4)
        .filter(|slice| count_of(&clusters, *slice) > 0.0)
        .collect();
    assert_eq!(reached, [1, 2], "the two slices its volume reaches");
}

#[test]
fn the_slices_divide_depth_rather_than_distance() {
    // A perspective view with a near plane at one and a far plane at twenty one puts its slice
    // boundaries at 1.0, 1.31, 1.91, 3.5 and 21.0: the first slice is three tenths of a unit
    // deep and the last is seventeen and a half. A grid that divided the distance instead
    // would put them five apart, so a light two and a half units out would land in its first
    // slice — it is seven and a half per cent of the way along the distances — and here it is
    // in its third. That is the whole of what dividing depth rather than distance means.
    let grid = grid(1, 1, 4, 4);
    let two_and_a_half_out = light("out", perspective_at(2.5), 0.1);
    let clusters =
        assign(&grid, &[two_and_a_half_out], &perspective(), &pool()).expect("the assignment runs");

    let reached: Vec<usize> = (0..4)
        .filter(|slice| count_of(&clusters, *slice) > 0.0)
        .collect();
    assert_eq!(reached, [2], "two thirds of the way along the depth range");
}

#[test]
fn a_light_at_each_plane_lands_in_the_slice_at_that_plane() {
    let grid = grid(1, 1, 4, 4);
    let near = light("near", perspective_at(1.0), 0.1);
    let far = light("far", perspective_at(21.0), 0.1);
    let clusters =
        assign(&grid, &[near, far], &perspective(), &pool()).expect("the assignment runs");

    assert_eq!(
        count_of(&clusters, 0),
        1.0,
        "the near plane is the first slice"
    );
    assert_eq!(count_of(&clusters, 3), 1.0, "the far plane is the last one");
    assert_eq!(count_of(&clusters, 1), 0.0);
    assert_eq!(count_of(&clusters, 2), 0.0);
}

#[test]
fn a_cluster_over_its_cap_keeps_the_highest_contributing_lights() {
    // One cluster, and two lights that reach it. The first in document order is dim and sits
    // its radius away from the box, so it is the weakest light in the cluster; the second is
    // brighter and stands inside it. A cap of one has to keep the second.
    let grid = grid(1, 1, 1, 1);
    let dim = ClusterLight {
        label: "dim".to_owned(),
        position: [WIDE / 2.0 + 4.0, 0.0, -0.5],
        colour: [0.5, 0.5, 0.5],
        radius: 4.0,
        shadow_projection: identity(),
        clustered: true,
        global: false,
        casts_shadow: true,
        virtual_shadow: false,
        virtual_page: [NO_PAGE; 4],
    };
    let bright = ClusterLight {
        label: "bright".to_owned(),
        position: [0.0, 0.0, -0.5],
        colour: [1.0, 1.0, 1.0],
        radius: 4.0,
        shadow_projection: identity(),
        clustered: true,
        global: false,
        casts_shadow: true,
        virtual_shadow: false,
        virtual_page: [NO_PAGE; 4],
    };
    let clusters =
        assign(&grid, &[dim, bright], &parallel(), &pool()).expect("the assignment runs");

    assert_eq!(count_of(&clusters, 0), 1.0, "the cap is one light");
    assert_eq!(
        light_at(&clusters, 0, 0),
        1.0,
        "the brighter light is kept and the first one to arrive is not"
    );

    // And the author is told, in one note, with the count and the name of what went missing.
    assert_eq!(clusters.lights_dropped, 1);
    assert_eq!(clusters.overflows.len(), 1);
    assert_eq!(clusters.overflows[0].reached, 2);
    assert_eq!(clusters.overflows[0].dropped, ["dim".to_owned()]);

    // **One** note, not one per overflowing cluster: the cluster is an interior of a grid the
    // author recomputes, and the label is the string they go and edit.
    let notes = clusters.notes(&grid, &Path::root().key(CLUSTERING));
    assert_eq!(
        notes.len(),
        1,
        "one note for the whole view, whatever overflowed"
    );
    assert!(
        notes[0].message.contains("1 of the 1 clusters")
            && notes[0].message.contains("1 of the light entries"),
        "the note counts: {}",
        notes[0].message
    );
    assert!(
        notes[0].message.contains("`dim`"),
        "and it names the light that was dropped, in backticks: {}",
        notes[0].message
    );
    assert!(
        notes[0].path.to_string().contains(LIGHTS_PER_CLUSTER),
        "and it points at the field the author changes: {}",
        notes[0].path
    );
}

#[test]
fn a_light_that_covers_the_view_takes_no_cluster_slot() {
    // The exemption, and now two of them: a light that reaches everywhere is in no cluster by
    // design, whether it got there by naming no reach or by naming one that holds the whole
    // scene. A scene of nothing but those emits a full record buffer and a wholly empty pair
    // of lists.
    let grid = grid(1, 1, 2, 4);
    let sun = ClusterLight {
        label: "sun".to_owned(),
        position: [0.0, 0.0, -0.5],
        colour: [1.0, 1.0, 1.0],
        radius: 1.0,
        shadow_projection: identity(),
        clustered: false,
        global: true,
        casts_shadow: true,
        virtual_shadow: false,
        virtual_page: [NO_PAGE; 4],
    };
    let clusters = assign(&grid, &[sun], &parallel(), &pool()).expect("the assignment runs");

    assert_eq!(
        clusters.records.len(),
        1,
        "its record is still there for stage 3b"
    );
    assert!(clusters.counts.bytes().iter().all(|byte| *byte == 0));
    assert!(
        clusters
            .notes(&grid, &Path::root().key(CLUSTERING))
            .is_empty(),
        "nothing was dropped, so there is nothing to report"
    );
}

/// Returns the bytes a named buffer entry carries.
fn buffer_bytes(instance: &Definition, entry: &str) -> i64 {
    entries_of(instance)
        .get(entry)
        .unwrap_or_else(|| panic!("the entry `{entry}` is emitted"))
        .variant_fields()
        .and_then(|fields| fields.get(graphics::binders::SIZE_BYTES))
        .and_then(Value::as_int)
        .unwrap_or_else(|| panic!("the entry `{entry}` is a buffer"))
}

/// Returns the instance entries a compile emitted.
fn entries_of(instance: &Definition) -> std::collections::BTreeMap<String, Value> {
    instance
        .value
        .as_field_map()
        .and_then(|fields| fields.get(ENTRIES))
        .and_then(Value::as_dict)
        .expect("an instance holds its entries")
        .clone()
}

/// Returns the grid these tests use, which is small enough to read.
fn grid(width: usize, height: usize, depth: usize, cap: usize) -> Clustering {
    Clustering {
        width_extents: width,
        height_extents: height,
        depth_extents: depth,
        lights_per_cluster: cap,
    }
}

/// Returns a pool big enough that no test here runs out of pages, so the assignment tests
/// measure the assignment rather than the pool.
///
/// The tests that are about the pool pass their own.
fn pool() -> LightPool {
    LightPool {
        page_width: 1024,
        page_height: 1024,
        pages: 64,
    }
}

/// Returns a point light with the reach the tests need.
///
/// Every light these tests build is drawn on the **pool**, which is the path the page ranking
/// belongs to: `virtual_shadow` false is what says so, and it is the flag rather than the
/// sentinel address that says it, because a light with no address could be either a pool light or
/// a virtual one whose camera saw nothing.
fn light(label: &str, position: [f64; 3], radius: f64) -> ClusterLight {
    ClusterLight {
        label: label.to_owned(),
        position,
        colour: [1.0, 1.0, 1.0],
        radius,
        shadow_projection: identity(),
        clustered: true,
        global: false,
        casts_shadow: true,
        virtual_shadow: false,
        virtual_page: [NO_PAGE; 4],
    }
}

/// Returns the identity matrix, which is the projection a light no shadow is looked up
/// through carries.
///
/// The assignment does not read it and the packing only writes it, so a test that is about
/// which lights reach which clusters has nothing to say about a projection — but the record
/// has to carry one, and a light with no page is never looked up through it.
fn identity() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
}

/// Returns how many lights a cluster kept.
fn count_of(clusters: &Clusters, cluster: usize) -> f64 {
    clusters
        .counts
        .component(cluster, "count", 0)
        .expect("every cluster has a count")
}

/// Returns the light in one slot of one cluster.
fn light_at(clusters: &Clusters, cluster: usize, slot: usize) -> f64 {
    let cap = clusters.indices.len() / clusters.counts.len();
    clusters
        .indices
        .component(cluster * cap + slot, "light", 0)
        .expect("every cluster has a row of the cap")
}

/// Returns a parallel camera: the convention one, framing a box of half width four.
///
/// Its eye stands at z twelve looking at the origin, its frame is eight and a bit wide each
/// way, and its depth range is five to twenty — so a world position is a distance from the eye
/// by subtracting its z from twelve, which is what the two helpers below do.
fn parallel() -> Projection {
    Camera::read(None, &Path::root(), &bounds(), 1.0)
        .expect("the convention camera reads")
        .projection()
}

/// Returns a world position a given distance in front of the parallel camera.
fn parallel_at(distance: f64) -> [f64; 3] {
    [0.0, 0.0, 12.0 - distance]
}

/// Returns a perspective camera with its near plane at one and its far plane at twenty one,
/// its eye standing at z ten.
fn perspective() -> Projection {
    let triple = |v: [f64; 3]| Value::List(v.into_iter().map(Value::Float).collect());
    Camera::read(
        Some(&Value::structure([
            ("eye".to_owned(), triple([0.0, 0.0, 10.0])),
            ("target".to_owned(), triple([0.0, 0.0, 0.0])),
            ("projection".to_owned(), Value::variant("Perspective", [])),
            ("near".to_owned(), Value::Float(1.0)),
            ("far".to_owned(), Value::Float(21.0)),
        ])),
        &Path::root(),
        &bounds(),
        1.0,
    )
    .expect("a written camera reads")
    .projection()
}

/// Returns a world position a given distance in front of the perspective camera.
fn perspective_at(distance: f64) -> [f64; 3] {
    [0.0, 0.0, 10.0 - distance]
}

/// Returns the box the cameras frame, whose half width the tests read their light positions off.
fn bounds() -> Bounds {
    Bounds::around([0.0, 0.0, 0.0], 4.0)
}

/// Compiles a scene the way a host would, and returns what it emitted.
fn compiled(scene: &Value) -> CompileOutput {
    let mut registry = Registry::new();
    registry.put_schema(
        SchemaIdentifier::new(GRAPHICS_SCHEMA),
        graphics::graphics_schema(),
    );
    registry.put_schema(
        SchemaIdentifier::new(INSTANCE_SCHEMA),
        graphics::instance_schema(),
    );
    register_standard_scene_schema(&mut registry);
    let source = Definition::new(
        DefinitionIdentifier::new("clustered.source"),
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        scene.clone(),
    );
    source
        .validate(&registry)
        .expect("the scene validates against the schema");
    StandardScene::new(
        DefinitionIdentifier::new("clustered.graphics"),
        DefinitionIdentifier::new("clustered.instance"),
    )
    .compile(&CompileContext {
        scene: &source,
        previous_graphics: None,
        previous_instance: None,
        registry: &registry,
        camera: 0,
        // No machine behind this compile, so it is made against the floor.
        limits: None,
    })
    .expect("the scene compiles")
}
