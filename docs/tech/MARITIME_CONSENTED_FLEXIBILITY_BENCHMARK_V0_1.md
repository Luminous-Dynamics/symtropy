# Consented Vessel-Window Flexibility Benchmark v0.1

**Status:** research contract; all initial data synthetic, effects-disabled.  
**Tracker:** [MARITIME-FLEX-001 #1903](https://github.com/Luminous-Dynamics/symtropy/issues/1903).  
**Composes with:** [JIT arrival and port energy #1858](https://github.com/Luminous-Dynamics/symtropy/issues/1858), [contingent recovery #1867](https://github.com/Luminous-Dynamics/symtropy/issues/1867), and [staged capital / operations #1883](https://github.com/Luminous-Dynamics/symtropy/issues/1883).

## 1. Paradigm shift: flexible demand is a resource, but only with consent

A port planner usually receives arrival demand as a fixed schedule or forecast, while infrastructure planning treats capacity as something to build. The alternative is to represent **voluntary, bounded, compensated timing flexibility** as a finite option that can sometimes avoid congestion or defer a capacity increment.

This is not permission for a central optimizer to shift a vessel at will. Each offer has an owner, bounds, expiry, service obligations and authorization lifecycle. A planner can propose using it; only the separately authorized participant workflow may accept or activate it.

A 2026 peer-reviewed study by Lei and Jia integrates vessel arrival shifting, queueing and data-driven port throughput capacity. In its Shanghai port case, the authors report 14.47–16.67% lower total system costs than a non-integrated baseline, and a reported 0.91% optimality gap in one tested result. These are the authors' results on their specific data and model—not Symtropy targets or reproduced gains. The study provides a strong reason to compare demand and capacity decisions jointly: https://doi.org/10.1016/j.tre.2025.104660

The IMO/IAPH/IHMA Port Call Optimization Guide, launched in March 2026, emphasizes harmonized operational and nautical data exchange between port-call participants. Interoperable data is a prerequisite for coordinated planning, not authority to change a contractual schedule:
- https://greenvoyage2050.imo.org/port-call-optimization-guide-launched-at-imo/
- https://www.iaphworldports.org/pickup/21900/

## 2. Exact ownership

| Concern | Owner |
|---|---|
| Candidate voyage/performance-envelope admissibility and engineering claim limits | Symthaea, including [MARINE-FREIGHT-001 #7444](https://github.com/Luminous-Dynamics/symthaea/issues/7444) |
| Pure demand-flex evaluation, resource accounting, Pareto outputs and finite optimization | Symtropy |
| Actual participant consent/authorization, offers, commitments, bookings, rebooking, contracts, custody, compensation and realized outcomes | Mycelix, including new lifecycle contract [LOG-FLEX-001 #5185](https://github.com/Luminous-Dynamics/mycelix/issues/5185), reusing [LOG-COMMONS-S0 #4901](https://github.com/Luminous-Dynamics/mycelix/issues/4901), [LOG-COMMONS-S1 #5044](https://github.com/Luminous-Dynamics/mycelix/issues/5044), [PROC-MULTIREQ-001 #5144](https://github.com/Luminous-Dynamics/mycelix/issues/5144), and [PROC-OUTCOME-001 #5120](https://github.com/Luminous-Dynamics/mycelix/issues/5120) |
| Read-only spatial/temporal display of offered/accepted/expired flexibility and system impacts | Sol Atlas, reusing [MARITIME-ATLAS-001 #115](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/115) |

This issue operationalizes the flexibility alternative listed in [MARITIME-CAPITAL-001 #1883](https://github.com/Luminous-Dynamics/symtropy/issues/1883); it does not duplicate its investment portfolio solver. It also does not duplicate JIT allocation or recovery logic from #1858/#1867.

## 3. Flexibility offer contract

An offer should carry the smallest sufficient, versioned contract:
- stable offer ID and revision; authorized participant/issuer reference;
- exact vessel call, cargo/service obligation and baseline window;
- allowed earliest/latest service time, maximum advance/delay, and explicit non-flexible intervals;
- resource/service profile released or consumed by each allowed shift;
- activation notice, lock time, expiry, cancellation/revocation state and valid decision window;
- compensation rule and payer/payee role references where applicable, or explicit no-compensation terms;
- service-quality limits: deadlines, transfer connections, temperature/quality constraints or other justified hard bounds;
- capacity/compatibility profile, units, source evidence and status;
- minimal-disclosure policy and references to owner-authorized consent/acceptance receipts.

Do not infer flexibility from past behavior. Do not silently expand an offer because a model predicts a participant would probably accept. Until an accepted, current authorization is presented, the offer is only a candidate for a hypothetical plan.

Keep these states separate: preference/forecast → offered → accepted conditional option → activation authorized → schedule changed → observed outcome → disputed/cancelled/expired. A digest or signature establishes integrity/issuer attribution only; it does not prove the physical vessel shifted or service completed.

## 4. Finite first slice

Start with a compact synthetic terminal with a few vessel calls, discrete time windows, a declared throughput envelope, and bounded alternatives. Include berth, yard/handling and shore-power demands where the fixture explicitly provides typed quantities and compatibility. Compare:
- fixed schedule/no flexibility;
- a deliberately unsafe unrestricted shift control;
- a demand-shift-only plan at fixed capacity;
- the current staged-capital portfolio baseline without flexibility;
- joint staged capacity plus consented flexibility, using the same complete scenario set;
- an independent exhaustive oracle for fully enumerated tiny cases.

Each case must freeze:
- participant/demand IDs and service priorities;
- each baseline window and acceptable alternatives;
- cost/compensation and delay/inventory/service consequences, all in declared units;
- the physical resource/period capacity ledger and resource identity aliases;
- offer authority, acceptance state, expiry, revocation and decision-time evidence;
- any demand, tariff, policy or service information available at each decision stage;
- expected feasible allocation(s), objective vector and claim/completeness ceiling.

Synthetic costs or service penalties are model inputs, not estimates of real commercial terms. Do not fabricate probability distributions for hand-authored stress cases.

## 5. Feasibility and accounting invariants

- Only explicit, authorized and currently valid accepted flexibility can be counted as committed capacity release; offered but unaccepted flexibility may appear only in a clearly marked hypothetical plan.
- The pure optimizer cannot accept, activate, revoke, purchase or settle an offer.
- Shifts must remain within participant-specific timing bounds and hard service/cargo constraints.
- Count capacity released/consumed by canonical physical resource ID and operating interval; graph-edge aliases cannot mint extra capacity.
- An offer cannot be spent by both the primary schedule and a recovery branch, or across two conflicting capital/dispatch scenarios as though both occurred.
- Preserve half-open interval and time-profile semantics; reject incompatible units and capacity scopes.
- Compensation transfers, operator costs, delay/inventory/cargo losses, social resource costs, and capital deferral are distinct measures; never claim that paying one party is automatically a social loss or gain of the same amount.
- Report outcomes for every affected participant/service class; aggregate gains must not hide displacement or uncompensated burdens.
- Privacy-by-default: use opaque references and reveal only fields required for matching, evaluation or verification.
- Missing/revoked/contradictory authorization, costs, capacity, service bounds or units remain indeterminate—not free flexibility.
- Any search truncation marks the result incomplete; optimality only applies to a completely enumerated declared space.

## 6. Metrics

Report a vector rather than a universal score:
- peak and time-bucket resource utilization, overloads and waiting/queueing measures supported by the declared model;
- served/unserved demand and hard-deadline/service-quality misses;
- total shift amount, direction, maximum deviation and number of affected participants;
- operator/system cost, compensation transfers, delays/inventory costs, explicitly scoped energy/emissions, and capital increment deferred/avoided in the synthetic scenario;
- per-participant benefit/burden, displacement, uncompensated shifts and distributional sensitivity;
- unused or expired offers, offer acceptance/activation state and option opportunity cost;
- Pareto frontier, dominance under each declared scenario, and regret against the exhaustive oracle on tiny cases;
- candidate/branch counts, p50/p95 runtime, peak memory, truncation and deterministic receipt identity.

Do not target a fixed percentage of capex avoided before the fixture and baseline are defined and run. In early phases, the strongest numerical target is correctness on small cases, not a preselected savings headline.

## 7. Frozen machine-readable corpus

The first starter corpus is [MARITIME_CONSENTED_FLEXIBILITY_V0_1.json](https://github.com/Luminous-Dynamics/symtropy/blob/research/maritime-consented-flex-v0-1/benchmarks/fixtures/MARITIME_CONSENTED_FLEXIBILITY_V0_1.json). It contains 12 synthetic cases for offered-versus-accepted state, a bounded shift that releases a peak slot, hard deadline rejection, double-spending an offer, physical-resource aliasing, revoked/expired offers, wrong-subject reuse, future authorization leakage, compensation accounting, no-flex controls, scenario-dependent investment deferral, and deterministic permutation.

This is a versioned starter corpus, not an executable test result. The eventual verifier must independently recompute expected feasibility, per-resource usage and objective components; no expected savings or ranking should be trusted solely because it is written in a fixture.

## 8. Required hostile cases

- unaccepted offer is treated as committed flexibility;
- an offer is revoked or expires before its activation lock time;
- the requested shift exceeds maximum advance/delay;
- shifted arrival breaches a hard cargo/service deadline or declared quality envelope;
- offer identity refers to the wrong call, owner, revision, scenario or capacity profile;
- identical offer or time window is double-pledged across primary/recourse/capital plans;
- two graph edges alias a single physical resource;
- offer is accepted after the decision-time cutoff, or a future event leaks into an earlier decision;
- compensation transfer is double-counted as net social value;
- one participant gains while another is silently displaced;
- no eligible flexible demand exists, so no synthetic capacity may be created;
- data is incomplete/contradictory or units are incompatible;
- the unrestricted shift baseline violates consent and must be detected;
- search-budget exhaustion cannot claim global optimality;
- canonical input permutations and duplicate/replayed offer receipts preserve deterministic/idempotent results.

## 9. Proposed acceptance gates (not measured)

- 100% agreement with an independently implemented exhaustive oracle on complete tiny cases: feasibility, flex allocation, resource ledger, objective vector and frozen tie-breaker.
- Zero violations of consent, acceptance status, expiry, hard deadlines, service-quality constraints or physical capacity.
- Zero double-spending of any offer across base, recourse and investment decisions.
- 100% rejection/indeterminate disposition for missing, revoked, stale or contradictory mandatory authority/evidence.
- 100/100 deterministic replays and canonical input permutations yield byte-identical receipts.
- Correctly distinguish offered, accepted, activated and executed states in every frozen lifecycle control.
- Compare against fixed/no-flex, unrestricted-shift (negative control), flex-only and capital-only baselines on identical scenarios; report participant-level burden and service outcomes.
- Demonstrate at least one fixture in which flexibility defers an investment and another where flexibility cannot substitute for a required capacity increment. Both outcomes must be oracle-derived, not hard-coded as a success claim.
- Larger workloads report p50/p95, peak memory, offer/candidate counts, resource-ledger operations, completeness and per-scenario Pareto results.
- Historical/real evaluation requires lawful source snapshots, permission for participant-level data, explicit decision-time cutoffs and owner-qualified outcome receipts. Synthetic success is not empirical validation.

## 10. Authority and claim ceiling

This is an effects-disabled research model. It does not contact a participant, accept/purchase flexibility, change arrival times, rebook cargo, reserve a berth, mutate inventory, settle payment, alter contracts or authorize capital expenditure. Only Mycelix's separate authorized workflow may represent real commitments. Passing the synthetic oracle establishes finite-model contract correctness, not real cost savings, market fairness beyond the stated policy, emissions reduction, operational safety or legal compliance.

## 11. Research references

- Lei & Jia, *An integrated framework of vessel demand shifting and port capacity utilization for congestion mitigation*, Transportation Research Part E 208 (2026): https://doi.org/10.1016/j.tre.2025.104660
- IMO GreenVoyage2050, *Port Call Optimization Guide launched at IMO*, 26 March 2026: https://greenvoyage2050.imo.org/port-call-optimization-guide-launched-at-imo/
- IAPH, *The definitive Port Call Optimization guide for ports and shipping is published*, 25 March 2026: https://www.iaphworldports.org/pickup/21900/
