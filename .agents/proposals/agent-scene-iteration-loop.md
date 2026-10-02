# The scene iteration loop: what three agents collided with, and what the framework could have said

Status: **items 1 and 4 implemented.** The render harness now writes a manifest beside the
images and says when a change did not reach the frame, and a contact sheet's tiles can be the
frames' own pixels. Both contracts are recorded under *The render manifest* and *The contact
sheet* in [SPEC.align.md](../SPEC.align.md) and live in `crates/device/examples/render.rs`. The
full-resolution sheet kept the quarter-sample default and added `--sheet-full` rather than changing
what every existing invocation writes; the reasoning, and why scaling the whole sheet to a target
size was rejected, are in handoffs `render-manifest.md` and `contact-sheet.md`. This file is not a
design of any one feature. It is a cross-reading of three independent runs of the same task
against this framework, and a ranking of what the framework could have given them instead.

## The evidence

Three models were each given the same brief — build a bamboo courtyard as a 3D landscape in a
fresh worktree, change the scene compiler if you need to, deliver six renders, be judged on
aesthetics. They ran in separate worktrees with separate sessions and never saw each other.

| | DeepSeek | MiniMax-M3.1 | Kimi-K3 |
| --- | --- | --- | --- |
| worktree | `worktrees/dsf` | `worktrees/m3.1` | `.worktrees/k3` |
| steps / edits | 428 / 132 | 315 / 155 | 190 / 100 |
| scene file | `courtyard.rs` 38 KB, one `write` | `bamboo.rs` 65 KB, 186 touches | `bamboo.rs` 68 KB, 97 steps |
| images looked at | 30, all full size | 22 full size + **12 contact sheet** + 6 crops | 50, all downsampled |
| what broke the run | nothing structural | **context compaction, 365.9k → 29.7k** | **403 quota wall, 1h56m human gap** |
| finals matching something it had seen | **4/6 byte-identical** | **0/6, best SSIM 0.690** | 3/6 exact, 3/6 never re-looked |
| framework-level findings | none | none | **one, real** |

The session records are in `c:/resource/film-lang/.worktrees/{dsh,k3,m31}-session/` — a staged
analysis per phase plus a distilled transcript. Every number below is traceable to those files.

Two facts from the table drive most of this document. **The same failure appears in all three
runs** — *I changed something and the picture did not move* — and it is the single most expensive
thing that happened to any of them. And **all three shipped at least one image they had never
inspected**, which none of them reported.

---

## 1. The change is not reaching the frame, and the framework cannot say so

**Priority: highest. Cost: small. The machinery already exists.**

All three models hit this. The outcomes were wildly different, and the difference is entirely down
to whether the model thought to check the file timestamp.

- **dsf** spent close to ten minutes on it. A PowerShell batch replace had written a `MISS:`
  prompt line into the head of `courtyard.rs`, the build failed, and **the old binary kept
  rendering with the old arguments** while the model believed it was iterating.
- **m3.1** changed three separate things and got `IDENTICAL AGAIN. Bit-for-bit.` Its own
  conclusion — *"That means the render is not re-running, or the PNG is not being overwritten.
  This is the obvious thing I should have checked"* — was right, and it took one step to confirm
  by checking timestamps.
- **k3** got SSIM `1.00000` between two iterations and diagnosed it correctly and immediately:
  the cloud coefficient was too weak and most of the cloud sat above the `ray.y` range the
  post stage shows. `Fine — no harm.`

> The cost is not that the failure is invisible. It is that **the framework has the answer and
> does not print it.**

The compile already produces a **delta** against the previous emitted document — that is the
mechanism [`compile-cost.md`](compile-cost.md) is built on, and the one the device uses to avoid
re-drawing. **A delta of zero operations is precisely the statement "your edit did not change the
graphics".** It is computed, it is correct, and it is thrown away.

**The change.** After a compile, when the emitted document equals the last one rendered, the
harness says so — loudly, before the image — and names the nearest paths that *did* differ:

