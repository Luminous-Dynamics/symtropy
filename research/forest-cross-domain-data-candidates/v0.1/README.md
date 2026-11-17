# Forest Cross-Domain Data Candidates v0.1

**Parent:** Symtropy [#5173 — Tropical Forest Ecological Qualification](https://github.com/Luminous-Dynamics/symtropy/issues/5173)

This is a metadata-only addendum to the existing dataset candidate register. It introduces four complementary 2026 source families for later T0 observation admission; it does not create new observation, spatial, provenance, forecast, or rights systems.

## Candidate roles

| Source | Primary role | Do not use it as |
|---|---|---|
| TRY v7 | Plant functional-trait distributions, taxonomy and trait transfer | A census, biomass stock, ecosystem flux or demographic time series |
| BioTIME 2.0 | Repeated species/community abundance and composition | A forest-only dataset or causal intervention panel |
| FLUXNET Data Shuttle | Ecosystem carbon, water and energy fluxes at tower support | A closed thermodynamic control volume by itself |
| NEON 2026 products | Repeated plant/organism observations, structure, phenology and community data; NEON4Cast forecast scoring comparator | Direct tropical-forest calibration; its terrestrial sites are in the United States |

The machine-readable record is `sources.json`; its structure is described in `schema.json`.

## Shared admission contract

Before any record can enter model fitting, calibration or testing:

1. Pin the exact provider collection, release, query, site/product, source asset and SHA-256.
2. Verify the exact dataset license, contributing-author terms, intended use and derivative permissions. A website landing page or paper license is not a substitute for the source asset's rights.
3. Preserve taxon/variable identity, units, sampling method, support geometry, CRS, acquisition/census time, QA flags, detection limits, gap-fill state and measurement uncertainty.
4. Separate observation, derived estimate, model output, intervention and decision records.
5. Build dependence-aware splits by site, plot, campaign, time series and forecast origin; never random-split dependent records.
6. Preserve no-acquisition, unobserved, cloud/gap-filled, below-detection, invalid-QA and qualified absence as distinct states.
7. Compose with the existing observation-admission, support, consilience/dependence, forecast and source-rights owners under #5173/#5176; do not create a second validator or GIS.

## Recommended benchmark sequence

**Stage A — source-faithful baseline reproduction.** Use the smallest licensed subset of each source; publish data and code manifests, source checksums, split IDs and transformations. Keep missing sources NOT_RUN.

**Stage B — held-out forecasting.** Compare persistence/climatology and a simple statistical baseline first, then a process-based baseline (for example FATES/LPJ-GUESS where reproducible), then Symtropy. Score by site, region, season and forecast horizon.

- Continuous outcomes: CRPS, WIS, MAE/RMSE/bias, paired CRPSS/WIS skill, interval coverage and width.
- Counts/binary outcomes: Brier/log score, reliability, event precision/recall where thresholded.
- Phenology: event-date error and calibrated date intervals.
- Fluxes: variable-wise bias/CRPS plus explicit boundary residual only where all terms and storage changes are measured.
- Selective prediction: full risk–coverage curve and realized performance at each coverage level.
- Transfer: held-out sites, biomes, regions, years and source protocols; disclose where a task is out of domain.

**Stage C — process and intervention tests.** Evaluate stocks and fluxes separately; test interventions only when the evidence design identifies the causal question. A successful forecast is not a causal estimate.

## Thermodynamics boundary

FLUXNET's carbon, water and energy exchange products can support ecosystem-scale process accounting. They do not automatically provide a complete open-system energy boundary: radiation, sensible and latent heat, ground heat, storage and other terms need source- and site-specific inspection. Resource use, gross primary productivity, respiration, model dissipation and information entropy are not entropy-production measurements. No entropy, exergy, free-energy or moral-value inference is licensed by this candidate list.

## Rights and access boundary

- TRY reports that v5+ data are CC BY; verify the exact v7 extract, contributor terms, citation and attribution before use.
- BioTIME 2.0 has a database-level CC-BY grant; each underlying study has its own contributor-selected licence, which must be reviewed for the selected series.
- FLUXNET Shuttle states CC-BY subject to its harmonized data-use policy; preserve site-specific DOI and acknowledgement conditions.
- NEON's stable releases are fixed/citable product snapshots; RELEASE-2026 OS/AOP data end at 2024-12, while IS extends through 2025-06. Pin product DOI and release; keep provisional data separate. API tokens are required for product downloads or intensive use.

## Status

All four entries are `METADATA_ONLY_NOT_ADMITTED`; `downloaded=false`; `data_ingested=false`. No external raw data or credentials are stored in this package. These records are candidates for T0 admission review only.
