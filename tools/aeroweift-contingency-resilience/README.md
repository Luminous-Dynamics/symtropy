# AeroWeft Contingency Option Resilience Oracle

A dependency-free Rust evaluator for synthetic, evidence-qualified recovery-option portfolios.

The evaluator checks exact focal-flight scope, model applicability, owner-supplied evidence and authority states, evidence expiry, failed dependency domains, conservative response/energy/containment/separation/ground-risk bounds, option-inventory completeness, and a deterministic pairwise-independent witness under explicit common-mode exemptions.

For up to 24 admissible options, the evaluator computes an exact maximum independent set with deterministic tie-breaking. Above the frozen cap it returns a deterministic greedy lower-bound witness, marks it non-exact and abstains when the required diversity is not proven. Empty post-exemption dependency sets never count as independent. The cap is explicit because maximum independent set is NP-hard.

This crate does not simulate aircraft, compute physical risk, authenticate receipt bytes, authorize a flight, issue commands, certify an aircraft or declare a recovery option safe to execute. All tests use fictional data.
