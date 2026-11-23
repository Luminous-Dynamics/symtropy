# Multi-job Multi-period Manufacturing Policy Reference v0.1

**Owner:** Symtropy issue [#7168](https://github.com/Luminous-Dynamics/symtropy/issues/7168)  
**Implementation:** draft research branch `research/mfg-multiperiod-policy-reference-v0-1`  
**Evidence class:** synthetic, offline  
**Claim ceiling:** small exact multi-job/multi-period policy reference; no production scheduling, route qualification, execution or physical acceptance claim.

## Purpose

This is the first executable composition beyond #7085's one-job/two-stage model. It adds two jobs competing for a shared time-indexed resource, three resources with non-overlapping capacity windows, three decision periods, scenario-dependent outages and an unknown-resource state. The policy is optimized over observable histories, not caller-supplied information-set labels.

It is deliberately a small exact reference, not a production APS or MES.

## Model in this first slice

- 12 synthetic cases: six scenario families × two seeds. This is a smoke/reference corpus, **not** the eventual six families × 20 seeds campaign.
- Two jobs share the period-0 bottleneck RESOURCE-A.
- JOB-ONLY-A can use only RESOURCE-A and is due first.
- JOB-FLEX can use RESOURCE-A, RESOURCE-B or RESOURCE-C, with different costs/risk inputs.
- Capacity is time-indexed: A has capacity only in period 0, B in period 1, C in period 2.
- Scenarios reveal failed or unknown resource evidence at the beginning of each period.
- Hidden terminal quality outcomes are evaluator-only. A public route-level expected quality-loss input may influence policy cost; hidden realized rejection pairs cannot influence policy actions.
- A missing/unknown resource is never silently treated as available.

The canonical test instance exposes an opportunity-cost decision: a myopic cheapest-now policy assigns the flexible job to A and strands the A-only job. The exact policy assigns the constrained job to A, then uses B or C for the flexible job according to later observations. Hidden quality outcomes remain invisible to the policy even when two leaves share every observable history.

## Policy and exact oracle

The baseline tournament keeps two interpretable comparators: (1) maximum immediate assignment count then lowest immediate cost; (2) earliest due date, fewest admissible remaining routes over the declared horizon, then lowest immediate cost. The second is a stronger scarcity-aware heuristic and prevents the exact oracle being compared only to a weak cheapest-now rule.

The exact reference enumerates every feasible assignment set at each decision node. It recursively partitions future scenarios by the canonical prefix of visible observations plus the public completed-job/resource state. Each node chooses one common action for its entire information group. The objective is integer PPM-weighted cost and includes:
- route production cost;
- declared public expected quality-loss cost;
- tardiness cost based on the completion period;
- unserved-job penalty.

The first myopic baseline maximizes jobs completed immediately, then minimizes immediate cost with deterministic tie-breaking. A second scarcity/deadline-aware baseline prioritizes earlier due dates and fewer admissible remaining routes, then minimizes immediate cost. Both are one-step rules and do not optimize future recourse value. The exact policy is compared against both; this tiny corpus does not establish superiority over full industrial dispatch/APS baselines.

Hidden realized quality rejection pairs are excluded from optimization and used only to calculate a separate evaluator diagnostic. The test suite mutates these hidden outcomes and requires the policy tree to remain byte-for-byte identical.

## Non-anticipativity

There are no externally supplied information-set labels. At each period, the implementation derives a decision key from:
- the complete canonical sequence of observations visible up to that period;
- the public completed-job/resource state.

Scenario order is canonicalized. Failed and unknown resource sets are sorted; duplicate entries and contradictory failed+unknown status are rejected. The verifier checks every decision node for duplicate job assignments, route eligibility, current-period resource availability, capacity usage and identical actions for repeated decision keys.

This is an explicit-history path representation that derives the scenario tree/prefix nodes at load time. It is a small reference model; larger tree storage, event identity/clock provenance, multiple operations per job, setup sequences, in-process WIP, capacity reservations, inspection actions and distributionally robust policy sets remain future work tracked by #7168/#7102/#5520.

## Observed v0.1 result (exact head 33d20686506f0c209e62b8da2d456440b32f02f3)

The frozen run evaluated 12 cases, 60 scenario leaves and 96 decision nodes across six families. Expected-cost numerators are in parts-per-million weighted synthetic challenge units.

| Policy | Mean cost / case | Aggregate numerator |
|---|---:|---:|
| Myopic max-jobs / lowest immediate cost | 51.0 | 612,000,000 |
| Scarcity + earliest-due-date one-step rule | 11.3 | 135,600,000 |
| Exact non-anticipative policy | 11.3 | 135,600,000 |

The exact policy is 77.8% lower than the intentionally weak cost-only baseline. However, the stronger scarcity/deadline-aware heuristic ties the exact policy on **all 12 cases** (zero regret). This is the important result: v0.1 validates the model, non-anticipativity and replay machinery, but does not establish decision-quality uplift over a competent simple dispatch rule. The corpus has a structurally obvious A-only job and a flexible job; a scarcity-first dispatch rule is sufficient.

Per-family exact/scarcity mean costs (two seeds each):

| Scenario family | Mean challenge units |
|---|---:|
| Nominal resource window | 9.750 |
| Local resource outage | 9.950 |
| Correlated common-cause | 17.025 |
| Qualification Unknown | 10.150 |
| Deadline/capacity pressure | 12.000 |
| Quality-outcome uncertainty | 8.925 |

The highest-cost family is correlated common-cause disruption, but it still does not separate the exact policy from the scarcity/deadline heuristic. These two-seed means are descriptive only; they are not statistically powered comparisons.

Do not present the 77.8% figure as an industry comparison or SOTA result. The exact-versus-scarcity tie is a negative result to preserve. v0.2 must introduce interacting bottlenecks, overlapping eligibility, recovery-time uncertainty, operation/setup state and evidence-dependent route transitions, then compare against a tuned baseline portfolio on held-out regimes.

Validation on the exact head above:
- Focused workflow run #19: formatting, schemas, all 12 independent known answers, adversarial tests, exact policy execution, independent scenario replay, capacity-overbooking negative control and strict Clippy — all passed.
- Full repository CI run #21220: Ubuntu, Windows and macOS — all passed.
- PR #7174 remains draft and unmerged.

These checks establish only the declared synthetic reference behavior. They do not qualify any provider, manufacturing route, machine, measurement method or physical product.

## Known-answer and validation protocol

- Strict Draft 2020-12 validation for the case corpus and known-answer file, offline.
- Cross-language golden expected-cost numerators generated independently from the declared five-leaf model formula.
- Tests cover hidden-outcome non-interference, scenario permutation invariance, unknown resource exclusion, duplicate observation rejection, myopic opportunity-cost failure, exact-vs-myopic cost ordering and report claim boundaries.
- No external host, provider, resource reservation, machine command, work order or production release is used.

## First-slice limitations

This corpus uses one-operation jobs and stylized three-period capacity windows. It does not yet represent operation DAGs, partially completed operations, sequence-dependent setups, resource calendars with fractional capacity, multiple teams, material/lot constraints, inspection execution, quality rework loops or actual manufacturing costs. It should not be compared numerically to FJSPLib, SAP PP/DS, DELMIA, SLR2 or document-driven masked FJSP. Those are distinct matched benchmark tracks.

## Claim ceiling

A passing focused workflow would establish that this small Rust model and its frozen synthetic fixtures satisfy their declared schemas, exact cost known answers, history-based non-anticipativity and capacity checks. It does not establish production scheduling superiority, real provider reliability, real inspection sensitivity, quality conformity, manufacturing savings, physical recovery time or accepted output.
