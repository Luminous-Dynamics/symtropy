# PHRI-TAC-TRANSFER-001 synthetic oracle

A standalone Rust reference for the Symtropy #4776 cross-sensor tactile calibration-transfer contract.

Run from the repository root:

    cargo fetch --locked --manifest-path research/phri-tac-transfer-001/Cargo.toml
    cargo fmt --manifest-path research/phri-tac-transfer-001/Cargo.toml --all -- --check
    cargo test --locked --manifest-path research/phri-tac-transfer-001/Cargo.toml
    cargo clippy --locked --manifest-path research/phri-tac-transfer-001/Cargo.toml --all-targets -- -D warnings
    cargo run --locked --manifest-path research/phri-tac-transfer-001/Cargo.toml

## Active calibration planner

The library also exposes a coverage-first planner in `src/active_plan.rs`. It ranks only candidate contacts whose target unit/profile, phantom revision, reference calibration, method and operating conditions match the request. Mandatory operating strata are covered before optional contacts are ranked by a model-supplied information-per-cost score.

That score is a prioritization hint, not a measurement-uncertainty calculation. The planner does not operate a fixture, create force labels, certify calibration or authorize a physical contact. Seven unit tests cover coverage precedence, stale reference/method/condition rejection, exact identity binding, budget shortfall, deterministic tie-breaking, duplicate identities and invalid ranking scores.

The corpus and JSON Schema are language-neutral. The Rust deny_unknown_fields structs and semantic evaluator are the executable reference. The test suite includes mutations for sample/sequence/lot leakage, source-certificate reuse, target calibration mismatch, stale firmware, false metrology promotion, invalid reference independence and unqualified uncertainty.

Claim ceiling: synthetic reference only. A passing run does not qualify a real sensor, calibration, uncertainty interval, transfer model, robot task, human interaction or safety claim.


## Active calibration policy comparison

The crate also exposes a deterministic synthetic nonlinear response-surface comparison in `src/active_benchmark.rs`.

The runner compares:
- full target grid reference;
- uniform stratified;
- seeded random stratified;
- maximin stratified;
- D-optimal stratified;
- source-model transfer without target labels;
- source-model prior plus D-optimal target adaptation.

The target candidate pool has 72 contacts across four temperature × loading-phase strata. The held-out target domain has 60 interleaved force/position/condition cases that are not passed to any selector. Sparse target policies receive 16 target reference contacts (four per mandatory stratum). Source reference contacts and target reference contacts are reported separately: the transfer-only arm uses source contacts only (zero target calibration contacts), while transfer-plus-adaptation reports both source and target contact budgets.

Results report synthetic regression RMSE, p95/worst absolute error, a regression prediction-interval diagnostic, and a transparent synthetic contact-cost proxy. The interval is not a GUM uncertainty budget, metrological uncertainty statement or calibrated physical coverage result. The cost proxy is not measured laboratory time. These are synthetic model estimates only.

The target response profile is deliberately shifted from the source response profile. The harness is intended to reveal whether an active policy or transfer prior helps under sensor shift; no policy is presumed superior. Use multiple preregistered seeds and keep the full held-out set out of policy selection.


### Selector/evaluator separation — hardened

The active-policy API receives only `&[Candidate]`. A candidate contains a commanded stimulus setpoint and a sensor-derived feature vector; it does not contain a reference-force label. Reference-force observations live in a separate `SyntheticDomain` used by model fitting after selection. Held-out target truth lives in `HeldoutDomain` and is passed only to the evaluator after every policy selection is frozen. The synthetic reference channel also has a separate deterministic noise term. This is a code-level data boundary, not merely a statement that the selector promises not to read labels.

The candidate setpoint is a declared stimulus, not an independently measured force. The held-out evaluator compares predictions with its separate synthetic truth. The regression prediction interval still does not constitute a metrological uncertainty budget.


## Single-seed policy comparison — preliminary only