```text
warning: your change did not reach the frame
  bamboo.rs:214  FOG_DENSITY  0.00028 → 0.00028   (unchanged)
  the emitted graphics are identical to the previous render
  nearest paths that differ from the last render: bamboo.rs:252 SUN_INTENSITY
```

**The second line is the one that pays.** "Nearest path that differs" turns a ten-minute bisect
into a one-second look, and it answers the question all three were actually asking, which was
never "did my edit save?" but **"what in this scene still moves the picture?"**

**A provenance stamp, in the sidecar.** The position sidecar
([`agent-debug-scenes.md`](agent-debug-scenes.md)) already establishes that an output writes a
file of numbers beside the png. Add to it: the binary's build id, the mtime of every source file
the compile read, the pass count and the document hash. A stale build then says so in the file
rather than in a ten-minute argument.

**Why this is not a compile-time note.** `CompileNote` is for what the *compiler* could not do
exactly — a clamped page count, a scene that lists nothing under `depthing`. This is a different
claim: it is about the *caller's* edit relative to the *caller's* last run. It belongs to the
harness, and it is the one diagnostic that is a function of history rather than of a document.

---

## 2. There is one debug channel, and all three hand-rolled the other three

**Priority: highest. Cost: the G-buffer, which is already named as the next thing.**

[`agent-debug-scenes.md`](agent-debug-scenes.md) landed `Position` and said, in its own words, that
a normal / identifier / albedo image "needs a G-buffer — several colour attachments written by one
geometry pass — which is a graphics-schema and renderer change of its own. It is the obvious next
channel and it is deliberately not this one", and again under *Still owed*: "the change that would
pay for itself next."

**The three sessions are the evidence for that ranking, independently and without coordination.**

| what they needed | who | what they did instead |
| --- | --- | --- |
| where is this pixel in the world | dsf | `--debug position`, twice. Found *the camera is too close* and *the roof is a 2400-unit magenta plane*. The single highest-value diagnostic in any of the three runs. |
| what is this surface's normal | m3.1 | hand-wrote a normal-as-colour splice, and then discovered it **was not executing on the artifact in question** — a probe that lied |
| is this the wall or the roof | m3.1 | planned an elimination test: *"render cam1 with the `wall` mesh not drawn. If the panel disappears → wall. If it stays → the roof. Two builds max."* |
| why is this colour that colour | k3, dsf | gave up on the framework entirely and sampled their own PNGs from PowerShell |

Three of the four are a `Position` sibling. The fourth is not a channel at all yet, and is item 6.

**The change.** Land `Normal` and `Identifier` from the G-buffer the proposal already scoped, plus
a bare `Depth` — which is nearly free, because the depth buffer already survives to the post stage
and `terraces` already recovers a world position from it. `Identifier` is the one that retires a
whole class of bisection: "is this the wall or the roof" becomes a pixel read instead of a build
budget.

Note the same contract the position channel already carries: a channel must not move a pixel of
the beauty image, and it must say what it does not cover rather than leave it to be discovered.
`Identifier` covering only what the depthing bucket drew is a contract the same way `Position`'s
is, and should be stated rather than measured.

---

## 3. All three buried the camera, and nothing checked

**Priority: high. Cost: small — the bounds are already computed.**

Every one of the three put a camera inside geometry, and each found out by looking at a broken
picture.

- **dsf** put a camera in the pond (`z = 4.2`, pond spans `1.6–7.6`) and got a water jar filling
  the frame. Three separate cameras were aimed directly at a courtyard wall, which it misread in
  turn as *the sky*, then as *the mountain ridge*, and only finally as the wall.
- **m3.1** grew a ridge on the camera side, so cameras 0 and 5 sat inside a 100 m hill. Its
  grove had `GROVE_RADIUS = 34` and the eye was at `33.4` — **0.6 units from being outside the
  grove.**
- **k3** spent twelve rounds on one camera, changing its position four times, and ended the run
  with a frame it had called `STILL blank milk` at step 103.

