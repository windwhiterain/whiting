# Yunnan's main vernacular house types: what structural and geometric rules drive a procedural model?

**Question.** What structural and geometric rules define Yunnan's main vernacular house types — the Bai
courtyard house (三坊一照壁 / 四合五天井), the Naxi house of Lijiang, the Dai bamboo stilt house (干栏 / 竹楼),
and the Kunming 一颗印 — precisely enough to drive a procedural model?

**Status.** Research note, complete. Rules and proportions only; every number carries a source tag
and every unverified number is marked as one. This note makes **no design decisions**.

**Date.** Started and completed 2026-10-18. All sources listed were accessed 2026-10-18.

**Sibling notes** on the same frame: `procedural-architecture-techniques.md` (the vocabulary this one
is written against).

## Verdict

The four types are not four plan shapes; they are **four settings of one timber frame**. A 2025
field survey of Yunnan timber housing identifies exactly five timber systems in the province
[Y6], and three of my four types sit on the same one: the **穿斗式** *chuándǒu* "column-and-tie"
frame carries the Bai courtyard house, the Naxi house of Lijiang and the Kunming 一颗印 alike.
The fourth, the Dai 干栏/竹楼, is on the **干栏式** *gānlán* stilt system, which is a different
thing: an open ground storey on many square posts, a boarded upper storey, and a light pitched
roof hung on a *人字架* "A-frame" of two raking beams. The Bai/Naxi/Yikeyin house is a **two-storey
bearing frame with non-loadbearing infill walls** — [Y1] states it flatly: 建筑物重量全部由构架承担,
墙壁只起围护作用 ("the whole weight is carried by the frame; the wall only encloses") — which is
the single most transferable structural fact in this note. [Y6] confirms it from the damage side:
when these houses fail in an earthquake it is the *joints* that fail first (拔榫, 脱榫, 榫头断裂),
the classic **strong-member / weak-joint** behaviour, while the infill walls crack and collapse out
of plane (外闪倒塌). A generator therefore needs a **frame object that owns the load**, and walls
that are a separate, non-structural layer hung on it — which is not what the current vocabulary's
"building mass = wall ring + plates" assumes.

Second, these buildings are **modular in a unit that is a building, not a bay**. The Bai 坊 *fāng*
is a whole three-bay two-storey house — a unit you can repeat to make a 三坊一照壁, a 四合五天井 or
a two-fang corner. [Y2] shows a family that built 二坊拐角 "two wings round a corner" first and
added the third wing later when it could afford to. So the generator's primary parameter is a
**wing object** with its own bay count, depth, storey count and eave-column/king-post grid, and the
courtyard type is an *arrangement of wings* plus one free-standing wall (照壁 *zhàobì*, screen
wall) or one gate block. 三坊一照壁 and 四合五天井 are the same objects at different counts and
rotations; the number 四合五天井 ("four sides, five skywells") names five courtyards, not four
wings — the fifth is the central one and four are the corner ones.

Third, **no roof in this slice is flat**, and two of them are not even simple planes. The Bai wing
is a **硬山** *yìngshān* "flush-gable" double-pitch roof — the gable wall rises to the roof, so the
roof's own triangular end is hidden — carried on a stacked 五架梁/七架梁 *wǔjiàliáng/qījiàliáng*
"five-purlin / seven-purlin beam" frame whose purlin count is *chosen from the depth*. The 照壁
has its own tiny four-way roof (四撇水) with lifted corners. The 一颗印 adds a **腰檐** *yāoyán*
"waist eave" — a second, lower roof over the courtyard — and its 耳房 roofs are **内长外短**,
longer inside, shorter outside, deliberately draining into the courtyard (四水归堂). The Dai roof
is a **45°–60° unequal-pitch 人字架 with 歇山 ends and a 重檐 on the lower part** [Y6]. Every one
of these breaks a "roof is a flat slab" assumption, and the 腰檐/重檐 cases break a "one roof per
building" assumption.

Fourth, the eave is the part that carries the type's identity and the part the vocabulary is
missing. [Y1] calls it "屋面檐口飞檐石的设置是白族民居区别于其它民居建筑的一大特色" — the
flying-eave-stone arrangement is *the* thing that distinguishes the Bai house from every other
dwelling. Bai eaves are **stone**: a raised 青石板 flying-eave stone *fēiyánshí*, graded large /
medium / small (大号 / 中号 / 虎牙) by which wall it sits on, with 飞砖 and 飞瓦 (projecting brick
and tile courses) under it. 一颗印 eaves are **timber** — the 腰檐 / 大厦 / 小厦 over the courtyard.
Dai eaves are a **deep, steep, unadorned overhang** on a light frame. A generator must model the
eave as an *independent element with its own projection, its own underside bracket, and its own
build-up*, not as an offset of the wall face.

Fifth, the **courtyard is a sized void, not a leftover**. 一颗印's courtyard is stated as
**3 m square** inside a **12 × 15 m** site — that is roughly 1/4 of the site's area, and the
building is built around it rather than the courtyard being whatever is left [Y3]. The Bai 三坊一照壁
courtyard at the surveyed 先锋街20号杨宅 is 面阔三间、进深三间 (three bays wide, three bays deep)
[Y2]. [Y3] names the courtyard's job in mechanical terms: it is a light well and stack-effect
vent, and the overhanging 腰檐 on all four sides are what make it work. So a generator must treat
the courtyard as an **authored rectangle with a target aspect and a target fraction of site area**,
with the surrounding eaves derived from it.

Sixth — and this is the negative result that matters most for scope — **Yunnan is full of claims
that repeat everywhere and that I could not verify.** A number such as "the Bai 开间 is 3.3–3.6 m"
appears in exactly one publication family [Y1], reproduced on at least three unrelated websites, and
carries **no sample size, no building names and no drawing references**; it may be one building
generalised. The 照壁's "6.5 m high, 0.6 m thick" comes from the same single text. Conversely the
**strongest numbers in this note come from one 2025 peer-reviewed field survey** [Y6] which names
its sources and its sample region — and that survey says almost nothing about the Bai, Naxi and
一颗印 plan dimensions, because it was a seismic study, not a dimensional one. **So: the structural
logic is well documented; the plan dimensions are thin, and a measured drawing set of three or four
named buildings is what would settle them.**

## The parts and their correspondence

Ordered outside-in, per type. Every term: 汉字 + pinyin + gloss at first use.

### The Bai 三坊一照壁 / 四合五天井 (Dali and the Erhai basin)

| part | 汉字 pinyin | gloss | structural? | carried by |
|---|---|---|---|---|
| 坊 | *fāng* | "wing" — the **unit** of Bai dwelling composition: a three-bay, two-storey house treated as one indivisible object. [Y1] "平面布局多以三个开间组成一坊为单元" | yes, it is a whole frame | its own posts and beams |
| 正房 / 主房 | *zhèngfáng* | main wing; in 三坊一照壁 it faces the screen wall and holds the elders [Y2] | yes | own frame |
| 厢房 | *xiāngfáng* | the two flanking wings, for the younger generation [Y2] | yes | own frame |
| 堂屋 | *tángwū* | the central hall bay of the 正房 — **wider than the side bays** [Y1] | yes, its own bay | own frame |
| 厦子 / 前廊 | *shàzi / qiánláng* | the **veranda / fore-gallery**: the covered depth between the eave column 前檐柱 and the king column 金柱 [Y1] | yes, roofed | the wing's own frame |
| 大出厦 / 小出厦 | *dàchūshà / xiǎochūshà* | the larger and smaller veranda roofs [Y1] | yes | eave columns |
| 漏阁 / 漏角屋 | *lòugé / lòujiǎowū* | "leaking corner house" — a small two-room annex hung off the end of the 正房, **shallower in depth and lower in height than the main wing**, forming a 一线天 "one line of sky" slot or a small 天井 for light, air and rainwater [Y1][Y2] | yes | the 正房's frame |
| 麻雀台 | *máquètái* | the small upper platform at the corner where 正房 and 厢房 meet, above the 漏阁 [Y1] | secondary platform | corner posts |
| 头梁合抱柱 | *tóuliáng hébàozhù* | "head-beam embracing-column": at the junction of 正房 and 厢房 the two wings **share one column**, the beam of one embracing the post of the other [Y2] | yes — a shared member, not a butt joint | itself |
| 八字墙 | *bāzìqiáng* | the alternative: an L-shaped wall that **separates** the two wings and *widens* the courtyard, used where the budget forbids the shared column [Y1][Y2] | enclosure | ground |
| 照壁 | *zhàobì* | screen wall closing the fourth side of the yard; **freestanding, not part of any frame** — masonry and 泥作 "mud work" only [Y1] | no — pure enclosure | its own footing |
| 门楼 / 大门 | *ménlóu / méndà* | gate block, normally **leaning against a gable** (当地叫过山门) [Y1] | its own small timber frame between two masonry piers | 门墩 piers |
| 跑马转角楼 | *pǎomǎ zhuǎnjiǎolóu* | "running-horse corner gallery": the **continuous gallery wrapping the yard** — where the four wings' upper floors and their colonnades join into one ring [Y1][Y2] | yes | the four wings' frames, tied |
| 山墙 / 封火山墙 | *shānqiáng / fēnghuǒ shānqiáng* | gable wall; 封火山墙 = "fire-sealing gable", raised above the roof as a parapet; [Y1] calls these gables a *major feature* of the Dali silhouette | enclosure, laterally braced by the frame | frame |
| 硬山 / 悬山 | *yìngshān / xuánshān* | flush-gable (gable end flush with the roof, roof triangle hidden) / overhanging-gable (roof overhangs the gable) | roof form | purlins |
| 腰檐 | *yāoyán* | a secondary lower eave/roof over a gallery or between storeys | yes | posts |
| 飞檐石 / 虎牙 | *fēiyánshí / hǔyá* | the **stone flying-eave corbel**: a bluestone slab on corbels that throws the eave out; graded 大号 "large" on the rear wall, 中号 "medium" on the gable, 虎牙 "tiger tooth" at the front corner [Y1] | yes — corbelled, not framed | the wall beneath it |
| 飞砖 / 飞瓦 | *fēizhuān / fēiwǎ* | projecting brick and tile courses tucked under the flying-eave stone [Y1] | no | the stone |
| 五架梁 / 七架梁 | *wǔjiàliáng / qījiàliáng* | the "five-purlin" / "seven-purlin" stacked beam frame; the count is **chosen by the depth** of the wing [Y1] | yes — this is the roof structure | posts |
| 金片结构 / 工字形构架 | *jīnpiàn / gōngzìxíng* | the two frame profiles [Y1] names | yes | — |
| 金柱 / 前檐柱 / 中柱 | *jīnzhù / qiányánzhù / zhōngzhù* | king column (inner main post) / eave column / intermediate post — the 穿斗 set | yes | — |
| 挂枋 / 盖梁 / 千斤担 | *guàfāng / gàiliáng / qiānjīndān* | purlin assembly: the **ridge purlin is three members** (one 挂枋 + two round 盖梁), **every other purlin is two** (one 挂枋 + one 盖梁); a 千斤担 "thousand-jin bearer" sits under the 挂枋 [Y1] | yes | — |
| 封檐板 | *fēngyánbǎn* | carved fascia board closing the rafter ends [Y1] | no | rafters |
| 悬鱼 | *xuányú* | the carved hanging fish-tail pendant at the apex of a gable, under the roof overhang | no, decorative | gable |

