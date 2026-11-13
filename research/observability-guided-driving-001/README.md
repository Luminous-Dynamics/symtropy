# OBSERVABILITY-GUIDED-DRIVING-001

**Status:** paired-observation auditor, 18 library tests, 5 CLI tests and synthetic occluded-crosswalk fixture authored; no trained policy or simulator result  
**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4981  
**Related apprenticeship:** Symthaea #11414 / #11480  
**Related causal branch auditor:** Symtropy #4556  
**Related active experience selection:** Symtropy #4550 / #4575  
**Related capability frontier:** Symtropy #4639  
**Related fault recovery:** Symtropy #4657 / #4794

## Paradigm: drive to become observable

A driving agent should not only infer a hidden road-user state from passive observations. It should learn when a decision is under-observed and choose whether a *safe* sensing or maneuvering action could improve the information available for that decision.

At an occluded crossing, candidate actions include:
- wait behind the conflict boundary for an occluder or road user to move;
- request another observation from a sensor that actually exists in the declared vehicle configuration;
- perform a bounded, lawful repositioning maneuver that improves the sightline without entering the conflict zone;
- retain a conservative belief and yield when no admissible information action exists.

This is different from #4550. That benchmark asks **which past experience should be acquired for training**. This benchmark asks **which action should be taken now to improve the current decision's observability**.

Information seeking is always subordinate to declared safety/traffic constraints, actuator limits, time budgets and recoverability. A policy is not permitted to enter a conflict area merely to reduce its uncertainty.

## What the auditor does

The standalone Rust crate in this directory consumes paired passive-control and active-observation branch records. Each pair is pinned to:
- the same root scenario and decision tick;
- the same root world-snapshot, scenario-seed and exogenous-randomness digests;
- an independent #4556 causal branch-audit digest;
- matching evaluator-only actor-presence and crossing labels;
- a predeclared action budget.

The evaluator checks declared policy-visible field IDs against evaluator-only hidden-field IDs and rejects overlap. It checks the passive/active roles, root decision identity, action admissibility, risk/time/energy/distance budgets, no-safe-action controls and unsafe repositioning outcomes.

It computes proper-score and information metrics but does not select an action or execute a vehicle controller.

## Metrics (separate, no composite)

### Belief quality and calibration
- Brier score and log loss for actor presence and crossing-within-horizon probabilities;
- ten fixed reliability bins and expected calibration error (ECE);
- mean confidence and observed frequency, with sample counts;
- prior/post-observation versions for passive and active branches;
- separately recorded epistemic and aleatoric uncertainty fields, both bounded to [0, 1].

ECE depends on binning and sample size. Always report the fixed bin definitions, sample counts and reliability table; do not treat one ECE value as a complete calibration assessment. Brier/log loss are proper scoring rules; evaluate them on held-out, independently labelled outcomes.

### Information value
- entropy reduction for the passive control;
- entropy reduction for the active-observation branch;
- additional entropy reduction attributable to the active branch;
- policy-estimated expected information gain (reported separately from realized belief change);
- whether the active branch changed the decision from the shared root decision;
- active action count and value of information after charging full action costs.

### Costs and safety outcomes
- action duration, energy and maneuver distance;
- estimated residual risk and admissibility declarations;
- conflict-boundary encroachment, contacts/collisions and traffic-rule violations;
- no-safe-information-action violations;
- worst-case and per-scenario results once a sufficiently large campaign exists.

There is no weighted observability score. In particular, a belief improvement cannot compensate for a conflict-boundary breach, collision, rule violation, exceeded risk budget or deadline miss.

## Required experimental arms

The eventual trained-policy campaign should compare:
1. passive policy with no deliberate information action;
2. fixed conservative wait/yield rule;
3. random admissible information action;
4. entropy-greedy action selection;
5. model-predicted expected-information-gain selection;
6. risk-constrained value-of-information selection;
7. risk-constrained VOI with independent SAM recoverability gate;
8. matched GRU/LSTM temporal baseline;
9. matched compact transformer temporal baseline;
10. HDC-LTC/CfC hybrid temporal model.

