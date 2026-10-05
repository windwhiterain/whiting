//! Which shadow pages exist, at what precision, and what happens when they cannot.
//!
//! # What these prove
//!
//! A virtual shadow map is a set of promises, and every one of them is about a number a shader
//! will read far away from the code that produced it:
//!
//! - a caster's density is **kept** at its own distance rather than at the back of the volume,
//!   which is what separates this from a fixed page, and which is why the grid is sized against
//!   the widest place a cone ever gets;
//! - a wide cone and a narrow one at the same reach do not get the same grid, because a texel is
//!   an angle and the world it covers grows with the angle;
//! - a caster is recorded at its own level and at **every level coarser than it**, so a receiver
//!   that finds nothing at a finer level always finds something further down the walk;
//! - a page the camera cannot see is **not allocated**, while a caster the camera cannot see is
//!   **not dropped**, which is the distinction the whole clipping design rests on;
//! - the page table answers with the slot the allocator numbered a page into, cross-checked
//!   against a direct scan of the pages themselves, which is the only place the encoding and the
//!   walk are tested against each other rather than each against itself;
//! - and the three ways an allocation can be asked for more than there is are each an **answer**,
//!   naming the light and the demand, rather than a shadow quietly recorded at a fraction of the
//!   resolution that was asked for.

use whiting_scene::standard::math::{look_at, perspective};
use whiting_scene::vshadow::{
    Allocation, Budget, Caster, Frustum, LightPlan, LightVolume, MAX_PAGES_PER_SIDE,
    MIN_PAGES_PER_SIDE, PAGE_SIZE, Page, Shortfall, allocate, caster_level, pages_at_level,
    unpack_high, unpack_low,
};

/// How far the test lamp's cone reaches, and how wide it opens.
const REACH: f64 = 4.0;
const HALF_ANGLE: f64 = 0.5;

/// A lamp four units above the origin, pointing straight down at it.
///
/// The cone's half angle is written as a field of view and its far plane as the reach, which is
/// the pair an emitter hands a light. `near_far` and `half_tangent` read both back out of the
/// projection, so a test that disagrees with the allocator about them disagrees with the shader.
fn lamp() -> LightVolume {
    LightVolume {
        view: look_at([0.0, REACH, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, -1.0]),
        projection: perspective(2.0 * HALF_ANGLE, 1.0, REACH / 20.0, REACH),
    }
}

/// The world point at one place in the lamp's plane at one distance from it.
///
/// Written out from the lamp's own basis — its right is world x, its up is world minus z, its back
/// is world y — rather than by calling the allocator's own corner builder, so the two cannot be
/// wrong together.
fn place(volume: &LightVolume, ndc: [f64; 2], distance: f64) -> [f64; 3] {
    let [hx, hy] = volume.half_tangent();
    let (near, far) = volume.near_far();
    assert!(
        (near - REACH / 20.0).abs() < 1e-9 && (far - REACH).abs() < 1e-9,
        "the lamp is not the lamp the tests were written against: near {near}, far {far}"
    );
    [
        ndc[0] * hx * distance,
        REACH - distance,
        -ndc[1] * hy * distance,
    ]
}

/// A thing hanging under the lamp, at one place in its plane and one distance from it, asking for
/// a shadow recorded at `density`.
fn caster(handle: u32, ndc: [f64; 2], distance: f64, radius: f64, density: f64) -> Caster {
    Caster {
        handle,
        position: place(&lamp(), ndc, distance),
        radius,
        density,
    }
}

fn plan(casters: Vec<Caster>) -> LightPlan {
    LightPlan {
        label: "over the table".to_owned(),
        volume: lamp(),
        faces: 1,
        casters,
    }
}

fn spendable(clip: Vec<Frustum>) -> Budget {
    Budget {
        atlas_side_pages: 32,
        clip,
    }
}

/// An axis aligned box, as eight corners in the bit order [`Frustum`] documents: the low bit is
/// which side of x, the next which side of y, the top one which side of z.
fn box_frustum(centre: [f64; 3], half: [f64; 3]) -> Frustum {
    Frustum::new(std::array::from_fn(|index| {
        [
            centre[0] + if index & 1 == 1 { half[0] } else { -half[0] },
            centre[1] + if index & 2 == 2 { half[1] } else { -half[1] },
            centre[2] + if index & 4 == 4 { half[2] } else { -half[2] },
        ]
    }))
}

