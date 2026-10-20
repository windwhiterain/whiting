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
//! cargo run -p whiting-device-rasterize --example render -- <scene> <out> [times] [mode]
//!
//!   render dunes out/dunes                          one frame, at the time the scene carries
//!   render dunes out/dunes 2.5                      one frame, at t = 2.5
//!   render dunes out/dunes 0 6 0.25                 thirteen frames, a quarter second apart
//!   render dunes out/dunes 0 6 0.25 --sheet         the same range, as one contact sheet
//!   render dunes out/dunes 0 6 0.25 --sheet-full    the same sheet, at every pixel of each frame
//!   render dunes out/dunes 0 6 0.1  --video         the same range, as an mp4
//!   render dunes out/dunes --debug position         and one image of where each pixel is
//!   render --document dunes.json out/dunes          one frame, from the document and not the gallery
//!   render dunes out/dunes --document dunes.json    the same, under the name `dunes`
//! ```
//!
//! # The document
//!
//! **A scene is a document, and the gallery is one way of getting one.** `--document` reads the
//! document from a file instead of building it from a name, and that path calls no gallery at
//! all: the file is put in the registry the way the rpc's `PUT /definition/{name}` puts one
//! there — validated against the scene schema, stored under the scene's own name — and the
//! scene that is compiled is the one the registry holds afterwards.
//!
//! The name is what the document is registered under and what the manifest records, so the two
//! paths can be compared against each other: a frame drawn from a document and a frame drawn
//! from the name are the same frame if the documents they compiled were the same, and the
//! manifest beside the images says so in hashes rather than in an opinion. **A document with no
//! name of its own takes the name from the document's own `name` field**, and one with neither
//! is refused: the registry is keyed by name, and a scene registered under nothing is a scene
//! nothing can ask for again.
//!
//! The census is the other half of the gallery's side of the bargain and is not answered here:
//! it reads the document, and the document is the producer's to write. `cargo run -p
//! whiting-gallery --example scene_document -- <name> <path> --census` prints it.
//!
//! # The contact sheet
//!
//! `--sheet` puts a range of frames in one grid, and it is the only view in which **the range
//! itself** is visible: three frames of a range composed alike, or one that drifts off and comes
//! back, cannot be seen one frame at a time. It is worth reaching for.
//!
//! It samples every **fourth** pixel, which is a deliberate default and not an accident — a dozen
//! frames of a thousand pixels each is a sheet nobody can see, and a range is read for its shape
//! more often than for its pixels — and what it samples is the frame rather than resizing it: a
//! tile is made of pixels the frame already had, so no sheet can say something about a frame the
//! frame does not say. The line that reports a sheet says how far apart it read, because a sheet
//! that does not say is a picture whose resolution is a guess.
//!
//! `--sheet-full` asks for every pixel instead, and it is for the judgement being made **on** the
//! sheet rather than for the finding of it: the last look before a range is delivered, or a crop
//! out of one tile afterwards. Six 1024² frames are a 3072×2048 sheet, which is a real image and
//! a real cost, and thirteen are 4096×4096 — which is why this is a flag and not the default.
//!
//! The step is a **choice rather than a consequence**: a tile is the frame divided by the step
//! and nothing else, so it does not move when a range gets longer, and the same frame is drawn at
//! the same size in a six-frame sheet and a thirteen-frame one. Scaling the sheet to land inside
//! some target size would be elegant and would give that up — a tile would become a function of
//! how many frames were asked for, and a range read across runs would change resolution under
//! the reader.
//!
//! # Debug images
//!
//! `--debug <channel>` asks the compile for an image *about* the frame rather than of it, and
//! every image the document declares is written beside the picture: the frame where it always
//! went, and each channel as `<out>-<channel>.png`. A channel is asked for **here and not in the
//! scene**, because it is a question the caller is asking rather than a fact about the scene;
//! nothing about the picture changes either way, and that is checked by rendering both.
//!
//! A channel image holds floats, and a png cannot: the png written beside it is normalized for
//! viewing and is therefore a picture of the values rather than the values. The numbers travel
//! in a sidecar of the same name, `<out>-<channel>.txt`, which gives them at a grid of pixels.
//! The sidecar's header is written by the **declaration**, not by this harness, because the
//! declaration is what knows what its image is and a file format does not: a png and a text
//! file cannot tell a world position from a depth, and a file that claims to hold the wrong one
//! is a measurement nobody can trust.
//!
//! **The header says which of the two coverage contracts applies, because they are different
//! claims.** A built-in channel reads a frame resource, so its header names the buffer and is
//! bounded by what the pipeline has. A scene-declared output is bounded by which buckets the
//! writing pass enumerated, so its header names that instead — there is no buffer to be missing
//! and saying so would send a reader looking for a depth buffer a normal never touched. Every
//! float image is written with one of the two, and an image nothing described is written with a
//! header that says it is undescribed rather than borrowing either.
//!
//! # The manifest
//!
//! Every run writes `<out>-manifest.json` beside the images: per frame, the hash of the
//! documents the compile emitted, the hash of the pixels that came back, the pass count, and
//! the identity of the binary that drew it. The next run reads that file before it renders and
//! says what changed — **including that nothing did**, which is the case that has cost three
//! separate runs of this harness ten minutes each. See [`report_against`] for what each
//! combination of the three hashes means.
//!
//! Nothing about the picture depends on any of it: the manifest is read and written beside the
//! render, never into it.

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use definition::{Definition, DefinitionIdentifier, FieldMap, Registry, SchemaIdentifier, Value};
use device::{RenderedFrame, RenderedImage, Renderer};
use facet::Facet;
use gallery::Gallery;
use graphics::{GRAPHICS_SCHEMA, INSTANCE_SCHEMA};
use scene::{
    CompileContext, CompileNote, Compiler, DebugChannel, STANDARD_SCENE_SCHEMA, StandardScene,
};
use whiting_device_rasterize::rasterizer;

/// Field the scene's time is written under.
const TIME: &str = "time";

/// How many samples a side of the probe grid has.
///
/// Nine samples across and down includes both edges and the middle, which is what makes a grid
/// readable against the picture: a caller that sees a band at a third of the way across finds it
/// on a sample.
const PROBE_SIDE: u32 = 9;

/// The suffix the manifest is named with, beside the images it describes.
const MANIFEST: &str = "manifest";

/// The shape of the manifest this harness reads and writes.
///
/// A number rather than nothing, so a manifest written by a harness that has changed its shape
/// is reported rather than half-read: a reader that quietly misreads a record is worse than one
/// that says it cannot.
const MANIFEST_FORMAT: u32 = 1;

/// The stack a scene document is read on.
///
/// # Invariants
///
/// **It is the reader's size, and the reader's size is measured rather than guessed.**
/// `facet_json::from_str` descends as it parses, and a document of a few hundred kilobytes
/// needs more than the megabyte a windows main thread is given — the first run of
/// `crates/device/examples/facet_value_probe.rs` died with a stack overflow before it could
/// finish its report, and that probe is where the size comes from.
const DOCUMENT_STACK_BYTES: usize = 64 * 1024 * 1024;

/// A moment a scene is drawn at.
struct Moment {
    /// The time the scene was patched to, when the range named one.
    time: f64,
    /// Every image the moment rendered, the frame among them.
    rendered: RenderedFrame,
    /// What each declared image **is**, keyed by its name, as prose written beside its numbers.
    ///
    /// # Invariants
    ///
    /// **It is read from the compiled document rather than from the image.** A sidecar's header
    /// is a claim about what the numbers mean, and the two claims a caller could make — a
    /// built-in channel's, which is bounded by a buffer, and a scene-declared output's, which is
    /// bounded by the writing pass's enumeration — come from different places and must not be
    /// blurred into one sentence about "coverage". The document is where the distinction is
    /// written down, so it is read from there and carried as owned text because the compiled
    /// documents are moved into the next moment's delta.
    described: Vec<(String, String)>,
}

/// The two documents one moment compiled to, carried to the next as its previous state.
struct Compiled {
    graphics: Definition,
    instance: Definition,
}

/// Parses a `x,y` pixel, refusing a negative or non-numeric one by name.
///
/// # Invariants
///
/// **The error says what was asked for and what was written**, because a pixel is the one
/// argument a caller gets wrong by being a column of a spreadsheet, and a message that only
/// says "invalid" sends them looking at the flag rather than at their own numbers.
fn parse_pixel(written: &str) -> Result<(u32, u32), String> {
    let (x, y) = written
        .split_once(',')
        .ok_or_else(|| format!("`{written}` is not a pixel: write it x,y"))?;
    let read = |part: &str, which: &str| {
        part.trim()
            .parse::<u32>()
            .map_err(|_| format!("`{}` is not a {which} coordinate: write it x,y", written))
    };
    Ok((read(x, "x")?, read(y, "y")?))
}

/// Reads a rectangle, written `x0,y0,x1,y1`.
///
/// # Invariants
///
/// **The corners are made a rectangle rather than trusted as one**: a caller who writes the far corner first meant
/// the same rectangle, and a scan whose bounds are the other way round is empty for a reason the caller cannot see.
fn parse_region(written: &str) -> Result<[u32; 4], String> {
    let parts: Vec<&str> = written.split(',').collect();
    if parts.len() != 4 {
        return Err(format!(
            "`{written}` is not a rectangle: write it x0,y0,x1,y1"
        ));
    }
    let mut numbers = [0u32; 4];
    for (index, part) in parts.iter().enumerate() {
        numbers[index] = part
            .trim()
            .parse::<u32>()
            .map_err(|_| format!("`{written}` has a corner that is not a number"))?;
    }
    let [x0, y0, x1, y1] = numbers;
    Ok([x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)])
}

