# CIRC-BUILD-003 — exact finite value-of-inspection benchmark v0.1

**Owner:** Symtropy  
**Status:** standalone synthetic reference model  
**Claim ceiling:** finite decision-theory contract only

## Paradigm shift

Do not ask whether a test is “accurate” in isolation. Ask whether its possible outcomes could improve the decision enough to justify the test cost under a declared model—and keep uncertainty, assumptions, evidence, and authority separate.

A test with better sensitivity can be worse economically if the additional information rarely changes the preferred action. A cheap but imperfect inspection may be valuable; an uninformative test may add cost without changing a decision. The result must be calculated from explicit prior weights, state-conditional losses, likelihoods, and intervention options, not inferred from a sensor score.

## Frozen finite model

- Hidden states: serviceable / degraded, with explicit prior weights 7 and 3.
- Actions: continue, repair, replace, each with a declared loss in each state.
- Observation outcomes: alarm / clear.
- Three synthetic inspection profiles: low-cost imperfect, higher-accuracy but expensive, and uninformative.
- Integer likelihood denominator 100 and exact integer loss numerators; no floating-point arithmetic.

The common score denominator is the sum of prior weights multiplied by the declared likelihood denominator. No-test and test-conditioned policies are therefore directly comparable. Inspection cost is applied to the same full denominator.

## Independent oracles

The production selection computes the minimum-loss action for each observable outcome. The reference oracle exhaustively enumerates every action assignment across the outcome set and independently picks the lowest total loss. The fixture pins exact policy choices and all expected numerators.

The perfect-information lower bound is computed by allowing the best action separately in each hidden state before observing anything. Gross EVSI cannot exceed the maximum gross value of perfect information. Net EVSI subtracts the inspection cost and may be negative.

## Targets

These are synthetic acceptance targets, not empirical claims:

- Zero discrepancy with exhaustive enumeration for every finite inspection policy.
- Exact integer/rational scoring and common denominators.
- 100% match to frozen baseline, perfect-information bound, action choices and loss numerators.
- No test's gross EVSI may exceed gross value of perfect information.
- Unknown or unmodeled states, priors, likelihoods or action losses must never be silently dropped or filled with zero.
- The uninformative test has zero gross EVSI and negative net EVSI after its cost.
- Every report includes raw manifest SHA-256, source revision, tool version, assumptions and claim ceiling.

## Ownership

- **Symtropy:** finite loss calculation and exhaustive reference-policy verification.
- **Symthaea:** later ranks authorized information-action proposals through existing SE-FOSS-003F (#6203) only after prerequisites and exact-current FEP behavior qualify. This benchmark does not create a second scheduler and never authorizes a physical experiment.
- **Mycelix:** provenance, authority and accountable receipts if a real inspection protocol is later adopted.
- **Sol-Atlas:** spatial/facility/inspection applicability only when supported by source-scoped geographic evidence.

## Research anchors

- Dadfarnia, Sharp & Herrmann (2025), systematic review of condition-monitoring evaluation methods: https://doi.org/10.1016/j.jmsy.2025.06.015
- NIST publication page for the review: https://www.nist.gov/publications/comprehensive-evaluations-condition-monitoring-based-technologies-industrial
- Crespo del Castillo & Parlikad (2024), condition-monitoring investment under remaining-useful-life uncertainty: https://doi.org/10.1016/j.ifacol.2024.08.150
- Digital Product Passport for Remanufacturing controlled case study (2025): https://doi.org/10.1016/j.ifacol.2025.09.147

The likelihoods, priors and losses are synthetic. No field diagnostic accuracy, maintenance saving, real EVSI, or physical qualification is claimed.
