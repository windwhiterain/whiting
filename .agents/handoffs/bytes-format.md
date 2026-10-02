# A byte run in a document: what is on this branch

Worktree: `./.worktrees/bytes-format`, branch `feature/bytes-format`, on top of `main` at
`232a861`. **Not merged**, by instruction: the owner decides.

The design record, the measurements and the alternatives are
[proposals/a-byte-run-in-a-document.md](../proposals/a-byte-run-in-a-document.md). This is the
state, the verification and the decisions left open.

## What is here

| file | what |
| --- | --- |
| `crates/definition/src/bytes.rs` | new: `encode_bytes`, `decode_bytes`, `BytesError`, `BytesText` — the base64 codec and the transparent spelling |
| `crates/definition/src/value.rs` | `Value::Bytes` is spelled through `BytesText` |
| `crates/definition/src/packed.rs` | `PackedListProxy`, and `PackedError::Bytes` for a run that is not the spelling this crate writes |
| `crates/definition/src/lib.rs` | `pub mod bytes`, `PackedListProxy` re-exported |
| `Cargo.toml`, `Cargo.lock` | `facet-json`'s `fast` feature, which fixes two of the three `f64` round trip failures |
| `crates/device/examples/bytes_format_probe.rs` | new: the measurement, the codec's round trip over 67,329 sequences, the snapshot envelope, and the two file-reference experiments |

`crates/device/examples/f64_probe.rs` was the throwaway used to measure the `f64` defect before
and after `fast`; its numbers are in the proposal and it is deleted before the commit.

## Verified

```text
cargo check --workspace          # clean
cargo test --workspace           # 10 + 3 + 13 + 8 + 3 + 6 + 11 + 7 + 3 + 12 + 1 + 2 + 5 pass, 0 fail
cargo run -p whiting-device --example bytes_format_probe
                                 # "probe finished: nothing disagreed with what went in"
```

The device's render tests report `0 tests` without the `wgpu` feature, which is how they ran:
the workspace builds and tests with no adapter, and the feature stays optional.

`cargo test --workspace` was run with `facet-json/fast` enabled, which is the state on this
branch.

## The decision this branch took

The byte spelling is the load-bearing change and the file sidecar is not sufficient on its own,
because a file reference at a list-shaped slot is resolved as a *document*: the run inside that
document was spelled as 1,262,194 bytes of decimal octets and read with a 608 ms parse either
way. The sidecar relocates the problem; it does not answer it. With the codec in place the
sidecar is worth doing for the re-send, and it is now cheap, because the document it names
carries the run at ×1.33.

## Left open for the owner

1. **Keep `facet-json`'s `fast` feature?** It fixes `-0.0` and every magnitude at or above
   `1e39` (including `f64::MAX`, which the writer currently emits as 319 digits and its own
   reader then refuses), at the cost of three `lexical-*` crates. It is one line to drop.
2. **`NaN` and `±inf` are still silent.** Json cannot spell them; the writer emits `null` and
   the reader takes `null` for a float as `0.0` with no error. Not fixable from this side — see
   the proposal — and worth an upstream issue.
3. **The file store is not shipped.** The mechanism is fifty lines in `definition`; the policy
   is the host's, because a store that reads `$path` from a document makes the device read
   whatever file a client names.
4. **`RenderOutput.bytes` in `device/src/rpc.rs` is still decimal**, which is the same defect on
   the render response. One attribute whenever the response's wire contract may change.
