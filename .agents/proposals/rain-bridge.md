# 风雨桥 — the covered bridge, and the drum tower's other half

The gallery has had a 鼓楼 since the fourth type. A 侗寨 is a tower **and** a bridge, and the bridge is the one
piece of the region's architecture that is a building and a road at once: a timber gallery over a river, with
stone piers in the water, a tiled gable the length of the span, and — the type's own signature — a **pavilion**
raised over the middle, where the old people sit and the crossing becomes a room.

## The three facts, and how each is built

| fact | how |
| --- | --- |
| **the deck is a slab and the piers are posts** | a `Slab::plate` from bank to bank, wider than the gallery, carried by three stone posts to a set standing in the water |
| **the gallery is a mass, because a roof needs a host** | `Roof::new` names the mass it sits on, so the gallery is a thin-walled volume over the deck and its roof is a gable running the length of the span |
| **the pavilion stands on the gallery through `Join::Storey`** | the relation the drum tower's seven floors introduced, at the middle of a bridge: the middle third is a second mass on the first, wearing its own hip |

## Two refusals, both correct

- **`MemberDegenerate` on the two top rails.** Their axis runs the span and their `across` was written along it
  too, so the section had no width; the expansion answered with the members' own length, `26.0`. A rail's
  section is measured **across** its span. The same mistake the 土掌房's parapet made in round eight — the second
  time, which is why the note is here and not only there.
- **`StoreyOverhangs` on the pavilion.** The pavilion was authored `0.55` proud of the gallery, and the storey
  relation claims its plan is *inside* its host's: `area: 31.8, shared: 25.2`, measured, and the claim was false.
  The walls are flush now and **the roof is what overhangs** — `0.75` of it — which is also how the type is built:
  a pavilion's projection is its eaves, not its walls.

## What the type still wants

- **open sides**: the gallery is a closed thin-walled volume because the roof needs a mass to hang on, and a real
  风雨桥's long sides are open bays with benches. The mass would need its faces cut into openings, which the
  openings machinery can do — the scene has not;
- **the water**: the surface under the bridge is the harness's own plane; a river is not authored;
- **the abutments**: the deck ends in mid-air on both banks, and a real bridge lands on a stone abutment.

Identity: **1 434 rows**, and twenty-three scenes in the gallery.
## The gallery is open now

A 风雨桥 is an arcade over the water with a bench between its posts, and the gallery was a closed thin-walled
box — the roof needed a mass to hang on and the mass was solid. Both long walls are now **cut into nine bays each
and every bay is opened** between a 0.55 m sill and 0.40 m under the wall's head, with the margin the crate's own
rule names: the wall's thickness plus five centimetres.

The sill is a hand's height above the deck because that is where the bench is, and the bridge now reads as what it
is from the bank: nine dark bays in a row under one long tiled eave, with the railing in front of them and the
pavilion over the middle.

Identity: 1 434 → **1 506 rows**.
## The bridge lands, and the river is a channel

The deck used to stop in mid-air on both banks, and the ground under the bridge was the harness's own plane. Three
pieces now, and they are the type's setting rather than its structure:

- **the river is a height function**: a channel `1.6 m` deep and `10 m` wide along the bridge's own axis, with
  `2 m` of bank either side rising to `1.1 m`. The ground mesh is drawn from it, so the piers stand in the cut and
  the deck clears the water — and **the water is the channel bed and not a surface**: nothing in this frame is a
  liquid, and a river with a reflection is a material this scene does not author;
- **two stone abutments**, one at each end, `1.8 m` of plate carried down from the deck to the bed. A slab and not
  a mass, because a retaining block has no inside — the same reason a parapet is a member;
- **a bench down each side of the gallery**, `0.46 m` above the deck and `0.36 m` wide: a 风雨桥 is where the
  village sits, and open bays without benches are a colonnade with nowhere to sit.

Identity: 1 506 → **1 530 rows**.
