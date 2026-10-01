//! Renders a named gallery scene to pngs, on the real gpu.
//!
//! This is the authoring loop: a scene is built, compiled by the `StandardScene` compiler,
//! rendered headlessly, and written out as an image. Nothing here is part of the framework —
//! it is the harness that makes a scene visible while it is being written.
//!
//! A scene is a still until a `time` range is given, and a range is rendered by patching
//! the scene's `time` field and rendering each value in turn. That keeps a frame a pure
//! function of the documents and one number, so the same range rendered twice produces the
//! same images, and each frame after the first is an incremental compile of the one before
//! rather than a fresh one.
//!
//! ```text
//! cargo run -p whiting-device --features wgpu --example render -- <scene> <out> [times] [mode]
//!
//!   render dunes out/dunes                    one frame, at the time the scene carries
//!   render dunes out/dunes 2.5                one frame, at t = 2.5
//!   render dunes out/dunes 0 6 0.25           thirteen frames, a quarter second apart
//!   render dunes out/dunes 0 6 0.25 --sheet   the same range, as one contact sheet
//!   render dunes out/dunes 0 6 0.1  --video   the same range, as an mp4
//! ```

use std::path::{Path, PathBuf};

use definition::{Definition, DefinitionIdentifier, Registry, SchemaIdentifier, Value};
use gallery::Gallery;
use graphics::{GRAPHICS_SCHEMA, INSTANCE_SCHEMA};
use scene::{CompileContext, Compiler, STANDARD_SCENE_SCHEMA, StandardScene};
use whiting_device::{RenderedFrame, Renderer, WgpuRenderer};

/// Field the scene's time is written under.
const TIME: &str = "time";

/// A moment a scene is drawn at.
struct Moment {
    /// The time the scene was patched to, when the range named one.
    time: f64,
    /// The frame drawn at that time.
    frame: RenderedFrame,
}

/// The two documents one moment compiled to, carried to the next as its previous state.
struct Compiled {
    graphics: Definition,
    instance: Definition,
}

#[tokio::main]
async fn main() {
    if let Err(failure) = run().await {
        eprintln!("error: {failure}");
        std::process::exit(1);
    }
}

