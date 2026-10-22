# 中国西南传统建筑 · 索引

This file is the **index** of the catalogue: **29 forms**, **15 settlements**, and **80 scenes** measured by
`cargo run -p whiting-gallery --example scene_identity_probe`. Every number below was read from that run, not typed by hand.

**Established**: 252 rounds of work; the gallery's own registers agree (`80 names, 80 of them buildings, 80 listed, unmeasured [], unknown []`;
`the walk asked for 80 scene(s) and 80 printed a table: true`).

## How a scene is registered

Four places, all of which must agree — the probe now **fails** when they do not (round 232):

1. `crates/gallery/src/lib.rs` — `pub mod <name>;`, a name in `Gallery::names()`, and an arm in `build`;
2. `crates/gallery/src/naming.rs` — `building_of`'s dispatch (a plate shares its document's arm);
3. `crates/gallery/examples/scene_identity_probe.rs` — the `SCENES` const, whose length is checked against the gallery;
4. the scene function itself, returning `crate::village::scene_of(...)`.

## The standing verdicts

These are the owner`s own rulings on how this work is done, kept here because every round is read against them:

- **Never render through the GPU.** A frame is path-traced on the CPU: `scene_document` writes the json, then
  `render_cycles --document <path> <out.png> [samples] [CPU]`.
- **The audit is `audit/audit.ps1`**; `audit/full-audit.ps1` runs only with `-OnOwnerPermission`. Warm runs stay **under 10 s**.
- **No plate/PNG hashing inside the audit**, and no frame comparison in it.
- **Examples and scenes are instruments, never swept or audited.** The identity probe is an instrument and not a gate.
- **Change what you touch and measure its closure**: the audit computes the change`s downstream, not every test.
- **Model a relation rather than overlapping two things**; land a change whole and merge it before starting the next.
- **A change that moves neither the frame nor the table is not a change.**

## The forms

A form is a building, a street or a town; several forms are also settlements, which compose them.


| # | type | file | scenes |
|---|---|---|---|
| 1 | 竹楼 (bamboo house on piles) | `bamboo_house.rs` | `bamboo_house` |
| 2 | 土掌房 (earthen flat-roofed house) | `earthen.rs` | `earthen_house` |
| 3 | 一颗印 (the seal-script court) | `yikeyin.rs` | `yikeyin`, `yikeyin_court`, `yikeyin_gable` |
| 4 | 白族三坊一照壁 | `bai_court.rs`, `bai_lane.rs` | `bai_court`, `bai_lane`, `bai_gate` |
| 5 | 纳西院 | `naxi_court.rs` | `naxi_court` |
| 6 | 穿斗 (through-tenon frame) | `chuandou.rs` | `chuandou_house` |
| 7 | 木楞房 (log house) | `log_house.rs` | `log_house` |
| 8 | 石板房 (slate house) | `slate_house.rs`, `slate_court.rs` | `slate_house`, `slate_court` |
| 9 | 屯堡街 | `slate_street.rs` | `slate_street`, `slate_gate`, `slate_end` |
| 10 | 苗族吊脚楼 | `miao_stilts.rs` | `miao_stilts` |
| 11 | 侗寨 · 鼓楼 · 风雨桥 | `dong_village.rs`, `drum_tower.rs`, `rain_bridge.rs` | `dong_village`, `dong_square`, `dong_axis`, `dong_water`, `drum_tower`, `rain_bridge` |
| 12 | 景颇寨 | `jingpo.rs` | `jingpo_village`, `jingpo_mouth` |
| 13 | 傈僳村 | `lisu.rs` | `lisu_village`, `lisu_yard`, `lisu_steps` |
| 14 | 羌碉 · 羌寨 | `qiang_tower.rs` | `qiang_tower`, `qiang_double` |
| 15 | 藏族碉房 | `tibetan.rs` | `tibetan_house` |
| 16 | 傣庙 · 傣寨 | `dai_temple.rs`, `hamlet.rs` | `dai_temple`, `dai_stupa`, `dai_hamlet` |
| 17 | 蘑菇房 | `mushroom_house.rs` | `mushroom_house` |
| 18 | 本主庙 | `benzu_miao.rs` | `benzu_miao`, `benzu_miao_court` |
| 19 | 风水塔 | `fengshui_tower.rs` | `fengshui_tower` |
| 20 | 水碾房 | `watermill.rs` | `watermill` |
| 21 | 盐井与盐灶 | `saltworks.rs` | `saltworks` |
| 22 | 马店 | `caravan_inn.rs` | `caravan_inn` |
| 23 | 龙窑 | `dragon_kiln.rs` | `dragon_kiln` |
| 24 | 牌坊 | `paifang.rs` | `paifang` |
| 25 | 戏台 | `opera_stage.rs` | `opera_stage` |
| 26 | 碉楼 | `watchtower.rs` | `watchtower` |
| 27 | 会馆 | `guild_hall.rs` | `guild_hall`, `guild_stage` |
| 28 | 场镇 (the market town) | `market_town.rs` | `market_town`, `market_shops`, `market_guild`, `market_west` |
| 29 | 井亭 (the covered well) | `well_pavilion.rs` | `well_pavilion` |

