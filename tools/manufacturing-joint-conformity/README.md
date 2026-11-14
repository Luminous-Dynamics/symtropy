# Dependence-robust joint manufacturing conformity

This standalone Rust reference bounds the probability that all required characteristics of one part conform, given only marginal conformance-probability intervals. It does not assume statistical independence.

## Fréchet bounds

For events A_i that characteristic i conforms, with exact marginal probabilities p_i:

    max(0, sum(p_i) - (n-1)) <= P(intersection A_i) <= min(p_i)

For marginal enclosures p_i in [l_i,u_i], a conservative joint enclosure is:

    joint_lower = max(0, sum(l_i) - (n-1))
    joint_upper = min(u_i)

The lower and upper bounds can be wide. The independence product is emitted only as a diagnostic comparator and is never used to decide.

## Decision rule

An externally approved part-level policy supplies a minimum joint conformance probability for ACCEPT and maximum joint conformance probability for REJECT. If the Fréchet enclosure does not meet either threshold, the result is INDETERMINATE.

Every characteristic is tied to the same part, lot, operation and policy context. The required-characteristics manifest must exactly match the marginal set. Each marginal keeps its own characteristic/measurand/unit/specification identity and source digest.

## Why the independence shortcut is dangerous

For five characteristics each with marginal conformance probability 0.9:
- Fréchet joint range without dependence evidence: [0.5, 0.9].
- Independence product (diagnostic only): 0.9^5 = 0.59049.

A policy that rejects at joint conformance probability <=0.60 would reject under the independence assumption, but the dependence-robust result is INDETERMINATE. Perfect positive dependence could produce joint conformity of 0.9.

## Build and run

    rustc --edition=2024 -D warnings --test tools/manufacturing-joint-conformity/joint.rs -o /tmp/mfg-joint-tests
    /tmp/mfg-joint-tests

    rustc --edition=2024 -D warnings tools/manufacturing-joint-conformity/joint.rs -o /tmp/mfg-joint
    /tmp/mfg-joint --verify
    /tmp/mfg-joint --csv

## Research ladder

1. Prove the Fréchet oracle against exhaustive binary joint-event atom constructions for small n.
2. Preserve Fréchet bounds as the assumption-free baseline.
3. Add a qualified copula / multivariate posterior only with explicit dependence evidence and independent validation.
4. Compare independence, comonotonic, countermonotonic, Gaussian copula, tail-dependent and shared process-bias cases.
5. Report part/lot-level false acceptance/rejection, uncertainty width and indeterminate rate.

## Claim and authority boundary

The reference verifies mathematical joint bounds over supplied marginal intervals. It does not validate marginal posterior calibration or actual process dependence, certify a part or lot, or authorize release. Production quality authority remains external.