fn in_box(centre: [f64; 3], half: [f64; 3], point: [f64; 3]) -> bool {
    (0..3).all(|axis| (point[axis] - centre[axis]).abs() <= half[axis])
}

/// Reports whether a page's window holds one place in the light's plane.
///
/// The far edge is **excluded**, because that is how a page holds a place in fact: a texel belongs
/// to one page and a walk finds its page by integer division, so the last texel of one page and
/// the first of the next are never both answers. A window read as closed at both ends is a place
/// on a shared edge belonging to two pages at once, which is a claim about the grid rather than a
/// property of it.
fn holds(page: &Page, ndc: [f64; 2]) -> bool {
    (0..2).all(|axis| {
        let low = page.window[axis];
        let high = page.window[axis] + page.window[axis + 2];
        ndc[axis] >= low && ndc[axis] < high
    })
}

/// The finest allocated page covering one place in the plane, found by looking at every page in
/// turn rather than by walking the table.
fn finest_covering(pages: &[Page], ndc: [f64; 2]) -> Option<&Page> {
    pages
        .iter()
        .filter(|page| holds(page, ndc))
        .min_by_key(|page| page.level)
}

#[test]
fn a_grid_is_the_smallest_that_keeps_its_density_and_is_never_rounded_down() {
    let volume = lamp();
    let [hx, _] = volume.half_tangent();
    // The widest place the cone ever gets, which is where a texel is at its largest and therefore
    // where a promised density is hardest to keep.
    let width = 2.0 * REACH * hx;
    let density = 400.0;
    let allocation = allocate(
        &[plan(vec![caster(0, [0.0, 0.0], 2.0, 0.05, density)])],
        &spendable(Vec::new()),
    )
    .expect("a caster with a density sizes the grid");
    let side = allocation.lights[0].pages_per_side;
    let needed = width * density;

    assert!(
        f64::from(side * PAGE_SIZE) >= needed,
        "{side} pages of {PAGE_SIZE} texels is {} across, short of the {needed:.0} the density \
         asks for at the back of the cone",
        side * PAGE_SIZE,
    );
    if side > MIN_PAGES_PER_SIDE {
        assert!(
            f64::from(side / 2 * PAGE_SIZE) < needed,
            "the grid went one doubling further than it had to, which spends pages that buy \
             nothing back"
        );
    }
    assert!(
        side.is_power_of_two(),
        "a grid halves cleanly from level to level only if its side is a power of two"
    );
}

#[test]
fn a_wider_cone_at_the_same_reach_needs_a_finer_grid_than_a_narrow_one() {
    // Two lamps of one reach and one density whose only difference is how wide they open. The wide
    // one has more world at the back of its cone, so keeping the same density costs it more pages
    // — and a grid sized by reach alone, which is what a shadow extent measures, would give the
    // two of them the same answer and leave the wide one coarser than it was promised.
    let narrow = allocate(
        &[plan(vec![caster(0, [0.0, 0.0], 2.0, 0.05, 400.0)])],
        &spendable(Vec::new()),
    )
    .expect("the narrow lamp sizes itself");
    let wide = allocate(
        &[LightPlan {
            label: "wide".to_owned(),
            volume: LightVolume {
                view: lamp().view,
                projection: perspective(1.4, 1.0, REACH / 20.0, REACH),
            },
            faces: 1,
            casters: vec![caster(0, [0.0, 0.0], 2.0, 0.05, 400.0)],
        }],
        &spendable(Vec::new()),
    )
    .expect("the wide lamp sizes itself");

    assert!(
        wide.lights[0].pages_per_side > narrow.lights[0].pages_per_side,
        "narrow grid {} pages wide, wide grid {} pages wide, and they were asked for the same \
         texels per unit over cones of different widths",
        narrow.lights[0].pages_per_side,
        wide.lights[0].pages_per_side,
    );
}

