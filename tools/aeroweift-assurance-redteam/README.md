# AeroWeft Assurance Argument Red-Team Oracle

A synthetic-first, seeded challenge harness that attacks the *real* Track W assurance-case types and evaluator. It applies domain-aware mutations, compares results with a separately implemented policy-invariant oracle, classifies confirmed counterexamples vs missed/conditional outcomes, and greedily minimizes reproducible witnesses.

The oracle checks a declared finite invariant profile; it cannot discover every physical hazard or prove completeness of the universe. `NO_COUNTEREXAMPLE_FOUND_WITHIN_BUDGET` is not a pass, a safety proof, or an authorization. A confirmed result means a seeded invariant was independently violated and Track W failed to block it; a W rejection without an oracle invariant is reported separately as overblocking/disagreement.

This crate does not run aircraft, calculate acceptable risk, create flight authority, authenticate source evidence, or replace Symtropy's differential fuzzer (#4446), common-cause analysis (#5981), independent physics oracles (#3749), or expert/authority review.
