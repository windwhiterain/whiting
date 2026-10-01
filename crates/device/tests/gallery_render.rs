//! A scene that samples an image, rendered on a real gpu.
//!
//! # What this proves that the compiler's own tests cannot
//!
//! `crates/scene/tests/image_channel.rs` proves the compiler emitted a texture payload, a sampler
//! and a binding point. It cannot prove the gpu ever read them: a texture that was bound but never
//! filled, or a sampler that was never created, produces a frame that is merely wrong, and a wrong
//! frame is still a frame.
//!
//! So the test here renders the one gallery scene whose surface detail comes from an image, and
//! reads the pixels back. What it looks for is **variation**: the scene's image is a tile pattern,
//! so a frame drawn from it is a frame of several colours, and a frame drawn from a texture that
//! never reached the gpu is one flat colour however the lighting went.

#![cfg(feature = "wgpu")]

use definition::{Definition, DefinitionIdentifier, Registry, SchemaIdentifier};
use gallery::Gallery;
use scene::{CompileContext, Compiler, STANDARD_SCENE_SCHEMA, StandardScene};
use whiting_device::{Renderer, WgpuRenderer};

/// A note printed when there is nothing to test on.
const NO_ADAPTER: &str = "skipped: no gpu adapter is present";

#[test]
fn a_scene_that_samples_an_image_renders_more_than_one_colour() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    let frame = render(&renderer, "tiles").expect("the tiled floor renders");

    // The scene's image is a four by four tile pattern, and the floor is drawn at a low angle,
    // so the level chain averages much of it away: what the frame holds is the pattern's own
    // colours and the greys between them. A frame of **one** colour is what a texture that never
    // reached the gpu looks like, and an empty frame is what a scene drawn wrong looks like, so
    // the two things this holds are that something was sampled and that a floor was drawn on.
    let distinct: std::collections::BTreeSet<[u8; 4]> = frame
        .rgba
        .chunks_exact(4)
        .map(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
        .collect();
    assert!(
        distinct.len() > 1,
        "a floor sampled from a tile pattern draws more than one colour, and this frame is one \
         colour: {distinct:?}"
    );
    let sky = [0u8, 0, 0, 255];
    assert!(
        distinct.contains(&sky) || distinct.len() > 2,
        "the floor was drawn over the frame's own clear: {distinct:?}"
    );
}

#[test]
fn every_gallery_scene_renders() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    // The gallery is what tells a rule that can be changed safely from one that cannot: a
    // binding numbering that shifts by one still renders, and still renders the wrong thing, so
    // what this holds is that none of them stops rendering at all.
    for name in Gallery::names() {
        render(&renderer, name).unwrap_or_else(|error| panic!("`{name}` does not render: {error}"));
    }
}

/// Returns a renderer, or nothing when the machine has no adapter.
fn renderer() -> Option<WgpuRenderer> {
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    runtime.block_on(async {
        if !WgpuRenderer::probe().await {
            return None;
        }
        WgpuRenderer::new().await.ok()
    })
}

/// Compiles a gallery scene and renders it.
fn render(renderer: &WgpuRenderer, name: &str) -> Result<whiting_device::Frame, String> {
    let mut registry = Registry::new();
    scene::register_standard_scene_schema(&mut registry);
    registry.put_schema(
        SchemaIdentifier::new(graphics::GRAPHICS_SCHEMA),
        graphics::graphics_schema(),
    );
    registry.put_schema(
        SchemaIdentifier::new(graphics::INSTANCE_SCHEMA),
        graphics::instance_schema(),
    );
    let source = Definition::new(
        DefinitionIdentifier::new("gallery.source"),
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        Gallery::build(name),
    );
    source
        .validate(&registry)
        .map_err(|error| format!("the scene does not satisfy the scene schema: {error}"))?;
    let compiler = StandardScene::new(
        DefinitionIdentifier::new("gallery.graphics"),
        DefinitionIdentifier::new("gallery.instance"),
    );
    let compiled = compiler
        .compile(&CompileContext {
            scene: &source,
            previous_graphics: None,
            previous_instance: None,
            registry: &registry,
            camera: 0,
        })
        .map_err(|error| format!("the scene does not compile: {error}"))?;
    renderer
        .render(&compiled.graphics.value, &compiled.instance.value)
        .map_err(|error| format!("the frame does not render: {error}"))
}