#[test]
fn a_coin_and_a_wall_under_one_lamp_do_not_share_a_shadow_texel() {
    let allocation = allocate(
        &[plan(vec![
            // The coin asks for eight times the density the wall does, which is exactly three
            // doublings, and both stand at the same distance from the lamp so the only difference
            // between them is what each asked for.
            caster(1, [0.3, 0.0], 2.0, 0.05, 400.0),
            caster(2, [-0.3, 0.0], 2.0, 0.4, 50.0),
        ])],
        &spendable(Vec::new()),
    )
    .expect("one lamp can hold two casters of two densities");

    let coin = finest_level(&allocation, 1);
    let wall = finest_level(&allocation, 2);
    assert_eq!(
        coin,
        Some(0),
        "the coin set the lamp's density, so it is level zero"
    );
    assert_eq!(
        wall,
        Some(3),
        "a wall asking for an eighth of the density is three levels coarser and no finer"
    );
    // A texel is a fixed angle of the cone, so at one distance its world width doubles with the
    // level: eight times the density is eight times the resolution, which is the whole claim that
    // a coin and a wall need not share a page's precision.
    assert_eq!(
        1_u32 << (wall.unwrap() - coin.unwrap()),
        8,
        "the two casters asked for densities eight times apart and got levels that are not"
    );
}

/// The finest level one caster was recorded at, or `None` when it was recorded nowhere.
fn finest_level(allocation: &Allocation, handle: u32) -> Option<u32> {
    allocation
        .pages
        .iter()
        .filter(|page| page.casters.contains(&handle))
        .map(|page| page.level)
        .min()
}

#[test]
fn a_caster_is_recorded_at_its_own_level_and_at_every_one_coarser_than_it() {
    let allocation = allocate(
        &[plan(vec![
            caster(1, [0.3, 0.0], 2.0, 0.05, 400.0),
            // Half the finest density, so one level down, and a ball big enough to cover a span of
            // pages at every level it is drawn into.
            caster(2, [-0.2, 0.1], 2.5, 0.3, 200.0),
        ])],
        &spendable(Vec::new()),
    )
    .expect("two casters of two densities size one grid");
    let levels = allocation.lights[0].levels;
    // The grid's own width, read the way [`allocate`] reads it rather than written out, so the
    // level the allocator chose and the level this asserts about are the same number.
    let (_, far) = lamp().near_far();
    let [half_x, _] = lamp().half_tangent();
    let own = caster_level(
        200.0,
        2.0 * far * half_x,
        allocation.lights[0].pages_per_side,
        levels,
    );
    assert_eq!(own, 1, "half the finest density is exactly one level down");

    for level in own..levels {
        assert!(
            allocation
                .pages
                .iter()
                .any(|page| page.level == level && page.casters.contains(&2)),
            "the sparse caster vanished at level {level} of {levels}, so a receiver asking for its \
             shadow there falls off the end of a chain that was supposed to be unbroken"
        );
    }
    assert_eq!(
        finest_level(&allocation, 2),
        Some(own),
        "a caster that asked for half the density was recorded at a level that is not the one its \
         density buys"
    );
    assert_eq!(
        levels, allocation.lights[0].levels,
        "the level a caster is placed on and the grid it is placed in have to be the same depth"
    );
}

#[test]
fn a_caster_that_asked_for_no_density_is_recorded_at_the_coarsest_level_only() {
    let allocation = allocate(
        &[plan(vec![
            caster(1, [0.3, 0.0], 2.0, 0.05, 400.0),
            caster(2, [0.0, 0.0], 2.0, 0.6, 0.0),
        ])],
        &spendable(Vec::new()),
    )
    .expect("a zero density is still a caster");
    let levels = allocation.lights[0].levels;
    assert_eq!(
        finest_level(&allocation, 2),
        Some(levels - 1),
        "a caster that asked for nothing drove a page finer than the root, so its own coarse \
         shadow is being read out of pages somebody else paid for"
    );
    assert_ne!(
        levels, 1,
        "the test means nothing on a grid with one level, where coarsest and finest are the same"
    );
}

#[test]
fn a_page_the_camera_cannot_reach_is_never_allocated_and_a_scene_with_no_camera_clips_nothing() {
    let casters = vec![caster(1, [0.3, 0.2], 3.0, 0.05, 400.0)];
    let whole = allocate(&[plan(casters.clone())], &spendable(Vec::new())).expect("no camera");
    assert!(
        !whole.pages.is_empty(),
        "a scene with no camera in it spends every page the light has"
    );

    // A room four hundred units to one side of anything the lamp touches.
    let elsewhere = allocate(
        &[plan(casters)],
        &spendable(vec![box_frustum([400.0, 200.0, -300.0], [1.0, 1.0, 1.0])]),
    )
    .expect("the lamp is still allocable, it just has nothing worth allocating");
    assert!(
        elsewhere.pages.is_empty(),
        "{} pages were allocated for a lamp the camera has its back to, and no fragment in the \
         frame can read a single one of them",
        elsewhere.pages.len()
    );
    assert_eq!(
        elsewhere.lights[0].slots, 0,
        "the light kept its grid and its table, and filled neither"
    );
    assert!(
        elsewhere.atlas().is_none(),
        "an empty allocation still asked for a texture to be built"
    );
}

