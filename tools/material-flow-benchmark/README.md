# Material Flow Benchmark contract tool

A standalone Rust benchmark toolkit combining strict scenario contracts, handoff-state validation, analytical convergence and a reduced-order Symtropy physics episode.

## What it checks

- Schema version and strict unknown-field rejection.
- Opaque identifier shape and SHA-256 / Git commit digest shape.
- Unique actor, asset, location, segment, action, event and fault identifiers.
- Package-to-tote and tote-to-asset axis-aligned fit, container gross-mass and asset payload-mass limits.
- Cargo handling/seal class against final storage class.
- Actor roles, receiver endpoint, grant validity window and required actions.
- Transfer-chain continuity and declared asset/mode compatibility.
- Mandatory safety, authorized-destination, package-identity, household-privacy and unauthorized-mutation gates.
- Required event declarations and fault-injection references.

## Receiver presence observation consistency gate

The `mfb-adjudicate-presence` CLI exercises a typed receiver-observation contract independently of simulator ground truth.

The v1 policy requires at least two fresh observations from distinct declared common-cause groups. A group may identify shared controller, power, firmware, calibration or clock dependencies. Two devices in one group count as one source; missing group membership is unknown and does not count as independent.

The adjudicator reports `ConfirmedPresent`, `ConfirmedAbsent`, `Unknown`, `Contradictory` or `Stale`. It rejects exact scenario/subject mismatches and contradictory source identities. Identical event-ID replay is idempotently suppressed; reuse of an event ID with different content is contradictory. It checks declared observation time/expiry against a strict decision-time interval and a maximum age.

Run the adversarial false-positive fixture:

```bash
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-adjudicate-presence -- \
  tools/material-flow-benchmark/fixtures/false-positive-receiver-observation.json
```

The fixture deliberately has a receiver-presence sensor report `present` while a separate gate-contact sensor reports `absent`. Expected result: `verdict=contradictory`, `positive_presence_confirmed=false`, and `custody_completion_permitted=false`.

Even when two declared independent groups agree, the report hard-codes `sensor_truth_established=false`, `trusted_time_verified=false`, `cryptographic_authentication_verified=false`, `motion_authority_granted=false`, `physical_command_emitted=false` and `custody_completion_permitted=false`. The group labels and time fields are caller-declared; this tool cannot authenticate or prove them. This is a consistency gate, not a sensor verifier or a custody authority.

## L1 closed-gate obstruction fault experiment

The `mfb-simulate-gate-fault` binary adds a static closed-gate collision proxy across the tote opening and executes the same 480 Hz rigid-body episode plus same-host replay.

This is the first scene-level physical-fault proxy in the MFB suite: the package contacts the obstruction, remains outside the tote, and the physical-placement predicate fails. The handoff protocol records an observation disagreement after logical release, enters `reconciliation_required`, and blocks a retry against the old command ID. The fault report succeeds only when those physical and protocol oracles match.

Run it from the repository root:

```bash
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-simulate-gate-fault -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json
```

The report separates `physical_fault_effect_observed`, `gate_contact_event_count`, `gate_contact_max_impulse_ns`, `reconciliation_required`, `blind_retry_blocked`, `false_completion_blocked` and `fault_oracle_passed`. A passing fault oracle requires a non-zero package↔gate `CollisionEvent` and measured impact impulse. These events are emitted by the physics engine once per finalized resolved contact manifold after the solver, and are captured in `WorldSnapshot` replay evidence. A passing fault oracle is not a successful delivery: the nested nominal `SimulationResult.accepted` must be false and the transaction must remain unresolved.

**Exact-head verification (2026-11-16):** workflow [43836138408](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43836138408) passed on `c1a388ab592d2bbbd07ced65662aea827b77e26e` with **79 Rust tests**, strict formatting and Clippy, all pre-existing MFB gates, and the L1 gate fault. The fault report recorded **1,841 package↔gate collision events**, maximum impulse **2.1043744963 N·s**, peak penetration **2.6422119 mm**, same-host replay equality, stable final package, `reconciliation_required`, retry blocked, and no false completion. The nested delivery result was `accepted=false`, while `fault_oracle_passed=true`.