### The Naxi house of Lijiang (大研古城)

The Naxi house is **not** a separate building type structurally. [Y6] classifies the province's
timber housing into 穿斗式 *chuándǒu* / 干栏式 *gānlán* / 井干式 *jǐnggān* / 平顶式 *píngdǐng* /
人字架 *rénzì* and places Lijiang in the first — the same system as the Bai and the 一颗印. Its own
literature is organised around **纳西建筑木构架与土坯** "Naxi timber frame and adobe" [Y6 ref.
解婧雅 2014] and **纳西族乡土建筑建造范式研究** "a study of the *construction paradigm* of Naxi
vernacular building" [Y6 ref. 潘曦 2014, 清华大学 doctoral thesis] — the clearest statement in the
cited literature that Naxi building is a rule-governed craft rather than a set of shapes.

Lijiang inherits the Bai composition: [Y2] opens its Lijiang chapter "三坊一照壁是丽江白族民居中
最基本、最常见的型式" and every worked example it gives is a Lijiang address. Three verified
differences: the gallery is called **厦子** *shàzi*; the wing junction uses **头梁合抱柱** (the
shared column) or, on tight plots, **照面梁榫卯 + 跑马转角楼** instead [Y2]; and the house carries
the 悬鱼 gable pendant and the **三眼井** *sānyǎnjǐng* "three-eye well" — a single wellhead with
three apertures, which is a *topology* object (one node, three openings) and not a dimension.
**Naxi bay widths, depths, storey heights and roof pitch: not established this session — see §7.**

### The 一颗印 (Kunming and the central plateau, 滇中)

| part | 汉字 pinyin | gloss | structural? |
|---|---|---|---|
| 颗印 | *kēyìn* | "one seal" — the name is the plan reading as a square seal [Y3] | — |
| 三间四耳倒八尺 | *sānjiān sìěr dàobāchǐ* | the standard form: **three-bay main range, two ear-houses on each side, and a 倒座 "inverted" range 八尺 deep**, the three enclosing one small square courtyard [Y3] | yes |
| 倒座 | *dàozuò* | the range facing the gate on the street front; 八尺 deep, roofed as a **门廊** *ménláng* porch [Y3] | yes |
| 耳房 | *ěrfáng* | "ear house" — a two-storey range flanking the courtyard, shallower than the 正房 | yes |
| 正房 | *zhèngfáng* | the three-bay main range | yes |
| 天井 | *tiānjǐng* | the courtyard void, **about 3 m square** [Y3] | void |
| 腰檐 | *yāoyán* | the eave over the courtyard — over the 正房 it is the **大厦**, over the 耳房 the **小厦**; all four connect, so a person can walk round in rain [Y3] | yes |
| 楼梯巷 | *lōutīxiàng* | a narrow **stair lane** — the vertical circulation, which is a *plan* element occupying a strip [Y3] | yes |
| 暗沟 | *àngōu* | the covered drain ringing the stone-paved courtyard [Y3] | site |
| 四水归堂 | *sìshuǐ guītáng* | "four waters return to the hall" — 耳房 roofs are pitched **内长外短**, longer inside and shorter outside, so their water is thrown *inward* into the courtyard [Y3] | roof rule |
| 夯土墙 / 土坯墙 | *hāngtǔqiáng / tǔpīqiáng* | the outer walls — thick, with **very small openings** [Y3] | enclosure only |
| 下店上居 / 前店后居 / 半颗印 | *xiàdiàn shàngjū / qiándiàn hòujū / bàn kēyìn* | the shop-below / dwelling-above, shop-front-house-behind, and "half a seal" (三间两耳) variants [Y3] | — |

### The Dai 干栏 / 竹楼 (西双版纳; 瑞丽 and 元江 variants)

| part | 汉字 pinyin | gloss | structural? | carried by |
|---|---|---|---|---|
| 干栏 | *gānlán* | "dry column" — the stilt house; also **千脚落地房** "the thousand-legged house that stands on the ground" [Y6] | — | — |
| 竹楼 | *zhúlóu* | "bamboo house"; the Xishuangbanna form is nicknamed **凤凰展翅** *fènghuáng zhǎnchì* "phoenix spreading its wings" [Y5] | — | — |
| 柱础石 | *zhǔchǔshí* | the column-base stone, **half-buried in the ground**, post standing on it; some houses have **no foundation at all** and set posts straight on the earth [Y6] | foundation | ground |
| 穿枋 / 穿条 | *chuānfāng / chuāntiáo* | through-ties, 穿条 in 滇南. **The longitudinal and transverse through-ties are not in the same plane — they are offset vertically by about 30 cm** [Y6] | yes | posts |
| 方柱 | *fāngzhù | **every post is square-section**, and there are many of them [Y6] | yes | — |
| 馒头榫 | *mántóusǔn* | "bun tenon" — joins the 人字架 roof to the lower frame [Y6] | yes | — |
| 齿接 / 单齿接 | *chǐjiē / dānchǐjiē* | **notched joints, all single-notch** in the Dai frame; between raking beam and centre post, and between beams [Y6] | yes | — |
| 一层 | *yīcéng* | ground storey — **no wall at all** [Y6]; the open working / livestock / storage level | — | posts |
| 二层 | *èrcéng | living storey — enclosed by 木制围护墙 timber enclosure and partition walls, **board thickness 20–25 mm, nailed to the 穿条** [Y6] | enclosure | posts / 穿条 |
| 檐廊 / 前廊 | *yánláng / qiánláng | the deep covered gallery at the upper storey, under the eave | yes | posts |
| 晒台 | *shàitái | the drying deck — an **unroofed external platform** | — | posts |
| 人字架 | *rénzìjià | the "A-frame": **two raking beams** carry the roof, passing the load to the 大梁 and then the posts; the raking beam meets the centre post with a **single notch** [Y6] | yes — the whole roof structure | posts |
| 斜梁 / 大梁 | *xiéliáng / dàliáng | raking beam / main beam of the A-frame | yes | — |
| 檩条 | *lǐntiáo | purlins, **nailed** to the raking beams [Y6] | yes | 斜梁 |
| 椽条 / 挂瓦条 | *chuántiáo / guàwǎtiáo | rafters fixed perpendicular to the purlins, with **battens between them**; the 缅瓦 tiles are **hung** on those battens [Y6] | yes / tile support | 檩 |
| 缅瓦 | *miǎnwǎ | the Dai palm-fibre tile, hung not laid; the upper tile laps directly over the lower, vertical joints staggered; at ridge and junctions 筒瓦 barrel tiles, sealed with **cement** [Y6] | covering | battens |
| 重檐 | *chóngyán | the **lower second roof tier**, which the Dai frame deliberately gives a **smaller pitch** so that, at equal volume, it throws a larger shadow [Y6] | yes | posts |
| 火塘 | *huǒtáng | the hearth — dimension **not established**, see §7 | — | floor |

**The material division of labour in the Dai frame must be stated or a generator will pick one
material**: [Y5] records that from the 1970s the bamboo houses were rebuilt in **bamboo-and-timber**
because bamboo is termite-attacked, with **timber for the main frame** (卯榫结合, mortise and tenon)
and **bamboo for purlins, rafters, floor, screen walls and railings**. The name 竹楼 survives; the
load-bearing members are wood.

## Numbers

Every figure carries its source tag. **Ranges without a stated sample are ranges from a single
unsurveyed text and should be treated as a single observation, not a distribution** — see §3.8.

### The Bai 坊 *fāng* as a unit

| quantity | value | source | confidence |
|---|---|---|---|
| bays per 坊 | **3** (三间) | [Y1] "平面布局多以三个开间组成一坊为单元" | high — stated as the definition |
| side-bay width 开间 | **3.3 – 3.6 m** | [Y1] | low — no sample, no building named |
| central 堂屋 bay width | **3.6 – 3.8 m** (wider than the sides) | [Y1] | low — same |
| 坊 depth 进深 | **5 – 6 m** | [Y1] | low |
| 厦子 depth: eave column 前檐柱 to king column 金柱, centre to centre | **1.8 – 2.0 m** | [Y1] "前檐柱至金柱中距1.8米至2米" | low, but **the single most useful number for a generator** — it is the gallery depth expressed in the frame's own terms |
| storeys | **2** | [Y1], [Y2] (both say 三坊每坊皆三间二层) | high |
| 正房 floor level above 厢房 | **≈ 0.15 m** | [Y1] "主房比厢房室内水平高15厘米左右" | low, but note it is a *hierarchy* rule, not a dimension |
| 漏阁 size | two rooms, **shallower in 进深 and narrower in 开间 than the 正房** | [Y1] | high as a rule; the magnitudes are **not established** |
| 麻雀台 | exists at the 正房/厢房 corner above the 漏阁 | [Y1] | high; dimensions **not established** |

**Derived proportions** (my arithmetic on [Y1], not measurements): 坊 total width ≈ 3×3.5 ≈ 10.5 m;
depth 5–6 m of which the 厦子 is 1.8–2.0 m, leaving **3–4 m of enclosed depth**. So a 坊 is roughly
**10.5 m × 5.5 m**, and its covered gallery is **about 35 % of its depth**. Those ratios are *my*
composition of an unsampled range, not a surveyed value.

### Bai courtyards and 照壁 *zhàobì*

