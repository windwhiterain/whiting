# What the binary form of a scene document costs

**What does the binary form of a scene document cost, in bytes and in time, and does it hold the
same document as the json form?**

Recorded 2026-10-20. Measured on this machine, in the `release` profile, with
`cargo run -p whiting-gallery --example document_binary_probe -- <scene> out`. The format itself
is specified in `crates/definition/src/binary.rs`, and the probe that reports the codec on its
own is `crates/definition/examples/binary_probe.rs`.

## The answer

**The binary form is a quarter smaller and an order of magnitude faster to read, and it holds
the same document.** The size that is left is not the encoding's: it is the mesh.

## `hall`

| | json | binary |
| --- | --- | --- |
| bytes | 5,602,756 | **4,273,159** (76.3%) |
| encoded | 13.71 ms | **4.22 ms** |
| written | 2.91 ms | **3.24 ms** |
| read | 9.98 ms | **12.20 ms** |
| parsed / decoded | 43.85 ms | **4.29 ms** |
| read, end to end | 53.83 ms | **16.49 ms** (3.3×) |
| written, end to end | 16.62 ms | **7.46 ms** (2.2×) |

The document holds 4,794 values, of which 4 are packed runs of 3,819,648 bytes and 1 is a byte
sequence of 131,072 bytes. Every list in it holds 238 elements together, and the longest holds
12.

## `alpine_oak`

| | json | binary |
| --- | --- | --- |
| bytes | 652,794,206 | **480,911,625** (73.7%) |
| encoded | 2.48 s | **419.58 ms** |
| written | 401.40 ms | **453.27 ms** |
| read | 302.39 ms | **190.67 ms** |
| parsed / decoded | 6.84 s | **435.41 ms** |
| read, end to end | 7.14 s | **0.63 s** (11.4×) |
| written, end to end | 2.88 s | **0.87 s** (3.3×) |

The scene builds in 9.83 s, and the json size is the 652,794,206 bytes the proposal
[scene-def-the-renderer-draws-a-document.md](../proposals/scene-def-the-renderer-draws-a-document.md)
records, so the measurement is of the same document that proposal measured.

**The writing times are the disk's, not the codec's.** Across the runs of this measurement the
json write was 343 ms to 401 ms and the binary write 258 ms to 453 ms, and the encoding itself
is the part that moved: 1.87 s to 2.48 s of json against 386 ms to 448 ms of binary. The read
is the number to read, and it is the disk either way.

## What the size is made of

| | json | binary |
| --- | --- | --- |
| the packed runs | 562,399,744 (base64) | **421,799,808** (raw) |
| the byte sequences | 699,052 (base64) | **524,288** (raw) |
| everything else | 89,695,410 | **58,587,529** |
| the geometry's share | 86.2% | **87.7%** |

The tree holds 3,315,173 values, of which 5 are packed runs. Every list in it holds 1,894,770
elements together, and the longest holds 59,521.

**The base64 is the whole of the difference in the runs**: 421,799,808 bytes of run are
562,399,744 bytes of text, and the binary form writes the 421,799,808. Everything else — the
rows and the floats in them — shrinks from 89,695,410 bytes of decimal text to 58,587,529, which
is the difference between a float as text and a float as eight bytes.

## The two forms hold the same document

The comparison is made by the bytes of the binary form rather than by a walk of the two trees:
the binary form writes a float as its bits, so two documents that hold the same numbers are the
same bytes and two that hold numbers that print alike are not.

- the json document and the binary document are the same document, for both scenes;
- the binary document reads back and writes the same bytes, for both scenes;
- the json document reads back to the same document as the binary one, for both scenes.

**The json form loses nothing of either scene.** That is worth stating because it is what makes
the frame below the same frame: the two paths differ only in how the document is spelled.

## The frame is the same frame

`hall` rendered from the json document and from the binary document gives the same document
hash and the same image hash, and the harness says so in its own words:

```text
warning: hall at camera 0, the scene's own time is the last render again — nothing changed
  documents  identical
  pixels     identical
```

against `document e74fcca4725153fa9281a8f1600950f1e49f7cd4438733a7d824efc237f4afbf`, `image
72717cd0d81ccce351b1ab4d7ee1cd088148c9f6c1c6bbadaf3f5cb0bab75c34`, 13 passes.

## The document is the same bytes every time

The producer's document and the probe's document are byte-identical — sha256 `408af0aa55a89c
983a70787900f54a96f573580148872c52d3e13ecf683b07bf` for `hall`, `87cbe46f0d510223cb0656edf
009ac2ffa98f83a940c7905f89dbe4a7a5e92ce` for `alpine_oak` — and the `hall` document written by
the `debug` producer and by the `release` producer is the same file. A document is therefore
comparable by its bytes, and a diff of two documents is a diff of two files.

## What this says about the next step

**The codec is not the next step; the mesh is.** 87.7% of the binary document is packed runs of
triangles, and no encoding makes a triangle smaller than the three vertices it is. The codec
took 172 MB off the oak's 653 MB and made the document ten times faster to read; what is left
is the 421,799,808 bytes of geometry the document ships, which is a fully tessellated mesh
rather than the source that makes it.

**What was not measured:** the cycles backend, which reads its document with `facet_json` still
and was not touched; the cost of the binary form on a device, which is a different question; and
the size of the document once the mesh is not fully tessellated, which cannot be measured until
it is not.
