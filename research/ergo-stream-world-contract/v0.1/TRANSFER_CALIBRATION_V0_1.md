# ErgoStream transfer calibration and selective risk v0.1

**Implementation:** \`src/calibration.rs\`  
**Sidecar:** optional \`transfer_forecasts\` in \`schemas/agent_transfer_bundle_v0.schema.json\`  
**Example:** \`examples/agent_transfer_bundle_v0.example.json\`

## Why this capability matters

A transfer system should not merely emit a correspondence and wait for the evaluator to accept or reject it. It should forecast whether each declared claim will survive independent evaluation, make an explicit assertion/rejection/abstention decision, and be scored for calibration as well as correctness.

This creates a measurable separation between:
- proposing a structural or behavioural map;
- estimating the probability that the exact declared transfer claim qualifies;
- deciding whether confidence is high enough to make the claim;
- independent evaluator outcome.

The evaluator owns the labels, binning, thresholds and final scores. The learner does not get to tune the acceptance threshold on the confirmatory outcomes. Outcomes also carry an evaluator-assigned `evaluation_stratum_id`; it is not present in the candidate forecast. `evaluate_transfer_calibration_by_stratum` returns a deterministic per-stratum report after validating the complete cohort.

## Forecast contract

Each optional transfer-forecast item carries:
- a unique claim ID;
- claim type: structural correspondence, deterministic dynamics, simulation, bisimulation or mechanism transfer;
- probability of qualification in integer parts per million (0 to 1,000,000);
- decision: assert transfer, reject transfer or abstain.

The probability is about the exact claim type and scope, not a generic statement that "the analogy looks good." For example, a valid induced structural embedding does not automatically make a mechanism-transfer forecast positive.

The evaluator binds each claim ID to the exact source/target episodes, map kind, declared scopes and evidence-prefix digests. Duplicate/unknown claim IDs are rejected by the adapter before scoring. Evaluator outcomes remain outside the candidate bundle.

## Metrics

### Brier score

For forecast probabilities \(p_i \in [0,1]\) and binary evaluator outcomes \(y_i \in \{0,1\}\):

\[
\mathrm{Brier}=\frac{1}{N}\sum_{i=1}^{N}(p_i-y_i)^2
\]

Lower is better. The implementation represents this as an exact integer numerator and denominator using ppm probabilities.

### Log loss

\[
\mathrm{LogLoss}=-\frac{1}{N}\sum_i [y_i\log p_i+(1-y_i)\log(1-p_i)]
\]

Lower is better. The executable clips only exact 0/1 probabilities to one ppm away from the boundary for this diagnostic; it reports how many observations were clipped. Brier remains exact and unclipped.

### Fixed-bin expected calibration error

The implementation uses a fixed, evaluator-selected number of equal-width probability bins. For each bin it stores the count, positive count and exact probability sum. The ECE numerator is the sum of absolute differences between total forecast probability mass and observed positive mass within each bin. The denominator is \(N \times 1{,}000{,}000\).

ECE depends on binning and can hide within-bin errors. It is secondary to proper scoring rules and must be reported with the bin count. Never tune bins on the held-out confirmatory cohort.

### Uncertainty around empirical risk and coverage

Each selective-risk point also exposes a two-sided 95% Wilson interval for:
- coverage, with denominator = all scored claims;
- selective risk, with denominator = claims actually selected above the confidence threshold.

These are descriptive IID binomial reference intervals. They are not valid as-is for clustered experiments, repeated measures, overlapping windows or temporally dependent claims. A real campaign must identify the independent unit (campaign, world seed/family, episode cluster, site, or other common-cause unit) and use cluster-aware bootstrap or another justified dependence-aware method. Do not let a large number of correlated frames create artificial precision.

### Selective risk / coverage

For a decided forecast, confidence is the probability of its selected class:
- \`assert_transfer\`: confidence = \(p\);
- \`reject_transfer\`: confidence = \(1-p\);
- \`abstain\`: no claim; it is not counted as a negative prediction.

At each preregistered minimum-confidence threshold:
- **coverage** = decided claims above threshold / all claims;
- **selective risk** = incorrect selected claims / selected claims.

Report the full risk–coverage curve, not only one operating point. An abstention strategy is useful when it lowers risk while retaining enough coverage to be operationally meaningful.

## Executable known-answer behaviours

The Rust test suite currently checks:
1. perfectly extreme forecasts yield zero Brier and ECE;
2. a 95%-confident false transfer is heavily penalized;
3. abstention reduces coverage rather than becoming a false negative;
4. confidence in a rejection is \(1-p\), not \(p\);
5. missing, duplicate and out-of-range records are rejected;
6. duplicate/out-of-range confidence thresholds and invalid bin counts are rejected;
7. duplicate evaluator outcome IDs and orphan outcomes are rejected;
8. per-stratum calibration surfaces a shift-specific false-transfer failure that would be diluted by pooled metrics;
9. zero observed errors in a small selected sample still has a nonzero Wilson upper risk bound, and coverage/risk use distinct denominators.

## Benchmark protocol

Use the exact same forecast set for all map types and compare:
- confidence calibration by claim type (structure, dynamics, simulation, bisimulation, mechanism);
- Brier and log loss;
- reliability plots / fixed-bin ECE;
- risk–coverage curves and coverage at predeclared risk ceilings;
- false-unification rate among high-confidence assertions;
- abstention rate on non-identifiable and out-of-scope controls;
- calibration under representation-only shift, semantic/mechanism shift and combined shift;
- calibration drift from source to target domain;
- compute, evidence/intervention count and wall time per qualified claim.

Minimum controls:
- a constant base-rate forecast;
- an empirical-frequency or isotonic/Platt calibration baseline trained only on development data;
- a raw confidence baseline from the strongest matched graph/world-model system;
- the same learner with forecast calibration disabled;
- a reject-option baseline using fixed confidence thresholds.

Do not use the confirmatory set to fit calibration, choose thresholds or tune bins. Keep an adaptive development cohort separate from a sealed confirmatory cohort.

## Split-conformal binary transfer prediction sets

The calibration module now also provides fit_split_conformal_binary_transfer. It calibrates the nonconformity score for the exact binary outcome "this scoped transfer claim qualifies":

- for a true transfer calibration example, score = 1 - p_transfer;
- for a false transfer example, score = p_transfer.

The finite-sample order statistic uses rank ceil((n+1)(1-alpha)). If that rank is n+1, the quantile uses the conservative full-scale fallback, yielding the full two-label set. At prediction time it includes transfer and/or non-transfer labels when their nonconformity scores are below the calibrated quantile.

Disposition is set-valued:
- transfer only: singleton AssertTransfer;
- non-transfer only: singleton RejectTransfer;
- both labels: ambiguous / abstain;
- neither label: empty set / no decision (do not force a class).

Both-label and empty prediction sets are retained as distinct outcomes. A tie at the calibrated quantile is included (`score <= q`), not discarded.

The calibration function requires one evaluator-owned stratum and matched forecast/outcome IDs. It does not pool across strata. The predictor, preprocessing, probability forecast and nonconformity mapping must be frozen independently of the calibration outcomes; the calibration cohort must not be used to fit or select that predictor. Under exchangeability between the independent calibration cohort and future claims within that predeclared admitted stratum, this gives a finite-sample **marginal** coverage guarantee for the prediction set at the requested alpha. It does not give arbitrary conditional coverage, cross-stratum coverage, robustness to arbitrary shift, or a sequential online guarantee. The learner must not choose the calibration stratum after observing outcomes.

## Owner-issued calibration receipt

The quantile is not a free-floating number. The owner-side receipt schema is:
- `schemas/split_conformal_transfer_receipt_v0.schema.json`
- example: `examples/split_conformal_transfer_receipt_v0.example.json`

The receipt binds the split-conformal calibration artifact to:
- predictor, preprocessing and score-function digests;
- exact calibration cohort, sample-ID set, split manifest and evidence-prefix digests;
- evaluator-assigned stratum, independent-unit kind and independent-unit count;
- exact transfer-claim scope digest;
- claim type and claim-scope digest;
- alpha, calibration count, finite-sample rank, quantile and tie rule;
- episode-owner custody receipt;
- active/revoked/expired state, with a revocation reason required when revoked.

The receipt is owner-issued, not candidate-authored. Owner-side semantic validation must verify model_frozen_at < calibration_started_at < issued_at, quantile_rank <= calibration_count + 1, the exact rank formula from alpha/count, and consistency of the sample-ID digest/count with the sealed calibration manifest. A change to predictor, preprocessing, score definition, calibration cohort, stratum or scope creates a different receipt and invalidates the old binding. Digests identify artifacts; they do not by themselves prove truthful acquisition, secure custody or exchangeability.

## Prospective internal targets

These are proposed acceptance targets, not achieved results:
- Brier score at least 10% lower than the strongest matched uncalibrated baseline on the sealed cohort;
- zero high-confidence false mechanism-transfer assertions on the finite adversarial controls;
- at a fixed 90% claim-coverage operating point, selective risk lower than the strongest matched baseline;
- for non-identifiable cases, either abstain or retain explicit alternatives—never invent a unique answer;
- calibration does not collapse under representation-only relabeling or time-scale transformations;
- report calibration separately for semantic/mechanism shift and format shift.

The exact risk ceiling, sample size and confidence intervals must be preregistered for each domain. A zero-error result on a small synthetic fixture is not evidence of a zero real-world risk.

## Integration boundary

- #7173 owns the finite calibration metric and candidate forecast sidecar.
- #7193 owns paired-world transfer outcomes and the Transfer Atlas.
- #5990 owns evaluator-side episode custody and hidden labels.
- #11171 owns the production research-policy adapter.
- #5476/#5478/#8721 own prospective value-of-computation and metareasoning.
- #11060 remains the blind AgentView/AgentAction episode protocol.

The Rust module computes metrics only. It does not calibrate a live learner, select the operating threshold, authorize transfer, or execute actions. The learner submits forecasts before hidden target outcomes are revealed; the evaluator computes scores afterward.

## Reproduction

\`\`\`sh
cargo fmt --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml -- --check
cargo test --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml
cargo clippy --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml --all-targets -- -D warnings
\`\`\`
