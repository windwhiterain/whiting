# Guizhou–Xiangxi minority dwellings: what structural and geometric rules define a procedural model

**Question.** What structural and geometric rules define the Guizhou–north-Guangxi–Xiangxi minority
dwellings — the Miao stilt house 苗族吊脚楼, the Dong timber house and drum tower 侗族木楼/鼓楼,
the Bouyei stone house 布依族石板房, the Xiangxi Tujia stilt house and corner tower 土家族吊脚楼/转角楼,
and the Tunpu stone courtyard 屯堡 — precisely enough to drive a procedural model, and which of the
current volume vocabulary's assumptions break on them?

**Slice.** Guizhou, north Guangxi, Xiangxi. Chongqing/Yunnan stilt houses and the old-town settlement
fabric belong to sibling notes: [chongqing-stilt-houses.md](chongqing-stilt-houses.md) and
[yunnan-dwellings.md](yunnan-dwellings.md). The frame and roof vocabulary itself is surveyed in
[chinese-timber-frame-and-roof.md](chinese-timber-frame-and-roof.md), whose terminology (立帖, 瓜柱,
穿枋, 雷公柱) this note uses without re-deriving.

**Date and access date for every source.** **2026-10-18.**

**Network condition, stated because it bounds the evidence.** The proxy was intermittent all session.
`web_fetch` reached several hosts (baike, `cctv.com`, `qdnrbs.cn`, `thepaper.cn`, county and provincial
government sites) and failed on others (`zh.wikipedia.org`, `en.wikipedia.org`, `core.ac.uk` 403,
`museum.zhaiu.com` 503, `aho.brage.unit.no`, `baike.baidu.com` 403 on the desktop domain);
`hsfwest.com`, `aipub.cn` and `kjhcy.org` returned HTTP 200 with an empty body;
`urbandesign.tsinghuajournals.com` returns `application/x-download`, which the fetch tool rejects. Both
PowerShell `Invoke-WebRequest` routes failed SSL (direct and via `127.0.0.1:7897`), but `curl.exe -k`
succeeded at roughly 25 KB/s and is how two full-text academic PDFs were obtained and read.
**Consequence:** the bibliographic record below is much stronger than the verified-measurement record.
Every number carries the tag of the source that printed it, and a number that exists only in one
`[unverified]` page is marked as carrying no structural weight.

**Standing rule applied throughout.** No number appears below unless a source printed it. Where a
number is widely repeated but uncorroborated it is marked as such rather than dropped, because a
repeated-unverified claim is itself a finding.

**Honest headline on what this session produced.** Two full-text academic/survey documents were read
end-to-end and they cover the **Dong** building completely and quantitatively. For the **Miao**, the
**Bouyei**, the **Tunpu** and the **Xiangxi Tujia**, this session obtained **qualitative structural
vocabulary and almost no measured dimensions**. Two of the most important numbers in the brief —
the **吊脚楼 roof pitch** and the **石板瓦 roof pitch** — were **not established**, and no substitute
has been invented for them. That is the largest single limitation of this note.

---

## Verdict

These dwellings are not buildings that happen to be made of timber and stone; they are **stacked
post-and-beam frames standing on ground of unequal height**, and the terrain is the first design act.
The current vocabulary — a wall ring plus plates, everything polygon-offset in plan, every roof a FLAT
slab — fails in six specific places, each of which is a named missing construct rather than a missing
feature.

1. **A post's length is a terrain query, not a plan offset.** A stilt house is not a two-storey
   building on stilts; it is a *single-storey frame translated vertically* so its floor clears an
   existing slope, and the only thing that varies is how long each post is. Huang Caigui 黄才贵
   records this as a typological constant in both ethnic groups he surveyed: main posts "以地面直到
   顶屋" — they run from the ground all the way to the top of the building [S1]. Nothing in a
   wall-ring-plus-plate vocabulary can express a member whose length is `terrain(x,y) + parameter`.
2. **There are two grids, not one, and their ratio is a rule.** For the Dong long house, "建筑开间
   一般按立帖内的柱距计算，使用开间则为建筑开间的两倍" — the *structural* bay is the post spacing
   within one frame, and the *used* bay is **two** structural bays [S1]. The famous 平流寨 "7间屋" is
   therefore **14 structural bays**, excluding the two side wings [S1]. A generator that stores one
   `bay_width` and a room count is wrong before it draws anything.
3. **The roof is a purlin frame, and the frame is the design object.** 穿斗 chuāndòu "threading" frames
   of columns, blocks and tie-beams stand *in front of* the enclosure and carry the roof; walls are
   infill hung on them. In the Bouyei stone house this is stated outright — "房屋的承重结构是穿斗式
   木构房架，石料只作围护结构和屋面防水材料" [S21]. A flat slab is the wrong noun for any of this.
4. **The settlement piece is the primary object, not the house.** 鼓楼 gǔlóu, 风雨桥 fēngyǔqiáo,
   粮仓 liángcāng, 凉亭 liángtíng and 禾晾 hélàng are catalogued and protected **in their own right**
   by the provincial planning instrument, in a separate category from dwellings [S3]. A generator that
   emits one house per plot cannot produce a 侗寨.
5. **The three-level stack is a rule, and the middle storey's height is driven by the hearth.** Dong
   bedroom clear heights are given as **2–2.5 m**; the 火塘 huǒtáng room "常达3．5米以上，并且不设
   顶棚" — exceeds 3.5 m and has **no ceiling at all** [S4]. A single `storey_height` parameter cannot
   produce this; the hearth bay is a double-height slot through the middle storey.
6. **Stone is a *roof covering* in one sub-family, and that changes the roof law, not the wall law.**
   The Bouyei 石板房 shíbǎnfáng covers the same frame with sawn slate slabs laid lapped and staggered
   — "石片在屋面形成自然的弧线，利于排水，而不用像瓦片屋顶那样留出排水沟" [S21] — over a roof that
   keeps no drainage channels at all, because the slabs themselves overlap to form the curve. Slate is
   heavier than clay tile, so purlin spacing is a function of the covering [S9].

**Which constructs a generator must have, in order:** (a) a post whose length is a function of a
terrain field; (b) two distinct grids — structural bay and used bay — plus their ratio; (c) an exposed
frame as the primary solid, with walls as infill hung *on* it; (d) a pitched purlin-framed roof as a
sweep rather than a slab; (e) a settlement-level generator placing towers, bridges and granaries by
village rules rather than by plot; (f) an infill panel system with per-material thickness and per-material
corner treatment; (g) a material-dependent roof-covering law (interlocking tile vs. lapped slate).

---

## The parts and their correspondence

### 2.1 The vocabulary, with 汉字 + pinyin + gloss

Frame.

| term | 汉字 | pinyin | gloss / what it is |
| --- | --- | --- | --- |
| 干栏 (干阑) | 干阑式 | gānlán | pile dwelling; the generic term for the stilt-house family. Baidu's entry is explicit that 吊脚楼 is a *half* pile dwelling, since a true pile house is wholly suspended: "干栏应该全部都悬空的，所以称吊脚楼为半干栏式建筑" [S22] |
| 立帖 / 排扇 | 立帖，排扇 | lìtiě, páishàn | one transverse timber frame. 立帖 is the Dong/Miao word; 排扇 is the Tujia/Huizhou word for the same member [S1][S22] |
| 开间 | 开间 | kāijiān | bay. **Ambiguous in the sources**: it means both the structural bay (post spacing) and the used bay, and the two differ by a factor of two for the Dong [S1] |
| 柱距 | 柱距 | zhùjù | post spacing |
| 瓜柱 (瓜, 骑) | 瓜柱，骑 | guāzhù, qí | block post / short post above a column carrying a purlin. Transcribed 骑 qí in the Tujia "五柱六骑" idiom [S22] and 瓜 guā in the Bouyei "五柱四瓜" idiom [S21]; **these are the same member and the same count under two transcriptions** |
| 穿斗 | 穿斗式 | chuāndòu | "threading" frame construction: purlins pass *through* mortises in the columns rather than sitting on a wall head |
| 穿枋 | 穿枋 | chuānfáng | the tie beam that threads the columns of one frame together |
| 檩 | 檩 | lǐn | purlin |
| 椽 | 椽 | chuán | rafter |
| 叉首 | 叉首式 | chāshǒu | king-post strut rising to the ridge |
| 雷公柱 | 雷公柱（尖柱） | léigōngzhù (jiānzhù) | apex post at the top of a polygonal tower roof [S1] |
| 悬柱 | 悬柱 | xuánzhù | the hanging post; octagonal or square section, "下垂底端常雕绣球和金瓜" [S20] |

Site.

