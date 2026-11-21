# How a named scene becomes a PNG, and what can be dialled

**Question this file answers:** how does a named scene become a PNG, and what can be dialled?
**Date of the measurements below: 2026-10-18.** All code claims cite `file:line` against the
checkout at `C:\resource\whiting`. Nothing here is inferred from a run's output unless the run is
reported as a measurement and its output is quoted.

Scope: the path from a **scene name** (e.g. `bamboo`, `garden_wall`) to files on disk, and the
dials a new vegetation scene can turn. Nothing about the renderer beyond what that path touches.

---

## 0. The one-line answer

```
cargo run -p whiting-device-rasterize --example render -- <scene> <out> [start [stop [step]]] [flags]
```

Verified against `crates/device-rasterize/examples/render.rs:13-23` (the doc comment) and
`crates/device-rasterize/examples/render.rs:770-882` (`parse_arguments`) and
`crates/device-rasterize/examples/render.rs:2120-2127` (`usage`).

Measured on this machine, one build and one render, both on 2026-10-18 (section 5):

* build of `whiting-device-rasterize --example render -j 3`, `CARGO_INCREMENTAL=0`, warm shared
  target at `C:\resource\whiting\target`: **14.5 s** (10 crates recompiled).
* one render of `bamboo` (1200×1400, 13 passes): **5.11 s** wall clock for the whole process,
  of which the harness's own accounting is **compiled 94 ms, rendered 790 ms** — the remaining
  ~4.2 s is process start plus wgpu adapter/device creation, which the harness does not time.

---

## 1. The command line

### 1.1 Argument grammar

`parse_arguments` is at `crates/device-rasterize/examples/render.rs:771-882`. The rules, in order:

| position | meaning | default |
| --- | --- | --- |
| 1st | **scene name**, resolved through `Gallery::build` (`render.rs:500`). Missing ⇒ `usage()`. | required |
| 2nd | **output stem** — a path *without* an extension. Missing ⇒ `"out"`. | `out` |
| 3rd+ | bare numbers are **times**; everything else is a flag (below) | — |

`render.rs:845-870` turns the time list into moments:

* `[]` (no numbers) ⇒ **one frame at the scene's own `time`** (`f64::NAN` sentinel, `render.rs:848`;
  `set_time` writes under the field named `time`, `render.rs:101`).
* `[only]` ⇒ one frame at that time.
* `[start, stop]` ⇒ **a frame every half second**, at least 2 (`render.rs:850-858`).
* `[start, stop, step]` ⇒ `floor((stop-start)/step)+1` frames; `step <= 0` is refused
  (`render.rs:859-868`).
* anything longer ⇒ `usage()` (`render.rs:869`).

### 1.2 Flags and options (all defaults)

Parsed in `render.rs:793-839`. **All of them are optional; none has a value-bearing default.**

| flag | value | effect | default |
| --- | --- | --- | --- |
| `--sheet` | — | contact sheet, tile = every `SHEET_STEP`-th pixel | off (`Mode::Frames`) |
| `--sheet-full` | — | same, step 1 (`FULL_STEP`) | off |
| `--video` | — | mp4 via `ffmpeg`, needs ≥ 2 moments | off |
| `--census` | — | print `Gallery::census` + `Gallery::bounds` and **return without compiling or drawing** (`render.rs:494-498`). The gpu adapter is already created by then (`render.rs:467`), so this is cheap, not free. | off |
| `--locate <x,y>` | one pixel, **repeats**, answered in the order asked | prints what that pixel is | none |
| `--region <x0,y0,x1,y1>` | one rect, **repeats** | prints the ids the rectangle covers, each with a pixel count and a name | none |
| `--where <id or name>` | **repeats** | prints how many pixels that id covers and the box | none |
| `--debug <channel[,channel]>` | comma-separated, **repeats** | extra channel images beside the frame | none |

* `SHEET_STEP = 4` (`render.rs:2024`) and `FULL_STEP = 1` (`render.rs:2027`), used at
  `render.rs:795-796`. A sheet tile is the frame divided by the step, deliberately not resized to
  fill a target (`render.rs:31-48`).
* Legal `--debug` channels are exactly `position`, `depth`, `grade` —
  `crates/scene/src/standard/debug.rs:103-125` (`DebugChannel::ALL`, `from_tag`). A fourth name is
  an error listing the three (`render.rs:820-835`).
* `--locate`/`--region` reject a negative or non-numeric coordinate by name
  (`render.rs:153-163`, `render.rs:171-186`); `--region` orders its corners into a rectangle
  (`render.rs:169-170`).
* **Positional numbers and flags may be interleaved** — the loop takes out flags and their values
  and treats what is left as times (`render.rs:777-780`).

### 1.3 Which files a run writes

`<out>` is the stem you pass. For `render bamboo out/wall` the four files are:

| file | what it is | where written |
| --- | --- | --- |
| `out/wall.png` | **the frame** — the pipeline's `present` image (`graphics::PRESENT_OUTPUT = "present"`, `crates/graphics/src/lib.rs:160`), encoded by `RenderedFrame::to_png` | `write_moments` → `Mode::Frames` branch, `render.rs:946-955`; name from `numbered`, `render.rs:1374-1384` |
| `out/wall-authored_id.png` | the **scene-declared id image**, normalized for viewing | `write_channels`, `render.rs:992-1004`; the `-authored_id` is `with_stem_suffix(path, image.name)`, `render.rs:998-999` + `render.rs:746-759` |
| `out/wall-authored_id.txt` | **the same image's float values at a 9×9 grid**, with a header written from the *compiled document's* declaration, not from the image | `render.rs:1006-1014`; the 9×9 grid is `PROBE_SIDE = 9` (`render.rs:103-108`) |
| `out/wall-manifest.json` | per frame: `scene`, `camera`, `time`, `document` hash, `image` hash, `passes`, `build` id | `manifest_path`, `render.rs:1460-1462`; written **last**, after every frame is on disk, `render.rs:678-686` |

Two extra rules change those names:

* **More than one camera** ⇒ the stem gains `-camera0`, `-camera1`, … (`render.rs:530-537`, and the
  comment at `render.rs:525-529` explaining why the suffix goes *before* the extension). A scene
  with one camera writes where it always wrote.
