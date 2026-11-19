# Physarum traveling-network benchmark

*Physarum polycephalum* is an Amoebozoan plasmodial slime mould. This benchmark treats the plasmodium as a **moving organism with a continuously remodelled internal transport network**. It is not a fungus, and its tubular network is not a fungal hyphal graph.

## Paradigm shift: three coupled but independently measured layers

1. **Outer morphology and locomotion:** front advance/retraction, branch formation, migration, relocation and search.
2. **Internal network architecture:** tubular connectivity, width proxies, edge persistence, network cost and redistribution of routes.
3. **Contractile and transport dynamics:** oscillatory contraction, phase, wave direction, thickness variation and cytoplasmic flow/transport where directly measured.

A graph path does not prove flux. Image brightness, tube width or topology must not be silently promoted to measured nutrient delivery or cytoplasmic flow. The model must keep inferred quantities and directly observed quantities separate.

## First observation anchor: 2026 OME-Zarr release

Harrington & Harrington (2026), Zenodo [10.5281/zenodo.22868368](https://doi.org/10.5281/zenodo.22868368), provides 24 dish series from four six-dish experiments:

- 51,036 dish frames from 8,506 scans; 132.93 GB total.
- 2- and 5-minute scan intervals, with 2.5–6 day experiment spans.
- OME-NGFF 0.5 / Zarr v3, RGB uint8, axes t,c,y,x.
- Exact per-frame UTC nanosecond timestamps, dish-support metadata and per-frame dish-circle fits.
- Per-frame file manifests, checksums, QC notes, and SHA256SUMS.
- Public record is CC BY 4.0.
- The public release intentionally truncates two blown-out final frames from experiment 002 and a suspected-contamination interval from experiment 009. These must remain explicit, not imputed as normal biological progression.

The images are mirrored bottom-view scans and not registered. Use the documented physical pixel pitch of 0.08467 mm and per-frame dish fits. Do not use the incorrect scale embedded in image metadata. The dataset is raw pixels only; it does not provide segmentation or derived biology measurements.

Start with a modest, rights-verified subset of dish archives rather than trying to ingest all 132.93 GB at once. Keep the source manifest, checksums, exact timestamps and truncation/QC flags with every derivative. The full record is not declared admitted merely because its landing page is open.

## Contractility and ring-confined sources

Saiseau, Busson & Durand (2026), Dryad Part 1 [10.5061/dryad.2z34tmq08](https://doi.org/10.5061/dryad.2z34tmq08) and Part 2 [10.5061/dryad.v15dv4291](https://doi.org/10.5061/dryad.v15dv4291), provide ring-confined plasmodial recordings. Part 2 describes 47 ring experiments from 16 specimens, with typical 46-second sampling and recordings up to 412 hours. The release includes segmentation masks, ring geometry, per-sector thickness proxies, phase/amplitude/drift decompositions and analysis scripts. Some raw specimen files are sampled at 4 seconds; do not confuse this with the typical cadence of the processed ring experiment series.

The retrieved Dryad record did not expose a verifiable license field. These data remain NOT_ADMITTED pending exact rights, source-file inventory, checksums and attribution review.

## Model comparators

- **Tero adaptive-conductivity model (2007):** [10.1016/j.jtbi.2006.07.015](https://doi.org/10.1016/j.jtbi.2006.07.015). Flux-reinforced tube conductance; the original authors noted that empirical evidence for the proposed tube-thickening feedback was qualitative.
- **Tero network-design study (2010):** [10.1126/science.1177894](https://doi.org/10.1126/science.1177894). Cost, transport efficiency and fault tolerance; network similarity is not evidence of algorithm equivalence.
- **Bottom-up robust multi-agent model (2025):** [10.1098/rsif.2024.0701](https://doi.org/10.1098/rsif.2024.0701). A hierarchy of multi-agent models for foraging/network formation.
- **Traveling-network model (2026 preprint):** raw videos [10.5281/zenodo.21908143](https://doi.org/10.5281/zenodo.21908143) and code [CiraLab/AvoidantTravelingNetworks](https://github.com/CiraLab/AvoidantTravelingNetworks). The preprint reports quantitative matching of path length, relocation time and search efficiency from local remodelling measurements without further fitting. It is a direct comparator, not a ground-truth oracle.

## Benchmark tournament

Keep the following model classes distinct:

1. Empirical front / network statistics (non-parametric reference).
2. Static graph controls: Dijkstra, breadth-first search and minimum spanning tree. These are optimization baselines, not organism models.
3. Tero adaptive-conductivity / flux-reinforcement model.
4. 2025 bottom-up multi-agent model hierarchy.
5. 2026 traveling-network model.
6. Geometry-only growth/retraction model without transport feedback.
7. Coupled traveling morphology plus internal contractile/transport dynamics.

Use identical source splits, dish geometry, food layouts, initialization, observation times, pixel calibration and computational budgets. Report model-specific supported observables; do not force all contenders to predict quantities they were not designed to represent.

## Metrics

### Locomotion and morphology
- Front-position MAE/RMSE and spatial-distribution error.
- Front speed, branch formation/retraction and merge/split event error.
- Path length, relocation time, exploration coverage and food-arrival time.

### Network architecture
- Connected components, branch degree and cycle rank.
- Edge persistence, tube-width proxy distribution and pruning.
- Network cost/biomass investment proxy only when measured/calibrated.
- Connectivity, redundancy and fault-tolerance under matched synthetic edge failures (labelled simulation, not experimental biology).

### Contractility and transport
- Contraction period, amplitude, phase and propagation direction.
- Spatiotemporal phase coherence and phase drift.
- Thickness-derived transport proxies and uncertainty.
- Cytoplasmic flux or nutrient transport only when supported by direct flow or tracer measurement.

### Generalization and cost
- Prediction-interval coverage and worst-experiment performance.
- Error/cost Pareto frontiers across image throughput, compute and memory.
- Human curation burden and image-analysis failure rates.

No single “shortest path score” may override poor prediction of relocation time, cost, resilience, or dynamic transport behaviour.

## Data splitting and uncertainty

- Split by complete experiment and preserve dish-within-experiment grouping. Six dishes from one experimental run are not automatically six independent experiments.
- Never distribute adjacent frames of one dish between training and confirmatory test folds.
- Keep the released truncations, suspected contamination interval and acquisition cadence as part of the observation model.
- Resample at the highest defensible independent unit (experiment/specimen), not individual pixels, skeleton edges or frames.
- Report image-derived quantities with segmentation and registration uncertainty.
- Distinguish directly observed, algorithm-inferred and model-latent flux or mechanism claims.

## Claim ceiling

A successful benchmark can qualify a particular model for plasmodial migration and network remodelling on the declared image/contractility conditions. It does not prove that Physarum implements Dijkstra, demonstrate general intelligence, establish fungal physiology, or validate ecosystem-scale nutrient transport.