| term | 汉字 | pinyin | gloss |
| --- | --- | --- | --- |
| 吊脚楼 | 吊脚楼 | diàojiǎolóu | stilt house. Named for its construction, not its look: "因前檐柱吊脚，故而得名" — because the front-eaves columns are the stilted ones [S20] |
| 吊脚柱 | 吊脚柱 | diàojiǎozhù | the stilt post. Definition given as "每排柱的最外一根自上而下截齐上屋基处" — the outermost post in each row, cut down from the top to meet the upper house base [S20] |
| 挖填方 | 挖填方 | wātiánfāng | cut-and-fill. The site is cut into platforms and the offcut is used to build up others |
| 保坎 | 保坎 | bǎokǎn | retaining kerb wall holding a cut platform; stone, and "特别讲究" — the stonework is a status marker [S1] |
| 接柱建竖 | 接柱建竖 | jiēzhù jiànshù | lower-storey and upper-storey posts are **separate** posts. Dong; the older, "primitive" form [S1] |
| 通长柱建竖 | 通长柱建竖 | tōngchángzhù jiànshù | one post runs from the ground through the lower floor to the upper frame — the through-column form. The evolution from the first to the second is "由古代全木结构…逐渐发展" [S1] |
| 半边楼 / 半吊脚 | 半边楼 | bànbianlóu | half-hanging: the slope is cut into **two** levels, long posts on the lower, short posts on the upper, so that the front half of the floor is level with the back half's ground [S20] |
| 全吊脚 | 全吊脚 | quándiàojiǎo | fully stilted. The selection rule, from a local carpenter: "看地基建筑，宽敞的屋基可全吊脚，屋基有限的只半吊脚" — it depends on the site, not on the ethnic group [S6] |

Plan and rooms.

| term | 汉字 | pinyin | gloss |
| --- | --- | --- | --- |
| 堂屋 | 堂屋 | tángwū | central hall; ancestor shrine, banquets, weddings and funerals. The centre of gravity of the plan [S7][S22] |
| 火塘 | 火塘 | huǒtáng | the hearth; cooking, heating, family gathering, and the ancestor seat in Dong belief. It sets the height of the room it is in [S4] |
| 廊 | 廊 | láng | the veranda/corridor. In Dong plan-hierarchy it is **the largest single space**: "走廊往往是平面中面积最大者" [S4]. It is also the drying floor, the work floor, the reception floor and the place where unmarried adults of the patrilineage sleep [S1] |
| 美人靠 | 美人靠 | měirénkào | the outward-bulging S-curved bench-back rail on the corridor outside the hall. Miao names differ across sources — 嘎息 (ghab xil) [S22], 阶息 / 干席 [S6], 阶息站 [S6]; **sources disagree and none is a survey transliteration** |
| 火铺 | 火铺 | huǒpù | Tujia hearth-side sleeping platform, hung over the fire pit [S22] |
| 吞口 | 吞口 | tūnkǒu | the sunken entrance vestibule of the 地房 "一字型" house type [S3] |
| 退堂 | 退堂 | tuìtáng | the hall set back from the front line; **not established this session** — the term appears in the brief but was not found in any source read |
| 偏厦 | 偏厦 | piānxià | side wing under the same roof. The Dong long house has fixed 偏厦间 on **both** sides [S1] |
| 转角楼 | 转角楼（走马转角楼） | zhuǎnjiǎolóu | corner tower; an L-shaped wing that turns the corner and carries a walk-around corridor. Named as a protected type in the provincial catalogue [S3]; "单转角楼" and "双转角楼" occur as a wealth gradation [S22] |
| 天楼 | 天楼（板楼 / 条楼） | tiānlóu (bǎnlóu / tiáolóu) | the top storey: 板楼 is plank-floored and used for grain cabinets, 条楼 is split-bamboo-floored with gaps so smoke from the fire room below escapes, and stores corn cobs [S22] |
| 阁楼 | 阁楼 | gélóu | the usable space inside the gable triangle of the roof frame; used for temporary guest rooms at festivals, storage otherwise [S4] |

Roof.

| term | 汉字 | pinyin | gloss |
| --- | --- | --- | --- |
| 悬山 | 悬山顶 | xuānshān | overhanging gable: the ridge is parallel to the eaves and the gable overhangs them. The most common roof for the Miao stilt house: "屋面多为悬山顶，也有部分歇山顶" [S7] |
| 歇山 | 歇山顶 | xiēshān | half-hipped; cited for the Miao: "吊脚楼为歇山顶穿斗挑梁木架干栏式楼房" [S20] |
| 硬山 | 硬山式 | yìngshān | flush gable, no overhang — attributed to the 安顺 region for stone houses [S21] |
| 出檐 | 出檐 | chūyán | eave projection. **Length: not established for any type in this slice** — see §7 |
| 披檐 / 腰檐 | 披檐，腰檐 | pīyán, yāoyán | lean-to eave / mid-level eave. Named as 披檐式 in the provincial catalogue via "歇山顶敞廊干栏式建筑"; **reach not established** |
| 攒尖顶 | 攒尖顶 | cuánjiāndǐng | polygonal pyramidal roof; the standard drum-tower crown [S1] |
| 青瓦 | 青瓦 (小青瓦) | qīngwǎ | small clay tile; the standard covering in 黔东南 [S7] |
| 石板瓦 | 石板瓦 / 石片 | shíbǎnwǎ | slate-slab covering; see §3.4 |
| 封火山墙 | 封火山墙 | fēnghuǒshānqiáng | firewall gable wall: "部分墙体高出屋顶" [S23] |

Settlement.

| term | 汉字 | pinyin | gloss |
| --- | --- | --- | --- |
| 鼓楼 | 鼓楼 | gǔlóu | drum tower; the village's meeting hall, council chamber and record ("寨胆和寨魂") [S5][S25] |
| 密檐 | 密檐 | mìyán | dense eaves; a tower whose storeys are nearly all eave [S1] |
| 风雨桥 | 风雨桥 | fēngyǔqiáo | covered bridge; a bridge with a roofed corridor and towers [S4] |
| 粮仓 / 谷仓 | 粮仓，谷仓 | liángcāng, gǔcāng | granary; separately placed from the dwelling, "每一个谷仓修建在村寨的外围" — on the village periphery [S1] |
| 禾晾 | 禾晾 | hélàng | drying rack for sheaves; a separate protected building type [S3] |
| 碉楼 | 碉楼 | diāolóu | watchtower; Tunpu |
| 屯堡 | 屯堡 | túnbǎo | fortified military-colony village, from 明洪武 "调北征南" / "调北填南" [S8][S23][S26] |
| 屯墙 | 屯墙 | túnqiáng | colony wall |

### 2.2 What sits on what — the carrying relationships

**The frame carries everything; the wall carries nothing.** This is stated for the Bouyei stone house
in the plainest terms: the four walls "看似牢固的墙体并不承重，而是木构架支撑着楼板和房顶，以
传统的立巾屋架为主，单排屋架由柱、瓜柱和穿枋以卯榫连接组成" [S21], and restated: "石料只作围护
结构和屋面防水材料" [S21]. It holds for every type in this slice, including Tunpu, where the apparent
massive stone exterior of 镇山村 is described as "以穿斗式木构架为骨、石板为肤" — the timber frame is
the bone, the slabs are the skin [S24].

**Post → footing.** Every post lands on something that is not the terrain surface: a stone plinth
(基石) inside a pond-like excavated base, a stone kerb 保坎 [S1], a 土建 platform, or bare rock. Where
the base is rock, the post is founded directly on the rock and its length is the whole hanging depth —
**the depth is a terrain query, not a parameter** (see §7; the specific rock-founding detail in the
brief was not corroborated by any source read this session).

**Post → purlin.** In 穿斗 construction the post is *drilled through* by the purlin rather than
carrying it on a wall head. The Dong tower frame is described with unusual precision: main posts "多以
4 根（也有1 根、6 根、8 根的）粗大的杉木作主承柱，以地面直到顶屋…是用穿枋将其连接成一个长筒形
的内柱网；而檐柱一般取12 根（也有8 根、14 根的），用圆枋连接成外柱网；内、外柱网再用穿枋、
瓜柱、瓜枋穿斗组合成小屋顶内收的构架" [S1]. That is a **two-ring column net that steps inward at each
eave**, with the number of posts in each ring given as a small set, not a single value.

**Purlin → rafter → covering.** The rafter and covering layer is what differs most between the tile
roof and the slate roof, and the difference is not cosmetic: the slate version has no channels, the
tile version does [S21].

**Floor → post.** The floor plane is *one* plane. Everything below it is the varying gap between the
floor and an undisturbed slope. This is the single most important geometric fact about the type and it
is why "wall ring plus plates" cannot express it: there is no second enclosure, no lower storey walls,
no lower storey slabs. The lower level is **air**.

**Infill → frame.** In the timber types, the wall is boarding (木板墙) set into the frame; in the stone
type, the wall is either 壁头墙 — rectangular slabs "3 厘米左右厚" let into the gaps between column and
tie-beam and pinned with cut nails driven half-way in, head out — or 砌墙 — random slabs laid up like
bricks to about "40 厘米左右厚" [S21]. Two different thicknesses, two different fixings, two different
corner details. A single `wall_thickness` cannot serve both.

### 2.3 The three-level stack and what each level is for

The stack is a **rule with a fixed content assignment**, not a storey-count parameter:

