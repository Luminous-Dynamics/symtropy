# Symtropy physics multi-fidelity policy — V0

A small, isolated Rust research tool for deterministic admission and escalation among a finite set of already-declared physics model candidates.

## Run

From this directory:

```sh
cargo test
cargo run -- fixtures/rigid-contact-known-answer.json
```

The CLI reads one JSON manifest (or standard input) and emits a deterministic policy receipt. It does **not** load or execute simulator plugins.

## What it decides

A candidate is admissible only when all gates pass:

1. Semantic equivalence is explicitly `matched`.
2. Evidence satisfies the declared decision-risk tier:
   - `exploratory`: synthetic-only, analytic or independently validated evidence is allowed; unqualified evidence is never allowed.
   - `engineering_decision`: analytic oracle or independently validated evidence is required.
   - `safety_critical`: independently validated evidence, an analytic anchor digest and at least two distinct independent validation receipt digests are required. This is a conservative prototype policy, not a numerical requirement prescribed by ASME/NIST.
3. The declared validity envelope contains the scenario and horizon.
4. Hard constraints are explicitly `within_bounds`.
5. Residual and uncertainty upper bounds satisfy the requested limits.
6. p95 wall time and peak memory fit the declared budgets.
7. Source, capability, mapping and model-passport digests are present and well-formed.

Among admissible candidates, the tool chooses lexicographically by p95 wall time, peak memory, and stable model ID. There is no scalar weighted score.

If no candidate is admissible, it abstains. Runtime observations bind the original input, selected model passport and observation receipt. Residual/uncertainty breaches *inside the declared validity envelope* may escalate to a pre-admitted peer. A hard-constraint violation, unknown hard constraint or validity-envelope exit immediately abstains; the old scenario receipt cannot authorize a new model for a changed or unsafe state. A fresh state-bound input is required before another model can be admitted. Tolerances are never relaxed.

## Important limits

This is a finite-input policy contract, not a physics solver, uncertainty estimator, credibility assessor, or physical truth oracle. Risk tiers are a declared prototype policy inspired by risk-proportional VVUQ practice; they do not claim to reproduce or certify compliance with an ASME or NIST standard. Its current model passport and capability digests are caller-supplied; V0 validates their syntax and binds them into the receipt, but does not authenticate or verify their external contents. The synthetic fixture is only a known-answer software test.

It does not establish:
- model validity or calibration;
- physical accuracy of Symtropy, Rapier, MuJoCo, Chrono or any other backend;
- correctness of an upstream uncertainty estimate;
- sim-to-real transfer;
- safety certification or authorization to operate physical equipment.

## Planned integration

The intended owners remain:
- #1472 for exact capability/model-path disclosure;
- #4366 for semantic equivalence admission;
- #3749 for analytic/manufactured oracles and V&V;
- #1172 for deterministic scenario and replay identity;
- #460 for the canonical physics qualification corpus;
- #4446 for counterexample generation, differential execution and minimized failures.

Do not create duplicate capability censuses, scenario identities or physics evidence ledgers here. The next step is an independently reviewed adapter from these existing authorities into this policy's input schema.