/// Prints what a rectangle of the frame is, in the terms the scene that drew it uses.
///
/// # Invariants
///
/// - **The answer is the id channel's, counted**, so a rectangle becomes the set of authored elements whose pixels
///   it covers: the distinct ids, each with how many pixels it owns, deepest first. That set is exactly what an
///   edit's selector is fed by — a box in world space cannot answer it, which is why the picture has to.
/// - **Every id the rectangle covers is printed, and the last line says so.** A table that stops where the cap
///   stood reads as a complete answer, because a reader cannot see an absent row: this arc has spent many slices
///   removing a reading that could not be taken from everything that looked like a whole one, and a rectangle whose
///   rows simply stop is that fault wearing a table. The cost is bounded by what the scan already did — the counts
///   are already collected and sorted before anything is printed, and the largest id count any frame in this
///   workspace holds is 140 — so nothing is truncated and nothing is claimed that a reader cannot check.
/// - **The order is a total one, and it is printed with the answer**: deepest first, ties by id ascending. Frames
///   carry ids covering equal numbers of pixels (`terrace`'s ids 33 and 89 each cover 1,918), so the count alone
///   leaves rows free to swap, and two runs of the same frame must not disagree about which came first.
/// - **The naming is the content's**, asked of the tables `--locate` and `--where` ask and in that order: a
///   building's own elements from `gallery::building_identity`, the reserved zero from `gallery::locate`, and
///   the vegetation naming registry only after both have declined. What an id *means* lives with the content,
///   and this harness only counts.
/// - **A rectangle outside the frame is clamped rather than refused**: a caller comparing two frames of different
///   sizes has already made the interesting decision, and an empty answer would hide it.
fn region(scene: &str, rendered: &RenderedFrame, region: [u32; 4]) {
    let [x0, y0, x1, y1] = region;
    println!("region {scene} ({x0}, {y0})..({x1}, {y1}):");
    let Some(image) = rendered
        .images
        .iter()
        .find(|image| image.name == "authored_id")
    else {
        println!("  this scene writes no authored_id, so there is nothing in it to name");
        return;
    };
    let right = x1.min(image.width.saturating_sub(1));
    let bottom = y1.min(image.height.saturating_sub(1));
    let mut counts: std::collections::BTreeMap<u32, u64> = std::collections::BTreeMap::new();
    let mut covered = 0u64;
    for y in y0..=bottom {
        for x in x0..=right {
            let Some(texel) = image.texel(x, y) else {
                continue;
            };
            let id = texel[0].round().max(0.0) as u32;
            *counts.entry(id).or_insert(0) += 1;
            covered += 1;
        }
    }
    let mut rows: Vec<(u32, u64)> = counts.into_iter().collect();
    // **One order, and it is total.** The count decides and the id breaks a tie, because a frame
    // holds ids that cover the same number of pixels -- `terrace`'s ids 33 and 89 each carry 1,918
    // -- and an order that leaves those two free to swap is an order two runs can disagree about.
    rows.sort_by(|one, other| other.1.cmp(&one.1).then(one.0.cmp(&other.0)));
    let naming = gallery::locate::naming(scene);
    for (id, count) in rows.iter() {
        println!(
            "  {count:>7} px  id {id:<6} {}",
            name_for_row(scene, naming, *id)
        );
    }
    // **The counted zero is a rectangle, while the sentence on it is a pixel's.** A region counts the
    // reserved zero like any other id -- that is what the scan above did before anything here named it,
    // and it is what a rectangle is for -- so the number on that row is this rectangle's pixels rather
    // than one pixel's reading, and the words beside them are asked of `locate` rather than written here.
    if rows
        .iter()
        .any(|(id, _)| *id == gallery::identity::NOT_AUTHORED)
    {
        println!(
            "  the reserved zero is counted here like any other id, so its row is this rectangle's \
             pixels and not one pixel's reading; the sentence on it is the one `--where 0` answers"
        );
    }
    // **The last line says the table is the whole of them, because a reader cannot see an absent
    // row.** The count is the number of rows above it, so the two cannot disagree, and the pixels
    // are the rectangle's own: every texel the scan read was counted into exactly one id, so the
    // column adds up to the number on the line and the arithmetic is a reader's to check.
    println!(
        "  {} distinct id(s) over {covered} px: every id the rectangle holds is above, deepest \
         first and ties by id ascending, and the counts add up to those {covered} px",
        rows.len()
    );
}

/// Returns what one counted id is called in a row of `--region`'s table.
///
/// # Invariants
///
/// - **A building's own id is asked of `gallery::building_identity`, and that table comes before the
///   naming registry.** The registry is the *vegetation* one -- a scene registers there to say its ids
///   are nodes of a plant -- and no building scene registers in it, so asking it first is what printed
///   `id 0, and this scene names nothing` for *every* id of *every* building: a wall, a slab and the
///   reserved zero alike, on the scenes whose elements [`locate`] and [`where_is`] both name.
/// - **The reserved zero is asked of [`reserved_zero_answers`] and never written here**, for the reason
///   [`where_is`] gives at length: it is no element to name, it is the one id every subsystem answers,
///   and `--where 0` asks that same call about the same number.
/// - **A row wants a handle and asks [`gallery::locate::handle_in`] for one.** A row of a rectangle is a count of
///   ids and carries no pixel, so the sentences `--locate` puts under the handle — a face, a place on it, an edit —
///   are about a pixel this table does not have. The registry answers both questions from one table, and which of the
///   two a caller asks is the caller's decision rather than the registry's.
/// - **A scene the gallery cannot name at all still answers in the harness's own words**, which are true
///   rather than a fallback for want of trying, and are the sentences this table carried before.
fn name_for_row(scene: &str, naming: Option<&'static gallery::locate::Naming>, id: u32) -> String {
    if id == gallery::identity::NOT_AUTHORED {
        if let Some(answer) = reserved_zero_answers(scene) {
            return answer;
        }
    }
    if let Some(handle) = gallery::locate::handle_in(scene, id) {
        return handle;
    }
    match naming {
        Some(_) if id == gallery::identity::NOT_AUTHORED => {
            "not an authored element: the ground or the sky".to_owned()
        }
        Some(_) => format!("id {id} names nothing here"),
        None => format!("id {id}, and this scene names nothing"),
    }
}

/// Returns the sentence the reserved zero is given by whichever domain owns the scene, or `None` when the
/// gallery names neither the scene nor the number.
///
/// # Invariants
///
/// **One ask, for every direction.** `--where 0` and the zero row of `--region` are the same question about
/// the same number, and two call sites each spelling out the `locate` call is two places to be wrong the day
/// the answer moves. A building scene gets its answer from the building's own identity table's refusal, a
/// landscape from the plain's, and a scene neither names from the sentence the gallery itself carries.
fn reserved_zero_answers(scene: &str) -> Option<String> {
    let the_values = &[(String::from(gallery::identity::ID_OUTPUT), 0.0)];
    gallery::locate::locate(scene, the_values)
}

/// Prints what one pixel is, in the terms the scene that drew it uses.
///
/// # Invariants
///
/// - **The generic half comes first and is always printed**: what every declared output holds at
///   that pixel, read from the image itself rather than from anything beside it. A caller with a
///   scene this harness has never heard of still gets an answer, and the numbers are the ones
///   the shader wrote.
///
/// - **The domain half is the scene's, and the harness only asks.** What an id *means* is the
///   content's own naming, which lives with the content and not here — this harness knows
///   nothing about a ridge or a belt. `gallery::locate` is asked, and a scene it does not know
///   says so rather than the harness guessing.
fn locate(scene: &str, rendered: &RenderedFrame, x: u32, y: u32) {
    println!("locate {scene} ({x}, {y}):");
    let mut values: Vec<(String, f32)> = Vec::new();
    for image in &rendered.images {
        if image.name == graphics::PRESENT_OUTPUT {
            continue;
        }
        match image.texel(x, y) {
            Some(texel) => {
                println!("  {:<20} {}", image.name, describe_texel(&texel));
                values.push((image.name.clone(), texel[0]));
            }
            None => println!(
                "  {:<20} outside the image, which is {} by {}",
                image.name, image.width, image.height
            ),
        }
    }
    match gallery::locate::locate(scene, &values) {
        Some(answer) => println!("  ->\n{answer}"),
        None => println!(
            "  -> this harness has no domain reading for `{scene}`. The numbers above are the \
             ones the shader wrote, and they are the answer for a scene whose content does not \
             name its own."
        ),
    }
}

/// Returns one texel as a line of numbers, dropping the trailing components a narrower
/// declaration never wrote.
fn describe_texel(texel: &[f32; 4]) -> String {
    match texel.iter().rposition(|value| *value != 0.0) {
        Some(last) => format!("{:?}", &texel[..=last]),
        None => "0".to_owned(),
    }
}