| quantity | value | source | confidence |
|---|---|---|---|
| 三坊一照壁 courtyard, 先锋街20号杨宅 (Lijiang) | **面阔三间 × 进深三间** — three bays wide by three bays deep | [Y2] | high for that one building |
| courtyard, 七一街星火巷41号李宅 (Lijiang) | **面阔约两间** — about two bays wide, i.e. *half* the first | [Y2] | high for that one building |
| rural 三坊一照壁 courtyard (白华大队中心二队和宅) | **larger** than the urban ones, because it must also take crop-drying and grain processing; paved smooth rather than in stone | [Y1][Y2] | high as a rule; the size **not established** |
| courtyard proportion of plot | **not established** — no source gives plan areas for Bai | — | — |
| 照壁 in a dwelling: thickness | **≈ 0.60 m** | [Y1] | low |
| 照壁 centre height | **≈ 6.5 m** | [Y1] | low |
| 照壁 flanking height | **≈ 1.1 m lower** than the centre | [Y1] | low |
| relationship of the flanking ridge to the wing | the flanking 瓦脊 must sit **below the underside of the 厢房 upper-floor eave 虎牙** | [Y1] | high as a rule — it is a *relative* datum, and a relative datum is safer than an absolute height |
| 照壁 墙裙 (dressed-stone base, laid on both faces) height | **≈ 1.0 m** | [Y1] | low |
| 翼形花边砖 thickness | **3 cm** | [Y1] | low |
| 照壁 centre eave upturn 起翘 | **≈ 20 cm** | [Y1] | low |
| 照壁 head form | 四撇水 — four-way drainage, both corners lifted vertically and projecting horizontally to blend with the centre | [Y1] | high as a rule |
| village-head / temple 照壁: thickness | **≈ 0.80 m**, height **≈ 5.0 m**, free-standing | [Y1] | low |

Note that 6.5 m and 0.60 m are **one unsampled sentence** in one monograph. They also sit oddly
against the 3.3–3.6 m bay: a screen wall 6.5 m tall against a two-storey wing implies an eave
around 5.5–6 m, which is plausible but nowhere confirmed.

### Bai 门楼 / 大门 (三滴水 *sāndīshuǐ* "three-drip" gate)

| quantity | value | source |
|---|---|---|
| 门墩 projection beyond the gable face | **≈ 0.50 m** | [Y1] "在突出山墙面约50厘米的大门墩上作制作" |
| 灯笼短柱 horizontal offset from the 门墩 | **≈ 0.60 cm** — i.e. two short "lantern" posts 0.60 m from the pier | [Y1] |
| 斗拱 grades | 一斗七架 / 一斗五架 / 一斗三架 — the grade expresses 门第 (status) | [Y1] |
| 翼角飞椽 pinned with | steel dowels, or round nails **15 cm long** | [Y1] |
| gate plan | set **against a gable**, locally called 过山门 | [Y1] |
| gate position, Dali | "大门大都开在东北角上", and the gate must **not** open straight into the yard — a screen is placed in front, usually with 福 on it | [Y1] |
| gate position, Lijiang 杨宅 | **south-east corner**, at the end of the east wing's 厦子, facing south | [Y2] |

**A direct disagreement**: Dali gates are said to be at the north-east corner [Y1]; the Lijiang
surveyed example puts its gate at the south-east [Y2]. Both can be true — the two are different
towns with different terrain conventions — but a generator must treat **gate corner as a parameter,
not a constant**.

### The shared 穿斗式 frame and its envelope (Bai, Naxi and 一颗印 alike)

From [Y6], a 2025 peer-reviewed field survey of five Yunnan timber systems [academic]:

| quantity | value | source |
|---|---|---|
| post layout | posts run **along the depth direction** (沿进深方向), **closely spaced**, tops receiving the purlins | [Y6] |
| post spacing | **"间距较小" — smaller, no number given. NOT ESTABLISHED.** | [Y6] |
| column diameter, beam section for the Bai/Naxi/一颗印 穿斗 frame | **NOT ESTABLISHED** — [Y6] gives diameters only for the *flat-roofed* Tibetan types (外柱 0.6–1.0 m; 一层内柱 square 0.24–0.5 m; 二层内柱 round 0.4–0.6 m; 中心柱 ≈0.8 m; 碉楼 0.15–0.25 m), which are a different system and **must not be transferred** | [Y6] |
| 瓜柱 / 京柱 split | frames are divided into **有瓜柱 / 无瓜柱**; in Yunnan the 瓜柱 *and* the landing post in line with it are both called **京柱**; hence 京柱落地 vs 京柱不落地 are two frame types | [Y6] |
| tenon types | 箍头榫 (hoop), 燕尾榫 (dovetail), 二蹬榫, 直榫 (through / half-through); grouped as 平肩类 (to square members) and 抱肩类 (to round members) | [Y6] |
| purlin assembly (Bai roof) | ridge purlin = **3 members** (1 挂枋 + 2 round 盖梁); every other purlin = **2** (1 挂枋 + 1 盖梁); a 千斤担 under the 挂枋 | [Y1] |
| beam frame forms | 五架梁 / 七架梁, **count chosen by the depth**; frame profiles 金片结构 or 工字形 | [Y1] |
| floor beam | **two large 枋 stacked**, with a 扣承枋 above controlling the joist positions | [Y1] |
| rafter 椽条 | round timber, **⌀ 60–80 mm** | [Y6] |
| rafter spacing | **220–240 mm** | [Y6] |
| tile laying rule | **两椽支一** — one course of tiles spans two rafters | [Y6] |
| panel-tile lap | **1/4 – 1/3 of the tile length** (after 范雪峰 2005) | [Y6] |
| adobe brick 土坯 | length **300–330 mm**, width **150–170 mm**, thickness **100–120 mm** | [Y6] |
| 土坯墙 thickness | **500–600 mm** | [Y6] |
| 夯土墙 thickness | **400–600 mm** | [Y6] |
| brick wall | 二四墙 (nominal 240 mm), red or grey brick; a 砖贴土 facing variant, and 四角砖柱 with rammed earth infill, are both used | [Y6] |
| 地脚枋 (ground tie beam between posts) | **present when the infill is timber; absent when it is brick or adobe** | [Y6] — a real generative rule |
| foundation | 地下基础 + 台基 + 柱础石; usually **stone strip under the wall plus isolated pads under the posts**; posts **rest loose (平摆浮搁) on the column-base stone**, friction-governed | [Y6] |
| load path | **all weight carried by the frame; walls are enclosure only** | [Y1] |

### The 一颗印

| quantity | value | source | confidence |
|---|---|---|---|
| standard form | **三间四耳倒八尺** | [Y3] | medium — a signed feature by a named academic, tracing to [Y4] |
| 正房 total width 面宽 | **≈ 12 m** over three bays → **≈ 4 m per bay** | [Y3] | medium |
| 倒座 depth | **八尺**, glossed by the source as "**2 m 多**" | [Y3] | medium; the source's own gloss is the number to use, since the 营造尺 conversion was not verified this session |
| site | a rectangle **≈ 12 m × 15 m** | [Y3] | medium |
| 天井 | **≈ 3 m square** | [Y3] | medium |
| courtyard as fraction of site | 3 × 3 / (12 × 15) ≈ **5 %** — my arithmetic on [Y3], and it is the number that makes the type legible: the yard is a *light well*, not a garden | derived | — |
| storeys | **2** throughout (正房 and 耳房 both) | [Y3] | medium |
| eaves over the yard | 腰檐 — **大厦** over the 正房, **小厦** over the 耳房; **all four connect** so the yard perimeter is walkable in rain | [Y3] | medium |
| 耳房 roof | **内长外短** — longer pitch inside, shorter outside, throwing water into the courtyard (四水归堂) and raising the outer wall for wind, fire and theft | [Y3] | medium |
| circulation | **楼梯巷**, a stair lane; **its plan position is not stated** | [Y3] | — |
| wall openings | **"开窗极小"** — very small; no dimension given | [Y3] | — |
| frame | 穿斗式, in local 杉木 / 松木 | [Y3] | medium |
| walls | 土坯 or 夯土, thick | [Y3] | medium |
| roof | 小青瓦 | [Y3] | medium |
| courtyard paving and drainage | 青石板 slabs with a **暗沟** covered drain around the perimeter | [Y3] | medium |
| recorded variants | 三间两耳倒八尺 · 三间六耳倒八尺 · 三间两耳 **半颗印** · **下店上居** · **前店后居** | [Y3] | medium |
| earliest record | a Ming Wanli-period note that Yunnan dwellings were "四合瓦屋" | [Y3] | medium |

### The Dai 干栏 / 竹楼

The **strongest numbers in this note for the Dai type** come from [Y6]; the overall dimensions come
from a folkloric web page and are the weakest thing here — see the confidence column.

| quantity | value | source | confidence |
|---|---|---|---|
| roof pitch, upper part | **45° – 60°** (after 刘一 2011) | [Y6] | medium — one thesis, range given |
| — as slope (rise/run) | tan 45° = **1.00**; tan 60° = **1.73**, i.e. 1:1.0 to 1:0.58 (rise:run) | **my arithmetic** on [Y6] | derived |
| ridge form | **人字型**, splitting into **two unequal pitches**; the **upper ends are 歇山** (hipped-gable) | [Y6] | high as a rule |
| lower part | a **重檐** second tier, given a **smaller pitch deliberately** so that at equal volume it casts a **larger shadow** | [Y6] | high as a rule — and it is a *reason*, which is what a generator wants |
| vertical offset between longitudinal and transverse 穿枋 | **≈ 30 cm** — the two tie directions are **not coplanar** | [Y6] | medium |
| upper-storey wall board | **20–25 mm**, nailed to the 穿条 | [Y6] (after 马若予 2015) | medium |
| ground storey | **no wall of any kind** | [Y6] | high |
| posts | **all square-section**; many — 千脚落地房 | [Y6] | high |
| post spacing | **NOT ESTABLISHED** — no figure in any source read this session | — | — |
| foundation | 柱础石 **half-buried**; **some houses have no foundation at all** | [Y6] | medium |
| joinery | 馒头榫 (roof to frame), 直榫 through / half-through (ties to posts), **单齿接** single notch (raking beam to centre post, beam to beam) — the simplest of the five Yunnan systems | [Y6] | high |
| roofing build-up | 檩条 nailed to 斜梁 → 椽条 perpendicular to the purlins → **挂瓦条 battens** → 缅瓦 **hung** on the battens; upper tile laps the lower directly, vertical joints staggered; 筒瓦 at ridge and junctions, sealed with cement | [Y6] | high |
| materials, post-1970 | **timber for the frame**, bamboo for purlins, rafters, floor, screen walls and railings — because of termite attack | [Y5] | medium |
| plan area | "60、70 平方米, 是正方形的" | [Y7] | **very low** — the page states 来源：网络 with no author, and this may be **one house described in folklore, not a survey** |
| floor height above ground | "楼板距地面约二米五" (≈ 2.5 m) | [Y7] | **very low** — same page |
| stair | "楼梯一般为九级" (generally nine steps) | [Y7] | **very low** — same page; at 2.5 m over 9 risers this is ≈ 278 mm per riser, internally consistent, which is *not* evidence |
| upper-storey sequence | stair → **走廊** corridor → **阳台** balcony | [Y7] | very low, but corroborated in kind by [Y5] |
| internal split | bamboo wall divides **堂屋** (outer, with the **火塘** hearth) from **卧屋** (inner) | [Y7] | very low; the hearth is corroborated in kind by [Y5] |
| vernacular nickname | 凤凰展翅, "phoenix spreading its wings" | [Y5] | medium |
| **Dehong / 瑞丽 variant** | a two-storey bamboo house, but **屋面脊长，出檐短** — *long ridge, short eave* — and galvanised corrugated sheet usually replacing the thatch | [Y5] | medium, and it **directly contradicts** the "deep eave" reading of the Xishuangbanna form |
| **Yuanjiang / Honghe variant** | **平顶土掌房** — a heavy, flat-roofed rammed-earth house, a *different type*, not a variant of the stilt house | [Y5] | medium |
| noble houses | same form, larger and more spacious, **timber-built and tile-roofed** | [Y7] | very low |