Evidence artifact: `mfb-l1-evidence-c1a388ab592d2bbbd07ced65662aea827b77e26e` (ID `14211340768`), ZIP SHA-256 `f32e707b516cdc8071d8c60aaa2e0c019bbc99e5b6985b6749d2e3b31cac38db`. Gate-fault JSON SHA-256: `f97581eefb8e52b2d2680158e36761a4673cd96dca6f0db77055be4cbb398cdb`.

**Limitations:** the gate is a static collision geometry proxy present in the scene. This does not simulate gate motion, controller/interlock timing, actuator power, sensor perception or firmware. It is not HIL or a real-world safety test. The detailed physical fault-injection programme remains #5500.

## 30-seed L1 gate-obstruction sweep

The `mfb-simulate-gate-fault-sweep` CLI executes the closed-gate collision proxy for a deterministic consecutive seed range (default 30; supported range 1..=256). Each seed gets its own full physics episode and bitwise replay.

The report retains every requested seed. A simulation error is recorded as a failed case with a reason; it is never dropped. The summary reports requested/executed/passed/failed counts, per-seed trace digests, package↔gate event counts, maximum impact impulse, worst contact penetration, reconciliation coverage, old-command retry blocking and false-completion blocking.

Run it from the repository root:

```bash
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-simulate-gate-fault-sweep -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json 30
```

A passing 30-seed sweep is a robustness check across the scenario's deterministic initial package offsets only. The robot, package/tote configuration, static gate geometry, physics engine, timestep, scene and site are unchanged. It is not 30 independent homes, robots, sensors or hardware units, and it does not support a field-reliability confidence claim. Moving gate timing, sensor/firmware, power loss and HIL are still separate milestones.

Exact-head CI qualification for the first 30-seed runner is pending.

## Distributed physical handoff reference state machine

`src/transaction.rs` supplies a deterministic reference state machine for:
- purpose/time/audience/endpoint/scope/use-count checks at the authorization boundary;
- one logical release command ID with duplicate suppression;
- independently authored source release, receiver receipt and custody acknowledgement;
- event ID replay/conflict detection and monotonic sequence checks;
- safe hold before any dispatch, versus reconciliation-required after an uncertain physical action;
- explicit reconciliation by a configured third-party custodian, with two distinct evidence digests and a terminal outcome (completed, no action taken, returned or quarantined).

Reconciliation closes the *record*, not the physical transaction atomically. It does not re-arm the old command: a new attempt requires a new transaction and a fresh grant. The state machine does not authenticate Holochain signatures, establish that two evidence sources are truly independent, or drive actuators.

## L1 stationary-receiver physics episode

The `mfb-simulate` binary runs a reduced-order package drop into an open-top tote using the existing `symtropy-physics` engine:
- 3D rigid-box proxy for the package;
- static rigid-box tote base and walls derived from internal/external dimensions;
- fixed 480 Hz timestep (1,920 steps over four seconds) with the physics crate's `deterministic-net` Q16.16 quantization feature;
- command tape and per-tick bitwise world snapshots;
- independent same-host replay comparison;
- rotated package world-space envelope, containment, settling, numerical-invariant and maximum-contact-penetration checks (current gate: 5 mm);
- custody state-machine completion only after the simulated placement and replay checks pass.

Run it from the repository root:

```bash
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-simulate -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json
```

The result prints an L1 JSON record containing the final package position, speed, trace SHA-256, replay equality, containment/settling results and transaction state. The CLI exits non-zero if the episode is not accepted.

This is an actual headless rigid-body simulation, but it is a **reduced-order stationary receiving scene**, not a complete logistics simulation. It does not model package deformation, a robot/gripper, carrier dynamics, aircraft, routes, sensor noise, actuator control, human exposure, or physical metrology. Bitwise replay is asserted only for two runs on the same host/build; cross-platform determinism is not claimed.


### Historical L1 contact baseline and current qualification

The first 120 Hz run is retained as a historical baseline: workflow [43768458267](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43768458267), 53 tests, 10.256 mm peak contact penetration. It is not the current acceptance threshold or current qualified setting.

