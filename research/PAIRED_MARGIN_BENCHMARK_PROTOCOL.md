# Paired resilience comparison protocol, v0.5

**Status:** proposed synthetic contract; not an executed controller experiment.  
**Evidence ceiling:** structural/evaluator correctness on authored fixtures. No physical model, real environmental trace, hardware-in-the-loop result, or controller superiority claim is made.

## Research contribution

The critical benchmark shift is from comparing aggregate scores produced by unrelated runs to a **paired, gate-by-gate controller comparison**. Each controller must be evaluated on every declared trajectory with the same initial-state identifier, hazard family, severity vector, replicate and comparison group. A missing execution must be represented as invalid or not evaluated; it may not disappear from the denominator.

The Rust evaluator requires a complete controller-by-trajectory matrix, validates signed gate margins against outcomes, reports every controller/gate coverage cell, and compares a declared reference controller with a candidate on the same trajectory IDs. Each gate must separately declare whether it is a hard gate, its margin unit, and the exact signed-margin definition; the evaluator accepts only hard gates. A positive margin means the declared gate criterion is met; a negative margin means a breach. Invalid and not-evaluated rows must have null margins. The synthetic fixture uses normalized unitless margins only to exercise contract logic; those numbers are not physical observations. One gate's surplus cannot compensate for another gate's breach.

## Content-addressed experimental identity

IDs alone are insufficient evidence that two controller executions saw the same inputs. Version 0.4 binds every case to three SHA-256 identities: the exogenous trajectory payload, its initial-state payload, and the exact declared controller artifact payload. The per-case digest references must agree with one artifact registry; paired rows must share trajectory and initial-state identities; all rows for a given controller must reference the same declared controller artifact.

The fixture holds small UTF-8 **synthetic descriptor strings** so the integrity checks are deterministic and portable. They are not simulation traces or executable binaries. The independent Python verifier recomputes SHA-256 over the exact UTF-8 payload bytes; the Rust executable checks format and registry bindings but deliberately delegates byte hashing to that independent verifier. The workflow must run both, and the report says that independent verification is required. A bare hash is not source authentication: for real experiments, the same scheme must bind immutable source/model/controller artifacts, exact executable/container and configuration digests, simulator version, and a signed run receipt or independently captured output hashes. Reproducibility and source rights still require separate review.

This is aligned with—but does not merely duplicate—recent benchmark engineering practice. The 2026 CSBenchLab toolbox paper highlights standardized interfaces, reproducible experiment metadata, and plugin-based reuse; IEEE PES TR143 (8 June 2026) stresses that resilience measures must fit the operational context; and PNNL's CoSim Toolbox report (2 February 2026) describes the engineering cost of linking multiple simulators. The opportunity for Symtropy is to make content identity, pairing integrity and evidence-admission first-class constraints across adapters, rather than treating each simulator run as an unstructured log.