> Every one of these is a fact the compiler already knows at the moment it emits the frame. The
> eye is a field. The target is a field. The compiler computes the bounds of every instance —
> `bounds_of_instances` is a named line in [`compile-cost.md`](compile-cost.md)'s profile.

**Correction, recorded here rather than fixed in place.** An earlier draft of this item also said
`--census` already prints the scene's bounding box, and offered that as the reason these notes are
cheap. **It does not, on `main`.** `Gallery::census` and the `--census` flag exist only on
`feature/dsf-bamboo-courtyard` and have never been merged; see item 7. The notes are still cheap,
because the bound is computed in the compiler either way — but they cannot lean on a printed
surface that is not there, and whoever writes them should decide deliberately whether to also land
the census rather than duplicating its output.

**The change.** Three notes, in the vocabulary
[`adapter-limits-and-the-page-pool.md`](adapter-limits-and-the-page-pool.md) already established
— *"a count above what the machine has is bounded and named; a count below one is refused"*:

- **the eye is inside an instance's bounds** — names the instance and how deep inside
- **one instance covers most of the frame** — this is dsf's water jar, k3's gatehouse, and it is
  a proportion, which makes it exactly the "bounded and named" case rather than an error
- **the target is nearer than the near plane**

All three are notes. None of them refuses to compile: a scene may legitimately want a camera
inside a volume, and the policy from the page-pool proposal is that a note distinguishes *a
smaller frame* from *a wrong one*. What must not happen is a broken picture that says nothing.

---

## 4. Nothing tells a scene how big it is, and scale is not scale-invariant

**Priority: high. Cost: small.**

- **dsf** built a roof from a rise in metres against a ridge in world coordinates, producing what
  it computed as *"a 6.6-meter-tall roof in a 14-meter-wide house, which is absurdly steep"*. It
  found this with a second `--debug position` probe, not by reading the code it had just written.
- **m3.1** inherited the template's attenuation constant `1 / (1 + 0.0035r + 0.00002r²)`, which
  [`photographic-scenes.md`](photographic-scenes.md) item 6 describes as *"tuned for a scene a
  couple of hundred units across"*, and applied it to a scene a thousand units deep — so its sun
  was attenuated more than twentyfold and its intensity was twenty times a radiance. That
  proposal already flags the constant as *"a factor the author has to know about and cannot
  set"*, and defers the fix because the constant lives in a struct another branch is rewriting.

> The compiler knows the extent. The consequence of the extent is not reported.

(As with item 3: the draft of this item leaned on `--census` printing bounds. It does not, on
`main`. The extent is computed in the compiler; it is not currently printed anywhere.)

**The change.** A compile note carrying the scene's span and what the pipeline's constants mean
at that span:

```text
note: this scene spans ~1120 units
  light falloff is tuned for a few hundred; your far light is attenuated ~20x
  shadow page covers 62 units of a 1120-unit scene
```

This is a note rather than a fix, and deliberately so. Making the falloff physical is the correct
end state, but the note is nearly free and the *scaling* of the note is the part that was
unknowable. It turns "the light is wrong and I do not know why" into a number.

---

## 5. All three delivered images they had never looked at

**Priority: high. Cost: small, and the tooling to do it has already been written by hand.**

| | finals it had inspected | how the blind spot happened |
| --- | --- | --- |
| dsf | 5/6 | the rain frame was changed at step 331 and never re-opened. **Delivered, and not mentioned.** |
| m3.1 | 0/6 (best SSIM 0.690) | at step 153 it switched to a 1060×710 contact sheet and stayed there for the last 45 minutes. `bamboo-camera3.png` was never opened at full size at all. |
| k3 | 3/6 | step 182 changed the paving, 183 re-rendered, 184 re-checked only cameras 2/3/5. |

m3.1's switch was **correct** and worth keeping — the step 154 self-assessment (*"三个严重问题——院子
像空沙盒、竹林像棕榈 plantation、三张图构图雷同"*) is only possible when six frames are seen at once,
and "three of them are compositionally identical" is invisible one frame at a time. The method was
right. **The cost was that the final judgement was made on a 1060×710 thumbnail**, and nothing
about the workflow said so.

