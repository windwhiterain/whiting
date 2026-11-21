# Verification record — reference photographs, Southwest China trees

*Compiled 2026-10-18 during the collection session.*

## What the sources actually behave like

Three findings that a repeat collector needs, because two of them invalidate the obvious recipe.

**1. The image CDN blocks `curl.exe` and `Invoke-WebRequest` outright.**
`img*.iplant.cn` answers those clients with a WAF page — *"您的访问请求可能对网站造成安全威胁，请求已被阻断"* — over HTTPS, and with a **24×81 "中国植物图像库 / PlantPhoto.Cn" watermark placeholder** over plain HTTP. A plain `urllib` request carrying `Referer: https://ppbc.iplant.cn/` gets the real bytes. Going through the local proxy at `127.0.0.1:7897` also returns 403, so it is not an egress-IP problem.

**2. It is also aggressively rate-limited.** Fetching candidates in a tight loop earns a 403 (or a TLS `UNEXPECTED_EOF`) that then refuses every other CDN host for several minutes. The harvest scripts therefore fetch one image at a time with a delay and exponential backoff, and are resumable — a block costs a pause, not the work. Downloads take ~40–60 s each.

**3. Thumbnails only, unless the size code is rewritten.** PPBC hands out `/image2/155/…` (235×155 px) and `/image2/i/…` (75×75 px). The folder segment selects the stored variant: `/image2/1200/…` returns **700 px on the long edge**. That single substitution is the difference between unusable thumbnails and reference-grade frames.

Working endpoints (all measured today):
- `https://ppbc.iplant.cn/ashx/getspphotolist.ashx?cid=<cid>&t=0.5&n=<seed>` — 4 URLs per call; `n` is a deterministic seed, so walking `n=0..12` returns different slices of one gallery.
- `https://www.iplant.cn/ashx/getfrps.ashx?key=<Latin>` — 中国植物志 text, the single richest source here (dimensions, indumentum, venation, phenology).
- `https://www.iplant.cn/ashx/getfoc.ashx?key=<Latin>` — Flora of China, English, often adds trunk diameter and habit words the Chinese omits.
- `https://api.gbif.org/v1/species/match?name=<Latin>` and `/v1/species?datasetKey=d7dddbf4-…&name=` — accepted name and authority.
- `https://api.gbif.org/v1/occurrence/search?taxonKey=<key>&mediaType=StillImage` — image metadata. All Bombax and Betula frames came from `inaturalist-open-data.s3.amazonaws.com`, which is reachable and is **not** the blocked `static.inaturalist.org`.

Dead ends, as the brief stated: wikipedia/wikimedia, gbif HTML, powo, fs.usda, biodiversitylibrary, florabase, baidu image search, baidu baike. Confirmed additionally dead: `getphotonum.ashx` and `getphotopage.ashx` return empty at every parameter spelling tried.

## The PPBC filler trap

PPBC serves the **same four images** for any species it holds no photographs of. They are identifiable because they recur byte-for-byte across unrelated taxa:

| File hash prefix | What it actually shows |
|---|---|
| `70FB538C5400F219` | pink-flowered *Ceiba* (floss-silk relative), long exserted stamens |
| `776BC85E8821B8DD` | a white star-flowered water-lily floating on dark water |
| `781A2B10780B7C7F` | a pink-flowered *Ceiba* in foliage |
| `7923D5C2639E9DF3` | a yellow-flowered forest herb on a leaf-litter floor |

These four were returned for 木棉, *Bombax malabaricum*, 西南桦 and 云南油杉. **Treat any species whose gallery is exactly four, or exactly these hashes, as having no photographs.**

## Cids that resolve

The colloquial name usually fails; the FRPS name usually works.

