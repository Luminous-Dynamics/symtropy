---
title: Community Water–Energy Resilience Benchmark
version: 0.1
status: proposed-experiment-contract
scope: Symtropy physical simulation; Symthaea policy evaluation; essential-service allocation under scarcity
owner: research/simulation
claim_boundary: protocol proposal only; no benchmark run or capability result is claimed
---

# Community Water–Energy Resilience Benchmark v0.1

## Purpose

Test whether a Symthaea policy operating through Symtropy can measurably improve reliable, equitable delivery of essential water and electricity under scarcity, forecast error, and infrastructure failure—relative to strong, matched baselines.

The practical unit of success is **essential service delivered safely per unit of scarce resource**, not an internal consciousness score, cooperation count, visually compelling simulation, or a single aggregate reward.

This document defines the experiment contract. It does not claim an executable benchmark, a field-tested controller, or a result. Synthetic fixtures must be clearly labeled as synthetic. Real-world use requires data provenance, domain review, and validation beyond simulation.

## Why this experiment

Symtropy already has cooperation, resource, and thermodynamic research examples. Its repository README reports tighter clustering under thermodynamic enforcement, but clustering alone does not establish that people receive more essential services, that the mechanism is causal, or that gains survive shifted conditions.

The next step should be a domain-facing, falsifiable test that composes physical simulation, resource accounting, fairness, and cognitive policy evaluation. Water and electricity are coupled: water pumping and treatment consume energy, storage and generation are constrained, and outages can interrupt delivery even when demand is accurately predicted.

This is also a useful stress test for broader claims about state-coupled physics and cooperative behavior. A positive result would demonstrate task-specific policy utility, not prove consciousness, integrated information theory, or a universal law of cooperation.

## Core hypotheses

- **H1 — service utility:** Under physically feasible scenarios, at least one coordinated policy reduces essential-service deficit-hours versus a preselected strong baseline.
- **H2 — causal contribution:** Any improvement attributable to Symthaea disappears or materially shrinks under targeted ablations of its reasoning, forecast use, memory, or coordination path.
- **H3 — shift robustness:** Gains persist on held-out weather, demand, and fault combinations rather than only on scenarios used to develop the policy.
- **H4 — no hidden transfer of harm:** Improvements in average service do not come at the expense of the worst-served households, critical facilities, or a protected service class.
- **H5 — auditable physics:** Every reported run satisfies declared water and energy inventory balances within documented numerical tolerances; infeasible demands remain visible as shortages rather than being silently erased.

H1–H5 are hypotheses to test, not expected findings. Null or negative results are publishable outcomes.

## System boundary and model contract

### Initial scope

Start with a stylized, deterministic Rust model of a community microgrid and water-delivery system. Use a seven-day horizon with 15-minute control intervals (672 steps) as the initial reference profile. Treat these as benchmark design choices, not claims that the model captures every real utility system.

Model only the components required to answer the question:

- renewable and dispatchable electricity supply, grid interruption, battery state of charge, conversion losses, and bounded charge/discharge rates;
- water inflow, usable-water inventory, treatment/pumping throughput, leakage, delivery capacity, and bounded storage;
- households and critical facilities with time-varying essential and discretionary demand;
- explicit resource limits, service priority declarations, and infrastructure faults;
- observable information available to the controller, with delay, noise, and missingness when a scenario requires them.

All variables must carry units. Validate dimensional consistency and impossible parameter combinations at input time. Do not invent unmeasured pump efficiencies, water quality, battery degradation curves, or network behavior and then describe them as real-world calibration.

### Conservation and feasibility

Maintain distinct, inspectable ledgers.

Use bus-side quantities in energy units per interval.

`G_available = G_used + curtailment`

`G_used + D_bus = served_load + C_bus + exports + grid_losses`

`SOC_next = SOC + eta_charge * C_bus - D_bus / eta_discharge`

Here, `C_bus` is energy drawn from the electrical bus to charge storage, while `D_bus` is energy delivered by storage to the bus. Keep the bus balance and storage-state update as distinct checks. The combined inventory verifier must account for storage conversion losses exactly once:

`SOC_t + G_available = SOC_next + served_load + exports + curtailment + grid_losses + charge_loss + discharge_loss`

`charge_loss = (1 - eta_charge) * C_bus`

`discharge_loss = (1 / eta_discharge - 1) * D_bus`

Curtailment is the unused portion of available generation. `served_load` includes pumping and treatment electricity as well as household and critical-facility loads.

Water uses separate raw-water and treated-water stocks:

`raw_next = raw + raw_inflow - raw_treated - raw_leakage - raw_overflow`