/// Renders the scene named on the command line into the images named after it.
async fn run() -> Result<(), String> {
    let request = parse_arguments()?;
    let name = &request.name;

    let renderer = WgpuRenderer::new()
        .await
        .map_err(|error| format!("no gpu adapter: {error}"))?;

    let mut registry = Registry::new();
    registry.put_schema(
        SchemaIdentifier::new(GRAPHICS_SCHEMA),
        graphics::graphics_schema(),
    );
    registry.put_schema(
        SchemaIdentifier::new(INSTANCE_SCHEMA),
        graphics::instance_schema(),
    );
    scene::register_standard_scene_schema(&mut registry);

    let compiler = StandardScene::new(
        DefinitionIdentifier::new("gallery.graphics"),
        DefinitionIdentifier::new("gallery.instance"),
    );
    let source_identifier = DefinitionIdentifier::new("gallery.source");
    let authored = Gallery::build(&name);

    // Every camera a scene names is rendered. A camera decides the projection every pass draws
    // through — and, for a fitted camera, how far back the eye stands — so it is an input to
    // the compile rather than something a renderer picks afterwards. A scene naming two
    // cameras is compiled and drawn twice; a scene naming none is drawn once, by the stated
    // convention, exactly as it always was.
    let cameras = camera_count(&authored);
    for camera in 0..cameras {
        let mut moments = Vec::with_capacity(request.times.len());
        let mut previous: Option<Compiled> = None;
        for &time in &request.times {
            let started = std::time::Instant::now();
            let mut scene = authored.clone();
            let cloned_at = started.elapsed();
            set_time(&mut scene, time);

            let source = Definition::new(
                source_identifier.clone(),
                SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
                scene,
            );
            source
                .validate(&registry)
                .map_err(|error| format!("the scene does not satisfy the scene schema: {error}"))?;
            let validated_at = started.elapsed();

            let compiled = compiler
                .compile(&CompileContext {
                    scene: &source,
                    previous_graphics: previous.as_ref().map(|output| &output.graphics),
                    previous_instance: previous.as_ref().map(|output| &output.instance),
                    registry: &registry,
                    camera,
                })
                .map_err(|error| format!("the scene does not compile: {error}"))?;
            let compiled_at = started.elapsed();

            // What the compiler learned and could not refuse on. A note is not an error: a
            // scene lit less than it was drawn still renders, and the render is wrong in a way
            // nobody sees until the frame is looked at. A host that keeps the field to itself
            // is a host that hands over a wrong image without a word, so it is printed here —
            // once per camera, because the notes are about the view that camera compiles.
            if camera == 0 {
                for note in &compiled.notes {
                    println!("note at {}: {}", note.path, note.message);
                }
            }

            let frame = renderer
                .render(&compiled.graphics.value, &compiled.instance.value)
                .map_err(|error| {
                    format!("the frame at {} does not render: {error}", describe(time))
                })?;
            let passes = graphics::passes(&compiled.graphics.value)
                .map_err(|error| format!("the emitted passes do not read: {error}"))?
                .len();
            // The three are reported apart because they are three different costs, and only
            // one of them is the delta's business: a delta is small or large, and the work
            // that finds it is the same size either way.
            println!(
                "{name}{}: {} drawn, {passes} passes, {} operations of delta, \
                 cloned {} ms, validated {} ms, compiled {} ms, rendered {} ms",
                if cameras > 1 {
                    format!(" (camera {camera})")
                } else {
                    String::new()
                },
                describe(time),
                compiled.graphics_delta.operations.len() + compiled.instance_delta.operations.len(),
                cloned_at.as_millis(),
                (validated_at - cloned_at).as_millis(),
                (compiled_at - validated_at).as_millis(),
                started.elapsed().as_millis() - compiled_at.as_millis(),
            );

            moments.push(Moment {
                time,
                frame: RenderedFrame {
                    renderer: renderer.name(),
                    frame,
                },
            });
            previous = Some(Compiled {
                graphics: compiled.graphics,
                instance: compiled.instance,
            });
        }

        // A scene with one camera writes where it always wrote, so every existing invocation
        // of the harness names the files it named before, and a scene with several says which
        // view each set is.
        //
        // The suffix goes **before** the extension. Appended after it, `with_extension` in
        // `numbered` reads `png-camera0` as the extension and replaces it with `png` again,
        // so every camera writes the same file and the last one silently overwrites the
        // rest — which is how a two camera scene produces one image and no complaint.
        let target = if cameras > 1 {
            Request {
                path: with_stem_suffix(&request.path, &format!("camera{camera}")),
                ..request.clone()
            }
        } else {
            request.clone()
        };
        write_moments(&target, &moments)?;
    }
    Ok(())
}

/// Inserts a suffix before a path's extension.
///
/// # Invariants
///
/// The suffix lands on the **stem**. A path carrying it after the extension still names the
/// same file once anything replaces the extension, which is what every writer in this
/// harness does before it writes.
fn with_stem_suffix(path: &str, suffix: &str) -> String {
    let as_path = Path::new(path);
    let stem = as_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "frame".to_owned());
    match as_path.extension() {
        Some(extension) => as_path
            .with_file_name(format!("{stem}-{suffix}.{}", extension.to_string_lossy()))
            .to_string_lossy()
            .into_owned(),
        None => format!("{path}-{suffix}"),
    }
}

/// Returns how many cameras a scene names, which is one for a scene that names none.
fn camera_count(scene: &Value) -> usize {
    scene
        .as_field_map()
        .and_then(|fields| fields.get(scene::CAMERAS))
        .and_then(Value::as_list)
        .map_or(1, |cameras| cameras.len().max(1))
}

