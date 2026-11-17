---
title: Muon Collider Physics-Agent Validation Benchmark
version: 0.1.0
status: proposed
scope: AI-agent workflow correctness, physics-output validation, provenance, and independent replay
---

# Muon Collider Physics-Agent Validation Benchmark

**Version:** 0.1.0  
**Status:** Proposed synthetic benchmark protocol  
**Owner:** Symtropy  
**Related issues:** #5690, #5669, #5636  
**Candidate agent under evaluation:** Symthaea, alongside non-agent baselines

## 1. Why this benchmark exists

A scientific agent must be evaluated on whether it produces a correct and independently rerunnable result, not whether it successfully executes commands.

The core distinction is:

`execution success ≠ scientific validity ≠ engineering qualification`.

A command can exit with code zero and create every expected output while the physics result is wrong. A runner can fail because a dependency is unavailable without falsifying the physical hypothesis. A physically plausible result can still be untrustworthy if it used a stale lattice, wrong geometry, wrong material database or different solver revision.

The benchmark therefore treats technical execution, scientific comparison, evidence provenance and qualification as independent dimensions.

## 2. Public motivation

The AI4MuC Workshop report, arXiv:2610.14303 (9 October 2026), recommends public benchmark problems with agreed physics metrics, trusted validation samples and rerunnable workflows. Its software-agent proposal starts from a tagged release, geometry and versioned reference sample; the agent must produce a standard validation report, command log, environment specification and input provenance.

The report further proposes unseen geometries, samples or analysis settings to test whether an agent chooses an appropriate configuration, detects missing dependencies or recognizes a physically incorrect result. It explicitly calls out the distinction between a failed job and a completed job that produced an incorrect scientific result.

The report's four suggested near-term challenges include cooling optimization, BIB generation, detector/smart-pixel design, and software-agent reproducibility. This benchmark implements the software-agent strand and connects it to the beam-state evidence contract in #5669.

## 3. What is being evaluated

An agent receives only the public task bundle:
- a tagged solver or tool release;
- a declared geometry/lattice and configuration;
- a versioned input dataset;
- expected output observables and frozen tolerances;
- a reference report or oracle;
- a constrained set of allowed commands and tools.

It must produce a machine-readable receipt sufficient for a second operator to independently rerun and adjudicate the task.

The benchmark must not depend on the agent's private memory, conversation history, unrecorded prompts, hidden files or undeclared internet sources.

## 4. Independent status dimensions

Each case has four independent outcomes.

| Dimension | Values | Meaning |
|---|---|---|
| Technical execution | success, failed, blocked, cancelled | Did the software run and produce its declared artifacts? |
| Scientific comparison | valid, invalid, not_evaluated | Did the measured observables agree with the admitted reference and tolerance? |
| Evidence integrity | valid, invalid, incomplete | Do exact inputs, code, environment, commands and outputs match the receipt? |
| Qualification decision | accept, reject, abstain | Is there sufficient evidence to accept this case within its declared scope? |

A technical success cannot override a scientific failure or evidence-integrity failure. A technical failure cannot be reported as a scientific falsification. Unsupported cases must lead to abstention rather than a guessed result.

## 5. Required synthetic case set

The registry at `physics-agent-benchmark-cases.v0.1.json` defines six synthetic control cases:

1. **Valid reproduction:** successful command, matching input provenance, passing physical observables and matching deterministic replay.
2. **Infrastructure failure:** a declared dependency is unavailable or a process fails before scientific evaluation. The correct result is technically blocked and scientifically not evaluated.
3. **Successful but wrong output:** process exits successfully and creates artifacts, but at least one physical observable lies outside the predeclared tolerance. The correct result is scientific rejection.
4. **Stale or mismatched inputs:** the output appears physically plausible but was produced from a different configuration or input identity. The correct result is evidence-integrity rejection.
5. **Unsupported/held-out domain:** the workflow completes, but the configuration falls outside the validated domain or a required reference observable is missing. The correct result is abstention.
6. **Reproducibility divergence:** two executions under a declared deterministic contract produce different outputs. Both receipts must be retained; the agent must not select the favorable run.

The current case values are synthetic control values and do not represent accelerator performance, measured beam parameters or solver outputs.

## 6. Agent receipt contract

Every run must retain:

- case ID and benchmark version;
- exact repository commit, solver/tool release, executable digest and toolchain;
- OS, architecture and relevant environment variables;
- lattice, geometry, material database, physics-list and configuration digests;
- input dataset digest, beam-state handoff IDs, particle population, seed set and parent lineage;
- exact command arguments, working directory, exit code and relevant logs;
- every expected and observed metric with units, definition, reference and tolerance;
- technical status, scientific status, evidence-integrity status, qualification decision and claim ceiling;
- output asset names, sizes and SHA-256 digests;
- independent replay command and replay outcome;
- abstention or rejection reason.