References:
- [CSBenchLab: A unified control system benchmarking toolbox](https://doi.org/10.1016/j.conengprac.2026.106984), *Control Engineering Practice*, Volume 173, August 2026.
- [IEEE PES TR143 — Power System Resilience Metrics and Evaluation Methods](https://resourcecenter.ieee-pes.org/publications/technical-reports/pes_tr_143_amps_060826), published 8 June 2026.
- [PNNL — Multi-Entity Simulation with CoSim Toolbox](https://www.pnnl.gov/publications/multi-entity-simulation-cosim-toolbox), published 2 February 2026.
- [IEC TR 63515:2025 — Conceptual framework of power system resilience](https://webstore.iec.ch/en/publication/73748), published 11 March 2025.

The separate Python verifier independently recomputes the coverage and paired-outcome summary from raw input rows. It does not import Rust scoring logic. The workflow must run the Rust CLI first and verify that exact emitted report, followed by adversarial mutation tests. Agreement on a synthetic fixture is evaluator evidence, not physical validation.

## Required reporting

For every controller and gate, publish pass, breach, invalid, not-evaluated and valid-evaluated counts. For each paired gate, publish both-pass, both-breach, reference-breach/candidate-pass, reference-pass/candidate-breach and incomplete-pair counts, plus the distribution summary of candidate-minus-reference signed margin among rows with valid margins. Do not collapse these into a universal scalar.

Do not call a controller resilient because a gate has no observed breach when its coverage is zero. Do not treat incomplete pairs as passes. Do not infer probabilities from an unweighted stress set. Report the empirical stress suite and probabilistic risk separately; probabilistic estimates require an admitted sampling/probability basis.

## Research plan and measurable targets

1. **Evaluator integrity:** zero false passes across declared negative controls; reject missing controller/trajectory pairs, mismatched trajectory metadata, unsupported evidence escalation, inconsistent status/margin pairs, report tampering and unknown fields.
2. **Paired study:** freeze a baseline ladder and a locked whole-event/site-year holdout before tuning. Give all policies identical disturbances, controller-visible signals, action constraints, forecast horizon and simulator budget. Any additional information or compute becomes a separately reported resource premium.
3. **Outcome study:** predeclare critical-service floor violations as non-compensatory; report unserved service by type, deficit duration/severity, recovery time, state debt after repeated events, invalid-run rate, cost, runtime and resource consumption.
4. **Statistical study:** use the episode/site-year as the independent unit, preserve temporal blocks, publish paired effect estimates and confidence intervals, and account for multiple confirmatory comparisons. Do not set an achievable superiority percentage until a pilot estimates variability; keep the existing 10% CVaR95 target explicitly aspirational and not measured.
5. **Search study:** compare engineering-authored cases, seeded space-filling sampling, interaction coverage and adaptive falsification at fixed evaluation budgets. Score distinct independently replayed counterexamples per model call, time to first counterexample, replay success, novel failure-family yield and locked-holdout generalization. Search quality and controller quality are separate claims.
6. **Model validation:** start with one narrow, validated electrical subsystem; test the plant independently before controller rankings. Emit timestamped traces and conservation/solver diagnostics through a versioned simulator adapter. Add multi-site communications partitions only after delay, loss, stale data, fallback and state reconciliation semantics are explicit.

## Causal mechanism attribution, ablation and placebo controls

A paired controller bake-off estimates whether two policies differ on the tested trajectories; it does not, by itself, establish *why* they differ. Claims such as "memory improved recovery," "longer forecasts improved resilience," or "state reconciliation reduced cascading failures" need predeclared interventions that isolate the named mechanism.

For each mechanism claim:

1. Freeze a base policy and an intervention variant that changes exactly one intended mechanism. Bind both artifacts, configuration, model, trajectory, initial state, observation budget and resource limits by digest.
2. Use paired disturbances and, where the simulator allows it, common random numbers for exogenous randomness. Randomize execution order across controller arms to reduce warm-up, cache and thermal-order artifacts. A controller that receives a different scenario or extra information is a different comparison, not an ablation.
3. Predeclare the primary outcome and analysis unit. Report paired per-episode/site-year effects and block-resampled uncertainty; minute-level samples within one event are not independent replicates. For each gate, retain breach, invalid and unevaluated outcomes.
4. Include a **no-op placebo**: activate the ablation pathway while preserving the policy's effective action. In a deterministic plant, output divergence above a declared numerical tolerance is an evaluator defect or unmodeled side effect. In a stochastic plant, compare distributions under preregistered equivalence margins rather than require byte equality of random outcomes.
5. Include **negative-mechanism controls**: a shuffled or deliberately stale forecast must not be described as a benefit of better prediction; a disabled reconciliation hook must not be credited with reconciliation gains; and a feature whose computation is enabled but whose output is not consumed should not improve outcomes except through explicitly measured resource effects.
6. Run sensitivity analysis across scenario families, plant/model uncertainty and admissible budget settings. Publish subgroup regressions and failed ablations; do not select the mechanism definition or delete negative results after the locked test.

This advances beyond leaderboard-style comparison toward causal explanation. A 2026 review of causal inference in power and energy systems notes the limited adoption of causal discovery in this domain and recommends adding at least one causally grounded validation step beyond prediction accuracy—such as treatment-effect estimates, placebo/refutation tests and sensitivity analysis—when the question supports those methods. It does not imply that every simulation comparison identifies a real-world causal effect; physical validity and deployment assumptions remain prerequisites.

### Benchmarks for attribution claims

- **Deterministic no-op placebo:** zero unexplained output divergences beyond the declared numerical tolerance.
- **Paired effect:** publish mean/median and tail-effect differences by scenario family, with a predeclared episode/site-year-level confidence interval and no timestamp-level pseudo-replication.
- **Placebo calibration:** for stochastic placebo suites, test the false-positive rate at a declared nominal level (for example, 5%) across repeated null interventions; report the interval around that rate instead of claiming it must be exactly 5% in a finite sample.
- **Mechanism robustness:** require the claimed direction to replicate across predeclared nonstationary families and at least one model-uncertainty perturbation before making a cross-regime mechanism claim.
- **Resource-adjusted effect:** report the paired effect alongside compute, memory, observation count, forecast horizon, runtime and cost. A positive control effect that disappears under matched budgets is a resource-premium result, not evidence of a better controller architecture.

Research reference: [Causal inference and machine learning in power and energy systems: A review of assumptions, applications and opportunities](https://doi.org/10.1016/j.egyai.2026.100774).

## Execution receipts and equal-resource comparison

The content-bound v0.4 paired input protects static identities; v0.5 adds the information-budget and per-trajectory observation-trace digests. Neither input proves that an execution occurred or that the measured run stayed within its declared budget. Versioned controller execution receipts separate these claims.

Each receipt binds the paired trajectory, initial state and controller artifact, along with the plant model, run configuration, executable, runtime environment, observation policy, resource policy, command, outputs and resource ceilings. The numerical limits are themselves bound by a canonical SHA-256 digest of the sorted, compact JSON form. The required `resource_policy` artifact must contain those exact canonical bytes and carry the same digest, so changing a budget or changing the policy independently is rejected. A separate `controller_artifact` inventory role must match the controller digest in the paired-input bindings; a label-only controller ID is not sufficient. The receipt template is explicitly `design_only_not_executed`, with null measurements, no output artifacts, no independent verification and `scoring_eligible=false`.

The independent receipt verifier rehashes payload-backed UTF-8 artifacts and cross-checks receipt identity against the paired input row. The receipt-set verifier supports both legacy receipt v0.1 with paired input v0.4 and additive receipt v0.2 with paired input v0.5. For v0.2 it binds the shared information-budget digest, each scenario's observation-trace digest, and the observation-policy payload to the exact budget bytes. It validates the complete paired input once, rehashes declared payloads and reuses case/trajectory/controller indexes across the receipt matrix. A new negative control tampers with the shared paired input before receipt-set verification and expects rejection. The receipt-set verifier then requires exactly one receipt for every declared controller-by-trajectory case and enforces:
- identical numeric resource limits, resource-policy digest, observation-policy digest, plant-model digest, executable digest and runtime-environment digest across arms;
- identical run-configuration digest for both controllers on a given trajectory;
- stable controller-artifact identity across every trajectory for that controller;
- identical trajectory and initial-state content identities for all controllers paired on the same trajectory.

A complete and internally consistent receipt set is a necessary integrity condition, not evidence that the physics is valid. Synthetic template receipts remain ineligible. **Important current limitation:** receipt schemas v0.1 and v0.2 never mark any result scoring-eligible. The receipt carries an explicit `execution_attestation.status=not_implemented` and a `scoring_eligible=false` hard gate, because the signed execution-attestation verifier does not yet exist. Receipt v0.1 is paired only with input v0.4; receipt v0.2 is paired only with input v0.5. A self-reported `independent_verification=passed` field is not cryptographic proof that a simulator run occurred. A future version can only lift this block after a trusted signing/attestation design is implemented and verified, in addition to completed execution, independently rehashed inputs/outputs, resource compliance and the complete fair receipt set. This offline verifier also requires payload-backed artifacts for its own direct hash checks; external artifact stores need a verifier that reads exact object bytes rather than trusting an operator-supplied URI/hash pair.

This design follows current benchmark-engineering concerns, but it is not a claim of novelty or superiority by itself. [CSBenchLab (August 2026)](https://doi.org/10.1016/j.conengprac.2026.106984) argues that custom-tailored scenarios and under-tuned baselines can bias controller comparisons, and emphasizes standardized interfaces and metadata. A 2026 open-access perspective on [reproducible benchmarking](https://www.frontiersin.org/journals/bioinformatics/articles/10.3389/fbinf.2026.1925740/full) similarly recommends versioned artefacts, explicit interface contracts, execution provenance and transparent evaluation. IEEE PES TR143 (8 June 2026) cautions that resilience metrics must fit the operational context rather than collapse into one universal measure. PNNL's [CoSim Toolbox report (2 February 2026)](https://www.pnnl.gov/publications/multi-entity-simulation-cosim-toolbox) describes the integration and synchronization burden when multiple simulator models are coupled. Symtropy's proposed contribution is to make pairing, artifact identity, resource-budget equality and receipt completeness executable invariants across those simulator boundaries.

## Signed execution attestation: strict future trust profile

The [execution-attestation profile](EXECUTION_ATTESTATION_PROFILE_2026.md) records why the benchmark cannot allow receipt metadata to authorize scoring. It proposes a detached Sigstore signature over the exact immutable receipt bytes, with an out-of-band-pinned policy, exact protected-main GitHub Actions identity and OIDC issuer, standardized bundle format only, reviewed Cosign version, and no transparency-log bypass. The current wrapper's test suite stubs Cosign, so it tests policy enforcement and failure behavior but **does not validate a real signature**. The protected-main attestation workflow and real signed fixture remain future work; receipt v0.1 and v0.2 score eligibility is false under their current schemas.

The version pin is based on the latest observed Cosign release at policy authoring (**v3.1.4, 20 October 2026**). Release/advisory review found the 2026 legacy-bundle verification bypass GHSA-fx35-mq7g-6g98 and malformed attestation/predicate validation issue GHSA-w6c6-c85g-mmv6. The policy rejects legacy bundle media types and will not authorize scoring merely because a cryptographic verification command returns success. A genuine score claim still requires a trusted execution job, complete paired receipts, identical plant/observation/resource constraints, an independently validated physical model and a paired locked-test study.

## Causal information parity and no-look-ahead (v0.5)

Version 0.5 binds every case to one shared information-budget digest and every trajectory to a content-addressed observation trace. The budget currently declares a 60-second decision interval, a maximum 300-second forecast horizon, a 1,000 ms compute budget, and 256 MB memory limit. These are **synthetic contract parameters**, not a validated plant's operating limits. Real values must be specified for the deployment class and then frozen before comparisons.

The independent verifier checks these temporal rules in each observation trace:
- Measurement observation time and availability time must be at or before the controller decision time.
- A forecast must have been issued and made available by decision time; its valid time may be in the future only within the declared forecast horizon.
- Decision times must advance by the declared interval; the trace must be content-addressed and bound to the exact trajectory.
- Every controller case must reference the same information-budget digest. A modified or absent budget is a contract failure, not an unreported comparison difference.

This is a substantial fairness control, but it is **not by itself proof that an executed controller never accessed hidden files, network data, or future state**. A real bake-off must also execute in an isolated environment with only declared inputs mounted, constrain network access, record the actual per-decision input bytes or digests, and capture output/artifact hashes in an execution receipt. The receipt must be independently verified. Any out-of-band access invalidates that run. Synthetic observation trace validation is not hardware-in-the-loop or field validation.

### New measurable benchmark: information-efficiency frontier

Do not give the candidate a richer forecast, denser telemetry, extra compute or more calls and then credit every gain to the policy architecture. Freeze the information budget and compare under the same observation schedule, forecast vintage, allowable look-ahead, sensor set, action limits and compute/memory limits. Then run a separate budget sweep—for example, forecast horizons and compute caps selected before testing—and publish the Pareto frontier of critical-service risk versus information/compute expenditure. This reveals whether a candidate is genuinely more efficient or simply consumes a larger resource budget.

Required anti-leakage negative controls include: measurement sampled after decision time; delayed measurement not yet available; forecast issued after decision time; forecast beyond allowed horizon; tampered observation bytes or digest; per-case information-budget mismatch; and input from an undeclared sensor/source. Every known-answer invalid control must be rejected. The benchmark should report both the number rejected and the number of controller runs invalidated by the control harness. False rejection of legitimate forecast vintages is also a defect and requires a positive control.

This control is timely, not speculative. A study published 9 October 2026 on energy forecasting reported that noncausal full-series decomposition can produce very large apparent forecasting gains; it compared causal and noncausal preprocessing under otherwise controlled conditions. Its exact findings are specific to those datasets and models, but its lesson transfers directly: the information available at the decision origin is part of the experimental input, not a cosmetic metadata field. A September 2026 systematic review of microgrid reinforcement learning also found no single method dominates simultaneously across cost, latency, resilience and deployment complexity, supporting multi-objective frontiers instead of one overall "winner."

References:
- [Leakage-controlled re-evaluation of full-series decomposition in energy forecasting](https://doi.org/10.1155/er/4433079), first published 9 October 2026.
- [Adaptive intelligence in smart microgrids: systematic review and comparative analysis](https://link.springer.com/article/10.1007/s10462-026-11663-x), published 9 September 2026.
- [NIST TEVV-Athlon Framework for Evaluating AI Systems](https://www.nist.gov/artificial-intelligence/ai-research/tevv-athlon-framework-evaluating-ai-systems), announced 7 August 2026.

## Industry comparison without category errors

- **Microgrid control testing:** IEC TS 62898-3-5:2026 covers HIL, commissioning and periodic testing for monitoring, control and energy-management systems in grid-connected or isolated microgrids. Use its applicable licensed clauses and a reviewed clause-to-test crosswalk; this research contract is not a claim of IEC conformity.
- **Resilience metrics:** IEEE PES's 2026 report, *Power System Resilience Metrics and Evaluation Methods*, emphasizes selecting metrics suited to system level and operational context. Preserve service quantities, recovery, constraints and uncertainty rather than optimizing a single opaque score.
- **HIL/PHIL grounding:** NREL's microgrid-controller testbed work is a reference for closed-loop controller testing against real-time simulation. Synthetic trace scoring is below this evidence level.
- **Multi-domain coupling:** HELICS is an existing co-simulation option for connecting power, communications and other model domains; choose and validate each plant model instead of claiming Symtropy itself supplies validated physical truth.

This benchmark's potential differentiator is a versioned, fail-closed evidence and pairing layer that allows multiple external simulators and policies to be compared under an auditable protocol. It does not replace OpenDSS, REopt, HOMER, EMT models or hardware testbeds by assertion. Every comparison must document which tool is serving as optimizer, solver, plant simulator or reference.

## References checked for this protocol

- IEC, [IEC TS 62898-3-5:2026 — testing for microgrid monitoring, control and energy-management systems](https://webstore.iec.ch/en/publication/84175), published 11 June 2026.
- IEEE PES, [Power System Resilience Metrics and Evaluation Methods (TR143)](https://resourcecenter.ieee-pes.org/publications/technical-reports/pes_tr_143_amps_060826), published 8 June 2026.
- NREL, [Microgrid controller hardware-in-the-loop / power-hardware-in-the-loop test-bed report](https://www.nrel.gov/docs/fy20osti/71338.pdf).
- [HELICS documentation](https://docs.helics.org/en/latest/).
- NREL, [Power System Resilience Evaluation Framework and Metric Review](https://www.nrel.gov/docs/fy22osti/80817.pdf).

## Current state

The v0.5 paired fixture is synthetic and encodes 12 result rows for two controllers across six trajectories, including valid passes, breaches, invalid runs and unevaluated outcomes. Receipt v0.1 and v0.2 templates and the receipt-set verifier test execution provenance, budget and observation-policy equality, artifact bindings and version isolation without claiming that any simulation was run. The six trajectories are not simulated physical systems. The Python verifiers and mutation suites check report structure, payload hashes, causal observation timing, shared information-budget identity and receipt-set contract; only the exact-head GitHub workflow can compile and execute the Rust implementation. Even a passing workflow validates software contracts only. The current observation schedule is synthetic and does not prove runtime information isolation. The remaining prerequisites for an empirical performance claim are real admitted inputs, an independently validated plant model, a frozen baseline and holdout, and paired experiments with uncertainty reporting.
