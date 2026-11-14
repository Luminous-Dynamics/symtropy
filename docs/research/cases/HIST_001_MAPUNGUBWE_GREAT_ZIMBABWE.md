# HIST-001 — Shashe–Limpopo and the Mapungubwe–Great Zimbabwe Network

**Version:** 0.1.0  
**Date:** 2026-11-02  
**Status:** Research case specification; not a calibrated simulation or complete archaeological dataset.

## 1. Why this is the first case

The Shashe–Limpopo region provides a bounded, evidence-rich case for testing how settlements, trade networks, environmental constraints, social differentiation, and neighboring communities interact over time.

It is especially valuable because it challenges simplistic stage models. The available chronology indicates that Mapungubwe and Great Zimbabwe overlapped; the regional record is better investigated as a polycentric network than as a single succession of one civilization replacing another.

This is a research case, not a claim that the entire region can be described by a single political unit or that one causal explanation for Mapungubwe's abandonment has been established.

## 2. Primary research questions

1. How did earlier forager and farming communities interact in the Shashe–Limpopo landscape?
2. What environmental and exchange-network conditions supported the growth of Schroda, K2 and Mapungubwe?
3. What changed as settlement activity shifted from K2 toward Mapungubwe Hill and its terraces?
4. How did imported goods, local production, craft practices and regional trade networks evolve?
5. What archaeological evidence supports changes in social differentiation, settlement organization and access to high-status goods?
6. How did Great Zimbabwe develop in relation to, and partly contemporaneously with, Mapungubwe and other regional centres?
7. What combination of environmental, political, economic and network processes may explain changing occupation patterns around the end of the Mapungubwe sequence?

## 3. Evidence ledger: claims, evidence, and limits

The following is a source map for data acquisition and expert review. It is not itself the source dataset. Every imported observation must retain the source publication and underlying measurement context.

### H-01 — Regional Middle Iron Age chronology

**Source:** Huffman & Woodborne (2021), *New AMS dates for the Middle Iron Age in the Mapungubwe landscape*, South African Journal of Science.  
DOI: https://doi.org/10.17159/sajs.2021/8980

The paper reports more than 500 Middle Iron Age sites dated broadly to AD 900–1320 and 15 new accelerator-mass-spectrometry dates from 11 archaeological sites. These dates help establish boundaries between ceramic facies and inform research on land use, population dynamics, social differentiation and rainfall fluctuations.

**Current transcription fixture:** [HIST_001_AMS_TABLE1_TRANSCRIPTION_V0_1.json](HIST_001_AMS_TABLE1_TRANSCRIPTION_V0_1.json) records the 15 Table 1 rows, including all listed SHCal20 1σ intervals and the source's struck-through/retained status. It is marked `transcribed_not_independently_reviewed`; it is a table transcription, not raw laboratory data or a calibrated probability distribution. Source-PDF and table-image digests remain null because the exact bytes have not yet been snapshotted.

**Source-internal discrepancy:** Table 1 reports IT-C-785 as 660 ± 52 BP, while a later paragraph in the same article refers to the same lab identifier as 650 ± 52 BP. Both statements are preserved in the transcription artifact's `source_internal_discrepancies`; the table value remains the Table 1 transcription, not a resolution of the source conflict. Do not average or silently select either number.

**Limit:** do not collapse a calibrated, potentially multimodal date distribution into one exact calendar year. Do not infer population size directly from site counts without a sampling/detection model. The published Table 1's crossed-out intervals are author-eliminated options, not calibrated posterior weights.

7. What happened in the Limpopo Valley after Mapungubwe's abandonment, and how should later Sotho/Icon and Kalanga/Khami occupation patterns be represented without treating the region as empty?

### H-02 — K2 and Mapungubwe settlement sequence and shell-bead variation

**Source:** 2023 article, *Shell disc beads and the development of class-based society at the K2-Mapungubwe settlement complex (South Africa)*, Archaeological and Anthropological Sciences.  
DOI: https://doi.org/10.1007/s12520-023-01735-2

The study discusses K2 and Mapungubwe as different spatial and temporal foci of a regional community. It reports a phase sequence in which K2 was prominent before a relocation toward Mapungubwe around the early thirteenth century, with differences in shell-disc-bead materials and spatial distributions. Its discussion connects material variation to changing social differentiation.

