# Lunar granular-flow source data acquisition and admission

**Status:** protocol committed; no external dataset bytes ingested as of 18 November 2026.

This document governs source retrieval and admission for the granular-flow benchmark. The machine-readable registry is `tools/lunar-granular-flow-benchmark/data-sources.json`; the hash-first receipt command is `scripts/register-lunar-granular-flow-source.sh`.

## Source inventory

### A. Wylęgała & Uhl — rotating drum wall/vacuum

- Publication: DOI [10.1016/j.actaastro.2026.07.006](https://doi.org/10.1016/j.actaastro.2026.07.006).
- Zenodo record: DOI [10.5281/zenodo.18749190](https://doi.org/10.5281/zenodo.18749190).
- Frame-metrics CSV: `metrics_frame_level.csv`, listed as 38.8 MB, publisher MD5 `55c298d9edccc571f0ca0a3ec8e36fbd`.
- Raw and processed video: `materials.zip`, listed as 39.4 GB, publisher MD5 `2bf754879dc088c2cb648c5441c7d640`.
- CSV is the first ingestion target. It contains frame-level angle and stability metrics and is a lower-cost path to source-summary reproduction.
- Video archive is only needed for segmentation audit, disputed trials and reconstruction of the five image-derived metrics.
- The Zenodo record exposes file names and MD5 values; exact dataset-level redistribution licence metadata must be verified at retrieval. Do not assume the article's licence automatically covers every dataset object.

**Reproduction tasks:** reconstruct material/wall/pressure/speed identifiers; confirm frame sampling and segmentation semantics; reproduce mean dynamic angle, angle stability CV, mask compactness, P95 pile height and normalized avalanche frequency; report per-cell sample count, missing frames, censoring and uncertainty. The CSV and video must remain separate source artifacts joined by source trial/frame IDs.

### B. Lappa, Watson & Vincent Bonnieu — LHS-1 hopper

- Publication: DOI [10.1122/8.0001055](https://doi.org/10.1122/8.0001055).
- Dataset: DOI [10.15129/22b792eb-7ca4-46b1-b2eb-6ef15b95a281](https://doi.org/10.15129/22b792eb-7ca4-46b1-b2eb-6ef15b95a281).
- University record: [dataset description and access](https://pureportal.strath.ac.uk/en/datasets/data-for-vibrationally-aided-flow-of-lunar-regolith-simulant-in-a/).
- Video corpus: ZIP listed as 558 MB; folder names map to figures in the article; record lists CC BY 4.0.
- The dataset page does not expose an MD5 checksum in its captured metadata. Use local SHA-256 after retrieval, check ZIP integrity, preserve original folder names, and mark upstream checksum as not provided.
- Request / record the full acquisition metadata and the mapping from videos to paper figures, apparatus configuration, vibration frequency/amplitude, charge mass, and observed regime.

**Reproduction tasks:** classify funnel / mass / asymmetric / ratholing flow; identify arching, clogging and restart events; measure mass flow only where source measurements are available; do not infer quantitative mass rate from video alone without scale/calibration. Keep LHS-1 data distinct from ICN-LHT-1.

### C. Stepanova, Dreyer & Garvey — ICN-LHT-1 parabolic-flight feeder

- Publication: DOI [10.1007/s44461-026-00021-y](https://doi.org/10.1007/s44461-026-00021-y).
- The article's data statement says load-cell records, accelerometer measurements and video are available from the corresponding author on reasonable request. No public 480-row archive has been located.
- Request: raw load-cell traces (including timestamps and tare/correction), raw accelerometer streams and axis orientation, per-parabola videos and time sync, condition schedule, assembly ID mapping, feeder encoder/RPM traces, calibration certificates, data dictionaries, material batch/preparation records, and any exclusions or failed-sensor labels.
- Request permission to derive and publish time-series labels / summary metrics and the required attribution/licence conditions.
- The current 480 entries in `advanced-doe-v0.json` are empty source-record templates, not measurement records. They must stay out of training and scoring until actual source files are received, hashed, and reconciled with the source article.

## Hash-first import

For a file with a publisher MD5 in the source registry:

```sh
bash scripts/register-lunar-granular-flow-source.sh \
  WYLEGALA-UHL-2026-ROTATING-DRUM \
  frame-level-metrics \
  /path/to/metrics_frame_level.csv
```

For files without an upstream MD5, the command records the local SHA-256 and marks the receipt as lacking an upstream checksum; ZIP files must pass structural integrity testing. Every receipt remains `pending_schema_license_and_provenance_review` and `training_eligible=false`.

The command never extracts or edits the source file. Preserve original bytes in an immutable/content-addressed source area; perform all normalization and analysis on derived copies.

## Admission gates after retrieval

1. **Bytes:** exact filename, size, retrieval time, publisher MD5 if provided, local SHA-256, archive integrity.
2. **Rights:** exact record/file licence and permission for redistribution, derived dataset publication, model training and AI processing. Unknown rights block onward publication.
3. **Schema:** source-specific parser and field dictionary; units, coordinate frames, sample rates, calibration and clock synchronization.
4. **Provenance:** DOI, creators, material, batch, preparation, wall, apparatus, gravity, pressure, temperature, vibration, feeder settings, trial/frame identity.
5. **Semantics:** missingness, exclusions, censoring, duplicated frames, failed sensors, invalid values and uncertainty.
6. **Reproduction:** match published tables/figures within stated rounding and measurement uncertainty before adding the data to model calibration.
7. **Split integrity:** hold out complete material batches/campaigns/experiments; video frames from one trial must never be split across calibration and blind test.

## Claim boundaries

- A public DOI is not proof that a download succeeded.
- A verified checksum proves byte identity, not semantic correctness or physical validity.
- LHS-1 and ICN-LHT-1 must remain separate material domains.
- Rotating-drum metrics do not qualify hopper discharge; hopper-to-feeder supply does not qualify free-gravity discharge or beneficiation.
- No data source in this registry has yet been used to train or score a Symthaea model.
