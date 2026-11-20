# Marine Runtime Action Shield

Standalone Rust 1.96-compatible, read-only one-step action admission evaluator for Symtropy issue #6269. It separates a planner's proposal from the independent decision on whether that proposal is admissible under the current evidence, current state and declared disturbance set.

## Commands

```sh
cargo fmt --manifest-path tools/marine-runtime-action-shield/Cargo.toml -- --check
cargo clippy --manifest-path tools/marine-runtime-action-shield/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/marine-runtime-action-shield/Cargo.toml
cargo run --manifest-path tools/marine-runtime-action-shield/Cargo.toml -- audit-action tools/marine-runtime-action-shield/fixtures/synthetic-proposed-action-permit-0001.json
cargo run --manifest-path tools/marine-runtime-action-shield/Cargo.toml -- audit-action tools/marine-runtime-action-shield/fixtures/synthetic-fallback-action-permit-0001.json
cargo run --manifest-path tools/marine-runtime-action-shield/Cargo.toml -- audit-action tools/marine-runtime-action-shield/fixtures/synthetic-no-safe-action-abstain-0001.json
```

## Planner / shield / executor boundary

1. **Planner** proposes one action plus any predeclared fallbacks. It cannot change the service contract, hard-gate roster, disturbance roster, model or operating domain.
2. **Shield** independently validates the current state sequence/digest and freshness, exact craft/actuator/model/ODD identity, action preconditions, all five disturbance successors, conservative service/resource intervals, hard-gate receipts, capabilities and minimum recovery/intervention margin.
3. **Fallback order** is frozen in the bundle. The evaluator selects the first safe fallback only after rejecting the proposed action.
4. **Executor** is external and separately authorized. This crate never invokes an actuator, issues a helm command or changes vessel state.

Possible decision results:
- `PERMIT_PROPOSED_ACTION`: proposed action passes all one-step checks.
- `PERMIT_FALLBACK_ACTION`: proposed action fails; first predeclared fallback passes.
- `ABSTAIN_NO_SAFE_ACTION`: no proposed or fallback action preserves the declared envelope.
- `ABSTAIN_CURRENT_STATE_NOT_ADMISSIBLE`: state is stale, expired, replayed or already outside its admitted service envelope.
- `INVALID_INPUT`: evidence identity, canonical digest, scenario coverage or contract structure is invalid.

The output is a decision receipt only. Its SHA-256 digest proves record identity, not signer identity or authority. The receipt explicitly records `executor_authorization: NONE`. A real executor must authenticate the authority, atomically consume the nonce, recheck the latest state sequence and action digest, and reconcile unknown execution outcomes before accepting another action.

## One-step robust check

For current state interval (X_t), proposed action (u_t), declared disturbance set (W_t), and model relation (F), the bundle supplies a conservative one-step successor envelope for every required scenario:

`X_{t+1} = union_{x in X_t, w in W_t} F(x,u_t,w)`

The evaluator does not derive the reachable set from dynamics. It checks the supplied evidence-bearing envelope against:
- all eleven non-compensatory service/resource floors, using the conservative lower bound for minimums and upper bound for maximums;
- every declared hard gate, where only `pass` with an evidence digest is accepted;
- craft, actuator, model and operating-domain identity;
- action preconditions against the exact monotonic state sequence and digest;
- predeclared recoverability status and the minimum time margin left after the required detection/replanning/communication/human-intervention budget.

A point prediction or a nominal-only successor is insufficient. Unknown, missing or stale evidence cannot produce a permit. The five disturbance classes are the same marine suite used by the trajectory and inter-sample auditors: nominal, adverse forecast error, generator outage, watermaker outage and common-cause generation/watermaker outage.

## Synthetic known-answer corpus

- `synthetic-proposed-action-permit-0001.json`: proposed action passes all five scenarios; expected `PERMIT_PROPOSED_ACTION`.
- `synthetic-fallback-action-permit-0001.json`: proposed action violates the energy reserve floor under adverse forecast error; first fallback `fallback-reduce-speed` passes and is selected.
- `synthetic-no-safe-action-abstain-0001.json`: proposed action fails freshwater under common-cause outage; one fallback fails energy under generator outage and the next fails a hard gate; expected `ABSTAIN_NO_SAFE_ACTION`.

All measurements, models, disturbance envelopes, hard-gate receipts and authority references are synthetic. The fixtures prove deterministic software behavior only.

## Evidence and safety limits

- The supplied transition/reachable envelopes and model/ODD digests are declared inputs. This tool does not independently validate hydrodynamics, actuator response, disturbance completeness or physical model error.
- A SHA-256 digest is not a digital signature. The executor must authenticate and atomically consume permits in a trusted ledger; this CLI does not persist nonce consumption.
- One-step admission does not prove recursive feasibility. The selected successor must be re-evaluated after new qualified telemetry, and a validated fallback controller is still required.
- No route guidance, stability certification, collision avoidance, helm interface, control actuation or navigation authority is included.