**Data requirements:** assemblage identity, find context, site/household/terrace, shell material, count, dating association, excavation history, classification method and uncertainty.

**Limit:** shell-bead variation may reflect chronology, trade, craft practice, status, access, or sampling. Do not convert a material difference directly into a fixed class label without archaeological interpretation.

### H-03 — Great Zimbabwe chronology and regional overlap

**Source:** *A Bayesian chronology for Great Zimbabwe: re-threading the sequence of a vandalised monument*, Antiquity (2013).  
DOI: https://doi.org/10.1017/S0003598X00049516

The study combines radiocarbon dates, limited stratigraphic information and datable imports in a Bayesian chronology. It argues that construction of stone walls probably began in the late twelfth or early thirteenth century, peaked in the fourteenth and fifteenth centuries, and occupation continued at least into the sixteenth and probably seventeenth centuries. It also argues that Great Zimbabwe overlapped with Mapungubwe, supporting a polycentric model of regional sociopolitical complexity.

**Data requirements:** dated contexts, stratigraphic relationships, imported-object dating associations, model priors, calibration curve, posterior distributions, and explicit model alternatives.

**Limit:** do not treat the chronology as exact at year resolution. Do not encode Great Zimbabwe as a simple one-way successor that begins only after Mapungubwe disappears.

### H-04 — Indian Ocean exchange and material circulation

**Source:** University of Cambridge, Department of Archaeology, *Archaeological science and globalisation: Great Zimbabwe*.  
https://www.arch.cam.ac.uk/research/projects/recently-completed-projects/archaeological-science-and-globalisation-great

The project describes Great Zimbabwe's wide regional and Indian Ocean connections and the circulation of gold, iron, ivory and other commodities outward, with imported alloys, glass beads and ceramics arriving and being adapted locally.

**Data requirements:** artifact provenance, material composition, source-region hypothesis, dating context, trade-route hypothesis, and confidence/alternative origins.

**Limit:** an imported object establishes circulation or contact more securely than it establishes a specific political treaty, direct voyage, or centralized trade institution. Distinguish direct exchange from down-the-line redistribution.

### H-05 — Forager–farmer interaction

**Source:** Forssman, *Contesting space: contact between foragers and farmers in the Mapungubwe cultural landscape*, Antiquity.  
https://doi.org/10.15184/S2978855200901938

The work describes the arrival of farming communities around AD 900 and proximity to forager camps, including exchange involving ivory and imported goods. The archaeological sequence also includes changes in the forager record.

**Data requirements:** site chronology, settlement type, artifact and faunal assemblage, material-use context, landscape use, and independent evidence supporting interaction.

**Limit:** disappearance from the archaeological record is not proof that a living population disappeared. Model archaeological visibility and survey coverage; do not turn absence of finds into demographic extinction without supporting evidence.

### H-06 — Climate and abandonment hypotheses

**Sources:**
- Huffman & Woodborne (2021), DOI above.
- UNESCO Mapungubwe Cultural Landscape tentative-list dossier: https://whc.unesco.org/en/tentativelists/5557

The UNESCO tentative-list dossier discusses drought and crop failures associated with the onset of the Little Ice Age as a factor in the region's transformation. This is a hypothesis to evaluate alongside political, economic, ecological, demographic and exchange-network explanations.

**Data requirements:** independently reconstructed rainfall/hydroclimate series, proxy type, location, dating uncertainty, calibration, spatial representativeness, and linkage assumptions to agricultural production.

**Limit:** the existence of drought or climatic change does not by itself establish that it caused abandonment. Avoid single-cause collapse narratives and distinguish a source's interpretation from direct proxy measurement.

### H-09 — Published settlement population estimates

**Source:** Huffman & Woodborne (2021), *New AMS dates for the Middle Iron Age in the Mapungubwe landscape*, Table 2. DOI: https://doi.org/10.17159/sajs.2021/8980.

Table 2 gives the authors' population estimates for the South African portion of the Shashe–Limpopo valley by facies. The table prints an explicit method: number of sites divided by 50 years, multiplied by 50 people. Its accompanying discussion says the authors assigned 50 people per homestead and a 50-year homestead duration, informed by Middle Iron Age burials at Kgaswe in Botswana, then added capital populations.