| level | content | evidence |
| --- | --- | --- |
| bottom | livestock byres (猪牛圈鸡笼), grain stores, farm tools, sometimes a stone mill 碓 | [S6][S7][S22] |
| middle | living; 堂屋 central, 火塘 beside it, bedrooms, 美人靠 outside the hall | [S6][S7][S22] |
| top | grain and seed; sometimes subdivided into 3–4 guest rooms and a children's study [S6]; in the Tujia version the top is the 姑娘楼 girls' floor [S22] | [S6][S22] |
| a true attic | the void inside the gable triangle of the roof frame, used for festival guest rooms and otherwise for storage | [S4] |

The Dong long house adds a fourth: 谷仓 at the top, and the granary is **staggered in plan against**
the hearth room and the bedroom below — "谷仓与火塘间和卧室间上下错位" [S1]. That staggering is a
load-path decision, not a space decision: the granary's dead load lands on the 偏厦, not on the
occupied rooms.

### 2.4 The 退堂/吞口 and 美人靠/挑廊 question

- **美人靠 is confirmed** and is the most reliably reported single element: it is installed on the
  outside of the middle storey, at the hall, "装上围栏竖条，以稍向外凸出的靠背栏杆" — vertical balusters
  with a back rail bulging outward [S6]. Its Miao name is reported four different ways across four
  sources (嘎息/ghab xil, 阶息, 干席, 阶息站) and **no source read here is a linguistic survey**, so the
  romanization is **not established**.
- **挑廊** (projecting corridor) is functionally the same member seen from the outside; the Dong
  equivalent is 廊 láng and the plan role is confirmed [S4][S1].
- **吞口** appears as a **type name**, not a room: "'一字型'地房吞口屋建筑" is one of the enumerated
  地房 types in the provincial catalogue [S3]. Its geometry is **not established**.
- **退堂** was **not found** in any source read this session. It is named in the brief and it is
  plausible, but no source here carries it.

---

## Numbers

Every cell carries the tag of the source that printed it. `not established` means no source read this
session printed the number — not that the number is unknown to the literature.

### 3.1 Dong 干栏 dwelling (侗族木楼) — the best-evidenced section

| quantity | value | source |
| --- | --- | --- |
| most common column grid | **二十柱四排三间** — twenty posts, four rows, three used bays: "传统的侗族民居大部分是用杉木建成的楼房，以二十柱四排三问的房子较多" | [S4] **[academic]** |
| structural bay | the post spacing inside one 立帖 frame; the unit in which 开间 is measured | [S1] **[academic]** |
| **used bay ÷ structural bay** | **2** — "使用开间则为建筑开间的两间，也就是两间建筑开间组合成1 间使用开间" | [S1] **[academic]** |
| worked example | 平流寨's "7间屋" = **14 structural bays** in the main block, plus the two 偏厦 | [S1] **[academic]** |
| bedroom clear height | **2–2.5 m** — "卧室的空间高度最小，多在2—2．5米" | [S4] **[academic]** |
| hearth-room height | **> 3.5 m, no ceiling** — "高度常达3．5米以上，并不设顶棚" | [S4] **[academic]** |
| corridor role in plan | largest single space: "走廊往往是平面中面积最大者" | [S4] **[academic]** |
| storey stacking | upper and lower posts separate ("接柱建竖") **or** one post through both floors ("通长柱建竖") | [S1] **[academic]** |
| outward stepping | "侗族的多层楼房还以挑梁吊柱的方式层层挑出" — cantilever tie-beams with hanging posts, each level projecting over the one below, shading and enlarging the floor above | [S4] **[academic]** |
| roof form | 悬山 or 歇山, double-pitch drainage: "多为双坡排水悬山或歇山屋顶" (Bouyei wording, same frame family) | [S21] **[unverified]** |
| column material | 杉木 (Chinese fir) throughout: "传统的侗族民居大部分是用杉木建成的楼房" | [S4] **[academic]** |
| upper-storey load | "多利用坡屋顶三角形屋架的中部空间，构成一个暗层" — the attic is part of the *structure*, not a floor plate | [S4] **[academic]** |
| roof pitch | **not established** | — |
| eave projection 出檐 | **not established** as a length; it is named as one of the four things the 掌墨 marks on the 总丈杆 — "柱高、面阔、进深、出檐尺寸" — so it is a first-class dimension, but no value was printed | [S4] **[academic]** |

