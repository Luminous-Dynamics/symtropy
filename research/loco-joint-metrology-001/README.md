# LOCO-JOINT-METROLOGY-001

Owner: Symtropy issue LOCO-JOINT-METROLOGY-001.

This protocol composes existing AoI, actuator response, actuator loss, energy-boundary and thermal-state owners into one instrumented single-joint physical experiment. It does not create a duplicate controller, actuator model, thermal model or energy ledger.

## Measurement chain

    source capture
      -> decision / regret / calibration
      -> optional remote inference request / admission
      -> action prefix / projection / assurance
      -> actuator command latch
      -> torque / motion response
      -> electrical input and mechanical shaft work
      -> per-node thermal / persistent reserve transition
      -> terminal safe-stop / recovery

Keep separate:
- source-to-actuator AoI;
- command-to-motion onset and torque response;
- inference/network/queue/projection/dispatch delay;
- electrical energy entering the declared boundary;
- mechanical work crossing the declared shaft boundary;
- thermal state and measured heat;
- remaining actuator authority and recovery reserve.

## Declared physical boundaries

Freeze the actuator/gearbox/drive/controller/load identities; electrical supply boundary; mechanical shaft boundary and torque sign; thermal control-volume nodes; robot and DAQ clock identities / bounded mapping; sensor calibration identities and uncertainties; sampling and anti-alias requirements; and exact firmware/model/policy/configuration digests.

At minimum measure source capture and sequence, actual actuator command latch, delivered torque, shaft position/velocity, load torque, bus voltage/current, per-node temperatures, faults/saturation/derating, energy boundary terms and terminal safe-stop/recovery.

Sampling frequency must be selected from actuator bandwidth and transducer/instrument specifications, then frozen before confirmatory comparisons. The fixture rate values are synthetic known answers, not a recommended universal rate.

## Physical accounting

For a declared boundary:

    E_electrical_in = integral(V_bus * I_bus dt)
    W_shaft_out = integral(torque * angular_velocity dt)

These are independent measurements with units and sign conventions. Their difference is not automatically “motor heat”: gear, drive, wiring, auxiliary loads, stored energy and uncertainty must be accounted or marked Unknown.

Use Symtropy #3612/#3615/#4044 for open-system first-law closure and energy-path instrumentation; #4206 for command-to-motion / torque response; #4179 for actuator losses; #4610 for coupled thermal state; #4713 for AoI; #3597/#4641 for scoped analytical entropy/exergy references. Entropy/exergy, joules, task-loss regret and EVSI must remain distinct.

A balance is only as complete as its declared boundary and instrument coverage. Compare the residual to a predeclared combined uncertainty budget, not an arbitrary universal percentage.

## Sequential protocol

Use a declared sequence of baseline, repeated starts/stops and reversals, step/ramp loads, sustained thermal operation, sensor/time uncertainty, queue/dispatch delays, derating/degradation, then recovery and safe-stop. Do not reset temperature, wear proxies, actuator authority or reserve state between perturbations.

Pilot runs may establish variance and sample size. Freeze the perturbation order, paired/crossover protocol, held-out split, independent episode unit, success/adverse-event definitions and confidence procedure before confirmatory trials.

## Benchmark arms

Use matched hardware, policy and plant:
- fixed AoI;
- fixed action chunk;
- progress-triggered replanning;
- hard-bound commitment without decision sufficiency;
- decision-sufficiency plus hard bounds;
- synchronous / asynchronous execution;
- local / admitted remote inference;
- alternative context retention policies where remote context is involved.

Ablate one architecture factor at a time. Preserve identical independent hardware interlocks and emergency stop.

## Causal trace adapter

The fixture input now includes a frozen `causal_identity` block with mission, robot, boot epoch, clock, scenario manifest, source-observation, decision, proposal, assurance, command, application, physical-transition and terminal IDs plus a strictly increasing event-sequence chain. The separate `configuration_and_calibration` block carries actuator/controller/policy/model/action-domain digests, sensor/torque/power/thermal calibration digests, sampling-spec and energy-boundary manifest digests, application receipt, decision calibration receipt, and expected/observed executed-prefix digests.

