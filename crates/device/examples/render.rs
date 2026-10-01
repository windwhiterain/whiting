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
//!   render dunes out/dunes --debug position   and one image of where each pixel is
//! ```
//!
//! # Debug images
//!
//! `--debug <channel>` asks the compile for an image *about* the frame rather than of it, and
//! every image the document declares is written beside the picture: the frame where it always
//! went, and each channel as `<out>-<channel>.png`. A channel is asked for **here and not in the
//! scene**, because it is a question the caller is asking rather than a fact about the scene;
//! nothing about the picture changes either way, and that is checked by rendering both.
//!
//! A position image holds floats, and a png cannot: the png written beside it is normalized for
//! viewing and is therefore a picture of the positions rather than the positions. The numbers
//! travel in a sidecar of the same name, `<out>-<channel>.txt`, which names the mapping it used
//! and gives the world position at a grid of pixels. That is what a caller reads to find out
//! where a pixel it is looking at actually is.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use definition::{Definition, DefinitionIdentifier, Registry, SchemaIdentifier, Value};
use gallery::Gallery;
use graphics::{GRAPHICS_SCHEMA, INSTANCE_SCHEMA};
use scene::{
    CompileContext, CompileNote, Compiler, DebugChannel, STANDARD_SCENE_SCHEMA, StandardScene,
};
use whiting_device::{RenderedFrame, RenderedImage, Renderer, WgpuRenderer};

/// Field the scene's time is written under.
const TIME: &str = "time";

/// How many samples a side of the probe grid has.
///
/// Nine samples across and down includes both edges and the middle, which is what makes a grid
/// readable against the picture: a caller that sees a band at a third of the way across finds it
/// on a sample.
const PROBE_SIDE: u32 = 9;

/// A moment a scene is drawn at.
struct Moment {
    /// The time the scene was patched to, when the range named one.
    time: f64,
    /// Every image the moment rendered, the frame among them.
    rendered: RenderedFrame,
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
    )
    // The channels are the caller's request rather than the scene's content, so they are set on
    // the compiler that is about to compile rather than patched into the document. Every camera
    // and every moment of this run is compiled with them.
    .with_debug(request.debug.iter().copied());
    let source_identifier = DefinitionIdentifier::new("gallery.source");
    let authored = Gallery::build(&name);

    // `--census` answers how big a scene is before any of it is drawn: how many instances each
    // pass holds, how many meshes, how many materials. It is the number "the grove looks thin"
    // wants beside it, and it costs no gpu work at all.
    if request.census {
        println!("{name}:\n{}", Gallery::census(&name));
        println!("{}", Gallery::bounds(&name));
        return Ok(());
    }

    // Every camera a scene names is rendered. A camera decides the projection every pass draws
    // through — and, for a fitted camera, how far back the eye stands — so it is an input to
    // the compile rather than something a renderer picks afterwards. A scene naming two
    // cameras is compiled and drawn twice; a scene naming none is drawn once, by the stated
    // convention, exactly as it always was.
    let cameras = camera_count(&authored);
    // The notes already reported, so that one fact about the documents is said once rather than
    // once per image of a range. See `print_notes` for why that is not the same as one per
    // compile.
    let mut reported = HashSet::new();
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
                    // The renderer was created before this loop, so the numbers the frame is
                    // actually drawn with are in hand at the point the compile is made
                    // against. A scene asking for a page wider or a pool deeper than this is
                    // given what this machine has and a note naming what it lost.
                    limits: renderer.limits(),
                })
                .map_err(|error| format!("the scene does not compile: {error}"))?;
            let compiled_at = started.elapsed();

            // The notes are printed **here**, at the compile that produced them, rather than
            // beside the writing of the images below. A note is a fact about this compile, and
            // a note printed once per image would be printed once per image of a range for a
            // fact that was established on the first one.
            print_notes(&mut reported, &compiled.notes, camera, cameras);

            // Every image the document declares, from **one** run of the pass list: the frame is
            // one of them and a debug channel is another, and rendering twice to get both would
            // be two frames of a scene that a channel cannot change and a doubled render cost.
            let rendered = RenderedFrame {
                renderer: renderer.name(),
                images: renderer
                    .render_outputs(&compiled.graphics.value, &compiled.instance.value)
                    .map_err(|error| {
                        format!("the frame at {} does not render: {error}", describe(time))
                    })?,
            };
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

            moments.push(Moment { time, rendered });
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