### Yunnan-wide construction dimensions (the shared kit)

Included because the 穿斗 frame of all three courtyard types is specified by the same numbers.

| element | value | source |
|---|---|---|
| 闪片 split board | **100–120 cm long × 15–18 cm wide × 2–3 cm thick**, split along the grain of green pine (after 范雪峰 2005) | [Y6] |
| 闪片房 外柱 diameter | **0.6 – 1.0 m** | [Y6] |
| 闪片房 一层内柱 | square section, **0.24 – 0.5 m** | [Y6] |
| 闪片房 二层内柱 | round section, **0.4 – 0.6 m** | [Y6] |
| 闪片房 中心柱 | **≈ 0.8 m** | [Y6] |
| 土掌碉楼 post | **0.15 – 0.25 m**, usually 分体柱 (split posts), 2½ or 3½ storeys | [Y6] |
| rammed-earth wall batter (闪片房 / 碉楼) | **bottom width 0.6–0.7 m, top width 0.4–0.5 m** — a pronounced inward taper from the base | [Y6] |
| flat-roof build-up | purlins → boards → an **even layer of dry grass** → **≈ 200 mm of rammed earth** | [Y6] |
| 人字架 | two raking beams; raking beam ↔ centre post **single notch**; raking beam ↔ main beam **single or double notch** | [Y6] |

### Where sources disagree, and which I trust

1. **白族 main-wing orientation.** [Y1]: 白族民居的主房一般是坐西向东 — the main wing faces
   **east**, which is what makes the 照壁 work (the wall must close the side the wing faces).
   [Y2]'s Lijiang 杨宅: 正房一坊朝南，面对照壁 — the main wing faces **south**. **I trust both
   for their own region**; [Y2] is a named measured building and [Y1] is a generalisation, but the
   difference is real and regional. A generator needs an orientation parameter, not a constant.
2. **Gate corner.** Dali north-east [Y1]; the Lijiang survey south-east [Y2]. Same conclusion —
   a parameter, not a constant.
3. **Dai eave depth.** Xishuangbanna is described as steep and deep-eaved [Y6]; the Dehong 瑞丽
   house is explicitly **脊长出檐短** — long ridge, short eave [Y5]. **I trust both**: they are
   different sub-regions with different rain and different available tile. A single "Dai roof"
   parameter would be wrong.
4. **Bai veranda terminology.** [Y1] names the 坊's own gallery roofs **大出厦 / 小出厦**; [Y3]
   names the 一颗印's courtyard eaves **大厦 / 小厦**. These are **not the same element** and must
   not be merged by a fuzzy-match generator.
5. **How much to trust the whole of §3.1–3.3.** [Y1] is one monograph chapter, reproduced verbatim
   on at least three unrelated sites ([Y1], [Y8], and a second copy of the same text) with **no
   sample size, no named buildings and no drawing references anywhere in it**. I treat it as
   **[academic] for rules and topology** (which are internally consistent and match [Y2]'s
   independent survey of eight named Lijiang houses) and as **a single observation for every
   number**. The numbers it gives are internally consistent but unverifiable.
6. **Bai courtyard size.** [Y2] gives two named buildings whose courtyards differ by **half**
   (三间 vs 两间 wide) [Y2]. So a "typical courtyard" for this type does not exist; the plot
   governs it, and a generator must be driven by site width.

### Numbers I could not establish, and the measurement that would settle each

| missing number | what would settle it |
|---|---|
| Bai/Naxi/一颗印 **post spacing and post diameter** in the 穿斗 frame | a measured drawing (测绘图) of one named house, with the frame plan dimensioned — the 剑川 studies [Y10][Y11] are exactly this and were not obtainable (§7) |
| Bai **storey height** and **净高**; floor-to-floor | the same measured drawing |
| Bai **roof pitch**, in degrees or as rise:run — [Y1] gives 五架梁/七架梁 purlin *counts* and a depth, never an angle | a measured section through a named Bai house; 范雪峰's thesis [Y6 ref.] is titled on exactly this and was not obtainable |
| Bai **出檐** eave overhang, in projection, beyond the wall face | the same section; note [Y1] describes the flying-eave stone's *position* and grading but never its projection |
| Naxi bay width / depth / storey height / roof pitch | 潘曦's 清华大学 doctoral thesis [Y6 ref.] or a 大研古城 measured survey |
| Dai **post spacing**, **floor-to-floor height**, **stair rise**, **eave projection**, **ridge length** | a measured drawing of one named Xishuangbanna house — 郭建伟's *建筑遗产* paper [Y6 ref.] is titled on the Dai's major timber members and was not obtainable |
| 傣族 **火塘** dimensions | a plan of the upper storey |
| 一颗印 **stair-lane position** and **腰檐 projection** | the 1938 刘致平 measured survey [Y4], which exists and was not obtainable |
| 四合五天井 corner-skywell dimensions | a plan of a surveyed 四合五天井; 严家大院 at Xizhou (built 1907, ≈ 2478 m², two 三坊一照壁 courts plus two 四合五天井 courts tied by a 走马转角楼 [Y9]) is the obvious candidate but I found no drawing set |
| Bai **wall thickness** for the stone base / 毛石墙 / 土基墙 build-up | a section; [Y1] gives only the 照壁's 0.60 m |

## How the type varies

### By region

The province's timber housing partitions cleanly by region, and [Y6] is explicit about it: 穿斗式
is "the most numerous and most widely distributed"; 干栏式 is **southern Yunnan** (Dai, Hani,
Blang, Wa); 井干式 is **north-west Yunnan** (Lisu, Pumi, Mosuo); 平顶式 is the **cold north and the
dry-hot valleys** (Tibetan, Yi); 人字架 is a simpler, cheaper variant that **mixes with 穿斗式** in
the same districts. My four types sit as: Bai / Naxi / 一颗印 in 穿斗式, Dai in 干栏式. So **region
in Yunnan is not decoration — it selects the structural system**, and a generator that treats
Yunnan as one style family will produce a 一颗印 with a Dai roof.

Within the Bai, [Y1] gives the physical reason for the gate and the orientation: Dali sits in a
valley-basin "fanned" by the north–south Hengduan ranges, so the house **must sit with its back to
the mountain, 依山傍水**, giving 坐西向东 rather than the Beijing courtyard's 坐北朝南. Terrain sets
the orientation; orientation sets the gate corner; gate corner sets which side the 照壁 goes on.

### By ethnicity — and by what they will actually build

[Y6] states the cause plainly: 不同木结构的抗震能力受风俗习惯、气候条件、经济水平等因素影响 —
seismic capacity varies with **custom, climate and economic level**. Those three are the generator's
axes. Concretely:

- **白族 (Bai)** build in 石木结构 (stone-and-timber) or 土木结构 (timber-and-earth) [Y1], with
  stone preferred where it exists — 大理民间有理有三宝，石头砌墙墙不倒 [Y1]. The type is already a
  two-storey courtyard type; the question is only how much of the 照壁 and 门楼 ornament it can
  afford. [Y1] names an **economised 照壁** explicitly — no 斗拱 ornament under the eave stone, only
  飞砖/飞瓦 lines and ink painting — and says this is what ordinary people built, including at
  昆明海埂"白族村" and in the Shenzhen recreation. So **economy is an explicit variant, not an
  accident**, and a generator should be able to switch ornament off.
- **纳西族 (Naxi)** in Lijiang: same frame, and [Y6]'s bibliography titles the field's own
  understanding of it as a **建造范式** (construction paradigm) [Y6 ref. 潘曦 2014], i.e. a rule set
  rather than a shape. The verified Lijiang-specific moves are the 厦子 gallery, the
  头梁合抱柱 shared column *or* the 照面梁榫卯 + 跑马转角楼 alternative, and the 悬鱼 gable pendant.
- **傣族 (Dai)** build in three distinct regional forms — Xishuangbanna stilt, Dehong/Ruili stilt
  with a long ridge and short eave, Yuanjiang/Honghe **flat-roofed 土掌房** [Y5] — and the name 竹楼
  is now a misnomer for the structure, which since the 1970s is timber-framed [Y5].
- **汉族 in 滇中** build the 一颗印, and [Y3] records it as a **Han** courtyard form specific to the
  central plateau, with a documented urban history from the 1930s [Y4].

### By wealth

The clearest documented gradient in this slice is the **规模 hierarchy within the 一颗印**
[Y3]: 三间四耳倒八尺 (standard) → 三间两耳倒八尺 and 三间六耳倒八尺 (ear count varied for family
size and means) → 三间两耳 **半颗印** (a "half seal", for constrained sites) → **下店上居 / 前店后居**
(shop below / dwelling above, shop front / house behind — the urban street variant). The *site
envelope* is held constant and the **wing count is the dial**. That is the single most useful
generative idea in this note: **these houses grow and shrink by adding and removing ears, not by
resizing the main range.**

The Bai equivalent is documented too: [Y2] records a family that first built **二坊拐角** — two wings
meeting at a right angle, which "cannot be built opposite" — and later completed the third wing when
they could afford it; the text is explicit that this is a *temporary* form pending 三坊一照壁.
[Y2] also records the land-value gradient: on a tight urban plot the 漏角屋 drop to one storey and
the gate takes a whole bay of a wing; on a rural plot the courtyard grows, the 漏角屋 keep two
storeys, the stone paving is replaced by smooth earth for crop work, and an 八字墙 replaces the
shared column precisely **because detaching the wings enlarges the yard**.

