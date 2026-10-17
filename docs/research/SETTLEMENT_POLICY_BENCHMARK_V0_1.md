# Settlement Policy Benchmark v0.1

**Status:** Research design for [SETTLEMENT-POLICY-001 #1752](https://github.com/Luminous-Dynamics/symtropy/issues/1752). Not yet an implemented multi-policy runner, a calibrated city model or evidence of policy superiority.

## Research synthesis

Three current research directions point toward the same evaluation architecture:

- NIST community-resilience planning explicitly treats infrastructure dependency and cascading consequences as critical to restoration sequence and recovery goals; it also emphasizes goal-setting with communities rather than technical optimization in isolation. [NIST guide](https://www.nist.gov/community-resilience/planning-guide) · [Volume II: dependencies and cascading effects](https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.1190v2.pdf)
- A 2025 systematic review of 88 urban-digital-twin papers finds that technical ambitions frequently outrun realized contribution because of interdisciplinary integration, consensual contextualization and procedural operationalization. [Azadi et al. (2025)](https://doi.org/10.3390/smartcities8010032)
- DAPP-MD (2026) makes a case for multidimensional uncertainty and interacting risk portfolios, rather than assuming one uncertainty axis captures the adaptation problem. [DAPP-MD](https://doi.org/10.1016/j.envsci.2026.104382)
- NASA's evolving Moon to Mars architecture deliberately separates and coordinates habitation, power, communications, logistics, ISRU, infrastructure support, mobility and autonomy. This argues for a common settlement vocabulary plus domain-specific solver/qualification contracts, not a universal physical simulator. [NASA architecture components](https://www.nasa.gov/moontomarsarchitecture-components/)

The resulting paradigm: **evaluate policies as paired, constrained counterfactuals under identical external disruptions and identical information access**. The policy is not allowed to change the scenario truth, rewrite a protected floor or receive future events as inputs.

## Benchmark unit

A single policy run binds:
- one frozen exogenous scenario and initial-state digest;
- one versioned policy and allowed-action set;
- one observation contract (latency, missingness, stale/conflicting telemetry, known versus unknown);
- one resource/crew budget and action-latency contract;
- one deterministic execution profile and seed;
- one run receipt, full per-tick trajectories and claim ceiling.

A paired comparison is valid only when exogenous events, initial state, domain assumptions and declared constraints match. Differences in information quality, budget or action availability must be explicit experimental factors, not hidden advantages.

## Event roles must not collapse

Keep these separate:
1. **Exogenous event** — outage, hazard, external demand shift, inflow or observed environmental change.
2. **Observation** — evidence available to the policy at a tick, including delayed/stale/missing state.
3. **Decision** — policy request, based only on observations received by that tick.
4. **Intervention** — accepted/deferred/rejected action with resource costs, latency, preconditions, and authority ceiling.
5. **Outcome** — realized synthetic component/service/resource/floor trajectory.

Repairs append events; they cannot delete the failure from history. A policy that repairs a pump has a different outcome path but the same exogenous outage record as a no-intervention policy.

## Transparent baseline family

- No intervention.
- Reactive repair after the declared observation/threshold is received.
- Floor-first action selection.
- Reserve-first allocation.
- Scheduled maintenance.
- Adaptive/reversible policy with preregistered triggers.
- Perfect-information oracle, if needed, only as an unattainable diagnostic upper bound and never presented as a deployable comparator.

The first campaign should use rules simple enough that reviewers can manually verify their decisions.

## Initial synthetic scenario matrix

| Case | Disturbance or ambiguity | Discriminating question |
|---|---|---|
| S-01 | Single grid outage with backup stock | Does the service degrade before the backup is available or depleted? |
| S-02 | Bounded backup, delayed repair and restoration | Does the policy preserve critical floors for longer without violating reserves? |
| S-03 | Restoration followed by recurrent outage | Is first restoration incorrectly reported as stable recovery? |
| S-04 | Power affects water pumping and critical care cooling | Which floors fail, and does the policy hide one service's harm behind another's benefit? |
| S-05 | Missing/stale/conflicting telemetry | Does the policy abstain, delay, or act on a declared safe fallback rather than using hidden truth? |
| S-06 | Repair part unavailable | Does the action fail/defer explicitly, with cost and latency recorded? |
| S-07 | Crew capacity conflict | Are repair queues, priority and fatigue costs represented rather than assuming infinite labor? |
| S-08 | False/missed/late adaptation trigger | Does the plan maintain acceptable service and avoid irreversible lock-in? |
| S-09 | Demand/population shift and cost shock | Does performance survive shifts not used for tuning? |
| S-10 | Community context conflict | Are the distinct affected-group outcomes preserved rather than averaged away? |
| S-11 | Low-connectivity interval | Can essential local services continue without a central controller? |
| S-12 | Habitat profile: supply delay + communication lag | Are space constraints attached to independently authorized domain-specific floors? |

All initial parameters and cohorts are synthetic. Space cases need their own physical boundary and safety authority; Earth-city floor values must never stand in for life-support thresholds.

## Metrics: never one grand score

### Hard gates
- Required scenario/policy coverage: 100% executed, or explicitly blocked.
- Negative controls: 100% executed with expected rejection/blocked disposition.
- Resource accounting: exact integer conservation for every declared flow; no hidden source, sink or unit conversion.
- Protected reserve: zero consumption below the declared reserve boundary.
- Floor safety: any violation remains visible; no scalar objective offsets it.
- Information parity: no candidate has future scenario truth or a wider observation stream.
- Replay: byte-identical canonical output for identical frozen deterministic inputs.
- Independent checker: 100% agreement on ledger conservation, floor disposition and recovery summaries for retained report receipts.

### Outcome vector
- Per-service breach count, duration, severity and affected scope.
- Resource stock, unmet demand, reserve margin and depletion ticks.
- First restoration versus stable recovery; later recurrence.
- Decision latency, intervention latency, repair duration, rejected actions and failed preconditions.
- Parts, energy and crew-hours consumed; service interruption caused by maintenance.
- Worst-case and lower-tail outcomes; scenario-wise regret against transparent baselines.
- False/missed trigger rate and trigger latency.
- Benefit/burden distribution across declared cohorts.
- Reversible-option value and lock-in cost where the scenario defines both.

No expected value, probability-weighted regret or CVaR unless the application supplies justified scenario probabilities. Do not report ecological, health, thermal or physical values without an independently qualified domain model.

## Proposed acceptance thresholds

These are contract gates, not achieved performance:
- **0** unexplained resource-balance errors in frozen synthetic tests.
- **0** uses of unknown future truth by a policy.
- **0** hidden service-floor violations.
- **100%** of runs either verified or explicitly blocked.
- **100%** strict-negative controls executed.
- **100%** receipts checked independently before comparison.
- **0** unauthorized changes to authorized floor/context identities in a campaign.
- Policy-efficiency goals (including the previously suggested 20% worst-case-loss reduction) stay provisional until baselines and held-out performance are measured. Then preregister a target per benchmark suite and include regression tolerance and sensitivity analysis.

## Qualification ladder

1. Static typed contract and fail-closed schema.
2. Deterministic timeline/resource-ledger accounting.
3. Independent receipt checker and tampering corpus.
4. Same-exogenous-event paired baseline runner.
5. Observation delay, intervention cost, crew/parts and held-out scenario campaigns.
6. Independent specialist model comparison with predeclared units, assumptions and tolerances.
7. Locally authorized real-place retrospective/human-reviewed evaluation.
8. Any operational deployment only after independent engineering validation, public/service governance review, fail-safe behavior and separately authorized operators.

A successful rung does not automatically qualify the next one.

## Symthaea / Mycelix roles

- **Symtropy** executes the declared synthetic timeline, constraints, ledgers and action semantics.
- **Symthaea** proposes/evaluates policies and uncertain hypotheses against paired reports, with abstention and evidence limitations.
- **Mycelix** provides references to locally authorized context/floors/authority and preserves the difference between evidence, legitimacy, ownership and operational authority.
- **Humans and affected communities** set legitimate local goals, address disagreements, and decide whether to accept real-world trade-offs.

No inference from model confidence, reputation or optimization score to authority. No direct simulation-to-actuation path.

## Current implementation boundary

The static core and temporal extension are under draft review in [PR #1730](https://github.com/Luminous-Dynamics/symtropy/pull/1730) and [PR #1743](https://github.com/Luminous-Dynamics/symtropy/pull/1743). They currently implement deterministic static/temporal synthetic execution, resource/floor outcomes and an independent receipt verifier. A paired multi-policy runner, intervention model with crew/parts costs, calibrated physical solvers and real-world validation are **not implemented yet**.

The current policy benchmark is therefore a specification and next-step programme, not a claim that an adaptive policy has improved city or habitat outcomes.


## Observation-boundary implementation dependency

A first typed policy-information-frame builder now exists in `crates/domains/symtropy-settlement-core/src/observation.rs`, with an explicit freshness window, source/target generation, sample tick, available tick, stale/missing/conflicting/unvalidated/revoked dispositions, canonical digest and a metamorphic no-lookahead test. See [SETTLEMENT-OBS-001 #1753](https://github.com/Luminous-Dynamics/symtropy/issues/1753) and [Observation Boundary v0.1](https://github.com/Luminous-Dynamics/symtropy/blob/research/settlement-timeline-v0-1/docs/research/SETTLEMENT_OBSERVATION_BOUNDARY_V0_1.md).

This is only the projection/conformance layer. The policy runner still must keep latent scenario state private, generate observations from a deterministic schedule, make decisions only from these frames, and separately receipt intervention costs/latency. No paired policy outcome or information-value result is implemented yet.


## Baseline decision interface implementation update

A first policy-decision interface now exists in `crates/domains/symtropy-settlement-core/src/policy.rs`:
- `NoIntervention` emits a no-action receipt.
- `ReactiveRepair` emits at most one request per decision tick, triggered only by a **Current** failed-component observation from a source explicitly authorized by a versioned repair rule.
- Requests declare parts, crew units, priority, dispatch latency, duration and expected completion tick.
- The decision binds the exact observation-frame digest, trigger observation ID/digest, policy ID/version and a deterministic action ID.
- The independent decision verifier rechecks the frame, trigger observation, rule identity, source authorization, costs, latency arithmetic and action identity.
- Eight policy tests cover basic baselines, current vs stale/missing/conflicting/unvalidated evidence, source restrictions, competing failures and priorities, malformed rules, overflow and tampering.

This deliberately does not execute repairs, reserve/spend resources, model a real repair crew, or compare policy outcomes. The emitted request is not actuation authority. The paired timeline runner, intervention acceptance/execution receipts, resource/crew contention, held-out policy campaign and external physical solver comparison remain unimplemented.


## Outstanding policy-runner semantics

Before turning requests into comparative outcomes, add a versioned intervention journal/state input so repeated observations cannot trigger duplicate active repairs. The executor must atomically check target/generation and pending intervention identity, reserve parts/crew under explicit budgets, accept/reject/defer with a reason, and write a completion/abort receipt. Idempotency keys must bind policy version, target generation, trigger observation, and action semantics. A request is not a reservation and must not be counted as completed work.

The temporal scenario runner must then separate exogenous event batches from intervention events. Every policy arm gets the same exogenous events and observation-generation contract; only accepted interventions differ. Report both request counts and accepted/executed intervention outcomes, including failed preconditions and resource contention. This prevents a policy from being rewarded for actions that it requested but the simulated institution could not carry out.