The same four-second nominal scene was refined to 240 Hz (5.023 mm) and then 480 Hz / 1,920 steps (2.307 mm). The current hard demonstrator gate is **5.000 mm**. The 480 Hz setting passed exact-head workflow [43769459756](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43769459756), source `2643fc9df9c22dfa760179c1deab0cf2e7f16a0e`; see the measured history in the benchmark specification and follow-on exact-head run below.

### Contact-depth refinement record

The first 120 Hz L1 baseline measured 10.26 mm maximum contact penetration. Refining the same four-second episode to 240 Hz reduced it to 5.023 mm; the 480 Hz / 1,920-tick episode then reduced it to **2.307 mm** and passed the hard 5 mm L1 gate. The 480 Hz exact-head result passed [workflow 43769459756](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43769459756), source `2643fc9df9c22dfa760179c1deab0cf2e7f16a0e`, with the same-host replay, placement and custody gates intact. Do not substitute the earlier 40 mm permissive demonstrator threshold for the current 5 mm gate.

### Executed ten-fault protocol campaign (2026-11-16)

The protocol-level fault campaign first passed on push workflow [43819492450](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43819492450) on source `a270a138095f187b6ad034fbb530cc27f746ec4f`. The latest pull-request verification [43820234221](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43820234221) also passed on PR head `d95a12f9264a2659ae98fd2c7303889efba3ab22` (merge SHA `77a6d6bcbc7c3d1b155631d6e4d294d874ac12af`), with **61 tests**, strict formatting/Clippy and all benchmark steps green.

That latest evidence artifact is `mfb-l1-evidence-77a6d6bcbc7c3d1b155631d6e4d294d874ac12af` (ID `14204525955`), four files, ZIP SHA-256 `dd3c5e32bd11d82e7a144c7c97e5b49ffa12def9529bc4f5f58b61182b691a83`. `environment.txt` records SHA-256 digests for the scenario fixture, L1 result, free-fall convergence report, protocol campaign report and committed Cargo.lock.
## Tier-B free-fall convergence check

The `mfb-validate-convergence` binary evaluates Symtropy's translational integrator against the closed-form uniform-gravity solution over four matched one-second runs:

- (dt) = 1/30, 1/60, 1/120 and 1/240 seconds;
- position and velocity error for every run;
- relative mechanical-energy drift and rotation/numerical-health diagnostics;
- monotonic position-error reduction and an estimated log2 convergence order.

Run it from the repository root:

```bash
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-validate-convergence
```

This is an analytical pre-contact translational check (Tier B for that narrow claim). It does not validate contact response, packaging, rotational inertia, robot handling or end-to-end delivery. Its result is archived beside the L1 receiving episode in the CI evidence artifact.

## Executable protocol fault campaign

The schema validator and the execution runner intentionally have different jobs:

- `mfb-validate-campaign`: checks that the frozen ten-fault matrix is structurally valid.
- `mfb-run-campaign`: executes each typed event sequence against a fresh handoff state machine and emits a machine-readable report.

The v1 runner checks:
- wrong container, over-mass and expired grant are rejected before dispatch;
- stale temperature, interlocked transfer gate and damaged/unknown seal enter safe hold before dispatch;
- replaying the same logical command is suppressed;
- a second physical release observation causes reconciliation rather than a second accepted release;
- missing receiver acknowledgement, power interruption and network loss after release require reconciliation;
- no ambiguous post-dispatch case permits a blind retry, and no injected fault is falsely recorded as completed.

Example result fields include `all_expected_outcomes_observed`, `pre_dispatch_faults_never_dispatched`, `post_dispatch_ambiguities_blocked_retry`, per-episode traces and observed transaction states.

## Evidence-typed reusable procedure graph

The `mfb-validate-procedure` binary validates a versioned DAG of typed receiving/put-away steps bound to an exact MFB scenario, source commit and configuration digest.

The nominal 15-step graph composes:
- manifest, authorization, receiver-capability, package identity, tote fit, payload mass, seal, temperature and gate/interlock preflight;
- a **release intent** (not a motor command or motion authorization);
- source-release and receiver-arrival observations;
- placement verification and counterparty custody acknowledgement;
- a parallel reconciliation branch for an unknown post-intent outcome.