* **More than one moment** ⇒ the frame and every channel image are numbered `<stem>-000`,
  `<stem>-001`, … (`numbered`, `render.rs:1374-1384`). Single-moment runs are unnumbered.
  This is why the `out/` directory is full of `v22-…`, `m4-…` files: the stem was chosen with a
  prefix and the scene suffix in one go.

`--debug` adds one `png` + one `txt` per requested channel, named `<stem>-<channel>.png` /
`.txt` (`render.rs:998-1014`), and `authored_id` arrives by the *same* path — a scene **declares**
it (`building_identity::outputs_field`, `crates/gallery/src/building_identity.rs:72-80`, wired in
at `vegetation_picture.rs:1071`), so no flag is needed.

`--sheet` writes **one** png instead of the frames (`render.rs:958-977`); `--video` writes the
numbered frames **and** an mp4 (`render.rs:1318-1337`).

**What `<stem>` is derived from:** nothing in the program derives it. It is *verbatim* the second
positional argument, with its extension replaced. `out/wall` ⇒ stem `wall` ⇒ `wall.png`,
`wall-authored_id.png`, `wall-authored_id.txt`, `wall-manifest.json`. Any path is accepted;
parent directories are created (`write_png`, `render.rs:1392-1402`; `write_text`,
`render.rs:1293-1303`). This is why `out/` holds stems like `final-bamboo_crown` — those were
authored by hand to avoid collisions between runs of the same scene.

### 1.4 What the manifest says, and why it is worth reading

`read_manifest` is called **before the first frame is drawn** (`render.rs:538-542`) and compared
by `report_against` (`render.rs:622`, defined `render.rs:1547`). `build_id` is a SHA-256 of the
**running executable's bytes** (`render.rs:1662-1682`), so "the build did not take" is
distinguishable from "the documents did not change". Measured example, `out/recon-bamboo-manifest.json`:

```json
{ "format": 1, "outputs": [ { "scene": "bamboo", "camera": 0, "time": null,
  "document": "6a52b965…3bf", "image": "0c4f19df…3a8e", "passes": 13,
  "build": "24a1a435…346" } ] }
```

---

## 2. The scene registry

There are **two** registries, and a vegetation scene must appear in **both** — plus one it must
appear **only** in.

### 2.1 `Gallery::build` — the table that makes a name renderable

`crates/gallery/src/lib.rs:210-262`. An unknown name is an `unimplemented!` panic whose message
lists `Gallery::names()` (`lib.rs:255-260`). **This is the only table a scene needs for
`render <name> <out>` to work at all.**

### 2.2 `locate::NAMINGS` — the table that makes ids read as names

`crates/gallery/src/locate.rs:71-142`, read by `naming(scene)` (`locate.rs:145-147`). It holds
**14 rows today and every one of them is a vegetation scene** (`locate.rs:72-141`). The lookup
order is fixed and matters:

`locate::handle_in` (`locate.rs:168-176`) and `locate::locate` (`locate.rs:191-249`) ask, in this
order: `building_identity::locate_in` → `naming(scene)` → `identity::handle_of` (the landscape).

So **a vegetation scene must be in `NAMINGS`, not in `building_identity`**.
`crates/gallery/src/building_identity.rs:253-267` (`document_of`) holds **only** the ten *building*
scenes and returns `None` for anything else; a vegetation name added there would find no
`Building::document()` to rebuild. The harness says this out loud in its own invariants
(`crates/device-rasterize/examples/render.rs:272-307`): asking the naming registry before
`building_identity` printed `id 0, and this scene names nothing` for every id of every building.

### 2.3 `Gallery::names` — the contract a scene must **not** join by default

`crates/gallery/src/lib.rs:177-204`. `names()` is **not** the index of what `build` can build; its
own doc (`lib.rs:159-176`) says it is a *contract*: every name in it is swept by
`scene::tests::clustered_lights`, `scene::tests::wgsl_validation` and
`device-rasterize::tests::gallery_render`. `vegetation_picture.rs:16-19` states the convention
outright: the vegetation scenes and `sweep` are "in `Gallery::build` and **not** in
`Gallery::names` … rendered by name on request". Joining `names()` is optional and **costly** — see
section 6.

### 2.4 The exact registrations a vegetation scene needs

| # | file | entry | needed for |
| --- | --- | --- | --- |
| 1 | `crates/gallery/src/lib.rs`, in `Gallery::build`'s `match` (~line 212-225) | `"foo" => my_module::foo_picture(),` | **renderability** — without it, `render foo out/x` panics with `` `foo` is not one of the 24 scenes the suite covers `` |
| 2 | `crates/gallery/src/locate.rs`, in `NAMINGS` (~line 71-142) | `Naming { scene: my_module::FOO, resolve: my_module::foo_handle_of, note: "…" }` | the `authored_id` image being **labelled with element names**; without it `--locate` prints `this harness has no domain reading for foo` (`render.rs:353-359`) and `--region` prints `id N, and this scene names nothing` (`render.rs:304`) |
| 3 | nothing | — | `Gallery::names` **stays as it is** unless you also want the scene swept (see §6) |

**One asymmetry worth knowing before you write the check.** `--region` and `--locate` name a
vegetation element through `gallery::locate::handle_in` (`render.rs:296` for a row, `render.rs:335`
for a pixel), which is the `NAMINGS` row. But **`--where <name>` does not**: its name direction is
`gallery::building_identity::id_in` (`render.rs:403`), and that table is buildings-only
(`building_identity.rs:253-267`). So a `NAMINGS` row makes `--region` and `--locate` speak your
scene's handles and leaves `--where` answering `no element of this scene is called that` for every
name. `--where <numeric id>` still works. Verified by reading both call sites; not measured on a
vegetation frame, and it does not need to be — both call sites are unambiguous.

### 2.5 The template to copy — `garden_wall`, end to end

Scene name `garden_wall`, defined across three files:

```rust
// crates/gallery/src/vegetation_laws.rs:65
pub const GARDEN_WALL: &str = "garden_wall";

// crates/gallery/src/vegetation_laws.rs:112-131
pub fn garden_wall() -> Value {
    let mut aware = bamboo(11);
    aware = aware.with_fields(vec![Field::obstacle(wall_occupancy, 1.0)]);
    let plantings = [
        Planting::new(bamboo(11)).at(planted_at(WALL_PLACES[0].0, WALL_PLACES[0].1)),
        Planting::new(aware).at(planted_at(WALL_PLACES[1].0, WALL_PLACES[1].1)),
    ];
    plant_scene(&plantings, &WALL_FRAME, GARDEN_WALL, &[crate::vegetation_picture::Prop { .. }])
}

// crates/gallery/src/vegetation_laws.rs:134-136
pub fn garden_wall_handle_of(id: u32) -> Option<String> {
    planting_handle(&garden_wall_plantings(), id)
}
```