/// Answers **where an element is** in the frame just rendered: how many pixels carry its id and the box they
/// occupy.
///
/// # Invariants
///
/// - **A number is an id and anything else is a name**, resolved by `gallery::building_identity::id_in`, so the
///   two directions are one table: the id `locate` reads off a pixel is the id `where_is` finds pixels for.
/// - **"No such element" and "not visible" are different answers.** A name the table does not hold says so; an
///   id that holds but covers no pixel says the element is not in this frame. A caller that conflated them would
///   read a typo as an element that happened to be off screen.
/// - **The reserved zero is a third answer, and it is asked rather than written.** `Identity::of` numbers from
///   one, so id 0 is not an element that happened to be off screen: it is what a surface that is not an authored
///   element writes, and the scan below skips it on purpose. Reported as "not visible" it said the opposite of
///   the truth about the largest region of a building frame — the ground and the sky between them cover
///   686,410 of `workshop`'s 1,048,576 pixels — so the sentence is `gallery::locate`'s own, asked for the same
///   number `--locate` is asked, and the two directions cannot drift apart.
/// - A scene that writes no `authored_id` says so rather than reporting zero pixels, for the same reason the
///   harness's `--locate` refuses a scene whose content does not name its own.
fn where_is(scene: &str, rendered: &RenderedFrame, asked: &str) {
    let Some(image) = rendered
        .images
        .iter()
        .find(|image| image.name == "authored_id")
    else {
        println!(
            "where {scene} {asked}: this scene writes no authored_id, so there is nowhere to look"
        );
        return;
    };
    let id = match asked.parse::<u32>() {
        Ok(id) => Some(id),
        Err(_) => gallery::building_identity::id_in(scene, asked),
    };
    let Some(id) = id else {
        println!("where {scene} {asked}: no element of this scene is called that");
        return;
    };
    // **The reserved zero is answered before the scan, and it is not counted.** It is what a surface that is
    // not an authored element writes — the ground from the one constant the shaders declare, and the sky from
    // the same value the target was cleared to — so it is the ground and the sky between them, both of them
    // plainly in the frame. The scan below skips it on purpose, which is right for a tool that counts
    // *elements*, and wrong for the sentence: an id that covers the largest region of the picture and reads
    // "not visible in this frame" is the sentence's whole content for anyone who asks. **The sentence is
    // asked of `locate` rather than written here**, because `--locate` on a ground pixel and `--where 0` are
    // the same question about the same number, and two places to write one answer are two places to be wrong.
    // Nothing is lost by not counting it either: `--region` reads the same image and counts the reserved zero
    // like any other id, which is what a rectangle is for.
    if id == gallery::identity::NOT_AUTHORED {
        match reserved_zero_answers(scene) {
            Some(answer) => println!("where {scene} {asked}: {answer}"),
            None => println!(
                "where {scene} {asked}: id 0 is the reserved `not an authored element`, and this scene \
                 gives it no reading of its own"
            ),
        }
        return;
    }
    let mut covered = 0u64;
    let mut box_: Option<(u32, u32, u32, u32)> = None;
    for y in 0..image.height {
        for x in 0..image.width {
            let Some(texel) = image.texel(x, y) else {
                continue;
            };
            if texel[0].round() < 0.5 || texel[0].round() as u32 != id {
                continue;
            }
            covered += 1;
            box_ = Some(match box_ {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            });
        }
    }
    match box_ {
        Some((x0, y0, x1, y1)) => println!(
            "where {scene} {asked}: id {id} covers {covered} pixel(s) in x {x0}..={x1}, y {y0}..={y1}"
        ),
        None => println!("where {scene} {asked}: id {id} is not visible in this frame"),
    }
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

    let renderer = rasterizer().await;

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

    // `--census` answers how big a scene is before any of it is drawn: every list's count, every
    // instance bucket, the box the placements occupy and the post stage's own constants. It is
    // the number "the grove looks thin" wants beside it, and it costs no gpu work at all — the
    // document is the cheap half of anything, and a scene that will not compile still has one.
    //
    // **A document's census is the producer's to print, and this path refuses it rather than
    // reaching for the gallery.** The census reads the document, and on this path the document
    // is the one that was read from the file; the gallery's own census reads the scene the
    // gallery builds, which is a different document whenever the two have drifted apart.
    if request.census {
        if request.document.is_some() {
            return Err(
                "the census of a document is the producer's to print: `cargo run -p \
                 whiting-gallery --example scene_document -- <name> <path> --census`"
                    .to_owned(),
            );
        }
        let name = request.name.clone().ok_or_else(usage)?;
        print!("{}", Gallery::census(&name));
        print!("{}", Gallery::bounds(&name));
        return Ok(());
    }

    // **The scene is a document, and this is where the document comes from.** A name builds one
    // through the gallery; a file gives one that was written before this run, and that path
    // puts it in the registry the way the rpc's `PUT /definition/{name}` does, so the scene
    // that is compiled is the one the registry holds rather than the one that was read.
    let (name, authored) = match (&request.document, &request.name) {
        (Some(path), Some(name)) => load_document(path, &mut registry, Some(name))?,
        (Some(path), None) => load_document(path, &mut registry, None)?,
        (None, Some(name)) => (name.clone(), Gallery::build(name)),
        (None, None) => return Err(usage()),
    };
    let name = &name;

    // Every camera a scene names is rendered. A camera decides the projection every pass draws
    // through — and, for a fitted camera, how far back the eye stands — so it is an input to
    // the compile rather than something a renderer picks afterwards. A scene naming two
    // cameras is compiled and drawn twice; a scene naming none is drawn once, by the stated
    // convention, exactly as it always was.
    let cameras = camera_count(&authored);
    // The notes already reported, so that one fact about the documents is said once rather than
    // once per image of a range. See `print_notes` for why that is not the same as one per
    // compile.
    // The identity of the executable this run is executing, read once. The scenes are compiled
    // **into** this example, so the binary is the only thing standing between an edit and a
    // frame, and comparing it against the one the last run used is the difference between
    // "the build did not take" and a ten minute argument about whether it did.
    let build = build_id();
    let mut reported = HashSet::new();
    for camera in 0..cameras {
        // Where this camera's images go, and the manifest that describes them. Both are named
        // before the first moment is compiled rather than after the last, because the
        // comparison below has to read what the last run left at that path.
        //
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
        let manifest = manifest_path(&target.path);
        // Read before the first frame is drawn, and said out loud either way: a first render
        // of a scene has nothing to compare against, and that is a fact about the run rather
        // than something to pass over in silence.
        let recorded = read_manifest(&manifest);
        let mut moments = Vec::with_capacity(request.times.len());
        let mut outputs = Vec::with_capacity(request.times.len());
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

            // What each declared image **is**, read from the document rather than guessed from
            // the bytes. A built-in channel says which buffer it is bounded by and a
            // scene-declared output says which buckets the writing pass enumerated; the two are
            // different claims and a header that made only one of them would be wrong about the
            // other. An image the document does not declare at all — the convention's `present`
            // in a document with no `outputs` field — has no claim to make, which is said.
            let described = describe_outputs(&compiled.graphics.value);

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

            // What this frame is, against what the last run recorded at the same path. The
            // report is made here, before the timing line and long before the image is written
            // below, because a warning nobody sees before they open the picture is a warning
            // that costs the ten minutes it exists to save.
            let record = RenderRecord {
                scene: name.clone(),
                camera: camera as u64,
                // A scene that carries a time of its own is recorded as carrying no time,
                // rather than as a time no json can spell.
                time: (!time.is_nan()).then_some(time),
                document: document_hash(&compiled.graphics, &compiled.instance),
                image: image_hash(&rendered),
                passes: passes as u64,
                build: build.clone(),
            };
            report_against(recorded.as_ref(), &record, &manifest);
            outputs.push(record);

            // Where a pixel is, in the scene's own terms. Made **before** anything is written to
            // disk, because the answer is worth most next to the picture and least without it. Every
            // `--locate` is answered, in the order asked, so a comparison is one run.
            for (x, y) in &request.locate {
                locate(&name, &rendered, *x, *y);
            }

            // **A rectangle is a selection**, and this is the answer that turns a picture into the set of handles an
            // edit can be aimed at: the ids the rectangle covers, each with the pixels it owns, named by the content's
            // own table. A box in world space cannot answer this, which is why it is asked of the frame.
            for rectangle in &request.regions {
                region(&name, &rendered, *rectangle);
            }

            // The other direction: the element asked for, and where the frame put it.
            for asked in &request.where_is {
                where_is(&name, &rendered, asked);
            }

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
                rendered,
                described,
            });
            previous = Some(Compiled {
                graphics: compiled.graphics,
                instance: compiled.instance,
            });
        }

        // A scene with one camera writes where it always wrote, so every existing invocation
        // of the harness names the files it named before, and a scene with several says which
        // view each set is.
        write_moments(&target, &moments)?;
        // Written last, and only once every frame is on disk: a manifest that describes an
        // image which was never written is evidence of nothing.
        write_manifest(
            &manifest,
            &RenderManifest {
                format: MANIFEST_FORMAT,
                outputs,
            },
        )?;
    }
    Ok(())
}

