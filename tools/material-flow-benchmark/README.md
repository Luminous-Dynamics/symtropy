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

The procedure graph describes what evidence each step needs. The separate `mfb-admit-procedure-evidence` CLI checks a synthetic bundle of those receipts against the exact scenario and procedure version.

It currently enforces:
- exact procedure ID/version and scenario ID;
- exact source commit and configuration digest on every receipt;
- one receipt for every externally-supplied evidence kind, with no unexpected or duplicate kinds/identities;
- evidence-kind-specific subject binding (package, tote, receiver endpoint, transfer mechanism, grant or scenario);
- issuer actor existence and allowed actor role for that evidence kind;
- SHA-256 digest syntax;
- freshness policy v1: receipt observation must precede the **earliest** plausible decision time; expiry must cover the **latest** plausible decision time; and every evidence kind has a versioned maximum observation age, measured at that latest plausible time.

Version 1 maximum ages (milliseconds):

| External evidence kind | Maximum age |
|---|---:|
| Manifest snapshot | 300,000 |
| Authorization receipt | 60,000 |
| Receiver capability receipt | 86,400,000 |
| Package identity observation | 2,000 |
| Container fit observation | 86,400,000 |
| Payload mass observation | 2,000 |
| Seal observation | 5,000 |
| Temperature observation | 2,000 |
| Transfer-gate observation | 500 |
| Release-execution receipt | 10,000 |
| Receiver-presence observation | 500 |
| Placement observation | 1,000 |
| Counterparty custody receipt | 30,000 |
| Ambiguous-outcome observation | 5,000 |

Run it from the repository root:

```bash
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml --bin mfb-admit-procedure-evidence -- \
  tools/material-flow-benchmark/fixtures/nominal-stationary-carrier.json \
  tools/material-flow-benchmark/fixtures/nominal-receiving-procedure.json \
  tools/material-flow-benchmark/fixtures/nominal-receiving-evidence.json
```

The current synthetic bundle contains 14 external evidence envelopes. The admission report explicitly sets `cryptographic_authentication_verified=false`, `sensor_truth_established=false`, `motion_authority_granted=false`, `physical_command_emitted=false`, and `task_success_attested=false`.

This is a structural receipt-admission gate, not a cryptographic verifier. It does not check signatures, revocation, hardware attestation, sensor truth, evidence-source independence or actual physical state. Do not equate an admitted envelope with a trusted observation or authorized actuation. CI archives the raw admission report and hashes it with the other MFB evidence outputs.

### Exact-head receipt-admission qualification (2026-11-16)

Dedicated workflow [43823753266](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43823753266) passed on source head `386fc63ea84561f072ec58804154cb02f0d4b8d9`: **76 Rust tests**, strict formatting, Clippy `-D warnings`, protocol campaign, procedure graph, 14-receipt admission, four-timestep convergence, 480 Hz L1 simulation/replay and evidence upload.

The report admitted **14/14** external evidence envelopes under freshness policy v1. It kept `cryptographic_authentication_verified=false`, `sensor_truth_established=false`, `motion_authority_granted=false`, `physical_command_emitted=false` and `task_success_attested=false`.

Artifact: `mfb-l1-evidence-386fc63ea84561f072ec58804154cb02f0d4b8d9` (ID `14205746262`), six files, ZIP SHA-256 `634d4ad1001fe0a5590cfdb9a77aa97c1df1b1f7281a8be35260da4b61b3593d`. SHA-256 outputs: receipt admission `cc8143b13d5e8820f456b61bc9153427e5f588c0e2d58ef091cc8001b739e8b0`; protocol campaign `bc21a056a4fef5c097567663fa020c9ed119094bad3a11b9764e7ef2b17dea6b`; procedure readiness `051507417022eca1705a4c1e200fc315229109e08100296bf36c1ee643341ed2`; convergence `d53fdc1130f7de4b1cdf75658d7af738a1b141bd1b5db4fbc539be5272549631`; L1 result `cd836f52f70483b4375e4aa39bca81be14b9546a6b4cf24ba607458501e85070`; Cargo.lock `9ec0cf115298a909b85593b829bd0966a9cc3bd67636cf691f8132f7c30fd654`.

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
