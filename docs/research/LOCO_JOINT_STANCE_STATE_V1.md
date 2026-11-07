# Six-leg joint stance-state and transition metrics V1

**Tracking:** Symtropy #4091 — LOCO-JOINT-STATE-001  
**Extends:** #3353 / PR #3354.  
**Composes:** #3926 phase-only six-leg generator, #3345 embodied CPG-to-contact experiments, #3357 FlyGym/NeuroMechFly baselines, Symthaea #10919 / PR #10922 phase-response transfer, and animal replay #2145.  
**Metric:** SIX_LEG_JOINT_STANCE_V1.  
**Status:** exact synthetic arithmetic contract; no empirical animal result is claimed.

## 1. Why a joint state is needed

For six canonical legs there are 64 possible simultaneous stance/swing configurations. Marginal duty factor, pairwise phase-vector strength and named tripod overlap are useful projections, but they cannot identify every possible six-way joint distribution.

The October 2026 Pugliese et al. Nature paper distinguishes the production of rhythmic motor output from inter-leg coordination and examines simultaneous stance count and relative phase in Extended Data Fig. 10. It is therefore important that Symtropy can score the full six-leg state, not infer coordination merely because each leg oscillates.

## 2. State definition

Canonical bit order:

    0 LEFT_FORE
    1 LEFT_MIDDLE
    2 LEFT_HIND
    3 RIGHT_FORE
    4 RIGHT_MIDDLE
    5 RIGHT_HIND

Bit 1 = STANCE; bit 0 = SWING. The six-bit integer is the stable state ID in [0, 63]. A full state is emitted only when all six leg observations are either STANCE or SWING.

Intervals are integer-tick, half-open [start_tick, end_tick), positive duration, chronologically ordered and non-overlapping. The caller binds the tick timebase.

## 3. V1 output contract

For each fully observed interval, accumulate duration in the corresponding joint-state bin:

    joint_ticks[mask] += end_tick - start_tick

The report includes:
- exact duration for all 64 configurations;
- duration distribution over 0 through 6 simultaneous stance legs;
- per-leg stance duration and pairwise simultaneous-stance duration derived from the same full-observation denominator;
- both named tripod-group all-stance and all-swing durations, kept separate;
- typed incomplete-observation duration per leg/status and by observed-leg mask;
- transition counts at contiguous boundaries when the complete state changes;
- merged dwell duration and episode count per state;
- unrecorded time gaps, which break transition continuity.

Occupancy mass is returned as exact reduced integer ratios. No floating-point rounding is involved.

### Transitions are not automatically a Markov chain

A directed count from state A to B means that two consecutive, contiguous, fully observed intervals had different masks A then B. Same-mask intervals merge into the current dwell episode. Time gaps and incomplete intervals break continuity and cannot create a transition.

These are empirical trace summaries. V1 does not assume or claim stationarity or the Markov property.

## 4. Decisive synthetic identifiability control

The fixture contains two six-leg stance distributions:
- EVEN-PARITY: all 32 masks with an even number of stance bits, each for one tick;
- ODD-PARITY: all 32 masks with an odd number of stance bits, each for one tick.

For any strict subset of 1–5 legs, the projected marginal distribution is exactly identical between these worlds. Consequently all per-leg marginals, pairwise stance contingencies and every triplet projection agree. The full six-way distributions differ. The number-of-legs-in-stance histogram also differs:

| Number in stance | Even parity ticks | Odd parity ticks |
|---:|---:|---:|
| 0 | 1 | 0 |
| 1 | 0 | 6 |
| 2 | 15 | 0 |
| 3 | 0 | 20 |
| 4 | 15 | 0 |
| 5 | 0 | 6 |
| 6 | 1 | 0 |

This is a mathematical counterexample to treating lower-order summaries as sufficient statistics. It is not a claim that real flies exhibit parity-constrained gait.

A second known-answer pair uses two traces with identical state occupancy [2,1,1] over three states but different order:
- FORWARD: 0→1→2→0
- REVERSE: 0→2→1→0

Occupancy must tie; transition counts must differ.

## 5. Missingness and inference boundary

MISSING_TRACK, OUT_OF_SUPPORT, FAILED_ASSAY, NOT_ASSESSED and DISPUTED are distinct states. Incomplete intervals:
- never become swing or stance;
- do not enter the complete-state denominator;
- remain in typed per-leg and observed-mask duration counts;
- break state-transition continuity.

This crate computes per-trial raw metrics only. Frames/ticks are not independent biological replicates. Animal/session/trial-level uncertainty and paired model comparison belong to the existing replay scorecard #2145 after its own qualification.

## 6. SOTA/industry comparison

Keep the comparison layers distinct:
1. per-leg frequency, stance duty and swing/stance timing;
2. pairwise PLV / phase-vector strength with explicit preprocessing identity;
3. versioned TCS (NatVar 2024 and Sapkal 2026 v2 are different denominators);
4. six-leg phase-offset/order-parameter summaries from #3926;
5. full 64-state joint stance distribution (this work);
6. transition counts and dwell structure (this work).

Benchmark against independent oscillators, central-coupled, sensory-feedback and hybrid generators in #3926, then source-pinned FlyGym/NeuroMechFly and matched controllers via #3357. Use the same synchronized input, sample cadence, missing-data treatment, per-trial boundary and compute budget.

The target is improved held-out joint-state prediction (proper joint log score and multiclass Brier score) on independent animals/sessions, not a higher scalar gait score. Do not claim superiority until data custody, power and paired uncertainty are qualified.

## 7. Claim ceiling

Passing V1 proves exact synthetic bit-mask, duration, transition, dwell and missingness semantics. It does not establish that parity worlds occur biologically, that a high-probability configuration is optimal, that PLV/TCS is causally sufficient, or that a particular connectome pathway generates a gait.
