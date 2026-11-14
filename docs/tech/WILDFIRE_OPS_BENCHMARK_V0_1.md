# Wildfire Operations Resilience Benchmark v0.1

**Issue:** [WILDFIRE-OPS-001](https://github.com/Luminous-Dynamics/symtropy/issues/2200)
**Fixture:** [WILDFIRE_OPS_V0_1.json](../../benchmarks/fixtures/WILDFIRE_OPS_V0_1.json)
**Status:** synthetic-first scenario evaluator; not an operational evacuation tool.

## 1. Research question

Can a decision method preserve evacuation feasibility and access constraints when hazard observations, road availability, lifeline status and shelter capacity are incomplete, delayed, contradictory or affected by one common cause?

This does not attempt to beat professional fire-behavior models at predicting every fire perimeter. It evaluates **decision quality conditional on an explicitly declared evidence set**. The operational question is: does a policy remain feasible, disclose uncertainty, and preserve hard safety/access constraints when reality differs from its initial plan?

## 2. Current data/model landscape and the comparison boundary

- **NASA FIRMS** publishes satellite-derived active-fire/thermal-anomaly detections. NASA states that these detections have limited accuracy, can reflect fire, hot smoke, agriculture or other sources, can be obscured by cloud cover, and represent a sensor pixel rather than proof that the whole pixel is burning. MODIS pixels are approximately 1 km and VIIRS approximately 375 m; confidence fields differ across products. Sources: https://firms.modaps.eosdis.nasa.gov/map2/ and https://firms.modaps.eosdis.nasa.gov/descriptions/FIRMS_MODIS_Firehotspots.html
- **USGS/USFS Monitoring Trends in Burn Severity (MTBS)** maps large-fire extent and severity retrospectively. The portal lists a release timeline of about 3 weeks to 12 months after containment, varying by assessment type: https://burnseverity.cr.usgs.gov/products/mtbs
- **US Forest Service Wildfire Risk to Communities** is planning-level community risk/exposure context, not an incident-scale, locally calibrated live forecast or individual-home risk estimator: https://wildfirerisk.org/about/faq/
- **Current evacuation optimization research:** Moreno et al., *Evacuation network design and planning under traffic congestion: An application to wildfires in Canada*, *Transportation Research Part E* (September 2026), uses a Cell Transmission Model and Branch-and-Benders-Cut algorithm, and reports benchmark instances plus a case study of the 2023 Chibougamau evacuation: https://doi.org/10.1016/j.tre.2026.104965

### Fair comparison rule

The 2026 evacuation paper is a relevant method-level comparison for evacuation network design and congestion. It is **not automatically a direct benchmark** for this v0.1 corpus: our synthetic cases test epistemic limits, route loss, common-cause lifeline availability and accessibility constraints rather than reproducing the paper's traffic/network model. A later matched comparison must align road graph, demand, evacuation zones, congestion dynamics, capacity rules, scenario uncertainty, computation budget and evaluation denominator. Do not claim superiority based on unlike tasks.

## 3. Paradigm shift: from predicting hazards to resilient decisions

A single “risk score” or predicted perimeter cannot represent the different kinds of uncertainty in an incident. Separate:

1. **Observation:** what the sensor, agency or model actually reported, including its source, time, coverage and limitations.
2. **Hazard hypothesis:** what might be burning or exposed, with alternatives and confidence evaluated externally.
3. **Network state:** roads, shelters, communications, power, water and responder links, each with independent status and time.
4. **Feasible action:** the set of plans that satisfy route, vehicle, shelter and accessibility constraints under the available evidence.
5. **Decision value:** whether a new observation could change the feasible plan before its useful decision deadline.

The new target is not “one accurate map.” It is a policy that degrades safely under observation loss, admits when no evidenced plan is feasible, and tells the operator exactly which missing measurement or failed capability changes the decision. Shelter total capacity and accessible-trip capacity are observations too: they must carry an observation time, be no later than the decision’s as-of time, and remain unusable when marked stale, unknown or conflicted.

## 4. Project ownership

- **Symtropy** owns the synthetic scenario replay, route-plan feasibility oracle, resource constraints, common-cause stress cases and exact-head reference receipt.
- **Symthaea** can propose alternative hypotheses, candidate plans and decision-sensitive observations. Its stated confidence is not assumed calibrated; scoring must use held-out evidence.
- **Sol-Atlas** owns geospatial data ingestion, terrain, spatial context, source freshness and exposed-asset layers. Wildfire benchmark code must consume qualified spatial data rather than create a second map/facts database.
- **Mycelix** can provide evidence provenance, actor permissions, disclosure policy and signed cross-agency receipts. A valid evidence receipt never grants dispatch or incident-command authority.
- Compose with existing route, habitat/site selection, water, transport, lifeline and resilience kernels; this issue is the wildfire-specific evaluator.

## 5. Frozen synthetic corpus

`WILDFIRE_OPS_V0_1.json` contains six cases. All hazards, routes, demand, capacities, timestamps and oracle truth are invented.

| Case | Challenge | Required behavior |
|---|---|---|
| HOTSPOT-NOT-PERIMETER-001 | Satellite-like hotspot with no confirmed perimeter; future retrospective severity included as a decoy | Keep a point detection distinct from a verified operational perimeter and exclude future information from active-response input |
| EVAC-ROAD-LOSS-002 | 12-minute route is blocked, alternative takes 22 minutes | The shortest-route baseline selects the blocked option; the constrained baseline rejects it and selects the evidenced alternative |
| COMMON-CAUSE-LIFELINE-003 | A declared synthetic cause affects primary communications, power and road availability | Preserve dependencies; reject a plan that requires unavailable lifelines; consider a distinct redundant communications option |
| MISSING-CONFLICTING-OBSERVATIONS-004 | Two route states are unknown/conflicted and the perimeter evidence is stale | Do not silently promote either route or the perimeter; abstain when no plan passes hard evidence gates |
| ACCESSIBLE-EVACUATION-005 | Shorter route reaches a shelter whose accessible capacity is below assisted-trip demand | Reject that plan even when aggregate shelter capacity and travel time look better |
| POSTFIRE-RECOVERY-006 | Burn-severity and watershed observations arrive after the active-response as-of time | Exclude retrospective products from active-response projection; keep recovery analysis as a separate decision phase; reject unknown route state |

The fixture uses pre-enumerated candidate plans so the initial oracle is small and hand-auditable. It does not yet solve traffic flow, congestion assignment, fire spread, wind-coupled propagation, smoke dispersion, shelter choice or a city-scale evacuation network. Those require separate, validated kernels rather than being smuggled into a toy score.

## 6. Reference baselines

- **B0 — oracle:** hidden actual road/incident state and expected feasible choice, evaluator only.
- **B1 — static shortest route (implemented reference):** choose the candidate plan with lowest stated travel time without applying road, lifeline, shelter or accessibility constraints. This deliberately weak control provides measurable failure cases.
- **B3 — evidence-bounded feasible choice (implemented reference):** use the as-of projection; accept only plans with all route edges explicitly confirmed open, all declared required lifelines explicitly confirmed available, sufficient confirmed shelter capacity for total demand, and sufficient accessible capacity for assisted-trip demand. Select the shortest remaining plan or return an explicit no-evidenced-feasible-plan result.
- **Future B2 — dynamic network optimization:** add congestion, departure waves, capacity sharing and vehicle/shelter assignment only after a traffic-network oracle and matched comparison contract exist.

B1 is a negative control, not a representation of all commercial evacuation software. B3 is a deterministic candidate-plan gate, not yet a robust optimizer or a validated real-world recommendation engine.

The receipt includes a corpus-level scorecard so reviewers need not infer method behavior by manually reading the six cases. For this exact synthetic corpus, the frozen reference assertions expect B1 to select five plans and all five to fail at least one feasibility gate, with two hidden blocked-edge violations and one of six oracle-choice matches. B3 should select three feasible plans, abstain in the other three cases, select zero infeasible plans, match all six predeclared toy-oracle outcomes, and report zero choice mismatches. These values are **synthetic acceptance checks only**; they are not field benchmarks or evidence of safer real-world evacuation.

## 7. Metrics

Keep feasibility, service coverage, time, accessibility, lifeline continuity, uncertainty and computation cost separately visible.

**Non-compensatory feasibility gates**
- Zero selected plans with a road marked blocked, unknown, stale or conflicted.
- Zero selected plans that require a lifeline marked unavailable, unknown, stale or conflicted.
- Zero demand reported served when shelter capacity is insufficient.
- Zero omitted assisted trips when accessible capacity is insufficient.
- If no plan passes all gates, report that no evidenced feasible plan exists; do not fabricate “safe” advice.

**Decision performance**
- Candidate-plan feasibility precision/recall against hidden synthetic feasibility truth.
- Blocked-edge violation count, unavailable-lifeline violation count, shelter-capacity overrun and accessible-trip shortfall.
- Travel-time cost among feasible plans, compared only after hard gates.
- Coverage in a declared useful time window, unserved demand and stranded demand when scenario labels permit it.

**Robustness and information quality**
- Results under stale/missing/conflicting route status, lost feeds, alternate perimeter hypotheses, common-cause failure and delayed retrospective data.
- Rate of invalid observation promotion (hotspot to perimeter, retrospective severity to live incident state).
- Explicit abstention reasons and the count of scenarios with no feasible plan.
- Deterministic canonical output under input-order permutations, plus fixture SHA-256 and exact source commit.

**Human and environmental effects**
- Accessibility, health/smoke protection, power, communications, water, ecological recovery and economic costs remain separate outcome dimensions. Do not optimize them into one score without openly declared weights and sensitivity analysis.
- CPU runtime on a synthetic fixture is not evidence of reduced field response time, injuries, mortality or better incident outcomes.

## 8. Fail-closed invariants

1. A hotspot detection is not a verified continuous fire perimeter.
2. A stale perimeter cannot be silently treated as current.
3. Missing road data is not proof that a road is open.
4. A future burn-severity or watershed observation cannot enter an earlier active-response projection.
5. All route/shelter/lifeline references must resolve.
6. Unknown/conflicted state is never coerced into confirmed availability.
7. Travel-time improvement cannot compensate for blocked roads or violated accessibility/shelter capacity.
8. If every plan is rejected, output abstention/infeasibility with binding reasons.
9. Oracle truth and expected invariants never enter the candidate projection.
10. No autonomous real-world dispatch, evacuation order, suppression operation or incident command.
11. No individually identifying location data in v0.1; only synthetic aggregate demand.
12. Passing synthetic tests is not evidence of accurate wildfire predictions, operational safety or regulatory compliance.

## 9. Proposed acceptance gates (not results)

- 100% agreement with hand-computed choice/abstention outcomes for the six frozen cases.
- 100% rejection of candidate plans using blocked, unknown, stale or conflicted road states.
- 100% rejection of candidate plans requiring unavailable or unverified mandatory lifelines.
- 100% preservation of shelter total capacity and accessible-trip capacity as hard constraints, including stale/future observation rejection.
- Zero promotion of hotspot detections to perimeter claims or post-fire observations to active-response inputs.
- Exact candidate/oracle separation and stable canonical receipt under all input-order permutations.
- Every comparison records exact code SHA, fixture digest, policy version, as-of time and denominator.
- No claim to outperform evacuation network SOTA until a matched traffic model, external held-out scenario set and independent operator-reviewed comparisons are available.

## 10. Claim ceiling and next stage

Passing the Phase 0 evaluator proves only that this Rust reference method enforces its declared synthetic constraints. It does not establish fire-behavior prediction accuracy, safe real-world evacuation, fewer injuries/deaths, dispatch effectiveness, faster response or successful ecological restoration.

This increment adds a separate Rust binary that independently recomputes B1/B3 selections, per-plan feasibility, the blocked-edge count, hidden-oracle scorecard totals and the raw fixture digest. CI must run this verifier against the generated receipt and exact checked-out source commit before uploading the artifact. Next, add a matched traffic-capacity/departure-wave model, evaluate robustness on held-out network topologies and hazard ensembles, and seek a qualified agency tabletop comparison with matched inputs and human-review safeguards.