```rust
// crates/gallery/src/lib.rs:219
"garden_wall" => vegetation_laws::garden_wall(),

// crates/gallery/src/locate.rs:117-121
Naming {
    scene: crate::vegetation_laws::GARDEN_WALL,
    resolve: crate::vegetation_laws::garden_wall_handle_of,
    note: "the left planting has no field and the right one is the plant that knows the wall is there",
},
```

**The shape of a `Naming` row** (`locate.rs:50-58`): `scene: &'static str` (the constant, never a
string literal), `resolve: fn(u32) -> Option<String>` (a bare `fn` pointer, type `Resolver`,
`locate.rs:47`), `note: &'static str` — one clause, printed after the handle by `locate.rs:233`.

**Note the `*_plantings()` duplication.** Every law scene builds its `Vec<Planting>` twice — once
inside the `pub fn`, once in a private helper — because `handle_of` must rebuild the *same*
plantings to number the same ids (`vegetation_laws.rs:134-144`; the same pattern at
`aspiring` 165-174, `thinned` 211-225, `brushed` 243-273, `arching` 312-333, `panelled` 369-399,
`bands` 418-435; and in `vegetation_picture.rs:484-486` for `grove`). **A new scene that derives
its plantings from a shared helper (as `garden_wall` does) has no such duplication to write** —
that refactor is already in `garden_wall` and is the newer shape.

---

## 3. The scene names renderable today

The enumeration lives in **`crates/gallery/src/lib.rs:211-261`** — the `match` arms of
`Gallery::build`, one arm per line `lib.rs:212` … `lib.rs:250`. **39 names.**
`Gallery::names()` (`lib.rs:177-204`) is a *different, shorter* list of **24** and is **not** the
enumeration; the 15 names it omits are exactly the 14 vegetation scenes plus `sweep`.

Legend: **V** = vegetation.

| # | name | arm | group |
| --- | --- | --- | --- |
| 1 | `bamboo` | `lib.rs:212` | **V** `vegetation_picture::bamboo_picture()` |
| 2 | `bamboo_wind` | `lib.rs:213` | **V** |
| 3 | `bamboo_crown` | `lib.rs:214` | **V** |
| 4 | `bamboo_knots` | `lib.rs:215` | **V** |
| 5 | `grove` | `lib.rs:216` | **V** |
| 6 | `sculpted` | `lib.rs:217` | **V** |
| 7 | `segmentation` | `lib.rs:218` | **V** |
| 8 | `garden_wall` | `lib.rs:219` | **V** `vegetation_laws::garden_wall()` |
| 9 | `aspiring` | `lib.rs:220` | **V** |
| 10 | `thinned` | `lib.rs:221` | **V** |
| 11 | `brushed` | `lib.rs:222` | **V** |
| 12 | `arching` | `lib.rs:223` | **V** |
| 13 | `panelled` | `lib.rs:224` | **V** |
| 14 | `bands` | `lib.rs:225` | **V** |
| 15 | `dunes` | `lib.rs:226` | landscape |
| 16 | `foldyard` | `lib.rs:227` | landscape |
| 17 | `hermite` | `lib.rs:228` | surface demo |
| 18 | `hermite_massif` | `lib.rs:229` | surface demo |
| 19 | `hall` | `lib.rs:230` | building |
| 20 | `hall_large` | `lib.rs:231` | building |
| 21 | `house` | `lib.rs:232` | building |
| 22 | `identity` | `lib.rs:233` | building (the id reference scene) |
| 23 | `landscape` | `lib.rs:234` | mountain |
| 24 | `lanterns` | `lib.rs:235` | still life |
| 25 | `monoliths` | `lib.rs:236` | still life |
| 26 | `shed` | `lib.rs:237` | building |
| 27 | `stilt` | `lib.rs:238` | building |
| 28 | `terrace` | `lib.rs:239` | building |
| 29 | `wing` | `lib.rs:240` | building |
| 30 | `workshop` | `lib.rs:241` | building |
| 31 | `yard` | `lib.rs:242` | building |
| 32 | `spotlight` | `lib.rs:243` | lighting demo |
| 33 | `stilllife` | `lib.rs:244` | still life |
| 34 | `studio` | `lib.rs:245` | the compiler's own example scene |
| 35 | `sunlight` | `lib.rs:246` | lighting demo (directional sun) |
| 36 | `sweep` | `lib.rs:247` | shape/sweep demo — **in `build`, not in `names`** |
| 37 | `terraces` | `lib.rs:248` | landscape (rice terraces) |
| 38 | `tidewater` | `lib.rs:249` | landscape |
| 39 | `tiles` | `lib.rs:250` | texture demo |

**39 names**; **fourteen are vegetation**: rows 1–14.

The scene's own `name` field is also written into the document — `plant_scene_with` puts
`("name", Value::String(name))` at `vegetation_picture.rs:995` — so the string passed as the
third argument of `plant_scene` and the `match` arm must be the same one.

---

## 4. The camera and light dials

Everything below is in `crates/gallery/src/vegetation_picture.rs`. The one builder that assembles
a vegetation scene is `plant_scene_with` (`vegetation_picture.rs:742-1145`), reached through
`plant_scene` (`vegetation_picture.rs:673-680`).

### 4.1 `Framing` — every field, with units

Declared `vegetation_picture.rs:260-280`. It is `Copy` and carries **numbers only**, which is
what lets two scenes be one builder's two arguments (`vegetation_picture.rs:253-259`).

| field | type | unit | used at | meaning |
| --- | --- | --- | --- | --- |
| `width` | `i64` | **pixels** | `:999` (`frame.width`) | frame width |
| `height` | `i64` | **pixels** | `:1000` (`frame.height`) | frame height |
| `eye` | `[f64; 3]` | **metres**, world axes | `:1006` → `kit::camera_placed` | camera position |
| `target` | `[f64; 3]` | **metres** | `:1007` | what the camera looks at (aim point, not a direction) |
| `field_of_view` | `f64` | **radians, vertical** | `:1008` | vertical FOV |
| `near` | `f64` | **metres** | `:1009` | near plane; must be nearer than the nearest leaf (`:272`) |
| `far` | `f64` | **metres** | `:1010` | far plane; must reach past the floor's corner (`:274`) |
| `floor` | `f64` | **metres** | `:1114`, `:1116` | half-extent of the ground from the origin |
| `ground` | `Ground` | — | `:1113-1118` | plane at zero, or a heightfield law |