These identifiers are intended to map directly into Symthaea [ROB-CAUSAL-MISSION-REPLAY-001](https://github.com/Luminous-Dynamics/symthaea/tree/research/rob-causal-mission-replay-001). The physical metrology protocol emits observations, decisions, action chunks, assurance records, actuator applications, physical transitions and terminal receipts; it does not invent IDs or rewrite upstream status. The clock relation field selects whether AoI may be computed or must remain Unknown.

## Two-part causal trace bridge

The research prototype now has two complementary reference components:

1. **Symtropy normalizer:** [`adapter/src/lib.rs`](https://github.com/Luminous-Dynamics/symtropy/blob/research/loco-joint-metrology-001/adapter/src/lib.rs) accepts a typed `JointTrace` with source/application/motion timestamps, clock-map evidence, decision, thermal, first-law, sampling and terminal receipts. It emits an `AdapterReport` with bounded AoI and command-to-motion intervals, reason-coded Qualified/Unknown/Inconclusive/Rejected outcomes, and the eight-event causal edge chain. It has 29 authored Rust tests and is isolated in a dependency-free Rust 1.96 package.

2. **Symthaea mission join:** [`joint_metrology_adapter.rs`](https://github.com/Luminous-Dynamics/symthaea/blob/research/rob-causal-mission-replay-001/src/joint_metrology_adapter.rs) consumes the authoritative Symthaea `MissionTrace` plus instrument and upstream energy-closure receipts. It verifies that those receipts bind to the exact decision, action-domain, executed-prefix, actuator application, thermal snapshot, predecessor-state and terminal identities; then composes the existing offline causal replay. It contains 17 authored Rust tests. [Symthaea draft PR #11664](https://github.com/Luminous-Dynamics/symthaea/pull/11664).

These are complementary references, **not yet a serialized, end-to-end inter-repository wire protocol**. The next integration gate is to freeze a versioned normalized-receipt schema and prove that a captured fixture can pass through the Symtropy normalizer and into the Symthaea mission join with exact identity preservation. No direct runtime dependency is claimed yet.

The Symtropy normalizer consumes the upstream first-law closure result; the Symthaea join checks the receipt identity and disposition. Neither implements another thermal, actuator, controller, viability or first-law model. Both are retrospective and never grant actuation authority.

## Synthetic known answers

- `fixtures/joint_metrology_v0_1.json`
- `fixtures/joint_metrology_v0_1.schema.json`

Nine cases cover complete closure, actuator queue dwell, response delay independent of AoI, unknown clock mapping, incomplete energy boundary, missing thermal node, hidden physical-state reset, decision-regret expiry before hard physical limits, and an independently isolated executed-prefix mismatch. JSON and schema are parsed and semantically preflighted when authored; this is not JSON Schema validator execution or a physical trial.

## Metrics

Report source-to-actuator AoI intervals per modality; regret and calibration coverage; inference/network/queue/projection/dispatch/response latency separately; torque tracking and derating; electrical joules, shaft work and energy residual/uncertainty; per-node thermal trajectory coverage; future torque/mobility reserve; RTA vetoes/safe-stops/adverse outcomes; and p50/p95/p99 planning, application and response latencies.

Use episodes / independent perturbation sequences as statistical units. Correlated samples from one joint trace are not independent replicates.

## Prospective targets (not achieved)

- 100% exact agreement on the finite synthetic fixture corpus.
- Zero accepted cross-clock AoI without a qualified bounded mapping.
- No energy PASS when the physical boundary is incomplete.
- No thermal coverage PASS when a required node is absent.
- No reset of persistent state between sequential disturbances.
- Zero action execution beyond its calibrated decision horizon or qualified hard physical bound.
- Demonstrate lower replanning/compute/energy only at matched torque tracking and safe-success.
- Publish null results and regimes where fixed or simpler controls dominate.

## Claim boundary

The schema and fixtures qualify only synthetic contract semantics. They do not qualify clock truth, sensor calibration, actuator dynamics, thermal forecasts, energy-boundary completeness, viability, safe control or whole-robot readiness. Physical claims require calibrated instruments, independent reference channels, held-out sequential trials and exact trace provenance.


## Executable Rust reference adapter

A dependency-free Rust package now lives at `adapter/`:

- `adapter/Cargo.toml` — standalone edition-2024 package, isolated from the application crate; the wire layer pins `serde_json = 1.0.150`.
- `adapter/src/lib.rs` — typed normalized-trace adapter for causal identities, bounded clocks, AoI, command-to-motion, decision sufficiency, thermal continuity, upstream energy closure, sampling and terminal safe-stop.
- `adapter/src/wire.rs` — versioned 0.1.0 receipt envelope, canonical payload digest, duplicate-binding verifier and no-actuation invariant.
- `protocol/normalized_joint_receipt_v0_1.schema.json` — strict Draft 2020-12 schema.
- `fixtures/normalized_joint_receipt_v0_1.json` — frozen synthetic positive receipt used for serializer/report conformance.
- `adapter/Cargo.lock` — committed lockfile for reproducible `--locked` runs.

Run the reference package with the repository-pinned Rust 1.96 toolchain:

```sh
cargo fmt --manifest-path research/loco-joint-metrology-001/adapter/Cargo.toml -- --check
cargo test --locked --manifest-path research/loco-joint-metrology-001/adapter/Cargo.toml
cargo clippy --locked --manifest-path research/loco-joint-metrology-001/adapter/Cargo.toml --all-targets -- -D warnings
```

The reference source contains 29 deterministic unit tests covering complete trace closure, independent AoI and actuator response, bounded and unknown clock relations, interval uncertainty, timestamp inversions, missing thermal nodes, thermal resets and predecessor mismatch, decision regret/expiry/prefix mismatch, energy boundary incompleteness and upstream receipt rejection, sampling limits, terminal safe-stop, and causal identity/sequence integrity. These are authored tests; exact-head CI is the execution gate.

### Cross-repository normalized receipt 0.1.0

The producer branch for LOCO-JOINT-WIRE-CONTRACT-001 #5000 adds a versioned JSON receipt envelope with:
- schema ID/version and producer package identity;
- original causal identities and event sequences;
- instrument/configuration/calibration digests and expected/observed executed-prefix digests;
- bounded clock relation and timestamp intervals;
- the original decision, thermal, first-law and sampling evidence;
- the normalized report, with AoI and command-to-motion separate;
- qualified / Unknown / Inconclusive / Rejected reason-coded outcomes;
- a SHA-256 digest over the canonical payload representation;
- actuation_authorized = false as an invariant.

The canonicalization profile is RFC 8785/JCS over the receipt's JSON payload. The current producer implementation uses serde_json's sorted object map plus integer/string/boolean-only payload fields; floating-point values are excluded. The frozen positive fixture fixes the payload digest to `94137cc32ebeb908b2e0ad50ad6f05fb9836f455056d7b88eb54dffc0c290aad`. The verifier rejects payload mutation, schema/version mismatch, missing or contradictory repeated identity/calibration bindings and any actuation-authorized report.

The consumer-side Symthaea conformance adapter and shared cross-repository negative mutation corpus are separate acceptance obligations under #5000. A producer round trip alone does not prove end-to-end integration. Schema/fixture compatibility, parser verification, exact MissionTrace/instrument/first-law join, and exact-head Rust 1.96 workflow results must all pass before the wire contract is considered cross-repository qualified.
The adapter accepts typed, normalized upstream receipts. It does not parse raw DAQ files, establish transducer calibration, estimate motor/thermal state, compute first-law closure, generate causal event IDs, or authorize actuation. An energy PASS requires a complete boundary and an upstream balanced receipt. A retrospective report always has `actuation_authorized = false`. Raw traces and calibration artifacts remain separately stored and digest-bound.

A dedicated `LOCO Joint Trace Adapter (Rust 1.96)` workflow job now runs format, tests and strict Clippy directly against this standalone manifest; the main application workspace does not include it.

### Receipt-binding hardening

The normalizer requires exact equality between the observed receipt and the frozen manifest for:
- actuator-application receipt and the event's application receipt;
- source sensor calibration and the observation's calibration;
- decision ID, model digest, action-domain digest and decision-calibration receipt;
- physical-transition ID and thermal sensor calibration;
- energy-boundary manifest;
- sampling-specification digest.

A decision whose regret exceeds its declared epsilon remains a typed *insufficient* result and cannot close the mission. A mismatched receipt is Rejected; missing/overlapping timing or incomplete measurement coverage remains Unknown/Inconclusive as appropriate. This is a source-level contract claim pending the exact-head Rust 1.96 workflow, not yet a passing build claim.
