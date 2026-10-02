# A byte run in a document

Status: **landed** on `feature/bytes-format`. The run of a packed list and the bytes of a
`Value::Bytes` are now written as base64 text, which a reader takes; the `f64` round trip is
fixed as far as json allows it. Numbers below are measured, and the tool that measures them is
`crates/device/examples/bytes_format_probe.rs`, added with this change.

## The observation

A packed list is the framework's representation of bulk geometry: for a mesh, a flat run of
little endian floats, compared with a `memcmp` and validated with `len % stride == 0`. It was
also the one payload a document could not carry, because `facet-json` writes a `Vec<u8>` as a
json array of decimal octets.

Measured on a run of 20,000 points at a 24 byte stride, which is 480,000 bytes of run, on one
machine and one debug build:

| | text | against the run | write | read |
| --- | --- | --- | --- | --- |
| decimal octets, as it was | 1,262,194 bytes | ×2.63 | 30 ms | 608 ms |
| base64, as it is now | 640,073 bytes | ×1.33 | 14 ms | 33 ms |

**Half the text and a read eighteen times faster.** A hundred thousand points is 2.4 MB of run:
6.0 MB of json and about 3 s of parse before, 3.2 MB and about 165 ms now. `Value::Bytes`,
which the emitted graphics documents already carry their packed vertices in, moves identically:
480,000 bytes go from 1.7 MB of text and 608 ms to 640 KB and 33 ms.

The write is the smaller half of the win because writing a number was never the expensive part
of walking a tree; reading one back is, because every octet is a number token to scan and a
number to parse.

The recorded baseline in the sibling probe (`facet-probe`, `facet_value_probe`) is 1,262,205
bytes, 57 ms to write and 565 ms to read for the same run. The write differs between the two
measurements because that probe serializes `Value::Packed` and this one serializes a mirror of
the shape the run had — a layout and a bare `Vec<u8>` — so the enum wrapper is part of the
difference. The size and the read agree.

## The two candidates

### A file sidecar

The mechanism exists and is worth stating exactly, because it is not what it looks like.

- **A slot whose schema is marked `.reference()` accepts a `Reference::File`**, whatever its
  kind: the loader resolves the spelling before it looks at the kind. The standard mesh schema
  already marks one slot — `meshes[i]`, the element schema of `meshes` — so a whole mesh can be
  named from a file today.
- **But what a file must hold is decided by the slot's kind, not by the value.** `Reader::file`
  asks the store for `Wanted::Asset` only for `SchemaKind::Bytes`, and for `Wanted::Document`
  for every other kind (`crates/definition/src/loaded/load.rs:246`). A packed run sits in a
  list-shaped slot, so a file there is wanted as a *document*, and a store that hands back
  `FileContent::Bytes` is refused: *"`points.json` holds bytes, and this slot wants a
  document."* That is the whole answer to whether a `Packed` slot takes a file: **it takes a
  file, but never raw bytes.**
- The mesh's own `points` slot is **not** marked, so a run cannot be named where it sits today;
  a mesh whose `points` spells a `FilePath` fails validation with *"expected list, found enum"*.
  The file granularity the schema offers is the whole mesh, which is the natural one anyway.
- `RegistryStore::file` always errors, by design, so a host that wants file references must
  implement `Store` itself. That is small: the probe's store is fifteen lines, and a real one
  adds reading the file, parsing it as a `Value`, resolving a relative path against the
  `Origin`, and deciding which paths are allowed. Nothing in it is device-specific, so the
  **mechanism belongs in `definition` beside `RegistryStore`; the policy belongs to the host.**
- The policy is the reason this is not shipped here. A store that reads `$path` from a document
  makes the device read whatever file a client names, and the current refusal is the only thing
  standing in the way of that. That is a decision for the owner of the process, not a library.

What the sidecar buys: a scene that names an unchanged mesh stops carrying its bytes on every
reload and in every snapshot. What it does not buy: **any reduction in what the bytes cost when
they do travel.** A file reference at a packed slot is resolved as a document, so the run
inside that document was spelled the same 1,262,194 bytes of decimal octets, and would have
been read with the same 608 ms parse. The sidecar relocates the problem; it does not answer it.

### A byte spelling that a reader takes

`facet-json` 0.44.7 (the version `Cargo.lock` holds) offers `SerializeOptions::bytes_as_hex()`,
which writes `{"Bytes":"0x00017fff"}` — and its own reader refuses exactly what its writer
produced, because a `Vec<u8>` is a *sequence* to the reader and a string is not. There is no
base64 in it at all.

The versions that resolve: `facet-json = "0.44"` resolves to 0.44.7, which is the newest 0.44.x.
The newer lines are 0.46.1 (stable) and 0.50.0-rc.7 (prerelease). Both still have
`BytesFormat::{Array, Hex}`, both still write hex and neither reads it, and neither has base64.
So the writer's hex is write-only in every available version, and no upgrade fixes it.