**The change.** Two parts, both small.

**A render manifest, written by the harness.** For every output: scene, camera, `time`, content
hash, pass count, instance count, and the mtimes of the sources the compile read. The film-lang
analysis team already built exactly this by hand — `stage-and-measure.py` and a `manifest.json`
with `final_to_wip` chains — because they needed it to answer "which of the six did it never
look at". They had to reconstruct it from byte counts and ffmpeg SSIM. It should not require
reconstruction.

**A `--sheet` whose tiles are full resolution.** The contact sheet is the right instrument and it
should not cost resolution. A sheet of six 1024² tiles is a 3072×1024 image; a model can crop it.

Neither of these can *force* a model to look at a picture, and nothing here pretends otherwise.
What they do is make "which pictures did I actually inspect" a question with an answer, instead of
a forensic exercise three teams have now run by hand.

---

## 6. Exposure and tone mapping are a black box, and two models lost hours to it

**Priority: medium. Cost: one scene field, plus a channel from item 2.**

Two independent runs, the same shape of error, both about a parameter that does not do what its
name says:

- **dsf** cut exposure from 0.86 to 0.48 — a 44% cut — and the image barely moved. The actual
  cause was a sun glow term adding `+0.108` to the sky. It established this by sampling its own
  PNG pixel by pixel from PowerShell and printing `(760,300) = 208,206,204`.
- **k3** darkened a clay albedo from `[0.33,0.19,0.11]` to `[0.135,0.082,0.05]` — 60% — and the
  pumpkins stayed orange. The real cause is ACES pushing bright red toward yellow-white at the
  high end, so the fix was to lower the **emissive** peak, not the albedo. **This took 92 minutes**
  (step 50 → step 150).

> In both cases the model was reasoning about a transform it could not observe and could not
> query. Neither guessed well. Both eventually left the framework and measured the output PNG
> with a script, which is the framework telling them, by its absence, that it has no answer.

**The change.** [`photographic-scenes.md`](photographic-scenes.md) item 6 already proposes the
exposure field and correctly identifies it as *"a scene field the emitter stamps into the post
uniform"*. It should land. Exposure is currently a number with no units, no meaning and no
statement of what it multiplies.

**And a grade channel**, in the same family as `Position`: render the **linear** image beside the
graded one. k3's 92 minutes was the cost of not having it; the channel would have made it one
comparison. It is also the honest generalisation of the position sidecar — a float image and a
sidecar of numbers, which is the pattern that already works.

---

## 7. WGSL is assembled by string concatenation, and its scoping errors surface as noise

**Priority: medium. Cost: medium — error locations through the assembly.**

The gallery's shaders are built by concatenating Rust string constants into a `main`. That design
produced three distinct failures across the dsf run, and two of them were not the model's fault:

- Step 113: a surface body that inlined a constants block containing **function definitions** into
  a function body where WGSL wants statements. The error was
  `lighting/lighting/ground does not parse as wgsl: expected assignment or increment/decrement,
  found "c_sun_share"` — which names the shader but not the cause.
- Steps 122–126: `ray_through` was not found. **The cause was structural**: the shared block is
  injected into every shader, but `ray_through` and `uniforms.camera` exist only in the post stage.
  A stage-scoping mistake surfaces as a missing symbol in an unrelated shader.
- Steps 139–213: seventy-three steps on two raw-string terminator defects, both introduced by the
  model's own PowerShell slicing. The first produced a **shifting** error —
  `prefix \`cast\` is unknown`, naming a different identifier after each edit — with the real
  message, `error[E0765]: unterminated double quote string`, buried at the end of the output.
  This is a compiler diagnostic that points at the wrong line and moves when you edit.

**The change.** Carry the source file and line of every fragment through the assembly, so an
assembled-module parse error reports where the fragment came from. `naga` already validates the
module; what is missing is the map from assembled offset back to `shaders.rs:412`. Two of the
three failures above become one step, and the third becomes legible.

