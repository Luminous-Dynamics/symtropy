# 2026 ICOS/NEON EBC Reproduction Manifest v0.1

**Parent:** Symtropy [#5955 — reproduce the 84-site ICOS/NEON benchmark](https://github.com/Luminous-Dynamics/symtropy/issues/5955)  
**Related implementation:** [#5946](https://github.com/Luminous-Dynamics/symtropy/issues/5946) and [PR #5950](https://github.com/Luminous-Dynamics/symtropy/pull/5950)  
**Related uncertainty:** [#5953](https://github.com/Luminous-Dynamics/symtropy/issues/5953)

The machine-readable source manifest is `manifest.json`; `schema.json` defines its closed schema.

## Exact data/software lineage

- Paper: Nicolini et al., Global Change Biology, first published 2026-05-04, DOI `10.1111/gcb.70892`.
- Processed data: Zenodo `10.5281/zenodo.19608436`, v1.0.0, published 2026-04-16. It reports 84 sites (38 ICOS, 46 NEON), non-gap-filled and gap-filled half-hourly data, and intermediate biomass heat storage, photosynthetic uptake and corrected radiation. The archive is 273.1 MB, with published MD5 `c6252cecac4d46d856196f0a565026e5`; the README is 9.8 kB, with MD5 `e072368b147941bbf28fea85f9d71114`.
- Original R analysis code: Zenodo `10.5281/zenodo.19631582` (EBC_code).
- Raw ICOS input source: `10.18160/S6HM-CP8Q`.
- Raw NEON input source: `10.48443/r7zp-y487`.

The processed dataset record states that raw inputs are not redistributed in the processed archive. Treat these as distinct records and preserve each source's own metadata, rights, processing version and citation.

## Rights and retrieval

At registry creation, the accessible metadata did not expose a verifiable per-file license for the processed data or exact code record. The records are not considered admitted until the exact record metadata, README, file-level licensing/attribution and source terms are inspected. Do not infer license from the article's open access status or Zenodo's default license.

No data or code archive was downloaded or ingested to create this manifest. Published MD5 values are reference checksums only; SHA-256 values remain null until local retrieval and independent calculation.

## Reproduction targets

The published summary targets are rounded values (AE1 mean RMA ~0.70; AE2 ~0.73; AE6 ~0.85; AE6 plus u* filtering ~16% overall improvement, IQR 11–20%, n=79). They guide reproduction but are not exact row-level fixtures. Exact reproduction requires the original R pipeline, site-level data, source crosswalk, units/sign/time definitions, filtering and aggregation logic.

## Important inference boundary

Half-hourly RMA regression and integrated EBR are different estimands. EBR uses ΣTE/ΣAE over a defined continuous period. QC and u* filtering change the sample population; report sample counts and attrition. The article explicitly says closure is a diagnostic constraint, not an optimization target; never rescale or allocate residuals solely to force a value of 1.

## Claim ceiling

This package is a source manifest only. It does not establish rights, checksum verification, data ingestion, reproduction, field calibration, total-system energy closure or model superiority.
