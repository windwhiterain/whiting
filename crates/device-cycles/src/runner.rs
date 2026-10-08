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
        .arg("--device")
        .arg(device)
        .arg("--samples")
        .arg(samples.to_string())
        .arg("--width")
        .arg(resolution.width.to_string())
        .arg("--height")
        .arg(resolution.height.to_string())
        .arg("--background")
        .arg("--output")
        .arg(image)
        .arg(scene)
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
/// `ERROR`, which is what `print_error` prefixes every diagnostic with. A `WARNING` is not
/// matched: this build prints two colour-management warnings for a scene with no OCIO
/// configuration, on renders that are perfectly good, and treating those as failures would make
/// the crate unusable without an OCIO setup that is not required.
///
/// The match is on the line's start rather than anywhere in it, because a shader name, a
/// filename or a scene label may legitimately contain the word.
fn first_error(diagnostics: &str) -> Option<String> {
    diagnostics
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("ERROR"))
        .map(str::to_owned)
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
