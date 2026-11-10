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

Thermal state is a vector indexed by an embodiment-specific node manifest, not an average. Keep motor, driver, battery, structure and compute nodes distinct where modeled. Bind every successor snapshot to the exact thermal model, context, node manifest, predecessor-state digest and resulting-state digest. A hot motor may invalidate an action even when the whole-body mean is below every other limit.

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

Compare immediate-action filtering, successor counts, per-hidden-state clairvoyant availability, observation-constrained common continuations, and the full thermal/fault/recovery composition. Later physical comparisons should include the separate A1 thermal-policy studies, residual thermal policy, FT-WBC, robust/impact-aware MPC and foothold/CBF methods in separate platform and evidence strata.

Do not combine headline values from different papers into one leaderboard. Match robot, payload, terrain, gait, ambient temperature, hardware, sensor set, model, controller budget and measurement boundary. Keep simulation and physical results separate. Use an independently qualified plant model or hardware measurements until Symtropy's own joint, actuator and discrete-step semantics are qualified (see #4516 and #4571).

## Verification boundary

The JSON corpus can establish finite semantic expectations only. Schema validity does not establish algorithm correctness; a passing finite evaluator does not establish successor completeness, sensor truth, continuous-time safety, sim-to-real fidelity, arbitrary-fault tolerance or physical qualification. All such claims require independent upstream evidence and held-out physical experiments.

No actuation authority is created by this protocol.
