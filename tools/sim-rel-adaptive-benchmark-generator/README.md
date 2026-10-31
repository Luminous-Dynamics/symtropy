# SIM-REL-018 adaptive scientific experiment benchmark generator

This deterministic generator uses a versioned task grammar and recorded seeds. It admits a task only after the finite stochastic kernel validates, the exact finite-horizon optimizer returns a non-empty frontier, the strict feasibility condition matches the task family, the one-step EIG baseline scores every action, and the all-horizon identifiability analyzer proves the expected mechanism relationships.

Initial families:

- Sensor/physical confounding: a persistent sensor offset cancels a specimen-state difference; repeated reads are not IID evidence, while calibrate-then-measure identifies the mechanism.
- Physical carryover: conditioning changes the process state without immediate information; condition-then-read is identifying.
- Non-identifiable control: hidden physical and sensor state differ, but all available actions emit the same output law. An all-horizon equivalence certificate is required, and no strict exact-claim policy may exist.

A suite is deterministic from the generator version, root seed, and task count per family. Each task records its seed, family, exact frontier cardinality, strict-feasibility policy/cost, one-step baseline ranking, and equivalence-class/certificate summary. The receipt verifier recomputes the optimizer, baseline, and identifiability result.

The initial grammar is intentionally small. New task families should be added only with explicit solvability or non-identifiability contracts, independent negative controls, and matched baseline semantics. Task count alone is not benchmark progress; novelty, causal failure-mode coverage, diversity, and exact verifiability matter.

Qualification pins Rust 1.96.0, checks format, locked tests, strict Clippy, exact checkout identity, and a clean tree.

These tasks are finite synthetic models. Passing them does not establish physical validity, transfer to real apparatus, or superiority to current experimental-design systems.