/// Prints what a compile wants the author of the documents to know about it, once each.
///
/// # Invariants
///
/// The **path** is printed with the message, and it is the half that makes a note worth
/// reading: a note that says the page pool ran out is actionable only if it says which field
/// of which block to raise. They go to the standard output beside the timing line rather than
/// to the standard error, because a note is part of what the run reported rather than a
/// failure of it — the frame this compile produced is still on its way to disk.
///
/// Nothing is printed for a compile with no notes, so a scene that compiles exactly as written
/// says nothing at all.
///
/// # Why a note is printed once and not once per compile
///
/// A compile happens per camera **and** per moment, so a run over a ten moment animation makes
/// ten compiles per camera. Cluster assignment depends on the camera and not on the moment, so
/// those ten compiles produce the same notes, and printing each of them says the same sentence
/// ten times. Deduping on the path and the message together removes that without losing
/// anything: a note that is **new** at a later moment is still printed, at the compile that
/// first raised it.
///
/// The camera is in the line whenever a scene names more than one, because two cameras
/// genuinely can differ — a cluster overflow is a statement about what that camera can see, so
/// suppressing the second camera's overflow on the grounds that it is "the same note" would
/// lose a real one. The dedup key deliberately does not include it.
fn print_notes(
    reported: &mut HashSet<(String, String)>,
    notes: &[CompileNote],
    camera: usize,
    cameras: usize,
) {
    for note in notes {
        if reported.insert((note.path.to_string(), note.message.clone())) {
            println!(
                "note{}: at {}: {}",
                if cameras > 1 {
                    format!(" (camera {camera})")
                } else {
                    String::new()
                },
                note.path,
                note.message,
            );
        }
    }
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

    // `--video`, `--sheet` and `--debug` are flags, and every bare number after the path is a
    // time, so the times are whatever is left once the flags and their values are taken out.
    let mut mode = Mode::Frames;
    let mut times: Vec<String> = Vec::new();
    let mut debug: Vec<DebugChannel> = Vec::new();
    let mut census = false;
    let mut index = 0;
    while index < rest.len() {
        match rest[index].as_str() {
            "--census" => census = true,
            "--sheet" => mode = Mode::Sheet,
            "--video" => mode = Mode::Video,
            "--debug" => {
                index += 1;
                let asked = rest
                    .get(index)
                    .ok_or_else(|| format!("--debug needs a channel: {}", channel_names()))?;
                for tag in asked.split(',') {
                    let channel = DebugChannel::from_tag(tag.trim()).ok_or_else(|| {
                        format!(
                            "`{}` is not a debug channel: {}",
                            tag.trim(),
                            channel_names()
                        )
                    })?;
                    debug.push(channel);
                }
            }
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
        debug,
        census,
    })
}

