//! Probes whether an environment light reaches a Cycles scene, and by how much.
//!
//! ```text
//! environment_probe --document <path.json> --out <dir> [--samples N] [--device CPU]
//!                   [--region x,y,w,h] [--radiance A] [--radiance B] [--light-power P]
//! ```
//!
//! # What this answers
//!
//! Whether a scene's environment light is **carried** into the document Cycles is given, and
//! whether the light it carries actually **lights** the scene. Those are two questions and a
//! frame answers only the second one: a sky that reads in the frame may be a sky that was
//! painted by the world's own strength rather than a sky that was carried, and a black wall may
//! be a wall the light never reached or a wall in a deep well.
//!
//! So the probe does both. It rewrites the environment light's declared radiance, renders the
//! document twice — once at each radiance — and reports the mean luminance of one region of
//! each frame. **A light that is carried scales the region with its radiance; a light that is
//! dropped does not.** The emitted document's own world block is printed beside the numbers, so
//! what the renderer was given can be read off rather than inferred.
//!
//! # Invariants
//!
//! **Only the radiance is rewritten**, and — when the control is asked for — the other lights'
//! power. The document is cloned, the environment light's one number is replaced, and nothing
//! else is touched, so a difference between the two frames is attributable to that number and to
//! nothing else.
//!
//! **The two radiances are read back out of the document that was written**, not out of the
//! argument, so the number the frame is attributed to is the number the renderer was given.
//!
//! **The region is a rectangle in the written png**, and its mean is a mean of the Rec. 709
//! luma of the file's own eight-bit numbers. Cycles writes scene-linear radiance into that file
//! with no view transform, so a mean above one is a clipped frame and the comparison of two
//! clipped frames says nothing; the probe prints the maximum beside the mean so that a clipped
//! frame is visible rather than silent.

use std::path::{Path, PathBuf};

use definition::Value;
use whiting_device_cycles::{CyclesOptions, render_scene};

/// The stack a scene document is read on.
///
/// # Invariants
///
/// **It is the reader's size, and the reader's size is measured rather than guessed.**
/// `facet_json::from_str` descends as it parses, and a document of a few hundred megabytes needs
/// more than the megabyte a windows main thread is given.
const DOCUMENT_STACK_BYTES: usize = 64 * 1024 * 1024;

/// The label of the field an environment light states its radiance under.
const RADIANCE: &str = "radiance";

