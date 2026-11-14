# AtomFab × Symtropy: Digital-Twin Contract

**Date:** 2026-10-19  
**Status:** Proposed integration contract; simulation/research only  
**Companion:** Symthaea AtomFab research track  
**Scope:** No laboratory hardware actuation is implemented or authorized by this document.

## Decision

Use Symthaea and Symtropy together, with strict division of responsibility:

- **Symthaea / AtomFab is the scientific decision layer.** It manages goals and constraints, candidate selection, atomistic-solver orchestration, uncertainty, evidence provenance, experiment selection, and explicit abstention.
- **Specialist atomistic engines are the atomistic physics layer.** DFT, molecular dynamics, and validated machine-learned interatomic potentials evaluate atomic structures and task-specific observables. Symtropy's rigid-body solver is not a substitute for electronic-structure or atomistic molecular simulation.
- **Symtropy is the manufacturing-workcell and process digital twin.** It simulates tools, manipulators, sample/stage fixtures, accessible work envelopes, collisions, actuator limits, sequence timing, calibration/drift scenarios, and failure/recovery at the apparatus/process level. It can render scenarios through Bevy, but a rendered scene alone is not validation.
- **Metrology is the evidence layer.** Real measurement records and calibration records remain distinct from simulation output. Later laboratory integration requires validated adapters, hardware interlocks, and qualified operator approval.
- **Shared contract is versioned and engine-neutral.** Begin with a serialized proposal/observation record, not a direct dependency between the two repositories' internal implementation types.

This architecture builds on capabilities present in Symtropy today: generic N-dimensional rigid-body geometry and collision simulation, deterministic single-thread replay on a given platform, Bevy presentation, and an existing Symtropy–Symthaea embodiment adapter. Those capabilities make Symtropy a plausible workcell twin; they do not establish atomistic predictive capability.

## Why both systems?

The key design principle is **one accountable owner per kind of truth**.

| Question | System of record | Explicit limitation |
|---|---|---|
| Which material / structure should be evaluated next? | Symthaea AtomFab planner | Candidate rankings need calibrated uncertainty and held-out validation |
| What do atomic calculations predict? | Pinned specialist solver plus its input/output manifest | A potential is only credible within demonstrated support; energy MAE alone is insufficient |
| Can the process sequence fit the workcell and equipment limits? | Symtropy twin and process constraints | Rigid-body feasibility does not prove chemistry or surface reaction feasibility |
| What was actually measured? | Metrology result with instrument, calibration, sample and timestamp provenance | A simulator-generated sensor value is never a physical observation |
| Should the next action proceed? | Symthaea decision policy over validated twin results and hard guards | The policy cannot bypass an interlock or operator approval |
| What did the whole campaign prove? | Evidence ledger containing all layers' references and hashes | Reproducibility of software is not proof of model-to-reality validity |

### A crucial boundary

Do **not** simulate bonding, electronic structure, chemical reactions, surface diffusion, or atomistic temperature effects using Symtropy's rigid-body engine and then present them as atomistic truth. Use a domain-appropriate solver or measured calibration for those phenomena. Symtropy can simulate the instrument and its motion around the sample, while atomistic calculations estimate what a tip/sample action might do. The coupling between those levels needs its own validation.

## Co-design and multi-fidelity routing

The joint architecture should choose a *manufacturing route*, not merely rank atomic structures. Candidate selection should account for the usable process window, tooling reachability, equipment limits, yield risk, material/resource cost, energy demand, uncertainty, repairability, and end-of-life recovery. These dimensions form a Pareto frontier; hard safety and feasibility constraints remain non-negotiable and must not be softened into weighted objectives.

Route each question to the lowest-cost adequate source of evidence, with escalation when uncertainty or decision value warrants it:

