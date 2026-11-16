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
