# Report a `reflect` only when dropping it loses something

## The question

`crates/gallery/src/kit.rs:672-686` writes `reflect` into every gallery material's custom data,
including the ones that go through `surface()` and can only ever hold `0.0`. The emitter cannot
carry the field, so it reports it once per material per pass — **472 of 498 report lines
gallery-wide describe a loss of nothing** — and under `LossyMode::Reject`, which is the `#[default]`,
it refuses the export instead.

The counts, the `Reject` path, the packer that makes the obvious fix impossible, and the sources are
in [the-loss-report-spends-472-lines-on-a-field-declared-as-zero](../investigate/the-loss-report-spends-472-lines-on-a-field-declared-as-zero.md).

## Three ways to do it

### A — write the field only when it means something (gallery side)

> **This does not work, and it was the recommendation before it was tried against the packer.** The
> reason is in "Decided" below: `reflect` is declared a *required* field, and `pack_struct` refuses
> a value missing a field its own schema declares. The sketch is kept because the reasoning behind it
> — that the root cause is a document asserting something it does not mean — is what makes B the
> right half rather than a patch.

One condition in `surface_reflecting`, where the field is built:

```rust
let mut fields = vec![
    (TINT.to_owned(), triple(tint)),
    (METALLIC.to_owned(), Value::Float(metallic)),
    (ROUGHNESS.to_owned(), Value::Float(roughness)),
    (EMISSIVE.to_owned(), triple(emissive)),
];
// The field's meaning is "this surface shows the sky at a grazing angle". A material that
// does not is not saying so with a zero; it is saying it has nothing to say.
if reflect != 0.0 {
    fields.push((REFLECT.to_owned(), Value::Float(reflect)));
}
Value::structure(fields)
```

**For:** fixes the source rather than the symptom, so both the report and the `Reject` refusal go at
once. It is **one rule, not two** — it covers `surface()` (always `0.0`), `foldyard` (all thirteen
groups pass `0.0`) and `stilllife`/`lanterns`/`monoliths` (which genuinely reflect) in the same
place, and it leaves `reflect` present exactly when a surface reflects, which is the meaning
`kit.rs:667-671` gives it. Smallest change in the repository that fixes both symptoms.

**Against:** does nothing for a scene outside this gallery that writes an explicit `0.0`.

### B — report a drop only when dropping it loses something (emitter side)

`material.rs` reads the value and skips the report, and the refusal, when it is the one the emitter
would have written anyway.

**For:** fixes every scene, this gallery and any other. Makes the report honest as a general
contract rather than by this repository's cooperation.

**Against:** it needs a **per-field table** saying which fields have a "dropping this changes
nothing" default, and this repository has already been bitten by exactly that shape of bug:
`attribute_map::is_mappable` and `geometry.rs`'s `UV_FIELDS` were two lists answering one question
and disagreeing, which put a false claim in the report on every mesh of every scene until they
were merged (recorded in
[cycles-emitter-dropped-material-fields](../investigate/cycles-emitter-dropped-material-fields.md)).
Adding a second such table invites the same failure. It also cuts against the stated reason
`Reject` exists at all — "a silently missing attribute produces a plausible image" — by teaching
the emitter to let a field through without carrying it.

### C — both

**For:** A fixes the documents, B makes the report right for documents A did not and will not
touch.

**Against:** two mechanisms for one fact, and the census of which is responsible becomes a question
with two answers. Given that this repository's own history is a story of two lists disagreeing,
one mechanism is worth more than two that agree today.

## Decided

**B shipped. A was recommended first and was wrong**, and the error is worth keeping because the
thing it missed is a property of the whole document format rather than of this field.

`gallery_custom_schema()` declares `reflect` as a **required** `Float` (`kit.rs:642-649`), so a
material that omits it is refused by `validate_fields` (`schema.rs:248-258`) with `MissingField`
before anything is emitted. A schema `default` does not rescue it: `pack_struct`
(`packing.rs:114-120`) walks the schema's field order and answers an absent field with *"the data
is missing a field its own schema declares"*, and the function that would fill it —
`Value::defaulted()` (`value.rs:353-372`), reached through `defaulted_value()` — **has no production
caller**. The compile path substitutes no defaults at all.