Each step declares typed evidence inputs, an evidence output and prerequisite steps. The validator rejects duplicate operations/IDs, unknown dependencies, cycles, missing typed inputs, invalid evidence ancestry, missing preflight ancestors and wrong scenario/configuration bindings.

Run it from the repository root:

```bash
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-validate-procedure -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json \
  tools/material-flow-benchmark/fixtures/nominal-receiving-procedure.json
```

A successful result means **structurally valid for independent external review** only. The report hard-codes `motion_authority_granted=false`, `physical_command_emitted=false` and `task_success_attested=false`. It does not authenticate the underlying evidence values/signatures, execute a robot, assess live freshness, authorize movement or accept delivery. The procedure remains a plan submitted to separately governed authority, executor, observer and evaluator components.

### Exact-head procedure graph qualification (2026-11-16)

The new 15-step `mfb-validate-procedure` path passed inside dedicated workflow [43821500342](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43821500342), exact branch head `970ae35eb28d6ffaae0c4fbb141b5156b4a2312f`. The run passed **67 Rust tests**, strict formatting, Clippy `-D warnings`, nominal and ten-fault manifest validation, all ten executed protocol faults, procedure graph validation, analytical free-fall convergence, the 480 Hz L1 receiver/replay and evidence upload.

The graph report confirms 15 steps and `structurally_valid_for_external_review=true`, while `motion_authority_granted=false`, `physical_command_emitted=false` and `task_success_attested=false`. This verifies structural procedure composition—not evidence authenticity or execution authority.

Evidence artifact: `mfb-l1-evidence-970ae35eb28d6ffaae0c4fbb141b5156b4a2312f` (ID `14204208667`), five files, ZIP SHA-256 `b8fcda86fad54b649148faae8c722970596b3716f3f92c0df26decb070fc0e4c`. It includes the procedure readiness report and SHA-256 bindings for the scenario, L1 result, convergence report, protocol campaign, procedure report and Cargo.lock.

## Procedure evidence-receipt admission

The procedure graph describes the external evidence each step needs. The separate `mfb-admit-procedure-evidence` CLI checks a synthetic bundle against the exact scenario and procedure version.

It currently enforces:
- exact procedure ID/version and scenario ID;
- exact source commit and configuration digest on every receipt;
- one receipt for every externally supplied evidence kind, with no unexpected or duplicate kinds/identities;
- evidence-kind-specific subject binding (package, tote, receiver endpoint, transfer mechanism, grant or scenario);
- issuer actor existence and allowed actor role for that evidence kind;
- SHA-256 digest syntax;
- declared observation/expiry timestamps against the full scenario clock-uncertainty window, with strict boundary comparisons;
- per-kind maximum observation age under freshness-policy version 1.

The current synthetic bundle contains 14 external evidence envelopes. The admission report classifies six future verifier domains: household grant authority, service-provider attestation, receiver capability attestation, physical-observation attestation, counterparty custody attestation and transaction-outcome attestation. These are routing labels for downstream verifier responsibilities, not proof that those verifiers are implemented.

The report explicitly separates:
- `evidence_envelopes_structurally_admitted=true`;
- `declared_freshness_policy_passed=true`: declared timestamps satisfy the scenario-relative freshness policy, including the full uncertainty interval;
- `trusted_time_verified=false`: no trusted timestamp authority or clock attestation is checked;
- `content_digest_verified=false`: digest syntax is checked, but the digest is not recomputed from canonical evidence content;
- `cryptographic_authentication_verified=false` and `issuer_authorization_verified=false`;
- `sensor_truth_established=false`, `motion_authority_granted=false`, `physical_command_emitted=false`, and `task_success_attested=false`.

Run it from the repository root:

```bash
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-admit-procedure-evidence -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json \
  tools/material-flow-benchmark/fixtures/nominal-receiving-procedure.json \
  tools/material-flow-benchmark/fixtures/nominal-receiving-evidence.json
```

This is structural envelope admission, not a cryptographic verifier or trusted chronology theorem. An admitted envelope and a declared freshness pass are not authenticated content, an authorized issuer or a truthful observation.

