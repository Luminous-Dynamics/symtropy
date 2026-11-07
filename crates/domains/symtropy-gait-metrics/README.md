# symtropy-gait-metrics

Deterministic per-event/per-trial gait metrics for embodied animal simulation.

## Scope

- Versioned TCS definitions for the 2024 Drosophila natural-variability paper and Sapkal et al. 2026 preprint v2.
- Pairwise phase-vector strength from supplied instantaneous phase arrays.
- Stance duty factor.
- Per-trial median TCS while refusing mixed metric IDs and empty samples.
- Integer-tick event timing and explicit invalid/undefined outcomes.

This crate does not filter signals, compute Hilbert transforms, detect stance/swing events from raw kinematics, simulate a body, or produce benchmark scorecards. Callers must bind the tick timebase, event-extraction method and phase preprocessing in their run manifest. Final scorecard aggregation and paired comparisons belong in the existing animal-replay layer, not here.

## Metric identity

- `TCS_NATVAR_2024`: common three-way swing overlap divided by earliest onset to latest termination.
- `TCS_SAPKAL_2026_V2`: common three-way swing overlap divided by first onset to last onset. This can be undefined for simultaneous onsets; the library rejects a zero denominator and does not clamp results.

The formulas are intentionally distinct and must not be pooled or relabelled.

The conventional phase-vector strength function is a declared formula over caller-supplied phases; it is not silently treated as identical to any paper-specific PLV pipeline.

## Evidence boundary

All tests use synthetic intervals and phase arrays. They qualify the arithmetic and input checks only, not a biological gait, neural circuit, locomotion controller or published result.


## Higher-order six-leg joint stance state

The `joint_state` module adds a versioned six-leg stance/swing representation over all 64 configurations, using the canonical order:

`LEFT_FORE, LEFT_MIDDLE, LEFT_HIND, RIGHT_FORE, RIGHT_MIDDLE, RIGHT_HIND`

It computes exact integer-tick occupancy, the derived number-of-legs-in-stance distribution, per-leg/pairwise/tripod projections, contiguous state transitions and dwell episodes. It preserves missing-track, out-of-support, failed-assay, not-assessed and disputed durations by leg. Incomplete intervals never enter the full-state denominator and never create transitions.

The transition output is an empirical trace summary, not a Markov/stationarity claim. The code does not score the "best" gait, infer biological mechanism, pool frames as independent replicates, or replace the paired replay scorecard.

See `docs/research/LOCO_JOINT_STANCE_STATE_V1.md` and its exact parity/transition fixture for the known-answer contract.
