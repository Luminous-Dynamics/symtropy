# Probability-based conformity risk reference

This standalone Rust reference demonstrates exact finite-support calculation of consumer and producer risk for an explicit process population and measurement likelihood. All values are synthetic. It does not infer a probability distribution from an expanded uncertainty value and it does not authorize product release.

## Model

- Eight true-value support points with an explicit process prior.
- An 8×8 row-stochastic likelihood matrix P(measured bin | true value).
- Closed specification interval [10,20].
- Approved synthetic thresholds: maximum specific consumer risk 0.15, maximum specific producer risk 0.25, maximum global consumer risk 0.05 and maximum global producer risk 0.05.

The measurement matrix contains non-zero tails. This ensures that false acceptance and false rejection risks are measurable.

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