**Cryptographic integration:** reuse Symtropy PHYS-EVID-03A/#1143, B0/#1144 and B1/#1145 with Xenia's dedicated attestor boundary `xenia-peer#341`. Do not add a parallel MFB signing scheme, local Ed25519-only fallback, or generic signing oracle. The integration and six-domain verifier matrix are tracked in [#5530](https://github.com/Luminous-Dynamics/symtropy/issues/5530).


## What it does not prove

The contract validator does not authenticate grant signatures, query Hearth or Commons, inspect a live asset, solve general 3D packing, prove safety, or qualify HIL / field performance. The separate L1 simulator executes one reduced-order stationary receiving scene; a valid manifest or successful L1 episode is not a physical safety case or field qualification.

All identifiers in public fixtures are synthetic opaque test identifiers. Do not place names, addresses, real DIDs, household inventories or raw sensor values in public scenarios.

## Commands

From the repository root:

```bash
cargo fmt --manifest-path tools/material-flow-benchmark/Cargo.toml -- --check
cargo clippy --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --all-targets
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-validate-convergence
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-validate -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json
```

The fixture is a synthetic L1 manifest, not an executed simulation result.

Validate the ten-fault campaign against the nominal scenario:

```bash
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-validate-campaign -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json \
  tools/material-flow-benchmark/fixtures/ten-fault-campaign.json
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-run-campaign -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json \
  tools/material-flow-benchmark/fixtures/ten-fault-campaign.json
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-validate-procedure -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json \
  tools/material-flow-benchmark/fixtures/nominal-receiving-procedure.json
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-admit-procedure-evidence -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json \
  tools/material-flow-benchmark/fixtures/nominal-receiving-procedure.json \
  tools/material-flow-benchmark/fixtures/nominal-receiving-evidence.json
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-simulate -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json
```

`mfb-validate-campaign` proves only that the ten episodes and expected terminal states form a consistent test plan. It does not execute faults. The separate `mfb-run-campaign` binary executes the ten typed fault events against a fresh instance of the Rust handoff state machine and verifies the expected terminal states and negative controls: pre-dispatch failures never dispatch, same-command replay is suppressed, duplicate physical release evidence enters reconciliation, and post-dispatch ambiguity blocks blind retry.

This is **protocol-level event injection**, not physical fault injection: it does not cut power, partition a live network, corrupt a sensor, move a real package or operate an actuator. Its report is included in the CI evidence artifact.

## Carrier–UAV co-availability screen (synthetic contract slice)

The `mfb-evaluate-coavailability` binary screens one proposed delivery-UAV sortie against a declared carrier itinerary. It checks:
- route-stop identity, chronological route-stop time envelopes and carrier stop windows;
- pad-resource identity bound to the exact launch and recovery route stops;
- conservative rendezvous bounds: the latest possible arrival and earliest possible departure define the guaranteed carrier-presence interval (not optimistic ETA bounds);
- launch and recovery service windows with interval arithmetic (launch service time is added before flight duration);
- payload, usable battery energy plus protected reserve, peak power and continuous power;
- declared operating permission and freshness/domain status of carrier route timing, pad/site resources, aircraft capability, permission and weather evidence;
- launch/recovery pad reservations using half-open intervals;
- exclusive aircraft occupancy from launch-service start through recovery-service completion, preventing one airframe from being double-booked while flying or turning around;
- earliest conflict-free slot search across launch pad, aircraft and recovery pad reservations, rather than rejecting a sortie merely because its first candidate slot is occupied.

It returns `feasible`, `infeasible` or `indeterminate`, plus a canonical SHA-256 input digest. Pad-resource and reservation ordering is normalized for hashing, while carrier route-stop order is preserved as semantic. This digest binds the report to serialized input content but does not authenticate issuers or prove the truth of source evidence. Unknown / stale / conflicted evidence and unknown / expired permission never become a pass. A known hard constraint violation is reported as infeasible. The output hard-codes `dispatch_authorized=false` and `physical_command_emitted=false`.

Run the synthetic arithmetic fixture from the repository root:

```bash
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-evaluate-coavailability -- \
  tools/material-flow-benchmark/fixtures/synthetic-carrier-uav-coavailability.json
```

