# ErgoStream multimodal microcosm v0.1

**Status:** first finite evaluator slice staged; exact-head Rust execution pending.  
**Owner:** Symtropy / ERGO-STREAM-MULTIMODAL-001 #7442.  
**Composes with:** #7149/#7152 hidden-world contract, #7193 paired-world Transfer Atlas, #7432 independent structural oracle, Symthaea #13401/#13402 acquisition protocol, and AV-STREAM #12837 clock/sensor ownership.

## Research question

Can a learner distinguish event correspondence from clock registration when two modality-native streams have separate local clocks, asynchronous arrival, missing observations and shuffled semantic pairings—and can it report ambiguity rather than force a unique alignment?

The v0.1 slice is intentionally categorical and finite. It is a correctness microcosm before raw audio/video, continuous trajectories or physical sensors are introduced.

## Related benchmark and method landscape

- **AV-SyncBench (Zhou et al., 2026-07-01)** separates temporal consistency (offset, local jitter, speed change) from semantic consistency (source/timbre replacement under fixed timing). It reports 3,269 videos and 38,390 samples. Its key methodological contribution for this work is to avoid a single synchronization score that confounds time and semantics. https://arxiv.org/abs/2607.00726
- **MATH-SYNC (Hechavarria-Hernandez et al., 2026-10-13)** studies robust affine-clock registration with explicit identifiability conditions, missing anchors, jitter and outlier timing. It preserves native rates and reports paired Huber-vs-OLS schedule-replay results on Harvard and WESAD cohorts. It is a relevant affine-clock baseline, not an event-correspondence learner. https://doi.org/10.3390/math14203720
- **TCC (Doula, ICML 2026)** transports source calibration through paired unlabeled observations and corrects residual mismatch. It motivates a later calibration-transfer track only after event pairing and clock registration are independently qualified. https://proceedings.mlr.press/v306/doula26a.html

These are distinct contributions: AV-SyncBench decouples semantic/timing evaluation; MATH-SYNC formalizes affine-clock identifiability and robust fitting; TCC addresses calibration transport, not latent event identity. ErgoStream must not conflate or claim to replace them.

## Executable slice

Implemented in `src/multimodal.rs` and exported as `multimodal` from the microcosm crate.

### Public observation contract

Each policy-visible observation contains only:

- modality-local observation ID;
- modality ID;
- public observation kind (event or explicit shared clock anchor);
- local sensor timestamp;
- arrival tick;
- categorical symbol.

The policy view is filtered by arrival tick and sorted by arrival order. Sensor timestamps are never silently substituted for arrival order. The view contains no seed, event ID, family ID, generator regime, hidden symbol map, correspondence truth or future observation.

### Evaluator-only world factors

The generator has three association regimes:

1. **Aligned:** both modalities observe the same latent event at the same event slot.
2. **Marginal-matched shuffled:** Beta event identities are permuted across visible event slots. The symbol and timestamp marginals within each modality are exactly preserved under the same seed, because missingness, timing jitter/delay and symbol mapping use independent PRNG substreams.
3. **Periodic ambiguous:** four cyclic phase correspondences are all evaluator-admissible; shared clock anchors are omitted. A single correspondence or affine clock map is therefore not justified.

Alpha and Beta have independent affine clocks with offset, rational rate, bounded timestamp jitter and bounded arrival delay. Event observations can arrive out of timestamp order. Missing events are retained as missingness rather than interpolated.

For the identifiable clock arms, two explicit shared calibration pulses support a separate affine clock-map score. This is a transparent calibration-anchor reference, not a claim that the learner has solved blind clock registration.

## Two independent score planes

### Clock registration

A candidate may propose an affine map from Beta local time into Alpha local time. The evaluator reports:

- whether two distinct shared anchors are available;
- the number of paired anchors;
- RMS and maximum residual;
- correct abstention when no unique map is supported;
- false-unique and unnecessary-abstention indicators.

Clock registration is not event identity.

### Event correspondence

A candidate returns one of:

- `Unique(pairs)`;
- `Alternatives([pairs...])`;
- `Unresolved`.

The evaluator scores exact admissible correspondence classes, false and missed alternatives, false uniqueness, correct set-valued reporting and correct/unnecessary abstention. Pairwise precision/recall/F1 are also reported for the best admissible mapping.

A valid clock map does not prove semantic event correspondence. A semantically plausible event mapping does not prove clock identifiability. Neither establishes causality.

## Initial tests

Seven tests cover:

1. aligned correspondence and independently evaluated affine clock mapping;
2. exact per-modality symbol and timestamp marginal preservation under shuffled cross-modal association, including dropout;
3. rejection of same-slot pairings in the shuffled negative;
4. four-way periodic correspondence ambiguity and correct set-valued/abstention outcomes;
5. clock scoring invariance to semantic shuffling;
6. clock abstention when shared anchors are absent;
7. causal arrival-prefix filtering and fail-closed configuration checks.

The existing `.github/workflows/ergo-stream-microcosm.yml` is the execution authority for Rust 1.96 formatting, tests and strict Clippy. The code is staged but not yet compiled or run in the authoring environment; no PASS is claimed until the workflow completes on the exact PR head.

## Next baseline ladder

The next executable comparisons should be added only after the reference slice is green:

1. nearest-local-time matching;
2. affine-clock OLS and robust Huber/MATH-SYNC-style estimation;
3. cross-correlation where the sampling assumptions fit;
4. DTW with a bounded window and an intentionally over-flexible negative control;
5. one-to-one assignment / unbalanced optimal transport for dropped events;
6. contrastive or dense audio-visual feature alignment on a native dataset adapter;
7. HDC-only, CfC/LTC-only and hybrid acquisition arms.

All candidates receive the same public prefixes and observation/compute budgets. Calibration pulses may be ablated, but hidden event labels never become candidate-visible.

## Limits and next gates

This slice does not yet include raw media, learned event segmentation, multi-stream alignment beyond two modalities, clock resets, piecewise drift, duplicate/corrupted events, active clock-anchor requests, or candidate process isolation. It does not estimate clock parameters with OLS/Huber/DTW, issue a security claim, or establish any physical or causal relationship.

Next gates:

1. exact-head Rust 1.96 format/tests/doctests/strict Clippy;
2. independent fixture checks, including matched marginal assertions and ambiguity counts;
3. explicit clock-anchor ablation and event/clock factorial suite;
4. robust affine estimator baselines and missing-event assignment baselines;
5. actual Symthaea #11171 policy adapter only after its owning callable is pinned;
6. sealed held-out worlds after acquisition and correspondence proposals are frozen.

No world-model, sensor/timebase, scheduler, learner, evidence-ledger or qualification authority is duplicated here.