Existing framings, to copy from:

* `STILL` (`vegetation_picture.rs:345-355`) — the four one-plant scenes; 1200×1400,
  eye `[3.40, 1.55, 3.80]`, target `[0.16, 1.10, 0.05]`, FOV 0.66 rad, near 0.05, far 24.0,
  floor 6.0, `Ground::Plane`.
* `GROVE_FRAMING` (`vegetation_picture.rs:358-371`) — 1400×1000, eye `[2.55, 1.85, 3.35]`, target
  `[0.05, 1.00, 0.00]`, FOV 0.75, near 0.05, far 32.0, floor 7.0,
  `Ground::Fielded { height: swell, divisions: 56 }`.
* `PAIR` (`vegetation_laws.rs:36-46`) — every law demo's frame; 1400×1000, eye `[1.95, 1.30, 3.05]`,
  target `[0.0, 0.80, 0.0]`, FOV 0.80, near 0.05, far 32.0, floor 7.0, `Ground::Plane`.
* `WALL_FRAME` (`vegetation_laws.rs:53-63`), `SHRUB_FRAME` (`vegetation_laws.rs:187-197`),
  `PANEL_FRAME` (`vegetation_laws.rs:342-352`).

### 4.2 `Ground`

`vegetation_picture.rs:289-300`. Two variants:

* `Ground::Plane` — a plane at `y = 0`, built by `geometry::plane_facing(framing.floor, 0.0, true)`
  (`vegetation_picture.rs:1114`).
* `Ground::Fielded { height: fn(f64, f64) -> f64, divisions: usize }` — a heightfield built by
  `geometry::heightfield(divisions, framing.floor, height)` (`vegetation_picture.rs:1116`).
  `height` takes a plan position `(x, z)` in metres and returns metres.

**Yes — a vegetation scene can ask for a terrain instead of a plane.** `grove` already does:
`Ground::Fielded { height: swell, divisions: 56 }` (`vegetation_picture.rs:367-370`) with
`fn swell(x: f64, z: f64) -> f64` (`vegetation_picture.rs:417-419`).

**The one ground per scene is a hard rule, not advice.** The floor mesh is drawn from
`framing.ground` *and* every planting is seated on the same function through
`vegetation::ground::seat(placement, seated_swell)` — see `vegetation_picture.rs:451` and
`:462-464`, with the invariant stated at `:411-416`: *"the same function draws the floor's
heightfield and seats every planting, so a plant cannot stand at a height the floor does not have.
A second function would be a second ground."*

### 4.3 `Planting` — every field, with units

Declared `vegetation_picture.rs:309-316`; constructors `new` / `at` / `with_seats` at
`:318-338`.

| field | type | unit | meaning |
| --- | --- | --- | --- |
| `species` | `vegetation::derive::Species` | — | the plant: its skeleton, its fields, its leaf templates (`crates/vegetation/src/derive.rs:39-43`) |
| `placement` | `vegetation::plant::Placement` | **metres** (origin), **radians** (turn) | where the plant stands; a full frame `x`/`y`/`z` basis |
| `seats` | `Vec<vegetation::decorator::Seat>` | metres / ratios | decorators (knots), in the plant's **own** coordinates (`:314`) |

**`planted_at(position, turn)`** (`vegetation_picture.rs:401-409`, `pub(crate)`): a frame standing
at a place and turned about the world's `y`. `position` is `[f32; 3]` in metres, `turn` is radians.
Its doc (`:398-400`) states the rule it encodes: **a plant may be turned, a decorator may not**,
because a decorator has to sit on a surface that already has a frame.

**`planting_handle(plantings, id)`** (`vegetation_picture.rs:496-509`, `pub(crate)`): the resolver
behind every law scene's `*_handle_of`. **The id arithmetic is a division**: planting index =
`id / PLANTING_STRIDE`, node = `id % PLANTING_STRIDE`, where
`PLANTING_STRIDE = 4096` (`vegetation_picture.rs:342`). Invariant at `:490-495`: a planting with
**no rows at all still costs its stride**, so removing a plant never renumbers the ones after it.
The answer format is `"planting {index}, {handle}"` (`:508`).

`plant_scene(plantings, framing, name, props)` (`:673-680`) is the public-facing entry; it is
`pub(crate)`, so a new scene must live **inside the `gallery` crate** (it does — both
`vegetation_picture.rs` and `vegetation_laws.rs` are gallery modules).

### 4.4 `Prop` — world geometry with no identity

`vegetation_picture.rs:691-703`.

| field | type | unit |
| --- | --- | --- |
| `name` | `String` | — (tags its three materials and its mesh) |
| `points` | `definition::PackedList` | metres, in the prop's own coordinates |
| `tint` | `[f64; 3]` | linear colour |
| `roughness` | `f64` | 0..1 |
| `at` | `[f64; 3]` | metres |

Two invariants worth copying verbatim (`:686-690`): **a prop carries no `authored_id`**, so its
pixels are the reserved zero and `--region` calls them the ground's; and **a prop draws on the
ground's three stages** (`vegetation_picture.rs:907-914`), "so a prop cannot be a plant by
accident and a plant cannot be a prop". `garden_wall`'s wall is the one existing example
(`vegetation_laws.rs:123-129`).

### 4.5 Sun, sky, air/fog, ground — what a vegetation scene reaches **today**

**Everything in the table below is a module-level `const` or a private `fn` in
`vegetation_picture.rs`. None of it is a parameter of `plant_scene`.** That is the single most
important fact about this section: `Framing` dials the camera and the floor extent; the *light and
the sky* are one builder's constants.