| Phase | Interval (AD) | Homesteads | Reported general population | Capital population | Total |
|---|---:|---:|---:|---:|---:|
| Mapungubwe | 1270–1320 | 114 | 5,700 | 5,000 | 10,700 |
| Transitional | 1150–1270 | 143 | 2,979 | 2,500 | 5,479 |
| K2, later subperiod | within 1000–1150 | 153 | 2,550 | 1,500 | 4,050 |
| K2, initial smaller-capital subperiod | first 25 years | same K2 row | 1,275 | 300 | 1,575 |
| Leokwe | 1000–1200 | 63 | 787 | none | 787 |
| Zhizo | 900–1000 | 22 | 550 | 300 | 850 |

The two K2 estimates are separate subintervals: 125 years and an initial 25-year smaller-capital interval. Do not add or average them without retaining their different time supports.

**Data artifact:** [HIST_001_TABLES_2_3_TRANSCRIPTION_V0_1.json](HIST_001_TABLES_2_3_TRANSCRIPTION_V0_1.json) preserves these rows and the method metadata.

**Limit:** these are author-derived estimates, not a census, direct population measurements, or calibrated population trajectories. Their outputs depend on assumed people per homestead, homestead duration, site identification, period boundaries and capital population estimates. They should be used as a published estimate target or prior with sensitivity analysis over the assumptions, not imposed as exact ground truth.