/// Reads a scene document and puts it in the registry, the way the rpc's `PUT /definition/{name}`
/// puts one there.
///
/// # Invariants
///
/// - **The document is validated before it is stored**, so a document that does not satisfy the
///   scene schema is refused at the boundary rather than at render time. The rpc refuses it the
///   same way, and a harness that compiled a document the device would have refused would be
///   drawing something no device would draw.
/// - **The scene that is compiled is the one the registry holds afterwards**, not the one that was
///   read off the disk. The round trip through the registry is the whole of what this path is:
///   a document that is read and compiled directly is not the document a device would hold, and
///   the two are the same only while nothing rewrites a value on the way in.
/// - **The document is read on a thread with a larger stack than a windows main thread has**,
///   because `facet_json::from_str` descends as it parses. See [`DOCUMENT_STACK_BYTES`].
/// - **The name is the caller's, or the document's own, and there is no third answer.** The
///   registry is keyed by name and the manifest records the name, so a document that carries
///   neither is a document nothing can ask for again.
fn load_document(
    path: &Path,
    registry: &mut Registry,
    name: Option<&str>,
) -> Result<(String, Value), String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("reading {} failed: {error}", path.display()))?;
    let value = read_value(text)?;
    let name = match name {
        Some(name) => name.to_owned(),
        None => document_name(&value).ok_or_else(|| {
            format!(
                "the document at {} names no scene, so it cannot be rendered on its own",
                path.display()
            )
        })?,
    };
    let identifier = DefinitionIdentifier::new(&name);
    let definition = Definition::new(
        identifier.clone(),
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        value,
    );
    definition
        .validate(registry)
        .map_err(|error| format!("the document does not satisfy the scene schema: {error}"))?;
    if registry.definition(&identifier).is_ok() {
        registry.remove_definition(&identifier).map_err(|error| {
            format!("the document already in the registry cannot be removed: {error}")
        })?;
    }
    registry
        .insert_definition(definition)
        .map_err(|error| format!("the document does not go into the registry: {error}"))?;
    let stored = registry
        .definition(&identifier)
        .map_err(|error| format!("the document is not in the registry: {error}"))?;
    Ok((name, stored.value.clone()))
}

/// Returns the name a scene document carries, or none when it carries none.
fn document_name(value: &Value) -> Option<String> {
    value
        .as_field_map()
        .and_then(|fields| fields.get(scene::standard::NAME))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// Reads the json of a scene document into the value it holds.
///
/// # Invariants
///
/// **The read happens on a thread of its own, because the read is what needs the room.** The
/// text is moved into that thread rather than copied, so the document is held once and not
/// twice: a document of a few hundred megabytes copied once is a few hundred megabytes of
/// memory the machine did not need to spend.
fn read_value(text: String) -> Result<Value, String> {
    std::thread::Builder::new()
        .stack_size(DOCUMENT_STACK_BYTES)
        .spawn(move || facet_json::from_str::<Value>(&text))
        .map_err(|error| format!("the reading thread does not start: {error}"))?
        .join()
        .map_err(|_| "the reading thread panicked".to_owned())?
        .map_err(|error| format!("the document does not read as a scene value: {error}"))
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
///
/// # Invariants
///
/// **The flags are taken out before the rest is read, and the rest is read in the order it was
/// written.** The name, the path and the times used to be told apart by their positions — the
/// first two arguments were the name and the path and everything after them was a time — and a
/// flag that carries a path of its own breaks that: `--document` takes a path, and the path it
/// takes is not the path the images are written under. So the flags go first, and what is left
/// is the name, the path and the times, in that order.
fn parse_arguments() -> Result<Request, String> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();

    // `--video`, `--sheet`, `--sheet-full` and `--debug` are flags, and every bare number after
    // the path is a time, so the times are whatever is left once the flags and their values are
    // taken out. `--sheet-full` is `--sheet` with a step of one rather than a mode of its own,
    // because it is the same picture at another resolution and there is nothing else to choose.
    let mut mode = Mode::Frames;
    let mut debug: Vec<DebugChannel> = Vec::new();
    let mut census = false;
    let mut document: Option<PathBuf> = None;
    // **--locate repeats**: one pixel per flag, in the order asked, because the interesting question is
    // usually a comparison — five shutters, five pixels, five names — and one flag per run made that five runs.
    let mut locate_at: Vec<(u32, u32)> = Vec::new();
    // **--region repeats for the same reason and answers the other question**: a rectangle, in the order asked,
    // scanned for the ids under it. It is what turns a picture into the set of handles an edit can be aimed at.
    let mut regions: Vec<[u32; 4]> = Vec::new();
    let mut where_is: Vec<String> = Vec::new();
    let mut rest: Vec<String> = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--sheet" => mode = Mode::Sheet { step: SHEET_STEP },
            "--sheet-full" => mode = Mode::Sheet { step: FULL_STEP },
            "--video" => mode = Mode::Video,
            "--census" => census = true,
            "--document" => {
                index += 1;
                let asked = arguments
                    .get(index)
                    .ok_or_else(|| "--document needs the path of a scene document".to_owned())?;
                document = Some(PathBuf::from(asked));
            }
            "--locate" => {
                index += 1;
                let asked = rest_arg(&arguments, index, "--locate needs a pixel, written x,y")?;
                locate_at.push(parse_pixel(asked)?);
            }
            "--region" => {
                index += 1;
                let asked = rest_arg(
                    &arguments,
                    index,
                    "--region needs a rectangle, written x0,y0,x1,y1",
                )?;
                regions.push(parse_region(asked)?);
            }
            "--where" => {
                index += 1;
                let asked = rest_arg(&arguments, index, "--where needs an id or a name")?;
                where_is.push(asked.to_owned());
            }
            "--debug" => {
                index += 1;
                let asked = rest_arg(
                    &arguments,
                    index,
                    &format!("--debug needs a channel: {}", channel_names()),
                )?;
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
            other => rest.push(other.to_owned()),
        }
        index += 1;
    }

    // **The name is optional only where a document supplies one.** A document carries the name
    // it was built under, so `render --document hall.json out/hall` is a whole invocation, and
    // a name given beside a document is the name the document is registered and recorded
    // under. Without a document the name is the scene's own and there is nowhere else to get
    // it.
    let (name, path, times) = match (document.is_some(), rest.as_slice()) {
        (false, [name]) => (Some(name.clone()), "out".to_owned(), Vec::new()),
        (false, [name, path, times @ ..]) => (Some(name.clone()), path.clone(), times.to_vec()),
        (true, [path]) => (None, path.clone(), Vec::new()),
        (true, [_, path, times @ ..]) => (Some(rest[0].clone()), path.clone(), times.to_vec()),
        _ => return Err(usage()),
    };

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
        document,
        mode,
        times: moments,
        debug,
        census,
        locate: locate_at,
        regions,
        where_is,
    })
}

/// Returns the argument after a flag, or the message the flag is answered with when there is none.
fn rest_arg<'text>(
    arguments: &'text [String],
    index: usize,
    missing: &str,
) -> Result<&'text str, String> {
    arguments
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| missing.to_owned())
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
    /// The scene to render, and the name the manifest records it under.
    ///
    /// It is absent when a document supplies one: the document carries the name it was built
    /// under, and a name given beside a document is the name the document is registered under.
    name: Option<String>,
    /// The path the images are written under, without an extension.
    path: String,
    /// The scene document to render, where the scene is one that was written before this run.
    document: Option<PathBuf>,
    /// How the moments are put down.
    mode: Mode,
    /// The moments to draw.
    times: Vec<f64>,
    /// The debug channels every compile of this run is asked to draw.
    debug: Vec<DebugChannel>,
    /// Whether to report what the scene holds rather than drawing it.
    census: bool,
    /// The pixel to say what it is, answered once per moment beside the picture it belongs to.
    locate: Vec<(u32, u32)>,
    /// The rectangle to say **what is in it**, answered as the ids it covers and how many pixels each.
    regions: Vec<[u32; 4]>,
    /// The element to say **where** it is: an id or a name, answered as a pixel count and a box.
    where_is: Vec<String>,
}

/// How the moments of a render are written out.
#[derive(Clone)]
enum Mode {
    /// One image per moment.
    Frames,
    /// One image holding every moment in a grid, each tile reading every `step`-th pixel of the
    /// frame rather than all of them. See [`SHEET_STEP`] for what the step is for.
    Sheet { step: usize },
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
        Mode::Sheet { step } => {
            let present = moments.first().and_then(|moment| moment.rendered.present());
            let (width, height) = present
                .map(|image| (image.width, image.height))
                .unwrap_or((0, 0));
            let encoded = contact_sheet(moments, step)?;
            // The size of the **file**, and not the size of the grid the frames would make if
            // nothing were sampled: a report that names a 3072x2048 sheet beside a 768x512 png is
            // the resolution of a thumbnail claimed as the resolution of a picture.
            let (cell_width, cell_height) = tile(width, height, step);
            let path = numbered(&request.path, 0, 1);
            write_png(&path, &encoded)?;
            println!(
                "wrote {} ({} frames, {}x{}, sampled 1/{step})",
                path.display(),
                moments.len(),
                cell_width * columns(moments.len()),
                cell_height * rows(moments.len())
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
/// the shape of what the channel holds is what a viewer reads, and a text file of the values at
/// a grid of pixels, because values normalized into eight bits are a picture of them rather
/// than the values. The text carries the channel's own description and the mapping the png
/// used, so the two cannot be read apart and neither can be read as the other.
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
            write_png(&path, &viewable(image, marks_alpha(&image.name))?)?;
            println!("wrote {} ({})", path.display(), describe(moment.time));

            if let Some(probe) = probe(
                image,
                described(moment, &image.name),
                marks_alpha(&image.name),
            ) {
                let sidecar = with_extension(&path, "txt");
                write_text(&sidecar, &probe)?;
                println!("wrote {} ({})", sidecar.display(), describe(moment.time));
            }
        }
    }
    Ok(())
}

