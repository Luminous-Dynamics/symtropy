# EV battery closed-loop qualification contract v0.1

Status: synthetic fixture and dependency-free read-only Rust reference audit. No physical cell, pack, manufacturing-yield, cycle-life, safety, cost or commercial improvement is claimed.

## Purpose

Verify that one exact battery design revision can be followed through:

mission-conditioned design → exact material/recipe/process-window identity → realized production runs → independent lot/line identities → cell test outcomes → exact pack membership → mission outcomes → downstream decision receipt.

A chain of individually plausible reports is not enough. Every edge must resolve to the same immutable configuration lineage. Ambiguous joins by chemistry name, cell format, approximate time or recipe label are not accepted.

## Canonical owners

- Symthaea BATTERY-INVERSE-DESIGN #11671 owns mission-conditioned candidate design and experiment selection.
- Symthaea BATTERY-DESIGN-LIFETIME-TRANSFER #11686 owns new-design lifetime transfer evaluation.
- Symthaea MAT-013 #4344 and MAT-PSPP #6224 own physical sample lineage and process–structure–property–performance semantics.
- Symthaea MAT-INTERFACE #6227 and MAT-UQ #6225 own interface identity, uncertainty and calibration.
- Symthaea EV-FIELD-EVIDENCE #11421 owns evidence rights by operation.
- Symtropy EV-BATTERY-LINEAGE-OUTCOME #4791 owns the EV-specific cell/pack/mission reference join.
- Symtropy EV-BATTERY-MFG-ROBUST #4833 owns lot/line and process-window conformance.
- Symtropy EV-BATTERY-ULTRASONIC-NDT #4840 owns sample-held-out ultrasonic electrode inspection.

This audit consumes those references. It creates no duplicate physics, metrology, genealogy, uncertainty, rights, process semantics or control authority.

## Evidence classes and outcomes

Synthetic fixture, assumption, vendor claim, literature result, simulation, surrogate, calculation, physical measurement and independent laboratory evidence remain distinct.

A physical cell outcome requires evaluation-admitted physical-measurement or independent-laboratory evidence with a metrology reference. A physical mission outcome has the same requirement. Synthetic passes/failures are useful for software known-answer tests but never count as physical results.

Incomplete and invalid measurements remain explicit. An out-of-window run is not silently discarded. A failed physical test remains a physical observation, not a successful cell.

## Identity and dependency rules

1. Candidate design, recipe revision and process-window revision are immutable and exact.
2. Every production run binds that design, recipe and process window plus cell, lot, line, equipment and its own digest.
3. A canonical lot or line lineage digest cannot be aliased under a second group identifier.
4. Every cell has one production run and one cell-test record in this v0.1 single-cell/single-test audit.
5. Every pack member is an exact tested cell with the same design revision.
6. Every mission outcome binds the exact pack revision, scenario and system boundary.
7. The decision receipt must enumerate exactly the evidence IDs and dependency digests used by the current graph. An upstream change makes the receipt stale.
8. No missing, censored, incomplete, invalid, blocked or unknown result is converted into a pass.

## Report semantics

The reference oracle reports run counts, in-window/out-of-window counts, unique physical lot/line identity counts, physical cell passes/failures, incomplete tests and simulated/physical mission outcomes.

It exposes whether the physical evidence chain is closed. Chain closure does not mean the design passed: a fully measured failed cell or out-of-window run is still a complete physical observation. Yield, confidence intervals, Cp/Cpk, causal effects and population claims belong to their qualified domain owners and require sufficient independent data.

## Benchmark programme

Compare nominal multi-objective ranking, bounded worst-case optimization, stochastic expected-value optimization, chance-constrained methods where independent groups support them, lower-tail/CVaR objectives, robust multi-objective Bayesian optimization, fixed-recipe/DOE baselines and exhaustive enumeration on tiny finite spaces.

Hold candidate spaces, information cutoffs, solver fidelity, compute and experiment budgets constant. Report end-to-end orphan/mismatch/stale-reference rates, lot/line group counts, false acceptance, abstention, worst-group outcomes, Pareto regret/hypervolume with frozen bounds, experiments and material/energy/time to first physically supported feasible design.

Do not optimize a single average while hiding the worst lot. Do not treat cells from one lot as independent manufacturing replications. Do not use a surrogate to certify a result from the same surrogate.

## First software acceptance

- Exact design/recipe/process-window and cell/pack/mission identity closure.
- Evidence admission and physical metrology gates.
- No duplicate lot/line aliasing.
- Exact pack-member/test closure.
- Exact decision evidence and dependency-digest receipt.
- Deterministic report under input permutation.
- Negative controls for stale, mismatched, unknown and incomplete evidence.

A passing audit establishes only the declared finite reference and evidence rules. It does not establish battery performance, process causality, manufacturing capability, safety, lifetime, commercial cost or product-release readiness.
