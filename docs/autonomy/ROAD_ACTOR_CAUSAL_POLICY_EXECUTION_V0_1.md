# Road Actor Causal Policy Execution Contract v0.1

**Status:** proposal / specification-only  
**Parent:** Symtropy ROAD-AUTONOMY-ACTOR-001 [#3471](https://github.com/Luminous-Dynamics/symtropy/issues/3471)  
**Execution substrate:** [ROAD-AUTONOMY-SCENARIO-001 #3387](https://github.com/Luminous-Dynamics/symtropy/issues/3387)  
**Claim ceiling:** deterministic, causally constrained *synthetic* actor policies under named assumptions. No human-behaviour validity, collision-risk, roadworthiness, or SAE automation claim.

## 1. Purpose and paradigm shift

Replace open-loop actor scripts as the only available interaction model with an **observation-mediated causal actor policy**:

```text
world state at tick t
  -> actor-permitted observation view delivered by t
  -> pure actor policy(state, view, exogenous input)
  -> bounded actor transition + receipt
  -> kinematic actor adapter
  -> existing PhysicsWorld step
  -> next world state / future observations
```

An actor reacts to *what it could have observed by that time*, not to the simulator's hidden state or the ego controller's private command stream. A fixed trajectory remains a useful baseline, but it is never labelled interactive.

The intended scientific object is not merely a plausible-looking trajectory. It is a falsifiable causal relation between an ego intervention, the observation made available to another actor, that actor's policy response, and the downstream world state.

## 2. Reuse the existing physics substrate

The current `symtropy-physics` API already supplies the intended execution primitive:

- `BodyType::Kinematic` is available in `body.rs`.
- `PhysicsWorld::add_body` and `body_mut` are available in `world.rs`; `body_mut` conservatively marks the static/kinematic broadphase cache dirty before the mutable body reference escapes.
- `integrator::integrate` returns early for non-dynamic bodies, clearing accumulators without integrating their pose. The world step still runs broadphase and contact processing. The actor adapter, not the dynamics integrator, therefore owns kinematic target-pose application.
- `ReplayTape` currently expresses force, impulse, linear/angular velocity and wake commands. It does **not** encode a kinematic pose transition.
- `WorldSnapshot` captures runtime `BodyHandle`-keyed body and collision-event state. It does **not** include an application-level scenario actor ID or a causal actor-policy receipt.

These are concrete constraints, not reasons to add another physics engine. The v0.1 adapter should keep a stable `ScenarioActorId -> BodyHandle` mapping, apply each accepted kinematic transition through `PhysicsWorld::body_mut`, and emit a separate append-only `ActorReplayLog`. The actor log and existing world snapshot are joined by a run-context digest and stable actor mapping. Do not claim that the present `ReplayTape` alone replays actor policy decisions.

Do not use `BodyHandle` as the scenario identity: it is a runtime allocation detail. `NetId` may be used by an integration where appropriate, but the contract remains independent of its networking semantics.

## 3. Pure policy boundary

The policy function is conceptually:

```rust,ignore
fn transition(
    policy: &PolicyConfig,
    prior: &ActorPolicyState,
    observation: &AllowedActorObservation,
    exogenous: &ActorExogenousInput,
) -> Result<ActorTransition, ActorPolicyError>;
```

It must be deterministic for identical canonical inputs. It must not access files, clocks, network, global RNG, simulator resources, ECS world state, hidden labels, or future event queues. Randomized policies are out of scope for v0.1; later stochastic policies must receive explicit, recorded exogenous draws rather than own an ambient RNG.

### 3.1 Inputs

**Policy identity/configuration**
- content digest of the policy artifact and normalized configuration;
- policy family and version;
- declared response-delay and motion bounds.

**Actor policy state**
- actor-local finite state (e.g. waiting, crossing, paused, complete);
- last accepted transition sequence and tick;
- only the policy's own persisted state, not a copy of hidden simulator state.

**Allowed observation view**
- explicit allow-list of actor-visible cues;
- each cue has a source/event identity, observed tick, delivered tick, and typed value;
- each cue status is one of `Observed`, `Stale`, `Missing`, `Occluded`, or `Unsupported`;
- stale, missing, occluded and unsupported are distinct; none is silently converted into a numeric zero or a fresh observation.

Permitted v0.1 cues are limited to synthetic, timestamped quantities such as visible ego range, visible relative speed, visible heading/turn-signal state, crossing-signal state, and actor-local geometry/route cues. The scenario profile must state which cues exist and when they are observable. Raw `PhysicsWorld`, all-body arrays, controller internals, planned/future ego commands, hidden scenario labels, holdout identifiers and future ground truth are forbidden policy inputs.

**Exogenous input**
- a materialized disturbance/noise value or a typed absence;
- source domain, draw/event index and content digest;
- no implicit calls to RNG.

### 3.2 Outputs

An accepted transition contains:
- prior and next actor policy state;
- typed actor action (e.g. `Wait`, `EnterCrossing`, `Continue`, `Pause`, `Retreat`, `Complete`);
- transition reason code;
- bounded target pose and/or velocity in a declared frame and unit system;
- effective tick and any explicit reaction-delay metadata;
- disposition (`Applied`, `Rejected`, or `ModifiedWithReceipt`).

Non-finite values, invalid frames, backward time, out-of-profile acceleration/speed, duplicate sequence numbers, or an invalid state transition are rejected. A safety-relevant clamp must not be silent: either reject it or emit `ModifiedWithReceipt` with the requested and applied values and the exact bound that caused the modification.

## 4. Clock and event-order contract

Use an integer scenario tick as the authoritative causal ordering key. The run profile declares the tick frequency and the mapping to physics `dt`; every conversion is explicit. Do not order events by floating-point timestamp comparisons.

For each tick, the harness performs this order:

1. Close the observation-delivery window at tick `t`; only cues with `delivered_tick <= t` enter the actor view.
2. Build the allow-listed observation view from the actor's permitted sensors/cues, including occlusion, delay, dropout and staleness status.
3. Invoke the pure policy with the prior actor state, that view, and the tick's declared exogenous input.
4. Validate the transition and append its receipt before applying it.
5. Apply an accepted transition to the mapped kinematic body via `PhysicsWorld::body_mut`; set transform and velocity according to the adapter's declared convention.
6. Step the existing physics world exactly once for the declared `dt`.
7. Capture requested world observables and the world snapshot; these may enter a later observation window, never the already-closed one.

A cue generated at tick `t` cannot affect a policy invocation earlier than its declared delivery tick. Event sequence numbers are strictly increasing per actor; timestamps are monotonic; one run has exactly one clock policy.

## 5. Causal isolation and paired interventions

### 5.1 Future-action noninterference

Two runs that are identical through tick `t`, but differ only in an ego command scheduled for a later tick `t+k`, must have identical actor policy inputs, actor transitions and actor state through tick `t+k-1`. Compare canonical actor outputs and receipts, not just final positions.

### 5.2 Hidden-state noninterference

Mutating simulator-only state that is not represented in the actor's allowed observation view must not change the actor policy output. This includes hidden controller mode, future scenario labels, privileged expert state and holdout-family metadata.

### 5.3 Observation mediation

An ego action may affect an actor only after that action changes a cue which the actor is permitted to observe and that cue is delivered. The contract does not prohibit a cue from being physically affected before it is visible; it prohibits the policy from learning that effect through an undeclared channel.

### 5.4 Shared exogenous disturbances

Paired counterfactual ego-action branches use common pre-materialized exogenous disturbance values where the disturbance is genuinely exogenous to the ego intervention. If an intervention physically changes occlusion, visibility, noise, or another downstream condition, that consequence must be recomputed causally; do not force branches to share a value that is no longer exogenous.

Keep seed/draw domains separate for scene generation, actor disturbances, policy stochasticity (not used in v0.1), and evaluator/reference generation. Record domain identity and draw indices. Never reuse the evaluator stream as a policy input.

## 6. Actor receipts and replay

Each transition receipt binds at least:

- schema/version and run-context digest;
- scenario ID, stable actor ID, actor-policy artifact/config digests;
- per-actor sequence number and integer tick;
- allowed-observation digest, cue statuses and source-event references;
- exogenous-input digest and seed/draw domain;
- prior-state digest, requested transition, applied transition and next-state digest;
- stable actor ID to runtime `BodyHandle` mapping identity;
- engine/profile identity and declared `dt`;
- previous receipt digest and current receipt digest.

Canonical serialization and deterministic field ordering are mandatory. A hash chain detects accidental mutation/reordering relative to a trusted root; it does not, by itself, authenticate an author or provide non-repudiation. Signed provenance is a separate future concern.

Replay reports must distinguish:
- **same-runtime exact replay:** identical named binaries, engine/backend, configuration, tick schedule and inputs; bitwise snapshot/receipt equality is expected where the implementation promises it;
- **portable semantic replay:** a separately declared set of observables and per-observable tolerances; never described as bitwise equivalence.

The v0.1 actor log must include actor transitions because the current `ReplayTape` does not encode kinematic pose commands. A fresh-process replay test must reconstruct the stable actor mapping and compare both actor receipts and world snapshots tick-by-tick.

## 7. Conformance and falsification suite

The machine-readable companion fixture is `docs/autonomy/fixtures/ROAD_ACTOR_POLICY_CONFORMANCE_V0_1.json`.

Required controls:

1. **Future-command noninterference:** changing a not-yet-delivered ego command cannot change actor output before delivery.
2. **Hidden-state isolation:** changing a hidden world/controller field cannot change output when the allowed observation is byte-identical.
3. **Occlusion:** an occluded cue cannot affect output as though observed.
4. **Staleness/dropout:** stale and missing cues trigger the declared fallback/hold behaviour and remain in the denominator.
5. **Reaction delay:** no policy response occurs before the configured observation delivery/reaction-delay condition.
6. **Ego intervention:** yield/continue/accelerate branches produce distinct actor responses only after the corresponding ego-motion cue is delivered.
7. **Policy-family sensitivity:** two declared policy profiles produce distinct traces in at least one predeclared discriminating case.
8. **Fresh-process replay:** same runtime + same initial world + same actor mappings + same actor logs + same exogenous trace produce identical per-tick receipts and snapshots.
9. **Fixed-script control:** open-loop playback remains a separately identified baseline and cannot satisfy the interactive-policy acceptance test.
10. **Identity/time validation:** duplicate actor IDs, duplicate receipt sequence, non-monotonic ticks, mapping changes, NaN/Inf and out-of-bounds transitions are rejected or explicitly typed.
11. **Incomplete execution:** timeout, missing observations, unsupported cue semantics and rejected transitions remain visible as typed outcomes; no dropped denominator.

For deterministic v0.1 conformance, target 100% pass of the required negative controls, zero future/hidden-state leakage, zero silent missingness, and bitwise equality in same-runtime fresh-process replay. These are software-contract targets, not evidence of realistic pedestrian behaviour or safe driving.

## 8. Evaluation: keep the scorecard disaggregated

Do not produce one weighted "actor realism and safety" score. Report at least:

| Dimension | Measures | Interpretation boundary |
|---|---|---|
| Causal integrity | future-action and hidden-state noninterference; observation delivery correctness | Contract conformance only |
| Policy sensitivity | actor response difference under delivered ego-motion interventions | Synthetic policy responsiveness, not human realism |
| Timing | reaction-time distribution, p50/p95, censored/unsupported fraction | Requires declared event and censoring semantics |
| Motion | displacement, velocity, mode/intent error by class and interaction phase | Requires an admitted external reference |
| Decision quality | selected-action regret, optimism gap and rank reversals by actor family | Requires independently qualified counterfactual reference |
| Completeness | expected/observed/missing/stale/occluded/unsupported counts | Denominators must remain visible |
| Reproducibility | receipt equality, world-snapshot equality, replay failure rate | Exactness only for named runtime |
| Cost | wall time, sim-time ratio, memory, CPU/GPU, timeout rate | Always report hardware and workload |

For motion prediction/actor simulation, use separate external tracks:
- Argoverse 2 multi-agent motion forecasting: minADE/minFDE and miss-rate are trajectory metrics, not decision-quality metrics.
- Waymo Open Sim Agents Challenge (WOSAC): realism, kinematics, interactivity, map adherence and displacement metrics assess simulation-agent qualities.
- **InterHub dense interaction reference:** use only as a later, source-admitted interaction-stratification track. The 2025 paper reports 65,399 extracted interaction segments involving 146,170 agents, with participant IDs, temporal bounds, path relations, priority labels, PET and interaction-intensity information. It notes that source interactions are sparse and some rare path-relation classes are unstable; report denominators and uncertainty per family. Trajectory-only evidence does not establish latent intent or unmaterialized anticipation.
- Closed-loop planning and robustness: CARLA Leaderboard 2.1, Bench2Drive, nuPlan and Fail2Drive; publish route completion, infractions, decision regret, scenario-family coverage, runtime and incomplete outcomes separately.

**InterHub rights gate:** the Figshare v7 wrapper is listed CC BY 4.0, but its record says redistributed third-party caches remain governed by their original dataset licences; the paper is CC BY-NC-ND 4.0. Freeze a source-file-level licence/provenance manifest before importing cached WOMD, nuPlan, Lyft or INTERACTION data, and do not assume the wrapper licence authorizes commercial reuse. Use event metadata first if source data rights are not admitted.

The external scores are comparison anchors, not acceptance criteria for this synthetic adapter. Dataset and benchmark licenses/protocols must be checked before importing or publishing data.

## 9. Implementation sequence

**A0 — Contract fixture:** validate the JSON, freeze policy rules, cue allow-list, clock and negative controls.

**A1 — Pure policy module:** implement a validated stable `ScenarioActorId`, typed observations, actor state with monotonic decision ticks, deterministic policy trait and bounded transition validation without Bevy or physics dependencies.

**A2 — Kinematic adapter:** own the stable actor-ID/BodyHandle map; apply transitions through `body_mut`; call the existing world step once per tick; expose typed adapter errors.

**A3 — Actor replay log:** canonicalize and hash-chain receipts; join actor log to run context and `WorldSnapshot`; reconstruct in a fresh process.

**A4 — Conformance harness:** implement all negative controls, two policy profiles, and the fixed-script baseline.

**A5 — Evidence bridge:** emit a machine-readable result bundle for Symthaea's discrepancy evaluator and decision-regret analysis. Keep search, statistical estimands, reference qualification and safety interpretation in Symthaea.

Do not start A2 before A1's policy boundary is stable. Do not claim A4 before all controls have executed. Do not claim human behavioural fidelity before external reference evaluation.

## 10. Industry comparison and open research

Current public results show why the tracks must remain separate:

- AV2 multi-agent motion forecasting: the surfaced 2026.07 QCNeXt entry reports minADE@6 = 0.50 m, minFDE@6 = 1.02 m and miss rate@6 = 0.13; IMR reports 0.51 m minADE@6. These are trajectory-prediction results, not closed-loop planner or safety results.
- WOSAC 2025 simulation agents: the surfaced SMART-R1 entry reports realism = 78.58, kinematics = 49.44, interactivity = 81.10, map adherence = 92.01 and minADE = 1.2885. The component scores stay separate; this is simulation-agent quality, not ego driving performance.
- nuPlan closed-loop test: the surfaced DTCCL entry reports overall score = 69.66, with collision-rate component = 96.48, drivable coverage = 99.02, TTC = 85.67, comfort = 58.70 and progress ratio = 75.42. This is a different simulator/task protocol from CARLA and must not be numerically ranked against CARLA or AV2.
- Fail2Drive (April 2026) introduced 200 paired routes across 17 distribution-shift classes. The paper reports a 22.8% average success-rate drop and a 16.3% average harmonic-mean drop across seven learned methods. Its behavioral-generalization category shows an average 53.6% HM drop across the four models analysed by category. We should publish both percentage-point success loss and normalized relative success loss, plus harmonic-mean loss, by shift family; aggregate performance must never conceal the behavioral tail.
- NIST's 2026 measurement-science programme explicitly separates surrogate safety metrics, forward outcome prediction, counterfactual action comparison, and cross-scenario diagnosis. The Symthaea/Symtropy split should support this progression without claiming to duplicate NIST's physical validation.

The October 15, 2026 *Comfortably Stuck* audit reports that Bench2Drive's original comfort score correlated with stationary time at r = 0.97 across eight evaluated models; its proposed distance-based correction reduced that correlation to r = 0.06. The paper also reports that the default controller replaced model steering in more than half of frames in some scenarios and identifies train/evaluation navigation-target mismatch. These findings are specific to the audited protocol, models and scenarios—not a universal claim about every submission. Treat benchmark protocols and evaluators as objects of red-team testing; do not assume a leaderboard score is measurement truth.

Links:
- [AV2 multi-agent motion forecasting leaderboard](https://www.sota2.com/research/sota/multi-agent-motion-forecasting-on-argoverse-2-av2-test)
- [WOSAC leaderboard](https://www.sota2.com/research/sota/motion-simulation-on-waymo-open-sim-agents-challenge-2025)
- [nuPlan closed-loop test leaderboard](https://www.sota2.com/research/sota/closed-loop-autonomous-driving-on-nuplan-test)
- [Fail2Drive paper](https://arxiv.org/abs/2604.08535)
- [NIST Measurement Science for Automated Vehicles](https://www.nist.gov/programs-projects/measurement-science-automated-vehicles-0)
- [Comfortably Stuck (October 2026)](https://arxiv.org/abs/2610.21741)
- [ASAM OpenSCENARIO DSL 2.2.0](https://www.asam.net/standards/detail/openscenario-dsl/)
- [InterHub Scientific Data paper (2025)](https://pmc.ncbi.nlm.nih.gov/articles/PMC12218015/)
- [InterHub Figshare dataset v7](https://figshare.com/articles/dataset/_b_InterHub_A_Naturalistic_Trajectory_Dataset_with_Dense_Interaction_for_Autonomous_Driving_b_/27899754)
- [InterHub toolkit](https://github.com/zxc-tju/InterHub)
- [ASAM OpenSCENARIO XML 1.4.0](https://www.asam.net/standards/detail/openscenario-xml/)

## 11. Claim ceiling

Passing this work proves that the named synthetic actor policies obey the declared information boundary, respond causally to delivered observations, produce bounded transitions, and replay under the named runtime. It does **not** prove that they reproduce human intent, estimate real-world actor distributions, reduce crash risk, validate the physics backend, qualify an ADS, or support an SAE Level 4/5 claim.
