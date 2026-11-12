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

Without sensing, the only physical policy feasible in every world is `repair`. Its minimax regret is 15 service units. The evaluator also includes a separate two-world control where no single blind action is feasible in every world, but a discriminating sensor makes one robust action feasible in each outcome partition. In that case, the report marks `resolves_no_sensing_infeasibility` rather than inventing a finite regret reduction against a nonexistent baseline policy.

Four sensing actions are evaluated:

1. **Remote telemetry** costs 0.5 kWh, converted at a declared 2 service_units/kWh, and takes two ticks. It reports `healthy` in the healthy world but `missing` in both (a) degraded service with common-cause power loss and (b) healthy service with a telemetry-network fault. The two hidden causes are evaluator-only; the candidate receives the same `missing` token. Their per-tick service losses are 3 and 2 units, respectively. The missing-outcome partition remains ambiguous, giving worst-case post-sensing regret 20, worse than the no-sensing 15.
2. **Independent field inspection** has the same declared energy conversion but one tick of latency. It separates degraded service from the two healthy worlds. Its maximum acquisition penalty is 4 service units, and worst-case post-sensing regret falls to 4. The robust regret reduction is 11, so it is the selected sensing action.
3. **Uninformative status check** returns the same `status_unknown` result in all worlds. Its worst-case regret stays at 15 and it has zero decision value.
4. **Over-budget lab test** is perfectly informative but costs 2 kWh, above the 1 kWh budget. It is not recommended.

The reported regret is measured against the best feasible physical policy in each world **without** paying to sense. The sensor's energy cost and world-specific latency service loss are then charged against the chosen policy. Therefore the regret calculation captures both what was learned and what was lost while acquiring the information.

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

The deterministic sweep records no-sensing policy/regret; energy cost and conversion; acquisition latency and worst world-specific service loss; maximum total penalty; worst post-sensing regret and regret reduction; each observation partition and selected action; and evaluator-side prediction receipts including hidden missingness causes. These audit rows are produced from a separate evaluator API and are not part of the candidate-facing decision report.

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
```

## Claim ceiling

This is a synthetic known-truth protocol reference. It does not implement a general POMDP solver, establish real sensor performance, identify real causal mechanisms, validate field data or claim SOTA superiority.