/// Returns whether an image's fourth component **marks** the pixels the pass drew on.
///
/// # Invariants
///
/// **Only a built-in's fourth component marks anything**, and the registry is what says so —
/// `Position` and `Depth` fill their own fourth channel, and it is the fact that no surface was
/// drawn on a pixel. Nothing in a declaration can ask for a marker, so a scene-declared
/// output's fourth component is either the fourth number its shader wrote or **padding the
/// compiler defined**, and treating it as a marker would drop texels a rule invented rather
/// than a render left out: for a `vec3<f32>` whose padding is low, that is every texel in the
/// image, and the sidecar beside it would be a file of zeros.
///
/// **The declared count does not drive this, and asking it to would invert the truth.** The
/// declared count is now what decides where the padding starts, so it is a fact about the image
/// — `f32` is one number in a one wide image, `vec3<f32>` is three numbers in a four wide one —
/// and a reader keyed on it would call a `vec4<f32>` scene output a marker and an `f32` one not,
/// which is the opposite of what either of them is: a `vec4<f32>` has four declared values and
/// its fourth is one of them, and an `f32` image has no fourth component at all. Padding is by
/// definition not a marker, so the number that says "this component means nothing" cannot also be
/// the number that says "this component is a marker". Marking is a **reading convention**, and the
/// only thing in this workspace that owns one is the built-in registry.
///
/// The fact is read from the name because the registry is keyed by name and this file already
/// asks it the same question to choose a header: a name the registry does not hold is a scene's
/// own value by the decision above, and one it does hold takes the built-in's convention along
/// with the built-in's own description saying so.
fn marks_alpha(name: &str) -> bool {
    DebugChannel::from_output(name).is_some()
}

/// Returns what a moment's image is, from the description the document earned it.
///
/// A name the document did not describe — the convention's frame in a document with no
/// `outputs` field, which the harness never writes a sidecar for — falls back to saying so
/// rather than borrowing another image's claim.
fn described<'moment>(moment: &'moment Moment, name: &str) -> Option<&'moment str> {
    moment
        .described
        .iter()
        .find(|(declared, _)| declared == name)
        .map(|(_, description)| description.as_str())
}

/// Reads what each image a document declares **is**, as prose a sidecar can carry.
///
/// # Invariants
///
/// The two halves of `outputs` are answered from two different places, and this is where that
/// is decided rather than at the point of writing:
///
/// - A **built-in** names a buffer, so its own description is used verbatim. It is the
///   channel's claim about the frame resource it reads, and the emitter's note about a scene
///   with no `depthing` bucket is the same claim made at compile time.
/// - A **scene-declared** output names no buffer, so the compiler's own vocabulary is used:
///   the declared type, the location, and the contract that it covers exactly what the writing
///   pass enumerated.
/// - **An image the document does not declare** gets neither. It is described as unclaimed,
///   which is the one header that names its own ignorance rather than filling the gap.
fn describe_outputs(document: &Value) -> Vec<(String, String)> {
    let Ok(declared) = graphics::outputs(document) else {
        return Vec::new();
    };
    declared
        .iter()
        .map(|output| {
            let description = match DebugChannel::from_output(output.name) {
                Some(channel) => channel.description().to_owned(),
                None => scene::declared_description(output.name, output.ty, output.location),
            };
            (output.name.to_owned(), description)
        })
        .collect()
}

