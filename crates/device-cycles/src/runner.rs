//! Starting Cycles, and deciding whether what came back is a render or a failure.
//!
//! # The exit code is not the answer
//!
//! Cycles reports a fatal-looking scene error, renders the rest of the scene, writes the image
//! and exits **zero**. Measured on this build, twice: a `light_type` of `"pointt"` and a missing
//! `<include>` each produced `ERROR: Unknown value` on stderr, a complete png on disk, and
//! exit status 0.
//!
//! The loader validates enums on the reflection path and *warns*, then falls through to the
//! dispatch chain, where `assert(light_type == "spot")` is compiled out in a release build. So the
//! render proceeds with a light that was never created, and the picture is short one light with
//! nothing in the log to say so.
//!
//! This module therefore reads stderr and treats `ERROR` in it as failure, which is the signal
//! the format actually gives. The exit status is still checked, because a crash or a missing
//! binary produces neither an image nor a diagnostic worth reading.
//!
//! # Why the child's output is captured rather than inherited
//!
//! The one other subprocess in this workspace — the `ffmpeg` call in the render harness —
//! inherits stdio through `.status()`, which streams progress live and throws the diagnostics
//! away. That is the right trade for a tool whose failure is its own message on stderr. It is the
//! wrong one here: the whole point of this crate is to *read* stderr, and a child whose output is
//! inherited cannot be read. Progress is still reported, by [`RunOutcome::seconds`].

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

/// The name of the environment variable that names the binary.
pub const BINARY_VARIABLE: &str = "WHITING_CYCLES";

/// Where the binary is looked for, in order.
///
/// # Why a variable and not a constant
///
/// The build is not checked in — it is several gigabytes and a CMake configuration, so everyone
/// who has one has it somewhere else. A path compiled into the crate would work on exactly one
/// machine. A variable, then the usual place a binary is put, then the build this was developed
/// against, in that order, so that an install is found without configuration and a development
/// checkout works without it too.
const SEARCH: &[&str] = &[
    r"C:\resource\cycles\install\cycles.exe",
    r"C:\Program Files\Cycles\cycles.exe",
];

/// Finds the Cycles binary, or says where it looked.
///
/// # Errors
///
/// Returns the search that was performed rather than a bare "not found", because the difference
/// between "you have not set the variable" and "the variable points somewhere that is not a
/// file" is the whole of the diagnosis.
pub fn locate_binary(explicit: Option<&Path>) -> Result<PathBuf, CyclesError> {
    if let Some(path) = explicit {
        return if path.is_file() {
            Ok(path.to_path_buf())
        } else {
            Err(CyclesError::BinaryMissing {
                path: path.to_path_buf(),
            })
        };
    }

    if let Some(configured) = std::env::var_os(BINARY_VARIABLE) {
        let path = PathBuf::from(configured);
        return if path.is_file() {
            Ok(path)
        } else {
            Err(CyclesError::BinaryMissing { path })
        };
    }

    for candidate in SEARCH {
        let path = Path::new(candidate);
        if path.is_file() {
            return Ok(path.to_path_buf());
        }
    }

    Err(CyclesError::BinaryNotFound {
        searched: SEARCH.join(", "),
    })
}

/// Runs one render and says what came back.
pub struct RunOutcome {
    /// The image, if one was written.
    pub image: Option<PathBuf>,
    /// How long the process took.
    pub seconds: f64,
    /// Everything the process printed, kept because an error is only half of what a reader
    /// needs: the lines above the error are usually what explains it.
    pub diagnostics: String,
}

/// The size of the image to render, in pixels.
///
/// # Why this is an argument and not read from the document
///
/// The document *does* state it — the emitter writes the camera's `width` and `height` onto the
/// `<camera>` element, and the loader does read them back onto the camera. The buffer Cycles
/// renders into is a different thing, and it comes from this command line. Measured on this
/// build: a document whose camera says `1920, 1080` and whose `<film>` says nothing about size
/// writes a `1024, 512` png; the same document with `--width 320 --height 180` writes a
/// `320, 180` one.
///
/// There is nowhere in the document to put it. `NODE_DEFINE(Film)` in `src/scene/film.cpp`
/// declares `exposure`, `filter_type`, `pass_alpha_threshold` and the cryptomatte sockets, and
/// **no resolution socket at all** — unlike older Cycles, where the render layer carried it. So
/// a document that cannot be rendered at the size it asks for is a document that has to be
/// corrected from outside, and the size is carried out here.
///
/// # Who fills it in
///
/// **The scene's own frame, and nothing else.** The one producer is [`crate::render_scene`],
/// which reads [`scene::cycles_export::render_size`] — the emitter's own reader of the document's
/// `frame` — and nothing in this crate holds a size of its own: this struct has no `Default` and
/// no constant beside it. A scene that names no frame gets the schema's `DEFAULT_WIDTH` by
/// `DEFAULT_HEIGHT`, which is a property of the scene rather than of this backend. [`arguments`]
/// is where the two numbers become the process's `--width` and `--height`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl std::fmt::Display for Resolution {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}x{}", self.width, self.height)
    }
}

