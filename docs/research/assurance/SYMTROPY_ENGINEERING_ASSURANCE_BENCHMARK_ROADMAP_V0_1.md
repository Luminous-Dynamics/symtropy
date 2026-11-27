# Symtropy Engineering Assurance: Paradigm Shifts and Benchmark Roadmap v0.1

**Status:** Research roadmap; targets are proposed and not achieved results.

## Executive thesis

The opportunity is not to build another model validator, provenance database, digital twin or optimization solver. It is to make engineering claims composable and independently reviewable across all of them.

The proposed assurance chain is:

`Claim → required evidence → producer authority → artifact identity → transformation lineage → temporal applicability → uncertainty → independent replay`

A claim is not closed because it has a high score or because one component passed. Every required edge must close. A positive result may not compensate for a missing, stale, disputed, out-of-scope or invalid ancestor.

The Claim Closure Kernel v0.1 is a narrow structural substrate. Domain-specific systems remain authoritative for what evidence is required and what it means.

## 1. Industry baseline: complementary capabilities

| Reference | Established capability | What it does not replace | Symtropy integration opportunity |
|---|---|---|---|
| NASA-STD-7009B and NASA-HDBK-7009B (2026) | Model-and-simulation lifecycle credibility, development/use assessments and risk-informed communication | Domain-specific evidence semantics and an executable cross-repository dependency closure kernel | Make claim/evidence dependencies, assumptions and reuse limits replayable |
| ASME VVUQ 20.1-2024 | Multivariate experimental-versus-simulation comparison, including independent or shared uncertainty across validation points | Artifact provenance, producer authority, event chronology or a complete claim graph | Bind covariance and common-cause uncertainty reports to exact model and experiment records |
| IETF RFC 9943 (SCITT) | Signed statements, transparency receipts and auditable histories | Whether a signed engineering statement is semantically sufficient or physically true | Verify SCITT receipts in an upstream trust adapter; pass only authenticated status into claim evaluation |
| SLSA and in-toto | Software build provenance, attestations and dependency traceability | Physical evidence genealogy, calibration validity and engineering applicability | Reuse their provenance patterns for solver binaries, analysis environments and engineering artifacts |
| ROS 2 message_filters | Exact and approximate timestamp-based sensor synchronization | Calibration identity, clock uncertainty, spatial frame closure, source trust and temporal evidence completeness | Extend timestamp matching with frame, calibration, clock and provenance obligations |
| Waymo Open Dataset and nuScenes | Large multimodal datasets with sensor calibration, poses and synchronized camera/LiDAR/radar data | General-purpose evidence admissibility for high-consequence decisions | Use as external sensor-bundle benchmark sources; score synchronization and provenance separately from perception |
| ISO 23247-5:2026 and ISO 23247-6:2026 | Manufacturing digital-thread connectivity and digital-twin composition/interoperation | Cross-domain claim adjudication and uncertainty-aware proof of evidence closure | Map federated twins into explicit claim/evidence graphs with stable identities and replayable joins |

