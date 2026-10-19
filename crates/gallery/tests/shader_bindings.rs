//! The binding numbers and the word offsets a generated shader writes down, checked against the
//! schema that decides them.
//!
//! # What this holds that `naga` cannot
//!
//! [`whiting_device::check_resources`] compares a shader's declarations against a pass's layout,
//! and it needs the layout — which means a `Bindings`, which means buffers a real device made.
//! What it cannot see from the shader's side is **order**: a pass that numbered its binding points
//! one way and a template that listed them another way disagree by exactly one place, and every
//! resource after the disagreement lands on the wrong number. That is not a resource the check
//! would refuse, because each one still exists at *some* number. It is a frame of the wrong
//! numbers, which is the failure mode this project has now hit twice by moving a binding out of
//! the bound resources and into a target.
//!
//! # The one rule this file exists to hold
//!
//! **A stage's declared binding numbers ascend in the same order its names do.** The device
//! numbers a pass's binding points in name order from one, with the uniform at zero and a hole
//! wherever the pass renders into one of its own declarations. The holes are not visible here and
//! do not need to be: a hole moves every number after it, and so does a misordered name, and the
//! two show up the same way — as a shader whose numbers do not run with its alphabet. So the
//! assertion is about the shader's own list, read in the order the device would read it, and it
//! says nothing about where the holes are.
//!
//! The second half is the two numbers the lighting template spells out rather than derives: a
//! page's side, and how many words a light record is. Both are the compiler's facts, restated in
//! `WGSL`, and the restatement is the whole risk.

use scene::lighting::light_record_stride;
use scene::vshadow::PAGE_SIZE;
use whiting_gallery::kit::gallery_custom_schema;
use whiting_gallery::shaders;

/// A surface body empty enough to assemble.
const NOTHING: &str = "";

/// Returns every `@binding(n) var name` a generated module declares, in the order the module
/// declares them — which is the order the device numbers them.
fn declarations(module: &str) -> Vec<(u32, String)> {
    let mut found = Vec::new();
    for line in module.lines() {
        let Some(rest) = line.strip_prefix("@group(0) @binding(") else {
            continue;
        };
        let Some((digits, rest)) = rest.split_once(')') else {
            continue;
        };
        let number = digits
            .parse::<u32>()
            .unwrap_or_else(|_| panic!("`{digits}` is not a binding number"));
        // `var<storage, read> light_records: array<f32>;` and `var shadow_pool: ...` both name the
        // resource between the address space and the colon, and the address space is bracketed.
        let Some(after_var) = rest
            .find("var")
            .map(|at| rest[at + "var".len()..].trim_start())
        else {
            continue;
        };
        let after_space = match after_var.strip_prefix('<') {
            Some(bracketed) => match bracketed.find('>') {
                Some(end) => bracketed[end + 1..].trim_start(),
                None => continue,
            },
            None => after_var,
        };
        let name = after_space
            .split(':')
            .next()
            .unwrap_or("")
            .trim()
            .to_owned();
        if !name.is_empty() {
            found.push((number, name));
        }
    }
    found
}

/// Returns the number a `const NAME: u32 = NNNu;` line writes down.
fn constant(module: &str, name: &str) -> u32 {
    let head = format!("const {name}: u32 = ");
    let line = module
        .lines()
        .find(|line| line.starts_with(&head))
        .unwrap_or_else(|| panic!("the template declares no `{name}`"));
    line[head.len()..]
        .trim_end_matches(';')
        .trim_end_matches('u')
        .parse()
        .unwrap_or_else(|_| panic!("`{line}` does not state a number"))
}

#[test]
fn the_lighting_stage_declares_its_bindings_in_the_order_the_device_numbers_them() {
    let module = shaders::lighting(
        "",
        NOTHING,
        NOTHING,
        shaders::Projection::Latlong,
        &gallery_custom_schema(),
    );
    let declared = declarations(&module);
    assert!(
        declared.len() > 4,
        "a template that declared nothing would pass an ordering test it was written to fail: \
         {declared:?}"
    );
    let mut earlier: Option<(u32, &str)> = None;
    for (number, name) in &declared {
        if name == "uniforms" {
            // Binding zero is the uniform, and it is first by rule rather than by alphabet.
            assert_eq!(*number, 0, "the uniform is binding zero");
            continue;
        }
        if let Some((before, before_name)) = earlier {
            assert!(
                *number > before && name.as_str() > before_name,
                "`{before_name}` at {before} and `{name}` at {number} do not run together: the \
                 device numbers a pass's binding points in **name** order, so a shader that lists \
                 them in any other order has one of them at the other's number, and every point \
                 after it too"
            );
        }
        earlier = Some((*number, name));
    }
}

#[test]
fn the_lighting_stage_reads_the_atlas_the_shadowing_stage_writes() {
    let module = shaders::lighting(
        "",
        NOTHING,
        NOTHING,
        shaders::Projection::Latlong,
        &gallery_custom_schema(),
    );
    let names: Vec<String> = declarations(&module)
        .into_iter()
        .map(|(_, name)| name)
        .collect();
    assert!(
        names.iter().any(|name| name == "shadow_atlas"),
        "a fragment that shades from a light's pages has to bind the array they were drawn into, \
         and it is the one binding point a lighting pass gained in this stage: {names:?}"
    );
    assert!(
        names.iter().any(|name| name == "shadow_pool"),
        "and it keeps the pool, because the kinds whose grids have not landed are still drawn \
         into one page each: {names:?}"
    );
}

#[test]
fn a_record_reader_stays_inside_the_record_the_compiler_packed() {
    let module = shaders::lighting(
        "",
        NOTHING,
        NOTHING,
        shaders::Projection::Latlong,
        &gallery_custom_schema(),
    );
    let words = u32::try_from(light_record_stride()).expect("a stride that fits a u32");
    assert_eq!(
        constant(&module, "LIGHT_RECORD_WORDS"),
        words,
        "the template writes the record's width down and so does the schema that packs it, and \
         the two are one fact about one buffer"
    );
    // Every `at + N` the reader uses is an offset inside one record; one that reached past the
    // width would read the next light's colour, or the page table, and call it a field.
    for offset in offsets_of(&module, "at") {
        assert!(
            offset < words,
            "`at + {offset}u` is past the end of a {words} word record, and the run after it is \
             the page table"
        );
    }
}

#[test]
fn a_page_is_the_size_the_allocator_allocates() {
    let module = shaders::lighting(
        "",
        NOTHING,
        NOTHING,
        shaders::Projection::Latlong,
        &gallery_custom_schema(),
    );
    assert_eq!(
        constant(&module, "PAGE_TEXELS"),
        PAGE_SIZE,
        "a page's side is the constant the grid is measured in, and the walk and the allocator \
         are the two ends of the same texel"
    );
}

/// Returns every `subject + N` written in the module, for the one subject asked about.
fn offsets_of(module: &str, subject: &str) -> Vec<u32> {
    let head = format!("{subject} + ");
    let mut found = Vec::new();
    for (at, _) in module.match_indices(&head) {
        let tail = &module[at + head.len()..];
        let digits: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(number) = digits.parse::<u32>() {
            found.push(number);
        }
    }
    found
}
