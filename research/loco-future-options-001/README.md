# LOCO-FUTURE-OPTIONS-001 — observation-constrained thermal mobility reserve

Status: research protocol / synthetic known-answer corpus. No controller, physical qualification or safety claim.

Parent issue: [#4588](https://github.com/Luminous-Dynamics/symtropy/issues/4588).

## Research question

Does a locomotion action preserve useful future mobility when the controller only receives its actual observations, rather than the hidden physical state available to an evaluator?

A robot may have a feasible continuation in every hidden state and still have no single continuation it can select when those states look identical to its sensors. Counting the union of per-state actions is a clairvoyant upper bound, not an operational capability measure.

## Finite contract

Inputs are frozen and identity-bound:

- initial state and embodiment identity;
- first-action domain and continuation-action domain;
- transition-model / abstraction identity;
- complete physically possible successor-branch manifest;
- mapping from each physical branch to its delivered observation-equivalence class;
- exact per-branch, per-continuation-action upstream viability disposition;
- thermal node manifest and path-bound thermal state where applicable;
- evidence freshness, context and deadline;
- upstream route/topology and terminal safe-stop witnesses.

For an observation class O, let B_O be every physically possible branch that yields the same delivered observation. The common continuation set is the intersection of V(b) over every b in B_O, where V(b) is the set of canonical physical continuation actions independently admitted for that branch.

The clairvoyant reference is the per-branch availability (or union of per-branch actions). It is reported separately as an optimistic diagnostic bound. It must never be substituted for the observation-constrained common set.

A fresh, valid observation can split an observation class and permit a different continuation per newly distinguishable class. A stale, missing, delayed or uncalibrated observation cannot.

## Disposition

- Qualified: complete branch/action manifests and valid evidence permit the finite common-set calculation.
- Invalid: a supplied hard constraint is explicitly violated (for example a node temperature exceeds its declared limit).
- Unknown: manifests, receipts, state identity, freshness or physical successor coverage are incomplete or mismatched.

Unknown never contributes a positive continuation. Physically possible branches remain in the robust calculation even if their calibrated probability is zero. A finite count is only a result over the exact declared action domain; it says nothing about omitted continuous actions.

## Thermal and mission coupling

Thermal state is a vector indexed by an embodiment-specific node manifest, not an average. Keep motor, driver, battery, structure and compute nodes distinct where modeled. Bind every successor snapshot to the exact thermal model, context, node manifest, predecessor-state digest and resulting-state digest. A hot motor may invalidate an action even when the whole-body mean is below every other limit. Also distinguish **measured sensor channels** from **model-estimated internal nodes** and **environmental boundary conditions**: each needs its own source class, identity, timestamp/clock and uncertainty. The 2026 residual-policy study describes a 14-node A1 thermal network (12 motors, onboard computer and ambient environment), while its residual-policy observation uses a 12-element motor-temperature vector. A 14-state thermal model therefore must not be misrepresented as 14 directly instrumented temperature channels.

Compose the existing mission-reserve and recovery-topology records rather than creating new thermal or graph models:

- [#4511 / PR #4512 / PR #4540](https://github.com/Luminous-Dynamics/symtropy/issues/4511)
- [#4549 / PR #4551 / PR #4555](https://github.com/Luminous-Dynamics/symtropy/issues/4549)

Report thermal headroom, torque/slew authority, energy, diagnostic cost, recovery routes/cut sets, safe-stop reachability and deadline status separately. Preserve Pareto incomparability; do not invent a scalar resilience score.

## Known-answer corpus

fixtures/known_answers_v0_1.json contains synthetic tests for:

1. the clairvoyance trap;
2. fresh observations that distinguish successor states;
3. a single over-limit motor hidden by a safe arithmetic mean;
4. a zero-probability but physically possible successor branch;
5. incomplete branch manifests;
6. duplicate labels for the same physical continuation;
7. thermal-state predecessor digest mismatch.

The companion schema is fixtures/known_answers_v0_1.schema.json.

The corpus is deliberately small and transparent. A Rust implementation should be checked against an independent brute-force oracle, not tested only against its own helper functions. Add permutation invariance, monotonicity under added physical branches, malformed identifiers, stale receipts, deadline expiry, common-cause topology and bounded resource-exhaustion cases before making broader claims.

## Composition / ownership

Reuse:
- Symtropy #4061 / #4083 for finite robust viability and transition branches;
- Symtropy #4119 for state-dependent evidence validity;
- Symtropy #4382 for policy-to-dynamics projection;
- Symtropy #4310 / #4313 for contact-information value and diagnostic actions;
- Symtropy #4179 / #4206 / #4044 for actuator and energy evidence;
- Symtropy #4511 / #4549 for recovery topology and mission state carryover;
- Symtropy #4338 for qualified operating-envelope composition;
- Symthaea #4795 and #11475 for the evidence-bounded robotics design process and belief-aware mission executive.

This benchmark is a consumer/composition layer. It does not create physical branches, qualify upstream evidence, estimate temperature, run a controller, authorize motor commands or certify safety.

## Baselines and experimental discipline

Compare immediate-action filtering, successor counts, per-hidden-state clairvoyant availability, observation-constrained common continuations, and the full thermal/fault/recovery composition.

### 2026 frontier comparators (different evidence strata)

- **Thermal-aware A1 locomotion:** [Qian et al., arXiv:2603.01631](https://arxiv.org/abs/2603.01631) reports over 27 minutes under a 3 kg payload versus roughly 7 minutes for its baseline. This is the first thermal-endurance reproduction target.
- **Residual thermal policy:** [Wan et al., arXiv:2605.27046](https://arxiv.org/abs/2605.27046) reports over 13 minutes under a 3 kg payload versus roughly 5 minutes for its nominal baseline. Keep this as a separate experiment; do not compare its runtime directly to the first paper. Its model includes 12 motor nodes, onboard compute and ambient environment, and the model's node states must not be confused with directly measured channels.
- **Fault-tolerant loco-manipulation:** [FT-WBC, CoRL 2026](https://ft-wbc.github.io/) reports Go2 + Airbot Play arm hardware trials under weakening and locked-joint faults. Its stated limitations are single-actuator failures and stationary-base or sequential move-then-manipulate evaluation; simultaneous multi-joint failures and dynamic manipulation while walking remain open.
- **Active fault diagnosis:** [Tian, SSRN preprint (24 August 2026)](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=7345456) reports 720 Go2 MuJoCo active-probe trials, 97.9% fault-type accuracy and 99.6% faulty-joint localization. Treat these as preprint/simulation claims, not physical qualification. This is a strong diagnostic baseline for #4310 and the future mission executive.
- **Foothold / barrier screening:** [STEP-CBF+](https://www.nature.com/articles/s41598-026-72507-w) reports 4.0% mean fall rate and about 13.6 ms mean selection time over six randomized simulation scenarios. It is reduced-order simulation, not hardware validation or a full-body invariance proof.
- **Impact robustness:** [Impact-Aware Robust Convex MPC](https://doi.org/10.1109/LRA.2026.3692086) reports Unitree Go2 simulation and hardware comparisons under intensified impact disturbances. It is a physical robust-control comparator, not a mission-reserve benchmark.
- **Policy-to-dynamics projection:** [PADP, IEEE RA-L 2026](https://doi.org/10.1109/LRA.2026.3734869) maps policy outputs into contact-consistent whole-body variables and reports quadruped-manipulator simulation and hardware results. Symtropy #4382 already owns a related projection boundary; projection alone is not the novelty claimed here.
- **Proprioceptive terrain adaptation:** [FAWBC, Autonomous Robots 2026](https://doi.org/10.1007/s10514-026-10263-6) reports physical Unitree G1 trials using encoders and IMU to infer terrain slope and adapt whole-body control. This is a humanoid cross-morphology comparator; the authors identify discontinuous steps as a limitation of retrospective contact-based terrain inference.
- **Low-DoF morphology:** [Kinetic Minimalism, IEEE CASE 2026](https://doi.org/10.1109/CASE69030.2026.11704153) reports an 8-DoF wheeled quadruped with physical jumping, stair traversal, bracing and environment-aided interaction. It belongs in a morphology/whole-body interaction stratum, not as a directly matched 12-DoF quadruped controller.
- **System-level energy:** [RAIBO2, Nature 2026](https://doi.org/10.1038/s41586-026-11102-5) reports a 42.195 km marathon in 4 h 19 min 52 s at total CoT 0.25. It combines mechanical design, motor electronics and policy.
- **Physics-grounded energy transfer:** [PACE sim-to-real, IJRR 2026](https://doi.org/10.1177/02783649261459628) reports a 32% reduction in full ANYmal CoT and transfer to ten additional robots. This is a strong actuator-energy-model and cross-robot transfer comparator.

Do not combine headline values from different papers into one leaderboard. Match robot, morphology, payload, terrain, gait, ambient temperature, hardware/firmware, sensor set, model, controller budget, training information and measurement boundary. Keep simulation, hardware and preprint evidence separate. Use an independently qualified plant model or hardware measurements until Symtropy's own joint, actuator and discrete-step semantics are qualified (see #4516 and #4571).

## Verification boundary

The JSON corpus can establish finite semantic expectations only. Schema validity does not establish algorithm correctness; a passing finite evaluator does not establish successor completeness, sensor truth, continuous-time safety, sim-to-real fidelity, arbitrary-fault tolerance or physical qualification. All such claims require independent upstream evidence and held-out physical experiments.

No actuation authority is created by this protocol.

## Executable finite reference — 12 November 2026

A standalone, dependency-free Rust 1.96 package now lives at `reference/`:

- `reference/Cargo.toml` and `Cargo.lock` isolate the finite evaluator from the application workspace.
- `reference/src/lib.rs` implements deterministic canonical-action mapping, observation-class grouping, per-branch admitted-action consumption, common-set intersection, clairvoyant branch counts, and reason-coded Qualified / Unknown / Invalid outcomes.
- The evaluator consumes upstream evidence; it does not generate physical branches, infer thermal state, solve viability, or create route topology.

The evaluator refuses positive option counts when the branch manifest, continuation domain, per-branch viability receipt, observation freshness, thermal predecessor chain, or recovery receipt is incomplete. Physically possible branches remain in the calculation even at zero probability mass. User-facing labels collapse by canonical physical action identity. Thermal nodes are checked individually using conservative upper-temperature bounds; no mean-temperature waiver exists. A qualified recovery receipt must establish a reachable terminal safe-stop.

**Fifty authored Rust tests** cover:
- clairvoyance trap versus observation-constrained intersection;
- valid observation separation;
- zero-probability physical branches;
- duplicate action labels / canonical action alias collapse;
- incomplete branch and viability manifests;
- Unknown observation freshness and thermal predecessor mismatch;
- per-node thermal hard-limit enforcement;
- terminal safe-stop reachability;
- branch permutation invariance;
- monotonicity when adding physically possible branches;
- independent brute-force oracle agreement;
- malformed or unmapped action labels.

The companion synthetic corpus and schema were expanded with complete continuation-domain, per-branch viability, observation and recovery receipts. The exact-head workflow now runs formatting, tests and strict Clippy on the pinned Rust 1.96 toolchain. These tests are authored but should not be reported as passing until that workflow completes on the exact head.

### New benchmark target: recoverable continuation, not clairvoyant option count

For each observation class, report:
1. canonical common-action set and finite-domain cardinality;
2. branch-by-branch clairvoyant availability as a separate upper bound;
3. clairvoyance gap (only when all required evidence is Qualified);
4. branch/domain/observation/thermal/recovery Unknown and Invalid denominators;
5. thermal node and reserve/topology provenance separately.

Prospective finite-reference goals:
- 100% exact agreement with an independent brute-force oracle;
- zero count inflation from aliases or display-label duplication;
- adding a physically possible branch never increases a common continuation set;
- no zero-probability physical branch is silently dropped;
- zero positive option claims from incomplete/Unknown evidence;
- no thermal mean can mask a per-node hard-limit breach;
- no qualified recoverability claim without a terminal safe-stop/recovery receipt.

This is a finite action-domain benchmark, not a continuous viability proof or a controller. The Rust output always has `actuation_authorized = false`.


The updated seven-case JSON corpus and schema were fetched back and parsed. A separate semantic replay compared each case's expected disposition and continuation sets against the expanded receipts: **7/7 cases matched**. This remains a semantic preflight; no Draft 2020-12 JSON Schema validator has been executed.

The package uses typed Rust receipts and intentionally has no third-party JSON parser/runtime dependency. The JSON corpus is a separate, versioned interchange/known-answer artefact governed by its schema; the Rust tests instantiate the same finite semantics directly. The current workflow validates Rust source only. A separate schema-validator execution / corpus-to-Rust ingestion adapter remains a subsequent qualification step.

### Bounded policy-tree extension

The same package now exposes `policy_tree::evaluate_bounded_policy_tree` for a frozen finite belief-state tree with horizon 1–3. A belief node is one delivered observation history and contains all hidden states still possible under that history. An action is eligible at that node only when the upstream admission receipt includes it for every state in the belief. Each physically possible transition then maps to the exact successor state set associated with its next observation node. Every horizon-zero leaf must carry a qualified terminal recovery / safe-stop receipt.

The evaluator distinguishes four results:
- `WinningPolicyFound`: at least one complete finite policy witness exists;
- `NoWinningPolicyInDeclaredDomain`: all evidence is complete, but no policy over the frozen finite domain reaches qualified terminal leaves;
- `Unknown`: action, transition, observation partition or leaf recovery evidence is incomplete;
- `Invalid`: a supplied required receipt is explicitly invalid.

The returned witness is a deterministic map from observation-history node to action. The evaluator does not pick one action per hidden state, so a clairvoyant policy cannot be mistaken for a deployable one. It permits a different action only after an evidence-qualified observation creates a new belief node. The test tree includes two indistinguishable successor states after a probe, then a fresh observation splits them so one child can retreat and the other continue; both leaves still require a safe-stop receipt.

The new module contributes 18 tests: bounded two-step winning policy; disjoint hidden-state actions; incomplete transition manifest; unknown/unreachable terminal recovery; observation-partition mismatch; incomplete transition receipt; impossible transition exclusion; incomplete action domain; permutation invariance; missing child node; invalid action identity; and explicit horizon bounds. The original 20 tests remain in the same crate, for 50 authored Rust tests total.

The one-step common-set evaluator and the bounded policy-tree evaluator are separate, intentionally composable reference surfaces. A policy-tree win over this finite graph is not a continuous-time viability proof, arbitrary fault tolerance, or authorization to execute the witness.

### Persistent physical-state continuity in the policy tree

The bounded evaluator does not treat an observation graph as sufficient physical evidence. Each belief state carries exact physical-state, reserve-state and thermal-state digests. Every possible transition must:
- match the parent's physical / reserve / thermal digests;
- name exact resulting physical / reserve / thermal digests;
- map to a child belief node whose per-state receipts match those results;
- preserve every successor that is physically possible, irrespective of probability.

Terminal recovery is also state-specific: each hidden state remaining at a leaf must have a matching reserve-state digest, topology receipt and safe-stop reachability result. Missing coverage is Unknown; contradictory predecessor/result identities are Invalid. This prevents an apparently valid policy from being stitched across different physical histories.

This adds four adversarial tests to the bounded tree: parent reserve/thermal mismatch, child result digest mismatch, incomplete per-state leaf recovery coverage, and terminal reserve digest mismatch.


## Frontier update — 12 November 2026

The comparison set has moved since the first protocol draft. These results are useful baselines but belong to **different experimental strata**:

- **Thermal-aware legged locomotion:** Qian et al. report Unitree A1 continuous operation beyond 27 minutes with a 3 kg payload, versus about 7 minutes for their baseline ([arXiv:2603.01631](https://arxiv.org/abs/2603.01631)). A separate residual-policy study reports over 13 minutes versus about 5 minutes on A1 under its own 3 kg payload/multi-terrain conditions ([arXiv:2605.27046](https://arxiv.org/abs/2605.27046)). These are separate methods and protocols; do not merge the runtimes into one leaderboard.
- **Manipulation success is approaching saturation on LIBERO:** PriorVLA reports 99.1% average LIBERO success ([project/paper summary](https://priorvla.github.io/)); MaskWAM reports 98.4% ([arXiv:2606.13515](https://arxiv.org/abs/2606.13515)). Streaming-WAM reports 98.35% with 41 ms action-chunk preparation and a 12× chunk-time speedup versus FastWAM in its own four-H100 evaluation ([official results](https://github.com/SJTU-DENG-Lab/Streaming-WAM)). These are distinct papers, models, data splits and runtime setups. They establish that high task-success alone is no longer enough to differentiate a robotics architecture; tail latency, causal execution alignment, observation-conditioned recoverability and hardware generalization matter.
- **Factory-scale foundation-model serving:** ROSA reports up to 12.06× factory productivity over conventional dedicated serving, using shared GPU pools and SLO-qualified task scheduling ([NVIDIA Research / arXiv:2607.01088](https://research.nvidia.com/publication/2026-07_rosa-robotics-foundation-model-serving-system-robot-factories)). That is factory productivity on its tested workloads, not a directly comparable speedup for this finite evaluator or a legged-robot result.

**Comparison rule:** never mix a manipulation LIBERO number, A1 endurance minutes, a Go2 fault campaign and factory serving throughput into one score. Reproduce each reference within its exact robot/morphology, task split, payload, terrain, controller/model, compute hardware, disturbance schedule, measurement boundary and success definition. Report simulation, hardware and preprint evidence separately.

### Proposed Luminous benchmark scorecard

The research programme should publish a *vector scorecard*, not a weighted “robot intelligence” scalar:

| Dimension | Primary measure | Required matched comparisons |
|---|---|---|
| Task capability | success, useful work, tracking error, adverse outcomes by category | strongest policy/control baseline on the same task and embodiment |
| Observation-conditioned recoverability | one-step common-action sets; clairvoyant upper bound; bounded winning-policy rate by horizon | successor-count baseline; per-hidden-state clairvoyant bound; common-set evaluator; policy-tree evaluator |
| Persistent reserve | per-node thermal trajectory/headroom, per-joint authority, energy and terminal reserve | nominal, thermal-aware, residual thermal, robust MPC / WBC |
| Decision integrity | regret upper bound, calibration coverage, decision-horizon expiry, stale-action rejection | fixed-age, fixed-chunk, latency-only and evidence-bounded horizon |
| Causal execution | applications linked, causal-closure ppm, AoI, command-to-motion, independent assurance and physical transition | event-order replay, missing-evidence negative controls, exact configuration identity |
| Fleet productivity | SLO-qualified safe missions per wall-clock, per-robot service-floor breaches, p95/p99 latency, GPU/VRAM/network/KV use | dedicated-local serving, RR, EDF, EVSI admission, shared-pool serving |
| Recovery and fairness | terminal safe-stop reachability, recovery-route/cut-set reserve, starvation by robot, unknown/abstention rates | matched fault sequences with physical state carried between events |

### Staged experimental targets

**Finite reference:** 100% agreement against independent enumeration for the committed synthetic corpus; zero clairvoyant or alias inflation; adding a physically possible branch cannot increase its observation class's common set; zero positive policy claims with Unknown branch, observation, state-transition, viability or recovery evidence; every policy leaf has per-state reserve/topology/safe-stop receipts.

**Software / replay qualification:** zero causal-closure credit for an application with missing, duplicate, expired, mismatched or unordered evidence; every negative control must be rejected or retained as Unknown as specified; preserve a separate denominator for all application attempts.

**Physical baseline reproduction:** reproduce the A1 thermal-aware baseline under its documented 3 kg payload and hardware/terrain/ambient/gait conditions before claiming a locomotion improvement. Then execute *sequential* disturbances without resetting temperature, SOC, actuator derating, wear proxy or recovery reserve. A single isolated successful fault test is not mission-resilience evidence.

**Superiority gate:** only claim improvement when the same hardware and scenario strata show non-inferior task success, tracking, per-category adverse outcomes and terminal recoverability, then a statistically supported gain on at least one declared dimension (e.g. mission productivity, thermal endurance or recovery probability) with no hidden regression on the others. Publish null and negative regimes.

A useful fleet objective is **SLO-qualified safe-mission throughput**: count only completed missions that meet task deadlines and predeclared evidence, physical-limit, service-floor and terminal-recovery constraints. Keep the numerator, all attempted missions, Unknown/censored attempts and per-robot fairness visible. This extends the factory-productivity framing without confusing high request throughput with a fleet that actually completes recoverable missions.

### Viability-conditioned VOI → bounded policy-tree bridge

Issue [#4855](https://github.com/Luminous-Dynamics/symtropy/issues/4855) defines the composition seam between [#4310 / PR #4313](https://github.com/Luminous-Dynamics/symtropy/pull/4313) and this policy-tree evaluator.

The new `reference/src/voi_policy_bridge.rs` consumes a typed projection of the upstream:
- `ViableVoiReport` gate disposition, outcome-local selected actions, joint outcome mass and Unknown mass;
- `ViableVoiProblem.probe` probe identity and physically possible branch/outcome support;
- `ViableVoiProblem.continuation_evidence` exact outcome-specific robust action sets, post-state-set digests and Unknown fallback;
- #4119/#4293 observation-equivalence, successor-state, physical/reserve/thermal and transition receipts.

It does **not** recalculate VOI, probabilities, loss, resource use, probe safety or robust viability.

The bridge groups outcomes by the actual delivered observation-equivalence class. If two outcome IDs are indistinguishable to the robot, their continuation action sets are intersected before they become one child belief node. Outcome-local selected actions remain diagnostics: a selected action may be valid for its own latent/outcome row and still be unavailable to the robot when the rows share one observation. `local_selection_is_deployable_by_observation` exposes that discrepancy. A qualified explicit Unknown bucket remains distinct from stale/missing observation evidence; stale evidence is merged into a conservative Unknown belief and cannot create a policy split.

The bridge verifies:
- report / continuation evidence outcome coverage;
- exact action-set equality between VOI report and upstream continuation receipt;
- local selected action membership;
- single-action explicit Unknown fallback;
- full joint-mass normalization and exact Unknown mass;
- exact observation-class post-state-set/receipt identity;
- complete physical branch/outcome support, including zero-mass pairs;
- per-successor physical/reserve/thermal state identity and transition receipt.

The bridge creates a **probe prefix**, not a complete mission policy. Its observation nodes deliberately have incomplete downstream transition manifests until #4293 (or the relevant transition owner) supplies the subsequent physical branches and #4549/#4555/#4511 supply terminal recovery evidence. The bounded policy evaluator therefore returns Unknown rather than claiming a multi-step win from one-step VOI data alone.

Twelve new bridge tests cover indistinguishable outcomes and action-set intersection, zero-mass physical support, gate short-circuit, continuation-set mismatch, Unknown fallback, mass accounting, stale/invalid observation handling, branch/outcome support, belief-state identity and the incomplete-downstream claim boundary. The main reference package now contains **57 authored Rust tests**. The separate Rust JSON fixture-runner package has one corpus conformance test; its exact-head workflow additionally executes the seven committed JSON cases through the Rust evaluator.


A subsequent fail-closed review tightened the VOI bridge: incomplete action or branch/outcome manifests now stop prefix construction; missing initial observation/state receipts are rejected; unknown/invalid observation classes and incomplete successor-state receipts expose **empty** deployable action sets rather than positive diagnostic counts. The current VOI regression corpus includes these cases. The policy-tree suite also asserts the AND condition: one safe successor does not compensate for another physically possible successor with no terminal recovery route.

The one-step evaluator's recovery receipt is now **branch-bound**. It must name the exact successor branch-manifest digest and contain one reserve/topology/terminal-safe-stop receipt for every physically possible successor branch, including zero-probability branches. Missing coverage is Unknown; a foreign branch or manifest mismatch is Invalid. The seven committed JSON cases now carry this receipt per branch; their fetch-back structural preflight confirmed exact manifest binding and 1:1 physical-branch/recovery-receipt coverage in all 7/7 cases.
