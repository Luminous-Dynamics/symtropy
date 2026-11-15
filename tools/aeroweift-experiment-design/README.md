# AeroWeft Validation Experiment Design Oracle

Synthetic-first Rust selector for aviation model-discrimination and validation experiment plans.

The selector applies hard admissibility gates before Pareto ranking. Every candidate binds an exact campaign, model/sensor/control scope, intended-use profile, experiment design, scenario manifest and measurement plan. Affirmative safety, envelope, independent-reference, semantic-equivalence and authorization gates require owner receipts. Controlled flight always requires external authorization; the frozen campaign policy can require it for HIL or other modalities too.

Utility inputs are conservative lower bounds for decision-relevant uncertainty reduction, model discrimination, worst-case robust reduction and decision relevance, plus an upper bound on uninformative probability. The estimator build, estimation method and confidence level are pinned by policy. The selector does not calculate or authenticate the estimates.

Costs remain separate: wall-clock minutes, setup, facility/operator, CPU, accelerator, energy and adjudication. A Pareto frontier is returned instead of a weighted score or single universal winner. Adaptive selection is restricted to development data; confirmatory and challenge holdouts remain sealed.

This tool does not authorize or execute tests, control aircraft, run simulators, establish physical model validity, authenticate provenance, prove realized information gain, or certify operations. Owner authority, safety, reference and semantic-equivalence receipts are input facts; this crate checks their declared structure, not their authenticity.

It composes AeroWeft Tracks T/U and Symtropy #4487/#460/#4366/#3749/#4446. It does not create a new simulator or physics qualification corpus.