### By terrain

Three documented responses, all in this slice:

- **A narrow urban plot** shrinks the courtyard and the corner annexes, and swaps the shared column
  for a separate wall [Y2].
- **A sloping site**: 一颗印 has **no fixed orientation** — it can follow the slope [Y3] — and the
  楼梯巷 is how a split level is absorbed inside a square plan.
- **A hot, wet, insect-driven site**: the Dai rebuild in bamboo-and-timber [Y5]; the Bai rebuild in
  stone-and-timber [Y1]; the 一颗印 thickens its walls and shrinks its openings [Y3].

### Which parameters a generator should expose, and which it should fix

**Expose** (they vary house to house and are the actual design decisions):

- number and arrangement of wings / ears / ranges — the primary dial in every one of the four types
- bay count per wing; **bay widths as a list**, because the central 堂屋 bay is wider [Y1]
- wing depth; 厦子 depth (1.8–2.0 m) — the gallery is a first-class parameter, not a fudge factor
- storey count; floor-to-floor height; whether the corner annexes are two storeys or one; whether
  one annex has **no floor** (the kitchen, built tall "but without a storey, so smoke can escape" —
  [Y2])
- whether wing junctions **share a column** or are **separated by a wall** [Y2] — a boolean that
  changes both the plan and the frame
- whether the upper gallery **connects** (跑马转角楼) or each wing is separate [Y1]
- **orientation** and therefore **gate corner** and **照壁 side**
- courtyard width, depth, and paving (stone / smooth earth)
- roof: pitch, purlin count (derived from depth [Y1]), eave projection, eave type — **stone
  flying-eave stone** (Bai, graded 大/中/虎牙), **timber 腰檐** (一颗印, 大厦/小厦), or **steep
  unequal-pitch with 重檐** (Dai)
- the 耳房 **内长外短** roof rule — a boolean with a geometric consequence
- wall material and thickness (夯土 / 土坯 / 二四砖 / 毛石 / 石木), and whether the infill is
  timber (which then **requires 地脚枋** [Y6] and nothing else does)
- ornament level — the economical 照壁 is a documented variant [Y1]
- **region** as the top-level switch, because it changes the structural system

**Fix** (they are structural constants of the type, and a generator that varies them has
mis-modelled it):

- the load path: **frame carries everything, wall encloses** [Y1] — walls are never load-bearing
  here
- 穿斗 frame rule: posts on the depth axis, closely spaced, tops receiving the purlins [Y6]
- the ground floor of a 干栏 house is **un-walled** [Y6]; the upper storey **is** walled
- posts are **square-section** in the Dai system and **rounder** in the 穿斗 system [Y6]
- two rafters carry one tile course (两椽支一) [Y6]
- the 一颗印 courtyard is a **minority of the site area** — roughly 5 % on the cited figures
- the Bai 坊 is **three bays and two storeys** — that is close to a definition [Y1][Y2]
- the 一颗印 is **square** — the whole name is the plan

### Which of the four are documented by measured survey, and which only by description

This is the honest answer to a question the brief asked, and the answer is uneven:

| type | measured survey (测绘图)? | what exists |
|---|---|---|
| **一颗印** | **Yes, historically, and now unobtainable** — 刘致平 surveyed it for 中国营造学社 at 龙泉镇 near Kunming in 1938 and published 《云南一颗印》 with measured drawings [Y3]; it is called the foundational document of the type | the 2025 review paper [Y6] did not repeat its dimensions |
| **白族 三坊一照壁 / 四合五天井** | **Yes, by description of named buildings.** [Y2] describes **eight named Lijiang houses** by street address with their plan topology — 杨宅 (先锋街20号), 李宅 (七一街星火巷41号), 和宅 (龙泉东开七队), 牛宅 (五一街下段31号 and 新华街6号), 赖宅 (七一街八一路46号), 和宅 (白华大队中心二队). **That is a survey; but no dimension appears in the reproduction I read** | [Y1] gives dimensions without naming a building — the inverse |
| **Naxi Lijiang** | **Unresolved** — two theses exist on the frame and the craft [Y6 refs. 潘曦 2014, 解婧雅 2014]; I could obtain neither. No measured dimension reached this note | description only |
| **Dai 干栏** | **Partly.** [Y6] gives construction dimensions (roof angle, board thickness, tie offset, rafter/purlin rules) from field work, but **no plan dimensions and no post spacing**. The only overall figures are a folkloric web page [Y7] | field data on the frame; description on the plan |

**So: the 一颗印 has the best provenance and I still could not get its numbers; the Bai type has
the most named buildings and the fewest numbers; the Naxi type is the largest hole; and the Dai
type has good frame numbers and no plan.** That distribution — rules everywhere, dimensions almost
nowhere — is the single most important finding for a modeller, because it means the generator's
**rules** can be built from this note with confidence and its **numbers** must come from drawings
this session could not reach.

## What a generator would need

### A parameter list, in data terms

Names a modelling program can hold. `enum` values are given where the choice is closed;
`vec<f>` is a list. "trusted" means the value is usable as a default; "single obs." means it came
from one unsampled sentence and should be treated as a starting point only.

#### Type level — the switch that picks a structural system

| parameter | data type | values / units | source |
|---|---|---|---|
| `region` | enum | `dali_bai` · `lijiang_naxi` · `kunming_yikeyin` · `xishuangbanna_dai` · `dehong_dai` · `yuanjiang_tu掌` | [Y5][Y6] |
| `frame_system` | enum | derived from `region`: `chuandou` \| `ganlan` — **not a free choice** | [Y6] |
| `typology` | enum | `fang` (wing) · `winding_court` (一颗印) · `stilt_block` (Dai) | [Y2][Y3][Y6] |

#### Module

| parameter | data type | values / units | source |
|---|---|---|---|
| `bays_per_wing` | int | 3 for Bai (near-definitional) | [Y1][Y2] |
| `bay_widths` | vec<f>, m | e.g. `[3.3, 3.7, 3.3]` — **the central bay is wider** | [Y1], trusted low |
| `wing_depth` | f, m | 5–6 | [Y1], single obs. |
| `gallery_depth` | f, m | 1.8–2.0, measured 前檐柱→金柱 centre-to-centre | [Y1], single obs. but highest-value |
| `enclosed_depth` | f, m | derived: `wing_depth − gallery_depth` | derived |
| `courtyard_widths` | vec<f> | one per side | [Y2] |
| `courtyard_aspect` | f | **not established for Bai**; ≈1.0 for 一颗印 | [Y3] |
| `courtyard_fraction_of_site` | f | ≈0.05 for 一颗印 | derived from [Y3] |
| `eave_count_per_side` | enum | `square_chi` (一颗印: 4 耳房) · `three` (三坊) · `four` (四合) | [Y2][Y3] |
| `wing_count` | int | 2 (二坊拐角) … 3 … 4 | [Y2] |

#### Section

| parameter | data type | values / units | source |
|---|---|---|---|
| `storeys` | int | 2 for Bai, Naxi, 一颗印 | [Y1][Y2][Y3] |
| `ground_storey_walled` | bool | `false` for 干栏 (Dai); `true` for the courtyard types | [Y6] |
| `floor_height` | f, m | **not established** for any of the four | — |
| `floor_to_floor` | f, m | **not established** for any of the four | — |
| `main_wing_floor_rise` | f, m | 0.15 (正房 above 厢房) | [Y1], single obs. |
| `annex_storeys` | int | 2, or 1 (tight plot), or 2-with-**no floor** (the kitchen, for smoke) | [Y1][Y2] |
| `annex_depth_ratio` | f | annex is shallower than the main wing; magnitude **not established** | [Y1][Y2] |

#### Structure

| parameter | data type | values / units | source |
|---|---|---|---|
| `post_spacing` | f, m | **not established** for 穿斗 or 干栏; only "较小" (closely spaced) | [Y6] |
| `post_section` | enum | `square` (Dai) \| `round` (穿斗) | [Y6] |
| `post_diameter` | f, m | **not established** for these three types | — |
| `gongzhu_pattern` | enum | `gongzhu_landed` (京柱落地) \| `gongzhu_floating` (京柱不落地) | [Y6] |
| `joinery` | enum | `goutou` 箍头榫 \| `yanwei` 燕尾榫 \| `erdeng` 二蹬榫 \| `zhijie` 直榫 \| `mantou` 馒头榫 \| `single_notch` 单齿接 \| `double_notch` 双齿接 | [Y6] |
| `tie_plane_offset` | f, m | 0.30 — longitudinal vs transverse 穿枋 offset, Dai only | [Y6] |
| `foundation` | enum | `pad_and_strip` (穿斗) \| `point_half_buried_stone` (干栏) \| `none` (some 干栏) | [Y6] |
| `column_base_bearing` | enum | `resting_loose` (平摆浮搁, friction-governed) for 穿斗 | [Y6] |
| `dijiaofang` | bool | ground tie beam between posts — **`true` if and only if the infill is timber** | [Y6] |

#### Enclosure

| parameter | data type | values / units | source |
|---|---|---|---|
| `wall_class` | enum | `rammed_earth` 夯土 \| `adobe` 土坯 \| `brick_2four` 二四砖 \| `rubble_stone` 毛石 \| `dressed_stone` 条石 \| `timber_board` 木墙 \| `bamboo_weave` 竹编 | [Y1][Y5][Y6] |
| `wall_thickness` | f, m | 夯土 0.40–0.60 · 土坯墙 0.50–0.60 · 二四砖 0.24 · **Bai stone/timber: not established** | [Y6] |
| `wall_taper` | f | battered inward with height (0.6–0.7 bottom → 0.4–0.5 top) — documented for the *rammed-earth flat-roof* types; **not established for the Bai/Naxi/一颗印 walls** | [Y6] |
| `plinth_height` | f, m | 1.0 for the 照壁 墙裙; Bai wall 墙裙 height **not established** | [Y1] |
| `plinth_material` | enum | `dressed_stone` (麻条石, laid upright and flat, both faces) | [Y1] |
| `opening_area_budget` | f | 一颗印: "开窗极小" — a policy, not a number | [Y3] |
| `adobe_brick` | vec<f>, mm | `[300–330, 150–170, 100–120]` | [Y6] |

#### Roof

