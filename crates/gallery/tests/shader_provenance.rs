//! A stage's WGSL says which file and line each declared block was written at.
//!
//! # What this proves, and what it does not
//!
//! That the assembler writes the marker where it says it does, above the block it was handed
//! and by the name of the splice point it was written into; that an assembled line inside a
//! declared block maps back to **the exact line of the file that declared it**, checked here
//! by reading this file; and that a block which declared nothing is spliced in verbatim, with
//! no marker at all.
//!
//! # Why no line number is written down in this file
//!
//! Because a `wgsl!` invocation is reshaped by `cargo fmt` after the test is written, and a
//! test holding a line number passes today and fails the next time anyone formats — which is
//! how the off-by-one this file guards against was found. Every expectation here is therefore
//! **read out of this file** instead: a statement's own line is found by searching this file
//! for the statement, and a block's first line is the line above its first statement, which is
//! where a literal opening with a newline puts the block's first and empty line. Reshape the
//! invocation and the expectations travel with it. Reshape it into a shape the macro cannot
//! report, and these tests fail rather than pass quietly.
//!
//! It is **not** a render. That a WGSL comment moves no pixel is shown by comparing two
//! renders by hash rather than here, and the sentence a device prints is shown by rendering a
//! deliberately broken scene.

use std::fs;

use graphics::provenance::{MARKER, explain, origin_of};
use whiting_gallery::kit::gallery_custom_schema;
use whiting_gallery::shaders::{Block, Projection, lighting, post};
use whiting_gallery::wgsl;

/// A surface body, declared, in the shape `rustfmt` leaves a `wgsl!` invocation in: the literal
/// opens on the line after the invocation, so the block's first line of text is the empty tail
/// of the literal's own opening line and the statement is the line after that. There is no other
/// shape to write this in — `cargo fmt` breaks an invocation whose literal opens on the
/// invocation's own line — and the `wgsl!` doc in `shaders.rs` says what that costs.
const SURFACE: Block = wgsl!(
    r#"
    color = color * (0.4 + 0.6 * shade);
"#
);

/// The same text, written the way every scene wrote it before a block existed.
const SURFACE_AS_TEXT: &str = r#"
    color = color * (0.4 + 0.6 * shade);
"#;

/// The WGSL statement a block is made of, found in this file rather than counted by hand: a
/// test that wrote the number down would keep passing after the file moved under it, and the
/// number is the whole claim. The needle is the **indented** line, because this file also
/// spells the statement out in a constant.
const STATEMENT: &str = "color = color * (0.4 + 0.6 * shade);";

/// The same statement as a block writes it, which is what the module holds and what the first
/// line of this file carrying the indented text is.
const STATEMENT_IN_BLOCK: &str = "    color = color * (0.4 + 0.6 * shade);";

#[test]
fn a_declared_block_is_spliced_under_a_marker_naming_its_file_and_line() {
    let module = lighting(
        "",
        SURFACE,
        "",
        Projection::Latlong,
        &gallery_custom_schema(),
    );
    let marker = marker(&module, "surface").expect("a declared block is spliced with a marker");
    // A literal opening with a newline puts the block's first line of text — an empty one — on
    // the line above the statement, so the marker names the statement's line less one.
    let first = statement_line() - 1;

    assert!(
        marker.contains(&format!("{}:{} ", this_file(), first)),
        "the marker names the file the block was declared in and the line its first line of \
         WGSL is on: {marker}"
    );
    assert!(
        marker.ends_with(" lines=2"),
        "and how many lines the block contributes, which is what the reader needs: {marker}"
    );
    assert!(
        module.contains(&format!("    {STATEMENT}")),
        "and the block itself is in the module unchanged, because the marker is a comment"
    );
}

#[test]
fn an_assembled_line_maps_back_to_the_exact_line_of_the_file_it_came_from() {
    let module = lighting(
        "",
        SURFACE,
        "",
        Projection::Latlong,
        &gallery_custom_schema(),
    );
    // The marker is on one line, the block's first line of WGSL is the empty one below it, and
    // the statement is the third line from the marker.
    let statement = marker_line(&module, "surface") + 2;

    let origin = origin_of(&module, statement as u32)
        .expect("the statement is inside the block the marker declares");
    assert_eq!(origin.name, "surface");
    assert_eq!(origin.file, this_file());
    assert_eq!(
        origin.line as usize,
        statement_line() as usize,
        "the line to open is the line the statement is written on, not the line of the \
         assembled module it landed in"
    );
    assert_eq!(
        origin.block_line as usize,
        statement_line() as usize - 1,
        "and the block starts on the line above, which is where a literal opening with a \
         newline puts the block's first — empty — line"
    );

    let written = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/shader_provenance.rs"
    ))
    .expect("this file is on disk");
    assert_eq!(
        written
            .lines()
            .nth(origin.line as usize - 1)
            .expect("the line the message names is a line this file has")
            .trim(),
        STATEMENT,
        "the line a parse failure is reported against is the WGSL it failed on"
    );
}

#[test]
fn a_block_that_declared_nothing_is_spliced_in_verbatim() {
    // The additive property. A scene that writes a plain `&str` gets the module it always got:
    // no marker, and the text spliced where it always was. The proof that this moves no pixel
    // is two renders compared by hash; this is the half of it that is arithmetic.
    let module = lighting(
        "",
        SURFACE_AS_TEXT,
        "",
        Projection::Latlong,
        &gallery_custom_schema(),
    );

    assert!(
        !module.contains(MARKER),
        "a block with no declaration writes no marker, because a marker is what a declaration buys"
    );
    assert_eq!(
        module.matches(STATEMENT_IN_BLOCK).count(),
        1,
        "and the text is spliced once, unchanged"
    );
    assert_eq!(
        explain(&module, 1),
        String::new(),
        "so a parse failure in it is reported with the message it was reported with before"
    );
}