`treated_next = treated + treatment_yield * raw_treated - delivered - treated_leakage - treated_overflow`

`raw_treated` is raw-water volume entering treatment; `treatment_yield` is dimensionless and must remain within declared bounds. If the first implementation deliberately collapses the raw-water loop, it must instead define `inflow` as already-treated water and must not add treatment output on top of it.

The exact sign conventions, storage boundaries, treatment boundary, and efficiency units must be declared in code and fixtures. A pump/treatment action is feasible only if both its water throughput and electricity requirements are feasible. Keep unserved demand as an explicit outcome.

The verifier must recompute balances from emitted flows rather than trust controller-reported totals. It must reject impossible negative inventories, capacity violations, invalid identities, omitted flows, and inconsistent units. Set tolerances relative to declared units and scale; test them on boundary fixtures before choosing a single global number.

## Evaluation hierarchy

Do not collapse every result into one weighted reward. Use a lexicographic constraint-and-outcome report:

1. **Feasibility and safety:** physical constraints, declared critical-service rules, unsafe actions, and accounting residuals.
2. **Essential-service outcomes:** shortage volume and deficit-hours by service class and household.
3. **Distribution of harm:** worst-served decile, maximum household deficit, critical-facility outcomes, and between-group differences.
4. **Resource and lifecycle cost:** fuel/electricity consumed, curtailed energy, water lost, storage throughput and degradation proxy, operating cost where prices are grounded, and emissions only where generation factors are sourced.
5. **Recovery and operations:** time to restore target service after a fault, action latency, stale-data handling, communication burden, and operator interventions.

Report every primary measure with denominator, units, uncertainty, and scenario family. Means must never replace tail and per-household distributions.

### Primary outcome

**Essential-service deficit-hours**, counted separately for water and electricity and for every declared critical service class. Define deficits against a manifest-fixed demand profile and distinguish a true shortage from a demand forecast error, a controller action, a component outage, or an exogenous physically infeasible condition.

### Secondary outcomes

- total unmet essential demand and longest uninterrupted shortage;
- worst-decile and maximum household service deficit;
- critical-facility shortage-hours;
- water loss, energy curtailment, total resource use, and operating cost under a sourced price schedule;
- recovery time after each fault and fraction of scenarios that recover within the declared horizon;
- resource-ledger residuals and number/type of invariant violations;
- controller p50/p95/p99 action latency, peak memory, and work per decision on a pinned host.

## Baselines and ablations

Every controller receives the same observation schema, action space, control cadence, and per-decision budget. The evaluator owns the hidden truth and outcome computation.

Required comparisons:

1. **Greedy feasible policy:** serves the currently highest ranked feasible request with a declared deterministic tie-break.
2. **Proportional rationing:** allocates shortages proportionally within predeclared service tiers.
3. **Max-min / leximin allocator:** a fairness-oriented comparator where the implementation is available and independently verified.
4. **Decentralized local policy:** each node acts on local observations and declared messages only.
5. **Symthaea-controlled policy:** Symthaea selects or ranks actions; Symtropy enforces physical transition and action constraints.
6. **Oracle reference on tiny worlds:** an exact solver or exhaustive search over small fixtures, used only as a bound and never given to the deployable controller.

Select the strongest baseline using development/validation scenarios before unsealing the held-out test set. Publish all baseline results; do not select a weak straw baseline after seeing the test results.

Required Symthaea ablations, matched as closely as possible for compute and available information:

- replace the cognitive policy with its strongest scripted/rule-based counterpart;
- remove forecasts while retaining current observations;
- remove memory/history while retaining current observations;
- remove inter-agent coordination while retaining local inputs;
- replace any proposed state-coupling signal with a constant or shuffled control;
- disable the specific mechanism claimed to cause improvement while keeping other costs and dynamics fixed.

Do not compare a costly, resource-constrained system to an unlimited-energy or otherwise physically different world and interpret the entire difference as a cognitive benefit. Change one causal factor at a time where possible, then run a declared factorial subset for interactions.

## Scenario portfolio

The first version uses versioned synthetic profiles and must label them as synthetic. Generate and preserve:

- nominal supply and demand;
- renewable underproduction and correlated low-solar / high-demand intervals;
- water-inflow decline or treatment-capacity restriction;
- grid outage combined with a pump or battery fault;
- sensor delay, stale telemetry, missing measurements, and communication partition;
- demand spikes and forecast errors;
- heterogeneous household needs, different storage access, and critical facilities;
- strategic under-reporting, false scarcity reports, or repeated priority requests;
- combinations drawn from distributions different from the development distribution.

