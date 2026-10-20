//! The hall probe: one edit of a hall plate per run, through a `BufferSession`.
//!
//! # What it prints
//!
//! Everything `incremental_probe` prints, and one line it cannot: the fingerprint of
//! `gallery::hall::hall()` beside the fingerprint the **baseline** compile published. The four
//! stages are `hall`'s own, so the unedited program must be that scene's document and not merely a
//! document; the pair of lines is what says so, and `0` on the right would mean the composition
//! does not hold.
//!
//! # The four edits, one per run
//!
//! | edit | what it moves |
//! |---|---|
//! | `width` | a plan dial: nothing upstream to skip, so this is the honest ceiling |
//! | `lens` | a presentation dial: plan, build and parts must all be reused |
//! | `whitespace` | trailing spaces on a dial's line: no cell may move |
//! | `same` | the source again: the whole build must be reused |
//!
//! # Build the artifacts first
//!
//! ```text
//! cargo run -p whiting-lichen-stages --example generate_link
//! cargo build --release -p whiting-lichen-link
//! cargo run --release -p whiting-lichen --example hall_probe -- lens
//! ```

use whiting_lichen::{Edit, print_row, run, usage};

/// The program this probe drives, compiled in.
const PROGRAM: &str = include_str!("hall.lichen");

/// The program's path, which is what a `link` path and a `base` resolve against.
const PROGRAM_PATH: &str = "crates/lichen/examples/hall.lichen";

/// The four edits, one per run.
///
/// # Invariant
///
/// Every `from` appears **once** in [`PROGRAM`], so replacing it moves the one dial it names and
/// no other. `whitespace` adds two spaces to a line rather than a line of its own, because the
/// session keys its cells on spans: a blank line would move every span below it and would measure
/// the session rather than the edit. `same` replaces a line with itself, which is the *same source*
/// row and must move no cell at all.
const EDITS: &[Edit] = &[
    Edit {
        name: "width",
        from: "width = 10.0",
        to: "width = 12.0",
    },
    Edit {
        name: "lens",
        from: "lens = 1.0",
        to: "lens = 1.08",
    },
    Edit {
        name: "whitespace",
        from: "depth = 16.0",
        to: "depth = 16.0  ",
    },
    Edit {
        name: "same",
        from: "width = 10.0",
        to: "width = 10.0",
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

    let scene = lichen_stages::fingerprint::of(&gallery::hall::hall());
    println!("hall()           : fp {scene:016x}");
    println!(
        "baseline is hall : {}",
        if run.baseline_print == scene {
            "yes"
        } else {
            "NO"
        }
    );
    print_row(&run.row);
    // **Below the shared row's resolution**: `millis` prints three decimals of a second, so an
    // edit that reuses every cell reads as `0.000 s`, and this is what says how far below.
    println!("session (fine)   : {} µs", run.row.session.as_micros());
}