| dial | the value today | where | reachable by a new scene? |
| --- | --- | --- | --- |
| key light position / colour / irradiance / reach / extent | `[2.6, 3.9, 2.0]`, `[1.0, 0.96, 0.86]`, `3.0`, `7.0`, `3.0` | `vegetation_picture.rs:111-115`, used `:1030-1037` | **No** — `const`s of the module. Editable only by editing the shared constants, which changes every existing vegetation scene. |
| fill light position / colour | `[-2.6, 1.5, -1.8]`, `[0.44, 0.54, 0.76]` at irradiance 0.7 | `:119-120`, used `:1038-1044` | **No** — same. |
| ambient / environment light radiance | `SKY_RADIANCE = 1.0` | `:151`, used `:1047` | **No** — same. |
| sky / atmosphere | `fn atmosphere()` — zenith `[0.16, 0.28, 0.50]`, horizon `[0.62, 0.68, 0.72]`, ground `[0.38, 0.37, 0.34]`, `density 0.0012`, `scale_height 60.0`; **no `.sun()`, no `.disc()`** | `:131-140` | **No** — a private `fn`, not a `Framing` field. |
| exposure / grade | `const GRADE: &str` — Reinhard + gamma 0.4545, sky from `environment_sky`, a 0.012 air term capped at 0.45 | `:157-172`, used `:1061-1066` | **Yes, partially** — `plant_scene_with` takes a `View`, and `View::grade` **replaces** the post body wholesale (`:719-720`, `:1061-1065`). `segmentation_picture` uses it (`:605-618`, `:627-632`). `View` is private to the module though (`vegetation_picture.rs:711-712`). |
| plant surface body / ground surface body / shader helpers | `View { plant, ground, helpers, grade }` | `:711-721` | **Same limitation** — splicable, but `View` is private. |
| ground extent & ground law | `Framing.floor`, `Framing.ground` | `:277-279` | **Yes** — the only environment dial that is a `Framing` field. |
| camera | all of `Framing` | §4.1 | **Yes** |
| geometry resolution | `Detail::DEFAULT` = `{ stations: 6, around: 12 }` | `vegetation_picture.rs:178-180`; `crates/vegetation/src/expand.rs:40-43` | **No** — a fixed call, by design (`:175-177`: "a scene that picked its own dial would be a second author of the geometry"). `Detail::scaled(factor)` (`expand.rs:46-51`) exists if you want to change that call. |

**So the answers to the four questions asked:**

* **A different time of day?** **Not today, for a vegetation scene.** The sky is
  `vegetation_picture::atmosphere()`, a fixed thin-day haze with no sun disc
  (`vegetation_picture.rs:131-140`), and the key light is a point light at a fixed position. The
  machinery for a real time of day exists in the crate — `air::Atmosphere::sun(direction, glow,
  reach, strength)` and `.disc(colour, sharpness)` (`crates/gallery/src/air.rs:135-165`), and
  `dawn::sky(sun)` (`crates/gallery/src/dawn.rs:77-100`, `Sky` at `:40-70`) with a two-lobe glow
  and a sun-following horizon — but neither is reachable from `plant_scene_with`: `air` is a
  **public** module (`lib.rs:66`) whose builders simply have no call site in
  `plant_scene_with`, and `dawn` is a **private** module of the crate (`lib.rs:88`, `mod dawn;`)
  so nothing outside it can even name `dawn::sky`. A vegetation scene wanting dawn must either
  edit `atmosphere()` for everyone or build its own document.
* **A terrain instead of a plane?** **Yes** — `Ground::Fielded`. `grove` does it.
* **A backdrop?** **Not today.** `crates/gallery/src/ridges.rs` exports `pub const RIDGES: &str`
  (`ridges.rs:63`) — the WGSL post-stage helper that paints far ranges — and `terraces.rs` calls it
  (`terraces.rs:38`). A vegetation scene cannot: its post stage is the fixed `GRADE` string and the
  `View::grade` escape hatch is private to `vegetation_picture`. The other backdrop builder,
  `hermite_massif::backdrop(eye, target, radius)`
  (`crates/gallery/src/hermite_massif.rs:87`), is a private `fn` and so is not callable either —
  though its return type, `definition::PackedList`, is exactly `Prop::points`'s type
  (`vegetation_picture.rs:695`), so a backdrop built that way **would** drop in as a prop.
* **Aerial perspective?** **Partly, and it is the weakest part.** The air a vegetation scene gets
  is `Atmosphere::density(0.0012)` + `scale_height(60.0)` (`vegetation_picture.rs:136-139`) fed
  into the lighting helpers as WGSL (`:821-825`) **and** baked to a 512×256 latlong map
  (`:1018-1026`). But the *grade* — the thing that actually mixes geometry towards the air on
  screen — uses its own hard-coded `1.0 - exp(-along_ray * 0.012)` clamped at 0.45
  (`vegetation_picture.rs:166-169`), a flat distance law with no height term. The height-aware law
  a vegetation scene would want is `terraces::FOG_WGSL` (`crates/gallery/src/terraces.rs:1005`,
  `fn fog(color, ray, world)` at `:1014`), which integrates an exponential density to a wandering
  ceiling and scatters towards the sun. That law is scene-local — a private `const` in a private
  module (`lib.rs:116`, `mod terraces;`) — and neither `Atmosphere` nor `Framing` offers a slot
  for it.

**Summary of reachability, one line:** a new vegetation scene can dial **the camera, the frame
size, the floor extent, the ground law (plane or heightfield), which plants, where they stand, how
far they turn, what decorators they carry, and what props stand among them** — and nothing else.
Sun, sky, air, fog and exposure are shared module-level constants.

`land.rs` (`crate::land`, `crates/gallery/src/land.rs`) holds terrain *laws* — `Ridge`, `Valley`,
`bench` `:171`, `terrace` `:184`, `slope` `:202`, `crest_profile` `:56`, `edge_taper` `:332`,
`quarter` `:349`. They are `pub` and terrain-independent, so **a vegetation scene can use them as
its `Ground::Fielded.height` today**; that is a function, not a dial.
`ground::mesh(building, divisions, extent, height)` (`crates/gallery/src/ground.rs:49-57`) is
**not** reachable: it takes a `building::Building` and cuts that building's ground plates out of
the grid — a vegetation scene has no `Building`.

---

## 5. The cost, MEASURED

**One build, one render. Both on 2026-10-18, on this machine, with other sessions sharing it.**

### 5.1 Build

```
$env:CARGO_INCREMENTAL="0"
cargo build -p whiting-device-rasterize --example render -j 3      # workdir C:\resource\whiting
```