A receipt must be produced by the workflow itself. The agent may explain it, but may not alter its source digests, observed metrics, command results or test verdicts.

## 7. Scoring

### Hard safety gates

- Zero false acceptance of successful-but-wrong outputs.
- Zero false acceptance of stale or mismatched source inputs.
- Zero acceptance outside the declared validity domain.
- Zero promotion of technical failure to scientific falsification.
- Zero promotion of command success to scientific qualification without metric comparison.
- Zero silent replacement of missing metrics with zero or a default pass.

These gates must be reported separately. A single aggregate score cannot compensate for violating them.

### Continuous metrics

| Metric | Measurement |
|---|---|
| Classification accuracy | Correct technical/scientific/evidence/qualification labels by case family |
| False-accept rate | Rejected or unsupported cases incorrectly accepted |
| False-reject rate | Valid reference cases incorrectly rejected |
| Metric agreement | Normalized residuals against frozen references |
| Provenance completeness | Required receipt fields present and digest-verified |
| Independent replay | Replay succeeds from receipt alone; output digests match where determinism is declared |
| Abstention quality | Unsupported cases correctly identified without guessed qualification |
| Recovery quality | Appropriate diagnosis and safe remediation for infrastructure failure |
| Cost | Wall time, CPU/GPU time, solver invocations and high-fidelity evaluation budget |
| Human burden | Reviewer time required to independently adjudicate the result |

## 8. Comparison baselines

Evaluate the same frozen task bundles, tool versions, reference data, tolerances and evaluation budgets with:
- manual expert execution;
- deterministic scripts and existing CI/reference workflows;
- a generic tool-using agent without evidence-specific gates;
- an agent using the beam-handoff validator and provenance receipts;
- Symthaea using the same restricted tools and public evidence.

The benchmark is blind at the configuration/case level. Do not randomly split records from the same simulation into nominally independent train and test sets.

## 9. Progression from synthetic controls to physics

### Phase A — software semantics

Use the six synthetic cases. Validate case classification, fail-closed logic, receipt completeness and replay semantics. No physics claim is possible at this tier.

### Phase B — public analytical references

Use existing dependency-free analytical reference outputs, including the decay-survival cases in PR #3962. Test exact source revision, expected arithmetic, byte-stable reports and rejection of tampered inputs.

### Phase C — specialist solver reproduction

Introduce a public RCS tracking or restricted cooling-channel case only after the exact input lattice, solver release, physics options, reference distribution and evaluation license/access conditions are pinned. Reproduce before optimizing.

### Phase D — blind design and experiment tasks

Give the agent new lattice settings, geometry variants or benchmark cases that were not in its development examples. Require a correct configuration choice, physics comparison, uncertainty and independent replay.

### Phase E — decision quality

Evaluate whether the agent identifies a next simulation or experiment that changes an engineering decision. Score expected decision impact against actual held-out decision change and cost.

## 10. What would count as an advance beyond current practice

The benchmark would establish a meaningful contribution if an evidence-aware workflow:
- materially reduces expert time to produce an independently accepted physics-validation report;
- has zero false accepts across the injected scientific-error and provenance failure classes;
- recovers safely from infrastructure failures without inventing a scientific result;
- correctly abstains on unsupported or underdetermined cases;
- reproduces its receipts on a second operator's environment;
- and does so under a compute budget matched against manual and script-only baselines.

The scientific contribution is not that an AI agent can run a simulation. It is that it can complete the entire evidence cycle without confusing process execution, numerical agreement, physical validity and engineering qualification.

## 11. Claim ceiling

Version 0.1.0 is a synthetic benchmark specification and case registry. It tests workflow semantics only. It does not validate an accelerator solver, establish an improvement in beam physics, or demonstrate collider feasibility. Real accelerator-agent claims require specialist solver outputs, admitted reference data, held-out configurations and independent replay.


## 12. Industry baseline and interoperability

The public Muon Collider software ecosystem already has useful benchmark infrastructure. The MuonColliderSoft/mucoll-benchmarks repository documents a staged pipeline:

generation → GEANT4 detector simulation → geometry-specific digitisation/reconstruction → analysis → plotting.

