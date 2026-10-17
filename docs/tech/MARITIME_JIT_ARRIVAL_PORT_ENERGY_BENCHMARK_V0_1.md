# Maritime JIT Arrival, Berth, and Shore-Energy Benchmark v0.1

**Status:** research contract; proposed targets only, not measured results.  
**Tracking issue:** [MARITIME-JIT-001 #1858](https://github.com/Luminous-Dynamics/symtropy/issues/1858).  
**Profile boundary:** deterministic, effects-disabled planning over frozen route/performance alternatives. This document does not specify navigation software or a physical-vessel simulator.

## 1. Research question

Can a planner choose voyage/speed, arrival time, berth/handling interval, and shore-energy service as one feasible plan—rather than optimize each component independently—while preserving hard physical-resource and evidence constraints under forecast errors and operational disruptions?

This is the missing decision seam between multimodal transport optimization, the maritime scenario/replay substrate, and shipment lifecycle state. It is not a claim that an optimization prototype can make actual ships safer or deliver a given fuel saving.

The IMO's GHG strategy explicitly points to port-call optimization and facilitating just-in-time arrival as areas for port/shipping cooperation. Recent research examines robust holistic ship design/voyage optimization under multiple uncertainties and energy-efficient weather routing with arrival-window assurance. These are research motivations; published effect sizes are not adopted as expected performance for this benchmark.

## 2. Cross-project ownership

| Concern | Owner and contract |
|---|---|
| Vessel/design validity, declared speed/energy envelopes, physical evidence and engineering claim limits | [Symthaea MARINE-FREIGHT-001 #7444](https://github.com/Luminous-Dynamics/symthaea/issues/7444), with the separate service-routing chain [#7851](https://github.com/Luminous-Dynamics/symthaea/issues/7851), [#7863](https://github.com/Luminous-Dynamics/symthaea/issues/7863), [#7876](https://github.com/Luminous-Dynamics/symthaea/issues/7876) |
| Deterministic plan optimization, time-window resource accounting, objective vectors and scenario replay | Symtropy [TRANSPORT-OPT-001 #1601](https://github.com/Luminous-Dynamics/symtropy/issues/1601), [TRANSPORT-ALLOC-002 #1639](https://github.com/Luminous-Dynamics/symtropy/issues/1639), [MARITIME-SIM-001 #1612](https://github.com/Luminous-Dynamics/symtropy/issues/1612), [MARITIME-TIME-001 #1650](https://github.com/Luminous-Dynamics/symtropy/issues/1650), [TRANSPORT-RES-005 #1671](https://github.com/Luminous-Dynamics/symtropy/issues/1671) |
| Participant-scoped shipment lifecycle, custody, event provenance, owner-authorized reservation/commit, delivery acceptance and outcome history | Mycelix [LOG-COMMONS-S0 #4901](https://github.com/Luminous-Dynamics/mycelix/issues/4901), [LOG-COMMONS-S1 #5044](https://github.com/Luminous-Dynamics/mycelix/issues/5044), [PROC-OUTCOME-001 #5120](https://github.com/Luminous-Dynamics/mycelix/issues/5120) |
| Read-only spatial and temporal projection, evidence coverage, freshness and lineage | Sol Atlas [MARITIME-ATLAS-001 #115](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/115), [SOL-ATLAS-009 #106](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/106) |

Reuse these owners; do not add a second canonical route graph, generic provenance envelope, maritime event vocabulary, or simulator.

## 3. Versioned scenario inputs

All first-slice inputs should be synthetic, compact, and frozen in a manifest. Real source snapshots are a later stage requiring exact revision, usage rights, provenance, units, and coverage review.

Each candidate voyage profile should identify:
- vessel/configuration/model profile and intended-use boundary;
- declared route option identity and its source/qualification status;
- duration and arrival-interval envelope;
- energy/fuel use as a function of declared operating profile, with units and evidence class;
- cargo compatibility, payload constraints, and any service deadline;
- model/version/source snapshot and uncertainty representation.

Each constrained port operation should identify:
- canonical physical resource ID, not just a graph edge or display name;
- operation kind (berth, pilot/handling resource, charger, swap pack, terminal interface);
- typed capacity quantity and time basis;
- half-open operation interval and the permitted reuse semantics;
- evidence source, revision, validity window, retrieval/observation/publication times, and conflict status.

Each forecast/profile should distinguish:
- model initialization/reference time;
- publication/issue time;
- observation time where relevant;
- decision/assessment time;
- requested target/valid time and valid interval;
- policy-bounded issue age and forecast lead;
- source quality, uncertainty and applicability policy.

A recently downloaded old forecast is still old. A forecast's final valid-time endpoint is not its initialization time. Unknown, stale, contradictory, unsupported or out-of-envelope inputs must stay inadmissible or indeterminate.

## 4. Decision variables and hard constraints

The first finite model may select among pre-generated alternatives for:
1. a declared voyage/speed profile;
2. an arrival target window;
3. a berth and handling interval;
4. charging or battery-swap service windows;
5. optional downstream connection and bounded reserve/recovery options.

Hard feasibility conditions include:
- every chosen voyage candidate is declared admissible for the benchmark's stated engineering purpose;
- arrival and service windows are compatible under the declared time semantics;
- vessel/port/cargo/handling interfaces match declared capabilities;
- a single physical resource is not overcommitted across aliased route edges, terminals or profiles;
- quantities and resource rates use compatible explicit units and time bases;
- energy/reserve conditions hold under the selected declared scenario envelope;
- evidence is applicable at decision time;
- demand served and unserved are both explicit; missing demand cannot vanish from the report.

Hydrodynamics, weather response, propulsion, current/tide effects, under-keel clearance, structural loads, and other physical calculations must come from qualified external models/data with appropriate intended-use evidence. If unavailable, the inputs remain clearly synthetic envelopes. The benchmark must not invent coefficients or elevate a synthetic scenario pass into an engineering result.

## 5. Objectives: expose the trade space

Publish a Pareto vector instead of a universal weighted score:
- voyage energy/fuel and emissions under an explicit lifecycle/operational boundary;
- anchorage and port-idle time/energy where represented by the inputs;
- end-to-end elapsed time, arrival-window miss and turnaround/dwell time;
- served/unserved demand by class and missed downstream connections;
- occupancy and conflicts by physical resource, charging demand and reserve consumption;
- per-scenario tail outcomes, plan changes and recovery time;
- private operator cost and wider social/environmental cost separately, where inputs support them.

Never trade away safety, legal or physical feasibility for fuel savings. If scenario probabilities are not empirically justified, publish scenario-wise and worst-case results rather than inventing expected values. A CVaR or chance constraint is admissible only when its probability model, calibration and decision meaning are explicit.

## 6. Baselines

Use the same frozen input manifest for each arm:
1. **Earliest-arrival:** chooses the earliest declared feasible voyage profile, then checks the service window.
2. **Energy-minimizing:** chooses the lowest declared energy profile before joint port scheduling.
3. **Greedy per-voyage allocation:** schedules each demand in a pinned deterministic order without revisiting prior choices.
4. **Simple JIT rule:** targets arrival at the nominated service time using a stated buffer, without joint multi-resource optimization.
5. **Joint optimizer:** considers voyage choice, berth/handling interval and shore energy together.
6. **Exhaustive tiny-instance oracle:** enumerates the complete finite feasible set independently of production scoring, resource-ledger and feasibility helpers.

The baseline implementations and tie-breaking rules must be frozen before the final holdout is opened.

## 7. Mandatory frozen adversarial cases

- two graph edges alias one crane, berth, charger, swap pack or other physical resource;
- adjacent half-open intervals versus genuinely overlapping operations;
- rate capacity confused with concurrent occupancy or period quota;
- capacity expires or is superseded before its proposed operation;
- berth release is delayed after a plan is selected;
- shore power or a required energy module becomes unavailable;
- the voyage arrives early but service is unavailable, or late beyond a downstream deadline;
- a speed/energy option is attractive but outside its declared admissibility envelope;
- recently retrieved but stale forecast; target time outside the valid interval; unknown model initialization;
- missing units, incompatible time base, unknown schema/profile, contradictory current evidence;
- omitted/unserved demand, forged aggregate totals, duplicate proposal or idempotency key with changed payload;
- input order permutations, search-budget exhaustion and integer-overflow boundaries.

Negative controls must remain negative. Missing information must not become zero delay, free capacity, clear water, guaranteed charging, confirmed delivery or a successful outcome.

## 8. Proposed qualification gates

These are goals, **not current measurements**:
- 100% agreement with exhaustive enumeration for every frozen tiny case on feasibility, selected plan within the stated tie-breaker, resource ledger and objective vector.
- Zero hard-constraint violations and zero physical-resource overcommitment among all admitted plans.
- 100% correct fail-closed/indeterminate dispositions on frozen stale, missing, contradictory and unsupported-input controls.
- 100/100 identical replay and equivalent input permutations produce identical canonical result bytes/receipt.
- Every bounded or heuristic run states whether the candidate set is complete and limits optimality claims to the verified search space.
- Compare all methods on paired inputs; report objective vectors, dominated solutions, served/unserved demand, p50/p95 runtime, peak memory, candidate count, expanded states, resource rejections, replanning frequency and incomplete/truncated state.
- Only claim savings on held-out cases if they are actually measured, source/data lineage and model validity support them, and service/hard constraints are not degraded. Report confidence intervals or scenario bounds as appropriate.

Synthetic correctness is the first gate, not external validation. Afterward, a small rights-cleared historical case may be considered if time-resolved outcomes and pre-event information cutoffs exist. Narrative-only reporting supports a qualitative stress scenario, not empirical savings or predictive validity.

## 9. Live policy and schema versioning

As checked on 17 October 2026:
- The IMO says work on the Net-Zero Framework remains in progress following the October 2025 adjournment; MEPC 85 is scheduled for 30 November–3 December 2026, and the extraordinary session is planned to resume 4 December subject to confirmation. Model pending policy options as separate, dated scenarios—not as already binding law. Sources: [IMO update](https://www.imo.org/en/mediacentre/pressbriefings/pages/imo-progresses-work-on-ship-emissions-pollution-and-ocean-protection.aspx), [ISWG-GHG 22 summary](https://www.imo.org/en/mediacentre/meetingsummaries/pages/iswg-ghg-22.aspx).
- DCSA announced Track & Trace 3.0.0 on 30 September 2026, adding IoT and reefer events and improving correction/withdrawal semantics. DCSA states that initial availability is for members/DCSA+ partners before public release in the second half of 2027. Pin the actually permitted version; do not silently treat a synthetic v3.0-like fixture as public production conformance. Source: [DCSA Track & Trace 3.0.0 announcement](https://dcsa.org/newsroom/track-and-trace-3-0-0-iot-reefer-visibility).

Rule publication time, effective time, schema/profile version and decision time must be separate fields. A later policy/schema revision must not silently rewrite past decisions or shipment history.

## 10. Authority and claim ceiling

The output is a **proposed plan** only. It is not a berth booking, capacity reservation, dispatch command, navigation recommendation, safety clearance, approved design, customs decision, contractual commitment, delivery proof or payment finality. Mycelix's separately authorized owner workflow remains necessary for real commitments; Sol Atlas remains read-only. Passing this benchmark establishes optimizer correctness only within the frozen candidate set and declared input envelope, not operational safety, regulatory compliance, seaworthiness, real-world savings or predictive accuracy.

## Research references

- IMO, *2023 IMO Strategy on Reduction of GHG Emissions from Ships*: https://www.imo.org/en/ourwork/environment/pages/2023-imo-strategy-on-reduction-of-ghg-emissions-from-ships.aspx
- Nikolopoulos & Boulougouris, *A Robust Holistic Optimization Ship Design Approach with Voyage Simulation Principles and Its Applications*, published 27 July 2026: https://doi.org/10.5750/ijme.v168iA1.1498
- Guo et al., *Ensemble-forecast-based uncertainty-robust framework for ship energy-efficient weather routing with arrival window assurance*, August 2026: https://doi.org/10.1016/j.cie.2026.112143
- DCSA, *Track & Trace 3.0.0: IoT and Reefer Visibility*, 30 September 2026: https://dcsa.org/newsroom/track-and-trace-3-0-0-iot-reefer-visibility

The research sources justify testing coupled optimization and uncertainty. They do not substantiate a performance claim for this implementation.