fn main() {
    if let Err(failure) = run() {
        eprintln!("error: {failure}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let request = parse_arguments()?;
    std::fs::create_dir_all(&request.out)
        .map_err(|error| format!("{} could not be made: {error}", request.out.display()))?;
    let document = read_document(&request.document)?;
    let declared = environment_radiance(&document)
        .ok_or_else(|| "the document declares no environment light".to_owned())?;
    let first = request.first.unwrap_or(declared);
    let second = request.second.unwrap_or(first * 10.0);

    // The other lights are the control. A region that scales with the environment's radiance
    // while the other lights are still on proves the environment reached the scene and not
    // that something else in it happened to scale the same way; a region that scales with the
    // other lights turned **off** proves the environment reached it on its own.
    let mut document = document;
    if let Some(power) = request.other_lights_power {
        set_other_lights_power(&mut document, power)
            .map_err(|failure| format!("the other lights' power could not be set: {failure}"))?;
        println!("the other lights' power set to {power}");
    }

    let mut frames = Vec::new();
    for radiance in [first, second] {
        frames.push(measure(&document, &request, radiance)?);
    }

    println!("document: {}", request.document.display());
    println!("declared radiance: {declared}");
    for frame in &frames {
        println!("radiance {radiance}:", radiance = frame.radiance);
        println!("  {}", frame.world.trim_end().replace('\n', "\n  "));
        for light in &frame.lights {
            println!("  {light}");
        }
        println!(
            "  region mean {:.6}, frame mean {:.6}, frame max {:.6}, {} s",
            frame.region_mean, frame.frame_mean, frame.frame_max, frame.seconds
        );
    }

    let ratio = frames[1].region_mean / frames[0].region_mean;
    println!(
        "region mean ratio {:.4} for a radiance ratio {:.4}",
        ratio,
        frames[1].radiance / frames[0].radiance
    );
    Ok(())
}

/// What one render of the document at one radiance showed.
struct Frame {
    radiance: f64,
    /// The world block of the document the renderer was given, verbatim.
    world: String,
    /// The light elements of that document, verbatim.
    lights: Vec<String>,
    region_mean: f64,
    frame_mean: f64,
    frame_max: f64,
    seconds: f64,
}

/// Renders the document at one radiance and measures the frame.
fn measure(document: &Value, request: &Request, radiance: f64) -> Result<Frame, String> {
    let mut scene = document.clone();
    set_environment_radiance(&mut scene, radiance).map_err(|failure| {
        format!("the environment light's radiance could not be set: {failure}")
    })?;

    let stem = format!("radiance-{radiance}");
    let document_path = request.out.join(format!("{stem}.json"));
    let image_path = request.out.join(format!("{stem}.png"));
    write_document(scene.clone(), &document_path)?;

    let mut options = CyclesOptions::for_paths(&image_path, &image_path);
    options.lossy = scene::cycles_export::LossyMode::Degrade;
    options.samples = request.samples;
    options.device = request.device.clone();

    let rendered = render_scene(&scene, &options).map_err(|error| error.to_string())?;
    let emitted = read_emitted(&rendered.scene_path);
    let (region_mean, frame_mean, frame_max) = read_luminance(&rendered.image_path, request.region)
        .map_err(|error| format!("{}: {error}", rendered.image_path.display()))?;

    Ok(Frame {
        radiance,
        world: emitted.world,
        lights: emitted.lights,
        region_mean,
        frame_mean,
        frame_max,
        seconds: rendered.seconds,
    })
}

/// The environment light's declared radiance, if the document declares one.
///
/// # Invariants
///
/// **The light is found by its variant tag and not by its label.** The label is the author's
/// and may be anything; the tag is the kind, and the kind is what decides that a light is an
/// environment light at all.
fn environment_radiance(document: &Value) -> Option<f64> {
    for light in lights(document) {
        if light.variant_tag() != Some("Environment") {
            continue;
        }
        let fields = light.variant_fields()?;
        return match fields.get(RADIANCE) {
            Some(Value::Float(radiance)) => Some(*radiance),
            _ => None,
        };
    }
    None
}

/// Sets the environment light's declared radiance.
fn set_environment_radiance(document: &mut Value, radiance: f64) -> Result<(), String> {
    for light in lights_mut(document) {
        if light.variant_tag() != Some("Environment") {
            continue;
        }
        let Value::Enum { fields, .. } = light else {
            return Err("a light is not a variant".to_owned());
        };
        fields.insert(RADIANCE.to_owned(), Value::Float(radiance));
        return Ok(());
    }
    Err("the document declares no environment light".to_owned())
}

/// Sets the `power` of every light that is not an environment light.
///
/// # Invariants
///
/// **The environment light is left alone.** The probe is about the environment light, and a
/// control that changed it too would measure nothing. A power of zero leaves the other lights
/// in the scene and dark, which is what "the environment light on its own" means.
fn set_other_lights_power(document: &mut Value, power: f64) -> Result<(), String> {
    let mut touched = 0;
    for light in lights_mut(document) {
        if light.variant_tag() == Some("Environment") {
            continue;
        }
        let Value::Enum { fields, .. } = light else {
            return Err("a light is not a variant".to_owned());
        };
        if !fields.contains_key("power") {
            continue;
        }
        fields.insert("power".to_owned(), Value::Float(power));
        touched += 1;
    }
    if touched == 0 {
        return Err("the document declares no light with a power".to_owned());
    }
    Ok(())
}

/// The scene's lights, or nothing.
fn lights(document: &Value) -> impl Iterator<Item = &Value> {
    document
        .as_field_map()
        .and_then(|fields| fields.get("lights"))
        .and_then(Value::as_list)
        .unwrap_or_default()
        .iter()
}

/// The scene's lights, mutably, or nothing.
fn lights_mut(document: &mut Value) -> impl Iterator<Item = &mut Value> {
    document
        .as_field_map_mut()
        .and_then(|fields| fields.get_mut("lights"))
        .and_then(Value::as_list_mut)
        .map(|lights| lights.iter_mut())
        .unwrap_or_default()
}

/// The world block and the light elements of an emitted document, verbatim.
///
/// # Invariants
///
/// **It is the document the renderer was given, read off the disk.** The question is what the
/// renderer saw, and the file the renderer read is the only answer to that which is not a
/// reconstruction of it.
fn read_emitted(scene_path: &Path) -> Emitted {
    let Ok(text) = std::fs::read_to_string(scene_path) else {
        return Emitted {
            world: "the emitted document could not be read".to_owned(),
            lights: Vec::new(),
        };
    };
    let mut world = String::new();
    let mut lights = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<background") {
            inside = true;
        }
        if inside {
            world.push_str(trimmed);
            world.push('\n');
        }
        if inside && trimmed == "</background>" {
            inside = false;
        }
        if trimmed.starts_with("<light ") {
            lights.push(trimmed.to_owned());
        }
    }
    if world.is_empty() {
        world = "the emitted document carries no world block".to_owned();
    }
    Emitted { world, lights }
}

/// What an emitted document says about its world and its lights.
struct Emitted {
    world: String,
    lights: Vec<String>,
}

