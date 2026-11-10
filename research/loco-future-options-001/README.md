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
