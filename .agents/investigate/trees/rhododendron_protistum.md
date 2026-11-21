# What does *Rhododendron protistum* var. *giganteum* look like, and how does it grow?

**Question this note answers:** the form and the numbers of the giant treelike rhododendron of Gaoligongshan, for a procedural-vegetation renderer, plus the range of the treelike species of the genus.
**Measured:** 2026-10-18 (all URLs fetched that day).
**Language note:** written in English; Chinese names kept.

---

## Identity

| Item | Value | Source |
|---|---|---|
| Chinese name | **大树杜鹃** "big-tree rhododendron" | [PPBC cid 42060](https://ppbc.iplant.cn/list21?keyword=%E5%A4%A7%E6%A0%91%E6%8A%B9%E5%86%9C) |
| Chinese name, parent species | **翘首杜鹃** = *Rhododendron protistum* | [efloras FOC 14](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200016531) |
| Accepted name | ***Rhododendron protistum* Balf.fil. & Forrest**, var. ***giganteum* (Forrest) D. F. Chamberlain** | [efloras FOC 14, vol. 14 p. 341](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200016531) |
| Family | Ericaceae (杜鹃花科) | [GBIF](https://api.gbif.org/v1/species?name=Rhododendron%20protistum) — key 7327960, status ACCEPTED, *Notes Roy. Bot. Gard. Edinburgh* 12: 151, 1920 |
| Type | *Notes Roy. Bot. Gard. Edinburgh* 12: 151. 1920 | [efloras FOC 14](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200016531) |
| Chinese flora page | 植物智, FOC tab | <https://www.iplant.cn/info/Rhododendron%20protistum?t=foc> |
| Photo bank | PPBC, cid **42060** (var. giganteum), **24924** (species), 25017 / 25019 / 25054 (relatives), 953699 (genus) | <https://ppbc.iplant.cn/list21?keyword=%E5%A4%A7%E6%A0%91%E6%8A%B9%E5%86%9C> |

**Nomenclatural note the modeller should know:** GBIF does **not** index the trinomial — a query for `Rhododendron protistum var. giganteum` returns zero records today; only the binomial resolves ([GBIF](https://api.gbif.org/v1/species?name=Rhododendron%20protistum%20var.%20giganteum)). Flora of China treats the taxon at **species rank with two varieties** — var. *protistum* and var. *giganteum* — and the whole species, not just var. *giganteum*, is the treelike one. Both varieties are 5–10 m trees in the Flora text; the Chinese field record for var. *giganteum* is far larger (see the flagged discrepancy under Habit).

---

## Site

| Fact | Value | Source |
|---|---|---|
| Altitude, Flora of China | **2400–4200 m** — "Mixed forests, *Abies* forests on mountain slopes" | [efloras FOC 14](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200016531) |
| Altitude, Chinese field record for the Gaoligongshan population | **2100–3200 m**, in **evergreen broad-leaved forest (常绿阔叶林)** | [国家林业和草原局政府网](https://www.forestry.gov.cn/c/www/lcdt/564371.jhtml) |
| Counties / provinces | **Tengchong (腾冲), Fugong (福贡), Gongshan (贡山), Lushui (泸水), Yunnan**; Flora of China also gives SE Xizang and NE Myanmar | [国家林业和草原局政府网](https://www.forestry.gov.cn/c/www/lcdt/564371.jhtml); [efloras FOC 14](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200016531) |
| What it grows with | mixed forest and *Abies* (fir) forest on mountain slopes — so it is a **subalpine-slope** tree, standing below/among firs, not a lowland companion | [efloras FOC 14](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200016531) |

**The two altitude figures disagree and both are honest.** FOC's 2400–4200 m is the botanical range; the Chinese forestry record of 2100–3200 m describes the Gaoligongshan population specifically. Use 2100–3200 m for a Gaoligongshan scene and 2400–4200 m for a generic montane scene.

### Is it a relict / endemic, and what protects it

- **Not a relict and not endemic to China.** It is a Sino-Himalayan element; Flora of China gives SE Xizang, W Yunnan and NE Myanmar. Do not model it as a Chinese-endemic relict — its rarity is demographic, not evolutionary.
- **Protection status:**
  - **IUCN: Critically Endangered** — "categorized as critically endangered by the International Union for Conservation of Nature for its small population and restricted distribution" ([Kunming Institute of Botany, CAS](http://english.kib.cas.cn/rh/es/202206/t20220606_306200.html); same text at [english.cas.cn](https://english.cas.cn/special-reports/5thwcbr/wcbrrn/202601/t20260105_1145082.shtml)).
  - **Yunnan 极小种群野生植物保护名录** — listed in 2010 under the provincial ten-year rescue plan; **removed in 2021** because protection had succeeded ([KIB CAS](http://english.kib.cas.cn/rh/es/202206/t20220606_306200.html); [国家林业和草原局政府网](https://www.forestry.gov.cn/c/www/lcdt/564371.jhtml): "因保护成效明显，大树杜鹃已被移出《云南省极小种群野生植物保护名录》").
  - Wild population: about **3,000+ plants** nationally; **1,771** in the Datang (大塘) branch of Gaoligongshan National Nature Reserve, of which ~1,200 adults and >500 seedlings ([国家林业和草原局政府网](https://www.forestry.gov.cn/c/www/lcdt/564371.jhtml); [KIB CAS](http://english.kib.cas.cn/rh/es/202206/t20220606_306200.html)).
  - **Growth is very slow:** the first five or six years are slow and it takes **more than 20 years to flower** ([国家林业和草原局政府网](https://www.forestry.gov.cn/c/www/lcdt/564371.jhtml)). A model that shows a flowering crown on a 5 m tree is wrong — flowering trees are at least 20 years and typically over 100 years old.

---

## Habit

> **FLAGGED DISCREPANCY — read before using any height number.**
> Flora of China describes the **species** as "Trees, **5–10 m tall**" ([efloras FOC 14](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200016531)). The Chinese field record for var. *giganteum* says **"树高可达30米"** (height up to 30 m) and that the tree grows "十多米" ([国家林业和草原局政府网](https://www.forestry.gov.cn/c/www/lcdt/564371.jhtml)). Forrest's 1918 type discovery was **25 m tall, 87 cm DBH** ([KIB CAS](http://english.kib.cas.cn/rh/es/202206/t20220606_306200.html)). FOC's 5–10 m cannot be right for the giant variety; treat it as the species' *typical* stature and use **10–30 m** for var. *giganteum*, with 25 m and 30 m documented.

- **Height:** typical mature trees roughly **10–20 m**; the largest known reach **25–30 m**. Documented values: 25 m (Forrest's 1918 tree, [KIB CAS](http://english.kib.cas.cn/rh/es/202206/t20220606_306200.html)); up to 30 m ([国家林业和草原局政府网](https://www.forestry.gov.cn/c/www/lcdt/564371.jhtml)).
- **Trunk diameter — the species' whole point, and the best-measured numbers available:**
  - **87 cm DBH** — Forrest's 1918 tree.
  - **Twelve trees of ca. 100 cm DBH** found by the 1982 survey.
  - **Largest in the world: basal diameter 307 cm in 1982, grown to 330 cm** by the 2022 report.
  - All four figures: [KIB CAS](http://english.kib.cas.cn/rh/es/202206/t20220606_306200.html) (Xinhua) and [english.cas.cn](https://english.cas.cn/special-reports/5thwcbr/wcbrrn/202601/t20260105_1145082.shtml).
  - So: **trunk radius at breast height 0.44 m for a large tree, up to 1.65 m at the base of the single largest individual.**
- **Silhouette, in one sentence:** a stout straight single trunk running clear for much of the tree's height and then forking into a few thick ascending limbs that carry a sparse, open crown made of discrete drooping leaf rosettes with one round flower ball on each — a candelabra in form, not a dome.
- **How the crown changes with age:** a sapling is a single upright shoot carrying rosettes at every node along it (visible in [14-rosette-labelled-plant.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/14-rosette-labelled-plant.jpg) and [15-rosette-new-flush.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/15-rosette-new-flush.jpg)). With age the lower leaf rosettes are shed, the shoot extends, and the leaf clusters become confined to the ends of the limbs; the trunk becomes clean and the crown becomes a sparse scatter of clusters with visible sky between them. [01-habit-whole-tree-slope.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/01-habit-whole-tree-slope.jpg) shows the mature crown — open, irregular, with gaps.

---

## Branching

- **Phyllotaxy — the single most important fact in this note.** *Rhododendron* does **not** spread its leaves along the shoot. The leaves are crowded into a **terminal rosette** at the end of each shoot, the shoot then dies or continues from a lateral, and the next flush is again a rosette. Confirm on the photographs: [09-rosette-top-view-count-leaves.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/09-rosette-top-view-count-leaves.jpg) shows two rosettes of **7–9 leaves each**, all leaves radiating from one point, with the bare shoot running out of the frame below; [08-rosette-abaxial-tomentum.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/08-rosette-abaxial-tomentum.jpg) and [rel-R-arboreum-rosette-top.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/rel-R-arboreum-rosette-top.jpg) show the same geometry in other species of the genus.
- **Shoot:** "shoots stout, **ca. 1 cm in diam.**, yellow-gray-tomentose" ([efloras FOC 14](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200016531)).
- **First-order branches:** a few thick ascending limbs from the upper trunk, the branches themselves sinuous and often contorted (see [02-crown-mossy-branches.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/02-crown-mossy-branches.jpg) and [01-habit-whole-tree-slope.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/01-habit-whole-tree-slope.jpg)). Unlike *Davidia*, the limbs here **droop** — the rosettes and their long leaves hang.
- **Lower branches:** shed as the trunk extends, leaving a long clean bole. Visible in [03-trunk-bark-CONGENER-delavayi.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/03-trunk-bark-CONGENER-delavayi.jpg).
- **Is the crown layered?** No — it is a scatter of separate clusters, not tiers and not a shell. This gap-and-cluster texture is the genus's signature at distance.
- **Bark:** Flora of China gives no bark description for *R. protistum*. Two congeneric treelike species do: *R. delavayi* "bark **grayish, exfoliating into thin and small irregular flakes**" ([efloras FOC 14, *R. delavayi*](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200016396)) and *R. arboreum* "bark **gray-brownish, exfoliating into thin and small irregular flakes**" ([efloras FOC 14, *R. arboreum*](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200016337)). The photograph [03-trunk-bark-CONGENER-delavayi.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/03-trunk-bark-CONGENER-delavayi.jpg) shows exactly that: a stout grey bole with thin small flaking scales. **This is a congener, flagged as such — PPBC holds no trunk photograph of *R. protistum* itself.** For the record, the largest known specimen has a basal diameter of 3.30 m, so a base radius near 1.65 m is within the model range, and the bole is buttressed at that scale.

---

## Trunk

- Straight and stout, **ca. 1 cm in diam. for shoots**, trunk up to **3.30 m** basal diameter on the single largest known tree.
- Colour and texture, from the two congener accounts cited above: **grey to grey-brown, exfoliating into thin, small, irregular flakes** — a fine scaly/flakey bark, not a furrowed or plated one. Photo: [03-trunk-bark-CONGENER-delavayi.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/03-trunk-bark-CONGENER-delavayi.jpg) (congener, *R. delavayi*, Cangshan).
- Limbs in the crown are **dark, contorted and often mossy**, as in [02-crown-mossy-branches.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/02-crown-mossy-branches.jpg).

---

## Foliage

All from [efloras FOC 14](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200016531) unless marked. **var. giganteum** figures marked "var.".

| Property | Value |
|---|---|
| Leaf type | simple |
| **Distribution along a shoot** | **crowded in a TERMINAL ROSETTE**, not spread along it. This is the defining contrast with *Davidia*. |
| **Blade length × width** | species **12–45 × 5–20 cm**; **var. giganteum 20–45 cm** long. *Flagged:* a Chinese field report gives leaves "最长接近70厘米" (longest approaching 70 cm) ([国家林业和草原局政府网](https://www.forestry.gov.cn/c/www/lcdt/564371.jhtml)) — that is 1.5× Flora of China's maximum and is probably a record of one exceptional leaf, or is measuring leaf-plus-petiole-plus-arching-tip. Use FOC's figure; treat 70 cm as an outlier. |
| **Petiole** | **25–40 mm, stout, terete (round in section), glabrous** |
| Blade shape | **oblong-lanceolate to oblong-oblanceolate** — for var. giganteum, the long oblanceolate end dominates, so the leaf is widest above the middle and tapers to the base |
| Base | **cuneate** (wedge-shaped) |
| Apex | **obtuse, mucronate** — rounded with a small point, *not* drawn out |
| Margin | **entire** (FOC states no margin teeth for this species) |
| Thickness | **thickly leathery** |
| Venation | midrib **stout and conspicuously raised on the underside**; the blade is strongly **bullate / corrugated** between the lateral veins, visible in [05-leaf-upper-surface.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/05-leaf-upper-surface.jpg) and [10-leaf-with-hand-scale.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/10-leaf-with-hand-scale.jpg) |
| Evergreen / deciduous | **evergreen.** FOC does not state it for this species; the thickly leathery blade and the observed year-round foliage of cultivated plants (e.g. Shanghai Botanical Garden, [10-leaf-with-hand-scale.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/10-leaf-with-hand-scale.jpg)) mark it evergreen. Marked as **inferred**, not flora-given. |
| Upper surface | **glabrous**, sometimes with vestiges of hairs; bright to dark green, glossy |
| Lower surface | **appressed pale brown tomentum**; in **var. giganteum the tomentum is restricted to a marginal band at maturity** — a diagnostic field mark. See [07-leaf-underside-marginal-indumentum.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/07-leaf-underside-marginal-indumentum.jpg) and [11-leaf-underside-macro.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/11-leaf-underside-macro.jpg) |

**Rosette geometry for the modeller** — counted from the photographs, not from flora:

| Quantity | Value | Basis |
|---|---|---|
| **Leaves per rosette** | **6–9** | **ESTIMATE, counted** from [09-rosette-top-view-count-leaves.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/09-rosette-top-view-count-leaves.jpg) (7 in one, 9 in the other) and [rel-R-arboreum-rosette-top.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/rel-R-arboreum-rosette-top.jpg) (8–9) |
| Rosette diameter | **25–40 cm** | **ESTIMATE**, from the 20–45 cm leaf length arranged radially in [09](C:/resource/whiting/out/refs/rhododendron_protistum/09-rosette-top-view-count-leaves.jpg) and [14](C:/resource/whiting/out/refs/rhododendron_protistum/14-rosette-labelled-plant.jpg) |
| Angle of a leaf from the shoot axis | **40–70°** — ascending out of the rosette centre then arching down | **ESTIMATE**, from [04-branch-rosette-with-truss.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/04-branch-rosette-with-truss.jpg) and [rel-R-lacteum-crown.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/rel-R-lacteum-crown.jpg) |
| Rosette vertical spacing on a branch | **20–40 cm** | **ESTIMATE**, from [06-flower-truss-masses.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/06-flower-truss-masses.jpg) |

---

## Reproductive

All from [efloras FOC 14](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200016531).

- **Inflorescence type:** **racemose-umbellate, 20–30-flowered**, on a **rachis ca. 60 mm**. So this is not a tight ball but a short raceme with the pedicels crowded at the top — an umbel on a short axis.
- **Pedicel:** stout, **1.5–2 cm**, densely tawny-tomentose.
- **Calyx:** **8 lobes**, 2–3 mm, unequal, triangular, tomentose.
- **Corolla:** **funnelform-campanulate**; **8 lobes**; **with a dark basal blotch**; colour across the species creamy-white, rose, or deep purple-red, length **5–8 cm**. For **var. giganteum: 5–5.5 cm, creamy-white tinged rose**.
- **The octamerous plan — 8 calyx lobes, 8 corolla lobes, 16 stamens — is diagnostic of this species** and is why the truss reads as a densely many-petalled, almost spherical mass in [06-flower-truss-masses.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/06-flower-truss-masses.jpg).
- **Where the truss sits relative to the leaf rosette: in the centre of it.** The inflorescence is terminal on the shoot and its pedicels radiate out of, and just above, the whorl of leaves, so the flower ball sits inside the star of the rosette. That is exactly what [04-branch-rosette-with-truss.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/04-branch-rosette-with-truss.jpg) and [06-flower-truss-masses.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/06-flower-truss-masses.jpg) show. Do **not** model the truss as a separate organ hanging from the branch.
- **Ovary** 9–11 mm, densely tawny fasciculate-tomentose; **style ca. 4 cm**, glabrous; **stigma large and discoid-lobulate**.
- **Capsule:** cylindric, **ca. 40 × 12 mm**, tomentose.
- **Season: flowering March–May, fruiting August–November.**
- Scale check: the [国家林业和草原局政府网](https://www.forestry.gov.cn/c/www/lcdt/564371.jhtml) report says the largest flower balls are "直径最大的能与排球相当" — as wide as a volleyball (about 21 cm). FOC's 20–30 flowers × 5–5.5 cm corolla is consistent with that.

---

## The five facts that let you recognise this tree from 200 m away, as a silhouette

1. **Candelabra, not dome.** A stout straight bole running clear for a large fraction of the height, then a few thick limbs forking upward and outward. There is no rounded, closed crown outline — the crown has a ragged, irregular, upward-then-drooping outline. `[01-habit-whole-tree-slope.jpg]`
2. **The crown is a scatter of separate leaf CLUSTERS with sky between them.** Each cluster is a tight star of 6–9 long leaves radiating from a single point, and the clusters are metres apart. A model that produces a continuous foliage shell has produced the wrong genus. `[01-habit-whole-tree-slope.jpg, 09-rosette-top-view-count-leaves.jpg]`
3. **Nearly every cluster carries one round flower ball of about the same width as the cluster.** In flower the tree reads as pale pink/lilac balls scattered over dark limbs, not as a coloured canopy. `[06-flower-truss-masses.jpg]`
4. **The leaves hang.** Each rosette's long oblanceolate leaves droop out of and below the whorl, so the crown fringe is a fringe of drooping blades and the whole crown looks shaggy rather than smooth-edged. `[04-branch-rosette-with-truss.jpg, rel-R-lacteum-crown.jpg]`
5. **The leaf clusters are resolvable at 200 m.** With 20–45 cm leaves on a 10–30 m tree, individual rosettes subtend a large angle and the crown texture is coarse. Most tree species have gone to a uniform texture by that distance; this one has not. That coarse cluster texture is the giveaway.

---

## Modelling numbers

| Quantity | Value | Basis |
|---|---|---|
| Mature height | **10–20 m typical; 25–30 m for the giants** | 25 m documented (Forrest 1918), 30 m documented ([forestry.gov.cn](https://www.forestry.gov.cn/c/www/lcdt/564371.jhtml)). FOC's "5–10 m" flagged as inconsistent for this variety |
| Trunk DBH, large tree | **0.87–1.00 m** | documented (Forrest 1918; the twelve 1982 trees) |
| Trunk basal diameter, largest known individual | **3.30 m** (was 3.07 m in 1982) | documented ([KIB CAS](http://english.kib.cas.cn/rh/es/202206/t20220606_306200.html)) |
| Trunk radius at base | **0.44–0.50 m typical; up to 1.65 m for the largest** | half the DBH figures above |
| Clear trunk height | **0.35–0.55 × total height**, i.e. 4–10 m on a 10–20 m tree | **ESTIMATE** — trunk fraction in [03-trunk-bark-CONGENER-delavayi.jpg](C:/resource/whiting/out/refs/rhododendron_protistum/03-trunk-bark-CONGENER-delavayi.jpg) and the bole in `01-habit-whole-tree-slope.jpg`; FOC gives no value |
| Crown width | **0.5–0.8 × height** | **ESTIMATE** from `01-habit-whole-tree-slope.jpg` |
| Crown depth | **0.3–0.5 × height** — the crown is much shallower than wide, and vertically it is mostly droop | **ESTIMATE** from `01-habit-whole-tree-slope.jpg` |
| Number of first-order branches | **4–8** | **ESTIMATE** — counted in `03-trunk-bark-CONGENER-delavayi.jpg` and `01-habit-whole-tree-slope.jpg` |
| Angle of a first-order branch from the trunk axis | **30–60°**, then arching to horizontal and down | **ESTIMATE** from the same two images |
| Twig length (the shoot carrying one rosette) | **15–30 cm** | **ESTIMATE** — FOC gives shoot diameter (~1 cm) but no length; from `09-rosette-top-view-count-leaves.jpg` |
| Shoot diameter | **ca. 1 cm** | flora (FOC) |
| **Leaves per shoot tip (rosette)** | **6–9** | **ESTIMATE, counted** from the photographs |
| **Leaf length × width** | **20–45 × 5–20 cm** (var. giganteum) | flora (FOC) |
| **Petiole length** | **2.5–4 cm** | flora (FOC) |
| Rosette diameter | **25–40 cm** | **ESTIMATE** from leaf length arranged radially |
| Leaf angle from the shoot axis in the rosette | **40–70°** | **ESTIMATE** from photographs |
| Rosette spacing along a branch | **20–40 cm** | **ESTIMATE** from `06-flower-truss-masses.jpg` |
| Truss flower count | **20–30** | flora (FOC) |
| Truss rachis length | **ca. 6 cm** | flora (FOC) |
| Pedicel length | **1.5–2 cm** | flora (FOC) |
| Truss diameter | **12–18 cm; up to ~21 cm for the largest** | **ESTIMATE** from flower count × corolla size; the volleyball comparison ([forestry.gov.cn](https://www.forestry.gov.cn/c/www/lcdt/564371.jhtml)) gives the ceiling |
| Corolla length | **5–5.5 cm** (var. giganteum), 8 lobes | flora (FOC) |
| Capsule | **ca. 40 × 12 mm** | flora (FOC) |
| Minimum age to flower | **> 20 years** | documented ([forestry.gov.cn](https://www.forestry.gov.cn/c/www/lcdt/564371.jhtml)) |

---

## Other treelike rhododendrons of SW China — which ones, and why

The client offered *R. griffithianum* and *R. burjaticum*. Two naming corrections and one judgement call:

**Name corrections, checked today against Flora of China vol. 14 and PPBC:**

| Chinese name offered | Flora of China vol. 14 (2026-10-18) | PPBC (2026-10-18) |
|---|---|---|
| 黄杯杜鹃 | ***Rhododendron wardii* W. W. Smith** (FOC 14 p. 348, taxon id 200016606) — *R. griffithianum* is a separate accepted name in FOC under the Chinese name **不丹杜鹃** | cid **24942**, resolved to *Rhododendron wardii* |
| 乳黄杜鹃 | ***Rhododendron lacteum* Franch.** (FOC 14 p. 390, taxon id 200016463) | cid **25054**, resolved to *Rhododendron lacteum* |

`Rhododendron burjaticum` was **not found** in Flora of China vol. 14 or on PPBC today. The taxon PPBC and FOC both hold under 乳黄杜鹃 is *R. lacteum*, and it is genuinely treelike, so the client's intent survives the name change.

**The judgement call.** Flora of China gives these habits verbatim, and they decide the selection:

| Species | FOC habit statement | Treelike? |
|---|---|---|
| *Rhododendron wardii* (黄杯杜鹃 = client's *R. griffithianum*) | "**Shrubs ca. 3 m tall**" | **No.** A 3 m shrub does not show the treelike growth form the client asked about. |
| *Rhododendron lacteum* (乳黄杜鹃) | "**Shrubs or small trees, 2–8 m tall**" | **Yes** — and it is the client's own suggestion, so it stays. |
| *Rhododendron arboreum* (树形杜鹃) | "**Trees, usually evergreen, (1–)5–20(–30) m tall**; trunk well-defined" | **Yes** — and it is literally named "tree rhododendron". |
| *Rhododendron delavayi* (马缨杜鹃) | "**Shrubs or trees, 1–7(–12) m tall**" | **Yes** |

So the three kept as treelike references are ***R. arboreum***, ***R. delavayi*** and ***R. lacteum***. *R. wardii* / *R. griffithianum* is **excluded from the image set and documented here as the shrub extreme** (a 3 m shrub with a cup-shaped yellow or white corolla, 3–4 × 4–5 cm, at 3000–4600 m in SW Sichuan, SE Xizang and NW Yunnan — [efloras FOC 14, *R. wardii*](http://www.efloras.org/florataxon.aspx?flora_id=2&taxon_id=200016606)); PPBC photographs of it were not fetched, and none is claimed.

The three kept span the range well: *R. arboreum* is the biggest (up to 20–30 m) with a narrow 6–15 cm leaf and a 3.5–4 cm crimson trumpet; *R. delavayi* is a multi-stemmed 1–7 m tree with a narrow 1–5 cm leaf and fleshy deep-crimson 3–5 cm bells; *R. lacteum* is a 2–8 m small tree with the broadest leaf of the three (8–17 × 6–8 cm) and an unspotted pure-yellow 3.5–4.5 cm corolla.

---

## Images

All under `C:\resource\whiting\out\refs\rhododendron_protistum\`.
Source bank: PPBC. Full-size URL pattern `http://img0.iplant.cn/image61/b/<HASH>.jpg`; detail page `https://ppbc.iplant.cn/tu/<id>`.
`cid 42060` = var. *giganteum*; `cid 24924` = *R. protistum* (species); `cid 25019` = *R. delavayi*; `cid 25017` = *R. arboreum*; `cid 25054` = *R. lacteum*.

| File | Bytes | Pixels | Shows | Taxon / id / photographer / locality | Source URL |
|---|---|---|---|---|---|
| `01-habit-whole-tree-slope.jpg` | 806,597 | 810×1080 | **Whole mature tree** standing on a steep misty montane slope; open irregular crown of leaf clusters, sinuous limbs | var. *giganteum* / tu=27821529 / 赵新杰 / 云南怒江州 福贡县 | <http://img0.iplant.cn/image61/b/3F6D31CF4403DFE0DF1B2FF4497F73BE.jpg> |
| `02-crown-mossy-branches.jpg` | 539,274 | 720×1080 | **Branches of the crown** — dark mossy contorted limbs carrying terminal rosettes and trusses (photographer's fingers at frame edge) | var. *giganteum* / tu=20449368 / 孙庆美 / 云南怒江州 泸水 | <http://img0.iplant.cn/image61/b/D7549EF2383B4CC2F9ECB1BB6B6770AE.jpg> |
| `03-trunk-bark-CONGENER-delavayi.jpg` | 698,568 | 1080×717 | **The trunk and limbs of a mature treelike rhododendron**: stout grey bole, thin small exfoliating flakes, ascending branches, terminal rosettes, crimson trusses. **CONGENER — *R. delavayi* 马缨杜鹃, not this taxon.** PPBC holds no trunk photograph of *R. protistum* | *R. delavayi* / tu=1251618 / 刘军 / 云南大理州 苍山西坡 | <http://img0.iplant.cn/image61/b/CC6CE5EEA036292A.jpg> |
| `04-branch-rosette-with-truss.jpg` | 582,634 | 1080×720 | **The terminal rosette with its flower truss** — leaves radiating from one point, long oblanceolate blades with strongly raised midrib, the pink truss sitting in the centre of the rosette, dark contorted mossy branch behind | var. *giganteum* / tu=27695449 / 赵新杰 / 云南怒江州 贡山县 | <http://img0.iplant.cn/image61/b/86B348083840FABF2DC41DEC1954CF73.jpg> |
| `05-leaf-upper-surface.jpg` | 520,076 | 1080×720 | **Leaf close-up, adaxial**: bullate/corrugated lamina, deeply grooved midrib, raised pale lateral veins | var. *giganteum* / tu=1694488 / 宋鼎 / 云南昆明市 金殿 | <http://img0.iplant.cn/image61/b/7D43F64BEFCF3D79.jpg> |
| `06-flower-truss-masses.jpg` | 561,131 | 607×1080 | **A wall of flowering 大树杜鹃 in the wild** (Tengchong Jietou): every shoot tip ends in a rosette of big oblanceolate leaves carrying one dense magenta-pink truss | var. *giganteum* / tu=26325588 / 黄健 / 云南保山市 腾冲界头 | <http://img0.iplant.cn/image61/b/419E72FE8A410E8574C560CA3B2933B5.jpg> |
| `07-leaf-underside-marginal-indumentum.jpg` | 387,945 | 1080×720 | **Leaf abaxial surface**: conspicuously raised midrib, corrugated lamina, and the pale-brown tomentum running as a **marginal band** — the diagnostic mark of var. *giganteum* | var. *giganteum* / tu=1669801 / 宋鼎 / 昆明植物园 | <http://img0.iplant.cn/image61/b/308FADF11B451703.jpg> |
| `08-rosette-abaxial-tomentum.jpg` | 285,012 | 1080×720 | Terminal rosette from below, showing the pale brown tomentose abaxial surfaces of every leaf, reddish petioles, and the brown scales at the shoot base | var. *giganteum* / tu=1669799 / 宋鼎 / 昆明植物园 | <http://img0.iplant.cn/image61/b/693CCE5794ACECFF.jpg> |
| `09-rosette-top-view-count-leaves.jpg` | 759,668 | 1080×810 | **Two whole rosettes laid flat, seen from directly above — the leaf count is directly countable (7 and 9)**. The best single image for rosette geometry in the genus | ***R. protistum* species**, not var. *giganteum* / tu=27695921 / 刘翔 / 苏格兰 Arduaine garden | <http://img0.iplant.cn/image61/b/903DD0B2F2AF52352D448AAE99055A08.jpg> |
| `10-leaf-with-hand-scale.jpg` | 714,843 | 810×1080 | **A single leaf held in a hand — scale reference.** Deeply impressed midrib, corrugated blade, brown pubescent twig and rosette behind | var. *giganteum* / tu=28614992 / 李连杰 / 上海市松江 上海植物园 | <http://img0.iplant.cn/image61/b/7E1D2C22A8D797DF4CF38AC0FDDFB1F9.jpg> |
| `11-leaf-underside-macro.jpg` | 370,598 | 1080×720 | Leaf abaxial macro: raised midrib, parallel lateral veins, pale underside | var. *giganteum* / tu=1669798 / 宋鼎 / 昆明植物园 | <http://img0.iplant.cn/image61/b/90E9D7EC548E92B1.jpg> |
| `12-bud-break-scales.jpg` | 207,419 | 1080×720 | Bud break: red scales peeling, new tomentose shoots emerging inside the rosette | var. *giganteum* / tu=1669802 / 宋鼎 / 昆明植物园 | <http://img0.iplant.cn/image61/b/BC701EC53CAD3772.jpg> |
| `13-corolla-dissected-with-scale.jpg` | 342,857 | 1080×720 | **A corolla split open and laid flat beside a ruler** — shows the 8 lobes, the reticulate venation and the pink-then-white gradient | var. *giganteum* / tu=27695192 / 赵新杰 / 云南怒江州 福贡县 | <http://img0.iplant.cn/image61/b/87176BA5B0EBBFCFF695990DFA8DF265.jpg> |
| `14-rosette-labelled-plant.jpg` | 560,434 | 810×1080 | A rosette of 6–7 huge oblanceolate leaves with the plant label 大树杜鹃 beside it (cultivated, Shanghai Botanical Garden) | var. *giganteum* / tu=28614993 / 李连杰 / 上海植物园 | <http://img0.iplant.cn/image61/b/DFEE7EF38C87E8A78EA8C19A33C17906.jpg> |
| `15-rosette-new-flush.jpg` | 375,031 | 720×1080 | The rosette with a **pale, densely tomentose new flush** rising in the centre, over old leaves | var. *giganteum* / tu=29113656 / 周立新 / 云南怒江州 片马 | <http://img0.iplant.cn/image61/b/3EFDEB7D7128E8FF9FD4F569C2110A28.jpg> |
| `rel-R-arboreum-rosette-top.jpg` | 331,158 | 1080×719 | **Tree rhododendron rosette from directly above, 8–9 leaves**, brown scales and the new bud cluster in the centre — the genus's rosette geometry in a second species | *R. arboreum* 树形杜鹃 / tu=4977277 / 徐永福 / 贵州毕节 | <http://img0.iplant.cn/image61/b/2377B726FE852D80.jpg> |
| `rel-R-lacteum-crown.jpg` | 640,260 | 1080×720 | **乳黄杜鹃 *R. lacteum* crown** — terminal rosettes and pale-yellow trusses on dark contorted branches; a small tree 2–8 m tall | *R. lacteum* / tu=2749571 / 宋鼎 / 云南大理州 苍山 | <http://img0.iplant.cn/image61/b/B4CFCFDF01E890D7.jpg> |
| `rel-R-delavayi-flowering-branch.jpg` | 443,993 | 1080×717 | **马缨杜鹃 *R. delavayi* in flower** — fleshy crimson tubular-campanulate corollas, grey exfoliating branch bark, silvery-fawn abaxial leaf surfaces | *R. delavayi* / tu=1251587 / 刘军 / 云南大理州 苍山西坡 | <http://img0.iplant.cn/image61/b/F778C0642FAC268F.jpg> |

**18 photographs kept** (15 of var. *giganteum*, 1 of the species, 3 relatives — one of which, `03-trunk-bark-CONGENER-delavayi.jpg`, is a congener standing in for the trunk the client asked for and for which PPBC holds no photograph).