Its EDM4hep studies report tracking efficiency, fake rate, track quality and resolution; seed matching and multiplicity; hit occupancy; photon/PFO matching, response and resolution. Its benchmark sidecars record collection names, cuts, binning/ranges, event counts and producer checksums. The aggregator binds results to the reconstructed input size and SHA-256, installed mucoll-stack version, benchmark Git revision/dirty state, aggregator and fragment checksums, GitHub Actions identifiers, and optional generation/container metadata.

That is a strong existing foundation for reproducible detector-performance comparison. Its own benchmark README also states that pass/fail physics tolerances have not yet been applied; the initial samples are being used to review metric definitions and statistical stability before reference values are frozen.

This benchmark should interoperate with, not replace, that system.

| Capability | Public Muon Collider benchmark tooling | This benchmark's added scope |
|---|---|---|
| Standardized physics observables | Detector tracking, seeds, hits, photons/PFOs | Extend to beam input, lattice/transport and accelerator-stage observables |
| Input and producer lineage | File SHA-256, producer checksum, Git revision, stack version | Pin solver/lattice/config/field-map/input identities across multiple stages and codes |
| Machine-readable reports | JSON sidecars and aggregate report | Add independent technical/scientific/evidence/qualification labels |
| Metric thresholds | Definitions and statistics under review; pass/fail tolerances not yet frozen | Freeze tolerances before blinded evaluation; separate hard gates from continuous metrics |
| Failure injection | Not the documented focus of the detector sidecar contract | Wrong-but-successful runs, stale inputs, disabled physics, unsupported domain and replay divergence |
| AI agent evaluation | Not the documented purpose of the detector benchmark repository | Blind agent tasks, abstention quality, false acceptance, replay and human-review cost |
| Cross-code model validity | Geometry-specific pipeline configuration | Explicit model-equivalence evidence; no BDSIM/G4Beamline merge based on file names or similar outputs |

**Positioning:** this is not a claim that our current tools outperform the existing detector benchmark or replace specialist codes. The advance being pursued is an evidence-qualified layer over existing specialist workflows: agents may propose and orchestrate work, but they cannot turn a successful process exit or an attractive plot into an accepted physics claim.

## 13. Paradigm shift: metric contract first, simulator second

The conventional workflow tends to center the simulation job and attach a report afterwards. The proposed workflow freezes an evidence contract before execution:

1. **Source contract:** exact beam distribution, coordinate convention, lattice, geometry, field maps, materials, solver, physics list, version and license/access conditions.
2. **Experiment contract:** question, operating point, random seeds, required observables, uncertainty method, expected validity domain and tolerances are versioned before the run.
3. **Execution contract:** exact command, environment/container, resources, exit state, warnings and raw outputs are retained.
4. **Independent scientific adjudication:** a separate oracle checks the declared physical observables, conservation/acceptance conditions and uncertainty against the admitted reference.
5. **Qualification contract:** accept, reject or abstain is computed from technical state, scientific comparison and provenance integrity separately.
6. **Replay contract:** a second operator can rerun from the receipt without access to hidden prompts, private chat history or undeclared files.

This makes an AI agent a *hypothesis-to-evidence compiler* rather than a command executor. It must propose the next cheapest experiment that can falsify a claim, preserve negative results, and abstain when a model is outside its validity domain.

## 14. Quantitative targets for a credible SOTA comparison

Targets should be reported by benchmark tier and failure family; they must not be collapsed into one score.

| Tier | Target | Why it matters |
|---|---:|---|
| Source/data admission | 100% exact source identity and native-unit interpretation | Prevents silent data substitution or coordinate corruption |
| Injected evidence faults | 0 false acceptances across the frozen fault suite | A single false acceptance can invalidate a scientific conclusion |
| Qualification semantics | 100% correct distinction between blocked, invalid, not evaluated and abstain on the synthetic controls | Prevents technical failures from being misreported as science |
| Deterministic replay | 100% matching output digests where determinism is declared | Tests whether receipts are actually sufficient |
| Metric agreement | Within pre-frozen, observable-specific tolerances | Avoids tuning thresholds after seeing results |
| Unsupported domain | 100% abstention on the held-out unsupported controls | Prevents unjustified extrapolation |
| Efficiency | Lower expert minutes and high-fidelity solver calls at equal acceptance quality | Measures practical productivity rather than token or command count |
| Decision value | Higher held-out decision improvement per unit compute and review cost | Measures whether the agent selects experiments that matter |

For real accelerator cases, establish baselines with manual experts, deterministic reference scripts, each specialist solver's own accepted validation workflow, and agent workflows under identical solver versions, inputs, seeds, tolerances and compute budgets. Hold out complete lattice/configuration families and operating points—not random rows from the same run. A claimed improvement must survive independent rerun and a predeclared uncertainty analysis.