The last row is worth dwelling on, because it is a *negative* result with a positive use: the
dimensions a 掌墨 zhǎngmò (master carpenter) actually sets are exactly **column height, bay width,
depth, eave projection**, and they are carved onto the 丈杆 zhànglán (the surveyor's rod) in the field,
from the site measurement alone, with no drawing [S4]. A generator's parameter list is already
determined by the craft; the rod types are 总丈杆 (overall: bay width, depth, column height) and
分丈杆 (part rods: 檐柱丈杆 eave-post rod, 金柱丈杆 centre-post rod, 明间面宽丈杆 central-bay rod,
次间面宽丈杆 side-bay rod) [S4].

### 3.2 Dong drum tower (鼓楼) — and the one place the sources flatly contradict each other

**Plan and frame** [S1] **[academic]**, National Museum of Ethnology survey:

| quantity | value |
| --- | --- |
| plan | **even-sided polygon**: "其平面均为偶数边形，一般有正方形、6 边形、8 边形" — square, hexagonal or octagonal |
| crown | 悬山, 歇山 or multi-pitch 攒尖; within the 攒尖 there is a further split into 叠顶 and 歇山暗顶 |
| main load-bearing posts | **4**, sometimes 1, 6 or 8 — 粗大的杉木, from the ground to the top of the building |
| inner post ring | the main posts joined with 穿枋 into "一个长筒形的内柱网" — a long cylindrical net |
| eave posts | **12**, sometimes 8 or 14, joined with 圆枋 into an outer post net |
| above the post nets | 穿枋 / 瓜柱 / 瓜枋 threading, forming a frame that steps **inward** at each eave ("小屋顶内收的构架") |
| apex | a cross-shaped 十字枋 on the main posts carrying 雷公柱 (aka 尖柱); in some cases the bracket sets act as a 井干式 timber-grate frame and in others they bear directly — but the bracket sets are decorative in the first case only, and the source is careful to say so |

**Height and eave count — the conflict.** Three sources, three answers:

| source | claim | tag |
| --- | --- | --- |
| [S1] 黄才贵, 2010, National Museum of Ethnology survey | "立面均为奇数重檐，多则17 层，最高20M 左右" — odd number of eaves, at most 17, height about 20 m | **[academic]** |
| [S4] 三江侗族建筑的科技人类学考察 | "檐的层数一般五层，七层，也有九层，多至二十七层的…楼高一般十至四十多米" — 5, 7, 9 eaves, up to 27; height generally 10–40+ m | **[academic]** |
| [S5] CCTV, 2025, on 肇兴侗寨信团鼓楼 | "共有11 层，高24.37 米，每层的截面逐渐收缩" — 11 storeys, **24.37 m**, each section contracting | **[unverified]** |

**Which do I trust, and why.** [S1] and [S4] are both field-based and both read in full; [S5] is a
broadcast feature and its single figure is the most specific. The disagreement is not necessarily an
error — [S1] surveyed 增冲寨 and the 肇兴 group and may be describing a local norm, [S4] is describing
what a 三江 craftsman said he can build, and 信团鼓楼 is a *specific named building* that is
contemporarily cited. **What is safe:** the even-sided polygon plan, the odd eave count, the two-ring
column net with an inward step per eave, the post counts (4 / 12) and their small variant sets, and the
**qualitative** law that each storey's section contracts upward. **What is not safe:** any single
"typical" height or eave count. A generator should take *eave count as an odd integer parameter* and
*height as derived from the taper*, not as a fixed typical.

Also fixed by [S1]: 鼓楼 is classed into 干栏式 / 密檐式 / 厅堂式 (plus 楼阁式 and 门廊式 in the fuller
list) [S1]; and [S4] independently gives "厅堂式、干栏式和密檐式" for 三江. **The two independent
classifications agree on 干栏式 and 密檐式 and disagree on the third term** — a small negative result.

Counts as a settlement fact: 三江县境内共有**鼓楼108座** [S4]; 黎平县 "各类鼓楼429座" [S5].

### 3.3 Dong 风雨桥 (程阳风雨桥, 永济桥) — the best-measured structure in this note

This is a 国保 (national protected) building with published dimensions, and every figure below is from
one full-text thesis that cites a repair record [S4] **[academic]**.

| element | value |
| --- | --- |
| built | 1911; published as a national protected cultural relic **1982-02-23** |
| bridge deck total length | **77 m** |
| deck width | **3.8 m** |
| overall height (顶高) | **11 m** |
| stone piers, height | **9.8 m** after repair; **7.8 m** originally — raised during the repair, and the rubble core was replaced with concrete, faced back in bluestone to preserve appearance |
| spans | **four**, each clear span **14.8 m** |
| pier spacing | **17.3 m** |
| pier form | hexagonal prism ("六棱柱体"), bluestone skin, rubble core, **3 % batter (收分)** per unit height, with a sharp angle facing upstream and downstream to split the flow |
| pier head timber frame | two rows of **nine** long fir timbers each, pinned into one frame, cantilevering **2 m** to each side in two symmetric "balance" layers |
| roofed part, total length | **81.9 m** (i.e. **longer than the 77 m deck** — the corridor oversails both abutments) |
| towers on the deck | **five**, all seated on the stone piers |
| tower roofs | all **four-tier 重檐**; the centre tower is a **hexagonal 攒尖** rising more than **7 m** above the deck; towers two and four are four-tier, four-sided 攒尖 with a gourd finial, **7 m** high; the end towers are **three-tier 重檐歇顶** |
| corridor width (general rule) | **3–4 m**, with four rows of columns along the length, **抬梁式** (post-and-lintel) construction |
| under-rail eave | a further projecting eave, **about 1 m or a little more**, below the balustrade |
| bridge count as a settlement fact | 三江县境内 **115** 风雨桥 |
| fixings | no iron nails anywhere; 榫卯 throughout; the 椽 were fixed with **bamboo nails** treated in tung oil |

**Rule worth carrying, not just numbers:** "层层出挑的密布式悬臂托架和简支梁复合体系" — a
**multi-layer corbelled cantilever bracket system compounded with simply-supported beams** is what
increases pier span while absorbing the dead weight [S4]. That is a construct, and the vocabulary has
no corbel.

### 3.4 Miao stilt house (苗族吊脚楼)

| quantity | value | source | tag |
| --- | --- | --- | --- |
| ground slope the type is built for | **30–70°** — "选在30-70度的斜坡陡坎上" | [S6] | **[unverified]**, but independently stated as 30–70° by the 中南民族大学民族学博物馆 account [S20] — *two sources agree, neither a survey* |
| frame height | **6–7 m**, the house standing on a stone 保坎 "数米高" | [S20] | **[unverified]** |
| corridor bay width | "间宽两米左右" — about **2 m** for the access corridor | [S6] | **[unverified]** |
| posts per frame row | **5–7** — "吊脚楼采用穿斗式结构，每排房柱5至7根不等" | [S22] | **[unverified]** |
| posts per row (alternative transcription) | **4** in the 4排扇3间 house or **6** in the 6排扇5间 house; 中等人家 5柱2骑 / 5柱4骑; 大户 7柱4骑 | [S22] | **[unverified]** — note these two rows do not agree, and both are [unverified] |
| site preparation, 半边楼 | the slope is cut into **two** platforms; **long** posts on the lower, **short** posts on the upper; the front half of the floor is then level with the back half's ground | [S20] | **[unverified]** |
| site preparation, worked numbers | slope cut into two levels, each level's **进深 6 尺多** (≈ 2.0 m), each level's area **约100 m²**, the two levels differing by **4 尺多** (≈ 1.33 m); gable walls and outer uphill mass in stone | [S22] | **[unverified]** — the only "typical" dimension set found for the Miao type, and it is a single uncited encyclopedia figure. **Do not treat as measured** |
| used bays | 3, 4 or 5 — "开间三间、四间、五间均有" | [S7] | **[unverified]** |
| base module | 三间四立帖 or 三间两偏厦 | [S22] | **[unverified]** |
| frame type | 穿斗式, 榫卯 only, "不用一钉一铆"; the 穿斗 advantage stated as "用小材可以盖大房" — small timber spans a large house | [S6][S7] | **[unverified]** for the type, **[academic]**-adjacent as a widely repeated claim |
| corner post | must be 枫木 (Chinese maple), because the maple is the clan's totem | [S22] | **[unverified]** |
| hanging depth as a function of slope | **not established.** No source read gives a numeric relation between ground slope and hanging depth. The *qualitative* rule exists — the deeper the slope, the further the frame is cantilevered out — but it is not quantified anywhere here | — | — |
| roof pitch (rise:run and degrees) | **not established** | — | — |
| eave overhang 出檐 | **not established** | — | — |
| 披檐 / 腰檐 reach | **not established** | — | — |

### 3.5 Bouyei stone house (布依族石板房) — the roof-covering contrast

The stone-slate covering is the one thing in this slice where a contractor's practice note and a
published feature disagree on a number, which is itself useful.

| quantity | value | source | tag |
| --- | --- | --- | --- |
| wall height | **5–6 m** | [S21] | **[unverified]** (Baidu Baike, reproducing a 贵州省人大-published feature [S24-family]) |
| structural system | 穿斗 timber frame carries the floor and the roof; the stone is enclosure and waterproofing only | [S21] | **[unverified]** in the source, but the rule is the same one an [academic] source states for the timber types [S1][S4], so it is treated as sound |
| frame module | 立帖 roof frame of **三柱二瓜 / 五柱四瓜**, and **七柱六瓜** for deep plans | [S21] | **[unverified]** |
| plan type | **一明一次间** (one main bay, one side bay) or **一明两次间** (one main, two side) — "布依族的建筑平面比较简单" | [S21] | **[unverified]** |
| wall type A: 壁头墙 | slabs of about **3 cm** thickness let into the gaps between 柱 and 枋, pinned with iron nails driven half in and bent over | [S21] | **[unverified]** |
| wall type B: 砌墙 | random flat slabs laid like bricks, about **40 cm** thick | [S21] | **[unverified]** |
| available stone, thin | **1.5–5 cm** thick flag ("片石", locally 合硼石) — this is the roof material | [S21] | **[unverified]** |
| available stone, thick | **50–60 cm** flag, and **50–60 cm** blocks | [S21] | **[unverified]** |
| **slate thickness used on the roof** | **≈2 cm** — "每块石片的厚度为2厘米左右，高低叠压，错落有致，宛若鱼鳞" | [S21] | **[unverified]** |
| **contradiction** | a Guizhou contracting firm states the working range as **1–3 cm** and specifies a **minimum lap of 10 cm**, 2–3 fixings per slab, and a bamboo or timber bedding layer of 杉木皮/竹席/茅草 beneath | [S9] | **[unverified]**, and the page itself says 【文章内容来源网络仅供参考交流使用】. I trust [S21]'s ≈2 cm over [S9]'s range for *as-built*, because [S21] describes a specific tradition and [S9] describes a contractor's spec |
| roof form | **悬山顶, double-pitch drainage** for most; **硬山式** attributed to the 安顺 region, **悬山式** to the 贵阳 outskirts | [S21] | **[unverified]** |
| **how the roof sheds water** | "石片在屋面形成自然的弧线，利于排水，而不用像瓦片屋顶那样留出排水沟" — the slabs' own lapping forms the drainage curve; the slate roof has **no drainage channels**, unlike the tile roof | [S21] | **[unverified]**. This is the single most important *geometric* difference from tile and it should survive into the generator |
| ridge | **no ridge tile**: one side's slabs are run out over the other side's and pressed down, then a course of tidy slabs is laid along the ridge like tiles | [S21] | **[unverified]** |
| **roof pitch, rise:run and degrees** | **not established.** No source read gives a pitch. The only directional statement is "坡度宜偏陡" — pitch should be steep — from the contractor page [S9], which is advice, not a survey | — | — |
| maintenance cycle | slate roofs re-laid about **every 10 years**, weathered slabs replaced | [S21] | **[unverified]** |
| plinth | the uphill rear stands **30–50 cm** above grade; the downhill front plinth doubles as a retaining wall, with rubble or flag outside and rammed earth fill inside | [S21] | **[unverified]** |
| doors and windows | openings small, high thresholds; window heads pointed-arch, round-arch or stone-lintelled | [S21] | **[unverified]** |
| gable ornament | "龙口" at the eave of the gable wall | [S21] | **[unverified]** |
| 镇山村 | 布依 village on the 花溪 reservoir peninsula, founded in the 明万历 reign, **three sides surrounded by water**, streets paved in bluestone; **上寨** was the Ming garrison quarter and is enclosed by 屯墙, **下寨** is the Bouyei residential quarter, and the two are divided by the 屯墙; entered the 7th batch of 中国历史文化名村 in **2019** | [S24] | **[unverified]** (newspaper feature on a provincial-assembly site) — but the 上寨/下寨 split by 屯墙 is a *settlement* fact of the kind that only appears in local writing |

### 3.6 Tunpu (屯堡) — a military colony in stone

| quantity | value | source | tag |
| --- | --- | --- | --- |
| **碉楼** height | **10–15 m** | [S23] | **[unverified]** (party-government account on 澎湃) |
| 碉楼 storeys | **3–5** | [S23] | **[unverified]** |
| 碉楼 openings | "射击孔", "瞭望窗" on each storey; a 烽火台 on the top | [S23] | **[unverified]** |
| 碉楼 siting | on the colony's high points or at lane junctions | [S23] | **[unverified]** |
| material | walls in bluestone, roof in 青瓦, ground paved in bluestone slabs | [S23] | **[unverified]** |
| firewall gable | "部分墙体高出屋顶，形成'封火山墙'" | [S23] | **[unverified]** |
| lane geometry | trunk lanes **straight** for troop movement; secondary lanes **丁字形 / 回字形** to make a defensive maze | [S23] | **[unverified]** |
| courtyard type | 四合院 aligned on the Jiangnan 天井院 type, with a drain at the centre of the court producing 四水归堂 | [S23] | **[unverified]** |
| 云山屯 colony wall | about **6 m** high, over **ten 里** long; **14** sentry platforms (碉堡) at key positions | [S8] | **[unverified]**, and the source text is visibly OCR-damaged |
| 云山屯 founded | **1381**, 明洪武十四年 | [S8] | **[unverified]** here; the 调北征南 campaign itself is corroborated by [S23][S26] |
| 本寨 碉楼 count | **seven** surviving high 碉楼 in coarse dressed stone | [S8] | **[unverified]** |
| 明清一条街, 云峰八寨 | total length about **600+ m**, width about **3–5 m**, fully paved in bluestone | [S8] | **[unverified]** |
| the settlement's own maxim | "家自为塾，户自为堡，倘贼突犯，各执坚以御之" | [S8] | **[unverified]** |
| **wall thickness, floor heights, roof pitch, 碉楼 plan** | **not established** | — | — |

### 3.7 Granaries, and the size of a village's stock of them

One measured inventory, from [S1] **[academic]**: "在高巨洞察 30m 宽的末面上，有 **52 座**不同构架的
干阑谷仓组合的一大谷仓群。这小群仓中属于 **1 间**类型的有 **41 座**，其余的 **11 座**属于 **2 间**类型。
1 间的很明显属于1 户小体家庭所有，2 间的则属于两户素居家庭所共有。" Granaries are sited on the
village periphery. Four frame types are distinguished [S1]:

1. columns, 瓜 and 枋 in 穿斗, meeting at the apex;
2. two linked short posts propping the head 枋, forming a 蜀柱;
3. short posts with raking struts either side forming a triangular 叉首 (king-post) beam;
4. as (3) with a ridge member added on top, where the struts carry no load.

### 3.8 Wall thickness by material — the comparison the frame actually needs

| material | thickness | tag |
| --- | --- | --- |
| slate-slab infill (壁头墙), Bouyei | **≈3 cm** | [S21] [unverified] |
| random-slab wall (砌墙), Bouyei | **≈40 cm** | [S21] [unverified] |
| timber board wall | **not established** | — |
| Tunpu bluestone wall | **not established** | — |
| note | the two Bouyei wall types differ by a factor of **thirteen**, and the 40 cm wall is a *cavity* wall laid like masonry rather than an infill panel | [S21] [unverified] |

### 3.9 Stair pitch

**Not established.** No source read this session gives a stair or ladder pitch for any type in the slice.
The Dong use of a ladder between floors is mentioned in passing in [S1] ("両層の間は梯子で連結されて
いる"), but no dimension is attached. The measurement that would settle it: the rise and going of the
external access stair or ladder on any one 吊脚楼, taken off a measured drawing.

---

## How the type varies

### 4.1 By ethnic group, as the sources actually distinguish them

| | Miao 苗族 | Dong 侗族 | Bouyei 布依族 | Tunpu 屯堡 | Tujia 土家 (Xiangxi) |
| --- | --- | --- | --- | --- | --- |
| primary material | fir timber, board walls, 青瓦 or fir-bark roof [S6][S7][S20] | fir timber throughout, board walls, 青瓦 [S4] | timber frame, stone skin, **slate roof** [S21] | bluestone walls, 青瓦 roof, bluestone paving [S23] | timber frame, board walls, 小青瓦 [S22] |
| ground treatment | slope cut into one or two platforms; posts of differing length [S6][S20] | "几乎不动或极少动土" — almost no earth is moved; posts of differing height do all the work [S4] | plinth + retaining wall; buildings step down the slope [S21] | level street grid on a terrace; walls do the defence [S23] | slope cut; a named vocabulary of hanging types [S22] |
| plan module | 3–5 used bays, 三间四立帖 or 三间两偏厦 [S7][S22] | **used bay = 2 structural bays**; 二十柱四排三间 common; 偏厦 on both sides [S1][S4] | 一明一次间 / 一明两次间 [S21] | courtyard house, 天井院 [S23] | 长三间/长五间/长七间; 3柱2瓜/5柱4瓜/7柱6瓜 [S22] |
| roof | mostly 悬山, some 歇山, some 带偏厦 [S7] | 悬山 or 歇山, double pitch [S21] | 悬山 (贵阳) / 硬山 (安顺) [S21] | 青瓦, gable walls above the roof [S23] | 小青瓦; 雍正十三年改土归流 before tile was legal [S22] |
| settlement institution | none recorded as a separate type | **鼓楼 + 风雨桥 + 粮仓 + 禾晾** [S3] | none separate; 布依堂屋 is named as a heritage element [S27] | **碉楼 + 屯墙 + 寨门**, a defensive set [S3][S23] | 走马转角楼 named as a protected type [S3] |

### 4.2 By terrain

The strongest claim the sources support is that **terrain, not culture, selects the plan variant**. The
carpenter interviewed by 黔东南日报社 says the 半吊脚/全吊脚 choice "看地基建筑，宽敞的屋基可全吊脚，
屋基有限的只半吊脚" — it is decided by the site [S6]. And 黄才贵 records that the *same village* on the
same slope carries different house types by what the platform permits: on a one-level cut, houses of
one plan; on a two-level cut, "则建成吊脚楼" [S1]. Terrain therefore drives, in order: number of
platform levels → hanging depth → post length distribution → storey count → whether a 偏厦 is possible.

**Valley versus ridge.** Both groups site in three configurations, and the configuration constrains
density: 山脚河岸型 (at the mountain foot facing a river), which "基本上都属于此种类型" for 程阳八寨,
where the slope behind and the river in front squeeze the village into a "内部密缩状态" — an internal
compression state, no room to expand [S4]; plus types [S4] names but this note did not extract.

**Miao by slope.** The 30–70° figure is given as the working range for the type, and 半边楼 is the
two-platform response; 全吊脚 is the single-platform response [S6][S20].

### 4.3 By wealth — the one gradient the sources actually grade

- **Tujia**, from [S22] **[unverified]**: 小户 → 长三间, 大/中户 → 长五间 or 长七间; the wealthy add
  雕梁画栋, raised eave corners and stone-stepped approaches; and the corner tower comes in grades,
  正屋配单转角楼 (one corner tower) and 正屋配双转角楼 (two).
- **Miao**, from [S22] **[unverified]**: 一般人家 4排扇3间 or 6排扇5间; 中等人家 5柱2骑 or 5柱4骑;
  大户 7柱4骑 or a four-courtyard compound.
- **Dong**, from [S1] **[academic]**: the 长屋 size is measured by the occupants, not by the money —
  "人的习惯以正房使用开间的两倍加两侧偏厦间来衡量干阑长屋居住空间的大小". But *ornament* grades it:
  [S1] records one 平流寨 long house whose 2nd, 3rd and 4th bays are all carved window lattice and is
  locally called 雕房.
- **Tunpu**, from [S8] **[unverified]**: 本寨 is "大户人家选中的风水宝地，高宅大院较多" and therefore
  holds the seven surviving 碉楼.

### 4.4 What a generator should expose, and what it should fix

**Expose (it varies and a viewer reads it):**
- terrain: ground field, slope, aspect, water presence — because it selects everything downstream;
- hanging depth and post-length distribution (the product of the first two);
- platform count and platform heights — the 挖填方 decision;
- used-bay count and 偏厦 presence on each side;
- storey count and which level takes which function;
- roof form (悬山 / 歇山 / 硬山), eave projection, whether there is a 披檐 or 腰檐 and how far it reaches;
- covering material (青瓦 / 石板 / 杉皮) — because it changes the drainage geometry and the load;
- whether there is a 美人靠, and its outward bulge;
- the wall system (board / 壁头墙 slab infill / 砌墙 rubble wall);
- settlement role and presence of the settlement institution.

**Fix (it is invariant and a variable would only add noise):**
- **used bay = 2 × structural bay** for the Dong long house [S1] — this is stated as a rule of
  measurement, not a preference, and it is the strongest single number in the note;
- **used bay width equals the 偏厦 used-bay width** on both sides [S1];
- drum-tower **plan is an even-sided polygon**; the **eave count is odd** [S1][S4];
- the drum tower's **inner and outer column nets are separate**, the outer is 12 posts (with 8 or 14 as
  variants) and the inner is 4 (with 1, 6 or 8) [S1];
- each eave **steps the section inward** [S1][S5];
- 榫卯, not nails — with bamboo nails as the recorded exception, for the 椽 [S4];
- the **three-level stack with its fixed content assignment** (§2.3);
- the **hearth room is double-height with no ceiling** [S4];
- **no drainage channels in a slate roof**; **no ridge tile in a slate roof** [S21];
- **wall thickness differs by a factor of thirteen between the two Bouyei wall systems** [S21].

---

## What a generator would need

### 5.1 The parameter list, in data terms

```
SITE
  ground_field                      # the terrain the building answers to; a query, not a number
  platform_count          : int    # 1 = 全吊脚 form, 2 = 半边楼 form
  platform_heights[]      : float  # metres above datum, per platform level
  platform_depths[]       : float  # metres, per platform level  (进深)
  retaining_kerb          : bool   # 保坎 present, of stone
  foundation_type         : enum   # {stone_plinth, retaining_kerb, rammed_earth, rock, street_terrace}

FRAME
  used_bay_count          : int    # 开间, counted in USED bays
  bay_ratio_used_to_struct: float  # = 2.0 for the Dong long house [S1]; a parameter elsewhere
  structural_bay_count    : int    # = used_bay_count * bay_ratio_used_to_struct
  side_wing_present[side] : bool   # 偏厦, normally both sides for the Dong long house [S1]
  frame_type              : enum   # {chuandou 穿斗, tailiang 抬梁, well_log 井干}
  columns_per_row         : int    # varies: 3,4,5,6,7 all occur; Dong common case 20 posts / 4 rows [S4]
  blocks_per_frame        : int    # 瓜 count; frame module 三柱二瓜 / 五柱四瓜 / 七柱六瓜 [S21]
  corner_post_species     : enum   # {fir 杉, maple 枫} — maple mandated at the Miao corner [S22]
  through_column_style    : enum   # {separate_posts 接柱建竖, through_post 通长柱建竖} [S1]
  stilted_side            : enum   # {none, front_only 单吊/钥匙头, both_ends 双吊/撮箕口,
                                #        both_wings 四合水, upper_storey 二屋吊, level_site 平地起吊} [S22]
  post_bottom_rule        : enum   # {ground, plinth, kerb_top, rock}
  cantilever_step         : float  # per-storey outward projection, 挑梁吊柱 [S4]

STOREYS
  storey_count            : int    # 2 or 3 is the normal range; 5–6 reported for 通道侗族 [S22]
  level_function[]        : enum   # {l_and_storage, living, grain, attic_guests, girls_floor}
  clear_height_by_space[] : float  # NOT one per storey: bedroom 2–2.5 m [S4], hearth room >3.5 m [S4]
  hearth_double_height    : bool   # implies no ceiling over the hearth bay [S4]
  grain_storey_staggered  : bool   # 谷仓与火塘间和卧室间上下错位 [S1]

ROOF
  roof_form               : enum   # {gable_overhang 悬山, half_hip 歇山, flush_gable 硬山,
                                #        polygonal_pyramid 攒尖, two_stage 二滴水}
  roof_pitch              : float  # rise:run — NOT ESTABLISHED for any type in this slice
  roof_pitch_degrees      : float  # derived; also NOT ESTABLISHED
  eave_projection         : float  # 出檐 — named as a first-class 掌墨 dimension [S4], value NOT ESTABLISHED
  ridge_board_drops       : float  # the step at each eave that contracts the section [S1][S5]
  eave_count              : int    # drum towers only; MUST be odd [S1][S4]
  has_lean_to_eave        : bool   # 披檐
  has_mid_eave            : bool   # 腰檐
  covering                : enum   # {grey_tile 青瓦, slate_slab 石板瓦, fir_bark 杉皮}
  slate_lap_minimum       : float  # >= 0.10 m per contractor practice [S9]; NOT ESTABLISHED by survey
  slate_thickness         : float  # ~0.02 m [S21] vs a 0.01–0.03 m working range [S9]
  slate_has_drain_channels : bool   # false for slate, true for tile [S21]
  slate_has_ridge_tile    : bool   # false for slate [S21]
  purlin_spacing_rule     : enum   # {by_covering_weight} — slate is heavier than tile [S9]

ENCLOSURE
  wall_system              : enum   # {board, slab_infill 壁头墙, rubble_wall 砌墙,
                                #        bluestone, none — for tower/bridge structures}
  wall_thickness          : float  # 0.03 slab infill / 0.40 rubble wall [S21]; branch on wall_system
  wall_is_loadbearing     : bool   # FALSE in every surveyed case [S21]
  corner_detail           : enum   # {flush, lapped, timber_post}
  gable_ornament          : enum   # {none, dragon_mouth 龙口} [S21]
  opening_head            : enum   # {lintel, pointed_arch, round_arch} [S21]

SEMANTIC SPACE
  has_central_hall        : bool   # 堂屋
  has_hearth              : bool   # 火塘
  has_veranda             : bool   # 廊 — the largest space in a Dong plan [S4]
  has_meirenkao           : bool   # 美人靠, on the veranda outside the hall
  meirenkao_bulge         : float  # outward bulge of the back rail [S6]
  hall_setback            : float  # 退堂 — NOT ESTABLISHED
  sunken_vestibule        : bool   # 吞口 [S3]
  hearth_side_platform    : bool   # 火铺, Tujia [S22]
  attic_usable            : bool   # the gable triangle [S4]

SETTLEMENT
  village_role            : enum   # {dwelling_only, dwelling_and_tower, walled_colony}
  drum_towers[]           : list   # {plan_sides(4|6|8), eave_count(odd), height,
                                #  inner_posts(4|1|6|8), outer_posts(12|8|14)}
  bridges[]               : list   # {deck_length, deck_width, pier_spacing, pier_batter, pier_height,
                                #  roofed_length, tower_count, tower_eave_tiers}
  granaries[]             : list   # {bay_count(1|2), frame_type(1..4), on_village_periphery}
  defence_elements[]      : list   # {watchtower 碉楼, wall 屯墙, gate 寨门}
  lane_topology           : enum   # {straight_trunk + 丁字/回字 secondary, wind_around_water, along_contour}
```

### 5.2 The constructs the current flat-plate volume vocabulary lacks, named as constructs

1. **A terrain-driven member.** A post whose length is `ground(x,y)` displaced by a parameter, with the
   base resolved to whatever the foundation enum says. Nothing in the vocabulary expresses a length
   that is not a difference of two authored planes.
2. **A two-grid plan.** A structural grid at the post spacing and a room grid at twice that, with
   partitions placed on the *room* grid and the frame generated on the *structural* grid. This is a
   derivation relationship, not an offset.
3. **A column-net shell.** The Dong tower's inner and outer post rings joined by 穿枋 and threaded with
   瓜柱/瓜枋 into a form that **steps inward per eave**. The vocabulary has a wall ring and plates; it
   has no *post ring* and no taper between rings.
4. **A corbel / multi-layer cantilever bracket.** 程阳桥's "层层出挑的密布式悬臂托架" — a stack of
   cantilevered bracket layers working in combination with simply-supported spans [S4]. A generator
   that can only place a beam cannot place this.
5. **A pitched purlin-framed roof as a sweep.** The roof is defined by purlin stations along the
   slope and a section that contracts; the covering is a separate surface laid on that sweep. The
   current vocabulary's roof is a flat plate, so the pitch, the section taper and the covering law are
   all absent.
6. **A covering with a laying law, not a material.** Two coverings are needed with *different geometry*:
   interlocked tile with drainage channels, and lapped slate with **no channels and no ridge tile**
   [S21]. A covering is a procedural lay-up with a lap, a stagger and a curve, and its parameters
   change the purlin spacing because slate is heavier than tile [S9].
7. **Non-loadbearing infill.** Every wall here is infill [S21], so the wall must attach to frame faces
   without being expressed as structure, and must branch to two thicknesses an order of magnitude
   apart.
8. **A settlement generator.** Towers, bridges, granaries, drying racks and defence elements are
   placed by village rules [S3][S1], and some of them — the granary — are placed on the *periphery*,
   not in the village [S1]. No per-plot generator produces this.
9. **A semantic-space layer independent of the solids.** 堂屋, 火塘, 廊, 美人靠, 天楼, 阁楼, 火铺 and
   吞口 are *named* positions that must exist as places for anything to be hung on them. Several of
   them (廊 largest [S4], 谷仓 staggered against the rooms below [S1]) are not derivable from the
   solids.
10. **A variable-height, variable-storey-count stack.** Three levels with a fixed content assignment, a
    double-height hearth bay, a usable attic in the gable triangle [S4], and an *optional* extra storey
    (二屋吊) [S22] — the current "plates" assume a uniform floor-to-floor.

---

## Sources

All access date **2026-10-18**. Tag in brackets; "full text read" means the document body was read, not
just an abstract.

**Primary survey and academic documents (full text read)**

- **[S1]** 黄才贵 (Huang Caigui), 《侗族住居空间构成的调查报告》, *国立民族学博物館研究報告* 第18巻第2号,
  国立民族学博物館 (National Museum of Ethnology, Osaka), 公開日 2010-02-16. DOI 10.15021/00004224.
  https://minpaku.repo.nii.ac.jp/record/4232/files/KH_018_2_003.pdf — **[academic]**,
  ethnographic field survey in Guizhou (榕江 Rongjiang, 黎平 Liping, 从江 Congjiang, 三江 Sanjiang),
  45 pp. **Full text read.** The single most valuable source in this note for measured and structural
  fact. OCR of the extracted text is poor (侗族→個族, 鼓楼→鼓棲); readings of damaged characters were
  made against context and are flagged where doubtful.
- **[S4]** 《三江侗族建筑的科技人类学考察》, 广西大学 (Guangxi University) 硕士学位论文.
  https://www.renrendoc.com/free-down/7156010120000044.pdf — **[academic]**, master's thesis, fieldwork
  in 三江侗族自治县 (Sanjiang, Guangxi), 51 pp. **Full text read.** Author name not legible in the
  extraction; the institution and degree are. Carries the 程阳风雨桥 measured dimensions and the
  掌墨 craft rules.

**Heritage and planning instruments (full text read)**

- **[S3]** 《贵州省"十四五"民族特色村寨保护与发展规划》, 贵州省民族宗教事务委员会, issued as
  黔民宗发〔2021〕21号. Text as republished by 道真仡佬族苗族自治县人民政府 (2022-01-14) and
  贵阳市云岩区人民政府 (2023-02-17).
  http://www.gzdaozhen.gov.cn/xxgk/xxgkml/zdlygk/wwbh/202201/t20220114_72314731.html ·
  https://www.yunyan.gov.cn/zwgk/zdlygk/mzzj/202302/t20230217_82703640.html — **[heritage survey]**,
  **full text read (both mirrors)**. Its 专栏3 *全省各民族特色民居保护目录* is the official type
  catalogue and the best available enum source for this slice: it separates 干栏式建筑, 地房建筑 and
  其他建筑, and names 鼓楼建筑 and 风雨桥建筑 as their own protected class.
- **[S26]** 韦应学, 《立足活态传承　打造美好家园》, 《贵州政协报》, 2015-11-12.
  https://www.gzszx.gov.cn/gzzxb/web/doc/detail/1866/A2 — **[heritage survey]**, **full text read**.
  Used for the village-count facts and for the recorded demolition of 600-year-old 花溪布依大寨 (2015).
- **[S25]** 郭楚雯、周真刚、罗晓辉, 《民族地区传统村落整体保护和活态传承调查研究》, 《贵州政协报》,
  2025-12-31. https://www.gzszx.gov.cn/gzzxb/web/doc/detail/d_1720626511347744 — **[heritage survey]**,
  **full text read**. Policy; 757 Guizhou villages on the national list at the time of writing.

**Secondary and official-feature sources (full text read; no structural weight)**

- **[S24]** 娄波 / 王子琪, 《布依族石板建筑的生存智慧与设计新生》, 贵州省人大网站, 2025-04-11
  (originally 贵州日报 天眼新闻). https://www.gzrd.gov.cn/gzwh/202504/t20250411_87504652.html —
  **[unverified]** for measurement, but it is the **original** of the article the Baidu Baike 石板房 entry
  reproduces, so it is the traceable ancestor of the §3.5 numbers.
- **[S20]** 中南民族大学民族学博物馆, 《民族文化｜苗寨吊脚楼营造技艺》, 搜狐, 2018-08-06.
  https://m.sohu.com/a/245515087_99936334 — **[unverified]**. University museum account; the source of the
  半边楼 two-platform rule and the 6–7 m frame height.
- **[S23]** 中共普定县委宣传部政务号, 《屯堡·家国六百年丨军屯石韵：屯堡建筑里时光回响》,
  澎湃新闻, 2025-11-09. https://m.thepaper.cn/newsDetail_forward_31930843 — **[unverified]**. The only
  source found with any 碉楼 dimensions; party-government account.
- **[S6]** 张希才, 《独特的苗寨吊脚楼》, 黔东南新闻网 / 黔东南日报社, 2021-04-19.
  http://qdnrbs.cn/whly/2021-04/19/131_157421.html — **[unverified]**, reporting one local carpenter.
- **[S5]** 央视网, 《文化中国行 | 一半诗意，一半烟火 这是属于侗寨鼓楼的时光胶囊》, 2025-03-26.
  https://news.cctv.com/2025/03/26/ARTIue9B3yl6gkWbdrnfMYJY250326.shtml — **[unverified]** for
  measurement. Sole source for 信团鼓楼 11 层 / 24.37 m and for 黎平县 429 座鼓楼.

**Aggregator pages (full text read; recorded as claims, never as findings)**

- **[S22]** 《吊脚楼（干栏式建筑）》, 百度百科. https://wapbaike.baidu.com/item/吊脚楼/71296 —
  **[unverified]**, user-edited, cites exactly **one** reference for the whole entry. It is nevertheless
  the densest single source found for Miao and Tujia plan vocabulary, and it is the origin of the
  "5–7 posts per row", "3 尺 hearth pit" and "6 尺 进深" figures. Treated accordingly.
- **[S21]** 《石板房（以石条或石块建造的房屋）》, 百度百科.
  https://wapbaike.baidu.com/item/石板房/3267522 — **[unverified]**, but reproduces (with attribution)
  the article in [S24] and others; the §3.5 Bouyei numbers trace to it.
- **[S7]** 《苗族吊脚楼》, 百度百科. https://wapbaike.baidu.com/item/苗族吊脚楼/1281564 —
  **[unverified]**, 18 references cited. Source of the 3/4/5-bay claim and the 悬山/歇山 split.
- **[S8]** 《安顺云峰八寨屯堡景区》, 360百科. https://baike.so.com/doc/9485667-9828549.html —
  **[unverified]**, user-edited and visibly OCR-damaged (characters interleaved mid-word).

**Bibliographic records located but NOT obtained (listed so the next session does not re-hunt them)**

- **[S11]** 彭永鹏、张庆高、肖晓维、阿依丁、马宏, 《贵州省松桃大湾苗寨建筑数字化保护》, *大连民族大学学报*
  2017年03期. https://wap.cnki.net/touch/web/Journal/Article/DLMY201703014.html — **[academic]**,
  **abstract only read** (CNKI paywall). Terrestrial-laser-scan point cloud of a Miao stilt-house village
  used to derive dimensions and joint details and to produce 2D construction drawings and a BIM model.
  **The closest existing thing to a measured drawing set for a Miao stilt-house village.**
- **[S12]** 徐梦真, 《吊脚楼主体结构参数化生成研究——以鄂西南地区为例》, 广州大学硕士学位论文 —
  **[academic]**, **not obtained**. Directly on-topic: a parametric-generation study of the *main structure*
  of a stilt house. Despite being 鄂西南 (a sibling note's slice) it is the closest existing precedent for
  this exact modelling task and should be read before a design is fixed.
- **[S13]** 蔡凌, 《侗族建筑遗产保护与发展研究》, 北京: 科学出版社 — **[academic]**, **not obtained**.
- **[S14]** 越剑, 《河里的石头滚上坡——贵州安顺屯堡民居》, 贵阳: 贵州科技出版社 — **[academic]**,
  **not obtained**.
- **[S15]** 罗德启, 《贵州民居》, 北京: 中国建筑工业出版社 — **[academic]**, **not obtained**. The standard
  survey of this slice.
- **[S16]** 王绍周总主编, 张家泰等分篇主编, 《中国民族建筑》第5卷, 南京: 江苏科学技术出版社 —
  **[academic]**, **not obtained**.
- **[S17]** 刘俊, 《湘西土家族建筑与文化》, 沈阳: 辽宁美术出版社 — **[academic]**, **not obtained**.
- **[S27]** 韦应学, quoted in [S26] for the heritage-element list including 布依堂屋; no standalone source.
- **[S28]** 杨瑞兴, 《贵州省传统村落居民点空间分布及其影响因素》.
  http://www.sssampling.cn/down/2020%20杨兴荣-贵州省传统村落居民点空间分布及其影响因素.pdf —
  **[academic]**, **not obtained**; settlement distribution, not building geometry.
- **[S29]** 胡慧光等, 《基于图底关系理论的屯堡视觉特征分析——以贵州本寨屯堡聚落为例》;
  李晓黎, 《贵州安顺屯堡聚落空间形态及建筑特征研究》; 周政旭、封基铖等,
  《贵州扁担山 白水河地区布依族聚落调查研究》(中国建筑工业出版社); 董静, 《大山里的石头民居》
  (经济科学出版社); 吴育标, 《中国世界遗产战略管理模式研究——以西江千户苗寨为例》(人民出版社);
  罗建平, 《安顺屯堡的防御性与地区性》(清华大学出版社); 邵琨, 《贵州干栏建筑形制初探》;
  肖冠忠, 《中国西南干栏建筑体系研究》; 周浩, 《基于风环境模拟的湘西传统村落和吊脚楼自然通风
  传承性研究》; 董茜静, 《湘西土家族吊脚楼营造技艺及价值启示》; 宁天阳,
  《土家族传统民居形式在乡村民宿设计中的应用研究》; 王昭, 《土家吊脚楼建筑平面空间与营造特征研究》;
  郭璐, 《基于人类聚居学理论的侗族传统村落空间形态研究——以广西三江高定侗寨为例》; 周志军,
  《侗族传统木构建筑的保护与创新研究——以广西三江高定侗寨为例》; 杨瑞, 《明清黔中屯堡建筑研究
  ——以本寨为例》; 惠飞, 《贵州麻山地区格凸河流域苗族村寨实态调查研究》; 尹芳,
  《贵州镇宁县布依族传统村落景观空间形态研究》; 祁峰, 《基于文化基因挖掘的贵州丙安村传统村落保护
  发展策略研究》; 杜倩, 《贵州喀斯特山区民族传统乡村聚落形态研究》; 王橦橦, 《乡土元素在民族村落
  景观中的传承与应用研究》; 王长馨, 《传统村落改造中的材料运用与研究》; 郭兆魁,
  《湘西民居窗饰艺术研究》; 贺平安, 《湘西土家族民居营建技艺研究》; 吴宏宇, 《贵州黔东南季刀苗寨
  民居改造研究》; 张和平, 《苗族数理文化》(民族出版社); 粟周榕, 《六洞、九洞侗族村寨》
  (贵州人民出版社); 《黔东南苗族侗族自治州志·文物志》(贵州民族出版社) and 《城建环保志》;
  《雷山县城乡建设资料》; 《从江县民族民间文化知识读本》; 《镇远县志》(贵州人民出版社);
  《湘西土家族建筑与文化》; 孙宗文, 《中国建筑与哲学》(江苏科学技术出版社, 2000-03, p.194 — the
  thesis in [S4] quotes it for 程阳桥). **[academic]** for the theses and 地方志; **not obtained**;
  all surfaced through a search index that returned local corpus paths rather than resolvable URLs, so
  none of these can be cited to a stable link from this session.

**Hosts that failed or returned empty, recorded so they are not retried**

- **[S30]** Failed: `zh.wikipedia.org`, `en.wikipedia.org` (fetch failure); `core.ac.uk` (403);
  `museum.zhaiu.com` (503); `aho.brage.unit.no` (connection failure);
  `baike.baidu.com` desktop domain (403 security challenge); `admin2017.cqup.com.cn`; `queshu.com`;
  `wap.baidu.com/item/土家转角楼` (403); `www.sssampling.cn`; `d.wanfangdata.com.cn`.
- **[S31]** Returned HTTP 200 with an empty body: `www.hsfwest.com`, `aipub.cn`, `www.kjhcy.org`,
  `app.asrbs.net`.
- **[S32]** Returned a content type the fetch tool rejects: `urbandesign.tsinghuajournals.com`
  (`application/x-download`), `www.pjlss.edu.pk`, `wb.gywb.cn` (`application/pdf`).
- **[S33]** Direct `Invoke-WebRequest` failed SSL on every host, both direct and via `127.0.0.1:7897`.
  `curl.exe -k` succeeded at ~25 KB/s; that is the route that produced [S1] and [S4].

---

## Not established this session (do not treat as fact)

**The two most important missing numbers.**

- **Roof pitch, in rise:run and in degrees, for every type in this slice.** No source read printed a
  pitch for 吊脚楼, 侗族木楼, 石板房 or 屯堡民居. The only directional statement anywhere is a
  contractor's "坡度宜偏陡" for slate [S9], which is advice rather than measurement. **What would
  settle it:** the rafter length, purlin spacing and ridge height of one surveyed 吊脚楼 and one surveyed
  石板房, from a measured drawing — the 松桃大湾 laser-scan set [S11] would be the cheapest route, and
  《贵州民居》 [S15] the fallback. This is the single highest-value missing measurement, because pitch
  sets eave projection, ridge length, roof area and the whole silhouette.
- **Slate-slab dimensions as a laying geometry.** Slate *thickness* is reported (≈2 cm [S21]; 1–3 cm
  working range [S9]) and a *minimum lap* is reported (≥10 cm [S9]), but **slab plan dimensions, the
  overlap expressed as a fraction of slab length, the number of courses, and the resulting pitch were
  not established**. **What would settle it:** one roof measured — a plan of 10 consecutive courses with
  each course's slab count and the rise between the eave and the ridge.

**Geometry and construction, per type.**

- **Hanging depth as a function of ground slope.** The *qualitative* rule is confirmed (deeper slope →
  further projection; 半边楼 is the two-platform response [S20]; 全吊脚 when the site allows [S6]) but no
  numeric relation was found. **What would settle it:** for 10 houses, the terrain slope under the
  building, the platform heights, and the resulting shortest and longest post lengths.
- **吊脚柱 founded on rock.** The brief asks for this; **no source read this session mentions it**. The
  nearest verified fact is 黄才贵's account of posts founded on 基石 (stone plinths) inside an excavated
  base, and on carved stone 保坎 [S1]. Whether a rock-founded post is a distinct practice or simply what
  happens where the 保坎 is omitted is **not established**.
- **Eave projection 出檐, and 披檐 / 腰檐 reach.** 出檐 is named as one of the four dimensions the 掌墨
  marks on the rod [S4], so it is definitely a first-class parameter, but **no value was printed by any
  source**. **What would settle it:** eave projection measured on one building of each type.
- **Stair or ladder pitch.** Not established for any type (§3.9).
- **Column diameters and column spacing in absolute millimetres.** Not established for any type. The
  *counts* are established (二十柱四排三间 [S4]; 4 inner / 12 outer posts in a drum tower [S1];
  三柱二瓜 / 五柱四瓜 / 七柱六瓜 [S21]) but the *sizes* are not, and the only relative statement is
  "柱子需要粗一点的木料，而穿斗式的梁架部分所需要的木料就比较细了" [S21] — thicker posts, thinner frame,
  which is a ratio not a number.
- **Wall thickness for timber board walls and for Tunpu bluestone walls.** Only the two Bouyei systems
  were found (3 cm and 40 cm, [S21]).
- **转角楼 geometry.** Only that it exists as a protected type [S3] and comes in one- and two-corner
  grades [S22] was found. Its plan, roof section, and how the wrapped roof is framed are **not
  established**. **What would settle it:** a plan and section of one 走马转角楼, and the 《湘西土家族
  建筑与文化》 [S17] or 《湘西土家族民居营建技艺研究》 [S29].
- **Tunpu house dimensions.** Wall thickness, floor heights, roof pitch and 碉楼 plan are all
  **not established**; only 碉楼 height (10–15 m) and storey count (3–5) were found, and from a
  party-government feature. **What would settle it:** 越剑, 《河里的石头滚上坡——贵州安顺屯堡民居》
  [S14], or a 文物保护规划 for 本寨 or 云山屯.
- **风雨桥 forms other than 程阳桥.** The measurements in §3.3 are all from **one bridge**, described
  at length by a thesis that also quotes its repair record. They are the best numbers in this note and
  they are still one building.

**Claim-level negative results.**

- **"Eave counts are odd" is corroborated across two independent academic sources [S1][S4], but the
  height range is not.** [S1] says at most 17 eaves and about 20 m; [S4] says up to 27 eaves and
  10–40+ m; [S5] gives 11 storeys and 24.37 m for a *named* tower. **Treat eave count as an odd
  parameter and height as derived from the taper. Do not encode any of the three ranges.**
- **"The Miao stilt house is built on 30–70° slopes" is stated identically by two sources** [S6][S20],
  which is the closest thing to a corroborated number in the Miao section — but **both are
  non-survey** (a news feature quoting a carpenter, and a university museum popular article). No
  heritage survey or academic source read this session states it. **Unverified but repeated.**
- **The Miao "typical" dimension set — 6 尺 进深, ~100 m² per level, 4 尺 level difference, 5–7 posts
  per row — is a single uncited Baidu Baike figure** [S22], sitting alongside a *different* set in the
  same entry (4排扇3间 / 6排扇5间 / 5柱2骑 / 7柱4骑) [S22]. **This is exactly the "a typical
  dimension that turns out to be one building" failure mode, and it should be recorded as such rather
  than used.**
- **The Miao name of the 美人靠 is reported four different ways** (嘎息/ghab xil, 阶息, 干席, 阶息站)
  across [S6][S22] and none is a linguistic survey. **Not established.**
- **Two independent drum-tower type classifications disagree on the third term**: [S1] gives
  干栏式 / 楼阁式 / 密檐式 / 门廊式 / 厅堂式, [S4] gives 厅堂式 / 干栏式 / 密檐式. The two agree on the
  first, third and fifth. The disagreement may be scope (province vs county) rather than error.
- **退堂 was not found in any source read.** The brief names it; this session did not corroborate it.
- **A tourist-reconstruction claim could not be checked.** The 苗族吊脚楼 Baidu entry's own reference
  list includes a 2023 Tencent travel piece, and the provincial plan [S3] records the general practice of
  stripping vernacular villages to a uniform "青瓦白墙" [S26], which is the failure mode; but no specific
  reconstruction was identified and measured against its original.

**What would most cheaply close the largest gaps, in order.**

1. Read **[S11]** (松桃大湾, laser-scanned Miao stilt houses → extracted dimensions + 2D construction
   drawings). One paper, and it is aimed at exactly the missing numbers.
2. Read **[S15]** 罗德启《贵州民居》 — the standard survey, and the likely home of pitch, post diameter
   and wall thickness for all of Miao/Dong/Bouyei at once.
3. Read **[S12]** (parametric-generation study of a stilt house's main structure) **before** any design is
   fixed, so the generator's parameter list is compared against an existing one rather than invented.
4. Obtain **[S14]** 越剑《河里的石头滚上坡》 for the Tunpu dimensions, and a 文物保护规划 for 本寨 or
   云山屯 — a protected-site plan is required to publish its own inventory.
5. Obtain **[S17]** 刘俊《湘西土家族建筑与文化》 or the 贺平安 thesis for 转角楼.
