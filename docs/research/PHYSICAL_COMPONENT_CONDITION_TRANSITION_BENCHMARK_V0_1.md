---
title: Physical Component Condition-Transition Benchmark
version: 0.1
status: draft-research-contract
scope: evidence-bound damage, component condition, repair, deconstruction, and recovery continuity
owner: physics / structural-integrity / materials / lifecycle-assurance
related:
  - https://github.com/Luminous-Dynamics/symtropy/issues/4867
  - https://github.com/Luminous-Dynamics/symtropy/issues/2209
  - https://github.com/Luminous-Dynamics/symtropy/issues/4859
  - https://github.com/Luminous-Dynamics/symtropy/issues/4864
  - https://github.com/Luminous-Dynamics/symtropy/issues/4862
---

# Physical Component Condition-Transition Benchmark v0.1

## Objective

Create an independently checkable **physical component condition-transition benchmark** that connects admitted load/damage evidence to revisioned component condition, repair/rework, deconstruction, and recovered-material disposition.

This is a composition/continuity workstream under [PHYS-TRANSFORM-001 #2209](https://github.com/Luminous-Dynamics/symtropy/issues/2209). It does not create another fatigue model, load-history conditioner, fracture solver, material ledger, generic digital twin, or maintenance authority.

## Parentage and reuse

Compose existing owners without copying their algorithms:

- [#2209 PHYS-TRANSFORM-001](https://github.com/Luminous-Dynamics/symtropy/issues/2209) — physical component/connection/material events and deterministic transformations.
- [#4859 PHYS-DAMAGE-REFERENCE-001](https://github.com/Luminous-Dynamics/symtropy/issues/4859) — bounded S-N/Miner and Paris-law damage references.
- [#4864 PHYS-LOAD-HISTORY-001](https://github.com/Luminous-Dynamics/symtropy/issues/4864) — source-bound signal conditioning and cycle-count receipts.
- [#4862 PHYS-DAMAGE-DATA-001](https://github.com/Luminous-Dynamics/symtropy/issues/4862) — rights-aware, publication/specimen-grouped measured-data evaluation.
- [#1856 CIRC-BUILD-001](https://github.com/Luminous-Dynamics/symtropy/issues/1856) — intervention comparison and material-flow hard gates.
- Symthaea [ROB-LIFE-001 #4863](https://github.com/Luminous-Dynamics/symthaea/issues/4863) — robotics lifecycle consumer; Symthaea proposes and explains but does not mutate physical truth or authorize maintenance.

## The missing question

The current workstreams can separately describe a structure, count cycles, estimate a bounded damage quantity, or compare lifecycle interventions. They do not yet qualify the **continuity of one physical component's condition across all of those transitions**.

A valid damage estimate can still be attached to the wrong component/configuration, applied twice, applied after a newer inspection, or silently treated as a measured state. A repair can accidentally reset accumulated usage or erase unresolved damage. A mass-balanced salvage operation can still misclassify a recovered component as fit for reuse.

The benchmark must make those failure modes executable.

## Paradigm

Treat component condition as an append-only, revisioned, multi-mechanism state with explicit evidence and applicability—not a mutable scalar health score.

The lifecycle chain is:

```text
physical article + configuration + material lot
  -> source-bound load/observation history
  -> admitted damage evaluation
  -> revisioned condition event
  -> inspection / repair / replacement / reinforcement
  -> post-action condition and configuration verification
  -> deconstruction / recovered component or material lot
  -> new lineage, with previous history preserved
```

Every edge binds exact predecessor/successor identities, event identity, source/configuration digests, time semantics, evidence class, units, uncertainty and model applicability.

## P0 — typed condition and transition contract

Define a dependency-light schema/reference verifier (not a new physics engine) with:

1. **Subject identity:** physical article, component, assembly path, configuration revision, material/heat/lot identity and effectivity.
2. **Exposure lineage:** exact load-history ID/digest, channel, unit, frame/sign convention, clock basis, calibration, conditioning profile, gaps and cycle-count profile.
3. **Damage claim:** mechanism/model/profile identity, parameters and provenance, input receipt IDs, output quantity/unit, applicability interval, uncertainty and typed disposition (`Evaluable`, `OutOfDomain`, `Incomplete`, `Inconclusive`, `Invalid`). A model estimate remains a model estimate.
4. **Condition vector:** preserve mechanism-specific quantities and statuses (e.g. Miner model damage, crack length, corrosion loss, wear, deformation, inspection indication, unknown/unmodelled mechanism). Do not collapse unlike axes into one authoritative health scalar.
5. **Uncertainty taxonomy:** keep measurement, parameter, model-form, numerical, load-history/conditioning, boundary-condition and between-article variability distinct. Unknown dependence is not independent by default.
6. **Revisioned transition:** exact expected source revision, unique event ID, parent condition receipt, action, consumed/installed material lots, resulting configuration and postcondition references. Stale, duplicate, replayed, impossible or unauthorized transitions are atomic no-mutation failures.
7. **History preservation:** repair/rework does not reset prior usage, fatigue history or unresolved damage by default. Component replacement creates a new physical-article/configuration generation with explicit ancestry; only evidence-backed state fields may change.
8. **Recovery separation:** exact mass/quantity accounting remains with #2209 / existing material-flow owners. Reuse fitness, material grade, contamination and requalification are separate evidence claims; recovered mass is not automatically reusable supply.
9. **Decision boundary:** Symthaea may rank inspection/repair alternatives and explain uncertainty. Its proposal cannot write component state, override hard safety constraints, or authorize a maintenance/release action.

Missing, stale, revoked, mismatched, out-of-domain or unauthenticated evidence must remain visible and must not be silently promoted to a stronger evidence class.

## P1 — frozen synthetic composition campaign

Start with small independently checkable cases, then extend:

- **Clean exposure:** valid load history -> valid cycle receipt -> in-domain damage result -> one condition revision.
- **Duplicate application:** replay the same damage event; second application must be rejected without mutation or double accumulation.
- **Stale observation:** apply a result against an older component/configuration revision after a newer inspection; reject.
- **Wrong subject:** valid result for component A is attached to component B; reject.
- **Unit/frame mismatch:** stress/unit/sign/frame mismatch; reject or mark not evaluable.
- **Gap and clock fault:** no cycles are invented across missing or unordered history.
- **Out-of-domain:** stress/Delta-K outside declared calibration; abstain rather than extrapolate.
- **Repair without reset:** replace/repair a subcomponent while preserving parent history and unresolved condition fields.
- **Replacement:** new article/configuration generation; do not merge its usage with the predecessor as if it were the same physical specimen.
- **Inspection update:** measured indication updates only supported axes; model-derived values are not relabelled as measured.
- **Deconstruction/recovery:** topology and exact material quantities close; unknown contamination/grade leaves reuse disposition unresolved.
- **Common-cause uncertainty:** shared calibration/material-lot bias is not treated as independent evidence.
- **Fault injection:** duplicate IDs, stale signatures/authority, changed digests, invalid units, NaN/Inf, overflow and tampered parentage all fail closed.
- **Replay/permutation:** deterministic replay for identical ordered inputs; permutation invariance only where event semantics declare order irrelevant.

The evaluator must be separately authored from the production transition function and use frozen expected outcomes, mutation/negative controls and deterministic receipts.

## P2 — measured-data admission and validation

Only after P0/P1:

- Consume the rights-cleared source and specimen/publication grouping from #4862.
- Use the 26 October 2026 Strathclyde high-strength-steel S-N / crack-growth dataset only after workbook schema, source-row lineage, units, duplicates, test conditions and licence attribution are inspected.
- Keep NIST full-scale/large-component column-removal experiments as structural-response references where exact geometry, loading, boundary conditions and instrument outputs can be reconstructed; they are not fatigue-life datasets.
- Freeze grouped holdouts by specimen, publication/test campaign, material grade and environment. Never split adjacent points from one curve or adjacent time samples from one article across train/test.
- Separate equation verification, synthetic software tests, measured model validation, solver comparison and professional engineering approval.

## Baselines and comparative protocol

Compare the composition against matched alternatives, not an unequal feature checklist:

1. Static asset/material passport with manual condition updates.
2. Single scalar health score plus fixed threshold.
3. Damage model output without revision/identity/evidence continuity.
4. Conventional specialist fatigue workflow output where a licensed, model-equivalent reference can be configured.
5. A data-driven RUL baseline with the same training/measurement budget and leakage-safe holdout.
6. Proposed evidence-bound condition-transition composition.

Report per-axis results and hard-gate failures; do not hide failure behind a weighted average. Use matched inputs, fidelity, calibration data and time/compute/inspection budgets.

## Metrics and target gates

**Software/contract targets (synthetic):**
- 100% exact disposition on the frozen valid/invalid fixture matrix.
- Zero duplicate/stale/wrong-subject transitions accepted.
- Zero history resets or evidence-class promotions without the required event/measurement receipt.
- Zero invented cycles across gaps, missing units or invalid clocks.
- Exact material mass closure wherever quantities are fully declared; otherwise explicit incomplete/unaccounted state.
- Deterministic state replay and digest binding.
- 100% rejection/abstention on predeclared out-of-domain and unsupported-mechanism controls.

**Empirical/model targets (not preset as achieved or guaranteed):**
- Report grouped log-life / crack-growth error, bias, calibration and uncertainty coverage by material/grade/environment.
- Report RUL interval coverage and width, false-healthy rate, lead time, false-alarm burden and abstention/coverage frontier.
- Report intervention regret, first-pass repair success, retest failure, downtime, recovered usable mass (only when requalified), virgin material avoided, energy and lifecycle cost separately.
- Claim improvement only when the proposed composition improves a predeclared decision metric over the strongest matched baseline on held-out campaigns without degrading hard safety/evidence gates.

## Industry comparison and claim ceiling

Commercial Ansys nCode DesignLife and SIMULIA fe-safe provide much broader integrated fatigue workflows, including FEA coupling and advanced fatigue/damage methods. NAFEMS benchmarks and NIST structural experiments provide external verification/validation anchors. The proposed contribution is **not** replacing those solvers or claiming broader analysis capability. It is an inspectable, solver-independent continuity layer linking their admitted outputs to exact physical component identity, revisions, repairs and recovered material—while preserving uncertainty and refusing unsupported transitions.

This work does not certify a structure, predict universal service life, authorize repair/release/demolition, validate a material parameter from synthetic data, or establish physical truth from hashes. All safety-critical interventions remain subject to qualified engineering, inspection and authorized procedures.

## References

- NIST, *Performance of Steel Moment Connections under a Column Removal Scenario: Experiments* (2012): https://www.nist.gov/publications/performance-steel-moment-connections-under-column-removal-scenario-i-experiments
- NIST, *New Precast Concrete Moment Connections under a Column Removal Scenario* (2025): https://www.nist.gov/publications/new-precast-concrete-moment-connections-under-column-removal-scenario
- NASA Prognostics Data Repository, including fatigue crack growth in aluminum lap joints: https://www.nasa.gov/intelligent-systems-division/discovery-and-systems-health/pcoe/pcoe-data-set-repository/
- NAFEMS code verification: https://www.nafems.org/publications/code-verification/
- 2026 civil-infrastructure RUL review: https://link.springer.com/article/10.1007/s10462-026-11547-0
- 2026 BIM / infrastructure lifecycle interoperability review: https://www.mdpi.com/2076-3417/16/21/10749
- Ansys nCode DesignLife: https://ansys.synopsys.com/Products/Structures/Ansys-nCode-DesignLife
- SIMULIA fe-safe: https://www.3ds.com/products/simulia/fe-safe

## Definition of done

- The P0 schema and transition verifier have a standalone exact-source test harness.
- Frozen positive, negative, metamorphic and replay fixtures are checked by an independent oracle.
- A machine-readable report records denominators, all failures/abstentions, code/configuration/fixture digests and evidence classes.
- P1 composes with the exact contracts from #2209, #4859, #4864 and #4862 without copying their solver/accounting ownership.
- Measured validation is reported only after source admission and grouped holdouts; no CI success, synthetic pass or model score is represented as physical certification.

