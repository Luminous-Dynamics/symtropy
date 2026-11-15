# AFEX root pathway model tournament v0.1

**Parent:** [#5445 — AFEX-ROOT-STRATEGY-001](https://github.com/Luminous-Dynamics/symtropy/issues/5445)  
**Related:** [#5431 root turnover](https://github.com/Luminous-Dynamics/symtropy/issues/5431), [#5443 ratio estimands](https://github.com/Luminous-Dynamics/symtropy/issues/5443), [#5408 AFEX intervention](https://github.com/Luminous-Dynamics/symtropy/issues/5408)  
**Date:** 15 November 2026  
**Status:** comparator specification; no candidate model calibrated or run on AFEX

## 1. What industry SOTA now requires

A root strategy model cannot stop at “root traits versus mycorrhizal colonisation”. The 2026 ELMv2-SPRUCE modification (ELM-MYCI) explicitly separates three nutrient acquisition routes:
1. direct inorganic N/P uptake by uncolonised fine roots;
2. mycorrhizal acquisition of inorganic N/P;
3. mycorrhizal acquisition from organic nutrient sources.

It also makes fungal acquisition dependent on host non-structural carbohydrate availability and constrains fungal acquisition by carbon allocation. It was evaluated in a boreal peatland experiment with spruce, tamarack and shrubs—not in an Amazon AFEX forest. Its 37 new parameters and acknowledged need for direct constraints are an especially useful warning: added mechanistic detail can make a model more realistic while making it harder to identify. Paper: https://gmd.copernicus.org/articles/19/5827/2026/; code/data release: https://doi.org/10.5281/zenodo.17582789.

Myco-CORPSE provides a different comparator. It brings mycorrhizal processes into soil carbon/nitrogen decomposition, and a later application uses the model across more than 1,800 eastern US forest inventory plots to investigate fungal carbon costs. It is useful for the root–fungus–soil C/N coupling boundary, but its ectomycorrhizal temperate-forest domain is not a direct AFEX AM-root score baseline:
- https://doi.org/10.1016/j.soilbio.2023.109073
- https://doi.org/10.1029/2025JG009198

The empirical trait-model frontier also moved in 2026. A ^15N uptake study across 63 subtropical tree species reported R²=0.37 for a mycorrhizal-type hierarchical trait model versus R²=0.19 for a root-only model. This is evidence that symbiotic type can change trait–function relations, not a number to compare directly with AFEX plot-level turnover:
- https://doi.org/10.1111/pce.70522

Finally, the 2026 refinement of the root economics space argues that the fast–slow and do-it-yourself–outsourcing axes are context-dependent and that AM, ECM, ericoid, dual and non-mycorrhizal strategies should not be forced into one universal axis:
- https://doi.org/10.1016/j.tree.2026.02.003

## 2. The model tournament

The machine-readable matrix is [cases/afex-root-pathway-model-tournament-v0.1.json](cases/afex-root-pathway-model-tournament-v0.1.json). It defines eight structures, including:
- treatment-only factorial baseline;
- direct inorganic root uptake;
- root + mycorrhizal inorganic uptake;
- three-pathway ELM-MYCI-like uptake;
- mycorrhizal carbon-cost / soil C-N coupling;
- mycorrhizal-type hierarchical trait model;
- no-mediation model;
- omitted-mechanism open-world challenge.

This is a **model structure tournament**, not a declaration that every pathway can be estimated from existing AFEX records. The direct-root and AM inorganic pathways need soil nutrient availability and root uptake observations. The organic pathway needs available organic N/P and evidence of fungal access. Carbon-cost coupling needs host-to-fungus allocation, fungal biomass/turnover or a defensible external constraint. Species-hierarchical trait models need species-level mycorrhizal type and species-resolved responses. Those quantities are not all present in the current admitted crosswalk.

## 3. What is identifiable with current AFEX evidence?

The current AFEX records can support randomized plot-level treatment contrasts on several traits after their raw files and plot IDs are admitted:
- AM colonisation percentage in 0–10 cm;
- morphology and phosphatase by 0–10 and 10–30 cm;
- root nutrient concentrations bulked across four intervals;
- root productivity from ingrowth cores;
- later turnover only if plot-paired stock denominators are obtained.

They do **not** yet identify:
- fungal N/P transfer flux;
- direct-root versus fungal pathway fractions;
- fungal carbon cost;
- organic nutrient mining rate;
- species-level mycorrhizal strategy;
- a causal AMF mediation effect on whole-profile turnover.

This distinction should be explicit in every receipt. A proxy trait can establish a response in that proxy; it cannot silently become a nutrient flux or a causal pathway.

## 4. Matched evaluation design

All candidate models must use identical plot and temporal support, source rows, treatment assignment, forecast issue time, held-out plot/block split and compute budget. Keep:
- as-published AFEX model reproduction;
- preregistered full-factorial/randomization robustness analysis;
- out-of-block forecast/model comparison;
- synthetic open-world and measurement-corruption challenges

as four separate evidence products.

Primary process targets: root productivity, stock, turnover and the measured trait families separately. A richer pathway model is retained only if it improves a prespecified proper predictive score or decision loss against treatment-only, root-only and no-mediation baselines on held-out plot units, with uncertainty on the paired improvement.

Do not rank models on R² from different forests or different response variables. Report proper score, treatment-effect error, calibration, worst-block failure, runtime, parameter count, identifiability and observation/assay cost.

## 5. Proposed benchmark targets

- 100% refusal of unsupported depth, date, plot, species and treatment joins.
- No tree/core/point subsamples counted as additional treatment replicates.
- Every pathway decomposition accompanied by an identifiability report.
- A mycorrhizal or organic pathway must improve held-out predictions, not only in-sample fit.
- The open-world test must expose a shared omitted mechanism rather than selecting a more complex but equally wrong candidate.
- Added model complexity reported against incremental score gain, uncertainty reduction and measurement cost.
- Explicit abstention when the data do not distinguish direct from fungal acquisition.

These are proposed targets, not achieved outcomes.

## 6. Claim ceiling

The ELM-MYCI, Myco-CORPSE and 2026 mycorrhizal-trait studies are external architectural/methodological comparators. They are not matched AFEX benchmark results. No pathway model has been fit, no fungal carbon cost or nutrient transfer is estimated, and no AFEX raw dataset has been ingested.

## 7. Structural identifiability before fitting

The multi-pathway model is not automatically identifiable merely because each pathway has a biological interpretation. The follow-on [#5449 pathway observability benchmark](https://github.com/Luminous-Dynamics/symtropy/issues/5449) demonstrates the exact limit in a simplified shared-rate model.

With unknown direct-root inorganic, mycorrhizal inorganic, mycorrhizal organic and host-to-fungal carbon-cost rates, hypothetical total nutrient uptake across three colonisation fractions has exact rank 2 for four unknowns. The fungal inorganic and organic columns are identical; carbon cost is unobserved. A source-specific fungal tracer raises rank to 3, and a separate host-to-fungal carbon-allocation observation raises it to 4.

The current AFEX AM colonisation, morphology, phosphatase and tissue-concentration records provide no direct source-specific flux equation for those pathway rates. The exact-rank fixture and Rust validator make this limitation executable:
- [Identifiability protocol](AFEX_PATHWAY_IDENTIFIABILITY_V0_1.md);
- [synthetic matrix corpus](cases/afex-root-pathway-identifiability-synthetic-v0.1.json);
- [exact-rank Rust kernel](validator/src/afex_root_pathway_identifiability_kernel.rs).

This rank result is structural under the declared operator, not proof of biological validity or practical precision.
