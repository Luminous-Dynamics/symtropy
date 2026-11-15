# ForestPaths D-BEN Admission and Crosswalk v0.1

**Empirical reproduction owner:** [#5352 — FORESTPATHS-EMPIRICAL-REPRO-001](https://github.com/Luminous-Dynamics/symtropy/issues/5352)  
**Crosswalk owner:** [#5357 — FORESTPATHS-DBEN-CROSSWALK-001](https://github.com/Luminous-Dynamics/symtropy/issues/5357)  
**Related process lane:** [#5340 — FOREST-PROCESS-TRAJECTORY-001](https://github.com/Luminous-Dynamics/symtropy/issues/5340)  
**Related identifiability lane:** [#5349 — FOREST-FLUX-IDENTIFIABILITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/5349)  
**Source code:** [teatree1212/DBEN_Eckes_Shephard_et_al](https://github.com/teatree1212/DBEN_Eckes_Shephard_et_al)  
**Audit date:** 15 November 2026  
**Status:** source-code metadata audit only; no Zenodo archive bytes ingested

## 1. Why a crosswalk is mandatory

The ForestPaths benchmark is a multi-model comparison, not one homogeneous data table. Its processing repository provides per-model functions to translate file names, run IDs, variables, units, size classes and forcing identifiers into the paper's analysis vocabulary. The Zenodo record calls D-BEN an informal convention that is not yet settled. A direct concatenation of CSV/netCDF fields, or relabelling every column with a common name, would hide model-specific semantics.

The paper's stated comparison covers nine vegetation-demography model families and three main simulation sites (FIN, BIA and BCI). The analysis helper confirms the site and model vocabulary, but it also demonstrates that the common label may hide different source variables and transformations.

## 2. Exact source snapshot inspected

| File | Blob SHA | Relevant content |
|---|---|---|
| `scripts/DBEN_helper_functions.R` | `395aa877985de55c38080d5b74a920035e5efe87` | model/site/variable arrays, file path rules, run mapping, per-model unit and semantic adjustments |
| `scripts/Format-DBEN_Eckes-Shephard_et_al2025.R` | `6f8c1a69654feac83a463ca3582d8c5ff7564c65` | quantity vocabulary/units, PFT and size-class layers |
| `scripts/Eckes-Shephard_et_al2025.Rmd` | `5354a054234bf28f37d04c8341cdf611728c63a0` | equilibrium periods, carbon-budget diagnostic, figure and benchmark calculations |
| `LICENSE` | `261eeb9e9f8b2b4b0d119366dda99c6fd7d35c64` | Apache License 2.0 for repository code only |

These are Git blob IDs, not an immutable commit-level archive. A reproduction must first pin the full repository commit and retain the exact source files or archive. The code license does not establish the license for the separate Zenodo model-output/observation archives.

## 3. Repository-level dimensions

### Sites

- `FIN`
- `BIA`
- `BCI`

### Model names in the helper

1. `JULES-RED`
2. `FATES`
3. `ORCHIDEE`
4. `LPJ-GUESS`
5. `CABLE-POP`
6. `BiomeEP`
7. `BiomeE-Standalone`
8. `SEIB-DGVM`
9. `EDv3`

Do not collapse `BiomeE-Standalone` to `BiomeE` in the source lineage field. A display label may be normalized, but both the exact source ID and paper-family label must be retained.

### Declared common variables

The helper declares this commonly used 15-variable set:

`cveg, cwood, cwood_size, nstem_size, lai, CA, BA, height, WBgrowth, BAgrowth, cmort, stemmort, gpp, npp, nbp`

The format file defines additional aliases and metrics, including `AGcwood`, `AGcwood_size`, `AGB`, `cmort_size`, `cdstb_size`, `cmort_rate`, `stemmort_rate`, `growth_rate`, and `BA_size`. The full set and units must be captured in the machine-readable source crosswalk rather than inferred from column headings.

## 4. Quantity semantics and units in the paper-specific format

The source format file declares, among others:

| ID | Declared meaning | Format-layer unit |
|---|---|---|
| `cveg` | vegetation carbon mass by PFT | kgC m⁻² |
| `cwood` | wood carbon mass by PFT | kgC m⁻² |
| `AGcwood` | aboveground woody biomass | kgC m⁻² |
| `AGcwood_size` | aboveground woody carbon by size class | kgC m⁻² |
| `AGB` | aboveground total biomass | kgC m⁻² |
| `cwood_size` | wood carbon mass by size class | kgC m⁻² |
| `nstem_size` | stem number by size class | count ha⁻¹ |
| `stemmort_rate` | stem mortality rate by size class | % yr⁻¹ |
| `growth_rate` | woody biomass growth rate | % yr⁻¹ |
| `lai` | leaf area index | m² m⁻² |
| `CA` | crown area | m² ha⁻¹ |
| `BA` | basal area | m² ha⁻¹ |
| `height` | mean PFT height at the 95% height percentile | m |
| `WBgrowth` | woody biomass growth | kgC m⁻² yr⁻¹ |
| `BAgrowth` | basal area growth | m² ha⁻¹ yr⁻¹ |
| `cmort`, `cmort_size` | carbon lost by mortality **or other turnover processes** | kgC m⁻² s⁻¹ in the format layer |
| `cdstb_size` | disturbance carbon loss | kgC m⁻² s⁻¹ |
| `cmort_rate` | wood carbon loss rate | % yr⁻¹ |
| `stemmort`, `stemmort_size` | stem-number loss by mortality **or other turnover** | count ha⁻¹ yr⁻¹ |
| `gpp` | gross primary production | kgC m⁻² yr⁻¹ |
| `npp` | net primary production | kgC m⁻² yr⁻¹ |
| `nbp` | net biospheric production | kgC m⁻² yr⁻¹ |

These are format-layer declarations, not proof that every source model file is stored in those units before the helper transformation. Every read must bind raw units, raw values, transformation identity, transformed units, and the source file that triggered the path.

## 5. Model-specific semantic transformations

### BiomeE-Standalone

The helper explicitly maps:
- `WDgrowTot` to `WBgrowth`, where `WDgrow + WDrepr + WDmgrt` are combined. This includes reproduction and migration, not only individual-tree growth.
- `WDmortTot` to `cmort`, where `WDmort + WDkill + WDdstb` are combined. This includes density-related killing and disturbance removal, not a mortality-only signal.

These aggregates may be useful carbon-flux accounting quantities, but they must not be used as pure growth or pure mortality labels without a decomposition of their components.

### FATES

- `stemmort` is read from separate overstory and understory files and summed by the helper.
- The helper includes a comment noting a size-class handling caveat in this combination.
- GPP/NPP are passed through the seconds-to-year helper.
- `nstem_size` and `stemmort` are multiplied by 10,000 to scale per-m² values to per-hectare values under the noted run conditions.
- FATES uses `cru_jra2.1` as the forcing identifier in its source adapter, unlike several other models using `cru_jra2.2`.

### BiomeEP

- `cmort` is directed to a `cmort_size` file/quantity path.
- Size-class and stem mortality variables have filename/quantity-name distinctions.
- `stemmort_size` and `stemmort` receive a ×10,000 conversion in the helper for per-hectare values.
- Model run IDs are translated differently from the paper-level P0–P7 labels.

### LPJ-GUESS

- The helper adapts `412ppm` and `562ppm` to `PS_412ppm/` and `PS_562ppm/` directory conventions.
- Certain `cmort` variants and `nbp` have units relabelled to kgC m⁻² yr⁻¹ with a comment stating the exported values are already annual; do not multiply those a second time.
- The model output directory points into a specific postprocessed LPJ-GUESS subdirectory.
- Forcing ID is `cru_jra2.2`.

### CABLE-POP / JULES-RED / SEIB-DGVM

- Helper paths differ for P0 vs sensitivity runs.
- Some `cmort` and production variables are multiplied by seconds per year using `change_unit_stoy`.
- The common name is not a sufficient indication of whether the stored source value was per-second or already annual.

### ORCHIDEE

- The helper has a distinct forcing identifier spelling `crujra.2.2`.
- Its size-class representation is specifically adapted in the format file and is not directly aligned to the shared/FATES/Paper1 size bins.

The machine-readable crosswalk records these as **source-specific transformations**, not as already validated universal conversions. Re-fetch the source blob before production use.

## 6. Non-equivalent size-class supports

The format file defines multiple size-class families:
- shared DBEN: `<1, <5, <10, <15, <20, <30, …, <250, 300+`;
- ORCHIDEE: three custom categories `1, 2, 3`;
- FATES: its own sequence including `<1, <5, <10, <20, …, <300, 300+`;
- Paper1: `<1, <5, <10, <15, …, <200, >=200`;
- establishment cohorts: `sdlg<1, sdlg<5, saplg<10, saplg<15`.

These are not directly comparable bins. A valid common size-class score requires raw lower/upper bounds, overlap-based redistribution or another justified conversion, count/biomass conservation checks, and propagation of within-bin uncertainty. Never interpolate counts as though they were continuous densities without recording the assumption.

## 7. Run-name mapping and forcing identifiers

The helper maps benchmark run labels to model-specific directory tokens. This is part of the input identity and must be included in every output receipt.

| Model helper name | P0–P7 source run IDs |
|---|---|
| JULES-RED | 1, 1, 2, 3, 4, 5, 6, 7 |
| FATES | P0, P1, P2, P3, P4, P5, P6, P7 |
| ORCHIDEE | P0 for all listed protocol runs |
| LPJ-GUESS | P0, P1, P2, P3, P4, P5, P6, P7 |
| CABLE-POP | 0, 1, 2, 3, 4, 5, 6, 7 |
| BiomeEP | P0, P0, PS1, PS2, PS3, PS4, PS5, PS6 |
| BiomeE-Standalone | 00, 00, 01, 02, 04, 08, 20, 40 |
| SEIB-DGVM | P0, P1, P2, P3, P4, P5, P6, P7 |
| EDv3 | P0, 1, 2, 3, 4, 5, 6, 7 |

The helper comments explain that some model runs are equivalent but have no separate P0 output. The mapping must therefore be copied from the exact source snapshot rather than generated from a universal filename template.

## 8. Equilibrium windows are hand-selected analysis choices

The R Markdown file says equilibrium intervals were obtained by visually inspecting `cwood` time series. The lower/upper years are hard-coded for each site and model. They differ substantially between models (for example, the FIN lower window ranges from 150 to 715 years in model-order mapping).

These values are not a general equilibrium algorithm or a data-driven automatic threshold. A faithful reproduction must first use the exact windows from the published script, then run sensitivity analysis against:
- fixed-duration common windows;
- alternative visually plausible windows;
- an explicit predeclared trend/variance stopping rule;
- exclusion of each early/late segment.

Do not select an equilibrium window using the eventual held-out benchmark result.

## 9. Carbon-budget check and process interpretation

The published R Markdown contains an explicit woody-carbon budget diagnostic that compares `WBgrowth`, `cmort` and `cwood` over a stated post-disturbance range, with custom variable extraction for ORCHIDEE, FATES and BiomeE-Standalone. This is a valuable first reproduction target for #5352 and a source contract for #5340.

However, this script's `cmort` may include turnover and disturbance components and BiomeE-Standalone's `WDmortTot` explicitly combines several losses. Do not treat `WBgrowth - cmort` as a closed live/deadwood ecosystem carbon budget without accounting for recruitment, deadwood, decomposition, exports, disturbance boundary and any omitted pools. It is the source paper's declared diagnostic and should be reproduced as such before being generalized.

## 10. DOI, source and rights blockers

There is a source-level DOI discrepancy:
- R Markdown setup says `10.5281/zenodo.14415142`;
- paper data-availability statement and Zenodo archive instructions point to `10.5281/zenodo.14415143`.

Treat these as distinct records until their metadata and archive bytes are compared. The current #5352 register records 14415143 as the published paper-data target and the 14415142 mention as an unresolved source-code setup discrepancy.

The GitHub repository has an Apache-2.0 `LICENSE`; that license covers the repository code, not automatically the Zenodo data archive. The retrieved Zenodo page's visible record rights field is not populated. Exact data-file rights, attribution and derivative-use permissions must be confirmed before ingesting, modifying or redistributing archive content.

The GitHub main branch did not resolve `Process_observations.R` at either `scripts/Process_observations.R` or repository root during this audit, although the Zenodo record describes an observation-processing script. No root `renv.lock` was found. Locate the exact archived processing script and pin the R/DGVMTools/package versions before claiming reproduction.

## 11. Admission rules

Before normalizing or scoring any output:
1. pin the full repository commit and archive DOI/version;
2. verify exact archive checksums and file-level rights;
3. inspect raw NetCDF/CSV file variables, units, time axis, site and PFT/size-class dimensions;
4. bind model-specific file path, forcing ID, run ID and helper function;
5. preserve source quantity ID and transformed quantity ID;
6. record every value transformation (e.g. ×seconds-per-year, ×10,000, unit relabel only, or overstory+understory sum);
7. check stock/flux support and sign convention;
8. refuse ambiguous composite fluxes as pure growth/mortality;
9. preserve original size classes before any common-support conversion;
10. publish one-to-one source receipts before calculating aggregate scores.

### Claim ceiling

This crosswalk is based on inspected public scripts and format metadata. It does not establish archive retrieval, data rights, exact file schemas, model-output correctness, reproduced figures, a forest hindcast, practical flux identifiability or SOTA performance. The Zenodo archive remains NOT_DOWNLOADED / NOT_INGESTED in the current runtime.


## 12. Distinguish the dataset record from the archived code release

The Zenodo dataset record is **14415143**. It contains the `model_outputs.zip` and `observations.zip` archives, with published MD5 values recorded above. Its landing-page “Get started” instructions link to a separate archived code release **15870478**, version v1.0.0. That software record contains `teatree1212/DBEN_Eckes_Shephard_et_al-v1.0.0.zip` (89.9 kB, published MD5 `8a4d79b4a94dc58a67e36d7e7100cee1`) and links the GitHub tag `v1.0.0`.

The current R Markdown in that GitHub tag still tells the user to download the benchmark data from **14415142**. A direct targeted Zenodo lookup did not resolve that ID to the ForestPaths data archive. Preserve this as an unresolved/stale source-code reference; do not substitute it for 14415143 without evidence. The **14415142 / 14415143 / 15870478** roles should remain distinct in the source register: unexplained Rmd reference, data archive, and code release respectively.

I inspected the GitHub `v1.0.0` tag. Its helper, format and R Markdown blob SHAs match those inspected on main. `Process_observations.R` is still not found at `scripts/Process_observations.R` or repository root in the tag, despite being described as a source script in Zenodo metadata. The code archive and data archive are separate provenance objects and require separate checksums and rights review.
