# Algal motility benchmark: diatom gliding and Chlamydomonas phototaxis

This portfolio intentionally covers two different algal locomotion mechanisms. It is not a universal 'algae model'.

## 1. Raphid pennate diatom: raphe-contact gliding

Golfier et al. (2026), PNAS 123(14), e2506122123, DOI [10.1073/pnas.2506122123](https://doi.org/10.1073/pnas.2506122123), study *Craspedostauros australis* (CCMP3328), a rigid-walled pennate diatom that glides on submerged surfaces using an intracellular actomyosin motility complex and adhesive EPS strands secreted through raphe slits.

The key finding is that cells tune trajectory curvature by moving the active cell–substrate contact site and switching between one-raphe and two-raphe branch contact gliding. One-raphe gliding produces high curvature; two-raphe gliding produces lower curvature. The mechanism couples cell-wall/raphe morphology, substrate contact and trajectory dynamics; it is not flagellar swimming or amoeboid crawling.

Source dataset: Zenodo [10.5281/zenodo.17791511](https://doi.org/10.5281/zenodo.17791511), version v1, published 19 February 2026. It contains tracking data, raw SEM images, segmented raphes, raphe-curvature analysis, interference-reflection microscopy (IRM) contact observations, MSD data and analysis/model code. The six listed file bundles total 268.6 MB, while the record reports a data-volume figure of 6.3 GB. The Zenodo record's license field was empty in the inspected metadata. The paper itself is CC BY 4.0, but that does not establish the dataset's terms. Do not ingest the dataset until the exact record/file license and SHA256 inventory are verified.

Observation details from the paper:
- Brightfield single-cell tracking at 1 fps, up to 60 minutes, fields of view 3.2 mm × 3.2 mm, up to 100 cells per field.
- IRM contact tracking uses tiled imaging with 2.56- or 4.16-second effective frame intervals.
- SEM provides raphe structure and shape; IRM provides a substrate-contact proxy.
- The model's crucial observables are path curvature, speed and active raphe-contact mode, not only net displacement.

Benchmark tasks: predict path curvature from raphe geometry/contact state, detect transitions between one- and two-raphe contact, explain speed–curvature coupling, and generalize to cells of different size and raphe geometry.

## 2. Green microalga: Chlamydomonas swimming and phototaxis

Bosc, Pianetti & Paternò (2026), *Frontiers in Photobiology* 4:1800988, DOI [10.3389/fphbi.2026.1800988](https://doi.org/10.3389/fphbi.2026.1800988), present an integrated single-cell framework for *Chlamydomonas reinhardtii* phototaxis.

The representative experiment is a 60-second fluorescence video at 10 fps, with 10 seconds of baseline swimming followed by 470-nm directional light stimulation. The paper reports 607 trajectories from TrackMate in one representative sequence, of which 497 passed its tracking-continuity/quality checks. Three independent biological replicate analyses are publicly available.

The source workflow separates stationary/unmotile, confined/circling and directional tracks, then decomposes directional cells into positive- and negative-phototaxis subpopulations using circular-feature clustering. It uses time-windowed MSD to identify motion-regime changes. This is an important challenge to a population-mean-only phototaxis score: positive and negative responses can cancel.

Data and code: Zenodo [10.5281/zenodo.19354253](https://doi.org/10.5281/zenodo.19354253) and [10.5281/zenodo.19364857](https://doi.org/10.5281/zenodo.19364857). The article is CC BY; exact Zenodo record versions, per-file licensing, file manifests and checksums still require review.

A complementary population-kinetics dataset by Choudhary et al. (2024), Dryad [10.5061/dryad.sj3tx96dg](https://doi.org/10.5061/dryad.sj3tx96dg), contains light-response and recovery data across light intensities and cell densities: polarization, speed, orientation distributions, and response/recovery time constants. The retrieved Dryad page did not expose a dataset-specific license value; this dataset also remains NOT_ADMITTED pending exact rights verification.

Benchmark tasks: reproduce positive/negative response mixture fractions, per-cell speed/direction statistics, light-response and recovery kinetics, and time-windowed MSD. Compare the source pipeline against a global-mean directional baseline to quantify when averaging conceals subpopulations.

## Shared evaluation contract

The two lanes share only measurement/evaluation semantics:
- physical units and time/observation cadence;
- per-cell trajectory identity and quality/retention disposition;
- directional statistics with circular methods;
- uncertainty and replicate-level splitting;
- observed-window MSD and model-regime comparison;
- rights, provenance and reproducible source manifests.

The physical models remain separate: raphe-contact actomyosin gliding versus biflagellar swimming and light-sensory control.

## Comparators and success criteria

For diatoms compare with the source-authored raphe/contact geometry model, empirical per-cell curvature/speed distributions, and constant-curvature/constant-speed nulls. For Chlamydomonas compare with the source-authored TrackMate + circular-clustering + windowed-MSD pipeline, global-mean phototaxis and single-behaviour-class baselines.

Success requires source reproduction over the observed window, held-out biological-replicate predictions, calibration, and an error/cost report under identical observations and compute. Do not rank a gliding diatom and a swimming alga by raw speed as a cross-species universal performance measure.

## Rights and claim boundaries

- Diatom Zenodo dataset licence: NOT_VERIFIED; not admitted.
- Chlamydomonas Zenodo data/code exact versions and file terms: NOT_VERIFIED; not admitted.
- Chlamydomonas 2024 Dryad dataset licence: NOT_VERIFIED in the retrieved record; not admitted.
- Article licenses do not automatically license separately deposited data/code.
- No source data have been downloaded, ingested, fitted, transformed or redistributed in this workstream.

This portfolio defines comparison targets and source-readiness, not a completed biological simulation, reproduction, or SOTA result.