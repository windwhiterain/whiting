# Handoff: the dissected upland, authored by hand (35 crests, 16 pockets)

## Why

The massif table was written on an even ellipse with scheduled heights, which the header's own rule
forbids. It is now a small tree whose crest coordinates were placed by hand and whose valleys were
hand-placed with them. Density is no longer the goal: **finer pockets only bought slivers**, and the
facet's height is the flank's depth, so the second and third scales now come from *one* spur forking
into two branches rather than from more tips.

## The tree as authored

- **Thirty-five crests.** Fourteen are the divide — summit, six saddles, six shoulders, a west and an
  east tip. Twenty-one are seven spurs, each a stem and two branches, leaving seven shoulders.
- **Sixteen tips and sixteen valleys.** A valley per pocket; tips walked counterclockwise
  `RIM_TIPS = [30, 31, 25, 24, 19, 18, 6, 15, 16, 21, 22, 28, 27, 34, 33, 13]`.
- **Naming**: `dc0..dc13` the divide, `s1..s7` the spurs with `stem`/`A`/`B`; `W3 W2 W1` and
  `E1 E2 E3` the shoulders, `colWs colW2 colW1 colEs colE2 colE3` the saddles, `F1..F7` the spur
  forks. Sharpness `0.50` on the divide, `0.42` on the spurs.
- **Heads**: a spur's fork when both tips hang off that spur; the summit for the three pockets the
  whole divide lies inside (valleys 1, 3, 10); otherwise the shoulder where the pocket's wedge closes
  (W2, W1, W1, E1, E3, E2).

## Measured facts (established on the baseline, not guessed)

- **Valleys = ground tips = leaves of the crest tree.**
- **`Sampling::corner` accepts a junction only at a line's two ends.** A child standing on a
  parent's *interior* node welds geometrically but is walkable one way only
  (`Unwalkable(Disjoint)`). Every fork must therefore be a segment end: each crest is one
  segment between two forks (or a fork and a tip / a col).
- **A valley needs ≥ 3 floor nodes** (≥ 2 belts) once its walls are single crests: each flank
  needs one belt whose top run traverses the crest, and the head's first row only collapses.
- **The head floor node stands on the head point itself.** Standing it off the head leaves the
  head belt's quad with four distinct corners, the diagonal runs head-to-first-floor, and both
  sides of the valley then emit the same triangle once each — a doubled face, euler one too many
  per valley, and `is_manifold` false. This is the only thing that made the mesh wrong, and the
  symptom (euler = 1 + valleys) is what names it.
- **The flank side is read at each belt's `from` station, not at the node a rib lands on.** Two
  pockets may therefore both name one tip from opposite directions and still file opposite sides;
  a hand check that compares rib sides *at tips* reports a fault that is not there.

## Validation

`cargo run -q -p whiting-gallery --example alpine_probe` from the worktree root reports
`violations: 0`, `surface: 1 boundary loop(s), euler 1, manifold true`, plan-area difference
`0.000`, facet aspect median 3.0 and worst 7.4 over 314 faces.

## Rules

The tables in `crates/gallery/src/landscape/massif/authored.rs` are the deliverable and are
**authored** — scratch arithmetic to place a point is a ruler, a script that computes the tables
is a generator and must not exist. `crates/mountain/**` is out of scope. English docs; commit
with `git commit -F <file>`; merge at checkpoints.

## The control points are placed by hand (user's rule, binding)

Every node, every elevation and every correspondence is put where a person put it. **No formula,
grid, ellipse, spacing rule or generator places a point**, and a schedule that computes heights
(for example "each fork takes 0.50 × 0.72^n of its shoulder's relief") is a generator even when
it only writes numbers. Scratch arithmetic to find where a point should go is a ruler; anything
that writes the table is not. Regularity that arrives on its own — an even ellipse, an even
arithmetic progression of heights — is the smell of a generated table and must be re-authored by
hand.