Primary references:
- [NASA-STD-7009B](https://standards.nasa.gov/node/263) and [NASA-HDBK-7009B implementation guide](https://standards.nasa.gov/standard/NASA/NASA-HDBK-7009)
- [NASA M&S credibility factors](https://public.ksc.nasa.gov/mns/faq/)
- [ASME VVUQ 20.1-2024](https://www.asme.org/codes-standards/find-codes-standards/vvuq201-vvuq-20-1-multivariate-metric-for-validation)
- [IETF RFC 9943](https://www.rfc-editor.org/rfc/rfc9943)
- [SLSA provenance](https://slsa.dev/spec/build-provenance) and [in-toto specifications](https://in-toto.io/docs/specs/)
- [ROS 2 message_filters](https://github.com/ros2/message_filters)
- [Waymo Open Dataset](https://waymo.com/open/about/) and [nuScenes paper](https://openaccess.thecvf.com/content_CVPR_2020/papers/Caesar_nuScenes_A_Multimodal_Dataset_for_Autonomous_Driving_CVPR_2020_paper.pdf)
- [ISO 23247-5:2026](https://www.iso.org/standard/87425.html) and [ISO 23247-6:2026](https://www.iso.org/standard/87426.html)

These systems operate at different layers. A fair comparison must preserve their respective claim ceilings.

## 2. Paradigm shifts

### 2.1 From artifact approval to claim closure

Conventional toolchains often report that an artifact, simulation or test passed. The decision-maker must reconstruct why that result applies to the actual subject, configuration, operating envelope and acquisition time.

Make the claim—not the file—the unit of assurance. Each claim should bind:
- subject and configuration;
- scope and operating condition;
- acquisition interval and knowledge cut;
- exact required evidence identities and record digests;
- all recursively required upstream dependencies;
- authority and trust state;
- uncertainty-budget identities and shared causes;
- the independent replay result.

The kernel should report structural closure only. Domain evaluators decide semantic adequacy.

### 2.2 From one timestamp to three clocks

Every high-consequence claim may involve at least three different times:
1. **Effective time:** when the evidence or event applies to the real-world subject.
2. **Knowledge time:** when the evidence or trust statement was available to the decision process.
3. **Evaluation time:** when a particular decision/replay was computed.

Do not collapse these clocks into one timestamp. Preserve half-open validity intervals, explicit receiver/knowledge cuts, and trust-policy as-of times. Future knowledge must not leak into historical claims; later events may change current reassessment without rewriting the original as-known result.

### 2.3 From opaque verdicts to proof-carrying reports

Every positive output should carry:
- canonical input commitment;
- exact reached evidence references;
- exact dependency digests;
- typed blockers and uncertainty conditions;
- evaluator version and schema version;
- independently reproducible report content.

A second verifier should recompute the report from the input. It must reject altered, stale or substituted reports. This is stronger than trusting a solver's or service's self-reported `PASS`.

### 2.4 From aggregate scores to non-compensatory obligations

Do not combine identity, authority, applicability, uncertainty, statistical accuracy and physical qualification into one weighted score. A severe failure in one dimension must not be averaged away by strong results elsewhere.

Expose a vector of independent statuses and a precise claim ceiling. For example, a structurally closed graph can still contain physically false measurements; a validated model can still be inapplicable to the current configuration.

### 2.5 From scalar uncertainty to shared causal uncertainty

A list of independent scalar uncertainty budgets is insufficient when measurements share:
- a calibration standard;
- a clock or time-transfer chain;
- a model parameter;
- a registration frame;
- a material batch;
- a solver implementation;
- an environmental correction.

Build uncertainty as a dependency graph with common-cause identifiers and, where numerical validation requires it, covariance/correlation structure. ASME VVUQ 20.1-2024 is a direct comparator for shared and independent uncertainty across multiple validation points. The next Symtropy extension should bind covariance artifacts to exact inputs and validate positive-semidefiniteness, conditioning and missing-correlation policy.

### 2.6 From static tests to adversarial assurance benchmarks

Every accepted claim type should ship with:
- a valid positive control;
- single-fault mutations;
- interacting multi-fault mutations;
- order/permutation controls;
- stale/future time controls;
- identity and digest substitutions;
- missing-ancestor and cycle controls;
- report-tampering controls;
- independent expected results.

Mutations must target the semantics, not merely malformed JSON. A benchmark is weak if it tests only the code paths the implementation author expected.

### 2.7 From centralized trust to federated, scoped trust

Different producers may be authoritative for different evidence classes and scopes. Model trust as a time-bounded relationship between issuer, evidence type, subject and scope—not as a global boolean. Use transparency receipts and federation mechanisms as verifiable upstream inputs, while keeping issuer accountability distinct from truth.

## 3. Cross-domain benchmark scorecard

All values below are **proposed acceptance targets**, not measured Symtropy performance.

| Benchmark family | Required metrics | First target |
|---|---|---|
| Claim closure | False-closure rate, false-block rate, mutation kill rate, report-tamper detection, permutation invariance | 0 false closures and 0 false blocks on frozen known-answer controls; 100% report tamper detection and order invariance |
| Temporal replay | Post-cut leakage rate, replay determinism, duplicate-conflict detection, correction-chain closure, observer-coverage completeness | 0 post-cut leakage; 100% deterministic replay under all event permutations |
| Model credibility | Multivariate normalized error, interval coverage, calibration by stratum, residual correlation, OOD selective risk, applicability violations | Pre-register model-specific thresholds; report confidence intervals and per-stratum results, never a single unqualified accuracy number |
| Sensor synchronization | Timestamp skew p50/p95/p99, clock-drift error, bundle precision/recall, missing-frame detection, calibration/frame identity failures | 0 false-positive bundles in the frozen adversarial set; report skew and coverage by sensor pair |
| Flow optimization | Primal feasibility, dual feasibility, objective reconstruction, primal-dual gap, certificate verifier false acceptance, infeasibility witness validity | 0 false-accepted certificates; exact zero gap for exact integer cases; independent oracle agreement |
| Coherent scenarios | Context comparability, scenario coverage, unit/horizon consistency, independent report reconstruction, omitted-scenario sensitivity | 0 ranges across incompatible dimensions; scenario completeness explicitly unclaimed until independently established |
| As-built manufacturing | Continuous-domain functional margin, inspection coverage, material state, yield, uncertainty budget, matched-route cost/time | No route-superiority claim without matched geometry, requirements, measurement budget and genealogy |

### 3.1 False assurance must be measured statistically

A zero-error result on a small synthetic suite is not a field reliability estimate. For each negative-control family:
- publish the number of independent generated cases;
- report the observed false-closure count;
- provide an exact confidence interval or conservative upper bound;
- stratify by fault type and fault interaction;
- preserve held-out mutation families for independent evaluation.

For physical claims, separate software evaluator accuracy from real-world false-acceptance risk. The latter needs representative physical measurements and independent inspection.

### 3.2 Performance scorecard

For graph sizes of 100, 1,000 and 10,000 evidence records, report:
- p50/p95/p99 end-to-end evaluation latency;
- peak resident memory;
- records/sec and dependency edges/sec;
- scaling against graph depth, fan-out and duplicate rates;
- deterministic replay hash stability.

The initial provisional operating envelope is 10,000 records and 100,000 edges. Performance ceilings should be fixed only after a pinned-runner baseline is recorded.

## 4. Matched industry comparison protocol

Do not claim “better than SOTA” from heterogeneous demos. Use a reproducible protocol:

1. **Freeze the problem:** identical evidence graph, claim, time cuts, scope and mutation suite.
2. **Name each comparator's role:** NASA credibility process, ASME multivariate metric, SCITT receipt verification, SLSA/in-toto provenance, ROS 2 synchronization or solver certificate verifier.
3. **Separate functions:** authenticity, provenance, statistical validity, physical applicability, numerical correctness and claim closure are distinct outcomes.
4. **Use blinded holdouts:** the implementation team must not tune against the final mutation cases.
5. **Measure both safety and utility:** false closures, false blocks, mutation detection, latency and memory.
6. **Publish limitations:** unavailable raw bytes, unverified issuer authority, missing physical data and untested deployment conditions remain explicit blockers.
7. **Require independent replication:** another implementation must be able to reproduce the same disposition from the frozen input and schema.

## 5. Integration sequence

### Stage A — Structural substrate
- Stabilize the Claim Closure Kernel, schema, mutation corpus and independent report verifier.
- Add fuzz/property tests for permutation invariance, duplicate identity, cycle detection, depth and resource limits.
- Keep the kernel outside the production workspace until its API and claims are independently reviewed.

### Stage B — Domain adapters
- **AERO:** map temporal replay, model credibility and owner evidence records into claim-closure inputs; preserve the distinct effective, knowledge and evaluation clocks.
- **Spacecraft:** map synchronized sensor bundles, calibration identity, frame graph, clock evidence and source provenance.
- **Magnetics/manufacturing:** map material state, as-built geometry, inspection coverage, model discrepancy and functional margins.
- **SAFRB/flow:** map primal/dual certificates, lower-bound reductions and scenario reports as typed evidence, not as release authority.

### Stage C — Numerical and physical validation
- Add covariance-aware uncertainty graph nodes and correlation policies.
- Add model applicability and OOD/abstention reports per declared stratum.
- Add independent physical-measurement and continuous-domain acceptance data.
- Run matched-route tournaments only after equivalent geometry, target function, process budget and inspection protocol are frozen.

### Stage D — Federation and lifecycle
- Verify SCITT/in-toto/SLSA-style receipts in dedicated adapters.
- Add issuer/key lifecycle, revocation and transparency-log completeness proofs.
- Map manufacturing digital-thread and federated digital-twin identities using ISO 23247-5/-6 concepts.
- Require new exact-head evidence after every semantic change; stale qualification never silently carries forward.

## 6. Claim ceiling

The benchmark kernel's positive output means only that the submitted graph is structurally closed for independent review according to the declared v0.1 rules. It does not verify cryptographic signatures, authenticate issuers, recompute digests from unavailable source bytes, prove completeness of domain requirements, establish statistical calibration, validate physics or authorize release, dispatch or flight.

A future SOTA claim requires an independently reviewed frozen corpus, matched baselines, physical referents where applicable, statistically meaningful sample sizes and reproducible exact-head receipts.