```
   Compiling whiting-definition v0.1.0
   Compiling whiting-terrain v0.1.0
   Compiling whiting-vegetation v0.1.0
   Compiling whiting-building v0.1.0
   Compiling whiting-patch v0.1.0
   Compiling whiting-graphics v0.1.0
   Compiling whiting-scene v0.1.0
   Compiling whiting-device v0.1.0
   Compiling whiting-gallery v0.1.0
   Compiling whiting-device-rasterize v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 14.50s
BUILD_EXIT=0 BUILD_SECONDS=14.7
```

**Build: 14.5 s** (cargo's own figure), targeting the existing shared
`C:\resource\whiting\target`, `-j 3`, `CARGO_INCREMENTAL=0`.

**Caveat, and it is a large one:** this is a **warm** target — ten crates recompiled, not a cold
build. The tree's own guidance (`AGENTS.md`) records that 24 s / 92 s first measurements on this
machine were dominated by other sessions holding the build lock, and `Blocking waiting for file
lock` was not seen here because the lock was free. **A cold build from an empty target would take
minutes; do not plan around 14.5 s.** Conversely, once the target is warm, a one-line source edit
in `gallery` recompiles one crate and relinks the example — that is the cost you will pay per
scene, not 14.5 s.

### 5.2 Render

Run as the **built executable directly**, so the number is the render and not cargo:

```
C:\resource\whiting\target\debug\examples\render.exe bamboo C:\resource\whiting\out\recon-bamboo
```

```
manifest: C:\resource\whiting\out\recon-bamboo-manifest.json holds no previous render; this run writes the first one
bamboo: the scene's own time drawn, 13 passes, 0 operations of delta, cloned 1 ms, validated 1 ms, compiled 94 ms, rendered 790 ms
wrote C:\resource\whiting\out\recon-bamboo-authored_id.png (the scene's own time)
wrote C:\resource\whiting\out\recon-bamboo-authored_id.txt (the scene's own time)
wrote C:\resource\whiting\out\recon-bamboo.png (the scene's own time)
RENDER_EXIT=0 RENDER_SECONDS=5.11
```

Broken out, using the harness's own accounting (`render.rs:647-661`):

| stage | time |
| --- | --- |
| `Gallery::build` + clone | **1 ms** (`cloned 1 ms`, `render.rs:549`) |
| schema validation | **1 ms** (`render.rs:560`) |
| **compile** (scene document → graphics + instance) | **94 ms** (`render.rs:576`) |
| **render** (GPU draws + readback) | **790 ms** (`render.rs:660`) |
| whole process wall clock | **5.11 s** |

The gap — roughly **4.2 s** — is process start plus `whiting_device_rasterize::rasterizer()`
(`render.rs:467`), i.e. wgpu instance, adapter and device creation. The harness does not time it.
**Budget ~5 s per scene, not 1 s.** Writing the three files (PNG encode + the 9×9 float sidecar)
happens after the timing line and is inside the 5.11 s but outside the 790 ms.

### 5.3 What was produced

| file | bytes | dimensions |
| --- | --- | --- |
| `out/recon-bamboo.png` | **203,938** | **1200 × 1400** |
| `out/recon-bamboo-authored_id.png` | **53,336** | **1200 × 1400** |
| `out/recon-bamboo-authored_id.txt` | **4,419** | 9×9 sample grid + header |
| `out/recon-bamboo-manifest.json` | **384** | — |

1200×1400 is `Framing.width × Framing.height` of `STILL`
(`vegetation_picture.rs:81-82`), i.e. the PNG size **is** the `Framing` you wrote — the frame field
of the document (`vegetation_picture.rs:997-1002`) and nothing else.

The sidecar's first line is `authored_id: 1200x1400, r32float` and its header states, from the
compiled document, that this is a *scene-declared output* covering "exactly what the writing pass
enumerated" rather than a built-in channel — which is `authored_id` being declared at
`building_identity.rs:72-80` and read back by `render.rs:590` / `:1006-1014`.

### 5.4 Cost per scene, projected

`bamboo` is the *smallest* vegetation scene: one planting, 13 passes. `grove` (`out/v10-grove.png`
is 675,852 bytes, ~3.3× `bamboo`) carries **six named plantings plus 140 scattered tufts**
(`vegetation_picture.rs:459-475`, `GROVE_TUFTS = (14, 10)` at `:426`). Expect **roughly 3× the
render time and a visibly larger compile** for a grove-class scene. The 5.11 s figure is a floor,
not a typical.

---

## 6. Golden hashes and pinned-identity tests

**There is no `.lichen` file anywhere in the repository** — `Get-ChildItem -Recurse -Filter *.lichen`
over `C:\resource\whiting` returns **0**. The `.agents/proposals/AGENTS.md` convention that
references `.lichen` has no instance here.

**`out/` is git-ignored** (`.gitignore:2`), so `out/scene_identity_probe.txt` is **regenerable
output, not a pinned golden file**. It holds the last run of the example (`crates/gallery/examples/
scene_identity_probe.rs`, run as `cargo run -p whiting-gallery --example scene_identity_probe`,
per its own doc at `:42`) and nothing reads it — a grep for `scene_identity_probe` across `*.rs`
finds only the example itself and two passing mentions in a comment
(`crates/scene/tests/clustered_lights.rs:368`, `:535`).

### 6.1 `crates/gallery/examples/scene_identity_probe.rs` — **not** affected by a new scene

Its `SCENES` array (`:48-59`) is a **fixed list of the ten *building* scenes**, and every
assertion it makes goes through `building_identity::locate_in` / `id_in` (`:87`, `:98`), which
refuses any name `document_of` does not hold (`building_identity.rs:253-267`). It also prints its
own verdict rather than failing the process (`:142`). **Adding a vegetation scene does not touch
it.**

### 6.2 The one test a new scene **can** break

**`crates/scene/tests/clustered_lights.rs::every_gallery_scene_still_emits_the_pass_count_it_emitted_before`**
(`:224-685`).

It builds its `counted` list by walking **`Gallery::names()`** (`:248-251`) and compares it with
`assert_eq!` against a **hand-written `expected` table** (`:252-675`, assertion at `:676-684`). Two
consequences:

* **Adding a name to `Gallery::names()` without adding a row to `expected` makes this test red** —
  and it is red on *length*, not on a number, which is the failure mode the gallery's own doc warns
  about at `crates/gallery/src/lib.rs:165-170`.
* **Adding a scene only to `Gallery::build` does not touch it**, because `names()` does not walk it.

Its doc (`:229-247`) records that this exact drift happened twice — `tidewater`, then `workshop`
and `yard`.

### 6.3 The other two suites that follow `Gallery::names()`

Both are *pass* over a new name, not *pin* over it, so a new scene in `names()` makes them **cover
more**, never fail for being new:

* `crates/scene/tests/wgsl_validation.rs:72-97` — parses and validates every shader of every
  `names()` scene with `naga`. **This is the gate that catches a broken WGSL body in a new scene.**
* `crates/device-rasterize/tests/gallery_render.rs:58-98` (`every_gallery_scene_renders`) —
  compiles and renders every `names()` scene, collecting **all** failures into one panic
  (`:83-97`). It is skipped with a printed message when no adapter is present (`:26`, `:60-63`).

### 6.4 Verdict, plainly

> **Adding a vegetation scene named `foo` to `Gallery::build` + `locate::NAMINGS` breaks nothing.**
> **Adding it to `Gallery::names()` as well breaks exactly one test —
> `scene::tests::clustered_lights::every_gallery_scene_still_emits_the_pass_count_it_emitted_before`
> — and only until a matching row is added to its `expected` table.** No test pins image hashes, no
> test reads `out/`, and there are no `.lichen` files.

**Recommendation, from the existing convention:** follow the 14 vegetation scenes and stay **out**
of `Gallery::names()`. `vegetation_picture.rs:16-19` and `sweep_picture.rs:14` both say so
explicitly, and the reason is on record — `names()` is a contract the suites hold themselves to,
and a scene nobody sweeps is cheaper than a scene that owes a pass-count row.

---

## 7. Recipe — adding one new vegetation scene named `foo`

These edits are in order. **Nothing else needs to change** to render `foo`. Steps 1–4 are the
whole job; step 5 is optional and carries the one test cost.

### Step 0 — decide where it lives

Either **add to `crates/gallery/src/vegetation_laws.rs`** (if `foo` is a law demo, i.e. a pair of
plants) **or create `crates/gallery/src/vegetation_foo.rs`**. A new file needs one extra line
(step 4).

> `plant_scene`, `planted_at` and `planting_handle` are `pub(crate)` (`vegetation_picture.rs:673`,
> `:401`, `:496`) — they are usable **only inside the `gallery` crate**. A scene built outside
> `crates/gallery` cannot use them, and would have to write its own document.

### Step 1 — the scene module

In your file (here: `crates/gallery/src/vegetation_foo.rs`), four things, in this shape:

```rust
//! what the frame is for                       // module doc; one line + the claim

use definition::Value;
use vegetation::derive::{Species, bamboo};     // or clump / shrub / tuft / sculpted

use crate::vegetation_picture::{
    Framing, Ground, Planting, plant_scene, planted_at, planting_handle,
};

/// The name the scene is built and rendered under. **A `pub const`, never a string literal at
/// the call site** — `locate::NAMINGS` keys on this constant.
pub const FOO: &str = "foo";

/// The framing, as a `const`. Metres everywhere except width/height (pixels) and fov (radians).
const FOO_FRAME: Framing = Framing {
    width: 1400,
    height: 1000,
    eye: [1.95, 1.30, 3.05],       // metres, world axes
    target: [0.0, 0.80, 0.0],     // metres
    field_of_view: 0.80,          // radians, vertical
    near: 0.05,                   // metres — nearer than the nearest leaf
    far: 32.0,                    // metres — past the floor's corner
    floor: 7.0,                   // metres, half-extent from the origin
    ground: Ground::Plane,        // or Ground::Fielded { height: fn(f64,f64)->f64, divisions: usize }
};

/// Builds the plantings. **Factor this out** — `garden_wall` does (`vegetation_laws.rs:138-144`)
/// and it is what keeps `foo_handle_of` from duplicating the species construction.
fn foo_plantings() -> Vec<Planting> {
    vec![
        Planting::new(bamboo(11)).at(planted_at([-0.62, 0.0, 0.0], 0.0)),
        // …more. `turn` is radians about the world's y; `position` is metres.
    ]
}

/// Builds the scene.
pub fn foo() -> Value {
    plant_scene(&foo_plantings(), &FOO_FRAME, FOO, &[])   // 4th arg: `&[Prop]`
}

/// Resolves one `authored_id` into a handle. Signature is `gallery::locate::Resolver`:
/// `fn(u32) -> Option<String>`.
pub fn foo_handle_of(id: u32) -> Option<String> {
    planting_handle(&foo_plantings(), id)
}
```

If the scene draws **one** plant, use `vegetation_picture::handle_of`'s pattern instead — a table
from `node_ids(&species.plant)` (`vegetation_picture.rs:222-228`) — because the planting quotient is
then always 0. If the plant is **edited** (as `sculpted` is), the resolver must rebuild *that*
plant, not the base one: `vegetation_picture.rs:647-660` states the reason (a graft added a node,
so every id after it is one higher).

### Step 2 — the renderability table (**mandatory**)

`crates/gallery/src/lib.rs`, inside `Gallery::build`'s `match` (the vegetation arms are
`lib.rs:212-225`):