The first hosted execution on 16 November 2026 completed the Rust test suite, strict Clippy and tracked-corpus CLI. For the one frozen seed in v0.1, the 16-contact D-optimal arm produced a 0.004986 N synthetic RMSE versus 0.004234 N for the 72-contact target-grid reference. Uniform and maximin were substantially worse on this response surface; the source-only arm was poor under the declared source/target shift; the transfer-prior arm traded somewhat better worst-case error/interval diagnostic coverage against greater total source+target contact cost. These are deterministic synthetic results, not physical calibration measurements or a stable policy ranking.

This result is deliberately not the campaign conclusion. [PHRI-TAC-ROBUST-001 #5591](https://github.com/Luminous-Dynamics/symtropy/issues/5591) now owns paired multi-seed statistics, model-misspecification families, reference-channel common-mode/noise injection and per-stratum failure analysis. No policy should be selected from this single seed.


## Paired multi-seed robustness campaign

The follow-on executable exposes `run_active_calibration_campaign(first_seed, replicate_count)`. It requires at least 50 paired seeds and evaluates every policy on the same source, target and held-out domains for each seed. Each policy reports mean, median, p95 and maximum values for synthetic RMSE, p95/worst absolute error, prediction-interval coverage/width and generated fixture-cost proxy. It also reports the per-seed paired RMSE difference against the full target-grid arm.

The first implementation is a stability screen, not a complete statistical analysis: bootstrap confidence intervals, cluster-aware inference, model-family misspecification, common-mode reference faults and multiple physical target articles remain follow-on requirements under #5591. These repeated runs remain synthetic and do not establish physical uncertainty coverage or calibration savings.


### Paired seed-level bootstrap interval

The campaign now computes a deterministic 95% percentile-bootstrap interval for the mean **paired per-seed RMSE difference versus the full-grid arm** (2,000 resamples with replacement over the 50 or more seed-level paired differences). The CLI emits lower/upper interval endpoints per policy.

The resampling unit is a complete paired synthetic seed, not a contact/frame. This avoids treating dependent contacts inside one generated domain as independent replicates. It estimates variability across this fixed synthetic generator's random seeds only; it is not a confidence interval for physical sensor populations, calibration uncertainty, a GUM coverage interval or transfer to new hardware.


## First hosted 50-seed campaign — 16 November 2026

Workflow #78 on source head `a419a63a1a8aa9335d6a2ee0dbcde9f4f7c5d5c6` completed with 40 Rust tests passing, rustfmt passing, strict Clippy passing, the 34-case transfer oracle passing and the 50-seed campaign CLI completing.

First 50-seed campaign output (all values are synthetic model estimates; force units are N):

| Policy | Target refs | RMSE mean | RMSE median | RMSE p95 | RMSE max | Prediction-interval diagnostic mean | Paired RMSE delta mean | Paired bootstrap 95% interval | Synthetic cost ms mean |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Full target grid | 72 | 0.004234 | 0.004234 | 0.004234 | 0.004234 | 1.0000 | 0.000000 | [0.000000, 0.000000] | 45,240 |
| Uniform stratified | 16 | 0.033170 | 0.033170 | 0.033170 | 0.033170 | 1.0000 | 0.028936 | [0.028936, 0.028936] | 10,480 |
| Random stratified | 16 | 0.005848 | 0.005460 | 0.008348 | 0.010340 | 0.9610 | 0.001615 | [0.001273, 0.001995] | 10,019.4 |
| Maximin stratified | 16 | 0.062137 | 0.062137 | 0.062137 | 0.062137 | 1.0000 | 0.057903 | [0.057903, 0.057903] | 10,480 |
| D-optimal stratified | 16 | 0.004986 | 0.004986 | 0.004986 | 0.004986 | 0.9833 | 0.000752 | [0.000752, 0.000752] | 10,400 |
| Source-only transfer | 0 | 0.530757 | 0.530757 | 0.530757 | 0.530757 | 0.1000 | 0.526523 | [0.526523, 0.526523] | 45,240 |
| Transfer + D-optimal adaptation | 16 | 0.005161 | 0.005161 | 0.005161 | 0.005161 | 1.0000 | 0.000928 | [0.000928, 0.000928] | 55,640 |

### Interpretation and limits

- On this single synthetic response family, target-only D-optimal is the strongest sparse policy by mean and worst-case RMSE; it uses 16 rather than 72 target reference contacts and approaches the full-grid reference.
- Seeded random stratification is less stable: lower mean RMSE than a poor fixed design, but higher p95/worst errors and a lower mean regression prediction-interval diagnostic than D-optimal.
- Uniform and maximin are poor for this response basis and candidate geometry. A coverage metric alone does not guarantee parameter identifiability or good inverse-response prediction.
- Source-only transfer fails under the deliberately shifted sensor profile. Transfer plus target adaptation recovers but consumes 72 source and 16 target reference contacts, costs more than target-only D-optimal and does not improve its mean or worst-case RMSE. It is not a calibration saving in this test.
- Several seed-level distributions and bootstrap intervals collapse to a single value because this generator has fixed model coefficients, fixed stimulus geometry and only small deterministic noise changes. This does **not** establish policy stability across sensor articles, lots, response families or real physical units.
- The bootstrap resamples paired seed-level differences, but these seeds are not independent physical instruments. The interval is conditional on this synthetic generator. The pseudo prediction interval is not a GUM uncertainty budget, and the cost values are not measured fixture time.

The next required campaign changes the physical/model response family and introduces model misspecification, nonlinearity, hysteresis, reference common-mode faults, aging/remount and unit/lot variation. No physical acceptance decision should be based on this first campaign.


## Adversarial response-family campaign

The follow-on module `src/active_benchmark/adversarial.rs` adds five synthetic known-answer families while keeping the fitted inverse-response basis fixed:
- nominal control;
- cubic force response outside the inverse basis;
- force × temperature × loading-history interaction;
- upper response saturation/clipping;
- common-mode reference-channel bias applied to training reference labels while held-out evaluator truth remains independent.

The selector-visible candidate type still contains no reference label. The generated training domain stores labels separately, and the held-out domain stores evaluator truth separately. The campaign compares full-grid, uniform, random, maximin, D-optimal, source-only transfer and transfer-plus-adaptation over 50 paired seeds for each family.

These cases are synthetic fault injections; saturation, aging, hysteresis and common-mode behaviour are not claimed to reproduce a specific physical sensor until independently measured on #4753's fixture. The additional model-family campaign is intentionally a stress test of the current response basis, not a universal tactile sensor simulator.


## Independent reference-channel integrity tranche

The follow-on contract adds a second synthetic reference stream that is not used in model fitting. Qualified synthetic fits require both:
1. pairwise primary/check disagreement within the named synthetic-only tolerance (0.02 N in this executable fixture); and
2. an explicit synthetic reference-ancestry attestation indicating distinct calibration ancestry.

The report always states that pairwise agreement does **not** prove independence. A sixth response family injects the same 0.25 N offset into both reference streams. Its pairwise disagreement is small, but its ancestry attestation is false, so the qualified fitting path rejects it. A separate family biases only the primary stream; the numerical disagreement guard rejects it.

The adversarial campaign still executes the unguarded diagnostic fit to quantify the damage from ignoring a failed reference gate. These policy metrics are counterfactual diagnostics only and must not be treated as qualified measurements. The normal active-calibration benchmark and fit helper now enforce the reference gate before fitting.

The 0.02 N threshold and ancestry boolean are synthetic test-fixture parameters, not physical metrology thresholds or proof of actual instrument independence. Real qualification requires the #4753 owners to establish independent instrument identities, calibration ancestry, timebases, mounting/range applicability, uncertainty, and common-cause analysis.


## Reference integrity: two separate gates

The reference-channel prototype now requires both numeric agreement and an attested distinct calibration ancestry before the normal calibration-fitting path accepts a domain. The synthetic cross-check tolerance is 0.02 N and is named in code as a synthetic-only fixture value. It is not a physical metrology threshold.

A sixth adversarial family, shared reference common-mode bias, gives both numerical channels the same 0.25 N offset. Pairwise disagreement alone would accept it; the fixture therefore marks its ancestry attestation false, and the qualification path rejects it. The primary-only reference-bias family is rejected by a large pairwise difference. The report always states that numerical agreement does not prove independence.

The adversarial campaign retains explicit diagnostic-only functions that bypass reference qualification to measure the counterfactual error from using faulty data. They are isolated from the normal active calibration path and never qualify a measurement. The standard benchmark's source and target fits now require the integrity gate.

Real hardware still requires independent calibration ancestry, calibration validity/range, clock/time alignment, mounting/force-path analysis, uncertainty and common-cause review from the physical phantom/reference owner. A software ancestry boolean is not a calibration certificate.


## First hosted reference-integrity result

Hosted workflow #112 on source head `a431b6cf8d4688d0321e8769da1e0c18eeacbad3` passed 48 Rust tests, rustfmt, strict Clippy, the tracked 34-case transfer oracle, the 50-seed campaign and the six-family adversarial CLI. The subsequent workflow writeback commit `150237be23450c31078f3cc71fcea710a6142196` contains rustfmt-only changes to the source already tested.

Reference integrity results for 72 synthetic target calibration contacts (0.02 N synthetic-only tolerance):

| Response family | Max primary/check disagreement | Ancestry attested | Gate |
|---|---:|---:|---|
| Nominal | 0.004343 N | Yes | Pass |
| Cubic force | 0.004343 N | Yes | Pass |
| Force × temperature × history | 0.004343 N | Yes | Pass |
| Sensor saturation | 0.004343 N | Yes | Pass |
| Primary-only 0.25 N reference bias | 0.254343 N | Yes | Reject |
| Shared 0.25 N reference bias | 0.004343 N | No | Reject |

The last row is the critical common-cause result: both numeric streams agree because both share the injected 0.25 N offset. Pairwise agreement cannot detect that. The separate synthetic ancestry attestation rejects the scenario, and the output explicitly states that agreement does not prove independence.

The 48-test hosted pass belongs to source head `a431b6cf...`. Follow-up commit `150237be...` is formatting-only. The current branch/PR head may move again when a later hosted run writes generated formatting; do not silently treat an action-required bot event as a new test pass.

## New outstanding issue: range observability

The saturation family shows that a reference chain can be internally consistent while the tactile transducer itself has stopped responding at high loads. An independent reference channel does not fix loss of sensor information. This is now tracked separately under PHRI-TAC-OBSERVABILITY-001 / #5616. The future gate must detect flatlined response increments, clipped taxels and unidentifiable ranges and mark those operating points out-of-profile or more-evidence-required rather than interpreting model confidence as observability.


## Response observability guard — synthetic v0.1

The normal active-calibration path now checks raw-derived sensor response before policy selection/model fitting. Within each fixed position × temperature × loading-phase condition, candidates are sorted by commanded normal-force setpoint. Any adjacent force increase that does not produce a positive raw-response increment above the synthetic resolution fails the observability gate.

The checker also requires all six declared force setpoints (0, 2, 4, 6, 8, 10 N) in every one of the twelve synthetic condition strata. Missing force levels or missing conditions produce incomplete evidence instead of a pass. Reports include flatline/decreasing transition counts, affected conditions, first plateau start, maximum observed force, missing conditions and incomplete condition counts.

Tests cover nominal response, injected upper-range plateau, decreasing response, a truncated force range and a missing operating-condition stratum. The adversarial campaign reports observability per response family; diagnostic-only policy fits still execute under saturation to expose how models behave when the gate is ignored. The ordinary active-calibration runner uses the guard and refuses the fit if it fails.

The 1e-12 raw-response resolution, six-point force manifest and twelve-condition manifest are synthetic fixture declarations only. They are not physical ADC resolution, force range or a calibration tolerance. A monotonic response test is not a complete observability proof for real multi-axis, nonlinear, hysteretic or mechanically coupled tactile systems.