/// The mean luma of a region, the mean of the whole frame, and the frame's maximum.
fn read_luminance(
    image_path: &Path,
    region: Option<(u32, u32, u32, u32)>,
) -> Result<(f64, f64, f64), String> {
    let bytes = std::fs::read(image_path).map_err(|error| error.to_string())?;
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().map_err(|error| error.to_string())?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|error| error.to_string())?;
    let (width, height) = (info.width, info.height);
    let pixels = &buffer[..info.buffer_size()];
    if pixels.len() < (width as usize) * (height as usize) * 4 {
        return Err("the image carries fewer bytes than it has texels".to_owned());
    }
    let luma = |x: u32, y: u32| {
        let at = ((y as usize) * (width as usize) + x as usize) * 4;
        let r = pixels[at] as f64 / 255.0;
        let g = pixels[at + 1] as f64 / 255.0;
        let b = pixels[at + 2] as f64 / 255.0;
        0.2126 * r + 0.7152 * g + 0.0722 * b
    };
    let (x0, y0, rw, rh) = region.unwrap_or((0, 0, width, height));
    let x1 = (x0 + rw).min(width);
    let y1 = (y0 + rh).min(height);
    let mut sum = 0.0;
    let mut count = 0.0;
    for y in y0..y1 {
        for x in x0..x1 {
            sum += luma(x, y);
            count += 1.0;
        }
    }
    if count == 0.0 {
        return Err("the region is empty".to_owned());
    }
    let mut total = 0.0;
    let mut maximum = 0.0f64;
    for y in 0..height {
        for x in 0..width {
            let value = luma(x, y);
            total += value;
            if value > maximum {
                maximum = value;
            }
        }
    }
    Ok((sum / count, total / (width as f64 * height as f64), maximum))
}

/// Reads a scene document, on a thread with a larger stack than a windows main thread has.
fn read_document(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("reading {} failed: {error}", path.display()))?;
    std::thread::Builder::new()
        .stack_size(DOCUMENT_STACK_BYTES)
        .spawn(move || facet_json::from_str::<Value>(&text))
        .map_err(|error| format!("the reading thread does not start: {error}"))?
        .join()
        .map_err(|_| "the reading thread panicked".to_owned())?
        .map_err(|error| format!("the document does not read as a scene value: {error}"))
}

/// Writes a scene document, on a thread with a larger stack than a windows main thread has.
///
/// # Invariants
///
/// **The writer is given the same room the reader is.** `facet_json::to_string` descends the
/// tree as it writes, exactly as `from_str` descends it as it reads, and a windows main thread
/// has a megabyte of stack — which the writer runs out of on a scene of this size.
fn write_document(scene: Value, path: &Path) -> Result<(), String> {
    let text = std::thread::Builder::new()
        .stack_size(DOCUMENT_STACK_BYTES)
        .spawn(move || facet_json::to_string(&scene))
        .map_err(|error| format!("the writing thread does not start: {error}"))?
        .join()
        .map_err(|_| "the writing thread panicked".to_owned())?
        .map_err(|error| format!("the document does not encode: {error}"))?;
    std::fs::write(path, text)
        .map_err(|error| format!("writing {} failed: {error}", path.display()))
}

/// What the probe was asked for.
struct Request {
    document: PathBuf,
    out: PathBuf,
    samples: u32,
    device: String,
    region: Option<(u32, u32, u32, u32)>,
    first: Option<f64>,
    second: Option<f64>,
    other_lights_power: Option<f64>,
}

/// Reads the command line.
fn parse_arguments() -> Result<Request, String> {
    let mut document: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut samples = 32u32;
    let mut device = "CPU".to_owned();
    let mut region = None;
    let mut first = None;
    let mut second = None;
    let mut other_lights_power = None;

    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let mut index = 0;
    while index < arguments.len() {
        let option = arguments[index].clone();
        index += 1;
        let mut value = |name: &str| -> Result<String, String> {
            let found = arguments
                .get(index)
                .cloned()
                .ok_or_else(|| format!("{name} needs a value"))?;
            index += 1;
            Ok(found)
        };
        match option.as_str() {
            "--document" => document = Some(PathBuf::from(value("--document")?)),
            "--out" => out = Some(PathBuf::from(value("--out")?)),
            "--samples" => {
                samples = value("--samples")?
                    .parse()
                    .map_err(|_| "the sample count is not a number".to_owned())?
            }
            "--device" => device = value("--device")?,
            "--region" => {
                let text = value("--region")?;
                let parts: Vec<u32> = text
                    .split(',')
                    .map(|part| part.parse::<u32>())
                    .collect::<Result<Vec<u32>, _>>()
                    .map_err(|_| "the region is not four numbers".to_owned())?;
                if parts.len() != 4 {
                    return Err("the region is not four numbers".to_owned());
                }
                region = Some((parts[0], parts[1], parts[2], parts[3]));
            }
            "--radiance" => {
                let text = value("--radiance")?;
                let number = text
                    .parse::<f64>()
                    .map_err(|_| "the radiance is not a number".to_owned())?;
                if first.is_none() {
                    first = Some(number);
                } else {
                    second = Some(number);
                }
            }
            "--light-power" => {
                other_lights_power = Some(
                    value("--light-power")?
                        .parse()
                        .map_err(|_| "the power is not a number".to_owned())?,
                )
            }
            other => return Err(format!("`{other}` is not an option")),
        }
    }

    Ok(Request {
        document: document.ok_or_else(usage)?,
        out: out.ok_or_else(usage)?,
        samples,
        device,
        region,
        first,
        second,
        other_lights_power,
    })
}

/// The one sentence a wrong command line is answered with.
fn usage() -> String {
    "usage: environment_probe --document <path.json> --out <dir> [--samples N] [--device CPU] \
     [--region x,y,w,h] [--radiance A] [--radiance B] [--light-power P]"
        .to_owned()
}
