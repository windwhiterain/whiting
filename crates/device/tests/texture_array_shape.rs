//! What a texture payload says about its shape reaches the descriptors, the view and the
//! layout, and a count the device cannot build is refused by name.
//!
//! # What these prove, and what they do not
//!
//! `TextureShape` is pure arithmetic on a payload plus a limit, so it is checkable here
//! without an adapter, and these are the cases where getting it wrong would be silent: a
//! pool that clamps instead of refusing, a flat texture whose view dimension moved to
//! `D2Array`, and an array whose view covers one layer instead of all of them.
//!
//! The ordinary case is asserted as a whole rather than field by field, because "a flat
//! texture is unchanged" is the regression that matters: its descriptor, its view and its
//! view dimension are compared against the literals the module used to write, so a change
//! to any one of them fails here.
//!
//! What is **not** proved is that a device accepts these descriptors. Creating a texture,
//! a view or a bind group needs an adapter, and this machine has none; the descriptor path
//! is therefore unexecuted, and what is checked here is that the values handed to it are
//! the ones the document asked for.

#![cfg(feature = "wgpu")]

use definition::Value;
use whiting_device::TextureShape;

/// The most layers a device allows, which is what the floor this renderer requests puts it
/// at. It is spelled out here because `TextureShape::read` is given the limit rather than
/// reading it, and the test is about the arithmetic rather than about the number: what a
/// device really allows is read from the device in the renderer.
const MAX_LAYERS: u32 = 256;

#[test]
fn a_flat_texture_is_the_descriptor_and_the_view_it_always_was() {
    let shape = TextureShape::read("surface", &payload("Texture", None), MAX_LAYERS)
        .expect("a flat texture payload reads");
    assert_eq!(
        (shape.width, shape.height, shape.layers),
        (1024, 512, 1),
        "a flat texture is one layer, whatever its pixel count"
    );

    let descriptor = shape.descriptor("surface", wgpu::TextureFormat::Rgba8Unorm, 1);
    assert_eq!(
        descriptor.size,
        wgpu::Extent3d {
            width: 1024,
            height: 512,
            depth_or_array_layers: 1,
        },
        "the extent is the payload's size and one layer, which is the literal the module \
         used to write"
    );
    assert_eq!(
        descriptor.dimension,
        wgpu::TextureDimension::D2,
        "a flat texture was a D2 texture and still is"
    );
    assert_eq!(
        shape.view_descriptor(),
        wgpu::TextureViewDescriptor::default(),
        "the view of a flat texture is the default descriptor, so a document that says \
         nothing is byte for byte the document it was"
    );
    assert_eq!(
        shape.view_dimension(),
        wgpu::TextureViewDimension::D2,
        "a shader declaring texture_2d still matches a flat texture"
    );
}

#[test]
fn a_declared_layer_count_reaches_the_descriptor_and_the_view() {
    let shape = TextureShape::read("pool", &payload("TextureArray", Some(64)), MAX_LAYERS)
        .expect("an array of sixty four layers is within the limit");
    assert_eq!(shape.layers, 64, "the count the document wrote is the count read");

    let descriptor = shape.descriptor("pool", wgpu::TextureFormat::Depth32Float, 1);
    assert_eq!(
        descriptor.size.depth_or_array_layers, 64,
        "the count reaches the extent, which is where a backend reads the depth of a texture"
    );
    assert_eq!(
        descriptor.dimension,
        wgpu::TextureDimension::D2,
        "an array of two dimensional images is still a two dimensional texture; a volume \
         would be D3 and is a shape no document here declares"
    );

    let view = shape.view_descriptor();
    assert_eq!(
        (view.dimension, view.array_layer_count, view.base_array_layer),
        (Some(wgpu::TextureViewDimension::D2Array), Some(64), 0),
        "a shader reads every layer, so the view is the whole array counted from the first"
    );
    assert_eq!(
        shape.view_dimension(),
        wgpu::TextureViewDimension::D2Array,
        "the bind group entry declares the array a shader reads with texture_depth_2d_array"
    );

    let page = shape.layer_view_descriptor(40);
    assert_eq!(
        (page.dimension, page.base_array_layer, page.array_layer_count),
        (Some(wgpu::TextureViewDimension::D2), 40, Some(1)),
        "a pass draws into one layer as a flat image, because the layer is the view"
    );
}

#[test]
fn a_count_above_what_the_device_allows_is_refused_by_name_rather_than_clamped() {
    let error = TextureShape::read("pool", &payload("TextureArray", Some(257)), MAX_LAYERS)
        .expect_err("a pool deeper than the device allows cannot be built");
    let message = error.to_string();
    assert!(
        message.contains("257") && message.contains("max_texture_array_layers"),
        "the refusal names the count asked for and the limit it broke, so a reader knows \
         which number to change: {message}"
    );
    assert!(
        TextureShape::read("pool", &payload("TextureArray", Some(256)), MAX_LAYERS).is_ok(),
        "a count of exactly the limit is legal, so the limit is a limit and not an off by one"
    );
}

#[test]
fn a_count_below_one_is_refused_rather_than_rounded_up() {
    for count in [0, -1] {
        let error = TextureShape::read("pool", &payload("TextureArray", Some(count)), MAX_LAYERS)
            .expect_err("a texture with no layers is not a smaller texture");
        assert!(
            error.to_string().contains("at least one layer"),
            "the refusal explains what a count of zero means: {error}"
        );
    }
}

#[test]
fn a_pass_may_not_draw_into_a_layer_the_pool_does_not_have() {
    let shape = TextureShape::read("pool", &payload("TextureArray", Some(4)), MAX_LAYERS)
        .expect("a pool of four pages reads");
    assert_eq!(
        shape.check_layer("pool", "shadow", 3).expect("page three exists"),
        3,
        "the last page is a page the pool has"
    );
    for layer in [4, -1] {
        let error = shape
            .check_layer("pool", "shadow", layer)
            .expect_err("a page past the end of the pool is not a page");
        assert!(
            error.to_string().contains("4 layers"),
            "the refusal says how deep the pool is: {error}"
        );
    }
}

/// Builds a texture payload of the given shape, with a layer count or without one.
fn payload(tag: &str, layers: Option<i64>) -> Value {
    let mut fields = vec![
        ("width".to_owned(), Value::Int(1024)),
        ("height".to_owned(), Value::Int(512)),
    ];
    if let Some(layers) = layers {
        fields.push(("layers".to_owned(), Value::Int(layers)));
    }
    fields.push((
        "format".to_owned(),
        Value::String("depth32float".to_owned()),
    ));
    Value::variant(tag, fields)
}