#[test]
fn a_frame_that_sees_part_of_a_lamp_allocates_part_of_its_grid() {
    let casters = vec![
        caster(1, [0.0, 0.0], 3.5, 0.05, 400.0),
        caster(2, [0.7, -0.6], 2.0, 0.05, 400.0),
    ];
    let whole = allocate(&[plan(casters.clone())], &spendable(Vec::new())).expect("no camera");
    let clipped = allocate(
        &[plan(casters)],
        &spendable(vec![box_frustum([0.0, 0.5, 0.0], [0.3, 0.2, 0.3])]),
    )
    .expect("the camera sees one corner of the pool of light");

    assert!(
        !clipped.pages.is_empty() && clipped.pages.len() < whole.pages.len(),
        "a camera that sees a corner of the cone paid for {} pages against the whole grid's {}, \
         which is not that corner",
        clipped.pages.len(),
        whole.pages.len(),
    );
    // Every page that survived covers a window the unclipped grid had too, so what clipping does
    // is remove work rather than invent coordinates.
    for page in &clipped.pages {
        assert!(
            whole.pages.iter().any(|earlier| earlier.level == page.level
                && earlier.page_x == page.page_x
                && earlier.page_y == page.page_y),
            "clipping invented page ({}, {}) at level {}",
            page.page_x,
            page.page_y,
            page.level,
        );
        assert!(
            page.casters.contains(&1),
            "the caster the frame is looking at vanished from a page the frame kept"
        );
        // Only the finest grid makes the second claim: a caster's span is padded by a page on
        // every side, because a footprint is the silhouette of a sphere and the geometry that
        // casts is not that sphere, and one page of padding at a coarse level is a wide stretch of
        // the plane.
        if page.level == 0 {
            assert!(
                !page.casters.contains(&2),
                "a page at ({}, {}) level {} is nowhere near the second caster's shadow, and yet \
                 it is being asked to draw it",
                page.page_x,
                page.page_y,
                page.level,
            );
        }
    }
}

#[test]
fn a_caster_the_camera_cannot_see_still_draws_into_the_pages_its_shadow_falls_across() {
    // The near ball is the one the camera cannot see: it hangs just under the lamp, well outside
    // the box, and the cone of rays it blocks reaches the floor the box is looking at. The far
    // ball is inside the box. Both stand on the lamp's own axis, so every page the far one covers
    // the near one covers as well.
    let centre = [0.0, 0.5, 0.0];
    let half = [0.3, 0.2, 0.3];
    let seen = caster(1, [0.0, 0.0], 3.5, 0.05, 400.0);
    let unseen = caster(2, [0.0, 0.0], 0.6, 0.05, 400.0);
    assert!(
        !in_box(centre, half, unseen.position),
        "the test means nothing if the camera can see the caster it is meant to leave out"
    );
    assert!(
        in_box(centre, half, seen.position),
        "the test means nothing if the camera cannot see the caster it is meant to keep"
    );

    let allocation = allocate(
        &[plan(vec![seen, unseen])],
        &spendable(vec![box_frustum(centre, half)]),
    )
    .expect("the frame sees the far caster's patch of floor");
    assert!(
        !allocation.pages.is_empty(),
        "the frame sees something and the allocator spent nothing on it"
    );
    assert!(
        allocation
            .pages
            .iter()
            .any(|page| page.casters.contains(&2)),
        "the caster outside the frame was dropped from every page. Clipping is a property of the \
         page, never of the caster: an object behind the camera still blocks the light, and the \
         shadow it throws falls on geometry the camera is looking at."
    );
}