**Sensitivity experiment:** a standalone Rust calculator in [Symtropy PR #3356](https://github.com/Luminous-Dynamics/symtropy/pull/3356) reproduces the published general-population estimates at 50 people per homestead and 50-year homestead use, then sweeps alternative assumptions. It keeps the capital estimates separate. The tests are authored but unexecuted; no improved historical estimate is claimed.

### H-10 — Baobab chronology and severe-drought sequence

**Source:** Huffman & Woodborne (2021), Table 3, *Baobab climatic sequence and severe droughts recognised in the archaeological record*. DOI: https://doi.org/10.17159/sajs.2021/8980.

Table 3 pairs an archaeological `Cal AD` column with a `Baobab cal AD` column. It reports groups VI through XIIIb. Examples include Group IX: archaeological column AD 1300 and baobab-calibrated AD 1310±5; Group X: AD 1350–1400 and baobab AD 1390±10; Group VIII: AD 1200–1250 and baobab values 1208, 1226 and 1256. Some entries are blank in one column.

**Data artifact:** [HIST_001_TABLES_2_3_TRANSCRIPTION_V0_1.json](HIST_001_TABLES_2_3_TRANSCRIPTION_V0_1.json) preserves every row and the column distinction.

**Limit:** this is an event/sequence summary, not the underlying continuous baobab isotope series, raw proxy measurements, or full calibration model. Blank table entries are not zero and do not imply absence of drought. The article argues that Group IX contributed to Mapungubwe's abandonment through rainfall-related agricultural and political effects. That is a published causal interpretation to test against alternatives, not a benchmark ground-truth label.

### H-08 — Khami-period chronology and post-Mapungubwe reoccupation

**Source:** Huffman & Woodborne (2021), *Implications of new AMS dates for the Khami Period in the Mapungubwe Landscape*, South African Journal of Science, 117(7–8).  
DOI: https://doi.org/10.17159/sajs.2021/9438

The article describes post-Mapungubwe reoccupation of the Limpopo Valley first by Sotho communities associated with Icon pottery and then by Kalanga speakers associated with Khami pottery. It places a senior Kalanga chief at Machemma, approximately 60 km south, while petty chiefs administered parts of the valley. The authors describe Sotho and Kalanga occupation as occurring in pulses during higher-rainfall periods.

**Data requirements:** individual AMS sample/context records, laboratory measurements and uncertainties, calibration curve/version, calibrated probability distributions, pottery/ceramic assemblage contexts, site occupation intervals, survey coverage, and the hydroclimate proxy records used to support rainfall interpretations.

**Limit:** this is a published regional interpretation, not a continuous population curve or complete political map. The reported sequence does not prove that every settlement followed the same order, that local populations disappeared between phases, or that rainfall alone determined occupation. The authors overlap with the 2021 Middle Iron Age chronology work; treat both as related evidence until sample-level independence is resolved.

### H-07 — Earlier regional cultural sequences

**Source:** *Zimbabwe Culture before Mapungubwe: New Evidence from Mapela Hill, South-Western Zimbabwe* (2014).  
https://pmc.ncbi.nlm.nih.gov/articles/PMC4215987/

The study presents modelled radiocarbon dates for Mapela Hill and discusses chronology and material sequences that complicate simple origin and diffusion narratives.

**Limit:** site-specific dates and assemblages must remain tied to their own stratigraphic contexts. Do not generalize one excavated sequence to an entire region without an explicit sampling model.

## 3.1 Event ontology: separate chronology from political meaning

A central modelling requirement is to avoid one generic "civilization started/ended" event. The source record contains several different event families that may overlap without being contradictory:

- **Occupation interval:** evidence that a site or area was inhabited/used.
- **Construction interval:** building or modification of a monument/architecture.
- **Material-culture transition:** changes in ceramic, bead, or other assemblage patterns.
- **Regional-centre emergence:** a settlement's growth or role as a centre.
- **Network prominence:** changes in exchange connectivity, production specialization, or redistribution.
- **Political-control/authority shift:** evidence or interpretation that a polity/leadership gained influence, control, or legitimacy.
- **Abandonment/decline:** a change in site use or occupation intensity, distinguished from regional population disappearance.
- **Reoccupation/transformation:** later settlement or cultural practices in the same region, including changed political/cultural identities.

This distinction resolves an important apparent chronology tension. The Great Zimbabwe Bayesian chronology places the start of stone-wall construction around the end of the 12th or beginning of the 13th century and indicates overlap with Mapungubwe. The 2021 Mapungubwe landscape paper proposes that a drought around AD 1310 contributed to Mapungubwe's abandonment and that Great Zimbabwe subsequently became a new regional centre of power by gaining control of the gold belt. Those are not necessarily competing claims if they refer to different event types: construction/occupation may precede a later shift in regional power or network control. The model must not collapse them into a single "Great Zimbabwe founded" date.

Likewise, Mapungubwe's abandonment does not imply that the Limpopo Valley became empty: the later Sotho/Icon and Kalanga/Khami reoccupation evidence belongs to subsequent regional transformation.

The benchmark should explicitly test whether typed event representations reconcile multiple source chronologies better than a single linear succession variable, while retaining unresolved disputes and source-specific interpretations.

## 4. Competing explanatory models

CivBench should compare alternative mechanisms rather than fit one preferred story:

- **Environmental constraint model:** rainfall, floodplain/vlei productivity, drought frequency, and land-use constraints affect agricultural output and settlement viability.
- **Exchange-network model:** trade routes, imported goods, export demand, and network position affect settlement growth, specialization and elite access.
- **Political-spatial reorganization model:** changes in institutions, sacred leadership, settlement organization or elite-commoner access affect where activity concentrates.
- **Polycentric interaction model:** multiple centres exchange, compete, specialize and overlap rather than forming a single succession chain.
- **Multi-causal model:** environmental, demographic, political and trade mechanisms interact.

Each model must identify its observable implications and the evidence that could falsify it. Model comparison is not a vote on a single historical truth; several processes may have operated together.

## 5. Minimal computational representation

Begin with a pattern-oriented regional model, not a full synthetic civilization.

### State layers

- **Landscape:** river systems, floodplain/vlei zones, elevation, soils, rainfall proxy coverage and spatial uncertainty.
- **Settlements:** site identity, occupation intervals, built features, land-use context and archaeological visibility.
- **Populations:** broad population/occupation hypotheses with uncertainty; no unsupported exact counts.
- **Production:** agricultural and pastoral resource flows only where source data justify them.
- **Exchange:** material flows, origin hypotheses, routes, ports/markets as uncertain network edges, and down-the-line exchange.
- **Social differentiation:** observable spatial/material proxies, separated from inferred class or institutional categories.
- **External communities:** forager and farming groups represented without presuming assimilation, extinction, or a single identity.

### Dynamics

- rainfall and agricultural production;
- storage, shortage, and settlement movement;
- trade-link formation, disruption and redistribution;
- technological and craft-practice diffusion;
- changing settlement organization;
- sampling and preservation processes that determine which archaeological traces remain visible.

The model must distinguish historical state from the process by which evidence is produced and recovered. Archaeological observation is a filtered sample of past activity, not a complete recording of it.

## 6. Initial CivBench tasks

### HIST-001A — Chronology reconstruction

Given a set of dated contexts, reconstruct phase intervals and uncertainty while holding selected site dates out. Compare with expert chronology models. Preserve posterior distributions and alternative calibration mappings.

### HIST-001B — Settlement pattern transfer

Fit the model to selected sites and test whether it predicts broad occupation/relocation patterns at held-out sites or phases. Compare environmental-only, trade-only, political-spatial, and multi-causal baselines.

### HIST-001C — Exchange-network inference

Predict likely connectivity or provenance classes for held-out assemblages using only information available at the model's forecast origin. Avoid inferring direct contact from shared artifact styles alone.

### HIST-001D — Polycentric chronology

Test whether a model that permits overlapping centres reproduces the documented Mapungubwe/Great Zimbabwe timing better than a forced linear-successor model. This is a model-comparison task, not a claim that one chronology explains all regional political relations.

### HIST-001F — Post-abandonment reoccupation and regional continuity

Test whether models can represent reoccupation pulses, multiple communities, and political authority distributed across sites after the Mapungubwe sequence. Compare:
- a naive empty-region/collapse model;
- a continuous local-occupation model with changing archaeological visibility;
- a multi-community model with Sotho/Icon and Kalanga/Khami phases and uncertain site-level chronologies;
- a rainfall-constrained model with explicit alternative political, trade and land-use mechanisms.

The evaluation must preserve source uncertainty and not convert an article's regional interpretation into a mandatory world-state transition. Use held-out site/phase contexts and report whether the model can distinguish demographic change from changing archaeological visibility.

### HIST-001E — Climate causation stress test

Test whether climate-only models are insufficient or sufficient to reproduce a preregistered set of settlement/production patterns, then compare multi-causal alternatives. Do not call a correlation a causal effect.

## 7. Validation and uncertainty

- Use site- and time-forward holdouts; avoid random splits that leak nearby contexts or shared source families.
- Use leave-one-site-out or leave-one-assemblage-out transfer where sample sizes permit.
- Keep chronology calibration uncertainty, archaeological sampling bias, site-preservation bias, survey intensity and source interpretation uncertainty distinct.
- Compare against no-change, historical-frequency, environmental-only, trade-only, and other parsimonious baselines.
- Use interval/probabilistic scoring where outcomes are uncertain; report coverage and calibration.
- Report negative and null findings, including model failures to distinguish rival explanations.
- Do not label a pattern-oriented simulation a validated causal model unless an appropriate causal identification strategy exists.
- Every data row must point to the source, table/figure/context, extraction version and snapshot digest.

## 8. Relationship to the wider programme

- **Mycelix:** preserve source identity, archaeological context, source families, uncertain chronologies, conflicting interpretations, community stewardship and correction history.
- **Symthaea:** test cultural transmission, local learning, knowledge diffusion, social learning and belief updating as mechanisms, not as universal historical explanations.
- **Symtropy:** execute alternative mechanism models with deterministic replay, physical/resource constraints and explicit sampling processes.
- **Sol-Atlas:** visualize sites, settlement areas, trade-network hypotheses, landscape proxies and uncertainty without implying precise borders or known routes where evidence is approximate.

## 9. Research claim ceiling

This case can support:
- source-grounded descriptions of selected archaeological sequences;
- model comparisons against selected spatial, material and chronological patterns;
- uncertainty-aware hypotheses about interacting environmental, social and exchange mechanisms.

It cannot, without additional evidence, support:
- one definitive cause of Mapungubwe's abandonment;
- exact population totals or annual output curves;
- a single linear transition from Mapungubwe to Great Zimbabwe;
- universal laws of civilizational emergence or collapse;
- claims about the emergence of extraterrestrial civilizations.

**The purpose of HIST-001 is to make models accountable to difficult historical evidence—not to make the past look more complete or certain than it is.**
