//! The machinery the incremental probes share: one host over the dylib, one row per edit.
//!
//! # What lives here and what does not
//!
//! The **host** (loading the dylib and reading its four answers), the **view** a session compiles
//! (the preprocessed code, its base offset and its imports), the two compiles a row is made of, and
//! the row's own printing. What stays with each probe is **the program and its edits**: what an
//! edit *is* is the only thing a family's probe knows, and the oak's five edits are not the hall's
//! four.
//!
//! # Why one edit per run
//!
//! The slab is process-global and never empties itself, so a second edit in one process would start
//! from a slab the first edit had already warmed. One edit per process is what makes each row
//! attributable to its own dial.
//!
//! # How the two compiles are ordered, and what may be compared
//!
//! The session compiles first — the baseline, then the edit — and only then is the slab emptied
//! and **a brand-new store and session** compile the same source. The fresh row is therefore a
//! cold build through the *same* machinery, with only the retained cells and the slab missing. The
//! two document ids are **not** the same number, because the slab was emptied between them; what
//! must match is the document's fingerprint, which is what the table reports.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use lichen_language::Diag;
use lichen_language::link;
use lichen_language::package::PackageStore;
use lichen_language::preprocess::{self, ResolvedImport};
use lichen_language::program::LangProgram;
use lichen_language::session::{BufferSession, SessionReport};

/// One edit: the dial's own line, and what the edit turns it into.
///
/// # Invariant
///
/// **Every `from` appears once in the program**, so replacing it moves the one dial it names and no
/// other. An edit whose `from` and `to` are equal is the *same source* row: it recompiles what is
/// already in the session and must move no cell at all.
pub struct Edit {
    pub name: &'static str,
    pub from: &'static str,
    pub to: &'static str,
}

/// One row of the table.
pub struct Row {
    pub edit: String,
    pub baseline: Duration,
    pub session: Duration,
    pub fresh: Duration,
    pub reused: usize,
    pub frozen: usize,
    pub dropped: usize,
    pub session_id: u64,
    pub fresh_id: u64,
    pub session_print: u64,
    pub fresh_print: u64,
    pub slab: u64,
}

/// What one run measured: the row, and the fingerprint the **baseline** compile published.
pub struct Run {
    pub row: Row,
    pub baseline_print: u64,
}

/// Runs one edit of `program`, and returns what it measured.
///
/// # Invariant
///
/// The baseline's fingerprint is read **before** the edit is compiled and before the slab is
/// emptied, because both would invalidate it: an id means nothing across [`Host::reset`].
pub fn run(program_path: &str, program: &str, edit: &Edit) -> Run {
    let source = program.to_string();
    let source_edited = source.replacen(edit.from, edit.to, 1);
    if edit.from != edit.to {
        assert_ne!(source, source_edited, "the edit moved no bytes");
    }
    let host = Host::open(&link_path(program_path));

    // ── The baseline, then the edit: the two compiles the row is about.
    let mut store = PackageStore::<LangProgram>::new();
    let mut session =
        BufferSession::<LangProgram>::with_registry(String::new(), program_path, store.registry());
    let baseline = compile(&mut session, &mut store, program_path, &source);
    let row_baseline = baseline.1;
    refuse(&baseline.0.diagnostics, "baseline");
    let baseline_print = host.fingerprint(host.published());

    let edited = compile(&mut session, &mut store, program_path, &source_edited);
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
        program_path,
        fresh_store.registry(),
    );
    let fresh = compile(
        &mut fresh_session,
        &mut fresh_store,
        program_path,
        &source_edited,
    );
    let fresh_time = fresh.1;
    refuse(&fresh.0.diagnostics, "fresh");
    let fresh_id = host.published();
    let fresh_print = host.fingerprint(fresh_id);

    Run {
        row: Row {
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
        },
        baseline_print,
    }
}

/// One compile of `source` under `store`, timed.
///
/// # Invariant
///
/// The view is rebuilt from `store` every time, so a `link`ed library is loaded once per store and
/// the session's cells are the only thing a compile reuses.
fn compile(
    session: &mut BufferSession<LangProgram>,
    store: &mut PackageStore<LangProgram>,
    program_path: &str,
    source: &str,
) -> (SessionReport<LangProgram>, Duration) {
    let (code, base, imports) = view_of(source, program_path, store);
    session.set_view(
        code,
        base,
        &lichen_language::lex::line_starts(source),
        &imports,
    );
    let clock = Instant::now();
    let report = session.compile();
    (report, clock.elapsed())
}

/// Print one row, and the readings that make it checkable.
pub fn print_row(row: &Row) {
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

/// Print how the probe is run, and the edits it carries.
pub fn usage(edits: &[Edit]) {
    println!("usage: <this probe> <edit>");
    println!(
        "edits:  {}",
        edits
            .iter()
            .map(|edit| edit.name)
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// A duration as seconds with three decimals, so two rows can be compared.
pub fn millis(duration: Duration) -> String {
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
    /// `u64::MAX` means no call has published anything, so a row is never printed from a tap that
    /// never fired.
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
    /// The caller reads no id it kept across the call, which is why the session's id and
    /// fingerprint are taken before this is ever called.
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
fn link_path(program_path: &str) -> PathBuf {
    let base = base_path(program_path);
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
///
/// # Invariant
///
/// **The file name, not the session path**: a `link` directive and a `base` resolve against the
/// program as it sits on disk beside this crate's examples, while the session carries the editor's
/// own relative path. Joining the two would resolve every `link` twice.
fn base_path(program_path: &str) -> PathBuf {
    let name = Path::new(program_path)
        .file_name()
        .expect("the program's path names a file");
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
}

/// The code, its base offset and its imports, as a session compiles `source`.
///
/// # Invariant
///
/// A `link`ed library becomes a **package with direct bindings**, not a splice: the session path
/// is the editor's path, and it is what the incremental measurement rides on.
fn view_of(
    source: &str,
    program_path: &str,
    store: &mut PackageStore<LangProgram>,
) -> (String, u32, Vec<ResolvedImport>) {
    let base = base_path(program_path);
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