Use seed-paired controller comparisons (common environment realizations) to lower variance. Include scenarios where policies should tie, cases where coordination should help, cases where it cannot help because demand exceeds physical supply, and adversarial negative controls. More random seeds from one template do not count as independent scenario families.

Initial scale ladder: 10, 100, and 1,000 household agents, with fixed smaller fixtures reserved for exact oracle comparison. Test one component and one coupled failure before combining faults.

## Split integrity and evaluator independence

- Maintain separate controller-visible inputs and evaluator-only truth/labels.
- Freeze scenario-generation version, parameters, seeds, and split manifest before final evaluation.
- Keep held-out family parameters and outcome truth inaccessible to policy-selection code.
- Hash source revision, configuration, input manifests, emitted action trace, output report, and verifier version.
- The verifier independently recomputes resource balances and primary outcomes from the action trace.
- Include mutated-report tests: altered totals, missing intervals, reordered events, duplicated identities, negative inventories, and fabricated service delivery must be rejected.
- Replay the same manifest twice and require identical traces and report digests for deterministic modes.
- Treat missing, skipped, queued, or stale CI as **not validated**; local source inspection is not a test pass.

The component-based benchmark-auditing work in AgentSuite/COBA is relevant background: benchmark validity can fail at the task, environment, ground-truth, or evaluation layer, even when a solver runs successfully.

## Statistical protocol

1. Run a small engineering pilot to estimate variance and identify broken fixtures. Do not use pilot cases in the final held-out score.
2. Before the primary run, register the primary outcome, direction of effect, minimum practically important effect, comparison policy, exclusions, test-set manifest, and analysis procedure.
3. Use paired seeds and report per-seed differences, effect sizes, and 95% uncertainty intervals. Choose the final sample size with a prospective power analysis; 100 paired seeds per condition is an initial planning floor, not a substitute for power analysis.
4. Correct for multiple confirmatory comparisons, or explicitly mark secondary analyses exploratory.
5. Report null results and failures. Never delete a failed seed unless a predeclared infrastructure-failure rule applies and its evidence is retained.
6. Perform sensitivity analysis on scenario assumptions and cost/priority policies. If conclusions reverse under modest defensible assumptions, report the reversal.

## Candidate promotion gates

These are **proposed targets for preregistration**, not empirical results or universal service standards. Before a consequential real-world pilot, domain operators and affected communities must review the thresholds and definitions.