/// Returns the image as a png that can be looked at.
///
/// # Invariants
///
/// An eight bit image is written as it stands. A float image has no eight bit form, so it is
/// normalized over the texels the scene **drew**: a value at a pixel the scene never reached is
/// not part of the picture, and including it would compress everything that is into a corner of
/// the range. **"Drew" is a marker's word, and only a built-in image has one** — see
/// [`marks_alpha`]. For a scene-declared output every texel counts, because its fourth
/// component is not a marker and there is nothing in it that says a texel was skipped. The
/// mapping is returned by [`probe`] beside the file, because a picture whose scale is private to
/// the code that made it is a picture nobody can read a number from.
fn viewable(image: &RenderedImage, marks: bool) -> Result<Vec<u8>, String> {
    if let Some(frame) = image.as_frame() {
        return encode_png(frame.width, frame.height, &frame.rgba);
    }
    let (minimum, maximum) = covered_range(image, marks).ok_or_else(|| {
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
            let covered = !marks || texel[3] > 0.5;
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
///
/// **An image with no marker is measured over every texel it has**, which is the other half of
/// what [`marks_alpha`] says: there is no fourth component that could mean "nothing was drawn
/// here", so there is nothing to exclude on.
fn covered_range(image: &RenderedImage, marks: bool) -> Option<([f32; 3], [f32; 3])> {
    let mut minimum = [f32::INFINITY; 3];
    let mut maximum = [f32::NEG_INFINITY; 3];
    let mut covered = false;
    for y in 0..image.height {
        for x in 0..image.width {
            let texel = image.texel(x, y)?;
            if marks && texel[3] <= 0.5 {
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
/// this file is the half of the pair a caller reads the numbers out of.
///
/// **The header is the document's and not this file's.** A built-in channel knows what its image
/// is and a scene-declared output knows what it declared; a format says nothing. `position` and
/// `depth` are both `rgba16float` and hold genuinely different things, and a `vec4<f32>` normal
/// and a `vec4<f32>` colour are the same four floats in the same target, so a sentence written
/// here would be a claim about every float image this harness writes and the one that is wrong
/// is a file that says it holds world positions and holds depths instead. The one image nothing
/// described is described as exactly that.
///
/// The header also names the mapping the png beside it used, so a caller comparing the two
/// knows what the colours mean. **A `-` in the grid is a texel the pass drew nothing on, and
/// only an image with a marker has any**: for a scene-declared output the fourth component is
/// not one, so every sampled texel is printed and the legend says so rather than promising a
/// line that can never appear.
fn probe(image: &RenderedImage, claimed: Option<&str>, marks: bool) -> Option<String> {
    if !matches!(
        image.format.as_str(),
        "rgba16float" | "rgba32float" | "r32float"
    ) {
        return None;
    }
    let (minimum, maximum) = covered_range(image, marks)?;
    let described = claimed.map_or_else(|| unclaimed(image, marks), str::to_owned);
    let legend = if marks {
        "`-` is a pixel the scene drew nothing on"
    } else {
        "every sampled texel below is a value the pass wrote; this image has no marker, so there \
is no `-` line and no texel is dropped for want of one"
    };
    let mut text = format!(
        "{}: {}x{}, {}\n\
         {described}\n\
         red over [{:.3}, {:.3}], green over [{:.3}, {:.3}], blue over [{:.3}, {:.3}]\n\
         {legend}\n\n",
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
            if marks && texel[3] <= 0.5 {
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

/// Describes a float image that no channel drew and no declaration described.
///
/// # Invariants
///
/// It is the one header that names its own ignorance rather than filling the gap, because the
/// alternatives are the two failures this file was fixed for: borrowing a channel's sentences
/// and being wrong, or saying nothing and leaving the numbers to be read by their format. What
/// it states is true of every float image — the numbers are what the pass wrote — and what it
/// cannot state is what the alpha means, so it says that too. **It therefore measures over every
/// texel rather than over the ones a marker would have picked**, because an image nothing
/// described is exactly the image whose fourth component nobody can vouch for, and dropping
/// texels on a guess about it is the failure this header exists next to.
///
/// It is reached by an image the render returned under a name the document declared nothing
/// about, which is a document that named an output the compiler has no description for. It is
/// not the header a **scene-declared** output gets: one of those is described, by its own
/// declaration, in the enumeration's own words.
fn unclaimed(image: &RenderedImage, marks: bool) -> String {
    let ranges = if marks {
        "the three ranges below are the smallest and the largest value each colour holds across \
the pixels whose alpha is above one half, and they are the range the png beside this file was \
normalized by."
    } else {
        "the three ranges below are the smallest and the largest value each colour holds across \
every texel of this image, because nothing here says which of them a pass left alone."
    };
    format!(
        "no debug channel is named `{name}`, so this file does not say what the numbers mean.
They are the values the pass that wrote this image wrote, as it wrote them: nothing is
normalized and nothing has to be decoded to read one. They are to be read against the pass
that draws them.
alpha carries whatever that pass put there, and this file does not claim to know what it
stands for.
{ranges}",
        name = image.name
    )
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

/// What one run produced for one scene, camera and moment.
///
/// # Invariants
///
/// Every field is a fact about **what the run used**, never about what it was asked for. The
/// document hash covers the two emitted definitions as the compile produced them, the pixel
/// hash covers the frame as the gpu returned it, and the build covers the executable that asked
/// for both. A record reconstructed from a source file's timestamp or from the scene's name
/// answers a question nobody asked, which is how three separate runs of this harness ended up
/// bisecting by hand.
#[derive(Debug, Clone, PartialEq, Facet)]
struct RenderRecord {
    /// The scene the image is of.
    scene: String,
    /// The camera the image was drawn through.
    camera: u64,
    /// The time drawn, and null when the scene carried a time of its own.
    time: Option<f64>,
    /// Hash of the emitted graphics and instance documents.
    document: String,
    /// Hash of the frame's pixels, and null when the frame is not eight bit a channel.
    image: Option<String>,
    /// The passes the graphics document declared.
    passes: u64,
    /// Hash of the executable that drew it, which is where the scenes are compiled into.
    build: String,
}

/// The record of one render, written beside the images it describes.
#[derive(Debug, Clone, PartialEq, Facet)]
struct RenderManifest {
    /// The shape of this file.
    format: u32,
    /// Every output this run produced, in the order it produced them.
    outputs: Vec<RenderRecord>,
}

impl RenderManifest {
    /// Returns the record this manifest holds of one output, if it holds one.
    ///
    /// A frame is identified by what it is of and not by where it was written, so a render
    /// aimed at a different path still compares against the run that drew the same moment.
    fn record(&self, wanted: &RenderRecord) -> Option<&RenderRecord> {
        self.outputs.iter().find(|record| {
            record.scene == wanted.scene
                && record.camera == wanted.camera
                && record.time == wanted.time
        })
    }
}

/// Returns where the manifest of a render is written, beside the images it describes.
///
/// The stem carries the suffix and the extension is added after it, the same way
/// [`numbered`] derives an image's name — a suffix appended after the extension is read as
/// part of the extension by every writer that replaces it.
fn manifest_path(output: &str) -> PathBuf {
    with_extension(&PathBuf::from(with_stem_suffix(output, MANIFEST)), "json")
}

/// Reads the record the last run of this output left at `path`.
///
/// # Invariants
///
/// A file that is not there is reported rather than passed over, and it is not a warning: "no
/// previous render" is the ordinary state of a scene being rendered for the first time, and
/// saying so is what tells a caller the comparison below did not run rather than that it found
/// nothing.
///
/// A file that is there and does not read — because it is a different shape, or because it was
/// edited — is reported too, and then this run replaces it. Overwriting a record in silence is
/// how a record stops being evidence, and the next run would then compare against a file that
/// claims to be history and is not.
fn read_manifest(path: &Path) -> Option<RenderManifest> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            println!(
                "manifest: {} holds no previous render; this run writes the first one",
                path.display()
            );
            return None;
        }
        Err(error) => {
            println!(
                "manifest: {} cannot be read ({error}); this run writes a new one",
                path.display()
            );
            return None;
        }
    };
    match facet_json::from_str::<RenderManifest>(&text) {
        Ok(manifest) if manifest.format == MANIFEST_FORMAT => Some(manifest),
        Ok(manifest) => {
            println!(
                "manifest: {} is written in format {} and this harness reads {MANIFEST_FORMAT}; \
                 this run replaces it",
                path.display(),
                manifest.format
            );
            None
        }
        Err(error) => {
            println!(
                "manifest: {} does not read as a manifest ({error}); this run replaces it",
                path.display()
            );
            None
        }
    }
}

/// Writes the record of this run beside the images it describes.
fn write_manifest(path: &Path, manifest: &RenderManifest) -> Result<(), String> {
    let text = facet_json::to_string_pretty(manifest)
        .map_err(|error| format!("the manifest does not encode: {error}"))?;
    write_text(path, &text)
}

/// Says what this frame is against the frame the last run recorded for the same moment.
///
/// # Invariants
///
/// The build, the documents and the pixels are read as **three separate facts**, because the
/// four answers they give are four different situations, and a caller who cannot tell them
/// apart is back to checking file timestamps by hand:
///
/// | build | documents | pixels | what it means |
/// | --- | --- | --- | --- |
/// | same | same | same | nothing ran, or nothing changed. A stale build. |
/// | new | same | same | the edit did not reach the documents this camera reads |
/// | any | same | new | the render is not a function of the documents alone |
/// | any | new | same | the edit is real and this camera's draw does not read it |
/// | any | new | new | it worked, and this says nothing at all |
///
/// The last row is silence, because the timing line is already the report of a run that did
/// what it was asked.
///
/// The first two are warnings, and they go to the standard error rather than beside the timing
/// line: the run succeeds in both, the image is written in both, and a note on the standard
/// output is too easy to read past. The fourth is a note, not a warning — the change *did*
/// reach the compile and simply does not reach this frame, which is a real edit and a frame
/// that is unchanged on purpose, and being told so is the whole point of hashing the pixels.
fn report_against(recorded: Option<&RenderManifest>, record: &RenderRecord, path: &Path) {
    let Some(previous) = recorded else {
        return;
    };
    let Some(before) = previous.record(record) else {
        println!(
            "manifest: {} holds no record of {}; there is nothing to compare it against",
            path.display(),
            describe_record(record)
        );
        return;
    };
    let (Some(image), Some(was_image)) = (&record.image, &before.image) else {
        println!(
            "note: {} has no eight bit frame recorded, so the pixels are not compared",
            describe_record(record)
        );
        return;
    };
    let documents_moved = before.document != record.document;
    let pixels_moved = was_image != image;
    let build_moved = before.build != record.build;
    if documents_moved && pixels_moved {
        return;
    }

    // Each case says which stream it goes to, rather than the caller working it out from the
    // booleans: a warning is a claim about the run, and a note is a claim about the scene.
    let (warning, lines) = if !documents_moved && !pixels_moved {
        let mut lines = format!(
            "warning: {} is the last render again — nothing changed\n  \
             documents  identical\n  \
             pixels     identical\n  \
             binary     {}{}\n  \
             recorded in {}\n",
            describe_record(record),
            short(&record.build),
            if build_moved {
                format!(", was {}", short(&before.build))
            } else {
                ", the same one that drew the last render".to_owned()
            },
            path.display(),
        );
        lines.push_str(if build_moved {
            "  the build took and the change still did not reach the documents this camera reads"
        } else {
            "  if you just edited a scene, the edit did not reach the picture: check that the \
             build took"
        });
        (true, lines)
    } else if !documents_moved {
        (
            true,
            format!(
                "warning: {} has the last render's documents and a different picture\n  \
                 documents  identical\n  \
                 pixels     {}, was {}\n  \
                 binary     {}\n  \
                 recorded in {}\n  \
                 the same documents drew a different frame, so the renderer is not a function of \
                 them alone: another machine, another adapter, or a document the hash does not \
                 cover",
                describe_record(record),
                short(image),
                short(was_image),
                short(&record.build),
                path.display(),
            ),
        )
    } else {
        (
            false,
            format!(
                "note: {} has new documents and the same picture\n  \
                 documents  {}, was {}\n  \
                 pixels     identical\n  \
                 binary     {}, was {}\n  \
                 recorded in {}\n  \
                 the edit reached the compile and not this frame: this camera does not read what \
                 changed, so the picture is unchanged on purpose and the change is real",
                describe_record(record),
                short(&record.document),
                short(&before.document),
                short(&record.build),
                short(&before.build),
                path.display(),
            ),
        )
    };
    if warning {
        eprintln!("{lines}");
    } else {
        println!("{lines}");
    }
}

/// Returns how a record names the image it is about.
fn describe_record(record: &RenderRecord) -> String {
    format!(
        "{} at camera {}, {}",
        record.scene,
        record.camera,
        describe(record.time.unwrap_or(f64::NAN))
    )
}

/// Returns the identity of the executable this run is executing.
///
/// # Invariants
///
/// The scenes are compiled **into** this example, so there is no scene file to stamp: the
/// binary is the provenance, and hashing the bytes on disk is the only statement about it that
/// is a fact rather than an inference. A timestamp is not used — a build that produced the
/// same bytes is not a different build, and a clock that moved is not evidence of anything.
fn build_id() -> String {
    let unknown = || "unknown".to_owned();
    let Ok(path) = std::env::current_exe() else {
        return unknown();
    };
    let Ok(file) = std::fs::File::open(&path) else {
        return unknown();
    };
    // Read in chunks rather than in one: a debug build of this example is tens of megabytes,
    // and the id is wanted from every run including the first one, which has nothing to
    // compare it against.
    let mut reader = std::io::BufReader::with_capacity(1 << 16, file);
    let mut hash = Sha256::new();
    let mut chunk = [0u8; 1 << 16];
    while let Ok(read) = reader.read(&mut chunk) {
        if read == 0 {
            break;
        }
        hash.update(&chunk[..read]);
    }
    hash.finish()
}

/// Returns the hash of the two documents one compile emitted.
///
/// # Invariants
///
/// Both documents are covered, and each is covered **as content**: the identifier and schema it
/// was emitted under, and the value itself, walked rather than formatted. The version counter is
/// left out on purpose — it is bookkeeping about how a document arrived, and this asks what the
/// document says.
///
/// A float is hashed by its bits, not by its shortest decimal form, so two documents that hold
/// the same number are the same document and two that hold numbers that print alike are not.
fn document_hash(graphics: &Definition, instance: &Definition) -> String {
    let mut hash = Sha256::new();
    for definition in [graphics, instance] {
        feed(
            &mut hash,
            definition.definition_identifier.as_str().as_bytes(),
        );
        feed(&mut hash, definition.schema_identifier.as_str().as_bytes());
        feed_value(&mut hash, &definition.value);
    }
    hash.finish()
}

/// Returns the hash of a frame's pixels, or none when the frame is not eight bit a channel.
///
/// # Invariants
///
/// The pixels are hashed as they came back from the gpu, before any png is encoded, and the
/// width and the height go in beside them so that two frames of different sizes cannot hash as
/// one. This is the deepest of the three hashes and the one no previous run of this harness
/// had: a document can change in a field this camera never reads, and "the document changed
/// and this frame did not" is a statement the framework can now make rather than one every
/// caller has to re-derive.
fn image_hash(rendered: &RenderedFrame) -> Option<String> {
    let present = rendered.present()?;
    let frame = present.as_frame()?;
    let mut hash = Sha256::new();
    feed(&mut hash, present.format.as_bytes());
    hash.update(&frame.width.to_le_bytes());
    hash.update(&frame.height.to_le_bytes());
    hash.update(&frame.rgba);
    Some(hash.finish())
}

/// Feeds bytes to a hash with their length in front of them.
///
/// Without the length, `("ab", "c")` and `("a", "bc")` are the same stream of bytes to the
/// hash, and two documents that differ would hash as one document.
fn feed(hash: &mut Sha256, bytes: &[u8]) {
    hash.update(&(bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
}

/// Feeds one value to a hash, in a form that is decided here rather than by a formatter.
///
/// # Invariants
///
/// Each kind of value has its own tag, so a string holding `"3"` and an integer holding `3` are
/// not the same document, and each is fed its length. A field map is walked in its own order,
/// which is sorted (`definition::FieldMap` is a `BTreeMap` for exactly this reason), so the same
/// document hashes the same on every run and in every process.
///
/// A packed list is hashed as its layout and its bytes rather than as the tree it stands for:
/// it is a representation of the same data, and this is the representation the compile emitted.
fn feed_value(hash: &mut Sha256, value: &Value) {
    match value {
        Value::Unit => hash.update(&[0]),
        Value::Bool(value) => hash.update(&[1, u8::from(*value)]),
        Value::Int(value) => {
            hash.update(&[2]);
            hash.update(&value.to_le_bytes());
        }
        Value::Float(value) => {
            hash.update(&[3]);
            hash.update(&value.to_bits().to_le_bytes());
        }
        Value::String(value) => {
            hash.update(&[4]);
            feed(hash, value.as_bytes());
        }
        Value::Bytes(value) => {
            hash.update(&[5]);
            feed(hash, value);
        }
        Value::Struct(fields) => feed_fields(hash, 6, fields),
        Value::Enum { tag, fields } => {
            hash.update(&[7]);
            feed(hash, tag.as_bytes());
            feed_fields(hash, 0, fields);
        }
        Value::List(values) => {
            hash.update(&[8]);
            hash.update(&(values.len() as u64).to_le_bytes());
            for value in values {
                feed_value(hash, value);
            }
        }
        Value::Packed(run) => {
            hash.update(&[9]);
            hash.update(&(run.len() as u64).to_le_bytes());
            hash.update(&(run.layout().stride_bytes() as u64).to_le_bytes());
            for (field, _) in run.layout().fields() {
                feed(hash, field.as_bytes());
            }
            hash.update(run.bytes());
        }
        Value::Dict(fields) => feed_fields(hash, 10, fields),
    }
}

/// Feeds a field map to a hash, under `tag`, in the map's own order.
fn feed_fields(hash: &mut Sha256, tag: u8, fields: &FieldMap) {
    hash.update(&[tag]);
    hash.update(&(fields.len() as u64).to_le_bytes());
    for (name, value) in fields {
        feed(hash, name.as_bytes());
        feed_value(hash, value);
    }
}

/// Returns the first few characters of a hash.
///
/// Enough to compare two lines of output by eye, and no more: a line of output is read, not
/// diffed, and the whole hash is in the file beside it for anything that needs it.
fn short(hash: &str) -> &str {
    &hash[..hash.len().min(12)]
}

/// SHA-256, over bytes, as the hex a person can put next to another one.
///
/// # Why this is here
///
/// A hash is four operations and a constant table, and this harness needs exactly one of them,
/// exactly once per run. A dependency would be a second thing to keep current for a use this
/// file has one of, and the whole of it is below where a reader can check it against the
/// published vectors — which the tests at the bottom of this file do.
///
/// The implementation follows FIPS 180-4 and is the standard one: eight words of state, a
/// sixty four byte block, and a length in bits appended to the message.
struct Sha256 {
    /// The chaining value, one word per round of the compression.
    state: [u32; 8],
    /// The block being filled.
    block: [u8; 64],
    /// How much of `block` holds bytes of the message.
    filled: usize,
    /// How many bytes of the message have been absorbed, padding not yet included.
    length: u64,
}

impl Sha256 {
    /// Starts a hash.
    fn new() -> Self {
        Self {
            state: [
                0x6a09_e667,
                0xbb67_ae85,
                0x3c6e_f372,
                0xa54f_f53a,
                0x510e_527f,
                0x9b05_688c,
                0x1f83_d9ab,
                0x5be0_cd19,
            ],
            block: [0; 64],
            filled: 0,
            length: 0,
        }
    }

    /// Absorbs bytes, which may be of any length.
    fn update(&mut self, bytes: &[u8]) {
        self.length = self.length.wrapping_add(bytes.len() as u64);
        self.absorb(bytes);
    }

    /// Absorbs bytes without counting them, which is what the padding needs.
    fn absorb(&mut self, mut bytes: &[u8]) {
        while !bytes.is_empty() {
            let take = (64 - self.filled).min(bytes.len());
            self.block[self.filled..self.filled + take].copy_from_slice(&bytes[..take]);
            self.filled += take;
            bytes = &bytes[take..];
            if self.filled == 64 {
                let block = self.block;
                self.compress(&block);
                self.filled = 0;
            }
        }
    }

    /// Folds one block into the state.
    fn compress(&mut self, block: &[u8; 64]) {
        let mut words = [0u32; 64];
        for (word, chunk) in words.iter_mut().zip(block.chunks_exact(4)) {
            *word = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for index in 16..64 {
            let before = words[index - 15];
            let earlier = words[index - 2];
            let spread = before.rotate_right(7) ^ before.rotate_right(18) ^ (before >> 3);
            let mix = earlier.rotate_right(17) ^ earlier.rotate_right(19) ^ (earlier >> 10);
            words[index] = words[index - 16]
                .wrapping_add(spread)
                .wrapping_add(words[index - 7])
                .wrapping_add(mix);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for index in 0..64 {
            let first = h
                .wrapping_add(e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25))
                .wrapping_add((e & f) ^ ((!e) & g))
                .wrapping_add(ROUND_CONSTANTS[index])
                .wrapping_add(words[index]);
            let second = (a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22))
                .wrapping_add((a & b) ^ (a & c) ^ (b & c));
            (h, g, f, e, d, c, b, a) = (
                g,
                f,
                e,
                d.wrapping_add(first),
                c,
                b,
                a,
                first.wrapping_add(second),
            );
        }
        for (held, added) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *held = held.wrapping_add(added);
        }
    }

    /// Finishes the hash and returns it as hex.
    fn finish(mut self) -> String {
        let bits = self.length.wrapping_mul(8);
        self.absorb(&[0x80]);
        // Pad to the length that leaves room for the length itself: a block that has just been
        // filled starts a fresh one, so this is never a loop that cannot end.
        while self.filled != 56 {
            self.absorb(&[0x00]);
        }
        let mut last = self.block;
        last[56..].copy_from_slice(&bits.to_be_bytes());
        self.compress(&last);
        self.state
            .iter()
            .map(|word| format!("{word:08x}"))
            .collect()
    }
}

/// The first thirty two bits of the fractional parts of the cube roots of the first sixty four
/// primes, which is what the compression's round constants are made of.
const ROUND_CONSTANTS: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// Returns how many images one row of a contact sheet holds.
fn columns(count: usize) -> usize {
    (count as f64).sqrt().ceil().max(1.0) as usize
}

/// Returns how many rows a contact sheet of `count` images has.
fn rows(count: usize) -> usize {
    count.div_ceil(columns(count))
}

/// How many pixels apart a contact sheet reads a frame, unless a caller asks for every pixel.
///
/// Four is the step every sheet written so far was written at, and it stays the default for the
/// reason `contact_sheet` gives: a range is read for its shape, and a sheet of thirteen
/// thousand-pixel frames is not read at all. `--sheet-full` is the way to ask for
/// [`FULL_STEP`], and the arithmetic here is the same either way — the step is a number
/// this harness was already dividing by, not a second code path.
const SHEET_STEP: usize = 4;

/// The step that reads every pixel, which makes a tile the frame itself.
const FULL_STEP: usize = 1;

/// Returns the size of one tile of a sheet holding a `width` by `height` frame.
///
/// # Invariants
///
/// A tile is the frame **divided by the step** and never smaller than a pixel, because a frame
/// too small to divide down is still a frame and is shown as itself rather than as nothing. The
/// division is on the frame's own pixels rather than a resize: a tile holds pixels the frame
/// already had, so the grid cannot hold a frame saying something the frame does not say.
///
/// `step` of zero is read as one rather than refused, because a caller of this function is this
/// harness's own flag parsing and a sheet is a picture — a picture is worth more than the check
/// that would have stopped it.
fn tile(width: u32, height: u32, step: usize) -> (usize, usize) {
    let step = step.max(1);
    (
        (width as usize / step).max(1),
        (height as usize / step).max(1),
    )
}

/// Lays every frame out in a grid, so a range of times is one image to look at.
///
/// A sheet is sampled every [`SHEET_STEP`]th pixel by default, because a dozen frames of a
/// thousand pixels each is a sheet nobody can see, and a range is read for its shape rather than
/// for its pixels. `step` is that number rather than a choice between two ways of building the
/// grid, so a sheet asked for at full resolution is the same composition at every pixel and not
/// a second way of laying frames out.
fn contact_sheet(moments: &[Moment], step: usize) -> Result<Vec<u8>, String> {
    let Some(first) = moments.first() else {
        return Err("a contact sheet of no frames is not an image".to_owned());
    };
    let first = first
        .rendered
        .present()
        .and_then(RenderedImage::as_frame)
        .ok_or_else(|| "the frame of this render is not an image a sheet is made of".to_owned())?;
    let (columns, rows) = (columns(moments.len()), rows(moments.len()));
    let (cell_width, cell_height) = tile(first.width, first.height, step);
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
                // A frame is read rather than scaled, so the source is `step` pixels along and
                // the last column of an uneven division reads the last pixel there is instead
                // of running off the end of the row.
                let source_x = (x * step).min(source.width as usize - 1);
                let source_y = (y * step).min(source.height as usize - 1);
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
    concat!(
        "usage: render <scene> <out> [start [stop [step]]] ",
        "[--sheet | --sheet-full | --video] [--debug <channel>] [--census]\n",
        "       render --document <path> <out> [start [stop [step]]] (the scene's name is the ",
        "document's own)"
    )
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A definition holding one field, which is the smallest thing two documents can differ by.
    fn definition(value: Value) -> Definition {
        Definition::new(
            DefinitionIdentifier::new("test.graphics"),
            SchemaIdentifier::new("test.schema"),
            value,
        )
    }

    /// A field map holding one named field.
    fn holding(name: &str, value: Value) -> Value {
        let mut fields = FieldMap::new();
        fields.insert(name.to_owned(), value);
        Value::Struct(fields)
    }

    /// Returns the hash of one value as both documents hold it.
    fn hash_of(value: &Value) -> String {
        document_hash(&definition(value.clone()), &definition(Value::Unit))
    }

    /// The hash has to be SHA-256, or a record means nothing across machines.
    ///
    /// The three vectors are the published ones for the empty message, for a message shorter
    /// than one block, and for one that is shorter than two and longer than one — the last is
    /// the one a padding bug passes.
    #[test]
    fn the_hash_is_sha_256() {
        let hash = |bytes: &[u8]| {
            let mut hasher = Sha256::new();
            hasher.update(bytes);
            hasher.finish()
        };
        assert_eq!(
            hash(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hash(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hash(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    /// A message longer than one block, fed in pieces, is the case a block loop gets wrong.
    #[test]
    fn the_hash_covers_a_long_message_fed_in_pieces() {
        let mut hasher = Sha256::new();
        for _ in 0..1_000_000 {
            hasher.update(b"a");
        }
        assert_eq!(
            hasher.finish(),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    /// The same document twice is the same hash, which is the whole claim the manifest rests
    /// on, and two documents that differ are not.
    #[test]
    fn the_document_hash_separates_documents_that_differ() {
        let dunes = holding("dunes", Value::Float(12.5));
        assert_eq!(hash_of(&dunes), hash_of(&dunes.clone()));
        assert_ne!(
            hash_of(&dunes),
            hash_of(&holding("dunes", Value::Float(12.5001)))
        );
        // A number and the string that spells it are different documents, and a field map
        // walked in its own order hashes the same however it was built.
        assert_ne!(
            hash_of(&holding("count", Value::Int(3))),
            hash_of(&holding("count", Value::String("3".to_owned())))
        );
        let mut one = FieldMap::new();
        one.insert("b".to_owned(), Value::Int(1));
        one.insert("a".to_owned(), Value::Int(2));
        let mut other = FieldMap::new();
        other.insert("a".to_owned(), Value::Int(2));
        other.insert("b".to_owned(), Value::Int(1));
        assert_eq!(hash_of(&Value::Struct(one)), hash_of(&Value::Struct(other)));
        // The two documents are covered apart, so swapping them is a change.
        let left = document_hash(
            &definition(holding("which", Value::String("graphics".to_owned()))),
            &definition(Value::Unit),
        );
        let right = document_hash(
            &definition(Value::Unit),
            &definition(holding("which", Value::String("graphics".to_owned()))),
        );
        assert_ne!(left, right);
    }

    /// A record has to survive the file it is written to, or the next run compares against
    /// nothing and says so instead of warning.
    #[test]
    fn a_manifest_survives_its_own_encoding() {
        let manifest = RenderManifest {
            format: MANIFEST_FORMAT,
            outputs: vec![
                RenderRecord {
                    scene: "dunes".to_owned(),
                    camera: 0,
                    time: None,
                    document: "a".repeat(64),
                    image: Some("b".repeat(64)),
                    passes: 6,
                    build: "c".repeat(64),
                },
                RenderRecord {
                    scene: "tidewater".to_owned(),
                    camera: 2,
                    time: Some(2.5),
                    document: "d".repeat(64),
                    image: None,
                    passes: 11,
                    build: "e".repeat(64),
                },
            ],
        };
        let text = facet_json::to_string_pretty(&manifest).expect("the manifest encodes");
        let read = facet_json::from_str::<RenderManifest>(&text).expect("the manifest reads");
        assert_eq!(read, manifest);
        assert_eq!(
            read.record(&manifest.outputs[1]),
            Some(&manifest.outputs[1])
        );
        // A record is found by what it is of, not by where it sits in the file.
        let mut renamed = manifest.outputs[1].clone();
        renamed.scene = "dunes".to_owned();
        assert_eq!(read.record(&renamed), None);
    }

    /// Returns a moment whose frame is a `width` by `height` ramp starting at its own time, which
    /// is enough to tell one pixel from another and one frame from another.
    ///
    /// Nothing is drawn: a sheet is a composition of frames that were already drawn, so copying
    /// their pixels is the whole of what it has to get right, and they can be handed to it
    /// without an adapter.
    fn moment(time: f64, width: u32, height: u32) -> Moment {
        Moment {
            time,
            rendered: RenderedFrame {
                renderer: "test",
                images: vec![RenderedImage {
                    name: graphics::PRESENT_OUTPUT.to_owned(),
                    format: "rgba8unorm".to_owned(),
                    width,
                    height,
                    bytes: (0..width as usize * height as usize * 4)
                        .map(|index| ((index + time as usize) % 251) as u8)
                        .collect(),
                }],
            },
            // A frame that was never described declares nothing, and the sidecars beside it
            // say so rather than borrowing a claim from another image.
            described: Vec::new(),
        }
    }

    /// Returns the size and the pixels of a sheet, as the png it was written as holds them.
    fn decoded(sheet: Vec<u8>) -> (u32, u32, Vec<u8>) {
        let mut reader = ::png::Decoder::new(sheet.as_slice())
            .read_info()
            .expect("the sheet is a png that reads");
        let (width, height) = reader.info().size();
        let mut pixels = vec![0; reader.output_buffer_size()];
        reader
            .next_frame(&mut pixels)
            .expect("the sheet is a png that decodes");
        (width, height, pixels)
    }

    /// Returns the pixels of one tile of a decoded sheet.
    ///
    /// A tile is not a contiguous run of the sheet's bytes: its rows are the sheet's rows cut to
    /// the tile's width, and a sheet of more than one column has the next tile's pixels on the
    /// same line.
    fn tile_of(
        sheet: &[u8],
        sheet_width: usize,
        cell: (usize, usize),
        index: usize,
        across: usize,
    ) -> Vec<u8> {
        let (cell_width, cell_height) = cell;
        let (column, row) = (index % across, index / across);
        (0..cell_height)
            .flat_map(|y| {
                let start = ((row * cell_height + y) * sheet_width + column * cell_width) * 4;
                sheet[start..start + cell_width * 4].to_vec()
            })
            .collect()
    }

    /// Returns the rgba of a moment's frame, which is what a tile of it is made of.
    fn frame_rgba(moment: &Moment) -> Vec<u8> {
        moment
            .rendered
            .present()
            .and_then(RenderedImage::as_frame)
            .expect("the ramp is a frame")
            .rgba
    }

    /// A tile is the frame divided by the step, and the step a sheet defaults to is the one
    /// every sheet written so far was written at.
    #[test]
    fn a_tile_is_the_frame_divided_by_the_step() {
        assert_eq!(tile(1024, 1024, SHEET_STEP), (256, 256));
        // A step of one is the frame itself, which is what `--sheet-full` asks for.
        assert_eq!(tile(1024, 1024, FULL_STEP), (1024, 1024));
        // A frame too small to divide down is shown as itself rather than as nothing, and a step
        // of zero is a step of one rather than a division by nothing.
        assert_eq!(tile(3, 2, SHEET_STEP), (1, 1));
        assert_eq!(tile(1024, 1024, 0), (1024, 1024));
    }

    /// A sheet asked for at full resolution is the same composition of the same frames rather
    /// than a resampling of it, which is the whole claim `--sheet-full` makes.
    #[test]
    fn a_full_sheet_holds_the_frames_and_the_default_holds_a_sample_of_them() {
        let moments: Vec<Moment> = (0..4).map(|index| moment(index as f64, 8, 4)).collect();
        let across = columns(moments.len());

        // A step of one is every frame whole, in the same grid the default step uses.
        let (width, height, whole) =
            decoded(contact_sheet(&moments, FULL_STEP).expect("four frames make a sheet"));
        assert_eq!((width, height), ((8 * across) as u32, (4 * rows(4)) as u32));
        for index in 0..moments.len() {
            // Every tile is the frame it holds, byte for byte, and the frames are four different
            // ramps, so a tile holding the wrong one cannot pass for the right one.
            assert_eq!(
                tile_of(&whole, width as usize, tile(8, 4, FULL_STEP), index, across),
                frame_rgba(&moments[index]),
                "tile {index} is not the frame it holds"
            );
        }

        // The default step is a quarter of that, and every pixel of the smaller tile is a pixel
        // the frame had — sampled, never interpolated.
        let (width, height, sampled) =
            decoded(contact_sheet(&moments, SHEET_STEP).expect("four frames make a sheet"));
        assert_eq!((width, height), ((2 * across) as u32, (1 * rows(4)) as u32));
        let cell = tile(8, 4, SHEET_STEP);
        for index in 0..moments.len() {
            let held = tile_of(&sampled, width as usize, cell, index, across);
            let frame = frame_rgba(&moments[index]);
            for y in 0..cell.1 {
                for x in 0..cell.0 {
                    let at = (y * cell.0 + x) * 4;
                    let from = (y * SHEET_STEP * 8 + x * SHEET_STEP) * 4;
                    assert_eq!(&held[at..at + 4], &frame[from..from + 4]);
                }
            }
        }
    }
}