The current Rust crate is an evidence auditor, not those policies. All model arms must use the same sensor permissions, action set, initial corpus, simulator budget, held-out scenario families and compute measurement method.

## Scenario coverage to add

- pedestrian occluded by a parked van at an uncontrolled crosswalk;
- cyclist occluded by a stopped delivery vehicle;
- blocked T-junction and uncertain cross traffic;
- turning vehicle masking an oncoming road user;
- roundabout entry with multiple occluders;
- nighttime, glare, rain and low-contrast visibility;
- construction zones with temporary barriers;
- asynchronous camera/radar observations;
- a road user that pauses or changes intent while occluded;
- a no-safe-information-action case where waiting/yielding is the only admissible behavior.

The evaluator truth for actor existence and crossing intention is separate from the policy-visible camera/proprioceptive observation. The policy must not receive hidden actor identity, future motion, scenario labels or evaluator-only state.

## Fair counterfactual protocol

- Freeze checkpoint, recurrent-state identity, observation history, scenario/actor/renderer revision, clock and decision boundary.
- Branch passive and active options from the same root decision.
- Re-simulate causal descendants and reciprocal actor responses after an ego intervention. Do not reuse stale actor actions after their policy inputs change.
- Share random draws only for declared exogenous variables that are invariant under the intervention, and consume #4556 branch evidence.
- Preserve original and alternate branch receipts plus complete episode/branch denominators.
- Keep evaluator-only labels outside policy inputs and the online VOI estimate.
- Measure full latency/energy/cost of sensing or repositioning, not just the model's internal inference.

## Prospective targets (not results)

1. Zero overlap between declared policy-visible and evaluator-only field IDs.
2. Zero conflict-boundary encroachment caused solely by an information-seeking action.
3. On held-out occlusion families, improve Brier/log loss and reduce false confidence against passive and entropy-only baselines at matched input/compute budgets.
4. Improve decision-relevant uncertainty per second/joule without worsening collision/contact, traffic-rule, p99 deadline, recoverability or worst-family constraints.
5. Demonstrate positive realized value of information after charging the full time, energy and maneuver cost; information actions must change a consequential decision on held-out cases.
6. Demonstrate at least one matched HDC-LTC/CfC advantage in asynchronous observation handling, actor/occluder memory, tail latency or compute efficiency versus GRU/LSTM and compact transformer baselines.

Freeze numerical calibration and safety thresholds after pilot variance analysis but before sealed evaluation. Do not select thresholds after inspecting final test outcomes.

## Build and test

Requires Rust 1.96.0:

```sh
cargo fmt --manifest-path research/observability-guided-driving-001/Cargo.toml -- --check
cargo test --locked --manifest-path research/observability-guided-driving-001/Cargo.toml
cargo clippy --locked --manifest-path research/observability-guided-driving-001/Cargo.toml --all-targets --all-features -- -D warnings
```

Audit the checked-in fixture:

```sh
cargo run --locked \
  --manifest-path research/observability-guided-driving-001/Cargo.toml -- \
  research/observability-guided-driving-001/fixtures/occluded-crosswalk-paired-v1.json \
  /tmp/observability-report.json
```

Exit codes are 0 for structurally valid records, 1 for a rejected campaign (report still written), and 2 for input/invocation errors. The CLI bounds the input at 64 MiB and typed deserialization rejects unknown fields.

## Claim ceiling

A passing unit/fixture/CLI test establishes only that declared finite branch records, paired labels, calibration scores and action-cost constraints are audited as implemented. It does not authenticate simulator truth, establish a trained agent's semantic visual perception, prove physical causal validity, demonstrate safe driving, establish real-world crash-risk reduction or support public-road readiness.