| Requested name | Resolves? | cid | Genuine photos |
|---|---|---|---|
| 木棉 / Bombax ceiba | yes | 226155 | 5 of 9 |
| Bombax malabaricum (synonym) | yes | 22543 | **0** — the same 4 filler files |
| 筇竹 | yes | 222992 | 5 of 9 |
| 滇油杉 | **no** | — | — |
| 云南油杉 / Keteleeria evelyniana | yes | 10539 | 8 of 12 |
| 西南桦 | **no** (returns filler) | 286328 | 0 |
| 西桦 / Betula alnoides | yes | 11174 | 4 of 10 (2 were duplicates) |

## GBIF identifications are not reliable at face value

Of the Betula records GBIF returned under key 5331519, the **four from Thailand were a different plant entirely** — a narrow-leaved tropical broadleaf in a garden landscape, in a country where *Betula alnoides* does not occur. Only the Indian Himalayan frames were usable, and those are observer-identified in a genus where *B. utilis* is the standard confusion. The Chinese PPBC frames were therefore used for every leaf, catkin and fruit character.

For *Qiongzhuea tumidinoda*, **GBIF holds no images at all** (`candidates=0`), and the species is one of only two Class III protected bamboos, so there are no cultivated stock photographs either.

## Errors caught by looking at the files

Every kept image was opened with `read_image` before it was named. That caught:

- **4 of 9** PPBC candidates for 木棉 that were other species.
- **1 of 12** Keteleeria candidates — a broad-crowned tree in a manicured park bed whose needles could not be read, so its identity was unconfirmable; discarded.
- **1 GBIF Bombax frame** — a fallen flower photographed on concrete.
- **4 GBIF Betula frames** — the misidentified Thailand plants.

It also corrected the *file names* as written: PPBC file `01` for 木棉 is a flower close-up, not a habit shot, and the PPBC Bamboo file named for a "node" study turned out to be the strongest diagnostic frame in the whole set.

## Corrections to the brief, from the sources

1. **西南桦 is not white-barked.** FRPS gives **红褐色** (reddish-brown). The photographs show a pale papery outer bark peeling to a cinnamon underlayer. The white-bark birches are *Betula utilis*.
2. **"Taxus wallichiana var. yunnanensis" is used by nobody.** FRPS has **Taxus yunnanensis** as a species; GBIF returns an empty result for the varietal name and lists *T. yunnanensis* as a synonym of *T. wallichiana*.
3. **The silver leaves belong to the wrong subspecies of 沙棘.** The Southwest-China plant is subsp. *yunnanensis*, whose leaf underside is **grey-brown with rust-coloured scales**. Silver-white is subsp. *sinensis*, the northern crop.
4. **滇油杉 is not the name PPBC knows** — it resolves under 云南油杉.

## Deliverable status

| Species | Images kept | Note |
|---|---|---|
| Bombax ceiba | 9 | `bombax_ceiba.md` |
| Qiongzhuea tumidinoda | 5 | `qiongzhuea_tumidinoda.md` |
| Keteleeria evelyniana | 7 | `keteleeria_evelyniana.md` |
| Betula alnoides | 6 | `betula_alnoides.md` |
| Tsuga dumosa | — | `tsuga_dumosa.md` |
| Taxus yunnanensis | — | `taxus_wallichiana_var_yunnanensis.md` |
| Cyclobalanopsis glauca | — | `cyclobalanopsis_glauca.md` |
| Machilus yunnanensis | — | `machilus_yunnanensis.md` |
| Hippophae rhamnoides | — | `hippophae_rhamnoides.md` |
| Liriodendron chinense | — | `liriodendron_chinense.md` |

The bamboo holds **5** images, below the requested 6, and the *Keteleeria* set has **no dedicated whole-trunk bark close-up**. Both gaps are recorded in their notes rather than papered over.

## Scripts

Under `C:\resource\whiting\out\refs\_tools\`: `harvest2.py` (PPBC harvest, patient and resumable), `facts.py` (FRPS + FOC + GBIF into `facts/`), `gbif_dl.py` (GBIF occurrence media), `organize.py` (move verified files to content-named names), and `probe_*.py` (the endpoint probes behind the findings above).