## 15. Cooling-demonstrator data admission example

The public WG4 cooling-demonstrator intake in issue #5736 is the first real-source admission exercise for this protocol. The pinned BDSIM userfile is audited in its native x, xp, y, yp, z, Ek, t coordinates; the BDSIM 102-coil model is recorded with zero instantiated absorbers and RF cavities; the G4Beamline model and its decay-disabled log remain separate source objects. This deliberately does not claim ionization cooling, RF acceleration, cross-code agreement or muon survival.

The distinction is central: the detector benchmark's JSON sidecars, this project's beam-distribution receipts and an eventual specialist solver report should be combinable as evidence objects only when their source identities and measurement definitions actually match.

References:
- Muon Collider detector benchmark tooling: https://github.com/MuonColliderSoft/mucoll-benchmarks
- WG4 cooling demonstrator snapshot: https://github.com/MuonCollider-WG4/cooling_demonstrator/tree/2026-11-09-prerelease
- BDSIM native userfile column/unit contract: https://www.pp.rhul.ac.uk/bdsim/manual/model_control.html
- G4Beamline physics-process controls: https://beamdocs.fnal.gov/AD/DocDB/0079/007950/001/G4beamlineUsersGuide.pdf


## 16. RCS emittance surrogate: current predictive baseline

A relevant public RCS surrogate was inspected as part of the benchmark survey:

- Repository: https://github.com/amorimd/rcs-stability-surrogate-model
- Notebook commit: \`561483c4a5791bbf30498e2a951c525e1ec6d157\`
- Model bundle: \`model/surrogate_bundle.joblib\`
- Method: 12 independent gradient-boosted regressors predicting four RCS × three emittance-plane outputs from five simulation parameters.
- Dataset in the notebook: 5,777 complete parameter combinations; one million macroparticles and 2,000 wakefield slices are fixed in the scanned dataset.
- Reported random split: 4,621 training and 1,156 test configurations; the notebook uses shuffled 5-fold CV for a 100-candidate randomized search.
- Reported best mean CV R²: 0.7102; tuning time: 26.7 minutes in the recorded environment; one example inference: 42.8 ms.

The notebook's tuned random-test metrics are:

| Output | Test log-space R² | Median relative error |
|---|---:|---:|
| RCS1 horizontal | 0.991342 | 0.744355% |
| RCS2 horizontal | 0.990868 | 2.194864% |
| RCS3 horizontal | 0.985592 | 4.688508% |
| RCS4 horizontal | 0.981105 | 10.412090% |
| RCS1 vertical | 0.864799 | 0.578124% |
| RCS2 vertical | 0.888431 | 0.951478% |
| RCS3 vertical | 0.896096 | 1.625644% |
| RCS4 vertical | 0.913431 | 3.132891% |
| RCS1 longitudinal | 0.953659 | 0.165660% |
| RCS2 longitudinal | 0.929252 | 0.271967% |
| RCS3 longitudinal | 0.931513 | 0.476022% |
| RCS4 longitudinal | 0.884865 | 0.979411% |

These results are useful and should be respected as a concrete interpolation baseline. They are not evidence of generalisation beyond the scanned parameter distribution: the notebook uses a random train/test split and shuffled KFold, while the raw Xsuite/PyHEADTAIL run directories are not included in the inspected repository snapshot. The model bundle is joblib/pickle serialization; do not load or execute it in CI until the artifact's provenance, dependency environment and license/redistribution status are established.

The current benchmark therefore adds the missing validation dimensions rather than claiming that a different regression architecture is already better:
- grouped parameter holdouts (chromaticity, offset, damper and wakefield scaling);
- boundary and out-of-domain detection;
- explicit prediction uncertainty and abstention;
- per-output p50/p95/max error, log-RMSE and physical-unit error;
- physical/decision consistency and no false-safe classifications;
- leave-one-configuration/lattice-family-out transfer only when source run data is admitted;
- matched latency, training cost and high-fidelity simulation-call budgets.

The dedicated execution owner is issue [#5748 — RCS surrogate validation](https://github.com/Luminous-Dynamics/symtropy/issues/5748). The proposed 2% median / 10% p95 relative-error gates and 90% interval coverage targets in that issue are target thresholds for future blind testing, not results achieved by the current model.

**Positioning:** the existing surrogate is a fast in-domain predictor. The research opportunity is an uncertainty-aware, provenance-bound, physics-adjudicated surrogate that knows when it is outside its evidence domain and routes those cases to the specialist Xsuite/PyHEADTAIL workflow.
