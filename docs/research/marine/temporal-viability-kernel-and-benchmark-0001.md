# Marine Temporal Viability Kernel and Benchmark Protocol

**Date checked:** 19 November 2026  
**Work items:** Symtropy #6238, #6271; PR #6251, #6279  
**Claim status:** research protocol and synthetic reference implementation only

## 1. The paradigm shift

Weather routing asks: *which route best satisfies a route objective under predicted conditions?*

Whole-voyage mission viability asks: *does any executable policy keep the occupied vessel inside its safety, service and resource envelope at every instant and between observations, under the uncertainty that actually matters?*

The new object is a **trajectory viability envelope**, not a route score or arrival-state snapshot. A route is not acceptable merely because the final state has enough water, energy and food. A mid-voyage essential-service breach remains a breach after a port call replenishes the resource.

This is an event-sourced state machine:
- initial craft, occupancy, service contract, resources and authority are frozen by digest;
- forecasts and scenario members retain exact identities over time;
- observations, resource changes, replenishment, faults and repairs are timed evidence events;
- point states and conservative inter-sample envelopes are checked separately;
- an infeasible/unobservable trajectory yields BLOCKED/ABSTAIN, not an automatically relaxed route.

The initial Rust tool enforces temporal checks on the declared replay data. It does **not** produce the continuous-time bounds itself or prove that an upstream physics model is conservative.

## 2. State of the art and industry comparison

### Research frontier

**15 October 2026 — Risk-aware ship weather routing with ensemble forecasts on adaptive hexagonal grids.**  
Zhang et al., *Ocean Engineering* 366, Article 127661, DOI https://doi.org/10.1016/j.oceaneng.2026.127661.

The paper already combines adaptive multi-resolution H3/hexagonal search, Traffic Separation Scheme topology, edge-level IMO second-generation intact-stability pruning, and a 50-member perturbed ECMWF ensemble. It reports:
- failed deterministic-forecast routing can realize greater exposure than a weather-blind route;
- member-coherent CVaR can bracket realized exposure missed by marginal summaries;
- stability pruning at graph edges matters where node-only evaluation can miss intermediate resonance-susceptible steps.

**Implication:** adaptive routing, ensemble-aware cost, weather-blind comparison, TSS encoding and edge-level stability cannot be presented as novel by themselves. The next challenge is to qualify the *whole mission trajectory* and preserve evidence/authority boundaries around every decision.

### Autonomous-ship planning research

- Zhao & Xie et al., 24 July 2026, survey of autonomous ship route planning, DOI https://doi.org/10.3389/fmars.2026.1889559. Persistent gaps include real-time replanning, uncertainty, multi-vessel encounters, COLREGs and practical deployment.
- Wang et al., 10 August 2026, review of model-driven to deep-reinforcement-learning path planning, DOI https://doi.org/10.3390/jmse14161477. It highlights partial observability, strategic coupling among vessels, inconsistent benchmarks, weak cross-scenario generalization and insufficient full-scale validation.

**Implication:** the benchmark must include dynamic replanning, degraded sensor/evidence states, multi-vessel traffic and deployment cost—not only shortest route or average fuel.

### Commercial sailing baseline

PredictWind already documents:
- AI Weather Router/Briefing that reads eight model routes and summarizes passage segments, demanding periods, motion/comfort and forecast confidence: https://explore.predictwind.com/features/ai-weather-router
- AI Polars learned from vessel instrumentation and sailing behaviour, including wave characteristics: https://www.predictwind.com/features/ai-polars
- Comfort routing based on RMS roll, vertical acceleration and slamming. Its help documentation says that if a route cannot be found within the requested comfort limits, it may automatically rerun without those limits so it can return a route: https://help.predictwind.com/en/articles/2884622-weather-routing-change-weather-routing-settings

**Implication:** do not claim to invent weather routing, vessel-specific polars, crew comfort briefing or multi-model route comparison. A defensible differentiator to test is hard-gate non-relaxation plus event-sourced whole-voyage resource/service viability, with an explicit infeasible/abstain result and exact receipts.

## 3. Standards and regulatory boundaries

