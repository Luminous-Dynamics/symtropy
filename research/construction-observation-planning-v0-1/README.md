# Construction Observation Planning Core — v0.1

**Status:** synthetic active-observation selector and scenario-wise value-of-information evaluator. Not a sensor model, survey solution, inspection authority, safety controller or field productivity system.

Parent: [CONSTRUCTION-OBSERVATION-PLANNING-001 #4988](https://github.com/Luminous-Dynamics/symtropy/issues/4988).

Stacked dependency order:
[interop + resource temporal index #5009](https://github.com/Luminous-Dynamics/symtropy/pull/5009)
→ [accepted-floor ledger #5043](https://github.com/Luminous-Dynamics/symtropy/pull/5043)
→ [space-time occupancy #5044](https://github.com/Luminous-Dynamics/symtropy/pull/5044)
→ this observation-planning adapter.

## Implemented v0.1

- Declares observation actions with coverage-profile and calibration references, regions, a space-time access envelope, duration, crew time, cost and energy.
- Provides geometry-only baseline selectors and resource-aware selector variants.
- The targeted policy uses a deterministic lexicographic rule: maximize currently blocked work units covered, then minimize crew minutes, cost, duration and action ID.
- Resource-aware variants evaluate spatial conflict through the existing occupancy owner and call `ReservationBook::preview_reservation` for time/clock/expiry/capacity feasibility. No second resource calendar or geometry checker is introduced.
- Spatial conflicts and resource-calendar rejections are reported separately with exact typed reasons. A blocked high-coverage scan can be replaced by a lower-coverage feasible scan; the fixed resource-aware baseline never substitutes another action.
- The read-only preview does not mutate reservations. A selected scan must be re-admitted by the owner at execution/commit time; preview is not a lock or durable reservation.
- Keeps hidden scenario truth in a separate scenario oracle, never in the planner's selection inputs.
- Evaluates each scenario separately; there are no invented probabilities or expected-value averages.
- Calculates `net_voi = no_observation_loss - observation_cost - residual_loss_after_result` in integer minor currency units.
- Reports observation duration, crew minutes and energy separately from monetary cost and residual scenario loss.
- Distinguishes qualified clear, finding, incomplete, invalidated and unavailable outcomes.
- A qualified clear only makes work `ReadyForIndependentAcceptance`; findings require professional/site disposition; incomplete, stale or unavailable results remain `HeldUnknown`.
- A missing scenario result is a typed error, not a clear outcome.

## Local commands

```sh
cargo fmt --manifest-path research/construction-observation-planning-v0-1/Cargo.toml -- --check
cargo test --manifest-path research/construction-observation-planning-v0-1/Cargo.toml --locked
cargo clippy --manifest-path research/construction-observation-planning-v0-1/Cargo.toml --locked --all-targets -- -D warnings
```

## Interpretation

The scenario oracle supplies explicit no-observation and residual-loss figures for a frozen synthetic case. They are scenario inputs, not probability-weighted estimates, sensor detection rates or project cost estimates. A positive VOI is useful only within that declared case; negative VOI is a valid result.

This adapter records references and consumes the occupancy index and resource calendar. It does not compute coverage, verify calibration, authenticate evidence, update survey adjustments, run a full resource-constrained project schedule or authorize work. Only qualified external observation/acceptance owners can establish those facts.

## Claim ceiling

V0.1 establishes only deterministic synthetic policy semantics. It does not establish real sensor performance, survey accuracy, safety, inspection validity, access authorization, construction productivity, cost savings or suitability for a real site.
