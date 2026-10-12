# Comparing a frame against one from an older branch measures the wrong thing

Recorded 2026-10-12, after the post stage's environment binding was added and appeared to
change every pixel of `dunes`. It changed none of them. This is the method note for that, and
it cost a false alarm that pointed at the one failure mode SPEC warns is invisible.

## What happened

The binding was added, `dunes` was rendered, and it differed from `d1r-dunes.png` in 669608 of
1048576 pixels with a maximum difference of 16. The sky band was almost untouched — 404
differing pixels out of 350208 — and the ground differed everywhere.

That distribution looks like a precise diagnosis: the bindings are in the right place, because
the sky is fine, and something about the environment's contribution to surfaces moved, because
the ground is not. The obvious next reading is that adding a second binding to the map's upload
changed what the lighting stage samples from it, which would be the binding-number
misalignment that `shaders.rs:900-908` describes as a frame of wrong numbers rather than a
failure.

**That reading was wrong, and the comparison was the reason.**

## The mistake

`d1r-dunes.png` was not rendered from this branch. It was rendered hours earlier, from
`feature/dunes-declares-its-sky`, at a point where the scene's environment radiance and the
surrounding commits were not what they are on `main` now. Comparing the new frame against it
measured **the difference between two states of the repository**, not the effect of the change.

The check that settles it is cheap and should have come first:

```
# render the same branch twice
P1 vs P2 (same code, two runs): differing pixels 0 of 1048576
P1 vs d1r (old branch):         differing pixels 669608
```

Rendering is fully deterministic here — two runs of the same build are byte-identical — so
every differing pixel is a real difference and none of them is noise. That is a fact worth
having, and it is also what made the false reading convincing: there was no noise to hide
behind.

The control is the other half:

```
git stash push crates/scene/src/standard/emit.rs crates/scene/src/standard/pass_schemas.rs
cargo run --release -p whiting-device-rasterize --example render -- dunes ..\..\out\B1-dunes
```

`B1-dunes.png` is this branch with the change removed. Against that:

| | differing pixels |
|---|---|
| B1 vs P1 | **0** |

Zero. The binding changes nothing, which is what adding a binding nobody reads must do.

## What this says about the method

**A baseline is a build, not a filename.** Every number in this work's documents is meant to be
reproducible, and a frame whose filename survives across commits whose code does not is a
number that will not reproduce. The frames in `out/` are named `w1`, `w3`, `d1`, `L3` and each
was rendered at a particular commit; reading one as though it were the current baseline is the
error, and the repository keeps them all, which is what makes the error easy to make.

**Determinism is what makes a difference meaningful, and it does not make it correct.** Zero
noise says the two frames really do differ. It says nothing about whether they should.

## What the binding change actually did

It bound the scene's environment to every post pass, on the same terms the lighting pass binds
it — present whether or not the scene names an environment, a one texel black map when it does
not — and declared it on the post interface, because the emitted documents are checked against
the shader's own interface and a binding the interface does not name is refused:

```
error: the scene does not compile: the emitted documents do not agree with each other:
pass `post/post` does not match the interface of shader `post`: at binders:
field `environment_map` is not declared by the schema
```

That refusal is the check working, and it is the strongest evidence in this change. The frames
are unchanged because nothing reads the binding yet. The point of adding it is that a post stage
**can** now read the sky the tracer draws, and the scene has to be the thing that decides to.
