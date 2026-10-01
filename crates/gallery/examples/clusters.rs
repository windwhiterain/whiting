//! Prints what the cluster assignment produced for a gallery scene.
//!
//! A diagnostic rather than a test: it is what a scene author runs when a frame's lighting has
//! edges in it that the geometry does not explain. It compiles the named scene and prints the
//! count of every cluster and the first slots of each row, which is the data the shading pass
//! reads.
//!
//! ```text
//! cargo run -p whiting-gallery --example clusters -- stilllife
//! ```

use definition::{Definition, DefinitionIdentifier, Path, Registry, SchemaIdentifier, Value};
use graphics::{GRAPHICS_SCHEMA, INSTANCE_SCHEMA};
use scene::{CompileContext, Compiler, STANDARD_SCENE_SCHEMA, StandardScene};
use whiting_gallery::Gallery;

fn main() {
    let name = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "stilllife".to_owned());
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
    let source = Definition::new(
        DefinitionIdentifier::new("diagnostic.source"),
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        Gallery::build(&name),
    );
    if let Err(error) = source.validate(&registry) {
        eprintln!("the scene does not validate: {error}");
        return;
    }
    let compiled = StandardScene::new(
        DefinitionIdentifier::new("diagnostic.graphics"),
        DefinitionIdentifier::new("diagnostic.instance"),
    )
    .compile(&CompileContext {
        scene: &source,
        previous_graphics: None,
        previous_instance: None,
        registry: &registry,
        camera: 0,
    })
    .expect("the scene compiles");

    for note in &compiled.notes {
        println!("note at {}: {}", note.path, note.message);
    }

    let counts = numbers(&compiled.instance.value, "cluster_light_counts");
    let labels: Vec<String> = graphics::passes(&compiled.graphics.value)
        .expect("the passes read")
        .iter()
        .filter_map(|pass| pass.label().ok().map(str::to_owned))
        .collect();
    println!("{} passes, in the order they run:", labels.len());
    let mut seen: Vec<(String, usize)> = Vec::new();
    for label in &labels {
        let kind = label.split('/').take(2).collect::<Vec<&str>>().join("/");
        match seen.iter_mut().find(|(name, _)| *name == kind) {
            Some((_, count)) => *count += 1,
            None => seen.push((kind, 1)),
        }
    }
    for (kind, count) in &seen {
        println!("  {count:3} x {kind}");
    }
    let lighting: Vec<&String> = labels
        .iter()
        .filter(|l| l.starts_with("lighting/"))
        .collect();
    println!("the lighting passes run in this order: {lighting:?}");
    for pass in graphics::passes(&compiled.graphics.value).expect("the passes read") {
        let label = pass.label().unwrap_or("<unlabelled>").to_owned();
        if !label.starts_with("lighting/") && !label.starts_with("depth/") {
            continue;
        }
        let targets: Vec<String> = pass
            .targets()
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect();
        let depth = pass.optional("depth").map(|value| value.summary());
        println!("  {label} targets {targets:?} depth state {depth:?}");
    }
    let indices = numbers(&compiled.instance.value, "cluster_light_indices");
    let records = numbers(&compiled.instance.value, "light_records");
    println!(
        "{} clusters, {} counts, {} indices, {} record floats ({} lights)",
        counts.len(),
        counts.len(),
        indices.len(),
        records.len(),
        records.len() / 24
    );
    let cap = if counts.is_empty() {
        0
    } else {
        indices.len() / counts.len()
    };
    println!("the cap the buffers imply is {cap}");
    let distinct: std::collections::BTreeSet<u32> =
        counts.iter().map(|count| *count as u32).collect();
    let mut histogram: Vec<(u32, usize)> = Vec::new();
    for value in &distinct {
        histogram.push((
            *value,
            counts
                .iter()
                .filter(|count| **count as u32 == *value)
                .count(),
        ));
    }
    histogram.sort_unstable();
    println!("counts by value: {histogram:?}");
    for cluster in [0, 1, 15, 16, 100, 3312, 3313, 3455] {
        if cluster * cap + 4 <= indices.len() {
            println!(
                "cluster {cluster}: count {}, row {:?}",
                counts[cluster],
                &indices[cluster * cap..cluster * cap + 5]
            );
        }
    }

    // One record is `color`(3), `page`(1), `position`(3), `radius`(1), `shadow_projection`(16).
    for light in 0..records.len() / 24 {
        let record = &records[light * 24..light * 24 + 24];
        println!(
            "light {light:2}: colour {:?}, page {}, position {:?}, radius {}",
            &record[0..3],
            record[3],
            &record[4..7],
            record[7]
        );
    }
}

/// Returns one instance entry's bytes as floats.
fn numbers(instance: &Value, entry: &str) -> Vec<f32> {
    let path = Path::root().key("entries").key(entry);
    let Ok(resolved) = path.resolve(instance) else {
        eprintln!("no entry named `{entry}`");
        return Vec::new();
    };
    let Some(bytes) = resolved
        .as_value()
        .variant_fields()
        .and_then(|fields| fields.get("data"))
        .and_then(Value::as_bytes)
    else {
        eprintln!("`{entry}` carries no bytes");
        return Vec::new();
    };
    bytes
        .chunks_exact(4)
        .map(|word| f32::from_le_bytes([word[0], word[1], word[2], word[3]]))
        .collect()
}