#[test]
fn the_table_answers_for_a_page_with_the_slot_that_page_was_numbered_into() {
    let volume = lamp();
    let allocation = allocate(
        &[plan(vec![
            caster(1, [0.3, 0.0], 2.0, 0.05, 400.0),
            caster(2, [-0.3, 0.2], 3.0, 0.2, 100.0),
            caster(3, [0.1, -0.4], 1.0, 0.05, 0.0),
        ])],
        &spendable(Vec::new()),
    )
    .expect("three densities under one lamp");
    let map = &allocation.lights[0];
    let [head_low, head_high] = allocation.header(0).expect("the light has a table section");
    assert_eq!(unpack_low(head_low), map.pages_per_side);
    assert_eq!(unpack_high(head_low), map.atlas_side);
    assert_eq!(unpack_low(head_high), map.levels);
    assert_eq!(unpack_high(head_high), map.faces);
    assert_eq!(map.slots as usize, allocation.pages.len());

    for page in &allocation.pages {
        assert!(
            page.page_x < pages_at_level(map.pages_per_side, page.level)
                && page.page_y < pages_at_level(map.pages_per_side, page.level),
            "page ({}, {}) at level {} is off the grid of {}",
            page.page_x,
            page.page_y,
            page.level,
            pages_at_level(map.pages_per_side, page.level),
        );
        assert!(page.slot < map.slots);
        // A point that is not a fraction of the page a power of two divides into, because a
        // page centre is exactly a boundary of every finer grid and a round trip through world
        // space lands a hand's width either side of one. Anywhere else, and the walk and a direct
        // scan of the pages have the same answer or they do not.
        for across in [0.42, 0.61] {
            for down in [0.42, 0.61] {
                let ndc = [
                    page.window[0] + page.window[2] * across,
                    page.window[1] + page.window[3] * down,
                ];
                let expected = finest_covering(&allocation.pages, ndc)
                    .expect("the page holds its own interior");
                let found = allocation
                    .resolve(&volume, 0, place(&volume, ndc, REACH * 0.5))
                    .unwrap_or_else(|| {
                        panic!(
                            "the table lost page ({}, {}) at level {}, which it had just allocated",
                            page.page_x, page.page_y, page.level
                        )
                    });
                assert_eq!(
                    (found.level, found.slot),
                    (expected.level, expected.slot),
                    "the walk answered level {} slot {} where the finest page covering the point \
                     is level {} slot {}, and a receiver gets whichever of these the shader does \
                     not read",
                    found.level,
                    found.slot,
                    expected.level,
                    expected.slot,
                );
                assert!(
                    found.texel[0] < map.atlas_side * PAGE_SIZE
                        && found.texel[1] < map.atlas_side * PAGE_SIZE,
                    "the answer is a texel outside the layer the pass was told to draw into"
                );
                assert_eq!(
                    [found.texel[0] / PAGE_SIZE, found.texel[1] / PAGE_SIZE],
                    [found.slot % map.atlas_side, found.slot / map.atlas_side],
                    "a texel's page is its slot's page, whatever level it came from"
                );
            }
        }
    }
}

#[test]
fn a_point_outside_the_volume_has_no_answer_and_one_inside_it_falls_down_the_chain() {
    let volume = lamp();
    // One small caster well off the axis, so the grid is sparse around it rather than full.
    let allocation = allocate(
        &[plan(vec![caster(1, [0.8, 0.8], 2.0, 0.02, 400.0)])],
        &spendable(Vec::new()),
    )
    .expect("the caster sizes the grid");
    assert_eq!(
        allocation
            .resolve(&volume, 0, place(&volume, [0.8, 0.8], 2.0))
            .map(|found| found.level),
        Some(0),
        "the caster's own place on the plane was not answered at the level it asked for"
    );

    // A place this caster does not cover has **no** answer, and that is the answer.
    //
    // This used to be asserted the other way round, and the margin is why. The caster stands at
    // `[0.8, 0.8]` with a radius of two hundredths, so its silhouette is a small disc over there
    // and the middle of the cone is nowhere near anything it could shadow; a one page margin at
    // the coarsest level covered the whole of a two by two grid and handed the middle of the cone
    // a page anyway. The assertion said that a receiver far from every fine page falls down the
    // chain and reads something blurred, and it was really asserting that *some page somewhere in
    // the cone* answers — which is not a property the design wants, because a page that recorded
    // nothing is a shadow for a place nothing was drawn into.
    //
    // The property worth having is the other one, and
    // `a_caster_is_recorded_at_its_own_level_and_at_every_one_coarser_than_it` is where it lives:
    // **the caster's own shadow is answered at every level from its own down to the coarsest**, so
    // nothing inside its footprint ever falls off the end of the walk. A point outside that
    // footprint has no caster over it, reads nothing, and is lit — which is what the walk's
    // "a level that recorded nothing does not answer" rule is for.
    assert_eq!(
        allocation
            .resolve(&volume, 0, place(&volume, [0.0, 0.0], 2.0))
            .map(|found| found.level),
        None,
        "the middle of the cone answered, and nothing is drawn there: a page that recorded no \
         caster is a shadow for a place that has no caster"
    );

    // And the two places that genuinely have no page anywhere: outside the cone's own plane, and
    // behind the lamp. Neither is a sparse region — they are outside the light's volume, and an
    // answer for either would be a shadow borrowed from a neighbour.
    assert!(
        allocation
            .resolve(&volume, 0, place(&volume, [1.4, 0.0], 2.0))
            .is_none(),
        "a point outside the cone's plane has no grid to look up and answered anyway"
    );
    assert!(
        allocation
            .resolve(&volume, 0, [-10.0, 10.0, 10.0])
            .is_none(),
        "a point behind the lamp has no shadow to look up and must not borrow one"
    );
}

