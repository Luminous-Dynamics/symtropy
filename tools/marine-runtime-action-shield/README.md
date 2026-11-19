# Marine Runtime One-Step Action Shield

Standalone Rust evaluator for Symtropy #6269. It is a read-only permit/deny/abstain gate; it has no actuator interface.

## Command

```sh
cargo fmt --manifest-path tools/marine-runtime-action-shield/Cargo.toml -- --check
cargo clippy --manifest-path tools/marine-runtime-action-shield/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/marine-runtime-action-shield/Cargo.toml
cargo run --manifest-path tools/marine-runtime-action-shield/Cargo.toml -- audit-action-set tools/marine-runtime-action-shield/fixtures/synthetic-primary-rejected-fallback-permitted-0001.json
```

## Decision semantics

- Proposals are ordered by explicit priority and must bind an exact current-state digest, transition-model digest, operating-domain digest, action-value digest and permit expiry.
- Each proposal includes a predicted successor envelope for every member of the digest-bound five-scenario roster.
- The shield checks all eleven service/resource axes with conservative uncertainty endpoints: lower bounds for minimum resources and upper bounds for maximum exposures.
- Missing scenario, missing metric, stale state, mismatched model/ODD, changed action values, invalid successor digest or excessive permit TTL is never treated as safe.
- First safe primary/fallback in priority order receives a time-limited permit receipt; if none is safe, the decision is `ABSTAIN`.
- The permit receipt binds action ID/digest, state sequence/digest, model, ODD, service contract, scenario roster and expiry.
- `actuation_performed=false` always. The executor must persist and consume `permit_nonce` exactly once and reconcile unknown execution outcomes before a new action. This stateless tool does not enforce cross-process replay prevention.

The synthetic fixture rejects a cruise action whose energy/water successor bounds fail under adverse and outage scenarios, then permits a reserve-preserving fallback that passes all five scenario envelopes.

## Scientific and operational limits

The shield trusts a declared model-predicted successor envelope after verifying its identity and service-floor bounds. It does not independently validate the dynamics, actuator behavior, model calibration, control barrier condition, recursive feasibility or inter-sample safety. It is not a complete CBF/robust-MPC proof, a navigation certificate or an operational control system. All input models and fixtures are synthetic-only.