What fixes it is a spelling this crate owns, which is what landed: `facet`'s `proxy` mechanism
lets a type keep its representation and choose its *document* spelling.
`Value::Bytes(Vec<u8>)` gained a field proxy and `PackedList` a container proxy, both onto types
made for the purpose, and `facet-json` honours a proxy on the way in as well as on the way out
(checked empirically before the design was settled; the write-only trap of `bytes_as_hex` is
exactly the failure this avoids).

```
{"Bytes":"AAF//w=="}
{"Packed":{"layout":{"fields":[["position",3]]},"bytes":"AACAPwAAgD8AAIA/"}}
```

Base64 rather than hex because the run is the point: hex carries one byte in two characters,
which is still ×2.0 the run against base64's ×1.33, and nothing reads a document by eye. The
codec is forty lines in `crates/definition/src/bytes.rs` rather than a dependency, because this
crate depends on nothing but `facet` and `thiserror` and should keep doing so. It is strict in
both directions: `encode` and `decode` are inverses, and `decode` accepts exactly the canonical
padded spelling `encode` writes — a malformed run is an error rather than a different run. The
probe checks 67,329 byte sequences for the round trip and ten malformed spellings for refusal.

## What landed

- `crates/definition/src/bytes.rs`: `encode_bytes`, `decode_bytes`, `BytesError` and
  `BytesText`, the transparent spelling the two proxies use. Public, because a host writing a
  sidecar of its own wants the codec.
- `Value::Bytes` and `PackedList` are spelled through it, so every route — `PUT`/`GET
  /definition`, the snapshot file, `POST /restore`, deltas, definition reports — carries bulk
  bytes at ×1.33 instead of ×2.63 and reads them in a nineteenth of the time.
- `PackedError::Bytes`, so a run that is not the spelling this crate writes is refused with
  the reason rather than read as different bytes.
- `crates/device/examples/bytes_format_probe.rs`, the measurement above as a runnable report,
  following `facet_value_probe`'s convention of a probe that is an example and not a gate.

## The `f64` round trip, which is a separate defect

The document format fails three ways on `f64`, and they are `facet-json`'s, not this crate's.
Measured, then measured again with `facet-json`'s `fast` feature on, which is what this change
enables in the workspace manifest:

| value | before | after `fast` |
| --- | --- | --- |
| `-0.0` | written `-0`, read `+0.0`, bits lost, no error | written `-0.0`, read `-0.0` |
| `f64::MAX` | written as 319 digits, **refused by the reader** | written `1.7976931348623157e308`, exact |
| `5e-324` | written as 336 digits, round trips | written `5e-324`, exact |
| `f64::MIN_POSITIVE` | written as 336 digits, round trips | written `2.2250738585072014e-308`, exact |
| `NaN`, `±inf` | written `null`, read `0.0`, **no error** | unchanged |

`fast` adds `lexical-parse-float`, `lexical-parse-integer` and `lexical-util` to the lock; the
writer stops using `Display` (which never writes an exponent, which is why `f64::MAX` became 319
digits and then failed the reader's 128 bit integer accumulator) and the reader stops using
`str::parse` on a full decimal. The failure starts at `1e39`, whose shortest form is 40 digits:
the reader treats a literal with no point and no exponent as an integer and accumulates it in an
`i128`, which overflows there. It is one line in the workspace manifest and can be dropped
without touching the byte spelling, which is why it is called out here rather than folded in.

**The one case that cannot be fixed from this side is `NaN` and `±inf`.** Json has no spelling
for them, so the writer emits `null`, and the reader takes `null` for a float as its default
`0.0` with no error. Nothing in this workspace can intercept either end: the writer is
`facet-json`'s scalar path and the reader is `facet-reflect`'s default handling, and a proxy
cannot help because a proxy only chooses the *shape* a value is spelled as — the shape that
would work is `facet_json::RawJson`, which carries raw text on both sides, and the orphan rule
forbids implementing its conversions for `f64` from a crate that does not own either type. It
needs a change in `facet-json` (refuse a `null` for a float, or read it as an error) or a
document format that can spell it. **A `NaN` a scene computes is therefore silently stored as
`0.0` today**, which is worth its own issue upstream.

## What was left alone, and why

- **A file-reading store is not shipped.** The mechanism would be fifty lines in `definition`,
  and the probe carries the fifteen-line store that proves the load path works, but installing
  it in the device would let any client name any path on the host. The sidecar's remaining work
  is: a `FileStore` (or a device-owned one) that decides which paths are allowed, a build step
  that writes a mesh out as a file, and the decision about the granularity — a mesh per file
  works today, a run per file needs `points` marked as a reference in the scene schema.
- **`RenderOutput.bytes` in `crates/device/src/rpc.rs` is still decimal.** It is the same
  defect in the render response — a 1920×1080 rgba8 frame is 8.29 MB of bytes, about 21 MB of
  decimal json against 11 MB of base64 — but it is a *response DTO* whose reader is a client
  outside this workspace, so changing its spelling changes a wire contract that no in-tree code
  parses. It is a one-attribute change (`#[facet(proxy = BytesText)]`) whenever the owner wants
  it, and the byte spelling above is what makes it possible.