/// Reads the command line into what to render and where to put it.
fn parse_arguments() -> Result<Request, String> {
    let mut arguments = std::env::args().skip(1);
    let name = arguments.next().ok_or_else(usage)?;
    let path = arguments.next().unwrap_or_else(|| "out".to_owned());
    let rest: Vec<String> = arguments.collect();

    // `--video` and `--sheet` are flags, and every bare number after the path is a time,
    // so the times are whatever is left once the flags are taken out.
    let mut mode = Mode::Frames;
    let mut times: Vec<String> = Vec::new();
    let mut index = 0;
    while index < rest.len() {
        match rest[index].as_str() {
            "--sheet" => mode = Mode::Sheet,
            "--video" => mode = Mode::Video,
            other => times.push(other.to_owned()),
        }
        index += 1;
    }

    let number = |text: &str| -> Result<f64, String> {
        text.parse::<f64>()
            .map_err(|_| format!("`{text}` is not a number of seconds"))
    };
    let moments = match times.as_slice() {
        // No range means one frame at whatever time the scene itself carries, which is
        // left for the scene to decide rather than overridden here.
        [] => vec![f64::NAN],
        [only] => vec![number(only)?],
        [start, stop] => {
            let (start, stop) = (number(start)?, number(stop)?);
            // Two numbers with no step is a frame every half second, which is what someone
            // naming a span and nothing else almost always means.
            let count = (((stop - start).abs() * 2.0).round() as usize + 1).max(2);
            (0..count)
                .map(|index| start + (stop - start) * index as f64 / (count - 1) as f64)
                .collect()
        }
        [start, stop, step] => {
            let (start, stop, step) = (number(start)?, number(stop)?, number(step)?);
            if step <= 0.0 {
                return Err("a step of zero or less never reaches the end".to_owned());
            }
            let count = (((stop - start) / step).floor() as usize + 1).max(1);
            (0..count)
                .map(|index| start + step * index as f64)
                .collect()
        }
        _ => return Err(usage()),
    };
    Ok(Request {
        name,
        path,
        mode,
        times: moments,
    })
}

/// What a render was asked for.
#[derive(Clone)]
struct Request {
    /// The scene to render.
    name: String,
    /// The path the images are written under, without an extension.
    path: String,
    /// How the moments are put down.
    mode: Mode,
    /// The moments to draw.
    times: Vec<f64>,
}

/// How the moments of a render are written out.
#[derive(Clone)]
enum Mode {
    /// One image per moment.
    Frames,
    /// One image holding every moment in a grid.
    Sheet,
    /// A video encoded from the moments with `ffmpeg`.
    Video,
}

/// Returns how a moment is described, which is its time unless the scene chose it.
fn describe(time: f64) -> String {
    if time.is_nan() {
        "the scene's own time".to_owned()
    } else {
        format!("t={time}")
    }
}

/// Writes the moments of a render out the way the request asked for.
fn write_moments(request: &Request, moments: &[Moment]) -> Result<(), String> {
    match request.mode {
        Mode::Frames => {
            for (index, moment) in moments.iter().enumerate() {
                let path = numbered(&request.path, index, moments.len());
                let encoded = moment
                    .frame
                    .to_png()
                    .map_err(|error| format!("the frame does not encode: {error}"))?;
                write_png(&path, &encoded)?;
                println!("wrote {} ({})", path.display(), describe(moment.time));
            }
            Ok(())
        }
        Mode::Sheet => {
            let (width, height) = moments
                .first()
                .map(|moment| (moment.frame.frame.width, moment.frame.frame.height))
                .unwrap_or((0, 0));
            let encoded = contact_sheet(moments)?;
            let path = numbered(&request.path, 0, 1);
            write_png(&path, &encoded)?;
            println!(
                "wrote {} ({} frames, {}x{})",
                path.display(),
                moments.len(),
                width as usize * columns(moments.len()),
                height as usize * rows(moments.len())
            );
            Ok(())
        }
        Mode::Video => encode_video(&request.path, moments),
    }
}

/// Encodes the moments into a video with `ffmpeg`.
///
/// # Invariants
///
/// The frames are written first and encoded from the files, rather than piped. A pipe
/// loses every frame if the encoder fails part way through, and a sequence of files is
/// also the thing a caller wants anyway — a range of times is worth having as images
/// whether or not it was asked for as a video.
///
/// The frame rate is derived from the times rather than fixed, so a range rendered over
/// eight seconds plays back over eight seconds rather than over however many frames
/// happened to be rendered. A single frame has no rate to speak of and is written as a
/// numbered image instead.
fn encode_video(path: &str, moments: &[Moment]) -> Result<(), String> {
    if moments.len() < 2 {
        return Err("a video needs a range of times, not a single frame".to_owned());
    }
    let stem = Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "frame".to_owned());
    for (index, moment) in moments.iter().enumerate() {
        let encoded = moment
            .frame
            .to_png()
            .map_err(|error| format!("the frame does not encode: {error}"))?;
        write_png(&numbered(path, index, moments.len()), &encoded)?;
    }

    let span = (moments.last().map(|m| m.time).unwrap_or(0.0)
        - moments.first().map(|m| m.time).unwrap_or(0.0))
    .abs();
    let rate = (moments.len() as f64 - 1.0) / span.max(1e-6);
    let pattern = numbered(path, 0, moments.len())
        .to_string_lossy()
        .into_owned()
        .replace("000", "%03d");
    let video = with_extension(Path::new(path), "mp4");
    let status = std::process::Command::new("ffmpeg")
        .args(["-y", "-hide_banner", "-loglevel", "error"])
        .args(["-framerate", &format!("{rate:.4}")])
        .args(["-i", &pattern])
        .args([
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-crf",
            "18",
            "-movflags",
            "+faststart",
        ])
        .arg(&video)
        .status()
        .map_err(|error| format!("ffmpeg could not be run: {error}"))?;
    if !status.success() {
        return Err(format!(
            "ffmpeg failed with {status}; the frames are on disk"
        ));
    }
    println!("wrote {}", video.display());
    let _ = stem;
    Ok(())
}

