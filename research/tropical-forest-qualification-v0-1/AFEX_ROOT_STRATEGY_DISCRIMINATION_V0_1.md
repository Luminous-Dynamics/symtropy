# AFEX root strategy mechanism-discrimination protocol v0.1

**Issue:** [#5445 — AFEX-ROOT-STRATEGY-001](https://github.com/Luminous-Dynamics/symtropy/issues/5445)  
**Related:** [#5431 root turnover](https://github.com/Luminous-Dynamics/symtropy/issues/5431), [#5443 ratio estimand](https://github.com/Luminous-Dynamics/symtropy/issues/5443), [#5408 AFEX intervention](https://github.com/Luminous-Dynamics/symtropy/issues/5408), [#5198 model-set adequacy](https://github.com/Luminous-Dynamics/symtropy/issues/5198), [#5223 decision value](https://github.com/Luminous-Dynamics/symtropy/issues/5223)  
**Status:** source-grounded hypothesis matrix; child files not ingested; no mediation estimated

## 1. Why this is the next paradigm shift

The 2021 AFEX trait study and 2026 AFEX turnover study expose a potentially important process sequence. One-year cation addition increased fine-root diameter and AM colonisation, while phosphorus increased root productivity/diameter and decreased potential root phosphatase activity. The 2026 study then found lower year-two turnover under cations and a counteraction of the P-related turnover increase by cations. The authors propose that AM associations could help explain longer root lifespan, but explicitly state this needs future investigation.

The field should not compress this into a single “conservative–acquisitive” score. Trees may combine direct root exploration, biochemical P mining and mycorrhizal outsourcing in different proportions and under different nutrient combinations. The benchmark should test which combination predicts independent outcomes.

## 2. What is actually measured

| Observation stream | Support | Scientific meaning | What it is not |
|---|---|---|---|
| AM colonisation | Feb 2018; 0–10 cm; five within-plot points averaged to plot | % root length colonised; hyphae, arbuscules, vesicles | Fungal transfer flux, carbon cost or 0–30 cm measure |
| Morphology | Feb 2018; 0–10 and 10–30 cm; plant-community plot means | Root length/area/volume, diameter, SRL, SRA, RTD | Direct nutrient uptake flux |
| Root phosphatase | Feb 2018; 0–10 and 10–30 cm; root-tip potential assay | Potential acid phosphomonoesterase activity per dry mass | In-situ phosphorus flux |
| Root nutrient concentration | Four-interval composite Nov 2017–Aug 2018; by plot and depth | Tissue C, N, P, K, Ca, Mg, Fe, Zn, Mn concentrations | Uptake rate or total elemental accumulation without compatible biomass |
| Productivity / stock / turnover | Year 1 and 2; published turnover target 0–30 cm | Realized plot-level process and state response | Same support/time as every 2018 proxy |

The precise source record and observation operators are in [DATASET_CANDIDATE_REGISTER_V0_1.json](DATASET_CANDIDATE_REGISTER_V0_1.json). All four trait datasets state Open Government Licence, but each still needs file-level archive and plot-key admission.

## 3. Competing mechanism hypotheses

The machine-readable matrix [cases/afex-root-strategy-hypotheses-v0.1.json](cases/afex-root-strategy-hypotheses-v0.1.json) records five competing explanations:

1. Direct foraging / biochemical mining under P: higher productivity and root diameter, lower phosphatase activity and changed tissue P.
2. Mycorrhizal outsourcing under cations: higher AM colonisation and root diameter, with a possible later association with longer root lifespan.
3. Combined strategy: direct and fungal pathways operate simultaneously and interact.
4. No mediation: traits respond to treatment but do not causally mediate later turnover.
5. Omitted mechanism: soil pH, nutrient mobility, water, fungal community, species turnover or another shared cause explains the response.

The null and omitted-mechanism alternatives are mandatory. A joint-trait model must not automatically win simply because it contains more variables.

## 4. Temporal and support logic

The 2018 trait observations follow the start of fertilization and precede the year-two turnover interval. This is useful for a prospective predictive question, but not sufficient for causal mediation:
- There is no admitted pre-treatment baseline for these traits in the current source package.
- AM colonisation is 0–10 cm only; the 2026 turnover outcome is 0–30 cm.
- Morphology and phosphatase have depth-separated values.
- Tissue nutrient values are temporally bulked over four intervals.
- The stock data used in the 2026 turnover calculation are request-only.

The next data request should ask the corresponding authors for the plot-keyed, depth-resolved stock data used in 2026 and, if available, raw trait-level observations and uncertainty. A depth-matched 0–10 cm turnover target could then be considered separately from the published 0–30 cm target. Do not infer it exists until admitted.

## 5. Matched mechanism tournament

Compare:
- treatment-only factorial baseline;
- direct root-morphology/phosphatase model;
- AM-colonisation model;
- combined multi-trait model;
- no-mediation model;
- model-set alternative with an omitted common cause.

Primary endpoint: plot-level, year-two root turnover on identical admitted support, with root stock and productivity evaluated separately. Secondary endpoints: each trait's intention-to-treat treatment contrast.

Evaluation rules:
- preserve the randomized plot assignment and block structure;
- use intention-to-treat assignment as the primary treatment estimand;
- hold out entire plots/blocks, never points or roots independently;
- compare models with identical data, time support and compute budget;
- report held-out proper scores, treatment-effect error, calibration, worst-block errors and trait ablations;
- test whether trait additions improve out-of-sample turnover prediction over treatment-only baseline;
- quantify the value of a new AMF, morphology, phosphatase, tissue nutrient or soil-pH measurement using the existing decision-value and active-sensing owners;
- do not promote association to causal mediation without explicit confounding, temporal and interference assumptions.

## 5.1 External model tournament: three-pathway nutrient acquisition

The next comparator should represent nutrient acquisition as distinct pathways, not as a binary AM-colonised/not-colonised trait. The 2026 ELMv2-SPRUCE modification (ELM-MYCI) explicitly separates:
1. direct inorganic N/P uptake by uncolonised fine roots;
2. mycorrhizal uptake of inorganic N/P;
3. mycorrhizal acquisition of organic N/P.

It also constrains fungal acquisition through host non-structural carbohydrate availability / carbon allocation. The study evaluated a boreal peatland ecosystem with spruce, tamarack and shrubs, not a tropical Amazon AFEX forest. Its architecture is an external comparator, not a matched model result. The study adds 37 parameters and identifies a need for direct constraints on uptake rates and half-saturation points, which is a useful warning against pathway proliferation without identifiability. Paper: https://gmd.copernicus.org/articles/19/5827/2026/; source code release: https://doi.org/10.5281/zenodo.17582789.

Myco-CORPSE is a second, different comparator: it extends CORPSE to include mycorrhizal effects on soil carbon and nitrogen decomposition and has been applied to investigate fungal carbon allocation across eastern US forest inventory plots. It informs belowground C/N coupling and fungal carbon costs, but its ectomycorrhizal temperate-forest domain is not directly matched to AFEX's Central Amazon AM colonisation data:
- https://doi.org/10.1016/j.soilbio.2023.109073
- https://doi.org/10.1029/2025JG009198

A 2026 trait study using in-situ 15N tracers across 63 subtropical tree species reports R²=0.37 for a mycorrhizal-type hierarchical trait model versus R²=0.19 for a root-only model. It supports testing mycorrhizal type as a modifier of trait–function relationships, but its species-level tracer task is different from AFEX plot-level turnover: https://doi.org/10.1111/pce.70522.

The 2026 root-economics-space review warns that the fast–slow and direct-root–outsourcing axes vary with mycorrhizal guild and nutrient context. Do not collapse AM, ECM, ericoid, dual and non-mycorrhizal strategies into a universal scalar: https://doi.org/10.1016/j.tree.2026.02.003.

The eight-model tournament and its machine-readable claim boundaries are recorded in:
- [AFEX_ROOT_PATHWAY_MODEL_TOURNAMENT_V0_1.md](AFEX_ROOT_PATHWAY_MODEL_TOURNAMENT_V0_1.md);
- [comparator matrix](cases/afex-root-pathway-model-tournament-v0.1.json);
- [Rust qualification kernel](validator/src/afex_root_pathway_tournament_kernel.rs).

The three-pathway model's organic route is currently unidentifiable from admitted AFEX records. Fungal carbon allocation, fungal nutrient-transfer rates, available organic N/P and species-resolved mycorrhizal types are not established in the current crosswalk. The model register therefore includes these as explicit data requirements and refuses to promote the comparator to an AFEX-calibrated model.

## 5.2 Pathway observability gate

Before fitting any multi-pathway model, require an exact identifiability report for the actual observation operator. [#5449](https://github.com/Luminous-Dynamics/symtropy/issues/5449) shows that AM colonisation fraction is a covariate, not a source-specific uptake measurement. In a simplified model with four unknowns (direct-root inorganic, mycorrhizal inorganic, mycorrhizal organic and host-to-fungal carbon cost), colonisation plus hypothetical total uptake across varied fractions identifies at most two parameter combinations. Mycorrhizal inorganic and organic terms are aliased; carbon cost has no observation equation.

A source-specific fungal nutrient tracer can break the inorganic/organic alias. A separate host-to-fungal carbon allocation measurement is needed to identify fungal carbon cost. This is a measurement-design result, not evidence that the proposed assays will be precise or feasible in AFEX.

See [AFEX_PATHWAY_IDENTIFIABILITY_V0_1.md](AFEX_PATHWAY_IDENTIFIABILITY_V0_1.md), its [four-case synthetic fixture](cases/afex-root-pathway-identifiability-synthetic-v0.1.json), and [exact-rank validator](validator/src/afex_root_pathway_identifiability_kernel.rs).

## 6. Proposed benchmark targets

- 100% of unsupported depth/time joins refused.
- 100% of within-plot subsamples retained as nested observations, not treatment replicates.
- Every mechanism model must beat or explain the treatment-only baseline on held-out blocks before claiming additional explanatory value.
- The combined-strategy model must outperform both single-pathway models on prespecified held-out prediction targets, or the evidence should prefer the simpler model.
- Report marginal and interaction responses for N, P and cations separately.
- Explicitly quantify the benefit of requesting new depth-resolved stock measurements and repeated trait campaigns.
- Preserve model-set inadequacy and abstention where trait support cannot identify a mechanism.

These are proposed targets, not achieved results.

## 7. Claim ceiling

The 2021 and 2026 papers provide published evidence of randomized treatment responses and support the mechanism hypotheses. The source records establish measurement operators and sampling support. No raw child dataset has been ingested or joined; no AFEX mediator analysis, causal mediation, mechanism superiority or SOTA claim is made.
