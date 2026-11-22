# Navigation mission-level integrity risk contract v0.1

**Schema:** `mission_risk_contract.schema.json` (JSON Schema Draft 2020-12)
**Example:** `examples/urbannav_medium_research_only.json`
**Validator:** `nav-mission-risk-validate <schema.json> <contract.json>`

## Purpose

The mission-risk allocator divides a finite mission risk budget across a predeclared family of discrete integrity checks. This contract binds each allocation to a named failure event and a declared bound-evidence status. It prevents a mission from being labelled `qualified` while any check remains unqualified, any bound exceeds its allocated share, the check family is unfrozen, or key evidence/assumption receipts are absent.

## Exact risk arithmetic

All probabilities are represented as unsigned integer quanta at a fixed scale:

`1 quantum = 1e-15 probability`

`mission_budget_quanta = mission_probability * 1e15`

`sum(check.allocation_quanta) <= mission_budget_quanta`

`check.bound_evidence.upper_bound_quanta <= check.allocation_quanta`

This avoids floating-point accumulation when checking the budget. A `1e-3` mission budget is `1_000_000_000_000` quanta. A `0.95` confidence level is `950_000_000_000_000` quanta.

Unused mission budget is permitted and reported as `unallocated_quanta`; the validator does not redistribute it automatically.

## Required mission scope

Each contract names the mission unit, operational-domain reference, time-basis reference and truth/oracle scope. All qualified per-check evidence must name the same operational-domain reference. This does not establish that the referenced ODD or oracle is correct; it makes disagreement visible and machine-checkable.

## Per-check bound evidence

Each check has a unique `check_id`, a human-readable failure-event definition and an integer risk allocation. Bound evidence is either:

- `unqualified`: all result, confidence, independent-unit and evidence-receipt fields are null; review booleans are false. This is the correct status for the current UrbanNav example.
- `qualified`: a method, upper bound, confidence level, independent-unit definition/count, independence-basis reference, ODD reference, method-definition reference, calibration-split SHA-256, bound-result SHA-256, truth/oracle receipt SHA-256 and explicit assumption/truth review are required.

For `empirical_clopper_pearson`, the observed failure count is also required and cannot exceed the declared independent trial count. For non-binomial methods, `failure_count` must be null.

## Mission claim state

- `research_only`: the contract is descriptive; it makes no integrity claim.
- `candidate`: evidence is being assembled; it is not a qualified claim.
- `qualified`: the check set must be frozen and every declared check must have qualified bound evidence. The semantic validator also checks that the per-check bounds do not exceed their allocations and that allocations do not exceed the mission budget.

The status `qualified` means **structurally complete under this schema and validator only**. It is not an automatic safety certification. Human/independent review must verify receipt contents, statistical derivations, confidence calibration, independence, truth lineage, ODD coverage, completeness of the failure/check family and continuous-time behaviour between discrete checks.

## Digest and provenance limits

SHA-256 fields are format-checked as 64-character hexadecimal digests. This validator does not fetch or hash the external artifacts those fields identify, and it does not recompute `check_set_sha256` from a canonical representation of the check family. The evidence-production pipeline must define and implement that canonicalization before the digest is used as a cryptographic binding. A syntactically valid digest is not proof that a receipt exists or is correct.

## Statistical and temporal limits

- The per-check bound must be justified independently. Risk allocation alone does not produce a probability bound.
- Exact binomial intervals require independent, exchangeable, same-scope Bernoulli mission trials. High-rate epochs and overlapping windows are not mission trials.
- The union bound composes a finite predeclared set without requiring check independence, but it does not cover unobserved continuous time between checks.
- Continuous-time behaviour needs a separate dynamics/reachability argument, or a declared bound on error growth between checks.
- A frozen check list must be independently reviewed for hazard/check-family completeness; this validator detects duplicate IDs, not omitted hazards.

## Validation

The CLI runs JSON Schema Draft 2020-12 validation followed by semantic validation. The example is expected to pass both layers while reporting `structurally_complete=false`, `claim_status=research_only`, and zero qualified checks.

Successful validation does not prove physical validity, estimator calibration, statistical independence, data rights, ground-truth correctness or safety approval.