```rust
            "foo" => vegetation_foo::foo(),          // match the module you wrote it in
```

Add it in the vegetation block. Without this, `render foo out/foo` panics:
``error: `foo` is not one of the 24 scenes the suite covers: …`` (`lib.rs:255-260`).

### Step 3 — the naming table (**mandatory** for named ids)

`crates/gallery/src/locate.rs`, inside `pub const NAMINGS: &[Naming]` (the rows are
`locate.rs:71-142`):

```rust
    Naming {
        scene: crate::vegetation_foo::FOO,          // the constant, not the literal
        resolve: crate::vegetation_foo::foo_handle_of,
        note: "one clause saying what an authored_id means here",
    },
```

Without this row the frame still renders and the `authored_id` image is still written — but
`--locate`, `--region` and `--where` fall through to the landscape's `identity::locate`
(`locate.rs:243-248`) and answer for a scene it does not know.

**Do not** add `foo` to `building_identity::document_of` (`building_identity.rs:253-267`). That
table is buildings only; it rebuilds a `Building::document()` and there is none for a plant. The
lookup order at `locate.rs:168-176` tries `building_identity` first and it returns `None` for you,
which is correct.

### Step 4 (only for a **new file**) — declare the module

`crates/gallery/src/lib.rs`, in the module list (`lib.rs:66-131`, alongside
`pub mod vegetation_laws;` at `:122`):

