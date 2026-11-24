# SAFRB: paradigm shifts, competitive position and scorecard

## Core thesis

The valuable product is not another petroleum chatbot. It is an **evidence-qualified energy resilience system** that reconciles physical movements, exposes uncertainty, diagnoses constrained delivery and compares robust actions before an authorised operator acts.

The initial commercial wedge is fuel inventory and movement reconciliation. A national resilience twin is the longer-term public-interest benchmark and research platform.

## Six paradigm shifts

### 1. From reported volume to epistemic inventory

Represent what is known about each physical quantity: reported, measured, derived, forecast, scenario. Then separately classify whether the measured product is quality-released, accessible, located within a known physical boundary and time-valid.

The system should be able to say "we have not established this volume" instead of converting missing data into an estimate that looks like stock.

### 2. From country balance to delivery service

The national sum is not the outcome. The outcome is whether the correct grade reaches a specific demand zone by a deadline. Model time-expanded paths, edge capacities, product compatibility, tank limits, transit delay, transfer loss and outage dependencies.

### 3. From prediction to causal intervention

Symthaea should maintain competing causal hypotheses, state their evidence and uncertainty, and choose the next observation with highest expected information gain. A useful output includes a falsifying measurement or test.

### 4. From cheapest plan to resilient plan

Compare cost-optimal, robust and stochastic plans. Evaluate worst-zone service, CVaR95 shortage, recovery time and fairness, not just total litres or average cost. Keep critical demand categories explicit and operator/government governed.

### 5. From digital twin visualisation to executable assurance

Every run binds source digests, topology, state snapshot, model version, boundary conditions, solver tolerance, assumptions and output digests. A replay package should reproduce the same accounting and expose every difference. Independent oracles qualify results.

### 6. From fossil-fuel continuity to energy-service resilience

The benchmark should include demand substitution and resilience alternatives: electrification of appropriate fleets, rail availability, demand response, storage, distributed generation, renewable diesel and sustainable fuels where lifecycle and supply evidence supports them. Do not optimise simply to maximise fossil throughput. Evaluate continuity of essential services and transition pathways.

## Platform responsibilities

- **Symtropy:** physical/network constraints, state transition simulation, deterministic replay, later hydraulic/process solvers.
- **Symthaea:** causal hypotheses, demand/risk forecasting, uncertainty, active information gathering, human-readable evidence chain. HDC/CfC/LTC is an experiment to compare, not a presumed winning architecture.
- **Mycelix:** identity, signed attestations, portable credentials, event lineage, selective disclosure, replay/idempotency protection and cross-organisation reconciliation. Signatures authenticate a claim's issuer and integrity; they do not prove the physical truth of a sensor reading.
- **Sol-Atlas:** geospatial infrastructure graph, land/access rights, environmental receptors, communities and spatially distributed consequences.
- **Independent qualification:** external data, metrology, model review, negative controls and separate authority for execution.

## Competitive landscape

| Incumbent / standard | Existing strength | Where not to overclaim | Luminous wedge to test |
|---|---|---|---|
| AVEVA PI System | Industrial historians, data connectivity, contextualisation and operational data distribution | Do not claim to replace a site's trusted historian | Evidence-bound cross-organisation reconciliation over existing systems |
| Cognite Data Fusion | Industrial DataOps and OT/IT/engineering contextualisation | Vendor productivity claims are not independent benchmark results | Independent replay, quantity reconciliation and uncertainty provenance |
| SLB Lumi / digital platform | Petroleum-domain AI, domain models and operational context | SLB has field domain expertise and mature deployments | Vendor-neutral evidence assurance across parties and operators |
| Aspen HYSYS / AspenTech | Rigorous process simulation and refinery/process engineering | Do not replace validated process solvers with an unvalidated Symtropy model | Versioned, reproducible boundary conditions and cross-model qualification |
| GS1 EPCIS 2.0.1 | Standardised cross-enterprise visibility event sharing | Do not invent a proprietary duplicate event vocabulary | Map Mycelix credentials and event lineage to EPCIS event semantics |
| W3C VC Data Model 2.0 | Standardised verifiable claims | A credential signature is not a metrology certificate | Evidence policy, revocation, selective disclosure and trust-domain controls |

The differentiator is not "we have a digital twin" or "we use AI." It is the measurable assurance layer above heterogeneous incumbent systems.

## Scorecard and proposed thresholds

| Measure | v0.2 status | v1 pilot target (proposed) |
|---|---|---|
| Grade compatibility | Enforced in code | Zero cross-grade transfers in negative tests |
| Verified usable inventory | Evidence gates in code | Zero unverified litres promoted |
| Node/day mass balance | Checked in code | Zero non-zero residuals for accepted runs |
| Tank and route capacity | Enforced for dispatch and receipts | Zero accepted capacity violation |
| Scenario calibration | Not present | Independent review and frozen historical replay |
| Demand forecast | Not implemented | ≥10% WAPE improvement vs strongest eligible baseline with calibrated intervals |
| Allocation resilience | Not implemented | ≥10% CVaR95 critical-demand shortage improvement at matched cost/constraints |
| Provenance | Evidence struct only; no signature verifier | Independent signature/credential, replay and custody tests |
| Causal diagnosis | Not implemented | Higher event recall at equal false alarms and calibrated abstention |
| Safety | No control integration | Zero unauthorised OT writes; default read-only architecture |

Thresholds are gates to test, not a claim that a percentage gain is already achievable. For physical correctness, zero balance residuals in a self-contained accounting kernel are a software invariant, not evidence of real-system accuracy.

## Near-term highest-value work

1. Finish the standalone v0.2 compile/test/Clippy qualification.
2. Add a separate planner tournament: min-cost flow, LP, robust and two-stage stochastic optimisation.
3. Create a time-expanded disruption corpus with supplier outages and 7/14/30/60/90-day scenarios.
4. Extend adversarial tests for late, duplicated, conflicting, off-spec and replayed events.
5. Seek a read-only data partner: terminal operator, importer, distributor or large diesel consumer.
6. Publish a concise, technically grounded comment on NIST SP 800-82r4 IPD only after human review; do not submit automatically.

## Safety boundary

No model may directly command PLC, DCS, SIS, terminal control or fuel allocation. Initial use is read-only. Recommendations remain advisory. If the model loses data, authority, time synchronisation or evidence integrity, it must reduce confidence or abstain, not continue optimising on silently stale state.
