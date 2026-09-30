//! Renders a named gallery scene to a png, on the real gpu.
//!
//! This is the authoring loop: a scene is built, compiled by the `StandardScene` compiler,
//! rendered headlessly, and written out as an image. Nothing here is part of the framework —
//! it is the harness that makes a scene visible while it is being written.
//!
//! ```text
//! cargo run -p whiting-device --features wgpu --example render -- <scene> <out.png>
//! ```

use definition::{Definition, DefinitionIdentifier, Registry, SchemaIdentifier};
use graphics::{GRAPHICS_SCHEMA, INSTANCE_SCHEMA};
use scene::gallery::Gallery;
use scene::{CompileContext, Compiler, STANDARD_SCENE_SCHEMA, StandardScene};
use whiting_device::{RenderedFrame, Renderer, WgpuRenderer};

#[tokio::main]
async fn main() {
    if let Err(failure) = run().await {
        eprintln!("error: {failure}");
        std::process::exit(1);
    }
}

/// Renders the scene named on the command line into the png named on it.
async fn run() -> Result<(), String> {
    let mut arguments = std::env::args().skip(1);
    let name = arguments.next().ok_or("usage: render <scene> <out.png>")?;
    let output = arguments.next().unwrap_or_else(|| "out.png".to_owned());
    let scene_name = arguments.collect::<Vec<_>>().join(" ");

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

    let source = Definition::new(
        DefinitionIdentifier::new("gallery.source"),
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        Gallery::build(&name, &scene_name),
    );
    source
        .validate(&registry)
        .map_err(|error| format!("the scene does not satisfy the scene schema: {error}"))?;

    let compiled = compiler
        .compile(&CompileContext {
            scene: &source,
            previous_graphics: None,
            previous_instance: None,
            registry: &registry,
        })
        .map_err(|error| format!("the scene does not compile: {error}"))?;

    let passes = graphics::passes(&compiled.graphics.value)
        .map_err(|error| format!("the emitted passes do not read: {error}"))?
        .len();
    println!("{name}: {passes} passes");

    let frame = renderer
        .render(&compiled.graphics.value, &compiled.instance.value)
        .map_err(|error| format!("the frame does not render: {error}"))?;

    let rendered = RenderedFrame {
        renderer: renderer.name(),
        frame,
    };
    let encoded = rendered
        .to_png()
        .map_err(|error| format!("the frame does not encode: {error}"))?;
    std::fs::write(&output, &encoded)
        .map_err(|error| format!("writing {output} failed: {error}"))?;
    println!(
        "{name}: wrote {output} ({}x{}, by {})",
        rendered.frame.width, rendered.frame.height, rendered.renderer
    );
    Ok(())
}
