# SIM-REL-018 adaptive scientific experiment benchmark generator

This deterministic generator uses a versioned task grammar and recorded seeds. It admits a task only after the finite stochastic kernel validates, the exact finite-horizon optimizer returns a non-empty frontier, the strict feasibility condition matches the task family, the one-step EIG baseline scores every action, and the all-horizon identifiability analyzer proves the expected mechanism relationships.

Initial families:

- Sensor/physical confounding: a persistent sensor offset cancels a specimen-state difference; repeated reads are not IID evidence, while calibrate-then-measure identifies the mechanism with zero false-claim risk.
- Noisy sensor/physical confounding: calibration removes the persistent offset, but the subsequent measurement remains stochastic. The exact optimizer must respect a declared 1/4 false-identification budget; one-step EIG remains zero before calibration.
- Physical carryover: conditioning changes the process state without immediate information; condition-then-read is identifying.
- Non-identifiable control: hidden physical and sensor state differ, but all available actions emit the same output law. An all-horizon equivalence certificate is required, and no strict exact-claim policy may exist.

A suite is deterministic from the generator version, root seed, and task count per family. Each task records its seed, family, exact frontier cardinality, strict-feasibility policy/cost, one-step baseline ranking, and equivalence-class/certificate summary. The receipt verifier recomputes the optimizer, baseline, and identifiability result.

The initial grammar is intentionally small. New task families should be added only with explicit solvability or non-identifiability contracts, independent negative controls, and matched baseline semantics. Task count alone is not benchmark progress; novelty, causal failure-mode coverage, diversity, and exact verifiability matter.

Qualification pins Rust 1.96.0, checks format, locked tests, strict Clippy, exact checkout identity, and a clean tree.

These tasks are finite synthetic models. Passing them does not establish physical validity, transfer to real apparatus, or superiority to current experimental-design systems.


## Capability-discriminating selection

The `selection` module converts method-run results into a deterministic task shortlist. Every candidate task is re-verified against the exact optimizer, EIG baseline and identifiability receipt. Every task must have one result from the same complete method roster.

A task is ranked by the number of method pairs separated either by risk-budget compliance or by an exact regret gap above a declared rational threshold. Ties are broken deterministically by regret spread, number of risk-compliant methods, exact frontier size, family and task ID. Selection enforces a maximum task count and per-family cap, and reports tasks no method solved under its declared budgets rather than silently including them as successful challenges.

This is a first deterministic selector, not a statistically complete adaptive-benchmark framework. Paired-seed confidence intervals, holdout sealing, task-family novelty and external baselines still need a separate evaluation protocol.