#[test]
fn the_same_text_in_two_stages_is_two_blocks_with_two_meanings() {
    // The marker names the splice point rather than only the file, because a reader told only
    // a line in a sixty thousand line scene file is not told what they are looking at.
    assert!(
        marker(
            &lighting(
                "",
                SURFACE,
                "",
                Projection::Latlong,
                &gallery_custom_schema()
            ),
            "surface"
        )
        .is_some()
    );
    assert!(
        marker(&post(SURFACE, "", None), "grade").is_some(),
        "the same text is a grade in the post stage, and the marker says so"
    );
    assert!(
        marker(&post(SURFACE, "", None), "surface").is_none(),
        "and it does not claim to be the surface block it was in the other stage"
    );
}

#[test]
fn the_block_starts_on_the_line_after_the_one_the_macro_was_invoked_on() {
    // The arithmetic, stated once and checked from both ends: a literal opening on the line
    // after the invocation puts the block's first line of text — an empty one — on that line,
    // and the statement below it. `line!()` reads the invocation's line and nothing can see
    // where the literal opens, so the line the marker carries is the one after it.
    let invoked_at = line_of("const SURFACE: Block = wgsl!");
    assert_eq!(
        statement_line() as usize,
        invoked_at as usize + 2,
        "this constant is written in the shape the macro is documented for: the invocation on \
         one line, the literal opening on the next, and the statement on the line after that"
    );

    let module = lighting(
        "",
        SURFACE,
        "",
        Projection::Latlong,
        &gallery_custom_schema(),
    );
    let marker = marker(&module, "surface").expect("the block is marked");
    assert!(
        marker.contains(&format!("{}:{} ", this_file(), invoked_at + 1)),
        "so the marker names the line the block's first line of text is on, which is one on \
         from the invocation: {marker}"
    );
}

#[test]
fn every_block_the_crate_declares_is_written_in_the_shape_the_macro_can_report() {
    // The shape is not something a reader can check by eye in a file of thousands of lines, and
    // a block written in the other one is reported a line high with nothing to say so. Every
    // declaration in this crate's own sources is therefore read and held to the shape, doc
    // comments included: a documented example of the other shape would teach it.
    let mut found = Vec::new();
    declarations(concat!(env!("CARGO_MANIFEST_DIR"), "/src"), &mut found);

    assert!(
        !found.is_empty(),
        "the crate declares blocks with the macro this test is about"
    );
    for (file, number, line) in found {
        assert!(
            line.trim_end().ends_with("= wgsl!("),
            "{file}:{number} writes a `wgsl!` literal on the invocation's own line, which is \
             the shape the macro cannot report: {line}"
        );
    }
}

/// Returns every line of the gallery's own sources that declares a block with the macro, as
/// the path it was found under, its line number and the line itself.
fn declarations(directory: &str, found: &mut Vec<(String, usize, String)>) {
    let entries =
        fs::read_dir(directory).unwrap_or_else(|error| panic!("{directory} is readable: {error}"));
    for entry in entries {
        let path = entry.expect("an entry of the directory is readable").path();
        if path.is_dir() {
            declarations(&path.to_string_lossy(), found);
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }
        let file = path.to_string_lossy().into_owned();
        for (index, line) in fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{file} is readable: {error}"))
            .lines()
            .enumerate()
        {
            if line.contains("= wgsl!(") {
                found.push((file.clone(), index + 1, line.to_owned()));
            }
        }
    }
}

/// Returns the line the block's statement is written on, counted in this file.
///
/// The statement is searched for rather than counted by hand, and the first line holding it is
/// the block's own because the block is declared above the constant that spells the same
/// statement out — a test that guessed wrong there would be reporting against the other one.
fn statement_line() -> u32 {
    line_of(STATEMENT_IN_BLOCK)
}

/// Returns the path a marker carries for this file.
///
/// This is `file!()` read in this file, which is the very call the `wgsl!` above made: a test
/// that spelled the path out would pass against a marker naming a different one, and the file
/// a reader is told to open is the whole claim.
fn this_file() -> &'static str {
    file!()
}

/// Returns this file, as it is on disk rather than as the compiler last read it.
fn this_file_text() -> String {
    fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/shader_provenance.rs"
    ))
    .expect("this file is on disk")
}

/// Returns the 1-based line of this file holding the first line that contains `needle`.
fn line_of(needle: &str) -> u32 {
    this_file_text()
        .lines()
        .position(|line| line.contains(needle))
        .map_or_else(
            || panic!("this file holds `{needle}`, which the test is written against"),
            |index| index as u32 + 1,
        )
}

/// Returns the line the marker above the named block is on, counted as a reader counts.
fn marker_line(module: &str, name: &str) -> usize {
    module
        .lines()
        .position(|line| is_marker(line, name))
        .expect("the block is marked")
        + 1
}

/// Returns the marker above the named block.
fn marker<'module>(module: &'module str, name: &str) -> Option<&'module str> {
    module.lines().find(|line| is_marker(line, name))
}

/// Whether a line is the marker above the named block.
fn is_marker(line: &str, name: &str) -> bool {
    line.starts_with(MARKER) && line.contains(&format!(" {name} "))
}