| parameter | data type | values / units | source |
|---|---|---|---|
| `roof_form` | enum | `flush_gable` 硬山 \| `overhanging_gable` 悬山 \| `hipped_gable` 歇山 \| `flat_rammed` 平顶 | [Y1][Y5][Y6] |
| `roof_pitch` | f, deg | Bai **not established**; Dai **45–60** | [Y6] |
| `roof_slope` | f | derived `tan(pitch)`: 1.00 … 1.73 | derived from [Y6] |
| `pitch_asymmetry` | f | **> 1 for the Dai roof** — 人字型 with **two unequal slopes** | [Y6] |
| `tier_count` | int | 1 (Bai wing) · 2 (一颗印 耳房 内长外短; Dai 重檐) | [Y3][Y6] |
| `lower_tier_pitch` | f | deliberately **smaller** than the upper, for shadow | [Y6] |
| `drainage_direction` | enum | `outward` \| `inward_four_waters` 四水归堂 | [Y3] |
| `purlin_count` | int | 五架梁 / 七架梁 — **chosen by the depth** | [Y1] |
| `purlin_assembly` | enum | ridge = 3 members, others = 2 members | [Y1] |
| `rafter_diameter` | f, m | 0.06–0.08 | [Y6] |
| `rafter_spacing` | f, m | 0.220–0.240 | [Y6] |
| `tile_rule` | enum | `two_rafters_per_course` 两椽支一 (青瓦) \| `hung_on_battens` 挂瓦 (缅瓦) | [Y6] |
| `tile_lap` | f | 1/4 – 1/3 of tile length | [Y6] after 范雪峰 2005 |
| `tile_material` | enum | `grey_pan_and_barrel` 青筒板瓦 \| `palm_hooked` 缅瓦 \| `thatch` 茅草 \| `corrugated_sheet` 镀锌瓦楞铁 \| `flash_sheet` 闪片 | [Y1][Y5][Y6] |
| `eave_structure` | enum | `framed` (posts + beams) \| `corbelled_stone` 飞檐石 \| `gable_leaning` (gate, 过山门) | [Y1] |
| `eave_projection` | f, m | **not established** for any of the four | — |
| `flying_eave_grade` | enum | `large` 大号 (rear wall) \| `medium` 中号 (gable) \| `tiger_tooth` 虎牙 (front corner) | [Y1] |
| `eave_upturn` | f, m | 0.20 at the centre of a 照壁 | [Y1], single obs. |
| `gable_form` | enum | `flush` (硬山) \| `raised_parapet` 封火山墙 | [Y1] |

#### Court, gate, water, circulation

| parameter | data type | values / units | source |
|---|---|---|---|
| `screen_wall_present` | bool | true in 三坊一照壁 | [Y2] |
| `screen_wall_height_centre` | f, m | 6.5 | [Y1], single obs. |
| `screen_wall_height_flank` | f, m | centre − 1.1 | [Y1], single obs. |
| `screen_wall_thickness` | f, m | 0.60 (dwelling) / 0.80 (village head) | [Y1], single obs. |
| `screen_wall_datum` | enum | `below_wing_eave_tiger_tooth` — a **relative** datum, safer than an absolute height | [Y1] |
| `screen_wall_head` | enum | `four_way_drain` 四撇水, corners lifted in both axes | [Y1] |
| `gate_corner` | enum | `north_east` (Dali) \| `south_east` (Lijiang 杨宅) | [Y1] vs [Y2] — **disagreement** |
| `gate_roof` | enum | `three_drip` 三滴水 \| `single_plane_flush_gable` 一字平屋面 \| `half_hip` 撇水庑殿 | [Y1] |
| `gate_bracket_grade` | enum | `one_cap_seven` 一斗七架 \| `one_cap_five` 一斗五架 \| `one_cap_three` 一斗三架 — encodes status | [Y1] |
| `stair_lane_present` | bool | true for 一颗印 | [Y3] |
| `stair_rise` | f, m | **not established**; the only figure is "nine steps" from a folkloric page | [Y7] |
| `three_eye_well` | bool | Lijiang; **a single head with three apertures** — a topology, not a dimension | [Y2] |

### Comparison table across the four types

| | 白族 三坊一照壁 / 四合五天井 | 纳西族 Lijiang | 一颗印 | 傣族 干栏 / 竹楼 |
|---|---|---|---|---|
| frame system [Y6] | 穿斗式 | 穿斗式 | 穿斗式 | **干栏式** |
| unit of composition | the **坊** (wing), 3 bays × 2 storeys [Y1] | the 坊, same [Y2] | the **range + ear**; site is the fixed rectangle [Y3] | the **single block** on posts [Y6] |
| storeys | 2 [Y1][Y2] | 2 [Y2] | 2 throughout [Y3] | 2, ground storey **un-walled** [Y6] |
| plan | 3 or 4 wings + 照壁; courtyard = 3 bays × 3 bays (杨宅) or ~2 bays (李宅) [Y2] | same, plus 走马转角楼 [Y2] | **square**, 12 × 15 m site, 3 m square yard [Y3] | square-ish, one block [Y7] |
| how it grows | add a wing (二坊拐角 → 三坊) [Y2] | add a wing | **add/remove 耳房** [Y3] | rebuild with a larger frame |
| walls | stone-and-timber / timber-and-earth; load path is the frame [Y1] | timber frame + 土坯 [Y6 refs.] | 夯土 or 土坯, very thick, tiny openings [Y3] | none below; 20–25 mm board above [Y6] |
| roof | **硬山**, purlin count chosen by depth, **stone** flying eaves [Y1] | 硬山 with 悬鱼 pendant [Y2] | 小青瓦, **腰檐** over the yard, 耳房 **内长外短** [Y3] | **45°–60°**, unequal, 歇山 ends, **重檐**, 缅瓦 hung on battens [Y6] |
| the signature move | the **飞檐石** — [Y1] calls it the thing that distinguishes this house from every other | the **头梁合抱柱** shared column, or the 转角楼 | the **3 m square yard** inside a 12 × 15 m box | the **千脚落地房** ground storey |
| best provenance | 8 named surveyed houses, **no dimensions** [Y2] | **none obtained** | 1938 刘致平 measured survey, **not obtained** [Y3][Y4] | field dimensions on the frame, **none on the plan** [Y6] |

### The constructs the current flat-plate volume vocabulary does not have

Stated as constructs, not as code. The frame given in the brief is *a wall ring plus plates, a
subtracted courtyard ring, everything polygon-offset in plan, every roof a flat slab*. Named
against that, these types need:

1. **A frame as a first-class object, independent of any wall.** Posts, then beams between them,
   then purlins on the posts. [Y1]: 建筑物重量全部由构架承担，墙壁只起围护作用. Everything else in
   this list follows from this one. A wall ring cannot express "this wall is carrying nothing and
   that member is carrying everything".
2. **Post spacing as a rule, not a placement.** [Y6]: posts run along the depth axis at a small,
   regular spacing. Spans are a *consequence* of a grid, not authored per wall.
3. **An infill panel** — a wall that occupies one frame bay, has a thickness and a material class,
   is explicitly non-loadbearing, and can be absent (the Dai ground storey) or present (the same
   storey in a 穿斗 house). The current "ring" cannot be absent.
4. **A pitched roof as a surface over a ridge**, with a per-side pitch, *independently pitched
   sides*, and a termination at the ends (flush gable, overhanging gable, hipped gable). A flat
   slab cannot be tilted, and cannot have two different tilts.
5. **A roof supported by discrete members** — purlins, rafters, battens — each with a diameter and
   a spacing, because the tiles are carried by them and the tile module derives from the rafter
   spacing (两椽支一, [Y6]). A roof plate has no such structure.
6. **A tile as a repeating module on a pitched surface**, with a lap and a course rule. Every one
   of these roofs is read as a texture of discrete units; that is most of what a camera sees.
7. **A second roof tier** (腰檐, 重檐, 内长外短) at a different level and a different pitch,
   projecting *inward* over the courtyard. "One roof per building" cannot express this, and this
   is the geometric signature of the 一颗印 and the Dai house.
8. **A corbelled eave** — the flying-eave stone: a member projecting out of a wall on corbels,
   graded by which wall it sits on, with 飞砖/飞瓦 courses under it. This is an eave **carried by
   the wall**, which is the opposite of the framed eave in the Dai house; the two are different
   constructs, not one parameter.
9. **A joint** — a notch or mortise-and-tenon as an object that *joins* two members with a named
   type, so that "the raking beam is notched onto the centre post" is representable and a
   non-notched contact is distinguishable from a notched one.
10. **A stilt**: posts standing clear of the ground on base stones, carrying a floor plane above,
    with the space below left as open volume. There is no room for this in a wall ring; it is a
    second storey's worth of structure with no enclosure.
11. **A column-base stone as a distinct small object**, with the column resting *loose* on it —
    [Y6] notes this is why low-intensity shaking moves the column foot. A model that fuses column
    to foundation loses the joint that actually fails.
12. **A stone plinth / 墙裙**: a dressed-stone band of its own height and material under a wall or a
    screen wall, laid upright and flat on both faces. Distinct from the wall above it.
13. **A battered wall** — thickness reducing with height. [Y6] measures this on rammed earth
    (0.6–0.7 → 0.4–0.5); a polygon-offset-in-plan vocabulary has one thickness per wall.
14. **A ground tie beam (地脚枋)** — a member that exists **iff the infill is timber** [Y6]. A
    conditional member whose presence is derived from another parameter.
15. **A screen wall as a freestanding object**, not part of any ring, with its own tiny four-way
    roof, its own corners lifted in two axes, and a *relative* datum to the wings' eave.
16. **A gate block as a sub-assembly** — two piers, two columns, a bracketed head, a three-drip
    roof — with a status grade. It is an object that attaches to a wing, not an opening in a wall.
17. **A courtyard as an authored void with its own datum** — a target size, an aspect, a share of
    site area, a paving class and a drain — rather than as whatever is left after subtracting a
    ring.
18. **A stair as a run with a rise, a going and a landing**, plus the 一颗印's **楼梯巷**, which is
    a *plan-occupying lane* and therefore a massing decision, not a fitting.
19. **A reusable wing as an instance**, so that a courtyard type is an *arrangement* of wings and the
    same wing appears four times with different neighbours. [Y2]'s 二坊拐角 → 三坊一照壁 growth is
    only expressible if the wing is an instance.
20. **A terrain-following base**, since the 一颗印 "has no fixed orientation and can follow the
    slope" [Y3] and the 木楞房 types build a 平座 platform to absorb level change [Y6].
21. **A wall-material classification** with more than one class — rammed earth, adobe, brick,
    rubble stone, dressed stone, timber board, bamboo weave — because the *same* house in the same
    bay can be any of these and because [Y6]'s damage findings attach to the material, not the
    shape.