This is the same class of gap as [`texture-channel.md`](texture-channel.md)'s finding that a
**declared** rule is safer than an **inferred** one: the framework got a numbering rule wrong
because it inferred a need from a format, and the fix was to make the document declare it. Here
the framework infers stage scope from a string constant, and the fix is the same shape.

---

## Two process gaps, which are not code

**Nothing any of the three runs produced has reached `main`, and a partial hand-apply already
broke the build.**

The original draft of this section said only that k3's proposal was never collected. Checking the
branches rather than the worktree directories says considerably more:

| branch | ahead of `main` | what is stranded on it |
| --- | --- | --- |
| `feature/dsf-bamboo-courtyard` | 2 commits | `Gallery::census` + the `--census` flag; **327 lines of `gallery/geometry.rs`** including the `blade_stations` out-of-bounds panic; **`gallery/shaders.rs` +81** — a `lighting_shader_with_schema` that lets a scene pass its own custom schema instead of the hardcoded `gallery_custom_schema()`; `attributes.rs`; `clustered_lights.rs` test pins |
| `feature/k3-bamboo-courtyard` | 1 commit | `proposals/sky-fog-distance.md` — the only framework-level defect any of the three found; test pins |
| `feature/m3.1-bamboo-courtyard` | 0 | it never committed, by its own account (*"你没让我提交"*) |

Two of those are **library** changes, not scene files, and both are the kind that help the next
scene rather than the one that made them: the `blade_stations` panic is a bug any future foliage
hits, and `lighting_shader_with_schema` is the difference between a scene that may bring its own
`custom_schema` and one that cannot.

The consequence is not theoretical. Building this file's item 1, the workspace **did not compile**:
a fragment of the dsf branch had been hand-applied into `main`'s working tree — the
`"--census" => census = true` match arm was present in `render.rs:318` while the `census: bool`
field on `Request`, the `let mut census = false` binding, and `Gallery::census` itself all were
not. It was the only lone-LF line in an otherwise CRLF file. A partial copy of an unmerged branch
is worse than no copy, because it is indistinguishable from working code until the build fails.

**What is missing is not a reminder to read the worktrees.** It is a defined path for a finished
run's work to reach `main`, and a rule about which half-applied states are acceptable. Both are
process, and both are cheaper than the one ten-minute debugging session they would have saved.

**An orphaned handoff is sitting in main.**
`.agents/handoffs/photographic-scenes.md` describes work that is merged — the proposal is now the
record, and says so — but the handoff was never deleted. The other five merged handoffs were
explicitly removed (`Delete the light radius handoff: the work is merged`,
`Delete the debug outputs handoff: the work is merged`, and three more). `AGENTS.md` says to delete
a handoff when the implementation is finished. This one was missed.

---

## What is already right, and should not be touched by any of the above

The three models independently converged on the same working practices, which is the best evidence
in this file that the framework's core is sound:

- **All three ran a smoke render of an existing scene before writing any scene code.** Every one.
  `dsf` step 85: *"Let me first verify that rendering works at all, then I can measure the cost."*
  That instinct is the framework teaching correctly.
- **All three rejected a framework component when it was not good enough, and rebuilt it.** dsf
  tore out `air::Atmosphere` entirely to get atmospheric control, at the cost of five consecutive
  WGSL compile failures. That the framework is *replaceable* is a feature, and it is why all three
  shipped at all.
- **The uniform layout verification held throughout.** Across three runs, hundreds of custom
  schemas, and shaders written by hand against documents, the check that compares field names and
  offsets never produced a false positive. It is the single best piece of engineering in the
  workspace, and none of the failures above is in it.
- **`k3` did not weaken a test to get it green.** A pass-count pin went stale when it added a
  scene to the gallery; it updated the expected list and said why, rather than editing the
  assertion. The framework's contracts are strong enough to be worth keeping, which is what makes
  the rest of this list worth doing.

