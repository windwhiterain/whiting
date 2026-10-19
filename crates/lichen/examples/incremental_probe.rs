//! The probe: one Whiting edit per run, through a `BufferSession`.
//!
//! # What it prints
//!
//! For one named edit: the session's wall time, the same source compiled fresh
//! from an empty registry and an empty slab, the session's cell events, and
//! whether the session's document and the fresh compile's are the same document.
//!
//! # Why each edit is its own run
//!
//! The slab is process-global and never empties itself, so a second edit in one
//! process would start from a slab the first edit had already warmed. One edit
//! per process is what makes each row attributable to its own dial.
//!
//! # How the two compiles are ordered, and what may be compared
//!
//! The session compiles first — the baseline, then the edit — and only then is
//! the slab emptied and **a brand-new store and session** compile the same
//! source. The fresh row is therefore a cold build through the *same* machinery,
//! with only the retained cells and the slab missing. The two document ids are
//! **not** the same number, because the slab was emptied between them; what must
//! match is the document's fingerprint, which is what the table reports.
//!
//! # Build the artifacts first
//!
//! ```text
//! cargo run -p lichen-whiting-stages --example generate_link
//! cargo build --release -p whiting-lichen-link
//! cargo run --release -p lichen-whiting --example incremental_probe -- lighting
//! ```

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use lichen_language::Diag;
use lichen_language::link;
use lichen_language::package::PackageStore;
use lichen_language::preprocess::{self, ResolvedImport};
use lichen_language::program::LangProgram;
use lichen_language::session::BufferSession;

/// The program this probe drives, compiled in.
const PROGRAM: &str = include_str!("alpine_oak.lichen");

/// The program's path, which is what a `link` path and a `base` resolve against.
const PROGRAM_PATH: &str = "crates/lichen/examples/alpine_oak.lichen";

/// One edit: the dial's own line, and what the edit turns it into.
struct Edit {
    name: &'static str,
    from: &'static str,
    to: &'static str,
}

/// The five edits, one per run.
///
/// # Invariant
///
/// Every `from` line appears **once** in [`PROGRAM`], so replacing it moves the
/// one dial it names and no other. The values are positive because the language's
/// lexer reads no negative literal: `-6.0` is a parse error, not a number.
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

/// One row of the table.
struct Row {
    edit: String,
    baseline: Duration,
    session: Duration,
    fresh: Duration,
    reused: usize,
    frozen: usize,
    dropped: usize,
    session_id: u64,
    fresh_id: u64,
    session_print: u64,
    fresh_print: u64,
    slab: u64,
}

fn main() {
    let Some(name) = std::env::args().nth(1) else {
        println!("usage: incremental_probe <edit>");
        println!(
            "edits:  {}",
            EDITS
                .iter()
                .map(|edit| edit.name)
                .collect::<Vec<_>>()
                .join(", ")
        );
        return;
    };
    let edit = EDITS
        .iter()
        .find(|edit| edit.name == name)
        .unwrap_or_else(|| panic!("{name} is not one of the five edits"));
    let source = PROGRAM.to_string();
    let source_edited = source.replacen(edit.from, edit.to, 1);
    assert_ne!(source, source_edited, "the edit moved no bytes");
    let host = Host::open(&link_path());

    // ── The baseline, then the edit: the two compiles the row is about.
    let mut store = PackageStore::<LangProgram>::new();
    let mut session =
        BufferSession::<LangProgram>::with_registry(String::new(), PROGRAM_PATH, store.registry());
    let baseline = compile(&mut session, &mut store, &source);
    let row_baseline = baseline.1;
    refuse(&baseline.0.diagnostics, "baseline");

    let edited = compile(&mut session, &mut store, &source_edited);
    let session_time = edited.1;
    refuse(&edited.0.diagnostics, "edited");
    let cells = edited.0.cells;
    let session_id = host.published();
    let session_print = host.fingerprint(session_id);

    // ── The same source, cold: the slab emptied and a store and session that
    //    have never seen it. Same path, same program, nothing retained.
    host.reset();
    let mut fresh_store = PackageStore::<LangProgram>::new();
    let mut fresh_session = BufferSession::<LangProgram>::with_registry(
        String::new(),
        PROGRAM_PATH,
        fresh_store.registry(),
    );
    let fresh = compile(&mut fresh_session, &mut fresh_store, &source_edited);
    let fresh_time = fresh.1;
    refuse(&fresh.0.diagnostics, "fresh");
    let fresh_id = host.published();
    let fresh_print = host.fingerprint(fresh_id);

    let row = Row {
        edit: edit.name.to_string(),
        baseline: row_baseline,
        session: session_time,
        fresh: fresh_time,
        reused: cells.reused,
        frozen: cells.frozen,
        dropped: cells.dropped,
        session_id,
        fresh_id,
        session_print,
        fresh_print,
        slab: host.slab_len(),
    };
    print_row(&row);
}

/// One compile of `source` under `store`, timed.
///
/// # Invariant
///
/// The view is rebuilt from `store` every time, so a `link`ed library is loaded
/// once per store and the session's cells are the only thing a compile reuses.
fn compile(
    session: &mut BufferSession<LangProgram>,
    store: &mut PackageStore<LangProgram>,
    source: &str,
) -> (
    lichen_language::session::SessionReport<LangProgram>,
    Duration,
) {
    let (code, base, imports) = view_of(source, store);
    session.set_view(code, base, &line_starts(source), &imports);
    let clock = Instant::now();
    let report = session.compile();
    (report, clock.elapsed())
}

