# What the Cycles emitter drops from a material, and which of those it could carry

**Measured 2026-10-10** on `cycles.exe` 5.3.0, source at `C:\resource\cycles\src`, by wiring
shader graphs by hand into an emitted document and rendering. The probes are `out/probe-texture.py`
and `out/probe-sockets.py` in the worktree `feature/exporter-images`; both are diagnostics and
neither is repository source.

## The question

The loss report said a material declares custom data with no `principled_bsdf` socket to fold
into, and named only `tint`. Three fields were named across the gallery — `emissive`, `reflect`
and `image` — and every one of them was being dropped on every material of every scene. Which of
them could have been carried, and which genuinely cannot?

## What was measured

### `emissive` — carried, and it is now carried

A colour into a colour socket. `principled_bsdf` has `Emission Color`
(`src/scene/shader_nodes.cpp:2682`) and the field is written as a node attribute. It is the one
of the three that is the same law on both sides, so it needed no approximation, only a socket.
`Emission Strength` keeps the node's own default of zero unless the document says otherwise, so a
surface naming a colour and no strength still emits nothing — which is what the rasterised path's
own shader does with the same field.

### `reflect` — written, rendered, measured, and reverted

`principled_bsdf` carries `Specular IOR Level` (`:2664`) and the mapping is the obvious one. It
is wrong.

ours is how much sky a surface shows **at a grazing angle**; that socket is the reflectance a
surface has at **normal incidence**, with Fresnel supplying the rise towards grazing on its own.
Measured on `landscape`, pearson against the rasterised frame:

| band | without | with `reflect` written |
|---|---|---|
| whole frame | `+0.6223` | `+0.5733` |
| below the sky | `+0.6249` | `+0.5919` |

the document's zero is not a dielectric. Writing it removes the specular the sun and the sky
were contributing, and the frame loses more than it gains. Reverting restored `+0.6223` and
`+0.6249` **to the digit**, which is the check that `reflect` was the whole of it.

so the field stays in the report. that is the difference between an approximation and a mistake:
both are defensible, and only the measurement says which one this is.

### `image` — cannot be carried, and not for the reason the message gave

the message said "no `principled_bsdf` socket to fold into". The node **does** have one, called
`Base Color`. It cannot be **connected**:

```text
src/app/cycles_xml.cpp:219
  string_split(from_tokens, node.attribute("from").value());
  string_split(to_tokens,   node.attribute("to").value());
  if (from_tokens.size() == 2 && to_tokens.size() == 2) { ... }
  else LOG_ERROR << "Invalid from or to value for connect node.";
```

`string_split` splits on whitespace, so `"tiles_bsdf Base Color"` is **three** tokens and the
branch is not taken. **a socket whose name contains a space is unreachable in this format**,
whatever the node is.

measured, by wiring each connect on its own into an emitted `tiles` document:

| connect | loader errors |
|---|---|
| `coord UV` → `tex Vector` | **0** |
| `tex Color` → `tiles_bsdf Base Color` | 1 |
| both, either order | 1 |

the socket names are all correct — `texture_coordinate`'s `UV` output is at `:4036`,
`image_texture`'s `Vector` input and `Color` output at `:287` and `:289`, and
`principled_bsdf`'s `Base Color` at `:2647`. The node names are `texture_coordinate` (`:4028`)
and `image_texture` (`:246`); **`tex_coord` is not a registered node**, which is what a first
guess gets.

`diffuse_bsdf`'s colour socket is named `Color` (`:2598`) — no space, so it *is* connectable.
Substituting it would trade away the metallic, roughness and ior the same material declares,
which is a different shading model rather than a carrier for this one. So the field is refused,
and the report now says which reason rather than implying there was no socket at all.

## What this changed

| scene | dropped, before | after |
|---|---|---|
| `landscape` | 22 | 14 |
| `tiles` | 10 | 6 |
| `spotlight` | 18 | 8 |
| `sunlight` | 21 | 9 |
| `lanterns` | 30 | 17 |

the reduction is the `emissive` fields now honoured and the `texture` reports that were never
true; `reflect` is still listed, because it is still not carried, and the `tiles` image is
listed with its reason.

## The other half: a texcoord the report called uncarryable

separately, and found while bisecting the above. `geometry.rs` reads UVs from `UV_FIELDS`, whose
first entry is `texture` — the name this workspace's own `kit::shaded_point_schema` uses — while
`attribute_map::is_mappable` listed `uv`, `texcoord` and `uv_map` and **not** `texture`. Two
lists answering the same question, disagreeing.

the UVs were therefore read and written correctly the whole time, and the report claimed
otherwise on **every mesh of every scene**: `landscape` alone named five false drops. The two
lists are one list now.

## Sources

- `out/probe-texture.py` — wires the graph, renders patched and unpatched, judges by local detail
  against the frame's own blur rather than by an absolute "is it flat" threshold, which a lit
  flat floor fails for reasons that have nothing to do with the texture.
- `out/probe-sockets.py` — one connect per variant, counting loader errors, which is what
  separates a wrong name from a right one; the graph itself cannot.
- `src/app/cycles_xml.cpp:219` — the whitespace split and the two-token requirement.
- `src/scene/shader_nodes.cpp:246`, `:2598`, `:2647`, `:2664`, `:2682`, `:4028` — the node names
  and socket names used above.