So **a declared field is not optional in this format, whatever its value**, and "write it only when
it means something" cannot be done where the value is written. Every value the packer reads is
required by the schema that declares the layout.

**B is what remains, and it is the safe half of A.** `cycles_export/material.rs` reads the dropped
field through the same `parameter` reader every other parameter uses, and
`loses_nothing_when_dropped` skips both the record and the refusal when the value is the one the
emitted node already holds:

| | before | after |
|---|---|---|
| `reflect` lines, gallery-wide | 498 | **11** |
| `terraces` | 405 | **0** |
| `foldyard` | 39 | **0** |

**All eleven rendered frames are byte-identical before and after** by SHA256 over
`out/rl-<scene>.png`. Nothing else in any scene's report moved: the stage-line counts are unchanged
and `custom_data` still equals `reflect` in every scene. `cargo test --release -p whiting-scene`
passes.

The 11 that remain are the genuine losses, and 15 of the three scenes' original 26 lines turned out
to be zero-valued too. `LossyMode::Reject` itself was not run; see the investigation note.

## A's real version, and why it is not cheap

Giving the field its stated meaning back means letting it be absent, which means changing
`pack_struct` to substitute schema defaults — and `pack_struct` packs **instance rows and point
attributes** as well as custom data, so that is a change to the document format's layout contract
for the whole system, not to this field. A silent layout change is the failure this backend exists
to prevent, so it would carry its own measurement budget over every gallery frame. **It is a
separate proposal with its own evidence, not a refinement of this one**, and it buys a document that
does not carry a meaningless zero, not a picture that is more correct.

## Not settled here

**Whether `foldyard` should stop declaring `reflect` at all.** All thirteen of its groups pass
`0.0` through `Group::new`, so under A it stops declaring it and the 39 lines go. If a group ever
needs a non-zero value the field reappears on that material alone, which is the behaviour the
field's documentation already describes.

**Whether the `Reject` refusal is separately worth fixing.** Under A, a scene built entirely from
`surface()` has no custom-data loss left, but the gallery's point schemas still refuse every scene
under `Reject` — which is what `render_cycles.rs:53-56` already says. A is not what makes the
gallery strict-mode-exportable; it removes one reason from the list.

**Whether `pack_struct` should substitute schema defaults.** That is what makes the field absent
rather than merely unreported, and it is a change to the layout contract for every struct in the
system. Nothing here argues against it; it argues that it is not this change.

**What happens to the `reflect`-declaring scenes under `Reject` afterwards.** `lanterns`,
`monoliths` and `stilllife` declare a real value, and refusing them is correct: the picture would
otherwise come back without the mirror that the document asked for.

## Verification

Done.

1. The eleven-scene count: `reflect` lines fall **498 to 11**, with `terraces` 405 to 0 and
   `foldyard` 39 to 0, and the three scenes that declare reflection fall 8, 10 and 8 to 3, 3 and 5.
2. **The rendered frames must not move, and that is the check that matters.** All eleven are
   byte-identical before and after by SHA256 over `out/rl-<scene>.png` — dropping a declared zero
   cannot change a pixel, and none changed.
3. Every other line in every report is unchanged: stage counts are identical and `custom_data`
   still equals `reflect` in all eleven scenes, so nothing else was silenced.
4. `cargo test --release -p whiting-scene` passes.

**No new test is proposed.** A test asserting "a zero `reflect` is not reported" tests the exporter
against itself; the frames not moving is the evidence, and the report count is the visible one. The
one thing it does not cover is `LossyMode::Reject`, which nothing in the repository selects over a
gallery scene.

## Not included

**Carrying `reflect`.** Separately measured and separately wrong: the document's field is grazing-
angle sky reflectivity and `principled_bsdf`'s `Specular IOR Level` is normal-incidence
reflectance, and writing it cost `landscape` agreement on both bands. That is a mapping question
and this is not it.