- IMO describes the second-generation intact-stability work around five dynamic failure modes: dead ship condition, excessive acceleration, pure loss of stability, parametric rolling, and surf-riding/broaching. These are vessel- and operating-domain-specific; a generic wave threshold is not a qualified assessment. https://www.imo.org/en/ourwork/safety/pages/shipdesignandstability-default.aspx
- IMO COLREGs include 41 rules; Rule 10 addresses conduct in/near Traffic Separation Schemes. https://www.imo.org/en/about/conventions/pages/colreg.aspx
- ISO 12217-2:2022 is the published stability and buoyancy assessment/categorization standard for relevant sailing boats, including 6–24 m hull length. The ISO page says it is being revised. https://www.iso.org/standard/79073.html
- ISO/DIS 12217-2.2 (Edition 5) is now an enquiry-stage draft intended to replace ISO 12217-2:2022 and ISO 12217-3:2022. It is not yet the published standard and must not be treated as a completed compliance basis. https://www.iso.org/standard/88606.html

The software here performs no COLREGs interpretation, ISO/IMO compliance determination, vessel-specific stability calculation, or certification.

## 4. Benchmark ladder

| Stage | Measurement | Required comparator / control | Research target |
|---|---|---|---|
| Replay integrity | snapshot, window, grid, source-member and evidence digests | independent digest reconstruction | 100% exact identity; zero future information |
| Temporal service integrity | point and segment floor evaluation | arrival-only adversarial traces | zero false-feasible trajectories |
| Event accounting | inventory delta vs timed evidence events | delete, duplicate, retime and alter event controls | 100% reconciliation |
| Scenario coherence | member identity and source digest over time | member-switch and member-drop controls | zero member identity drift |
| Degraded evidence | stale/gapped/unknown measurement and hard-gate states | stale-data and missing-receipt injections | explicit BLOCKED/ABSTAIN, never silent pass |
| Forecast value | weather-informed vs weather-blind route and service outcomes | failed deterministic forecast cases | no accepted hard-gate or service-floor regression |
| Multi-vessel rules | COLREG/TSS manoeuvre and encounter suite | independent rule review / scenario oracle | zero known rule violations in the admitted test suite |
| Vessel dynamics | applicable stability/comfort failure modes | vessel-specific validated model/reference cases | no extrapolation outside qualified operating domain |
| Replanning | receipt-to-new-qualified-policy latency | fixed scenario count and named onboard hardware | report p50/p95/p99; set limits from real control cycle |
| Generalization | unseen voyage, weather event, vessel configuration and traffic regime | forward-chaining grouped holdouts | no adjacent-row leakage; report every held-out stratum |
| Operational qualification | software-in-loop, processor-in-loop, HIL, then supervised physical tests | independent system owners and test authority | progression only after each prior stage passes |

The first software target is 100% agreement with an independent reference evaluator on a preregistered synthetic adversarial corpus. That is a software-correctness target, not a marine safety claim.

Do not publish a universal p99 routing limit before the target computer, sensor/communication delay, decision/control period and scenario size are specified. Publish measured latency and resource use per hardware profile first.

## 5. Why the temporal implementation is structured this way

- **Point checks** catch observed/forecast state violations at recorded times.
- **Segment checks** catch declared continuous/inter-sample envelopes that cross a floor despite safe endpoints.
- **Endpoint containment** prevents a segment envelope from being narrower than either adjacent point uncertainty interval.
- **Cumulative exposure** prevents an outage or comfort breach from disappearing just because the system later recovered.
- **Event-sourced inventory** requires a timed receipt for energy/water/food/spares replenishment or waste-service changes.
- **Member identity continuity** prevents a forecast scenario from silently switching identity along a route.
- **Frozen grid/window digests** prevent evaluation from changing its horizon or weakening observation frequency without changing the mission identity.

The evaluator treats an inter-sample envelope as an asserted input. Independent validation of the process that calculates those bounds is a separate required work package.

## 6. Next technical steps

1. Independent reference implementation for point/segment and event-balance evaluation.
2. Property-based generation of floor crossings between safe endpoints, scenario member drift, stale observations and stock-event accounting defects.
3. Separate COLREG/TSS encounter evaluator and multi-vessel scenario suite.
4. Vessel-specific dynamics adapter with validated operating-domain and uncertainty metadata.
5. Shadow replay on measured instrument/forecast records with rights and first-availability evidence.
6. Software-in-loop and processor-in-loop testing on representative onboard compute; then HIL and only later supervised physical validation.

## 7. Claim ceiling

This is a synthetic replay/trajectory-qualification tool. It is not a live monitor, route planner, vessel stability model, safety certificate, seaworthiness determination or navigation authority. Real service floors and operating envelopes require the actual craft, crew, equipment, applicable law, qualified engineering and independent validation.
