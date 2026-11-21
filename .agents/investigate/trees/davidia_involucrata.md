# What does *Davidia involucrata* look like, and how does it grow?

**Question this note answers:** the form and the numbers of the dove tree, for a procedural-vegetation renderer.
**Measured:** 2026-10-18 (all URLs fetched that day).
**Language note:** written in English; Chinese names kept.

---

## Identity

| Item | Value | Source |
|---|---|---|
| Chinese name | 珙桐 (gǒng tóng), common names 鸽子树 "dove tree",  handkerchief tree | [efloras FOC 13](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200014694) |
| Accepted binomial | ***Davidia involucrata* Baill.** | [GBIF name match](https://api.gbif.org/v1/species?name=Davidia%20involucrata) (key 7322122, status ACCEPTED, published *Adansonia* 10: 115, 1871) |
| Family | Nyssaceae (蓝果树科) — GBIF backbone and the Catalogue of Life both also carry it in Cornaceae | [GBIF](https://api.gbif.org/v1/species?name=Davidia%20involucrata) |
| Varieties | **var. *involucrata*** (珙桐) and **var. *vilmoriniana*** (光叶珙桐) | [efloras FOC 13](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200014694) |
| Chinese flora page | 植物智 species page, Flora of China tab | <https://www.iplant.cn/info/%E7%8F%99%E6%A1%90?t=foc> |
| Photo bank | PPBC 中国植物图像库, cid 23831 (species), cid 42696 (var. *vilmoriniana*), cid 953816 (genus) | <https://ppbc.iplant.cn/list21?keyword=%E7%8F%99%E6%A1%90> |

### Where the two varieties differ

The varieties are separated **only by the indumentum of the leaf underside**, and nothing else in the flora text:

| | var. *involucrata* | var. *vilmoriniana* (光叶珙桐) |
|---|---|---|
| Leaf abaxial surface | densely (silky) pubescent | glabrous, or scarcely pubescent when young; sometimes glaucous |
| FOC wording | "Leaves abaxially densely pubescent." | "Leaves abaxially glabrous or scarcely pubescent when young, sometimes abaxially glaucous." |

Source for both cells: [efloras FOC 13](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200014694).

**Modelling consequence:** the two varieties are geometrically identical. Only the underside colour and its shading change — var. *involucrata* reads pale silvery-white from below, var. *vilmoriniana* reads bright glabrous green to glaucous. Build one mesh and two materials.

---

## Site

- **Altitude: 1100–2600 m** — [efloras FOC 13](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200014694): "Montane mixed forests; 1100-2600 m."
- **Provinces:** Guizhou, western Hubei, western Hunan, Sichuan, northern Yunnan — same source.
- **Vegetation belt:** montane mixed forest. It is a low-belt, mid-slope tree — it wants the mixed broadleaf/conifer communities of the wet montane flank, not the conifer zone and not the valley bottoms. The Dalaoling population reported by 宜昌三峡大老岭自然保护区管理局 grows in 原始森林区 (primary forest) with 白辛树 (a birch, *Betula* sp.) as its recorded associate: <http://www.sxdll.com.cn/html/zygl/zygl/4525.html>.
- **Photographic confirmation of the site picture:** [01-habit.jpg](C:/resource/whiting/out/refs/davidia_involucrata/01-habit.jpg) shows a flowering *D. involucrata* var. *vilmoriniana* standing among taller conifers (Chinese-fir-like) at 神农祭坛, Shennongjia, Hubei — a broadleaf tree understorey species sitting below a conifer canopy. [04-branch.jpg](C:/resource/whiting/out/refs/davidia_involucrata/04-branch.jpg) is from the same place and shows the cloud-forest slope behind the branch.

### Is it a relict / endemic, and what protects it

- **Yes to both.** 珙桐 is described by 宜昌三峡大老岭自然保护区管理局 as "为中国特有的单属植物" — a **China-endemic monotypic genus** — and "起源古老，是约1000万年前新生代第三纪留下的孑遗植物" — a **Tertiary relict of roughly 10 million years**, called "植物活化石" and "植物界的大熊猫". Source: <http://www.sxdll.com.cn/html/zygl/zygl/4525.html> (same text, shorter, at <http://m.cnhubei.com/content/2024-05/12/content_17878013.html>).
- **Protection: 国家一级重点保护植物 — national Class I protected wild plant.** Same source. Flora of China additionally records it as rare in China: "This species was listed as rare in China by Fang and Song (in Fu & Jin, China Pl. Red Data Book 1: 474-476. 1992)." ([efloras FOC 13](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200014694)).
- Flora of China notes the monotypic genus carries a second accepted name, *Davidia laeta* — i.e. the genus has historically been split, which is the taxonomic echo of the relict status.

---

## Habit

- **Height: to 20 m** — [efloras FOC 13](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200014694): "Trees to 20 m high, with ascending bark."
- **Trunk diameter: not given by Flora of China.** The only measured value found today is for the Dalaoling ancient-tree stand in Hubei: of ~30 ancient trees, 20+ have **DBH over 30 cm and the largest has DBH over 50 cm** (<http://www.sxdll.com.cn/html/zygl/zygl/4525.html>). Treat 0.5 m DBH as a *documented floor* for old trees, not a maximum.
- **Silhouette, in one sentence:** a tall, straight, single clear trunk carrying a broad, rounded-to-egg-shaped crown of roughly equal width and depth over the upper two-thirds of the tree, whose outline is broken all over by pendent white bracts in spring — a green mass with pale fluttering patches that read as doves at distance.
- **How the crown changes with age:** from [03-bark.jpg](C:/resource/whiting/out/refs/davidia_involucrata/03-bark.jpg) and [08-crown-flowering-forest.jpg](C:/resource/whiting/out/refs/davidia_involucrata/08-crown-flowering-forest.jpg) the species frequently develops as **several near-parallel, near-vertical stems from the base rather than one trunk**, and the crown then becomes a cluster of several parallel ascending limbs rather than a single axis. A young tree ([13-sapling-shoot-leaf-arrangement.jpg](C:/resource/whiting/out/refs/davidia_involucrata/13-sapling-shoot-leaf-arrangement.jpg)) is a single upright leader with leaves crowded along it; the crown broadens only after the leader forks.

---

## Branching

- **Phyllotaxy: spiral/alternate.** Leaves are alternate on the shoot (they are clearly offset from node to node along the red twig in [05-leaf.jpg](C:/resource/whiting/out/refs/davidia_involucrata/05-leaf.jpg) and in the whole shoot of [13-sapling-shoot-leaf-arrangement.jpg](C:/resource/whiting/out/refs/davidia_involucrata/13-sapling-shoot-leaf-arrangement.jpg)). Flora of China does not state the leaf arrangement in words; this is read off the photographs. The only structure Flora of China calls **opposite** is the pair of floral bracts: "Bracts 2 (or 3), **opposite**" — that is a floral structure, not a vegetative one.
- **First-order branches: strongly ascending.** In [04-branch.jpg](C:/resource/whiting/out/refs/davidia_involucrata/04-branch.jpg) (field, Shennongjia) and [08-crown-flowering-forest.jpg](C:/resource/whiting/out/refs/davidia_involucrata/08-crown-flowering-forest.jpg) the long first-order limbs leave the trunk or a major limb at a steep upward angle and run outward-and-up before flattening. They never hang.
- **Lower branches:** cannot be established from the reachable photographs — all the kept images are of crowns or of leafy shoots, none shows a full trunk-to-ground elevation. What *is* visible ([03-bark.jpg](C:/resource/whiting/out/refs/davidia_involucrata/03-bark.jpg)) is that the basal part of each stem is clean of branches over a long clear run, i.e. the clear trunk is a real feature. **Estimate, flagged: assume the first branch starts at 5–9 m on a 15–20 m tree.**
- **Is the crown layered?** No. It is a single continuous shell, not stratified into tiers. [02-crown.jpg](C:/resource/whiting/out/refs/davidia_involucrata/02-crown.jpg) and [14-crown-from-below.jpg](C:/resource/whiting/out/refs/davidia_involucrata/14-crown-from-below.jpg) both show one uninterrupted foliage mass with branches at all depths.
- **Where the flowers sit (the modeller's trap):** the inflorescences are **terminal on short lateral spurs**, not on the leafy tips of the current shoots. Flora of China: "Flower heads terminal, ca. 2 cm in diam. on slender peduncles ca. 7 cm, **on short lateral spurs**." ([efloras](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200014694)). So a flowering *Davidia* carries leafy current shoots **and** separate bare spurs carrying a single pendant inflorescence each. See [04-branch.jpg](C:/resource/whiting/out/refs/davidia_involucrata/04-branch.jpg).

---

## Trunk

- Flora of China's whole statement: "Trees to 20 m high, **with ascending bark**" ([efloras FOC 13](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200014694)). "Ascending bark" = the bark pattern runs upward along the stem.
- **Straight, not contorted.** [03-bark.jpg](C:/resource/whiting/out/refs/davidia_involucrata/03-bark.jpg) shows several straight, near-vertical grey-brown stems, each of essentially constant diameter over the frame, with no buttressing and no flaring; they meet the ground without visible root swell.
- **Bark colour and texture from the photographs:** smooth to faintly striate **grey-brown to grey-green**, matte rather than glossy, with scattered pale horizontal lenticels on the twigs (visible in [04-branch.jpg](C:/resource/whiting/out/refs/davidia_involucrata/04-branch.jpg) and [10-leaf-lower-and-fruit.jpg](C:/resource/whiting/out/refs/davidia_involucrata/10-leaf-lower-and-fruit.jpg)). Young shoots and petioles are conspicuously **red** (Flora of China: "Petiole **red**, 3.6-7 cm"). No exfoliation, no plates, no deep fissures is visible anywhere in the 55 PPBC frames examined.
- **Limitation, stated plainly:** PPBC holds **no close-up bark macro** for this species (species, variety and genus pages together = 55 frames, all examined). `03-bark.jpg` is an in-situ view of the trunks, not a macro, and it is labelled as such.

---

## Foliage

All from [efloras FOC 13](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200014694) unless marked.

| Property | Value |
|---|---|
| Leaf type | simple, alternate (spiral) |
| **Blade length × width** | **8–15 × 7–12 cm** |
| **Petiole** | **3.6–7 cm, red**, as long as half the blade |
| Blade shape | **broadly ovate** (近圆形/宽卵形) |
| Base | **cordate** — a deep, wide, roughly heart-shaped sinus |
| Apex | **acuminate** — drawn out to a point |
| Margin | **dentate-serrate with acuminate teeth**; the teeth themselves end in points |
| Venation | palmate-arcuate: several strong primary veins leave the base and curve up to the margin; the underside is "strongly veined" and the raised veins are plainly visible in [12-leaf-flat-lower.jpg](C:/resource/whiting/out/refs/davidia_involucrata/12-leaf-flat-lower.jpg) and [10-leaf-lower-and-fruit.jpg](C:/resource/whiting/out/refs/davidia_involucrata/10-leaf-lower-and-fruit.jpg) |
| Evergreen / deciduous | **deciduous** |
| Upper surface | "adaxially **bright green**… finally glabrous" |
| Lower surface | var. *involucrata*: "densely **silky pubescent**"; var. *vilmoriniana*: "glabrous or scarcely pubescent when young, sometimes abaxially **glaucous**" |
| **Distribution along a shoot** | **spread along it**, alternate, on long petioles — *not* a terminal rosette. Counted off [13-sapling-shoot-leaf-arrangement.jpg](C:/resource/whiting/out/refs/davidia_involucrata/13-sapling-shoot-leaf-arrangement.jpg): a 30 cm shoot segment carries 5–7 leaves at clear spiral intervals. Marked as a **count from the photograph**, not a flora figure. |

Blade outline for the modeller, in words: nearly circular, slightly longer than wide, cut in at the base by a broad cordate sinus whose two lobes flare out almost as far as the blade's widest point, and drawn out at the tip into a short acuminate point; the teeth are large (about 3–5 mm) and point forwards along the margin.

Best orthographic references: [11-leaf-flat-upper.jpg](C:/resource/whiting/out/refs/davidia_involucrata/11-leaf-flat-upper.jpg) (adaxial, laid flat) and [12-leaf-flat-lower.jpg](C:/resource/whiting/out/refs/davidia_involucrata/12-leaf-flat-lower.jpg) (abaxial, laid flat) — both fresh leaves pressed on a board, not herbarium sheets. Living in-situ leaf: [05-leaf.jpg](C:/resource/whiting/out/refs/davidia_involucrata/05-leaf.jpg).

---

## Reproductive

All from [efloras FOC 13](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200014694).

- **The inflorescence:** a globose head, **ca. 2 cm in diameter**, borne terminally on a slender peduncle **ca. 7 cm** long, on short lateral spurs. The head is made of many staminate flowers plus one pistillate or bisexual flower (per 宜昌三峡大老岭自然保护区管理局, <http://www.sxdll.com.cn/html/zygl/zygl/4525.html>).
- **The bracts — the reason the species is famous:**
  - **number: 2, occasionally 3**, and the two are **opposite** each other;
  - **shape: ovate to oblong-obovate**; **margin entire or serrate**;
  - **size: the larger one reaches 16 cm long; the smaller one is ca. half as long as the larger** (so roughly 8 cm);
  - colour: white, sometimes pale yellow (宜昌三峡大老岭管理局, same source).
- **The true flowers are the small dark head in the middle,** not the white. In [06-flower-bract-pair.jpg](C:/resource/whiting/out/refs/davidia_involucrata/06-flower-bract-pair.jpg) and [09-bracts-two-clear.jpg](C:/resource/whiting/out/refs/davidia_involucrata/09-bracts-two-clear.jpg) the pair of white bracts hangs from a slender stalk with a small dark purplish-brown globular head between them — the "dove": bracts as wings, head as body.
- **Season: flowering April** ("Fl. Apr, fr. Oct").
- **Fruit:** usually solitary, **pear-shaped, 3–4 × 1.5–2 cm**, green with a purple bloom when fresh (older fruit browns). See [07-fruit.jpg](C:/resource/whiting/out/refs/davidia_involucrata/07-fruit.jpg): paired green drupes on long pedicels on the same shoot as the leaves. Fruiting October.

---

## The five facts that let you recognise this tree from 200 m away, as a silhouette

1. **A broad egg or rounded dome on a long bare pole.** Crown width is close to crown depth, and the crown starts roughly half-way up a straight clear trunk. The outline is a clean oval, not a vase and not a column. `[01-habit.jpg, 03-bark.jpg]`
2. **In April the crown is speckled with pendent white bracts that hang below the foliage line.** They are not petals on the canopy surface; each is a 8–16 cm white wing on a ~7 cm stalk, hanging free of the leaves. From 200 m the tree reads as a green crown with pale fluttering patches, and the patches move. `[06-flower-bract-pair.jpg, 09-bracts-two-clear.jpg, 08-crown-flowering-forest.jpg]` — this is the single strongest recognition cue and the one that justifies the species' name.
3. **Nothing droops from the woody skeleton.** Every woody limb ascends. The only pendent things in the whole tree are the bracts and the fruiting pedicels. A model with a weeping branch habit is wrong.
4. **The crown is open, not solid** — big, widely spaced, singly-attached leaves on long petioles let light through, so the canopy edge is ragged and translucent rather than a hard shell. `[05-leaf.jpg, 13-sapling-shoot-leaf-arrangement.jpg]`
5. **Bare in winter, and studded with pear-shaped fruit.** Deciduous, with the 3–4 cm drupes persisting on the bare branches, so the winter silhouette is a fine open twig lattice with small dark beads. `[07-fruit.jpg]`

---

## Modelling numbers

Flora-given figures are unmarked. Anything I inferred is marked **ESTIMATE** with its basis.

| Quantity | Value | Basis |
|---|---|---|
| Mature height | **to 20 m** | flora (FOC) |
| Clear trunk height (ground to first branch) | **5–9 m** on a 15–20 m tree | **ESTIMATE** — read off the clean basal run of stem in `03-bark.jpg` and the trunk fraction of the whole tree in `01-habit.jpg`; FOC gives no value |
| Trunk DBH, mature | **≥ 0.50 m documented; 0.5–1.2 m plausible** | documented floor: Dalaoling ancient trees, largest > 50 cm DBH (<http://www.sxdll.com.cn/html/zygl/zygl/4525.html>). Upper part **ESTIMATE** from stem-to-crown-width ratio in `03-bark.jpg` |
| Trunk radius at base | **0.25–0.60 m** | half the DBH range above |
| Buttress / flare | **none** | `03-bark.jpg` |
| Crown width | **11–15 m** (≈ 0.65–0.8 × height) | **ESTIMATE** from `01-habit.jpg` |
| Crown depth (vertical extent) | **7–11 m** (≈ 0.5–0.6 × height) | **ESTIMATE** from `01-habit.jpg`, `02-crown.jpg` |
| Crown depth from top of trunk to apex | **crown starts at 5–9 m and ends at 20 m** | as above |
| Number of first-order branches | **8–15** | **ESTIMATE** — counted as distinct primary limb entries in `02-crown.jpg`, `03-bark.jpg`, `14-crown-from-below.jpg`; FOC gives no branching figures |
| Angle of a first-order branch from the trunk axis | **40–60°** (steeply ascending, then flattening) | **ESTIMATE** from `04-branch.jpg`, `08-crown-flowering-forest.jpg` |
| Second-order branches | sparse, 2–4 per first-order, ascending | **ESTIMATE** from `04-branch.jpg` |
| Twig length (leafy shoot) | **20–40 cm** | **ESTIMATE** — the leafy shoot segment in `13-sapling-shoot-leaf-arrangement.jpg` and `04-branch.jpg` |
| Leaves per 30 cm of shoot | **5–7**, alternate, spiral | **ESTIMATE**, counted in `13-sapling-shoot-leaf-arrangement.jpg` |
| Internode | **3–6 cm** | **ESTIMATE** from the same image |
| **Leaf length × width** | **8–15 × 7–12 cm** | flora (FOC) |
| **Petiole length** | **3.6–7 cm** | flora (FOC) |
| Leaf thickness | thin, not succulent | photographs |
| Bract length (large / small) | **to 16 cm / ca. 8 cm** | flora (FOC) |
| Bract width | **ESTIMATE 6–11 cm** | not in FOC; from the bract outline in `06-flower-bract-pair.jpg` against the 8–15 cm leaves beside it |
| Peduncle (bract stalk) length | **ca. 7 cm** | flora (FOC) |
| Flower-head diameter | **ca. 2 cm** | flora (FOC) |
| Fruit length × width | **3–4 × 1.5–2 cm** | flora (FOC) |

---

## Images

All under `C:\resource\whiting\out\refs\davidia_involucrata\`.
Source bank: PPBC 中国植物图像库. Full-size image URL pattern: `http://img0.iplant.cn/image61/b/<HASH>.jpg` (plain HTTP — the HTTPS path on `img*.iplant.cn` answers 403 from this machine; see the report).
PPBC detail page: `https://ppbc.iplant.cn/tu/<id>`.
Note: `cid 23831` = *D. involucrata*; `cid 42696` = var. *vilmoriniana*; `cid 953816` = genus *Davidia*.

| File | Bytes | Pixels | Shows | PPBC taxon / id / photographer / locality | Source URL |
|---|---|---|---|---|---|
| `01-habit.jpg` | 708,853 | 1080×810 | **Whole flowering tree standing in its site**, multi-stemmed base, broad rounded crown of pendent white bracts, taller conifers behind | var. *vilmoriniana* / tu=71439 / 李晓东 / 湖北神农架 神农祭坛 | <http://img0.iplant.cn/image61/b/ECC84CCE92F6F97B.jpg> |
| `02-crown.jpg` | 525,248 | 722×1080 | The crown from below, blue sky behind; bracts and leaves all over one continuous foliage shell | *D. involucrata* / tu=1656933 / 宋鼎 / 云南昆明市 | <http://img0.iplant.cn/image61/b/2AD9E35289656D7A.jpg> |
| `03-bark.jpg` | 715,611 | 722×1080 | **Trunk and stems** — several near-vertical grey-brown stems, constant diameter, smooth faintly striate bark, no buttressing. In situ, not a macro: PPBC has no bark macro for this species | *D. involucrata* / tu=1656930 / 宋鼎 / 云南昆明市 | <http://img0.iplant.cn/image61/b/4A501F12A501471F.jpg> |
| `04-branch.jpg` | 530,023 | 1080×810 | **A first-order branch**: long ascending limb, alternate leaves along it, and pendant inflorescences on separate short spurs; cloud-forest slope behind | var. *vilmoriniana* / tu=71445 / 李晓东 / 湖北神农架 | <http://img0.iplant.cn/image61/b/5DFF9878F0678472.jpg> |
| `05-leaf.jpg` | 278,863 | 1080×718 | **Leaf close-up**: broadly ovate blade, cordate base, acuminate apex, dentate-serrate margin, red petiole, alternate on the twig | *D. involucrata* / tu=594116 / 李光敏 / 北京 国家植物园南园 | <http://img0.iplant.cn/image61/b/EF4BEA754B6430B4.jpg> |
| `06-flower-bract-pair.jpg` | 316,498 | 719×1080 | **One inflorescence close up**: the two white bracts, opposite, with the small dark purplish flower head between them; serrate leaves beside them | *D. involucrata* / tu=4232500 / 魏泽 / 北京市海淀 北京植物园 | <http://img0.iplant.cn/image61/b/6E925C812CD15DD8.jpg> |
| `07-fruit.jpg` | 464,169 | 1080×727 | **Fruit**: paired pear-shaped green drupes on long pedicels, with upper- and lower-surface leaves on the same shoot | *D. involucrata* / tu=431643 / 陈又生 / 云南迪庆 维西县叶枝 | <http://img0.iplant.cn/image61/b/D7BEB6BA0A5B2EA1.jpg> |
| `08-crown-flowering-forest.jpg` | 597,353 | 717×1080 | Whole flowering crown in a forest, branches readable through the foliage | *D. involucrata* / tu=1593301 / 徐永福 / 湖南省怀化市 | <http://img0.iplant.cn/image61/b/1F2A42F838838C03.jpg> |
| `09-bracts-two-clear.jpg` | 482,417 | 1080×720 | **The pair of white bracts, clearest view of the two together**, hanging, with cordate leaves and reddish twig | *Davidia* genus / tu=20740893 / 杨福生 | <http://img0.iplant.cn/image61/b/B498300F99BAF83C1F759BBDD75AE0DD.jpg> |
| `10-leaf-lower-and-fruit.jpg` | 393,240 | 1080×724 | **Leaf abaxial surface** (pale, silky-pubescent, raised veins), petiole, twig with leaf scars, and one green fruit | *D. involucrata* / tu=54450 / 喻勋林 / 湖南衡阳 南岳树木园（栽培） | <http://img0.iplant.cn/image61/b/42ED125C02E06321.jpg> |
| `11-leaf-flat-upper.jpg` | 489,458 | 1080×720 | Fresh leaf pressed flat, **adaxial** surface — orthographic outline, cordate base, acuminate apex, tooth spacing | *Davidia* genus / tu=8638240 / 薛自超 | <http://img0.iplant.cn/image61/b/48382CCD72DA83AC.jpg> |
| `12-leaf-flat-lower.jpg` | 571,629 | 1080×720 | Fresh leaf pressed flat, **abaxial** surface — the palmate-arcuate venation and the tomentum | *Davidia* genus / tu=8638239 / 薛自超 | <http://img0.iplant.cn/image61/b/D5FC5B1FE0B7686A.jpg> |
| `13-sapling-shoot-leaf-arrangement.jpg` | 648,564 | 813×1080 | **A young shoot**: alternate leaves at spiral intervals along a red twig — the leaf-distribution reference | *Davidia* genus / tu=28367827 / 袁华 | <http://img0.iplant.cn/image61/b/54DE65C5286F9B0AA692ED2294AB1EB7.jpg> |
| `14-crown-from-below.jpg` | 774,128 | 810×1080 | Looking up into a flowering crown against white sky — dark branches with leaves and bracts distributed along them | *Davidia* genus / tu=29344815 / 江超 | <http://img0.iplant.cn/image61/b/D248C115B587F0965AA826AF74A09D3B.jpg> |

**14 photographs kept. Two things the client asked for that PPBC does not hold:**
1. **A close-up bark macro** — none exists in the 55 PPBC frames for species, variety or genus. `03-bark.jpg` is the closest available, and it is labelled as an in-situ view.
2. **A bare winter tree showing the branching** — PPBC holds **no leafless *Davidia* photograph at all**. Every frame is in leaf, flower or fruit. The winter description in "the five facts" above is therefore built from the decididuous statement in Flora of China and from the twig structure visible in `13-sapling-shoot-leaf-arrangement.jpg`, not from a photograph, and is marked accordingly.