#[test]
fn a_lamp_with_nothing_to_cast_is_refused_by_name() {
    let error = allocate(&[plan(Vec::new())], &spendable(Vec::new()))
        .expect_err("a shadowing light with no caster has no density to be sized by");
    match error {
        Shortfall::NoCasters { label } => assert_eq!(label, "over the table"),
        other => panic!("the refusal was about the wrong thing: {other}"),
    }
}

#[test]
fn a_density_past_the_grid_ceiling_is_refused_rather_than_recorded_coarser() {
    let error = allocate(
        &[plan(vec![caster(1, [0.0, 0.0], 2.0, 0.05, 100_000_000.0)])],
        &spendable(Vec::new()),
    )
    .expect_err("no grid holds a hundred million texels per unit");
    match error {
        Shortfall::Grid {
            label,
            wanted,
            limit,
        } => {
            assert_eq!(label, "over the table");
            assert_eq!(limit, MAX_PAGES_PER_SIDE);
            assert!(
                wanted > limit,
                "the message claims a demand of {wanted} against a ceiling of {limit}, which is \
                 not a demand at all"
            );
        }
        other => panic!("the refusal was about the wrong thing: {other}"),
    }
}

#[test]
fn an_atlas_too_small_for_the_pages_it_was_handed_is_refused_rather_than_dropping_them() {
    let casters = vec![caster(1, [0.0, 0.0], 2.0, 0.6, 400.0)];
    let roomy = allocate(&[plan(casters.clone())], &spendable(Vec::new())).expect("a wide atlas");
    assert!(
        roomy.pages.len() > 4,
        "the test needs a light that wants more pages than a two by two atlas holds, and it \
         wanted {}",
        roomy.pages.len()
    );
    let error = allocate(
        &[plan(casters)],
        &Budget {
            atlas_side_pages: 2,
            clip: Vec::new(),
        },
    )
    .expect_err("four slots will not hold what this light asked for");
    match error {
        Shortfall::Atlas {
            label,
            wanted,
            limit,
        } => {
            assert_eq!(label, "over the table");
            assert_eq!(limit, 4);
            assert_eq!(wanted as usize, roomy.pages.len());
        }
        other => panic!("the refusal was about the wrong thing: {other}"),
    }
}

#[test]
fn the_same_scene_allocates_the_same_pages_twice() {
    let plans = vec![
        plan(vec![
            caster(1, [0.3, 0.0], 2.0, 0.05, 400.0),
            caster(2, [-0.3, 0.2], 3.0, 0.2, 100.0),
        ]),
        LightPlan {
            label: "in the corner".to_owned(),
            volume: lamp(),
            faces: 1,
            casters: vec![caster(7, [0.0, 0.4], 1.5, 0.1, 300.0)],
        },
    ];
    let budget = spendable(vec![box_frustum([0.0, 1.0, 0.0], [1.5, 1.0, 1.5])]);
    let first = allocate(&plans, &budget).expect("the first compile");
    let second = allocate(&plans, &budget).expect("the second compile");
    // A compile that deltas against its own previous output has to be able to say *nothing
    // changed*, which it can only do if the allocation is reproducible without a seed.
    assert_eq!(first, second);
    assert_eq!(
        first.atlas().expect("two lights, one atlas").layers,
        2,
        "one layer per light with a grid"
    );
}
