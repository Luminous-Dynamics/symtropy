# CIVDYN active-sensing reference v0.1

This isolated, dependency-free Rust crate implements a deterministic finite-model benchmark in which a sensing request is a costly action, not a free observation.

It composes:
- [CIVDYN-STATE-OBSERVABILITY-001 #4759](https://github.com/Luminous-Dynamics/symtropy/issues/4759) for state-feature missingness and belief-state planning;
- [CIVDYN-DECISION-VOI-001 #4620 / PR #4624](https://github.com/Luminous-Dynamics/symtropy/pull/4624) for prior-free experiment selection by worst-case decision-regret reduction.

It intentionally owns a different question from PR #4624: how does the sensing action's own energy and delay change service before the next control decision?

## Frozen three-world fixture

The candidate chooses between two physical policies, `defer` and `repair`, with a 70 service-unit hard floor.

| Hidden world | Defer | Repair | Clairvoyant choice |
|---|---:|---:|---|
| healthy | 100 | 85 | defer |
| degraded | 0 (infeasible) | 80 | repair |
| healthy_sensor_fault | 100 | 85 | defer |

Without sensing, the only physical policy feasible in every world is `repair`. Its minimax regret is 15 service units. The evaluator also includes a separate two-world control where no single blind action is feasible in every world, but a discriminating sensor makes one robust action feasible in each outcome partition, with its 0.5-service-unit energy cost charged to regret. In that case, the report marks `resolves_no_sensing_infeasibility` rather than inventing a finite regret reduction against a nonexistent baseline policy.

Four sensing actions are evaluated:

1. **Remote telemetry** costs 0.5 kWh, converted at a declared 2 service_units/kWh, and takes two ticks. It reports `healthy` in the healthy world but `missing` in both (a) degraded service with common-cause power loss and (b) healthy service with a telemetry-network fault. The two hidden causes are evaluator-only; the candidate receives the same `missing` token. Their per-tick service losses are 3 and 2 units, respectively. The missing-outcome partition remains ambiguous, giving worst-case post-sensing regret 20, worse than the no-sensing 15.
2. **Independent field inspection** has the same declared energy conversion but one tick of latency. It separates degraded service from the two healthy worlds. Its maximum acquisition penalty is 4 service units, and worst-case post-sensing regret falls to 4. The robust regret reduction is 11, so it is the selected sensing action.
3. **Uninformative status check** returns the same `status_unknown` result in all worlds. Its worst-case regret stays at 15 and it has zero decision value.
4. **Over-budget lab test** is perfectly informative but costs 2 kWh, above the 1 kWh budget. It is not recommended.

The reported regret is measured against the best feasible physical policy in each world **without** paying to sense. The sensor's energy cost and world-specific latency service loss are then charged against the chosen policy. Therefore the regret calculation captures both what was learned and what was lost while acquiring the information.

## Non-myopic two-stage sensing trap

The sequential reference is a separate binary and API, not a pooled score with the single-sensor fixture. It tests a deliberately non-myopic sequence:

1. **Inspection-rig setup** costs 0.1 kWh (0.2 service units) and one tick (0.3 service units of delay loss per world). It returns the same candidate-visible `ready` observation in both worlds. By itself it does not resolve the absence of a common feasible physical action, so the myopic stage-one evaluator rejects it.
2. **Fine inspection** is admissible only after the first stage returns `ready`. It costs 0.15 kWh (0.3 service units) and one tick (0.4 service units of delay loss per world). It returns distinct sensor traces, `trace_alpha` and `trace_beta`, which permit the controller to choose `repair_a` or `repair_b` safely.

The full sequence costs 0.25 kWh, 2 ticks, 0.5 service units in energy and 0.7 service units in service loss during acquisition. The combined 1.2-service-unit penalty is charged against the original world oracle, leaving worst-case regret 1.2 while restoring a feasible action in each final observation-history partition. Because no common blind action existed, the evaluator reports **feasibility lift** and leaves regret reduction undefined; it does not invent a baseline regret.

This is a direct non-myopic value-of-information control: the first observation has no immediate decision value, but is necessary to unlock the second measurement. An uninformative second measurement still leaves the action set infeasible; a cost-over-budget sequence is not recommended. The second stage declares and checks its exact first-stage prerequisite.

Run the evaluator and the four-field candidate-only policy map:

```bash
cargo run --locked --manifest-path research/civdyn-active-sensing/Cargo.toml --bin sequential-active-sensing-sweep
cargo run --locked --manifest-path research/civdyn-active-sensing/Cargo.toml --bin sequential-active-sensing-sweep -- --candidate-policy-map
```

The evaluator emits per-partition/per-world costs and regret. The candidate-only export contains only `scenario_id, plan_id, observation_history, action_id`; it has no true-world identity or evaluator regret fields.

## Core contract

- No prior probabilities and no model-count weighting.
- Candidate partitions are keyed only by the observed categorical outcome.
- The candidate-facing ActiveSensingReport contains no missingness-cause field. Hidden causes are emitted only by the separate evaluator_prediction_audit API.
- Evaluator-only missingness causes are retained in separate audit receipts and never used as candidate-visible distinctions.
- Energy and service units are combined only through the explicit `service_units_per_kwh` coefficient.
- Latency loss is model-specific and multiplies declared acquisition ticks.
- A physical policy that falls below the hard service floor after sensing is infeasible.
- If any observation partition has no common feasible action, the sensing action receives no finite worst-case regret and cannot be recommended.
- If no action is feasible before sensing, an affordable sensor may still be recommended when every post-sensing outcome partition has a common feasible action. This is reported as a feasibility lift, not as a fabricated finite regret reduction.
- Missing/duplicate model predictions, digest mismatches, malformed missingness states, invalid costs and duplicate sensor IDs fail closed.
- Recommendations require affordable sensing and a positive worst-case regret reduction; ties resolve by lower maximum penalty, lower energy cost, then stable sensor ID.

## Metrics

The default deterministic sweep records the full evaluator audit. The separate `--candidate-policy-map` mode writes only `scenario_id,sensor_id,observation_id,action_id`, for safe policy execution or external controller adapters. The evaluator sweep records no-sensing policy/regret; energy cost and conversion; acquisition latency and worst world-specific service loss; maximum total penalty; worst post-sensing regret and regret reduction; each observation partition and selected action; and evaluator-side prediction receipts including hidden missingness causes. These audit rows are produced from a separate evaluator API and are not part of the candidate-facing decision report.

These are separate quantities, not a composite information score.

## Literature comparison

- **IJCAI 2026 Missingness-MDPs** models unknown state-feature missingness in a POMDP with a known transition model and learns the missingness function under MCAR, simple-MAR and selected MNAR assumptions. It supplies a natural observation-model learning/planning comparator, but does not by itself qualify infrastructure service loss while acquiring a measurement: https://ijcai.org/proceedings/2026/692
- **AISTATS 2026, MDPs with a State Sensing Cost** explicitly includes a decision of when to sense versus remain blind, using a discounted-cost MDP and lower bounds on optimal value / policy suboptimality: https://proceedings.mlr.press/v300/kapoor26a.html
- **Van Pelt et al. (August 2026), Balancing time and precision with active sensor selection and positioning** treats sensor modality as an action with precision/time trade-offs and a finite time budget: https://wmkouw.github.io/publication/multimodal-aif-sensing/
- **From Reactive to Active Sensing** surveys reactive, single-agent and multi-agent information gathering, and notes the lack of a common modeling framework across application domains: https://doi.org/10.1145/3583068

CIVDYN's proposed extension is to combine cost-aware sensing with policy-dependent MNAR/common-cause failure, infrastructure degradation during acquisition, hard service floors and prior-free worst-case decision regret. This v0.1 does not reproduce any of these papers.

## Run

```bash
cargo test --locked --manifest-path research/civdyn-active-sensing/Cargo.toml
cargo run --locked --manifest-path research/civdyn-active-sensing/Cargo.toml --bin active-sensing-sweep
cargo run --locked --manifest-path research/civdyn-active-sensing/Cargo.toml --bin active-sensing-sweep -- --candidate-policy-map
```

## Contingent sensing trees (prior-free robust policy planning)

The crate also contains an exact bounded policy-tree evaluator in `src/contingent.rs`. This is a different uncertainty contract from the Bayesian adaptive passive-measurement planner in [#4027 / PR #4031](https://github.com/Luminous-Dynamics/symtropy/pull/4031):

| Property | CIVDYN contingent tree | #4027 sequential diagnosis |
|---|---|---|
| Uncertainty | Finite set of possible worlds, no weights | Explicit prior and likelihoods |
| Objective | Minimax service regret subject to hard floors | Expected utility net of measurement cost |
| Observation branches | Public-token partitions over compatible worlds | Bayesian posterior branches |
| Sensing costs | Per-world energy + delay losses, charged per path | Declared experiment costs in utility units |
| Stopping | STOP competes with every sensor at every node | STOP competes with expected continuation value |
| Candidate information | Observation history, sensor IDs, actions only | Public experiment/outcome policy tree |
| Exactness | Bounded deterministic enumeration | Bounded exact Bayesian finite-horizon recursion |

The deterministic three-world challenge has a cheap first-stage screen. It returns `healthy` in one world and `missing` in two worlds: degraded service with a common-cause power failure and healthy service with a telemetry-network fault. Both fault causes yield the same public `missing` token. The candidate cannot tell those two causes apart.

The selected policy tree is:

1. Request `screen`.
2. If the observation is `healthy`, STOP and `defer`.
3. If the observation is `missing`, request `independent_inspection`.
4. If the follow-up is `degraded`, STOP and `repair`; if it is `healthy`, STOP and `defer`.

The independent inspection is charged only on the `missing` branch. Known-answer values:

| Quantity | Result |
|---|---:|
| No-sensing minimax regret | 15 service units |
| Contingent-tree worst leaf regret | 1.4 service units |
| Worst-case regret reduction | 13.6 service units |
| Maximum path energy | 0.35 kWh |
| Maximum path latency | 2 ticks |
| Maximum path acquisition penalty | 1.4 service units |
| Selected tree depth | 2 sensors |

A one-step greedy screen is not recommended: its worst regret is 15.5, worse than STOP at 15. An uninformative follow-up is not chosen; an over-budget follow-up is skipped; stopping wins a regret tie to avoid paying for unnecessary sensing. The search has a hard 10,000-node cap (depth at most 3, sensor catalogue at most 8) and fails closed rather than switching to a heuristic.

### Matched fixed-sequence comparator on the same worlds

The sweep also evaluates a **fixed two-sensor sequence** on the identical three-world fixture. It requests both `screen` and `independent_inspection` in every world, then chooses the physical action from the pair of public observations. The policy and evaluator use the same sensor protocols, world outcomes, service floors and energy/latency conversion as the contingent tree.

| Hidden world | Fixed sequence observations | Fixed action | Fixed penalty | Fixed regret | Contingent action | Contingent penalty | Contingent regret |
|---|---|---|---:|---:|---|---:|---:|
| Healthy | healthy → healthy | defer | 1.1 | 1.1 | defer after first screen | 0.6 | 0.6 |
| Degraded | missing → degraded | repair | 1.4 | 1.4 | repair after inspection | 1.4 | 1.4 |
| Healthy, sensor-network fault | missing → healthy | defer | 0.9 | 0.9 | defer after inspection | 0.9 | 0.9 |

Both policies have worst-case regret **1.4 service units** and the same worst-path acquisition envelope: **0.35 kWh, two ticks, 1.4 service units**. The contingent tree saves the second acquisition only on the clear healthy branch: **0.15 kWh and 0.5 service units** on that path, with no change to the other two paths.

This is deliberately a *pathwise* result, not an expected-value claim. No observation probabilities are assigned, so there is no claim about average savings across worlds. On this fixture the contingent policy does not beat the fixed sequence on worst-case regret; it weakly preserves the worst-case result while avoiding an unnecessary measurement on a branch already sufficiently identified. The CI contract checks the fixed and contingent per-world results against the same frozen outcomes.

### False-reassurance / stuck-sensor challenge

A second executable four-world challenge prevents the first-stage screen from being treated as infallible:
- a degraded system can report `healthy` because the sensor is stuck high;
- a healthy system can report `missing` because of a telemetry-network fault;
- a true degraded/common-cause failure also reports `missing`.

The policy must buy different corroborating measurements on the two screen branches:
- after `healthy`, request `stuck_high_confirmation`;
- after `missing`, request `independent_missing_branch_inspection`.

Both follow-ups have the same public role—distinguish healthy from degraded—but different protocol identities and branch eligibility. The resulting terminal policy repairs either degraded world and defers in either healthy world.

| Metric | False-reassurance challenge |
|---|---:|
| No-sensing minimax regret | 15 service units |
| Contingent-tree worst leaf regret | 0.6 service units |
| Worst-case regret reduction | 14.4 service units |
| Maximum path energy | 0.20 kWh |
| Maximum path latency | 2 ticks |
| Maximum path acquisition penalty | 0.6 service units |

This is an adversarial protocol fixture, not a statistical false-negative-rate estimate. It proves only that the policy tree does not treat a nominally healthy screen reading as ground truth when the declared world set includes a stuck-high sensor.

### Candidate and evaluator exports

The evaluator CLI writes separate records for policy nodes, branch predictions, and terminal-world regrets/costs. The candidate-only mode `--candidate-policy-tree` writes only six public fields:

`scenario_id, observation_history, node_type, sensor_id, observation_id, action_id`

It contains no world ID, true service outcome, hidden missingness cause, clairvoyant value, energy receipt, or regret. The candidate projection is a distinct type in Rust; hidden causes and per-world receipts remain evaluator-only.

### Research boundary

This prior-free tree complements, but does not replace:
- #4027, the Bayesian finite-horizon measurement/STOP planner;
- #3018, joint stochastic physical-state and sensor-state experimental design;
- #4620, one-step prior-free experiment discrimination;
- the active-sensing v0.1 one-shot and fixed two-stage references.

The claim is not that minimax is universally better than Bayesian decision-making. These methods answer different questions. The benchmark makes the trade-off explicit: worst-case safety and service regret without a model prior versus expected utility when an approved prior/likelihood model exists. The three-world fixture is synthetic only and establishes no external or field effectiveness.

Run the contingent tree:

```bash
cargo run --locked --manifest-path research/civdyn-active-sensing/Cargo.toml --bin contingent-active-sensing-sweep
cargo run --locked --manifest-path research/civdyn-active-sensing/Cargo.toml --bin contingent-active-sensing-sweep -- --candidate-policy-tree
```

## Claim ceiling

This is a synthetic known-truth protocol reference. It does not implement a general POMDP solver, establish real sensor performance, identify real causal mechanisms, validate field data or claim SOTA superiority.