22. **A settlement-fabric construct**: the house is a **plot-frontage** object. [Y2]'s gate sits in
    the end bay of a wing's gallery facing the street, and the 一颗印's 倒座 is a shop range on the
    street front — so street, frontage width and party-wall sharing are what determine the plan,
    not the building type.

## Sources

Access date for every entry: **2026-10-18**. "Full text read" means I read the text; "abstract
only" means the landing page only; "identified" means the bibliographic record is confirmed but
the document was not obtained.

### Read in full this session

- **[Y1]** 《白族民居的建筑形式及基本构造》 — an excerpt of 《云南大理白族民居建筑风格》 as printed
  in **《大理建筑文化论》, 云南民族出版社, 2006-10**. Full excerpt read.
  https://www.renrendoc.com/paper/160392875.html — host **[unverified]** (user upload, 2021-11-02,
  account "于\*\*", Tianjin); the *content* is a published monograph and is tagged **[academic]**.
- **[Y2]** 《三坊一照壁》 entry, 建设工程百科 / 造价通百科 (zjtcn.com). Full page read.
  https://www.zjtcn.com/baike/sanfangyizhaobi — host **[unverified]**, but the body is a **verbatim
  reproduction of the Lijiang chapter of 陆元鼎主编《中国民居建筑》** with its eight named measured
  examples intact, so it is cited as **[academic] via reproduction**. No dimensions appear in it.
- **[Y3]** 翟辉, 《连接过去与未来的"一颗印"（文化中国行·华夏博物之旅·传统建筑）》, 人民日报,
  2026-01-03 (人民号 platform). Full text read.
  https://www.peopleapp.com/column/30051125362-500007288099 — **[travel/blog]** (newspaper feature;
  the author is a named professor, 昆明理工大学建筑与城市规划学院). Used only where attributable to
  [Y4].
- **[Y5]** 战争, 《优美灵巧的傣族建筑》, 中国民族报, via 中国民族文化资源库 (mzzyk.cn, 贵州).
  Full text read. http://47.92.121.167/mzwhzyk/663688/686015/686018/635829/index.html —
  **[travel/blog]**; the strongest available description of the Dai material changeover and of the
  Dehong and Yuanjiang variants.
- **[Y6]** 叶阳, 杨健强, 张方浩, 明小娜, 周洋, 王杰, 刘绍昌, 高扬,
  《云南省五类木结构民居特征及震害总结》, 《中国地震》 2025, 41(2): 306–326.
  DOI/journal landing page: http://zgdz.eq-j.cn/zgdz/article/abstract/zgdzzwb-41-2-306
  **Full text read** via the publisher's XML export (HTTP 200, 139 830 bytes; the PDF and the HTML
  reader both returned empty to my fetcher, the export did not). Received 2024-08-15, accepted
  2024-12-02, online 2025-07-10. Funded by 川滇地区地震灾害建筑物埋压人员评估方法研究课题 XH24039C,
  云南省地震局青年基金 2024K08, 中国地震局昆明地震预报研究所自立项目 2023KZL03. — **[academic]**,
  peer-reviewed, by 云南省地震局 / 中国地震局昆明地震预报研究所 / 德宏州防震减灾局. **This is the
  strongest source in this note.** Its own bibliography is the field's working reference list and is
  what [Y12]–[Y30] below were identified from.
- **[Y7]** 《傣族 建筑》, 民间艺术网 (minjianyishu.net), dated 2009-07-17, **作者：正在核实中…,
  来源：网络**. Full text read.
  http://www.minjianyishu.net/?act=app&appid=241&mid=7040&p=view — **[unverified]**, and the page says
  so itself. **Every number taken from it in this note is tagged accordingly and none of them carries
  a structural conclusion.** The only Dai plan dimensions in the note come from here, which is
  exactly why they are marked "very low".
- **[Y8]** 《白族民居建築風格》, 三三古建築 (3343.xyz), dated 2016-12-14. Full text read.
  http://www.3343.xyz/gdjz/ziliao/gudaijianzhu/201612/25888.html — **[unverified]**; retained as a
  **second, independent witness that [Y1]'s text is a stable published passage and not a one-off
  upload** (the traditional-character version is textually identical to [Y1]).

### Landing page / abstract read only

- **[Y9]** 《严家大院》 document on 人人文库, referenced as
  https://www.renrendoc.com/free-down/6014133104000045.pdf — **not fetched** (the host served
  `application/octet-stream`). A search-engine snippet of this document gives: 严家大院 built by
  严子珍 in **1907**, site **≈ 2478 m²**, **一进四院**, two courts of 三坊一照壁 and two of
  四合五天井, all tied into one whole by a **走马转角楼**. — **[unverified]**; quoted as a pointer,
  not as a measurement.
- **[Y10]** 宾慧中, 《从传世口诀看滇西北剑川匠系的营造技艺》, 《建筑遗产》 2016, 案例解析, p. 098,
  CN 10-1071/TU. Abstract read.
  https://jianzhuyichan.tongji.edu.cn/info/1112/1269.htm — **[academic]**. The abstract states the
  content is 剑川 carpenters' **营造口诀** — transmitted mnemonics summarising **各作制度、常用构架
  尺度** (the work systems and the *usual frame dimensions*). **This is the single most promising
  unopened source for the Bai frame numbers**, and it is noted as such in §7.
- **[Y11]** 《剑川地区穿斗式木结构民居建筑榫卯类型研究》, 《华中建筑》 2025 年第 02 期.
  Bibliographic record confirmed from a CQVIP and a CNKI mirror listing; **the paper was not read.**
  https://wap-cnki-net-443.webvpn.imac.edu.cn/touch/web/Journal/Article/HZJZ202502025.html — **[academic]**.
- **[Y34]** 邵陆, 《从祭天敬祖仪式看白族民居的空间概念》, 《建筑遗产》 2016, p. 108. Abstract read.
  Same journal site as [Y10]. **[academic]**. Relevant to the *orientation* question (§3.8 item 1).

### Identified, not obtained — the bibliography [Y6] draws on

All **[academic]**. These are the documents whose absence is what §7 is about.

- **[Y4]** 刘致平, **《云南一颗印》**, 中国营造学社, 1938. Bibliographic identity established through
  [Y3], which states that 中国营造学社 was stationed at **龙泉镇 near Kunming from 1938**, that 刘致平
  carried out **系统测绘** (systematic measured survey) of the local houses, and that the paper —
  **"论文中严谨的测绘图"**, with rigorous measured drawings — **"至今仍是'一颗印'民居研究领域的
  重要文献"**. **The paper itself was not obtained.** This is the primary **measured drawing set** for
  the 一颗印 and is the most important document this session failed to reach. — **[heritage survey]**.
- **[Y35]** 《民间住宅》 — table of contents only, reproduced on 造价通's site. Full index read.
  https://news.zjtcn.com/50955180.html — **[unverified]** as a host, but the contents list is
  **[academic]** bibliographic evidence, and it is the best available map of the field's chapter
  decomposition. Recorded here because it **independently confirms the part list a generator needs**:
  白族民居的构成单元 / 白族民居的照壁 / 白族三坊一照壁 / 白族四合五天井 / 丽江民居的坊 / 丽江民居的
  两层楼房 / 丽江民居的厦子 / 丽江民居的铺地 / 丽江民居的院落组合 / 丽江民居的照壁 / 丽江民居大门的
  设置 / 悬鱼 / 一颗印民居 / 傣族民居 / 傣族民居的室内空间 / 干栏式民居 / 西双版纳傣族民居. Note what
  the index does **not** contain: 蛮楼 (see §7 item 12), and no chapter on 漏角屋.
- **[Y12]** 陆元鼎主编, **《中国民居建筑》下卷**, 广州: 华南理工大学出版社. The book behind [Y2].
  Not obtained; the Lijiang chapter was read only through [Y2].
- **[Y13]** 杨大禹, 朱良文, **《云南民居》**, 北京: 中国建筑工业出版社, 2009, 92 pp.
- **[Y14]** 范雪峰, 《云南地方传统民居屋顶的体系构成及其特征》, 硕士学位论文, 昆明理工大学, 2005.
  [Y6] takes its rafter/tile figures from this thesis. **The source of the only credible lead on Bai
  roof pitch and eave overhang.**
- **[Y15]** 刘一, 《西双版纳傣族民居的演变与更新研究》, 硕士学位论文, 昆明理工大学, 2011. **The source
  of the 45°–60° Dai roof pitch** quoted in [Y6].
- **[Y16]** 王文淞, 《西双版纳勐海县勐景来村傣族民居建构研究》, 硕士学位论文, 昆明理工大学, 2017.
  The one named, specific Dai village study; would be the natural place to find post spacing.
- **[Y17]** 郭建伟, 《傣族风土建筑大木构件考释——以中国西南西双版纳地区为例》, 《建筑遗产》 2022
  年第 2 期, pp. 25–32. Titled precisely on the Dai's **major timber members**. CNKI abstract page:
  https://wap.cnki.net/touch/web/Journal/Article/JZYA202202003.html — **not read.**
- **[Y18]** 马若予, 《西双版纳基诺族传统民居营造技艺研究——以基诺乡洛特老寨为例》, 硕士论文,
  昆明理工大学, 2015. [Y6]'s source for the 20–25 mm upper-storey board.
- **[Y19]** 高洁, 《云南汉式合院民居构架类型及建造逻辑研究》, 硕士学位论文, 昆明理工大学, 2018.
  Titled on **the frame types of the Yunnan Han courtyard**, i.e. on the 一颗印.
- **[Y20]** 潘曦, 《纳西族乡土建筑建造范式研究》, 博士学位论文, 清华大学, 2014. The Naxi frame and
  construction paradigm.
- **[Y21]** 解婧雅, 《纳西建筑木构架与土坯营造作法研究》, 硕士学位论文, 北方工业大学, 2014.
- **[Y22]** 赵西子, 《滇西北藏族传统民居"土掌碉房"营造技艺调查研究》, 硕士学位论文,
  西安建筑科技大学, 2018. [Y6]'s source for the flat-roof foundations.
- **[Y23]** 强明礼, 《云南藏族木结构民居特征研究》, 博士学位论文, 中国林业科学研究院, 2016. [Y6]'s
  source for the 闪片房 column diameters.