/// Returns the channels a caller may ask for, as a list for a message about one that is not.
fn channel_names() -> String {
    DebugChannel::ALL
        .iter()
        .map(|channel| format!("`{}`", channel.tag()))
        .collect::<Vec<String>>()
        .join(", ")
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
    /// The debug channels every compile of this run is asked to draw.
    debug: Vec<DebugChannel>,
    /// Whether to report what the scene holds, rather than drawing it.
    census: bool,
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
///
/// Every image a moment rendered that is **not** the frame is written beside the picture first,
/// in every mode, one file per moment: a sheet is one image of many frames and a channel is a
/// measurement of each, so a channel folded into the picture's own layout would be a picture of a
/// different thing.
fn write_moments(request: &Request, moments: &[Moment]) -> Result<(), String> {
    write_channels(request, moments)?;
    match request.mode {
        Mode::Frames => {
            for (index, moment) in moments.iter().enumerate() {
                let path = numbered(&request.path, index, moments.len());
                let encoded = moment
                    .rendered
                    .to_png()
                    .map_err(|error| format!("the frame does not encode: {error}"))?;
                write_png(&path, &encoded)?;
                println!("wrote {} ({})", path.display(), describe(moment.time));
            }
            Ok(())
        }
        Mode::Sheet => {
            let present = moments.first().and_then(|moment| moment.rendered.present());
            let (width, height) = present
                .map(|image| (image.width, image.height))
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

/// Writes every image a moment rendered that is not the frame, beside the picture.
///
/// # Invariants
///
/// A channel that holds **floats** is written twice, and both halves are needed: a png, because
/// the shape of where things are is what a viewer reads, and a text file of the values at a grid
/// of pixels, because a position normalized into eight bits is a picture of the positions rather
/// than the positions. The text names the mapping the png used, so the two cannot be read apart.
fn write_channels(request: &Request, moments: &[Moment]) -> Result<(), String> {
    for (index, moment) in moments.iter().enumerate() {
        for image in &moment.rendered.images {
            if image.name == graphics::PRESENT_OUTPUT {
                continue;
            }
            let path = numbered(
                &with_stem_suffix(&request.path, &image.name),
                index,
                moments.len(),
            );
            write_png(&path, &viewable(image)?)?;
            println!("wrote {} ({})", path.display(), describe(moment.time));

            if let Some(probe) = probe(image) {
                let sidecar = with_extension(&path, "txt");
                write_text(&sidecar, &probe)?;
                println!("wrote {} ({})", sidecar.display(), describe(moment.time));
            }
        }
    }
    Ok(())
}

/// Returns the image as a png that can be looked at.
///
/// # Invariants
///
/// An eight bit image is written as it stands. A float image has no eight bit form, so it is
/// normalized over the texels the scene **drew**: a value at a pixel the scene never reached is
/// not part of the picture, and including it would compress everything that is into a corner of
/// the range. The mapping is returned by [`probe`] beside the file, because a picture whose scale
/// is private to the code that made it is a picture nobody can read a number from.
fn viewable(image: &RenderedImage) -> Result<Vec<u8>, String> {
    if let Some(frame) = image.as_frame() {
        return encode_png(frame.width, frame.height, &frame.rgba);
    }
    let (minimum, maximum) = covered_range(image).ok_or_else(|| {
        format!(
            "`{}` holds no texel the scene drew, so there is nothing to write",
            image.name
        )
    })?;
    let mut pixels = Vec::with_capacity(image.width as usize * image.height as usize * 4);
    for y in 0..image.height {
        for x in 0..image.width {
            let texel = image
                .texel(x, y)
                .ok_or_else(|| format!("`{}` does not read at ({x}, {y})", image.name))?;
            let covered = texel[3] > 0.5;
            for axis in 0..3 {
                let span = maximum[axis] - minimum[axis];
                let value = if covered && span > 0.0 {
                    (((texel[axis] - minimum[axis]) / span) * 255.0).round() as i64
                } else {
                    0
                };
                pixels.push(value.clamp(0, 255) as u8);
            }
            pixels.push(u8::from(covered) * 255);
        }
    }
    encode_png(image.width, image.height, &pixels)
}

/// Returns the smallest and largest value each of the first three channels holds over the texels
/// the scene drew.
fn covered_range(image: &RenderedImage) -> Option<([f32; 3], [f32; 3])> {
    let mut minimum = [f32::INFINITY; 3];
    let mut maximum = [f32::NEG_INFINITY; 3];
    let mut covered = false;
    for y in 0..image.height {
        for x in 0..image.width {
            let texel = image.texel(x, y)?;
            if texel[3] <= 0.5 {
                continue;
            }
            covered = true;
            for axis in 0..3 {
                minimum[axis] = minimum[axis].min(texel[axis]);
                maximum[axis] = maximum[axis].max(texel[axis]);
            }
        }
    }
    covered.then_some((minimum, maximum))
}

/// Returns a grid of the values a float image holds, as text, or none for an eight bit image.
///
/// # Invariants
///
/// Every value is printed as the number the shader wrote and never as a normalized one, because
/// this file is the half of the pair a caller reads a position out of. The header names the
/// mapping the png beside it used, so a caller comparing the two knows what the colours mean.
fn probe(image: &RenderedImage) -> Option<String> {
    if !matches!(
        image.format.as_str(),
        "rgba16float" | "rgba32float" | "r32float"
    ) {
        return None;
    }
    let (minimum, maximum) = covered_range(image)?;
    let mut text = format!(
        "{}: {}x{}, {}\n\
         world position of the nearest surface each pixel shows, in scene units\n\
         `-` is a pixel the scene drew nothing on\n\
         the png beside this file is the same values normalized, red over [{:.3}, {:.3}], \
         green over [{:.3}, {:.3}], blue over [{:.3}, {:.3}]\n\n",
        image.name,
        image.width,
        image.height,
        image.format,
        minimum[0],
        maximum[0],
        minimum[1],
        maximum[1],
        minimum[2],
        maximum[2],
    );
    for row in 0..PROBE_SIDE {
        for column in 0..PROBE_SIDE {
            let x = column * image.width.saturating_sub(1) / (PROBE_SIDE - 1).max(1);
            let y = row * image.height.saturating_sub(1) / (PROBE_SIDE - 1).max(1);
            let Some(texel) = image.texel(x, y) else {
                continue;
            };
            if texel[3] <= 0.5 {
                text.push_str(&format!("({x}, {y}) -\n"));
            } else {
                text.push_str(&format!(
                    "({x}, {y}) ({:.3}, {:.3}, {:.3})\n",
                    texel[0], texel[1], texel[2]
                ));
            }
        }
    }
    Some(text)
}

/// Encodes an 8 bit RGBA image as a png.
fn encode_png(width: u32, height: u32, pixels: &[u8]) -> Result<Vec<u8>, String> {
    use ::png::{BitDepth, ColorType, Encoder};

    let mut bytes = Vec::new();
    {
        let mut encoder = Encoder::new(&mut bytes, width, height);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|error| format!("the image does not encode: {error}"))?;
        writer
            .write_image_data(pixels)
            .map_err(|error| format!("the image does not encode: {error}"))?;
    }
    Ok(bytes)
}

/// Writes a text file, creating the folders above it.
fn write_text(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("creating {} failed: {error}", parent.display()))?;
    }
    std::fs::write(path, text)
        .map_err(|error| format!("writing {} failed: {error}", path.display()))
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
            .rendered
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
    let Some(first) = moments.first() else {
        return Err("a contact sheet of no frames is not an image".to_owned());
    };
    let first = first
        .rendered
        .present()
        .and_then(RenderedImage::as_frame)
        .ok_or_else(|| "the frame of this render is not an image a sheet is made of".to_owned())?;
    let (columns, rows) = (columns(moments.len()), rows(moments.len()));
    let (cell_width, cell_height) = (
        (first.width / 4).max(1) as usize,
        (first.height / 4).max(1) as usize,
    );
    let (sheet_width, sheet_height) = (cell_width * columns, cell_height * rows);
    let mut pixels = vec![0u8; sheet_width * sheet_height * 4];

    for (index, moment) in moments.iter().enumerate() {
        let (column, row) = (index % columns, index / columns);
        let source = moment
            .rendered
            .present()
            .and_then(RenderedImage::as_frame)
            .ok_or_else(|| {
                "the frame of this render is not an image a sheet is made of".to_owned()
            })?;
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

    use ::png::{BitDepth, ColorType, Encoder};

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
    "usage: render <scene> <out> [start [stop [step]]] [--sheet | --video] [--debug <channel>]"
        .to_owned()
}