/// The arguments one render is started with, in order, after the binary's own name.
///
/// # Why this is a function and not the inside of [`run`]
///
/// **`--width` and `--height` are the only thing that decides how large the written file is**,
/// because `NODE_DEFINE(Film)` declares no resolution socket, so this list is the whole of the
/// size's route out of the document. Written inline it cannot be read without starting a
/// process, and a check that cannot run the thing it is about is a check that agrees because
/// it cannot differ: a runner that passed a constant here would render every scene square and
/// every test in the tree would still be green.
///
/// So the list is built by itself and compared against what the scene said, and the resolution
/// it carries is the one [`crate::render_scene`] read out of the document rather than a number
/// typed here.
///
/// # Invariants
///
/// The scene document is **last**, because it is the only positional argument and the loader
/// takes the flags before it. Moving it earlier makes a different process rather than a
/// differently-shaped render.
pub fn arguments(
    device: &str,
    samples: u32,
    resolution: Resolution,
    image: &Path,
    scene: &Path,
) -> Vec<OsString> {
    vec![
        OsString::from("--device"),
        OsString::from(device),
        OsString::from("--samples"),
        OsString::from(samples.to_string()),
        OsString::from("--width"),
        OsString::from(resolution.width.to_string()),
        OsString::from("--height"),
        OsString::from(resolution.height.to_string()),
        OsString::from("--background"),
        OsString::from("--output"),
        image.as_os_str().to_owned(),
        scene.as_os_str().to_owned(),
    ]
}

/// Runs one render and says what came back.
///
/// # Invariants
///
/// The returned `Ok` means the process exited zero **and** printed no `ERROR`. Both, because
/// neither alone is sufficient: the exit status misses a scene that failed to load, and the
/// stderr scan is the only thing standing between a missing light and a plausible picture.
///
/// The image is checked for existence rather than assumed from the exit status, because a
/// process can succeed and write nothing — an unwritable output path does exactly that.
pub fn run(
    binary: &Path,
    scene: &Path,
    image: &Path,
    device: &str,
    samples: u32,
    resolution: Resolution,
) -> Result<RunOutcome, CyclesError> {
    let started = Instant::now();
    let output = Command::new(binary)
        .args(arguments(device, samples, resolution, image, scene))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|source| CyclesError::Spawn {
            binary: binary.to_path_buf(),
            source,
        })?;

    let seconds = started.elapsed().as_secs_f64();

    // Cycles writes progress to stderr and diagnostics to it as well, so the two streams are
    // joined rather than one being preferred: an `ERROR` can appear in either.
    let mut diagnostics = String::from_utf8_lossy(&output.stderr).into_owned();
    diagnostics.push('\n');
    diagnostics.push_str(&String::from_utf8_lossy(&output.stdout));

    let failure = first_error(&diagnostics);

    if !output.status.success() {
        return Err(CyclesError::Failed {
            status: output.status.to_string(),
            diagnostics,
        });
    }

    if let Some(line) = failure {
        return Err(CyclesError::Reported { line, diagnostics });
    }

    if !image.is_file() {
        return Err(CyclesError::NoImage {
            path: image.to_path_buf(),
            diagnostics,
        });
    }

    Ok(RunOutcome {
        image: Some(image.to_path_buf()),
        seconds,
        diagnostics,
    })
}

/// Returns the first line that reports an error, if the process printed one.
///
/// # What counts
///
/// `ERROR`, which is what the log prefix carries. A `WARNING` is not matched: this build
/// prints two colour-management warnings for a scene with no OCIO configuration, on renders
/// that are perfectly good, and treating those as failures would make the crate unusable
/// without an OCIO setup that is not required.
///
/// # The line does **not** begin with the level, and this did not know that
///
/// In background mode every line carries a timestamp and a separator first:
///
/// ```text
/// 00:00.59 | ERROR: Failed to load 1 image files
/// ```
///
/// This used to match on the line starting with `ERROR`, which is what the level looks like
/// when there is no timestamp. That scan never fired. Measured on `foldyard`: the subprocess
/// printed `WARNING: Image file out\air does not exist` and then
/// `ERROR: Failed to load 1 image files`, and the run was reported as a success — which it is,
/// by exit status and by an image on disk. **Every render this crate accepted could have had a
/// fatal scene error in it**, which is the one thing the module exists to prevent.
///
/// So the prefix is taken off before the level is read, rather than the level being looked for
/// at the front of the line. The prefix is a timestamp and a `|`; anything else is left alone,
/// so a line that begins with the level still matches.
fn first_error(diagnostics: &str) -> Option<String> {
    diagnostics.lines().find_map(|line| {
        let level = line.trim();
        let level = match level.split_once('|') {
            // Only a prefix that looks like a clock is a prefix: a shader name or a filename
            // may contain `|`, and `code | ERROR: ...` is a message rather than a log line.
            Some((before, after))
                if before
                    .trim()
                    .split(':')
                    .all(|part| part.parse::<f32>().is_ok()) =>
            {
                after.trim()
            }
            _ => level,
        };
        level.starts_with("ERROR").then(|| line.trim().to_owned())
    })
}

/// Something about running Cycles did not work.
#[derive(Debug, thiserror::Error)]
pub enum CyclesError {
    #[error("the Cycles binary is not there: {path}")]
    BinaryMissing { path: PathBuf },

    #[error(
        "no Cycles binary was found. Set {BINARY_VARIABLE} to the executable, or put one at one \
         of: {searched}"
    )]
    BinaryNotFound { searched: String },

    #[error("Cycles could not be started at {binary}: {source}")]
    Spawn {
        binary: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Cycles failed ({status}). Its output was:\n{diagnostics}")]
    Failed { status: String, diagnostics: String },

    #[error(
        "Cycles reported an error and carried on anyway:\n  {line}\n\nIts output was:\n{diagnostics}"
    )]
    Reported { line: String, diagnostics: String },

    #[error("Cycles succeeded but wrote no image at {path}. Its output was:\n{diagnostics}")]
    NoImage { path: PathBuf, diagnostics: String },

    #[error("the image Cycles wrote could not be read back: {0}")]
    Decode(String),

    #[error("the scene could not be written as a Cycles document: {0}")]
    Emit(String),
}
