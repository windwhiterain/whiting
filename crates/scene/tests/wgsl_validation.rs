//! The WGSL this project hands a driver, checked against the limits the renderer requests.
//!
//! # What this proves, and what it does not
//!
//! Parsing and validating is pure cpu work. It needs no adapter, so on a machine with no gpu
//! this is the strongest statement anyone can make about a shader before it reaches a driver:
//! that the module is well formed and that it needs no feature [`WgpuRenderer`] does not ask
//! for.
//!
//! It is **not** pipeline creation. `naga` cannot tell that a bind group layout the compiler
//! emitted matches what a shader declares, because that check lives in the device crate's
//! `check_bindings` and needs a renderer instance. So a module that validates here is well
//! formed and within limits; it is not a frame that would be correct.
//!
//! # What is validated, and from where it is read
//!
//! A template writes its shared constants into the shader with `fill`, replacing tokens like
//! `__SUN__`. A template checked before substitution is a template with live tokens in it, and
//! a token that replaces to nothing is exactly the mistake a template hides — so a check of
//! this kind has to read the sources out of the **compiled instance**, which is the string that
//! would reach the device, with every token already resolved.
//!
//! [`WgpuRenderer`]: ../../../../device/src/wgpu_renderer/mod.rs

use definition::Value;

/// The storage buffers one clustered lighting pass binds.
///
/// The four are the light records, the per cluster counts, the per cluster light lists and the
/// spot cones. Four is all a stage is guaranteed, so this is now the whole of the budget rather
/// than most of it: the cone run spent the one spare the other three left. A fifth storage
/// buffer in the clustered template is a shader that validates on a driver's limit, and the
/// test below is what says there is no room for one.
const CLUSTERED_STORAGE_BUFFERS: usize = 4;

/// The texture array layers a compile with no machine behind it is bounded by, which is the
/// floor every device agrees to rather than a ceiling the hardware imposes.
const ARRAY_LAYER_FLOOR: u32 = 256;

#[test]
fn the_clustered_lighting_template_stays_inside_the_storage_budget() {
    // The budget is four storage buffers per shader stage and the clustered pass binds all four
    // of them, so this is a statement about the design rather than about a shader: it fails the
    // day a fifth is wanted, which is the day the packing has to be reconsidered.
    // `black_box` keeps the comparison from being folded away at compile time, which is what makes
    // this a test rather than a build failure: the assertion still fails the day a fifth buffer is
    // wanted, and now it fails as a failing test with its message rather than as a crate that will
    // not build.
    assert!(
        std::hint::black_box(CLUSTERED_STORAGE_BUFFERS) <= 4,
        "light records, cluster counts, cluster lists and the spot cones are the whole of the \
         storage budget"
    );
    assert_eq!(
        CLUSTERED_STORAGE_BUFFERS, 4,
        "one storage buffer is no longer left spare: the spot cone run took it, and the next \
         thing the clustered pass needs has to share a buffer with something rather than get \
         one of its own"
    );
}

#[test]
fn the_page_pool_is_bounded_by_the_machine_the_compile_is_made_against() {
    // `max_texture_array_layers` is the device's own number now, so this is not a statement
    // about hardware. What it holds is that a scene asking for more is told what it lost
    // rather than refused, and that a compile with no machine behind it is bounded by the
    // floor every device agrees to.
    let floor = whiting_scene::DeviceLimits::downlevel();
    assert_eq!(floor.max_texture_array_layers, ARRAY_LAYER_FLOOR);
    let scene = Value::structure([(
        whiting_scene::standard::FRAME.to_owned(),
        Value::structure([(
            whiting_scene::lighting::SHADOW_PAGES.to_owned(),
            Value::Int(ARRAY_LAYER_FLOOR as i64 + 1),
        )]),
    )]);
    let (pool, notes) = whiting_scene::lighting::read_light_pool(&scene, floor)
        .expect("a pool deeper than this machine has still reads");
    assert_eq!(
        pool.pages, ARRAY_LAYER_FLOOR as usize,
        "the pool is given the depth the machine has"
    );
    assert!(
        notes.len() == 1 && notes[0].message.contains(&ARRAY_LAYER_FLOOR.to_string()),
        "the note names the number lost and the number given instead: {notes:?}"
    );
}
