# A terrace out of two leaves

Started 2026-10-27. Branch `feature/building-terrace` (worktree `.worktrees/building-terrace`, from `main` at
`3209710`, the baseline record). The design is
[`agentic-building-generation.md`](../proposals/agentic-building-generation.md).

## The question, and the answer it got

A terrace is houses side by side sharing one wall. The vocabulary has no "shared wall", so the first question was
whether it needs one:

> Is a party wall **one wall claimed by two masses**, or **two leaves whose outer faces meet**?

`examples/terrace_probe.rs` answers the second reading empirically — two 6 × 8 masses flush at `x = 0`, a 0.15 m
wall each, no roofs (so nothing is hidden by the silent no-roof case):

| measurement | value |
| --- | --- |
| violations | **0** |
| the left house | manifold **true**, boundary loops **0**, volume **32.3520** |
| the right house | manifold **true**, boundary loops **0**, volume **32.3520** |

**Touching is allowed.** A zero-volume meeting is not an overlap, so the crate already accepts a terrace as **two
closed solids whose leaves meet** — 0.15 + 0.15 of wall that reads as one 0.3 m party wall, which is how many real
party walls are built. **No new vocabulary, no new violation, no decision to make.**

## Status log

- **2026-10-27** — worktree and branch from `3209710`; the probe and the measurements above.
- **Next**: the `terrace` scene — three units of one style side by side, the channel from its first line, and the
  party walls as the pictures' own evidence.