1. Static constraints and material/process rules.
2. Cheap atomistic or empirical candidate screening inside its declared validity domain.
3. Higher-fidelity electronic-structure / reaction-pathway evaluation for finalists or disagreements.
4. Symtropy workcell simulation for apparatus geometry, reachability, collision risk, timing, fault response and replay.
5. Calibrated metrology for physical claims.

These steps are not one scalar ladder. Atomistic feasibility and workcell feasibility are separate branches that meet at the proposed process. A green result from one does not compensate for an unknown or failing result in the other. Record each model's residual and domain of applicability. Use targeted counterexample searches to challenge the leading candidate rather than repeatedly selecting only the highest predicted score.

## Contract v0.1: proposal → twin → observation → decision

The companion Symthaea draft includes a strict JSON Schema v1, serde-backed Rust record types, semantic validation, and a shared valid process-proposal fixture for review: [schema](https://github.com/Luminous-Dynamics/symthaea/blob/research/atomfab-evidence-benchmarks-2026-10-19/crates/domains/symthaea-atomfab/schemas/contract-v1.schema.json) and [valid fixture](https://github.com/Luminous-Dynamics/symthaea/blob/research/atomfab-evidence-benchmarks-2026-10-19/crates/domains/symthaea-atomfab/fixtures/contract-v1-process-proposal.json) plus [negative fixture](https://github.com/Luminous-Dynamics/symthaea/blob/research/atomfab-evidence-benchmarks-2026-10-19/crates/domains/symthaea-atomfab/fixtures/invalid/contract-v1-no-abort-guard.json). The cross-repository contract remains proposed until both PRs are reviewed and the same fixtures are parsed by both sides. Every message must include:

- **schema_version**: exact contract version.
- **run_id**, **scenario_id**, and monotonic **sequence**: replayable identity, not free-form labels.
- **parent_digest**: digest of the proposal or observation being answered.
- **created_at**: RFC 3339 wall-clock record-creation timestamp. **clock_domain** labels the time base of timing quantities in the payload (simulation, monotonic process, or wall-clock); it does not change the meaning of created_at.
- **units**: explicit unit labels for every physical quantity; adapters should canonicalize interfaces to SI or record a versioned conversion, and no implicit conversion is permitted.
- **source_kind**: **proposed**, **predicted**, **simulated**, **measured**, or **independently_reproduced**.
- **producer**: crate / adapter, version, source revision, and configuration digest.
- **uncertainty**: uncertainty method, value/interval, domain of validity, and whether calibrated on the applicable split.
- **evidence_refs**: stable dataset/sample/reference IDs plus content digests; avoid copying large trajectories into every message.
- **limitations**: machine-readable abstention / out-of-domain / missing-input reasons.

### Message types

1. **ManufacturingIntent** — desired observable ranges, objective weights, hard/soft constraints, material/structure family, prohibited conditions, and acceptance test.
2. **AtomisticEvaluation** — structure fingerprint, composition, solver and reference method, energy/forces/stress units, selected observables, uncertainty, convergence status, and input/output digests.
3. **ProcessProposal** — ordered simulation actions, required equipment capabilities, work envelope, environmental preconditions, expected sensor signature, abort guards, and explicit unknowns.
4. **TwinEvaluation** — whether all hard constraints were satisfied, predicted geometry/contacts/timing/resource usage, injected faults, replay identity, residuals, and simulator limitations.
5. **MeasurementRecord** — real instrument identity, calibration state, sample identity, acquisition settings, raw-data digest, derived value, uncertainty, operator / approval provenance, and timestamp.
6. **DecisionRecord** — accept/reject/abstain/re-evaluate decision, matched proposal digest, reasons, constraint results, and next information-seeking action.

Unknown enum values, schema versions, units, source kinds, missing critical digests, non-finite values, stale state, or a missing required safety guard must fail closed. No implicit unit conversion and no silently defaulted uncertainty.

## Closed-loop workflow

1. Symthaea receives a goal and hard constraints, selects a candidate, and records why it was selected.
2. A pinned atomistic solver evaluates the structure, relevant reactions, and uncertainty.
3. Symthaea proposes a manufacturing sequence compatible with the target.
4. Symtropy simulates apparatus geometry, motion, timing, and injected faults.
5. Validators check hard bounds, uncertainty, and each model's applicability domain.
6. In simulation-only mode, the result updates the candidate or process plan and is recorded as evidence.
7. If a future hardware-enabled phase is approved, a separate operator and independent interlock gate is required before any physical action.
8. Metrology records what actually happened; Symthaea compares predictions to the measured outcome and updates the evidence ledger.

Symtropy must return a structured **rejection trace** for infeasible proposals. Symthaea must treat **rejected**, **unknown**, and **out_of_domain** as distinct outcomes, not as poor numerical scores that can be averaged away.

## First implementation slice (no runners or hardware needed)

### Slice A — Contract and fixtures

- Freeze one minimal schema version and explicit unit conventions; choose canonical SI at each interface or record a versioned conversion.
- Add valid fixtures for a process proposal, twin evaluation, a physical measurement record, and a rejected proposal.
- Add negative fixtures: missing units, NaN/infinity, unknown source kind, digest mismatch, stale sequence, violated hard constraint, missing calibration, and absent abort guards. A first no-abort-guard negative fixture is already authored on the companion AtomFab branch.
- Verify that both repos parse/validate the same fixtures; test JSON Schema conformance and Rust semantic validation separately.
- Use deterministic fixture files and content digests; keep experiment-specific data outside source code.

### Slice B — Symtropy workcell scenario

Build a minimal, inspectable *surrogate* workcell with a stage, tool carriage, keep-out regions, travel limits, and a sample holder. Inject bounded drift, sensor dropout, collision/near-collision, actuator saturation, and stale observation cases. These are apparatus-level surrogate dynamics, not atomistic simulation.

The scenario should use the existing physics/replay interfaces only where they provide the needed semantics. If collision or control behavior does not match the scenario's requirements, add a narrowly scoped primitive with its own tests instead of overclaiming fidelity.

### Slice C — Symthaea candidate selection

Use a small atomistic reference dataset and frozen evaluator outputs. The companion AtomFab branch now defines explicit false-feasible accounting, including an undefined rate when the planner accepts no cases (so abstaining everywhere is not scored as perfect safety); its unit tests are authored but not yet executed. See the [metric implementation](https://github.com/Luminous-Dynamics/symthaea/blob/research/atomfab-evidence-benchmarks-2026-10-19/crates/domains/symthaea-atomfab/src/lib.rs).

Compare:
- random candidate selection;
- uncertainty-only selection;
- diversity-only selection;
- cost-aware acquisition;
- the same strategy with Symthaea's evidence/history-based reasoning disabled.

This ablation establishes whether the Symthaea layer adds measurable value beyond conventional methods.

### Slice D — Hardware only after gates pass

Real STM/ALD/growth-equipment control is not part of this initial scope. Before a hardware adapter is considered, require approved operating envelopes, physical interlocks independent of Symthaea and Symtropy, safe fallback, signed/traceable operator approval, emergency-stop testing, and an explicit measurement protocol.

## Industry comparison and competitive claim boundary

AtomFab and Symtropy should compare with the strongest relevant published baselines at each layer rather than claim one combined leaderboard rank:

- **Atomistic model:** compare a pinned available pretrained potential against the chosen MS25 / EOS / relevant official benchmark results. OMat24-trained models already report leading Matbench Discovery performance; Symtropy is not an alternative to these models. [OMat24](https://doi.org/10.1038/s43588-026-00996-w).
- **Physical applicability:** include experimental references and domain-shift tests. UniFFBench reports a gap between computational benchmark performance and experimental mineral-property prediction; a numerically stable simulation must not be treated as proof of physical accuracy. [UniFFBench](https://doi.org/10.1038/s43588-026-01019-4).
- **Process twin:** baseline against a simple static constraint checker and conventional trajectory planner, then ablate the Symthaea decision layer. Measure false-feasible acceptance, recovery, and calibrated residuals; do not use a graphics benchmark as a proxy for manufacturing fidelity.
- **Industry-grade credibility:** follow a verification, validation and uncertainty quantification (VVUQ) process, including interoperability and traceable data. These are explicit priorities in NIST's 2026 manufacturing digital-twin workshop report. [NISTIR 8620](https://doi.org/10.6028/NIST.IR.8620).

The full benchmark matrix, source list and fair-comparison protocol are maintained in the companion [AtomFab SOTA and Industry Comparison](https://github.com/Luminous-Dynamics/symthaea/blob/research/atomfab-evidence-benchmarks-2026-10-19/docs/research/ATOMFAB_SOTA_AND_INDUSTRY_COMPARISON_2026-10-19.md) and the machine-readable [benchmark registry](https://github.com/Luminous-Dynamics/symthaea/blob/research/atomfab-evidence-benchmarks-2026-10-19/crates/domains/symthaea-atomfab/benchmarks/benchmark-matrix-v1.json), plus [campaign-record schema](https://github.com/Luminous-Dynamics/symthaea/blob/research/atomfab-evidence-benchmarks-2026-10-19/crates/domains/symthaea-atomfab/schemas/campaign-record-v1.schema.json) and the [matched-campaign validator](https://github.com/Luminous-Dynamics/symthaea/blob/research/atomfab-evidence-benchmarks-2026-10-19/crates/domains/symthaea-atomfab/src/campaign.rs).

## Proposed benchmark suite

These are targets to preregister before evaluation, not claimed performance.

| Benchmark | Measurement | Initial gate |
|---|---|---|
| Contract conformance | Valid/invalid fixture classification in both repos | 100% of frozen fixtures classified as expected; unknown critical fields rejected |
| Deterministic replay | State/action/event digest for same seed, initial state, solver versions and same platform | Exact replay on the supported single-threaded configuration; cross-platform equality claimed only if separately demonstrated |
| Hard-constraint enforcement | Violations accepted by the twin or planner | 0 accepted violations across adversarial synthetic tests; every rejection has a reason |
| Fault detection/recovery | Detection precision/recall, time to safe-stop, recovery rate, false alarms | Preregister thresholds by fault type; no averaged score may mask a missed critical fault |
| Twin validity | Prediction residuals against a calibrated reference workcell or instrument records | Report per-channel bias, RMSE, uncertainty coverage and applicability; no general digital-twin validity claim from synthetic data alone |
| Process feasibility | Valid proposals / accepted proposals, false-feasible rate | 0 false-feasible results on the frozen hard-constraint suite |
| Campaign efficiency | Evaluations to target, cost-normalized acceleration, best validated candidate at equal budget | Matched target + matched budget; 20 fixed seeds for the full comparative claim where feasible |
| Atomic-task validity | Structure recovery, energetic-ordering accuracy, property errors and held-out OOD calibration | Report metrics separately by material family and equilibrium/off-equilibrium regime |
| Multi-objective manufacturing quality | Pareto frontier / hypervolume for function, feasibility, yield, cost, resources, and uncertainty | Freeze normalization and objectives; never trade hard safety constraints for a better aggregate score |
| Counterexample discovery | Injected constraint violations and known model failure modes correctly detected | Critical false-feasible count must be zero in the frozen adversarial suite |
| Prediction-to-measurement gap | Residual between frozen prediction and real measured result | Baseline and tolerance fixed before the prospective run; preserve failures and rework |
| Evidence replay | Independent reconstruction from manifests and raw-data digests | 100% of accepted claims trace to referenced inputs; missing/broken evidence must block acceptance |

For self-driving-lab efficiency, report both acceleration factor (AF) and enhancement factor (EF), plus full learning curves and query costs. The literature notes that results depend strongly on benchmark and search-space design, so a headline multiplier without a frozen target is not sufficient. See [Adesiji et al., *Benchmarking self-driving labs*, Digital Discovery (2025/2026)](https://doi.org/10.1039/D5DD00337G).

For digital-twin credibility, follow the broader verification, validation, and uncertainty quantification (VVUQ) discipline discussed by NIST rather than treating visual similarity as evidence. NIST's recent manufacturing digital-twin priorities explicitly emphasize interoperability, VVUQ, cybersecurity, and trustworthy traceable data:
- [NIST — Digital Twins for Advanced Manufacturing](https://www.nist.gov/programs-projects/digital-twins-advanced-manufacturing)
- [NIST — Digital Twins Workshops Summary Report, NISTIR 8620 (2026)](https://doi.org/10.6028/NIST.IR.8620)
- [NIST — Credibility Consideration for Digital Twins in Manufacturing](https://doi.org/10.1016/j.mfglet.2022.11.009)

## Ownership / integration mapping

| Responsibility | Repository / component |
|---|---|
| Experimental objective, candidate ranking, uncertainty and evidence ledger | Luminous-Dynamics/symthaea, symthaea-atomfab |
| Atomistic reference data / solver adapters | AtomFab adapter boundary; external pinned solver packages |
| Workcell / apparatus dynamics and deterministic scenario replay | Luminous-Dynamics/symtropy, symtropy-physics / applicable Bevy bridge |
| Existing general embodied-control adapter | symtropy-symthaea-adapter; reuse patterns only after checking whether its uniform per-actuator command semantics fit the use case |
| Calibration and actual observations | Versioned metrology adapter / immutable measurement manifest |
| Cross-system evidence and release acceptance | Shared contract fixtures plus CI-independent local verifier; workflow status is never treated as test evidence |

## Acceptance criteria

This proposal is ready to implement only after reviewers agree to:
1. Keep atomistic calculations, workcell mechanics, and physical measurements as separate evidence domains.
2. Make the first scenario simulation-only and deterministic under a stated configuration.
3. Test fault handling and hard constraints before optimizing throughput.
4. Ablate Symthaea selection logic and compare against simple baselines.
5. Avoid hardware actuation until independent interlocks and human approval are part of a separate, explicitly reviewed design.

## Claim limits

This document proposes an architecture and benchmark plan. It does not claim that Symtropy currently implements an atomic process twin, that the existing embodiment adapter can safely operate laboratory hardware, that atomistic accuracy has been measured by AtomFab, or that any physical manufacturing improvement has occurred.


## Frozen workcell fault corpus v1

The repository now carries a machine-readable synthetic fault corpus and its JSON Schema:

- [Fault corpus](research/ATOMFAB_WORKCELL_FAULT_CORPUS_V1.json)
- [JSON Schema](research/ATOMFAB_WORKCELL_FAULT_CORPUS_V1.schema.json)
- [Corpus overview and claim ceiling](research/ATOMFAB_WORKCELL_FAULT_CORPUS_V1.md)

The frozen v1 suite has 14 uniquely identified cases: stale and non-finite observations, sensor drift outside calibration, actuator saturation, collision-margin breach, actuator interface mismatch, bridge timeout, malformed planner output, model out-of-domain, replay divergence, simulation-to-measurement provenance confusion, unqualified physical-actuation requests, and two valid controls. Expected outcomes include stable reason codes, action-authorization disposition, feasibility status, and hard-constraint disposition.

The AtomFab side now has a Rust regression test over its mirrored fixture to enforce unique IDs, fail-closed outcomes, visible rejection reasons, hard-constraint consistency, and accepted controls. This is not yet proof of Symtropy runtime parity: the Symtropy adapter/process-twin runtime still needs to consume the same cases in executable tests. Neither the corpus nor a passing software test qualifies a physical safety function, validates atomistic physics, or establishes physical manufacturability.
