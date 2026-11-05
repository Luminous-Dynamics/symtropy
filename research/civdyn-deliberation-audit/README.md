# CivDyn Deliberation Audit v0.1

A deterministic trace auditor for recorded group deliberation. It does not generate human behaviour, infer preferences from free text or define a universal measure of fairness.

## Recorded inputs

- Explicit participant identifiers, normalized stated preference weights and optional self-reported satisfaction/trust.
- Proposal IDs, authors and the option each proposal concerns.
- Observed turn events, speaking duration and proposal references.
- Facilitator summary proposal references.
- Facilitator claims with evidence IDs and a source-issued digest catalogue.
- Final integer allocation under a fixed resource budget.

## Reported outcomes

- Speaking-time Gini and turn-count Gini, plus member-level time/turn shares.
- Proposal authorship, proposals raised in turns, proposals represented in the facilitator summary, and suppression/omission.
- Evidence-backed versus unsupported facilitator claims.
- Declared consensus versus the fraction of participants with the same top stated preference.
- A deterministic preference-proportional baseline and final-allocation L1 shift. This is a descriptive counterfactual, not a welfare optimum or prediction of unguided human behaviour.
- Preference alignment, separately from self-reported satisfaction and trust changes.
- A paired facilitation-variant comparison that holds option set, budget, declared preferences and evidence catalogue fixed while measuring allocation sensitivity.
- Option-order sensitivity using stable semantic option IDs: preferences and proposal meanings are remapped across permutations, and allocation shifts are measured after mapping both results back to the same semantic option order. A pure reorder with unchanged semantic allocation must yield zero; a deliberate first-position steering control must be detected.


## Synthetic control

The executable emits an equal-treatment reference trace and a deliberately steered trace. In the steered case, one member's proposal is omitted from the facilitator summary, an unsupported evidence claim is introduced, the facilitator declares consensus despite divergent stated preferences, and trust self-reports increase while the allocation moves away from the deterministic preference-proportional baseline.

These are hand-authored software fixtures, not human-subject evidence or a reproduction of the FAccT 2026 study.

## Run

```sh
cargo fmt --manifest-path research/civdyn-deliberation-audit/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-deliberation-audit/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-deliberation-audit/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-deliberation-audit/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-deliberation-audit/Cargo.toml --bin deliberation-audit-sweep
```

## Claim ceiling and next gates

The v0.1 kernel measures properties of a supplied event trace. It cannot infer omitted turns, hidden power asymmetries, unrecorded interruptions, participant motivations, or whether the preference-proportional baseline is normatively correct.

A real evaluation requires source-exact study replication, consent and data rights, transparent event coding, blinded transcript annotation, inter-rater reliability, counterbalanced facilitator identity/order, and independent evaluation of factual provenance and appealability. Satisfaction/trust is self-report, not proof of procedural neutrality.


## Consensus semantics correction

Top-preference diversity is **not** evidence that a group failed to reach consensus. The audit reports modal top-preference share as a descriptive preference measure, but only flags a facilitator's consensus declaration as contradicted when an explicit participant-level non-endorsement is recorded. Missing endorsement data is counted as unknown, not dissent. This allows a group to reach an informed compromise despite different initial preferences.