/// Returns where the image of one moment is written.
///
/// A range of frames is numbered, because `frame-000.png` beside `frame-001.png` is easier
/// to read in order than two files with nearly the same name.
fn numbered(output: &str, index: usize, count: usize) -> PathBuf {
    let path = Path::new(output);
    if count <= 1 {
        return with_extension(path, "png");
    }
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "frame".to_owned());
    with_extension(&path.with_file_name(format!("{stem}-{index:03}")), "png")
}

/// Replaces a path's extension.
fn with_extension(path: &Path, extension: &str) -> PathBuf {
    path.with_extension(extension)
}

/// Writes an encoded image, creating the folders above it.
fn write_png(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("creating {} failed: {error}", parent.display()))?;
    }
    std::fs::write(path, bytes)
        .map_err(|error| format!("writing {} failed: {error}", path.display()))
}

/// Returns how many images one row of a contact sheet holds.
fn columns(count: usize) -> usize {
    (count as f64).sqrt().ceil().max(1.0) as usize
}

/// Returns how many rows a contact sheet of `count` images has.
fn rows(count: usize) -> usize {
    count.div_ceil(columns(count))
}

/// Lays every frame out in a grid, so a range of times is one image to look at.
///
/// A sheet is sampled every fourth pixel: a dozen frames of a thousand pixels each is a
/// sheet nobody can see, and a range is read for its shape rather than for its pixels.
fn contact_sheet(moments: &[Moment]) -> Result<Vec<u8>, String> {
    use ::png::{BitDepth, ColorType, Encoder};

    let Some(first) = moments.first() else {
        return Err("a contact sheet of no frames is not an image".to_owned());
    };
    let (columns, rows) = (columns(moments.len()), rows(moments.len()));
    let (cell_width, cell_height) = (
        (first.frame.frame.width / 4).max(1) as usize,
        (first.frame.frame.height / 4).max(1) as usize,
    );
    let (sheet_width, sheet_height) = (cell_width * columns, cell_height * rows);
    let mut pixels = vec![0u8; sheet_width * sheet_height * 4];

    for (index, moment) in moments.iter().enumerate() {
        let (column, row) = (index % columns, index / columns);
        let source = &moment.frame.frame;
        for y in 0..cell_height {
            for x in 0..cell_width {
                let source_x = (x * 4).min(source.width as usize - 1);
                let source_y = (y * 4).min(source.height as usize - 1);
                let from = (source_y * source.width as usize + source_x) * 4;
                let to = ((row * cell_height + y) * sheet_width + column * cell_width + x) * 4;
                pixels[to..to + 4].copy_from_slice(&source.rgba[from..from + 4]);
            }
        }
    }

    let mut bytes = Vec::new();
    {
        let mut encoder = Encoder::new(&mut bytes, sheet_width as u32, sheet_height as u32);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|error| format!("the contact sheet does not encode: {error}"))?;
        writer
            .write_image_data(&pixels)
            .map_err(|error| format!("the contact sheet does not encode: {error}"))?;
    }
    Ok(bytes)
}

/// Sets the time a scene is drawn at, leaving a scene that carries its own time alone.
fn set_time(scene: &mut Value, time: f64) {
    if time.is_nan() {
        return;
    }
    if let Some(fields) = scene.as_field_map_mut() {
        fields.insert(TIME.to_owned(), Value::Float(time));
    }
}

/// Returns the usage this harness was run with.
fn usage() -> String {
    "usage: render <scene> <out> [start [stop [step]]] [--sheet | --video]".to_owned()
}