## The settlements

| # | settlement | file | what it composes |
|---|---|---|---|
| 1 | 山村 (mountain village, square and west) | `village.rs` | the square, the tower, the lane, the 戏台 |
| 2 | 苗族山寨 | `miao_village.rs` | 吊脚楼 rows and their yards |
| 3 | 侗寨 | `dong_village.rs` | tower, three houses, granaries, well, racks, arch, stage, bridge, mill |
| 4 | 景颇寨 | `jingpo.rs` | the gate and the lane |
| 5 | 傈僳村 | `lisu.rs` | the terraced yards |
| 6 | 羌寨 | `qiang_tower.rs` | the double tower |
| 7 | 藏族村 | `plateau.rs` | the 碉房, the 玛尼堆 and the prayer flags |
| 8 | 傣寨 | `hamlet.rs` | the hedge, the gate and the houses |
| 9 | 白族巷 | `bai_lane.rs` | three courts and the gate |
| 10 | 屯堡街 | `slate_street.rs` | the flags, the kerb, the culverts, the gates, the 文笔塔, the 碉楼 |
| 11 | 一颗印院落 | `yikeyin.rs` | four ranges and the court |
| 12 | 土掌房村 | `tuzhang_village.rs` | the terraces and the roofs |
| 13 | 盐村 | `salt_village.rs` | works at the head, two 土掌房, the store at the foot |
| 14 | 陶村 | `pottery_village.rs` | the kiln, the throwing shed, the woodpile, the drying floor |
| 15 | 场镇 | `market_town.rs` | stage, arch, ten shopfronts, tower, 会馆, 马店 |

## The measured table

Every scene and the rows its document holds.

| scene | rows |
|---|---|
| `chuandou_house` | 1169 |
| `caravan_inn` | 190 |
| `compound` | 6852 |
| `compound_court` | 6852 |
| `fengshui_tower` | 839 |
| `guild_hall` | 161 |
| `guild_stage` | 161 |
| `hall` | 77 |
| `dai_hamlet` | 5689 |
| `hall_large` | 109 |
| `identity` | 77 |
| `house` | 35 |
| `salt_village` | 737 |
| `salt_yard` | 737 |
| `saltworks` | 128 |
| `shed` | 66 |
| `stilt` | 2167 |
| `dai_temple` | 2807 |
| `dong_axis` | 10976 |
| `dong_water` | 10976 |
| `dong_square` | 10976 |
| `dong_village` | 10976 |
| `dai_stupa` | 2807 |
| `drum_tower` | 4141 |
| `dragon_kiln` | 111 |
| `earthen_house` | 284 |
| `bai_gate` | 6633 |
| `bai_court` | 1843 |
| `bai_lane` | 6633 |
| `benzu_miao` | 1917 |
| `benzu_miao_court` | 1917 |
| `bamboo_house` | 938 |
| `jingpo_mouth` | 2347 |
| `jingpo_village` | 2347 |
| `lisu_steps` | 1255 |
| `lisu_yard` | 1255 |
| `lisu_village` | 1255 |
| `log_house` | 168 |
| `village_west` | 18831 |
| `tuzhang_mouth` | 1873 |
| `tuzhang_village` | 1873 |
| `village_square` | 18831 |
| `hillside_village` | 6341 |
| `longhouse` | 721 |
| `market_guild` | 1167 |
| `market_shops` | 1167 |
| `market_town` | 1167 |
| `market_west` | 1167 |
| `miao_stilts` | 13166 |
| `miao_village` | 13166 |
| `mushroom_house` | 286 |
| `naxi_court` | 3619 |
| `opera_stage` | 93 |
| `paifang` | 136 |
| `plateau_village` | 3576 |
| `pottery_village` | 173 |
| `qiang_double` | 2191 |
| `qiang_tower` | 2191 |
| `rain_bridge` | 1638 |
| `stilt_gallery` | 2167 |
| `yikeyin_gable` | 6896 |
| `yikeyin` | 6896 |
| `yikeyin_court` | 6896 |
| `street` | 12610 |
| `street_arcade` | 12610 |
| `terrace` | 123 |
| `slate_court` | 344 |
| `slate_street` | 3150 |
| `slate_end` | 3150 |
| `slate_gate` | 3150 |
| `slate_house` | 260 |
| `tunpu_gate` | 1242 |
| `tunpu_village` | 1242 |
| `tibetan_house` | 451 |
| `watchtower` | 111 |
| `watermill` | 339 |
| `well_pavilion` | 74 |
| `workshop` | 162 |
| `wing` | 99 |
| `yard` | 108 |