- **[Y24]** 姚瑞恺, 《井干式民居木墙体营造工艺研究》, 《邢台职业技术学院学报》 2021, 38(2): 101–104.
- **[Y25]** 范举国, 徐洲, 《浅谈穿斗式木结构的分类》, 《四川建材》 2017, 43(11): 83–84. The
  有瓜柱 / 无瓜柱 classification [Y6] uses.
- **[Y26]** 付帅, 陈泽华, 杨小军, 等, 《我国穿斗式木结构建筑研究现状与发展建议》, 《林产工业》
  2021, 58(5): 38–41.
- **[Y27]** 王现伟, 孙柏涛, 陈洪富, 《西南地区穿斗式木结构民居构造特点及震害特征分析——以贵州山区
  为例》, 《地震研究》 2024, 47(2): 290–299. The nearest published analogue to the 穿斗 numbers a
  Bai survey would contain.
- **[Y28]** 曲哲, 钟江荣, 孙景江, 《四川芦山 7.0 级地震中非抗震设防民居震害特征》, 《建筑结构学报》
  2014, 35(5): 157–164.
- **[Y29]** Qu Z., Dutu A., Zhong J. R., et al., "Seismic damage to masonry-infilled timber houses in
  the 2013 M7.0 Lushan earthquake", *Earthquake Spectra* 2015, 31(3): 1859–1874.
  https://doi.org/10.1193/122313EQS306 — the English-language source for "the infill wall behaves as
  a filler in a frame" that [Y6] reports for Yunnan.
- **[Y30]** 李萨, 《云南迪庆藏族民居特色研究》, 《山西建筑》 2021, 47(16): 18–20.
- **[Y31]** 潘静, 王海涛, 《中甸藏族传统土墙板房的建筑特色——以中甸县建塘镇解放村贡宾社为例》,
  《建筑知识》 2007, 27(2): 48–50.
- **[Y33]** 《丽江地震土木结构房屋震害分析》, 《云南工业大学学报》 1996 年第 04 期.
  CNKI record: https://wap.cnki.net/touch/web/Journal/Article/YNGY604.000.html. Would connect the
  1996 Lijiang earthquake to specific Naxi buildings.

### Negative results — hosts and documents that failed

- **[Y32]** 《云南省民居建筑特色设计导则》, 云南省住房和城乡建设厅 — **DEAD LINK.** Referenced as
  https://zfcxjst.yn.gov.cn/submodule/Editor/uploadfile/20170420124435552.pdf ; returned
  **HTTP 404** on 2026-10-18. A provincial design guideline for traditional dwellings would very
  likely have carried codified plan dimensions and is the single highest-value document I failed to
  reach. **Re-check this URL and the 厅's document index before assuming the numbers do not exist.**
- **[Y36]** A large Chinese-language corpus (books from 读秀/超星 and university theses, including
  张崇礼《白族传统民居建筑》, 《中国民居建筑》, 刘致平《云南一颗印》's volume, 《中国国家历史文化
  名城》, 《云南传统民居木构架系列比较研究》, 《西双版纳傣族民居的分析与借鉴》, 《基于BIM的丽江纳西传统
  民居营造技术研究》, 《中国白族传统合院民居营建技艺研究》) is indexed by the search tool under
  `/hdd/…` paths. **Those paths are not on this machine** (`C:\hdd`, `D:\hdd` and `/hdd` all absent)
  and `web_fetch` rejects them as non-URLs. **Every one of those documents is therefore
  unavailable to this note, and several of them are exactly the measured surveys this question needs.**
  Recorded so a later session does not re-run the same searches.
- **Host failures on 2026-10-18**, each tried at most three times then abandoned: `baike.baidu.com`
  returned **HTTP 403** (安全验证) on every attempt, which removed 百度百科 as a source entirely;
  `47.92.121.167` returned 200 for the Dai article and **403** for the Bai article; the journal PDF at
  `hsfwest.ijournals.cn` and the 中国地震 PDF returned HTTP 200 with **empty bodies**; `renrendoc.com`
  raw PDFs return `application/octet-stream`, which the fetcher cannot convert; a self-written PDF
  text extractor recovered **0 glyphs** from the 中国地震 PDF because it carries no ToUnicode CMap;
  `jstage.jst.go.jp` returned a PDF the fetcher would not convert.
- **[Y30b]** A Japanese-language study of 大研古城 street widths surfaced in search with the quoted
  finding 幅員2〜4mが圧倒的に多い — **street widths of 2–4 m predominate across 114 sampled points**.
  *Japanese Journal of Architecture and Planning* 85(2), 2020, art. 85_254.
  https://www.jstage.jst.go.jp/article/jjcanth/85/2/85_254/_pdf/-char/ja — **abstract/snippet only**,
  PDF not extractable. — **[academic]**, foreign-language, and about **settlement fabric rather than
  house geometry**. Not used for any number above; recorded because the 2–4 m figure is the only
  quantitative statement this session saw about the space *between* these houses.

## Not established this session

Stated as: the thing I could not find, and the measurement or document that would settle it.

1. **Every plan dimension for the Bai type except the 坊's bay width and depth, and the 厦子 depth.**
   The 3.3–3.6 m bay, 5–6 m depth and 1.8–2.0 m gallery all come from **one monograph chapter**
   [Y1] with no named building behind them. **Settled by:** a dimensioned plan of one named house —
   剑川 or Shaxi (沙溪) would serve — from the 剑川 studies [Y10][Y11] or 张崇礼《白族传统民居建筑》.
2. **白族 roof pitch and 出檐.** [Y1] gives the purlin *count* (五架梁 / 七架梁, "chosen by the
   depth") and the depth range, from which a pitch could be *derived*, but never an angle, and never
   an eave projection. **Settled by:** a section through a named Bai house, or 范雪峰's thesis [Y14],
   whose title is exactly this.
3. **白族 storey heights and floor-to-floor.** No source in this note states any. **Settled by:** the
   same section.
4. **Post diameter and post spacing for the 穿斗 frame** — i.e. for the Bai, Naxi and 一颗印 houses
   alike. [Y6] says only 间距较小, and gives diameters exclusively for the *flat-roofed Tibetan*
   types, which are a different system; **I have deliberately not transferred them.** **Settled by:**
   a frame plan, or 王现伟等 [Y27] for a comparable system.
5. **Any Naxi number at all.** 潘曦 [Y20] and 解婧雅 [Y21] are the two documents that exist; neither
   was obtainable. Lijiang's bay widths, depths, storey heights and roof pitch are **entirely
   unestablished**, and the 蛮楼 term I was asked to cover **did not appear in any source I read** —
   see item 12.
6. **The 一颗印's measured dimensions.** [Y4] 刘致平《云南一颗印》(1938) exists and is the
   foundational measured survey with drawings [Y3]; I did not obtain it. The 12 m / 15 m / 3 m
   figures come only from a 2026 newspaper feature [Y3]. **Settled by:** [Y4] itself, or 高洁 [Y19].
   In particular the **position of the 楼梯巷 inside the square plan**, and the **projection of the
   腰檐**, are unknown to me.
8. **Dai post spacing, floor height, stair rise, eave projection and ridge length.** [Y6] gives the
   frame construction well and the plan not at all; the only plan figures are the folkloric [Y7].
   **Settled by:** a measured drawing of one named house — 勐景来村 is the one with a thesis about
   it [Y16], and 郭建伟 [Y17] is titled on the major timber members.
9. **四合五天井 corner-skywell dimensions.** I found the type named repeatedly and the formula
   explained nowhere. 严家大院 at Xizhou [Y9] is a two-三坊一照壁-plus-two-四合五天井 composition of
   ≈ 2478 m² tied by a 走马转角楼, but I could not fetch its document and found no drawing set.
10. **Whether any tourist reconstruction here is documented as inaccurate.** I looked for this and
    **found nothing either way** — [Y1] and [Y2] both name *real* buildings and neither discusses a
    replica. The 1990 深圳"中国民俗文化村" and the 昆明海埂"白族村" are noted by [Y1] only as places
    where an **economised** 照壁 was used, not as inaccurate. So the negative result is: **no source
    I read makes an accuracy claim about a reconstruction**, which means a reconstruction cannot be
    used as evidence for a dimension. **Settled by:** the 深圳市 and 海埂 project documentation.
11. **Dai roof form: "短脊" versus "脊长出檐短".** The brief describes the Dai roof as a *steep
    short-ridge* roof with deep eaves. What I could verify is narrower and partly in tension: [Y6]
    gives 45°–60° with **two unequal slopes and 歇山 ends** and a **lighter-pitch 重檐** below; [Y5]
    describes the **Dehong** house as **脊长出檐短** (long ridge, *short* eave), which is the
    opposite of deep. **Neither source uses 短脊.** **Settled by:** a plan of one Xishuangbanna house
    showing ridge length against plan length.
12. **The term 蛮楼 (mánlóu) for a Naxi house type.** It did not appear in any source I read.
    [Y2]'s Lijiang vocabulary is 坊, 厦子, 漏角屋, 头梁合抱柱, 跑马转角楼, 过山门; and the contents
    list of [Y35], which devotes **seven consecutive chapters** to the Lijiang house and breaks it
    down exactly as this note does, contains **蛮子门** (item 125) but **no 蛮楼 at all**. So the
    nearest attested term is 蛮子门, and **I could not confirm 蛮楼 exists as a building-type name**
    and have not used it. **Settled by:** any Lijiang vernacular dictionary or the 丽江纳西族自治县
    地方志 — which also surfaced in search as an unread duxiu volume ([Y36]).
13. **火塘, 晒台 and 三眼井 as geometry.** All three are attested as *objects* ([Y5], [Y7], [Y2])
    and none has a dimension in anything I read. **Settled by:** an upper-storey plan and a
    courtyard plan.
14. **The Bai screen wall's relation to the wing eave** is given as a *relative* rule — the flanking
    ridge sits below the 虎牙 soffit [Y1] — but **the wing's eave height is itself unestablished**,
    so the two cannot yet be combined. That is a circular dependency worth naming: **fixing the Bai
    eave height fixes the screen wall height without ever measuring the screen wall.**
15. **Whether the 一颗印's 12 m face width is three 4 m bays or a face width with ears attached
    outside it.** [Y3] gives 面宽约12米 for 正房三间 without resolving whether 耳房 are inside or
    outside that 12 m. **Settled by:** a plan. **Settled by the same measurement as item 6.**
16. **Sources reachable only through the unreachable `/hdd` corpus** (see [Y36]) — in particular
    张崇礼《白族传统民居建筑》 and 《中国民居建筑》 — remain unread. **Nothing in this note should be
    read as exhausting the available literature; it exhausts the literature reachable without
    institutional access.**
