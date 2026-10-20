//! The marker an assembler writes, and the sentence a parse failure carries because of it.
//!
//! # What this proves, and what it does not
//!
//! The mapping is arithmetic over a string: a marker says which lines of the assembled module
//! a block occupies, and the reader turns one of those lines back into a file and a line. That
//! is checkable here with no adapter and no scene.
//!
//! It is **not** a render, and it is not a check that a shader compiles. What a device does
//! with the sentence is one line of `parsed_module`, and what a scene's WGSL does to a frame is
//! a different question: the marker is a WGSL comment, and the proof that a comment changes no
//! pixel is a render compared by hash, not a test in this crate.

use whiting_graphics::provenance::{MARKER, explain, mark, origin_of};

/// A block as a scene writes one: indented, opening with a newline, four lines of it.
const BLOCK: &str =
    "\n    let lit = 1.0;\n    let shade = lit * 0.5;\n    color = vec4<f32>(shade);\n";

#[test]
fn a_marked_line_maps_back_to_the_line_of_the_block_it_came_from() {
    // One line of template, the marker, then the block. The block's first line is the line
    // after the marker, so its own first line is `first` and the fourth is `first + 3`.
    let module = format!(
        "@fragment\nfn main() {{\n{}{BLOCK}}}\n",
        mark("surface", "scene.rs", 100, BLOCK)
    );

    let first = origin_of(&module, 4).expect("the block's first line came from the block");
    assert_eq!(first.name, "surface");
    assert_eq!(first.file, "scene.rs");
    assert_eq!(
        first.line, 100,
        "the block was declared to start at line 100"
    );
    assert_eq!(first.block_line, 100);

    let last = origin_of(&module, 7).expect("the block's last line came from the block");
    assert_eq!(
        last.line, 103,
        "four lines of block, so the last is three on"
    );
}

#[test]
fn a_line_the_markers_do_not_cover_is_nothing_at_all() {
    let module = format!(
        "@fragment\nfn main() {{\n{}{BLOCK}}}\n",
        mark("surface", "scene.rs", 100, BLOCK)
    );

    // The template's own lines, the marker itself, and the line after the block closes.
    for line in [1, 2, 3, 8, 9, 0] {
        assert_eq!(
            explain(&module, line),
            String::new(),
            "line {line} is not in a declared block, and a shader that declared nothing is \
             reported exactly as it was before any of this existed"
        );
    }
}

#[test]
fn a_module_with_no_markers_is_reported_exactly_as_it_was() {
    let module = "fn main() {\n    let x = ;\n}\n";
    assert_eq!(explain(module, 2), String::new());
    assert_eq!(origin_of(module, 2), None);
}

#[test]
fn the_marker_is_a_comment_and_the_text_after_it_is_untouched() {
    let module = mark("grade", "scene.rs", 7, BLOCK);
    let (marker, rest) = module.split_once('\n').expect("the marker is one line");

    assert!(
        marker.starts_with(MARKER),
        "it opens with the reserved prefix"
    );
    assert!(
        marker.starts_with("//"),
        "and it is a WGSL comment, so a backend never sees it"
    );
    assert_eq!(
        rest, BLOCK,
        "the block follows the marker with nothing between them"
    );
}

#[test]
fn a_marker_the_reader_cannot_read_is_skipped_rather_than_guessed_at() {
    // A comment that happens to open with the prefix, and one whose line count is not a number.
    // A reader that accepted either would name a file nobody wrote, which is the failure this
    // module exists to end.
    let module = format!(
        "//@whiting surface scene.rs lines=nonsense\n{BLOCK}//@whiting grade scene.rs:9 lines=1\n{BLOCK}"
    );
    assert_eq!(explain(&module, 2), String::new());
    assert_eq!(
        origin_of(&module, 7).map(|found| found.name),
        Some("grade".to_owned()),
        "the unreadable marker is skipped and the readable one after it is still found"
    );
    assert_eq!(
        origin_of(&module, 8),
        None,
        "and the second block claims one line, so the line after it is nobody's"
    );
}
