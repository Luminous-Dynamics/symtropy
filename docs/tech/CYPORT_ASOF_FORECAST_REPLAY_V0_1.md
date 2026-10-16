# CyPort As-of Forecast Replay Contract v0.1

**Status:** policy frozen; no historical forecast replay has run and no predictor is approved.  
**Machine-readable contract:** [CYPORT_ASOF_FORECAST_REPLAY_V0_1.json](./CYPORT_ASOF_FORECAST_REPLAY_V0_1.json)  
**Feature register:** [CYPORT_FEATURE_AVAILABILITY_V0_1.json](./CYPORT_FEATURE_AVAILABILITY_V0_1.json)  
**Benchmark tasks:** [CYPORT_BENCHMARK_TASKS_V0_1.json](./CYPORT_BENCHMARK_TASKS_V0_1.json)  
**Pinned target table:** [CyPort interaction profile](./MARITIME_CYPORT_INTERACTION_PROFILE_V0_1.md)

## Paradigm shift: replay what an operator could know, not what hindsight can reconstruct

The current source is a retrospective port–cyclone interaction table. That is useful for describing historical outcomes, but it does not prove that its realized wind, pressure, surge, rainfall, storm-path, network, or recovery fields were available at a decision time. The next benchmark must reconstruct a dated information set: an immutable forecast product, its actual publication time, the forecast's valid period, the exact source revision, and the row/port/storm mapping. Only then can performance be called a forecast rather than a retrospective association.

The NOAA National Hurricane Center (NHC) archive is a plausible route because it exposes a Tropical Cyclone Advisory Archive and historical text and graphical products. The archive itself warns that not all advisories are available; its existence is not proof of complete coverage. Start with the NHC [Data Archive](https://www.nhc.noaa.gov/data/) and audit product-level availability storm by storm.

Crucially, do **not** substitute final best-track values for archived forecasts. NHC's [HURDAT2 documentation](https://www.nhc.noaa.gov/data/hurdat/hurdat2-format-nov2019.pdf) describes best-track data as a post-storm analysis using observations, including observations that may not have been available in real time. NOAA's [IBTrACS](https://www.ncei.noaa.gov/products/international-best-track-archive) is a valuable consolidated best-track resource, but not an operational forecast archive. Those data can support target reconciliation or post-hoc sensitivity analysis; they must not masquerade as forecasts known before a port–storm event.

## Important limit: CyPort only supplies dates here

The pinned interaction table's `start_date` is date-only. The current profile has not established its timezone or within-day event-time semantics. A timestamped NHC issuance cannot be compared to a date-only target to truthfully report an exact 24-, 48-, or 72-hour lead time. Nor should a date silently be converted to midnight UTC and treated as the physical onset.

Accordingly, v0.1 deliberately blocks **all lead-time claims and replay execution**. The future 24/48/72-hour goals are research targets only; they become testable only after target timestamps or justified bounds and time-zone semantics are established. A weaker date-bucket study may later be useful, but it must be labelled as such and must not be compared to hour-scale operational warning benchmarks.

## Evidence needed for every candidate feature

Every captured value must be traceable to the exact source artifact and to an operator's information set at the chosen cutoff. The receipt contract lists the required fields, including the CyPort interaction and storm/port keys, product ID and URI, SHA-256 and byte length, publication and retrieval times, forecast valid interval, units, transformation lineage, join method, storm-crosswalk evidence, and target-time review.

Rules are fail-closed:

- A feature is eligible only if its source was published and accessible by the registered issue cutoff; retrieval time alone does not prove publication time.
- A forecast value must record both issuance and valid interval. A realized observation is ineligible if it became available after the cutoff.
- Retrospective best-track values, post-storm corrections, recovery markers, impact outcomes, and network statistics computed using future windows are not forecast-time predictors.
- A static-looking port attribute still needs source vintage, join and transformation provenance. It is not automatically safe merely because it changes slowly.
- Missing values need explicit reasons; they must not be backfilled from future records.
- The storm-ID crosswalk from CyPort `SID` to the source archive's storm identifier must be versioned, audited, and report unmatched and ambiguous IDs. No name-only fuzzy join.
- Raw data should stay outside the public repository unless source-specific terms and redistribution rights are verified. Keep compact receipts, immutable identifiers, and reproducible extraction rules instead.

## Staged work and benchmarks

| Stage | Gate | Benchmark / evidence |
|---|---|---|
| 0. Source identity | Pin each source resource and review reuse terms | Exact revision, byte length, digest, citation and rights record; no mutable `latest` alias |
| 1. Archive coverage | Map the 90 CyPort cyclone SIDs to archived NHC products | Report matched, unmatched, ambiguous, unavailable, and malformed products separately; never assume archive completeness |
| 2. Point-in-time feature audit | Prove publication/availability precedes cutoff | 100% of proposed feature values have source hashes, issue/valid/available times, units and lineage; zero post-cutoff predictors |
| 3. Target-time audit | Establish date timezone/semantics or better target timestamps | No exact-hour lead metric until the time precision supports it |
| 4. Baseline replay | Freeze all preprocessing and compare simple reference forecasts | Training-only prevalence/climatology, then regularized baseline; 2021 used only for calibration/threshold choice, 2022–2023 storm-grouped test locked |
| 5. Useful warning signal | Evaluate rare-event discrimination and probability quality | Aim for PR-AUC at least 1.5× the held-out event prevalence; Brier skill score at least +0.10 versus training-prevalence forecast; aim for recall ≥0.80 at false-positive rate ≤0.20 at a threshold frozen without test tuning |
| 6. Generalization | Check uncertainty and event/region shift | Bootstrap by cyclone SID; report independent positive-storm counts and intervals for every headline score; no confirmatory claim if positives are concentrated in too few storms |
| 7. Operational readiness | Replay multiple issue times against real archive vintages | Only after timestamps exist: benchmark 24/48/72-hour forecasts separately, report calibration, usable lead time and cost-weighted decisions, then replicate on an independent time period or source |

The numeric goals in Stage 5 are **aspirational research gates, not results or safety thresholds**. The current temporal test has only 18 cyclone groups and 26 marker-present rows; that is too small to claim stable calibration or broad operational skill. Collect a larger, independent test set—preferably at least 30 test storms and 50 positive examples before the first formal go/no-go, with more positives needed for subgroup calibration. A threshold must ultimately reflect the actual port operator's false-alarm and missed-disruption costs.

## Benchmarks that matter more than headline accuracy

The current rounded impact field is zero in 85.83% of records. A constant-zero classifier can look successful on raw accuracy while missing all positive rounded impacts. The primary suite should therefore use PR-AUC / average precision with event prevalence shown alongside it, Brier score and log loss versus a train-only prevalence reference, precision/recall at pre-registered operating points, and storm-clustered uncertainty. Add severity MAE/RMSE and count deviance only for models that predict `total_impact`; keep zero-inclusive and positive-only results separate. Recovery error or survival scores remain blocked until recovery and censoring semantics are established.

Keep the paper-reproduction lane separate. Kuai et al. (2026) report a model-based study built on CyPort; the reported filtering and random-split protocol is not directly comparable to this frozen temporal split. Use the published paper for a clearly labelled reproduction, not as a substitute for as-of replay.

## Current claim ceiling

This increment freezes the information-time rules and identifies an archive candidate. It does **not** assert NHC coverage, build the storm crosswalk, authorize a feature, execute a replay, fit a model, report predictive scores, estimate intervention effects, establish physical truth, or authorize operational action. The next data task is an auditable storm-by-storm archive inventory plus target-time semantics review—not a bigger model.