The fixture is synthetic and checks only declared envelopes and times. The v0.1 slice does **not** model aerodynamic performance, wind magnitude or gust effects, aircraft battery electrochemistry, route travel-time derivation, carrier traction energy, charger losses, site power, airspace truth, flight control, airworthiness, HIL or field performance. It is not dispatch authorization. Carrier arrival bounds, capability evidence, weather status and permission status are caller-supplied inputs and must be sourced/qualified by their owning systems before any operational use.

The v0.1 evaluator is a feasibility/admission layer, not an optimizer. It does not select routes, move a drone or truck, create reservations, or claim savings. Broader integration and external benchmark comparison remain tracked by [MFB-DRONE-CARRIER-COOPT-001 #6083](https://github.com/Luminous-Dynamics/symtropy/issues/6083).


## 3D cargo-bay loading validator (synthetic contract slice)

The `mfb-validate-cargo-load` binary checks one declared multi-package cargo arrangement against a supplied vehicle cargo-bay, mass and combined-CG envelope. It currently provides:
- six axis permutations for an axis-aligned cuboid;
- bay containment and pairwise non-overlap;
- straight-line extraction corridors from a declared X/Y bay face;
- total cargo payload and gross vehicle mass;
- combined CG using vehicle tare mass/CG plus package mass/box-centre contributions;
- evidence states for bay geometry, vehicle mass/CG, package geometry and restraint;
- a canonical SHA-256 input digest that sorts package and allowed-orientation sets while preserving the typed geometry and placement data.

Run the synthetic three-package fixture from the repository root:

```bash
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-validate-cargo-load -- \
  tools/material-flow-benchmark/fixtures/synthetic-three-package-cargo.json
```

The straight-line access corridor is a deliberately narrow v1 abstraction; it is not a robot/gripper reachability or rehandling planner. Geometry uses axis-aligned bounding boxes and treats touching faces as non-overlap. Combined CG is calculated from the supplied vehicle tare CG and box-centre mass assumptions, with exact rational bounds checks before the integer-mm report value is emitted.

The validator does **not** establish axle loads, suspension/tire capability, load restraint strength, crashworthiness, cargo deformation, irregular-shape nesting, stack stability, dynamic load transfer or roadworthiness. The supplied bay / vehicle envelope and restraint evidence must be qualified by the vehicle / road-dynamics and cargo-restraint owners. A `feasible` software report sets `physical_load_authorized=false` and is not a loading instruction.

Broader 3D loading, package access/release-time, pickup/return, locker and en-route resupply benchmark work is tracked by [MFB-CARGO-3D-DYNAMIC-001 #6147](https://github.com/Luminous-Dynamics/symtropy/issues/6147).



## Demand-adaptive logistics mode options (synthetic exact oracle)

The `mfb-evaluate-mode-options` binary solves a small finite-horizon activation problem for optional aerial capacity. It compares:
- static truck-only;
- static aerial-enabled;
- immediate demand-threshold activation;
- an exact optimized activation/deactivation policy over the supplied finite scenario tree.

Each region has an independently tracked aerial-capacity option in v1. The solver includes road/aerial capacity, road/aerial service costs, unserved-demand penalties, activation/deactivation costs and active-standby costs. It reports deliveries, unserved units, switching counts and active periods separately.

### Nonanticipativity and hard gates

Policy nodes are derived from the SHA-256 digest of each region's *observed history through the current period*. Scenarios with identical observed histories therefore share the same policy decision; future scenario information is not available to earlier actions.

Aerial service is eligible only when the current observation is both `clear_fresh` and `approved_fresh`. Blocked, stale, unknown, revoked or not-approved states never become eligible because the aerial mode has positive economic value. Enabling or activating a capacity option is not flight authorization.

The exact oracle is deliberately bounded to 16 periods, 8 regions, 32 scenarios and 20 nonanticipative policy nodes (at most 1,048,576 policies). Scenario probabilities use integer basis points and must sum to 10,000. It is a small-instance oracle, not a fleet-scale optimizer.

Run the synthetic two-region, four-period scenario:

```bash
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-evaluate-mode-options -- \
  tools/material-flow-benchmark/fixtures/synthetic-two-region-mode-options.json
```

The fixture contains two demand/weather paths and 13 derived policy nodes, so the exact evaluator checks 8,192 policies. The report includes the canonical input digest and explicitly sets `physical_dispatch_authorized=false` and `dispatch_commands_emitted=false`.

### v1 limitations

This is not a calibrated demand forecast or real-options valuation for South Africa. It does not yet model shared aircraft pools across regions, asset relocation, fixed depot hubs, detachable pods, AMR/locker alternatives, depot charging queues, battery degradation, crew constraints or route-level feasibility. It assumes independent per-region aerial capacity and deterministic per-period service capacities/costs. Those are subsequent research increments; do not interpret the synthetic result as operational cost savings or an optimal real fleet configuration.

The broader activation/sharing/repositioning/withdrawal benchmark is tracked by [LOGISTICS-MODE-OPTIONS-001 #6153](https://github.com/Luminous-Dynamics/symtropy/issues/6153). Related SOTA papers include the 2026 stochastic entry/exit model for truck-only versus drone-assisted delivery (https://doi.org/10.1016/j.trc.2026.105807) and the 2026 joint multi-truck/multi-drone fleet-sizing and routing study (https://doi.org/10.3390/drones10090658). Their results and instances must be reproduced under matched assumptions before claiming improvement.


## Shared multi-region UAV fleet allocation oracle

The `mfb-evaluate-shared-fleet` binary is the next layer above single-sortie co-availability and regional mode activation. It allocates a small shared fleet of aircraft across multiple carrier regions over a finite scenario tree.

Run the synthetic two-region, two-aircraft fixture:

```bash
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-evaluate-shared-fleet -- \
  tools/material-flow-benchmark/fixtures/synthetic-two-region-shared-uav-fleet.json
```

### What it models

- Aircraft have unique identities, initial regions, per-period service capacity and capability-evidence status.
- Each period, an available aircraft may stay in its current region, reposition to another declared region or remain idle.
- Cross-region repositioning has a strictly positive, directed duration and declared cost. The aircraft is unavailable throughout the reposition interval and cannot be redirected while in transit.
- A region's aerial capacity is the sum of the actual service capacity of the aircraft assigned there; heterogeneous aircraft are not approximated by a single fleet-wide maximum.
- One aircraft may serve only one region in a period. Road service absorbs residual demand; remaining units are counted as unserved.
- Weather, permission and capability evidence are hard dispatch gates. An enabled/available aircraft is not sufficient unless the period's observation is both `clear_fresh` and `approved_fresh`.
- Scenario-tree decisions use the complete observed multi-region history through the current period. Scenarios with identical observed prefixes share one decision node; future information is not available to earlier actions.
- Exact dynamic programming enumerates small aircraft assignment/reposition decisions and reports its state/action counts. The objective includes declared service and reposition costs, not inferred operating cost.

### Baselines / interpretation

The report compares an optimized shared fleet against:
- `truck_only`: all aircraft stay idle;
- `fixed_initial_assignment`: each aircraft stays at its initial region for the horizon.

The synthetic fixture has 2 regions, 2 aircraft, 4 periods, 2 demand/weather scenarios and 7 non-root history nodes. It is an exact small-instance benchmark, not a route planner or fleet-scale optimizer. Its per-period aircraft service capacities and reposition-time/cost matrix are supplied assumptions; they must be derived from qualified aircraft, route, carrier rendezvous, payload, battery and ground-handling evidence before they can represent real operations.

### Explicit exclusions

This v0.1 oracle does not calculate flight paths, travel-time distributions, battery energy, charge curves, weather forecasts, crew rosters, airspace permissions, airworthiness, aircraft maintenance dynamics, multiple simultaneous sorties per aircraft within a period, cargo loading/release, rendezvous or physical reservations. Those remain with the EV, route, cargo, co-availability, site, asset and regulatory owners. It emits no dispatch authority and no physical commands.

The broader programme is [LOGISTICS-SHARED-FLEET-001 #6177](https://github.com/Luminous-Dynamics/symtropy/issues/6177). The next benchmark increments are multiple carrier rendezvous per period, different reposition modes, explicit energy state/charging, multi-visit resupply, and a receding-horizon policy compared against this exact oracle on small cases.