/// Print one row, and the readings that make it checkable.
fn print_row(row: &Row) {
    println!("edit              : {}", row.edit);
    println!("baseline          : {}", millis(row.baseline));
    println!("session           : {}", millis(row.session));
    println!("fresh (cold)      : {}", millis(row.fresh));
    println!(
        "speed-up          : {:.2}x",
        row.fresh.as_millis() as f64 / row.session.as_millis().max(1) as f64
    );
    println!(
        "cells             : reused {} frozen {} dropped {}",
        row.reused, row.frozen, row.dropped
    );
    println!(
        "session document  : id {} fp {:016x}",
        row.session_id, row.session_print
    );
    println!(
        "fresh document    : id {} fp {:016x}",
        row.fresh_id, row.fresh_print
    );
    println!(
        "equivalent        : {}",
        if row.session_print == row.fresh_print && row.session_print != 0 {
            "yes"
        } else {
            "NO"
        }
    );
    println!("slab holds        : {}", row.slab);
}

/// A duration as seconds with three decimals, so two rows can be compared.
fn millis(duration: Duration) -> String {
    format!("{:.3} s", duration.as_secs_f64())
}

/// The library's own exported answers, reached by symbol.
struct Host {
    // The handle must outlive every call into it; that is what `Drop` says.
    library: libloading::Library,
    fingerprint: unsafe extern "C" fn(u64) -> u64,
    published: unsafe extern "C" fn() -> u64,
    slab_len: unsafe extern "C" fn() -> u64,
    reset: unsafe extern "C" fn(),
}

impl Host {
    /// Open the dylib and resolve the probe's four symbols.
    fn open(path: &Path) -> Self {
        // SAFETY: the library is the one the manifest's program gate accepted.
        let library = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|error| panic!("cannot load {}: {error}", path.display()));
        // SAFETY: each symbol's signature is the one the bridge exports.
        let (fingerprint, published, slab_len, reset) = unsafe {
            (
                *library
                    .get::<unsafe extern "C" fn(u64) -> u64>(b"lichen_whiting_fingerprint")
                    .expect("the library exports no fingerprint"),
                *library
                    .get::<unsafe extern "C" fn() -> u64>(b"lichen_whiting_published")
                    .expect("the library exports no publisher"),
                *library
                    .get::<unsafe extern "C" fn() -> u64>(b"lichen_whiting_slab_len")
                    .expect("the library exports no census"),
                *library
                    .get::<unsafe extern "C" fn()>(b"lichen_whiting_reset")
                    .expect("the library exports no reset"),
            )
        };
        Self {
            library,
            fingerprint,
            published,
            slab_len,
            reset,
        }
    }

    /// The id the program's last `publish` call carried.
    ///
    /// # Invariant
    ///
    /// `u64::MAX` means no call has published anything, so a row is never printed
    /// from a tap that never fired.
    fn published(&self) -> u64 {
        // SAFETY: the entry reads a static and writes nothing.
        match unsafe { (self.published)() } {
            u64::MAX => panic!("the program published no document"),
            id => id,
        }
    }

    /// The slab's fingerprint of the document under `id`.
    fn fingerprint(&self, id: u64) -> u64 {
        // SAFETY: the entry reads the slab and writes nothing.
        unsafe { (self.fingerprint)(id) }
    }

    /// How many values the slab holds.
    fn slab_len(&self) -> u64 {
        // SAFETY: the entry reads the slab and writes nothing.
        unsafe { (self.slab_len)() }
    }

    /// Empty the slab, so the next compile is a cold one.
    ///
    /// # Safety
    ///
    /// The caller reads no id it kept across the call, which is why the session's
    /// id and fingerprint are taken before this is ever called.
    fn reset(&self) {
        // SAFETY: upheld by this function's own contract.
        unsafe { (self.reset)() };
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        // The handle is dropped after the last call that could use it.
        let _ = &self.library;
    }
}

/// The dylib the program names, resolved the way its `link` directive resolves it.
fn link_path() -> PathBuf {
    let base = base_path();
    let resolved = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(link::resolve_path(
            Some(&base),
            "../../../target/release/whiting_lichen_link.dll",
        ));
    assert!(
        resolved.exists(),
        "the dylib is not built: run `cargo build --release -p whiting-lichen-link` (looked for {})",
        resolved.display()
    );
    resolved
}

/// The program's own path, which the preprocessor's `base` names.
fn base_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/alpine_oak.lichen")
}

/// The original file's line starts, for a view with a base offset.
fn line_starts(source: &str) -> Vec<usize> {
    lichen_language::lex::line_starts(source)
}

/// The code, its base offset and its imports, as a session compiles `source`.
///
/// # Invariant
///
/// A `link`ed library becomes a **package with direct bindings**, not a splice:
/// the session path is the editor's path, and it is what the incremental
/// measurement rides on.
fn view_of(
    source: &str,
    store: &mut PackageStore<LangProgram>,
) -> (String, u32, Vec<ResolvedImport>) {
    let base = base_path();
    let (pre, diagnostics) = preprocess::preprocess::<LangProgram>(source, Some(&base), store);
    for diagnostic in &diagnostics {
        eprintln!("preprocess: {}", diagnostic.message);
    }
    let mut imports = pre.imports.clone();
    match store.link_imports(&pre.links, Some(&base)) {
        Ok(linked) => imports.extend(linked),
        Err((_, message)) => eprintln!("link: {message}"),
    }
    (pre.code.to_string(), pre.code_base, imports)
}

/// Print every diagnostic, so a broken row is never a silent one.
fn refuse(diagnostics: &[Diag<LangProgram>], what: &str) {
    for diagnostic in diagnostics {
        eprintln!(
            "{what}[{:?}]: {} at {:?}",
            diagnostic.stage, diagnostic.message, diagnostic.span
        );
    }
}
