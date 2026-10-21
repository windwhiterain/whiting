# 多层体量 — a mass with levels, so storeys are one building and not a stack of relations

## The verdict this starts from

A multi-storey building, and an upper floor that **projects** past the one below it, belong to the building's own
primitive. They are not a relation between masses. The third relation that was proposed for a cantilevered storey
was the wrong shape, and the owner said so in one sentence: *"isn't this what the multi-storey building's
primitive should model? Why add a stacked relation?"*

## What exists today

`Mass` is **one outline and one top elevation**: a shell whose wall is vertical from the outline up to `top`, with
`crowns` able to raise individual nodes (which is how the 土掌房's gable walls rise into their roof). A building of
several storeys is therefore drawn as **several masses**, and the 鼓楼 is seven of them chained by `Join::Storey` —
a relation that exists only because the primitive cannot say "one building, seven floors".

## The primitive

```rust
pub struct Mass {
    /// The shell's horizontal sections, bottom first, at least two: the first is the foot, the last the top.
    pub levels: Vec<Level>,
    …
}

pub struct Level {
    /// The elevation this section is at.
    pub elevation: f32,
    /// The outline **at that elevation**: the outer face of the wall there.
    pub outline: Vec<PlanNode>,
}
```

The wall between two consecutive levels is the band between their outlines. Three things follow, and each one is
what an existing feature had to fake:

| the two outlines | the wall | what it gives |
| --- | --- | --- |
| the same | a vertical band | today's mass, exactly |
| the upper **inside** the lower | a band leaning in | **收分** — the battered earth wall the 土掌房 has been missing since round eight |
| the upper **outside** the lower | a band leaning out, plus a **soffit** where the floor projects | **挑出** — 骑楼's upper floor, 吊脚楼's gallery, the bridge's pavilion floor |

The storeys' floors are plates at the interior levels, and they hang off the mass the way its own plate does now.
Nothing about `Roof` changes: a roof still names the mass it sits on and its eave is still a plan loop.

## Invariants

- **the levels ascend strictly**: `levels[i].elevation < levels[i+1].elevation`;
- **every level's outline is closed, simple and positively wound**, and its `plan_area` is positive — the same
  three statements the one outline makes today, made once per level;
- **consecutive levels differ** — an outline equal to the one below is not an error but is also not a level, and
  the constructor folds it away so a document cannot carry a zero-height band;
- **the shell stays closed**: a projecting level adds a soffit face between the two outlines' shared boundary, and
  a receding level adds a horizontal face on top of the one below. Both are the wall's own, and
  `SolidOpen`/`SolidNonManifold` are what catch a mistake, unchanged;
- **no relation is added.** Two masses never overlap because a building is one mass with levels; `Join::Storey`
  stays for the case it really is — two *buildings* on one site, one standing on the other, which is a site
  question and not a storey question.

## What it costs

- **the wall expander** goes from one band per edge to one band per edge per level pair, plus the faces at the
  levels. That is the whole of the change; the plates, the openings, the grids and the roof derivation all keep
  reading "a mass's face" and "a mass's elevation", now with `levels.len()` of each;
- **openings name a face and a row**, and a row is a band: the rows of a face become the intervals between
  consecutive levels, so an opening's `row` already means what it should;
- **migration is an identity.** A mass built as it is today is *one* level pair — foot and top, same outline — and
  the expansion of a single vertical band is exactly what it is now. **All twenty-three scenes should be
  unchanged to the row** until one of them authors a second level, which is the test the change has to pass.

## What it unlocks, in the order it would be worth doing

1. **鼓楼 as one mass of seven levels**, with `Join::Storey` then used for nothing at all in that scene;
2. **骑楼 street: the upper floor projects** over the arcade — the type's definition, and the one thing the street
   still cannot say;
3. **吊脚楼: the gallery's floor projects** over the bank it hangs on;
4. **土掌房: 收分**, the battered wall, from a level whose outline is inset a few centimetres per metre;
5. **风雨桥: the pavilion's floor projects** over the deck, instead of its eaves doing all the work.

## What this note does not decide

- whether a level carries its own `wall` thickness or whether the wall is one thickness for the shell — a
  battered wall's *thickness* is usually constant and its *outline* moves, which suggests the outline is the whole
  of the answer and `Mass::wall` stays single;
- whether the plate at an interior level is a `Slab` the scene authors (today's way) or something the mass carries
  — the first is enough and is what the drum tower already does;
- the cost of the change, which is a round of its own and cannot be measured from a note.