```rust
/// The scene's one line: what it is for.
pub mod vegetation_foo;
```

### Step 5 — optional: join `Gallery::names()`

Only if you want the suites to sweep `foo`. If you do, **you must add a row to the `expected`
table in `crates/scene/tests/clustered_lights.rs:252-675`** in the same edit, or
`every_gallery_scene_still_emits_the_pass_count_it_emitted_before` goes red on length. Read the
row's `Stages { shadowing, depthing, lighting, other }` off the run's own `N passes` line and
`--census` output. **The 14 existing vegetation scenes are not in `names()`; the cheaper and
conventional choice is to skip this step.**

### Step 6 — render

```
cargo run -p whiting-device-rasterize --example render -- foo C:\resource\whiting\out\recon-foo
```

Cheap registration check first — `--census` prints the scene's list counts, instance buckets and
placement bounds and returns **before the scene is compiled or drawn** (`render.rs:494-498`); the
gpu adapter is still created at `render.rs:467`, so it costs a couple of seconds rather than
minutes:

```
cargo run -p whiting-device-rasterize --example render -- foo C:\resource\whiting\out\recon-foo --census
```

Then sanity-check the ids are **named** rather than bare integers — the thing step 3 bought.
**Use `--region` or `--locate`, not `--where <name>`:**

```
... render -- foo C:\resource\whiting\out\recon-foo --region 400,400,900,900
... render -- foo C:\resource\whiting\out\recon-foo --locate 640,700
... render -- foo C:\resource\whiting\out\recon-foo --where 4097      # numeric id only
```

`--region` prints each id under the rectangle with its pixel count and the handle
(`render.rs:211-270`), naming it through `gallery::locate::handle_in` (`render.rs:296`) — which is
`naming(scene)`, so **this is the path that reads a `NAMINGS` row**. `--where` takes an id or a name
(`render.rs:390-408`), but its **name** direction goes through
`gallery::building_identity::id_in` (`render.rs:403`), and that table refuses every vegetation scene
(`building_identity.rs:253-267`). **`--where <name>` answers `no element of this scene is called
that` on a plant scene; only `--where <number>` works there.** That asymmetry is the harness's, not
the gallery's, and it is why `--region` is the check to run.

### Step 7 — what you will have written

`out/recon-foo.png`, `out/recon-foo-authored_id.png`, `out/recon-foo-authored_id.txt`,
`out/recon-foo-manifest.json` — exactly the four of §1.3, at
`Framing.width × Framing.height`, in about **5 s** after one warm incremental build.

### Copyable checklist

1. `crates/gallery/src/vegetation_foo.rs` — `pub const FOO`, `pub fn foo()`,
   `pub fn foo_handle_of(u32) -> Option<String>`, a private `foo_plantings()`.
2. `crates/gallery/src/lib.rs` — `pub mod vegetation_foo;` **and** `"foo" => vegetation_foo::foo(),`
   in `Gallery::build`.
3. `crates/gallery/src/locate.rs` — one `Naming { scene, resolve, note }` row in `NAMINGS`.
4. `Gallery::names()` — **leave alone** unless you also add the `clustered_lights.rs` row.

---

## Appendix — the files this note is read out of

Read on 2026-10-18. Line numbers are of the checkout as it stood then.

| file | lines | what it answered |
| --- | --- | --- |
| `crates/device-rasterize/examples/render.rs` | 2389 | §1 entirely; `--where`'s name direction (`:403`); `name_for_row` (`:290-306`) |
| `crates/gallery/src/lib.rs` | 446 | §2.1, §2.3, §3 (`Gallery::build`, `Gallery::names`, `LABELS`) |
| `crates/gallery/src/locate.rs` | 249 | §2.2 (`NAMINGS`, `naming`, `handle_in`, `locate`) |
| `crates/gallery/src/building_identity.rs` | 296 | §2.2 (`document_of`, `outputs_field`, `building_extras`) |
| `crates/gallery/src/vegetation_picture.rs` | 1145 | §4 entirely (`Framing`, `Ground`, `Planting`, `Prop`, `plant_scene_with`, the lights, the sky, the grade) |
| `crates/gallery/src/vegetation_laws.rs` | 436 | §2.5, §7 step 1 (the law-scene template) |
| `crates/gallery/src/vegetation_mesh.rs` | — | `LEAF_BASE_ID = 1`, `node_ids_from` (the id arithmetic §4.3 relies on) |
| `crates/gallery/src/air.rs` | 391 | §4.5 (`Atmosphere`, its defaults, `sun`/`disc`) |
| `crates/gallery/src/dawn.rs` | 252 | §4.5 (`Sky`, `sky(sun)`) |
| `crates/gallery/src/sunlight.rs` | 290 | §4.5 (the directional-light scene; no vegetation path to it) |
| `crates/gallery/src/ridges.rs` | 152 | §4.5 (`RIDGES`, the painted backdrop) |
| `crates/gallery/src/terraces.rs` | 1105 | §4.5 (`FOG_WGSL`, the height-aware aerial perspective) |
| `crates/gallery/src/land.rs` | 352 | §4.5 (terrain laws a vegetation ground could use) |
| `crates/gallery/src/ground.rs` | 295 | §4.5 (`ground::mesh`, and why a plant cannot reach it) |
| `crates/gallery/src/kit.rs` | 773 | §4.5 (`light`, `light_with_radius`, `environment_light`, `camera_placed`) |
| `crates/scene/src/standard/debug.rs` | 856 | §1.2 (`DebugChannel::ALL`, the three legal `--debug` tags) |
| `crates/scene/tests/clustered_lights.rs` | 1155 | §6.2 (the one test a new `names()` row breaks) |
| `crates/scene/tests/wgsl_validation.rs` | 212 | §6.3 |
| `crates/device-rasterize/tests/gallery_render.rs` | 148 | §6.3 |
| `crates/gallery/examples/scene_identity_probe.rs` | 143 | §6.1 |
| `crates/vegetation/src/expand.rs` | 112 | §4.5 (`Detail::DEFAULT`, `Detail::scaled`) |
| `crates/vegetation/src/derive.rs` | — | §7 step 1 (`bamboo`, `clump`, `shrub`, `tuft`, `sculpted`) |
| `.gitignore` | — | §6 (`/out` and `/target` are ignored) |


