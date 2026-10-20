//! The probe: one Whiting edit per run, through a `BufferSession`.
//!
//! # What it prints
//!
//! For one named edit: the session's wall time, the same source compiled fresh from an empty
//! registry and an empty slab, the session's cell events, and whether the session's document and
//! the fresh compile's are the same document. The machinery behind those four readings is
//! `whiting_lichen`'s own; what is here is **the oak's program and its five edits**.
//!
//! # Build the artifacts first
//!
//! ```text
//! cargo run -p whiting-lichen-stages --example generate_link
//! cargo build --release -p whiting-lichen-link
//! cargo run --release -p whiting-lichen --example incremental_probe -- lighting
//! ```

use whiting_lichen::{Edit, print_row, run, usage};

/// The program this probe drives, compiled in.
const PROGRAM: &str = include_str!("alpine_oak.lichen");

/// The program's path, which is what a `link` path and a `base` resolve against.
const PROGRAM_PATH: &str = "crates/lichen/examples/alpine_oak.lichen";

/// The five edits, one per run.
///
/// # Invariant
///
/// Every `from` line appears **once** in [`PROGRAM`], so replacing it moves the one dial it names
/// and no other. The values are positive because the language's lexer reads no negative literal:
/// `-6.0` is a parse error, not a number.
const EDITS: &[Edit] = &[
    Edit {
        name: "lighting",
        from: "key_x = 0.0",
        to: "key_x = 7.0",
    },
    Edit {
        name: "detail",
        from: "stations = 8",
        to: "stations = 9",
    },
    Edit {
        name: "leaves",
        from: "density = 0.0",
        to: "density = 18.0",
    },
    Edit {
        name: "envelope",
        from: "envelope = 0.0",
        to: "envelope = 0.7",
    },
    Edit {
        name: "seed",
        from: "seed = 34",
        to: "seed = 35",
    },
];

fn main() {
    let Some(name) = std::env::args().nth(1) else {
        usage(EDITS);
        return;
    };
    let edit = EDITS
        .iter()
        .find(|edit| edit.name == name)
        .unwrap_or_else(|| panic!("{name} is not one of this probe's edits"));
    let run = run(PROGRAM_PATH, PROGRAM, edit);
    print_row(&run.row);
}
