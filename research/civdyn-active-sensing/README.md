# Maritime active-sensing and contingent-recourse reference v0.1

This is the first executable maritime application case for CIVDYN-ACTIVE-SENSING-001 (#4782). It composes the observation-action boundary with the existing maritime evidence owners and the bounded recovery programme. It is not a new event matcher, dependency graph, generic VOI engine, terminal optimizer, or operational controller.

## Research question

For a small port-call disruption, should the decision-maker:

1. act on current evidence;
2. acquire another observation, paying its energy and delay cost; or
3. abstain because the evidence cannot support the required claim?

The objective is to minimize worst-case downstream service regret after the sensing action's declared cost. No probabilities are assigned to model worlds, no model-count priors are used, and no information-gain score substitutes for decision value.

## Composition and ownership

- Symtropy #7009 / MARITIME-PHYSICAL-EVENT-ASSOCIATION-001: consume a unique physical-event correspondence disposition; this crate does not match events.
- Symtropy #7081 / MARITIME-CHANNEL-DEPENDENCY-BINDING-001: consume a role × stage × dimension dependency disposition; this crate does not discover dependencies or assert independence itself.
- Symtropy #6971: consume capture-completeness and channel-qualification results.
- CIVDYN-DECISION-VOI-001 #4620: the fixed-experiment, prior-free decision-regret framework is the conceptual and protocol owner. This crate does not claim Bayesian EVSI or create a second general experiment selector.
- CIVDYN-ACTIVE-SENSING-001 #4782: owns the cost-bearing sensing-action extension. Energy and acquisition latency reduce service value before recourse is selected.
- MARITIME-REC-001 #1867 and MARITIME-JIT-001 #1858: own the bounded recourse and port-call planning context. This fixture only selects from a tiny predeclared action catalogue; it does not replace either planner.

The evidence receipt strings in this fixture are synthetic identifiers. They are not cryptographic digests, signatures, authenticated receipts or real upstream API objects. The gate enums represent already-computed owner dispositions, not a local reimplementation of those owners.

## Frozen synthetic case

Three hidden worlds are declared:

| World | Primary berth | Reserve option | Clairvoyant service |
|---|---|---|---:|
| primary_ready | Ready | Available | 100 |
| primary_delayed_reserve_ready | Delayed | Available | 90 |
| primary_delayed_reserve_unavailable | Delayed | Unavailable | 20 |

Candidate recourse actions:

| Action | Ready world | Delayed / reserve ready | Delayed / reserve unavailable |
|---|---:|---:|---:|
| maintain_primary | 100 | 20 | 20 |
| use_reserve | 80 | 90 | Infeasible |
| hold | 0 | 0 | 0 |

The no-observation baseline must choose one action feasible in all worlds. Its minimax-regret choice is maintain_primary, with 70 service_units worst-case regret.

### Observation alternatives

| Observation | Evidence ceiling | Energy | Delay | Net result |
|---|---|---:|---:|---|
| no_observation | No new evidence | 0 kWh | 0 ticks | 70 regret |
| shared_tos_signal | Advisory only; shared gateway | 1 kWh | 0 ticks | 71 regret |
| independent_field_inspection | Independent observation candidate | 2 kWh | 1 tick | 5 regret |
| unknown_dependency_feed | Not admissible | 0 kWh | 0 ticks | Blocked |
| ambiguous_event_camera | Not admissible | 0 kWh | 0 ticks | Blocked |
| stale_field_sensor | Not admissible | 0 kWh | 0 ticks | Blocked |
| overbudget_independent_audit | Above budget | 4 kWh | 2 ticks | 12 service_units; budget is 8 |

Conversion is explicit and synthetic: 1 service_unit per kWh and 3 service_units per acquisition-delay tick. The independent inspection therefore costs 2 + 3 = 5 service_units. Its three distinct, qualified outcomes select maintain_primary, use_reserve, and maintain_primary respectively. The reserve action is never admitted in the world where reserve capacity is unavailable.

The shared TOS signal distinguishes a ready primary berth from a missing report, but the two delayed worlds remain in the same observation partition. Its dependency is known to be shared, so it is available only as advisory telemetry and cannot be promoted to independent corroboration. Its 1-service-unit cost makes its worst-case regret 71, worse than taking no observation.

The unknown-dependency, ambiguous-association and stale-evidence controls are blocked before they can partition worlds. The over-budget inspection is not recommended even though its synthetic outcome predictions perfectly discriminate all three worlds.

## Reproduction

From the repository root:

```sh
cargo fmt --manifest-path research/civdyn-active-sensing/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-active-sensing/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path research/civdyn-active-sensing/Cargo.toml --all-targets
cargo run --quiet --locked --manifest-path research/civdyn-active-sensing/Cargo.toml --bin maritime-active-sensing
```

The executable emits a deterministic CSV receipt. CI runs it twice and compares the output byte-for-byte.

## Claim ceiling

This is synthetic finite-state software evidence only. It demonstrates that, in this frozen toy case, a cost-bearing independent inspection can reduce worst-case decision regret from 70 to 5 service_units while shared, stale, ambiguous, unknown and over-budget observations are not misrepresented as independent evidence or selected.

It does not establish field efficacy, statistical superiority, causal identification, real port performance, operational safety, delivery of cargo, or permission to execute a recovery action. A unique event match is not a cargo delivery proof. An independent observation is not a custody receipt. A selected recovery action is only a proposal over synthetic, predeclared alternatives.

## Next qualification gates

1. Replace synthetic gate values with versioned read-only receipts from the owners of #6971, #7007 and #7037, without forking their logic.
2. Expand to missing-not-at-random observations, shared outage causes, delayed/failed inspections, revoked permissions and observation actions that alter physical state beyond a scalar service penalty.
3. Add a second contingent disruption and a scarce reserve resource; test simultaneous claims and participant-level displacement under #1867.
4. Compare no observation, shared-source telemetry, independent inspection, fixed rules and clairvoyant evaluator-only oracle on the same frozen worlds.
5. Add generated small cases and an independent exhaustive oracle; do not promote a scenario family based only on hand-authored fixtures.
6. Only after source rights, shadow-mode permissions and independent adjudication are established should real terminal/receiver outcomes be introduced.

Simulation PASS is not physical qualification or authority to operate.