---

## The order, and what has happened to it

Items 1 and 4 go together — they are the same file, and the delta-zero check needs a record of the
previous run, which is the manifest.

1. ~~**Say that the change did not reach the frame**, and **the render manifest** that makes it
   possible.~~ **Landed.** The harness writes `<out>-manifest.json` per run and compares the next
   run against it before drawing, separating the three cases the runs above could not tell apart:
   stale binary, edit that never reached the document, and a document change that does not reach
   this camera's pixels. Handoff: `render-manifest.md`.
2. **Camera notes at compile time.** **Landed.** Three notes — eye inside an instance's bounds, one
   box filling the frame, target nearer than the near plane — all notes and never errors. One note
   per *box* rather than per instance, and the frame-coverage condition also requires the box to be
   nearer than the target, which is what keeps a backdrop wall out of it. Handoff:
   `camera-notes.md`.
3. **`Depth` first, then `Normal` and `Identifier`.** **Partly landed, and the remainder is
   shelved by decision rather than merely undone.** `Depth` and the multi-colour-attachment support
   in the renderer are in. `Normal` and `Identifier` are **not**, and the reason is better than
   anything this file originally said: *a debug texture is only general once a shader graph exists
   and any value inside a shader can be wired to an output texture; until then every channel is a
   hand-wired special case.*

   That line runs between the two groups. `Position`, `Depth` and `Grade` each read a **frame
   resource** the pipeline already produced — a depth buffer, the shaded image — and are answerable
   by the machinery as it stands. `Normal` and `Identifier` read a **shader's computed value**, and
   that is where the per-channel cost stops being linear: each one is an enum variant, a format, a
   sidecar description, a coverage contract and an emitter insertion point. A graph turns all of
   that into an edge rather than a case.

   The multi-attachment work is **substrate, not a detour** — a graph whose outputs are several
   render targets needs exactly it, and it is landed and tested. Handoff: `gbuffer-channels.md`.
4. ~~**A full-resolution contact sheet.**~~ **Landed, as an opt-in.** `--sheet-full` asks for every
   pixel of every frame; `--sheet` still samples every fourth, so no existing invocation changes.
   Handoff: `contact-sheet.md`.
5. **Scale notes, the exposure field, and a `Grade` channel.** **Landed, all three.** The scale
   note reuses the bounds walk rather than making a second. The exposure field is a single linear
   multiplier applied before the curve, and a scene that names it without declaring it in the post
   shader is now **refused** rather than silently drawn unexposed — a real hole, found while
   building it. Handoff: `scale-and-exposure.md`.
6. **WGSL error locations.** **Landed**, and it is the weakest item on the page. A block declares
   its origin in a WGSL comment above itself, because the emitter rebuilds a shader's entry from
   `source` and `schema` alone and a map carried beside the string does not survive that boundary.
   The **stage-scoping** failure is *not* fixed; the fix is written down in the handoff.
7. **A path for a finished run to reach `main`.** Not code, and still owed. The orphan handoff is
   gone; the stranded-branch problem is not.

### Three things the implementing work found that the drafts above got wrong

- **A debug channel's sidecar used to describe itself as a world position whatever it held.** When
  `Depth` landed, `render --debug depth` wrote a file claiming to hold world positions, normalised
  over `[0.996, 1.000]` — a reader would conclude the sky is 1.0 units away. The header is now a
  property of the channel, and each names the three things it cannot show.
- **The harness was reporting a sheet size it never wrote** — `3072x2048` printed beside a 768×512
  png. The line now names the file's own size and its sampling.
- **A scene naming a uniform field its shader does not declare compiled clean and drew a frame
  that ignored the field.** The uniform is uploaded with the layout the shader itself declares, so
  an undeclared field is dropped by the upload rather than caught by the interface check. This was
  found while landing the exposure field, and is now refused by name. It is the same failure class
  as the texture-channel finding — *a document that is valid, a render that succeeds, and a result
  nobody asked for* — and it was live until today.
