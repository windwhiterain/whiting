# 村寨 — a village square, and the two operations that make composition possible

Every scene before this one is **one type**. A village is a composition of *types*, and the repository could not
say so: each type's document is a whole `BuildingSkeleton` authored at the origin, and nothing joined two of them.

## The two operations

| op | what it does | why it is not the other one |
| --- | --- | --- |
| `ops::translate(&mut skeleton, offset)` | moves every **plan** in a document: mass outlines, roof eaves and crests, slab outlines, a member's free ends, the site's walls | it moves geometry and **no index**: a host is still the same mass afterwards, which is what makes it composable |
| `ops::merge(&mut base, other) -> bool` | appends another document's skeleton and **re-bases every index it names**: hosts, anchors on members and faces, openings' faces, and the keys of the rhythm map | it moves **no geometry**: a merged document stands where it was authored, so the village says where each building is by translating it first |

**A merge refuses a document that places parts.** A placement names a template by index, a bay by slot and a pin
inside that template; re-basing it is a re-resolution against a grid, not a shift. `merge` answers `false` and
changes nothing, and a caller that wants parts places them after the merge.

## The arrangement

| building | where | why there |
| --- | --- | --- |
| the drum tower | north, facing down the square | a 侗寨's 鼓楼 is the centre of the village, and its square is the room in front of it |
| two 土掌房 | east and west, doors on the square | the flat-roofed house belongs to the same highland economy, and its 晒场 faces the light |
| two 木楞房 | south, the village's own edge | the log houses of a Naxi or Pumi village are on its edge |
| the paving | 16 × 12 at the centre, one grating | the piece of ground the four types share |

Identity: **4 229 rows** — five buildings and the paving, which is five times the identity of any one type and
still one document.

## What the village does not have, and the next scale

- **no ground of its own**: level earth and a paved square; a terraced village on a slope is a scene of its own,
  and the seating rules would have to be composed as carefully as the geometry;
- **no streets**: the square is bounded at four sides and open at two, and the lanes that run out of it are what
  `street` already knows how to build — the two scenes have not been joined;
- **no people, trees or well**: the square is the subject.

The next scale after the village is the **terrain**: the repository has `mountain`, `terrain`, `vegetation` and
`landscape`, and none of their work has ever had a village standing on it.