//! Where a line of an assembled shader came from.
//!
//! # What this is
//!
//! A shader is a template with a scene's own WGSL spliced into it, so a parse error names a
//! line of a string that is on no disk. `naga` is right about that line and it is no use to a
//! reader: opening the assembled module is not a thing anybody can do, and a line number into
//! a string that exists only in memory is not a location.
//!
//! An assembler that knows where a block was written puts one line above it —
//!
//! ```text
//! //@whiting surface crates/gallery/src/tidewater.rs:718 lines=37
//! ```
//!
//! — and [`explain`] turns an assembled line number back into the file and line to open. The
//! marker is a WGSL comment, so the module `naga` reads is the module the author wrote, and the
//! scan happens **only after a parse has already failed**: a shader that compiles never pays
//! for this and never sees it.
//!
//! # Why the marker is inside the source and not beside it
//!
//! Because the source is the only part of a shader that survives the trip. The scene emitter
//! rebuilds the entry a device reads from a shader's fields with `source` and `schema` and
//! nothing else, so a map held beside the string — a second field, a side table, anything the
//! document carried separately — is dropped between the two. A comment inside the string is the
//! one carrier that arrives.
//!
//! # Invariants
//!
//! - **A line no marker covers gets no sentence.** [`explain`] returns nothing for it, so a
//!   shader that declared no origin is reported exactly as it was before this module existed.
//! - **A marker names the block it is above and nothing else.** The block occupies the lines
//!   from the one after the marker, for as many lines as it has, so no closing marker is needed
//!   and a second block's marker cannot be read as the first one's.
//! - **The prefix is reserved.** A line of a scene's own WGSL that begins [`MARKER`] would be
//!   read as a declaration, so a block's text must not contain one. The prefix carries an `@`
//!   and a resolved file path with a line number, which is not something a comment is written
//!   with by accident.

/// The comment that opens the marker an assembler writes above a block it knows the origin of.
pub const MARKER: &str = "//@whiting ";

/// One marker, as written: the block's name, where it starts, and how many lines it has.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Declared {
    name: String,
    file: String,
    line: u32,
    lines: u32,
}

/// Where one line of an assembled shader came from.
///
/// # Invariants
///
/// `line` is a line in `file` a reader can open, and it is exact: a block declared at one line
/// has every one of its lines addressed, because the assembler knows how many lines the block
/// contributes and where the one in the assembled module is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    /// What the block is called at the splice point it was written into: `surface`, `helpers`.
    pub name: String,
    /// The file the block was declared in, as `file!()` spelled it.
    pub file: String,
    /// The line to open, in that file.
    pub line: u32,
    /// The line the block starts at in that file, which is what a reader opens to see all of it.
    pub block_line: u32,
}

/// Writes the marker an assembler puts above a block it knows the origin of, and the block.
///
/// # Invariants
///
/// The result is `text` with exactly one line in front of it, and `text` is unchanged after
/// that line: the marker is a comment, so the WGSL a backend receives is the WGSL that was
/// spliced. The line count written is the number of lines `text` contributes to the assembled
/// module, which is what [`explain`] needs to know which lines the block owns.
///
/// # What the line written is
///
/// `line` is the line of `file` the block's **first line of text** is on — the line a reader
/// opens to see the whole of it — and [`origin_of`] reads it back by adding the offset of the
/// line within the block and nothing else, so a line written here wrong is a line wrong in
/// every message the block produces. Where a declaration was written from, as against where its
/// text begins, is the writer's business: a `macro_rules!` expansion cannot see how the two
/// relate, so this format carries the text's line and leaves the difference to the writer that
/// can see it.
pub fn mark(name: &str, file: &str, line: u32, text: &str) -> String {
    format!(
        "{MARKER}{name} {file}:{line} lines={}\n{text}",
        text.lines().count()
    )
}

/// Returns where the given 1-based line of an assembled shader came from.
///
/// # Invariants
///
/// - `line` is counted in the assembled module as the device holds it, markers included. The
///   markers shift the lines after them, which is why the count is taken over the same string
///   the parse read rather than over anything the assembler held.
/// - **A marker's line is the block's first line of text**, so the offset of a line within the
///   block is all that is added to it, and no shape of a declaration is read here. A writer that
///   cannot say where its text begins writes a number that is not a location, and this reports
///   that number back unchanged rather than correcting a guess.
/// - A marker that cannot be read is skipped rather than reported: a reader that guessed at one
///   would name a file the author never wrote, which is the failure this module exists to end.
pub fn origin_of(source: &str, line: u32) -> Option<Origin> {
    for (index, text) in source.lines().enumerate() {
        let Some(declared) = text.strip_prefix(MARKER).and_then(read) else {
            continue;
        };
        // The marker is on line `index + 1`, so the block starts on the line after it.
        let first = index as u32 + 2;
        if line < first || line - first >= declared.lines {
            continue;
        }
        return Some(Origin {
            name: declared.name,
            file: declared.file,
            line: declared.line + (line - first),
            block_line: declared.line,
        });
    }
    None
}

/// Returns the sentence an error message carries, or nothing at all.
///
/// # Invariants
///
/// The empty string is the answer for every line no marker covers, which is what makes this
/// additive: a shader that declared no origin is reported with the message it was reported
/// with before any of this existed. A sentence that is returned begins with a newline and is
/// indented, because it is a second line of a message rather than a message of its own.
pub fn explain(source: &str, line: u32) -> String {
    let Some(origin) = origin_of(source, line) else {
        return String::new();
    };
    format!(
        "\n  this line came from the `{}` block in {}:{} (the block starts at line {})",
        origin.name, origin.file, origin.line, origin.block_line
    )
}

/// Reads one marker, or nothing when the line is not one this module wrote.
fn read(body: &str) -> Option<Declared> {
    // Three space separated fields, because a file path may not contain whitespace: that is
    // what `file!()` spells for a crate inside a workspace, and a marker that could not be
    // tokenised would be a marker whose origin nobody could read.
    let mut fields = body.split_whitespace();
    let name = fields.next()?.to_owned();
    let (file, line) = fields.next()?.rsplit_once(':')?;
    let lines = fields.next()?.strip_prefix("lines=")?.parse().ok()?;
    Some(Declared {
        name,
        file: file.to_owned(),
        line: line.parse().ok()?,
        lines,
    })
}
