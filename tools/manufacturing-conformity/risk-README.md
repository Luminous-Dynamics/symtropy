# Probability-based conformity risk reference

This standalone Rust reference is an exact finite-support global operating-characteristic oracle. It evaluates the joint distribution induced by an explicit process population and conditional measurement likelihood, computes per-observation posterior conformance, and integrates global consumer/producer risk over the policy's accepted/rejected regions. It is intended as an independent small-state mathematical oracle for the per-item histogram posterior path in Symtropy PR #3008, not as a second production posterior representation. All values are synthetic. It does not infer a probability distribution from expanded uncertainty and it does not authorize product release.

## Model

- Eight true-value support points with an explicit process prior.
- An 8×8 row-stochastic likelihood matrix P(measured bin | true value).
- Closed specification interval [10,20].
- Approved synthetic thresholds: maximum specific consumer risk 0.15, maximum specific producer risk 0.25, maximum global consumer risk 0.05 and maximum global producer risk 0.05.

The measurement matrix contains non-zero tails. This ensures that false acceptance and false rejection risks are measurable.

The frozen input manifest binds scenario ID `SYNTHETIC-METROLOGY-PRIOR-SHIFT-01`, specification `SYNTHETIC-SHAFT-10-20`, and separate SHA-256 digests for the process prior (`fa3fc7d7…`), likelihood (`7473cde9…`), risk policy (`0eb1be66…`) and specification (`cde0e2b5…`). The digests use compact UTF-8 JSON with array order preserved and object keys sorted lexicographically. The exact canonical input strings are: prior `[0.05,0.1,0.1,0.2,0.2,0.15,0.1,0.1]`; likelihood `[[0.6,0.25,0.1,0.04,0.01,0,0,0],[0.25,0.4,0.22,0.1,0.03,0,0,0],[0.1,0.22,0.36,0.22,0.08,0.02,0,0],[0.04,0.1,0.22,0.3,0.22,0.1,0.02,0],[0.01,0.04,0.1,0.22,0.3,0.22,0.1,0.01],[0,0.02,0.08,0.2,0.3,0.22,0.13,0.05],[0,0,0.04,0.1,0.22,0.3,0.24,0.1],[0,0,0.01,0.04,0.1,0.25,0.3,0.3]]`; risk policy `{"max_global_consumer_risk":0.05,"max_global_producer_risk":0.05,"max_specific_consumer_risk":0.15,"max_specific_producer_risk":0.25}`; specification `{"id":"SYNTHETIC-SHAFT-10-20","inclusive":true,"lower":10.0,"unit":"mm","upper":20.0}`. The CSV emits both the true-support population and the measured-bin decision/risk rows so the global calculation can be reconstructed.

## Calculations

For each measured bin j:

    q_j = sum_i p_i L_ij
    pc_j = sum_(i conforming) p_i L_ij / q_j

The policy is:
- ACCEPT when 1-pc_j <= max specific consumer risk.
- REJECT when pc_j <= max specific producer risk.
- INDETERMINATE otherwise.

The exact global risks are:

    global consumer risk = sum_(accepted j, nonconforming i) p_i L_ij
    global producer risk = sum_(rejected j, conforming i) p_i L_ij

The outcome masses for accepted, rejected and indeterminate measured bins are reported separately and must sum to one. Also reported are false-accept fraction among accepted observations and false-reject fraction among rejected observations.

## Frozen synthetic fixture

The synthetic policy yields:
- global consumer risk: 0.026;
- global producer risk: 0.0095;
- accepted observation mass: 0.182;
- rejected observation mass: 0.0495;
- indeterminate observation mass: 0.7685.

This deliberately conservative policy leaves most observations indeterminate. It illustrates the risk/coverage trade-off; it is not a target for real manufacturing.

## Standards context

JCGM 106:2012 distinguishes specific and global consumer/producer risks. Global risk depends on both the production population and the conditional measurement system, and reducing one risk generally increases the other. Eurachem/CITAC 2021 also discusses explicit guard bands, asymmetric distributions and indeterminate/non-binary decision rules.

The v0.1 oracle uses exact finite sums, not continuous PDF integration. A future continuous implementation must use an independently validated quadrature method with an error enclosure; if that enclosure crosses a risk threshold, the correct result is INDETERMINATE.

## Build and run

    rustc --edition=2024 -D warnings --test tools/manufacturing-conformity/risk_oracle.rs -o /tmp/conformity-risk-tests
    /tmp/conformity-risk-tests

    rustc --edition=2024 -D warnings tools/manufacturing-conformity/risk_oracle.rs -o /tmp/conformity-risk
    /tmp/conformity-risk --verify
    /tmp/conformity-risk --csv

## Claim ceiling

This is an exact synthetic finite-support risk oracle. It does not establish representativeness of the process prior, calibration of the measurement likelihood, actual factory risk, product conformance or release authority. Production claims require independent MSA, calibration, representative lot-level data, held-out risk calibration and authorized quality policy.
