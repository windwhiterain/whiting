//! The hall plate as four cacheable stages, one intern per stage.
//!
//! # Invariant
//!
//! A stage's key carries **every** input that can change its output and nothing else, which is
//! the rule [`crate::stages`] is held to and the reason [`parts`] reads its plan's dials back out
//! of the plan's own key rather than taking them again as arguments.
//!
//! | stage | reads | produces |
//! |---|---|---|
//! | [`plan`] | `width`, `depth`, `storeys` | the skeleton and the scene's `ops` |
//! | [`built`] | `plan` | the expanded solids |
//! | [`parts`] | `built` | the identity table and the four packed point lists |
//! | [`document`] | `parts`, `lens`, `aim` | the scene document |

use building::BuildingMesh;
use building::derive::{HALL, Starter};
use gallery::hall;

use crate::slab::{self, Held};

/// Plans a hall at a footprint and a storey count.
pub fn plan(width: f32, depth: f32, storeys: usize) -> u64 {
    let key = slab::key(
        slab::stage::PLAN,
        &[],
        &[slab::bits(width), slab::bits(depth), slab::count(storeys)],
    );
    slab::intern(key, || Held::Plan(hall::document_at(width, depth, storeys)))
}

/// Expands a planned hall into the solids its surfaces are made of.
pub fn built(plan: u64) -> Option<u64> {
    let key = slab::key(slab::stage::BUILT, &[plan], &[]);
    let Some(Held::Plan(building)) = slab::get(slab::stage::PLAN, plan) else {
        return None;
    };
    // The same sentence `gallery::hall` expects with: a plan this style derives expands, and a
    // plan that did not would have been refused by the scene before any of this existed.
    let mesh: BuildingMesh = building
        .build()
        .expect("the starter house expands: its outline is a quad and its roof's rows are paired");
    Some(slab::intern(key, || Held::Built(mesh)))
}

/// The identity table and the packed point lists a document of `built` draws.
pub fn parts(built: u64) -> Option<u64> {
    let key = slab::key(slab::stage::PARTS, &[built], &[]);
    let Some(Held::Built(mesh)) = slab::get(slab::stage::BUILT, built) else {
        return None;
    };
    let built_parents = slab::parents(built);
    let [plan] = built_parents.as_slice() else {
        return None;
    };
    let Some(Held::Plan(building)) = slab::get(slab::stage::PLAN, *plan) else {
        return None;
    };
    let starter = starter_of(*plan)?;
    Some(slab::intern(key, || {
        Held::Parts(hall::parts(&starter, &building, &mesh))
    }))
}

/// Assembles the scene document from parts already made, at a scaled lens and a moved aim.
pub fn document(parts: u64, lens: f32, aim: [f32; 3]) -> Option<u64> {
    let key = slab::key(
        slab::stage::PLATE,
        &[parts],
        &[
            slab::bits(lens),
            slab::bits(aim[0]),
            slab::bits(aim[1]),
            slab::bits(aim[2]),
        ],
    );
    let Some(Held::Parts(parts)) = slab::get(slab::stage::PARTS, parts) else {
        return None;
    };
    let view = hall::hall_view(f64::from(lens), aim.map(f64::from));
    Some(slab::intern(key, || {
        Held::Document(hall::hall_document_over_parts(&parts, &view))
    }))
}

/// The starter the plan under `id` was derived from.
///
/// # Invariant
///
/// **The dial is read back out of the plan's own key**, so the ridge and the extent the parts carry
/// are the ones the plan was planned at without the caller repeating them and having to keep the
/// two consistent. This is what `crate::stages::detail_of` does for the oak's wood.
fn starter_of(id: u64) -> Option<Starter> {
    match slab::parameters(id).as_slice() {
        [width, depth, storeys] => Some(HALL.starter(
            f32::from_bits(*width as u32),
            f32::from_bits(*depth as u32),
            *storeys as usize,
        )),
        _ => None,
    }
}
