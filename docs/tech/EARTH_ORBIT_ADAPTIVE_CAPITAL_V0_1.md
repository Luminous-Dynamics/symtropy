# Earth-to-Orbit Adaptive Capital Decision Contract v0.1

Status: research specification; documentation-only.

Parent: [EARTH-ORBIT-ADAPTIVE-CAPITAL-001](https://github.com/Luminous-Dynamics/symtropy/issues/3270)

Composes:
- [Earth-to-orbit chain #3228](https://github.com/Luminous-Dynamics/symtropy/issues/3228)
- [Launch learning curve #3242](https://github.com/Luminous-Dynamics/symtropy/issues/3242)
- [Southern African freight baseline #3243](https://github.com/Luminous-Dynamics/symtropy/issues/3243)
- [Freight modal choice #3201](https://github.com/Luminous-Dynamics/symtropy/issues/3201)
- [Orbital access #3202](https://github.com/Luminous-Dynamics/symtropy/issues/3202)
- [Maritime staged capital #1883](https://github.com/Luminous-Dynamics/symtropy/issues/1883)
- [Orbital foundry lifecycle #2161](https://github.com/Luminous-Dynamics/symtropy/issues/2161)
- [Sol-Atlas TRIM corridor evidence #400](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/400)

## 1. Decision contract

The decision problem is not “which facility should be built?” It is:

> Given the service demand, evidence, physical constraints and market state observable now, which reversible or irreversible action should be taken now, while preserving the ability to respond to future information?

A policy must be implementable using only information available at its decision time. A plan that selects actions using future scenario knowledge is a perfect-information bound, not an operational policy.

The model consumes qualified outputs from domain owners. It does not replace transport allocation, orbital mechanics, launch safety, GIS, environmental assessment, rail engineering, source qualification or investment authorization.

## 2. State, observation and action

At decision epoch $t$, define:

- $s_t$: latent system state (asset condition, true demand, actual capacity, market/technology state);
- $o_t$: observations admissible at $t$ (published data, qualified measurements, contracts, source freshness and uncertainty);
- $a_t$: chosen action;
- $w_t$: external scenario input (demand shock, price change, outage, policy change, construction delay);
- $e_t$: evidence/authority state (pass, fail, unknown, disputed, expired).

The transition is represented abstractly as:

\[
s_{t+1} = F(s_t, a_t, w_t; \theta)
\]

and the policy as:

\[
a_t = \pi(o_{0:t}, a_{0:t-1})
\]

The implementation must preserve the information boundary: $\pi$ cannot use $w_{t+1:T}$ before those outcomes are observed. A deterministic synthetic scenario tree is sufficient for v0.1; probability distributions are not required.

### 2.1 State variables

Keep at least these state families distinct:

| Family | Required distinctions |
|---|---|
| Physical assets | installed, under construction, commissioned, degraded, unavailable, retired |
| Capacity | theoretical, practical, operational, allocated, spare, observed utilization |
| Project stage | proposed, investigated, permitted, financed, committed, constructing, accepted, operating |
| Demand | contracted, manifested, qualified pipeline, forecast, latent/speculative, diverted, incremental |
| Market | provider availability, quoted price, observed contract price, estimated cost, concentration |
| Evidence | source, owner, valid time, retrieval time, confidence/uncertainty, rights, maturity, expiry |
| Public exposure | direct spending, guarantee, contingent liability, subsidy, tax, decommissioning obligation |

Unknown is not zero, false, unlimited, available, or a probabilistic mean. State transitions cannot silently upgrade a claim or evidence tier.

### 2.2 Actions

Action availability is gated by upstream evidence and capability profiles. Candidate action families:

- no-build / continue with existing providers;
- operational improvement, repair and maintenance;
- purchase, lease, reserve or diversify access to existing capacity;
- modular terminal, yard, integration, power, tracking or storage upgrade;
- short conventional-rail tunnel/bypass;
- source-qualified LIM/MagRail retrofit;
- staged new port, corridor, spaceport or range capacity;
- defer, suspend, abandon, repurpose or decommission.

A future orbital ring, elevator, skyhook or tether is not an available build action merely because a scenario assigns it a favorable economic score. Its required physical and materials claims must first be admitted by the relevant specialist owners.

## 3. Endogenous demand without demand invention

Do not extrapolate one aggregate payload or freight number.

Each demand cohort binds:
- customer/mission class and required service;
- origin, destination and target orbit where applicable;
- cadence, deadline, quality and compatibility requirements;
- contracted/manifested/qualified/forecast/latent evidence class;
- willingness-to-pay or economic-value basis, source and uncertainty;
- existing provider/route, switching constraints and substitute options.

Demand response is an explicit, versioned function or scenario table over generalized service attributes such as delivered price, reliability, lead time, cadence, coverage and mission compatibility.

The first implementation may use a small piecewise response table. It must not infer a universal elasticity. Distinguish:
- genuinely new demand;
- demand diverted from another route/provider;
- demand merely shifted in time;
- customer value unlocked by a new service;
- speculative demand with no contract or validating evidence.

An investment may not count diverted traffic as net new social benefit, and an uncontracted market estimate may not be labelled bankable demand.

## 4. Investment timing and project state

Each action declares:
- capex schedule and currency/price year;
- construction lead time and cost-to-complete;
- dependencies and critical path;
- commissioning/acceptance prerequisites;
- fixed and variable operating costs;
- maintenance, renewal and decommissioning;
- delay, cancellation, salvage and repurposing rules;
- financing schedule and public contingent liabilities;
- capacity delivered only after acceptance, not when capital is announced.

Do not count planned investment as completed infrastructure. A facility under construction does not deliver operational capacity until its qualified acceptance transition.

A project may be staged. A stage may create a real option to expand later, but the option is valuable only when the next stage is physically, legally and commercially available under its evidence gates.

## 5. Learning and market evolution

Consume launch-market experience-curve outputs from #3242 with their source, time basis, sample definition, model version and uncertainty.

The July 2026 PNAS Nexus study reports more than 4,400 launches (1960–2025) and an aggregate 21.2% decline in average cost per kg per doubling of cumulative payload-to-orbit. This is a replication/sensitivity target, not a universal vehicle-specific forecast. Price, provider cost, advertised price, contract price and public subsidy remain separate observables.

Structural breaks, provider concentration, export controls, fleet grounding, reuse changes, launch cadence and debris-related costs must be scenario inputs where supported—not hidden constants.

## 6. Evaluation modes

The benchmark should compare:

1. No-build / existing procurement.
2. Static forecast + deterministic NPV.
3. Greedy immediate-benefit investment.
4. Deterministic MILP/CP-SAT where its assumptions apply.
5. Two-stage robust optimization.
6. Multistage scenario-tree policy.
7. Rolling-horizon re-optimization.
8. Deferral / real-options heuristic.
9. Perfect-foresight dynamic oracle on small cases (lower-bound only).

For small cases, implement an independent dynamic-programming or exhaustive scenario-tree oracle. Production policy logic and oracle logic must not share transition, eligibility, cost aggregation or scoring helpers.

When scenario probabilities are unsupported, compare deterministic scenario sets, robust worst case and regret. Use expectation, CVaR or chance constraints only with calibrated probability models and declared coverage tests.

## 7. Objective vector and hard gates

Apply non-compensable eligibility gates before any objective comparison.

Hard gates include:
- physical capacity and compatibility;
- launch orbit/range compatibility;
- safety and emergency access;
- legal rights, permits and applicable authority;
- environmental and social exclusions;
- public affordability and explicit contingent-liability limits;
- evidence validity, freshness and provenance.

For eligible policies, preserve a vector rather than a universal weighted sum:

\[
J(\pi) = (
\text{accepted service},
-\text{lifecycle cost},
-\text{unmet demand},
-\text{energy},
-\text{emissions},
-\text{public exposure},
-\text{ecological burden},
\text{resilience}
)
\]

The implementation should report Pareto-efficient policies, robust regret and sensitivity. Do not hide disagreements between financial, fiscal, economic, environmental, distributional and resilience outcomes.

## 8. Frozen benchmark programme

Start with at least 30 synthetic policy instances; all values must be labelled synthetic. Include:

- no-build wins;
- operational improvement beats expansion;
- lease/reserve precedes a later build decision;
- modular expansion beats overbuild;
- low uptake strands a large project;
- high uptake justifies a deferred stage;
- launch learning reduces the value of a new spaceport;
- technology-learning structural break;
- provider concentration or exclusion shock;
- port expansion leaves rail as bottleneck;
- rail upgrade leaves launch integration as bottleneck;
- polar mission rejects equatorial ranking;
- tunnel bypass wins only above a threshold;
- dedicated underground corridor fails after capex escalation;
- public affordability fails despite positive private NPV;
- missing land/environmental/range evidence blocks a project;
- common-mode grid/port/tracking dependency defeats apparent redundancy;
- contractor delay pushes capacity beyond the demand window;
- competitor pricing cannibalizes incumbent demand;
- a low-cost investigation changes the best next action;
- unknown scenario probabilities require robust analysis;
- perfect-foresight policy materially outperforms an implementable policy;
- future orbital structure is unavailable until its physical evidence prerequisites pass.

Use hand-calculable numbers for the smallest fixtures. Keep one held-out family untouched during policy tuning.

## 9. Metrics

Report:
- service accepted by cohort, target orbit and deadline;
- time-to-first-service and time-to-sustained-service;
- unmet demand, diverted demand, incremental demand and stranded capacity;
- capex, cost-to-complete, opex, renewal, salvage and decommissioning;
- private NPV, public cash flow and contingent liability as separate views;
- cost per accepted service unit;
- energy, GHG, water, material and land impacts;
- schedule reliability, recovery, provider substitutability and common-mode loss;
- worst-case outcome, robust regret, Pareto coverage and value of information;
- forecast calibration / prediction interval coverage where observations exist;
- policy-vs-perfect-foresight gap;
- runtime, memory, optimality gap and case scale.

No single scalar score may erase the objective vector or override a hard gate.

## 10. Acceptance and claim ceiling

- 100% fail-closed disposition on hard-gate and missing-evidence controls.
- Exact agreement with the independent dynamic oracle for bounded small instances.
- No future-information leakage in policy decisions.
- Byte-stable replay for identical state, observation, action, scenario and model manifests.
- No-build and existing-provider baselines remain eligible at every epoch.
- Demand diversion and incremental demand reconcile separately.
- Every learning input is source- and time-bound.
- Report out-of-sample policy regret and baseline gaps with matched compute budgets.
- No claim of bankability, demand certainty, regulatory approval or physical feasibility.

This contract defines how to compare adaptive investment policies on declared inputs. It does not approve an investment, certify infrastructure, forecast a guaranteed market, or establish that any underground freight, spaceport, tether, skyhook or orbital-ring architecture can be built.
