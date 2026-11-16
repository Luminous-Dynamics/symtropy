# AeroWeft Contingency Option Resilience Oracle

A dependency-free Rust evaluator for synthetic, evidence-qualified recovery-option portfolios.

The evaluator checks exact focal-flight scope, model applicability, owner-supplied evidence and authority states, evidence expiry, failed dependency domains, conservative response/energy/containment/separation/ground-risk bounds, option-inventory completeness, and a deterministic pairwise-independent witness under explicit common-mode exemptions.

The independent-option witness is a proven lower bound, not a maximum independent set. If the witness does not prove the required option count, V0 abstains unless it can establish the special two-option all-correlated case from a complete inventory.

This crate does not simulate aircraft, compute physical risk, authenticate receipt bytes, authorize a flight, issue commands, certify an aircraft or declare a recovery option safe to execute. All tests use fictional data.