- **Physical validity:** 100% of accepted traces pass independent balance and capacity checks; all seeded invalid traces are rejected.
- **Reproducibility:** 100% deterministic replay agreement for the declared deterministic mode, including exact manifest and report-digest verification.
- **Policy-effect gate:** compose with the existing Living City policy-robustness target in [Symthaea #7618](https://github.com/Luminous-Dynamics/symthaea/issues/7618): a provisional target of at least 20% lower worst-case essential-service person-hours than the floor-first baseline. This domain profile does not independently claim that target is met. Align the exact denominator, scenario set, and duration metric first; mean deficit-hours and worst-case person-hours are distinct measures. Any additional water/energy-specific minimum effect must be preregistered with domain stakeholders before unblinding.
- **Distributional guardrail:** no material degradation in the worst-served decile or critical-facility service; set the absolute non-inferiority margin with stakeholder input before unblinding.
- **Robustness:** benefit must persist across multiple distinct held-out scenario families, not just seed variants of one template.
- **Mechanism evidence:** the declared causal ablation must materially shrink the claimed benefit. If it does not, retract or narrow the mechanism claim.
- **Operational value:** report resource cost, latency, memory, recovery time, and human/operator burden; do not promote based only on the primary service metric.

A hard feasibility failure, misleading hidden shortage, unfair concentration of harms, or absence of reproducible benefit blocks promotion regardless of the aggregate score.

## Data path to external validity

Use three evidence tiers and keep them separate.

**Tier 0 — synthetic mechanics.** Deterministic generated fixtures test invariants, known answers, causal controls, and evaluator integrity. This tier demonstrates only that the code and protocol behave as specified.

**Tier 1 — public-data replay.** Add documented data adapters with source links, license/terms review, units, temporal coverage, transformation logs, missing-data policy, and immutable source hashes. Candidate South African sources include:

- Eskom Data Portal, system hourly actual/forecast demand and available-capacity data: https://www.eskom.co.za/dataportal/demand-side/system-hourly-actual-and-forecasted-demand/ and https://www.eskom.co.za/dataportal/demand-side/system-hourly-demand-and-available-capacity/
- Statistics South Africa General Household Survey catalog, including water-supply interruption and electricity-access variables: https://www.datafirst.uct.ac.za/Dataportal/index.php/catalog/1027/data-dictionary/F5?file_name=ghs-2024-hhold-v1
- Department of Water and Sanitation Water Security Dashboard: https://www.dws.gov.za/dashboard.aspx

These sources have different spatial/temporal resolutions and data-quality constraints. National grid series and household survey indicators do not, by themselves, constitute matched community-level interval demand traces. Do not infer household time series or local pump parameters from aggregate indicators. Any modeled synthesis must be labeled as such.

**Tier 2 — laboratory / operator-reviewed evaluation.** Re-run with metered community or facility data under the appropriate permissions, compare against actual operational baselines, and have independent domain reviewers inspect failures. A simulation-only win is not evidence that live utility control is safe.

## Research context

- Li et al., “Reinforcement learning-assisted distributionally robust energy management for multi-microgrid networks,” *Scientific Reports* (published 22 June 2026): https://www.nature.com/articles/s41598-026-56338-3
- “Reinforcement learning for residential energy storage management at the neighborhood scale: A multi-benchmark evaluation,” *Energy and AI* (May 2026): https://www.sciencedirect.com/science/article/pii/S2666546826000236
- Suh et al., “AgentSuite: Toward More Reliable Agent Evaluation with a Component-Based Benchmark Auditing Pipeline,” ICML 2026: https://proceedings.mlr.press/v306/suh26a.html

These works motivate robustness, centralized/decentralized comparisons, and evaluator auditing. Their reported results do not validate Symthaea or Symtropy and should not be compared numerically without matched environments and protocols.

## Implementation sequence

1. **Freeze the domain profile:** declare unit conventions, water/energy action fields, critical-service mappings, manifest extensions, and metric denominators.
2. **Reuse the shared harness:** build against the Living City policy-robustness work in [Symthaea #7618](https://github.com/Luminous-Dynamics/symthaea/issues/7618) and its report/receipt path [PR #7664](https://github.com/Luminous-Dynamics/symthaea/pull/7664). Do not create a second scenario runner or competing report format.
3. **Integrate the shared world model:** coordinate with [Symtropy PR #1730](https://github.com/Luminous-Dynamics/symtropy/pull/1730) and [Symtropy issue #1739](https://github.com/Luminous-Dynamics/symtropy/issues/1739) for allocation pools, resource ledgers, bounded-duration actions, and temporal recovery. Verify current heads/status before depending on their APIs; do not fork a parallel city kernel.
4. **Add the domain extension:** implement explicit electric-storage, raw-water, treatment, and potable-water ledger extensions with hand-computable fixtures and an independent recomputation verifier.
5. **Reuse and qualify policies:** compare the shared harness's deployable baselines and Symthaea candidate on matched observations, action budgets, and scenario realizations; add domain-specific max-min or oracle comparators only where they are independently checked.
6. **Extend held-out cases:** add coupled pump/grid/storage failures, correlated scarcity, telemetry faults, and strategically distorted demand to the existing scenario families.
7. **Run the shared evaluation:** use one manifest/receipt/assessor contract, report service and fairness metrics, and retain failed runs and uncertainty intervals.
8. **Add external data only after provenance review:** public-data replay precedes any operator-reviewed or laboratory evaluation.

Keep the initial harness Rust-first. A documentation change is not an implementation, and no stage should be described as complete until exact artifacts and checks support it.

## Related work and integration boundaries

The benchmark is a **domain profile**, not a parallel evaluation framework. The shared policy-robustness harness and its matched-world receipts live in [Symthaea #7618](https://github.com/Luminous-Dynamics/symthaea/issues/7618) and [draft PR #7664](https://github.com/Luminous-Dynamics/symthaea/pull/7664). Relevant Symtropy integration points are the settlement/allocation kernel in [PR #1730](https://github.com/Luminous-Dynamics/symtropy/pull/1730), temporal transitions and recovery in [issue #1739](https://github.com/Luminous-Dynamics/symtropy/issues/1739), energy-ledger hardening in [PR #1622](https://github.com/Luminous-Dynamics/symtropy/pull/1622), and max-min regeneration allocation in [PR #1748](https://github.com/Luminous-Dynamics/symtropy/pull/1748). This document does not assume any of those changes are merged or validated; check exact heads and outcomes before integration.

Reuse the shared scenario manifest, policy interface, run receipt, comparison format, and fail-closed assessor. This profile's distinct contribution is the coupled water/electricity domain model, units, physical inventory equations, service/fairness outcomes, and external-data provenance. An independent verifier must still recompute water and energy balances; a valid policy receipt alone does not prove domain physics. Existing cooperation examples remain useful for mechanism research but are not substitutes for this applied qualification.
