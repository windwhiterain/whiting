//! The order the solver visits cells in: highest first, so that a cell's water and its load
//! are both handed on before the cell they arrived from is visited.
//!
//! # Invariants
//!
//! - **It is a total order, and it is the same total order every run.** Descending height
//!   with ties broken by ascending lattice index — a cell that hands its water to a
//!   neighbour of identical height has already been visited, because the receivers descend
//!   or stand still and the index order settles the stand-still. Without the tie-break a flat
//!   riverbed would route differently on different machines, and a landscape that erodes to
//!   a slightly different valley each time it is built is not reproducible.
//! - **The input order is rebuilt from the lattice every round**, not carried over from the
//!   last one. The sort below is stable, so it keeps ties in the order it was given them,
//!   and a permutation carried over would resolve every tie by *last* round's answer.
//! - **The cells with no rock under them are never in it.** They are not visited, so nothing
//!   is handed to them and they hand nothing on; their heights stay the rasteriser's.

/// An order-preserving key for a finite `f32`, with `-0.0` sorting below `+0.0` exactly as
/// [`f32::total_cmp`] orders them.
///
/// The field heights are finite by construction — they are barycentric reads of finite
/// vertices — and a key that did not order every `f32` would leave a NaN to sort into
/// whichever bucket it landed in.
fn ascending_key(height: f32) -> u32 {
    let bits = height.to_bits();
    if bits & 0x8000_0000 == 0 {
        bits | 0x8000_0000
    } else {
        !bits
    }
}

/// Sorts `candidates` by descending height, ties by the order the candidates are given.
///
/// # Invariants
///
/// - **It is a four pass radix sort over the inverted key**, not a comparison sort, and that
///   is a cost decision rather than a taste one: the solve visits every cell once per round
///   and a comparison sort makes each of those rounds `n log n` comparisons over a buffer
///   whose arithmetic is otherwise three multiplies. The radix sort is linear in the cells
///   and stable, and stability is what supplies the tie-break for free.
/// - **Only four passes of eight bits**, because the inverted key is 32 bits wide and a
///   shorter key would leave the high bytes unsorted, which for a height field is exactly
///   where the sign and the exponent live.
pub fn by_height_descending(
    heights: &[f32],
    candidates: &[u32],
    order: &mut [u32],
    scratch: &mut [u32],
) {
    order.copy_from_slice(candidates);
    let mut counts = [0_usize; 256];
    let length = order.len();
    // The two runs are written through an array of borrows rather than by swapping the
    // references themselves, because a swap would need a `split_at_mut` on a pair the borrow
    // checker cannot see both halves of.
    let runs = [&mut *order, &mut *scratch];
    let mut from = 0_usize;
    for pass in 0..4_u32 {
        counts.fill(0);
        let shift = pass * 8;
        for value in runs[from].iter().copied() {
            counts[((!ascending_key(heights[value as usize]) >> shift) & 0xff) as usize] += 1;
        }
        let mut base = 0_usize;
        for slot in counts.iter_mut() {
            let hold = *slot;
            *slot = base;
            base += hold;
        }
        for index in 0..length {
            let at = runs[from][index];
            let slot = ((!ascending_key(heights[at as usize]) >> shift) & 0xff) as usize;
            runs[1 - from][counts[slot]] = at;
            counts[slot] += 1;
        }
        from = 1 - from;
    }
    if from == 1 {
        // Destructured rather than indexed, because an index borrows the whole array and
        // the two runs are exactly the two things that must not borrow each other.
        let [read, write] = &mut { runs };
        read.copy_from_slice(&write[..length]);
    }
}
