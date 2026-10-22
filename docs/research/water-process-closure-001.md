# WATER-PROC-CLOSURE-001 — full-process treatment, residual, and lifecycle closure

**Status:** synthetic-first contract and fixture proposal; no plant model or process has been qualified.  
**Research date:** 2026-10-19.  
**Parent:** [WATER-PROC-CLOSURE-001 tracking issue #2152](https://github.com/Luminous-Dynamics/symtropy/issues/2152).

## Executive decision

A water treatment process should be assessed as a resource-transforming physical system, not as a black box with a single removal-efficiency score. A filter, membrane, biological train or dewatering unit may produce several output streams and consume utilities over a cycle. Those streams need separate physical and evidentiary semantics.

This tranche freezes **16 synthetic accounting cases plus a separate expected-output oracle**. It is a data-contract / fixture tranche only. It does not implement a runtime evaluator, run a treatment simulator, qualify a treatment technology, or establish health, reuse, discharge or lifecycle claims.

## Paradigm shift

From:

~~~text
influent -> treatment box -> “clean water” score
~~~

To:

~~~text
source-specific input and assumptions
  -> declared process stages and internal recycles
  -> treated effluent / reuse output / backwash / reject / sludge water
  -> residual solids / by-products / unknown streams
  -> electricity / heat / chemicals / consumables / time
  -> cycle and maintenance obligations
  -> qualified measurements and separate approval claims
~~~

The complete boundary is the key. Internal recycle is a flow within the boundary; it must not be counted as a new external input at the full-system boundary. Water volume is not total mass, contaminant concentration is not contaminant mass, and process mass closure is not evidence of treatment efficacy.

The US EPA's wastewater life-cycle assessment material documents how treatment alternatives can trade improved nutrient removal against increased electricity, chemicals, greenhouse-gas emissions and residual volumes. South Africa's Department of Water and Sanitation describes faecal-sludge management as a service chain including treatment and safe reuse/disposal. These motivate evaluating the whole cycle, not only the treated-water outlet:
- https://www.epa.gov/nutrientpollution/research-and-reports-nutrient-pollution
- https://www.dws.gov.za/Campaigns/Sanitation/faecalsludgemanagement.aspx
- https://www.dws.gov.za/Campaigns/Sanitation/default.aspx

## Ownership and composition

- **Symtropy:** water-process scenario composition and physical water/tracer semantics within demonstrated model fidelity. This folder owns water-specific process fixtures and benchmark boundaries, not a duplicate general-purpose mass-accounting engine.
- **Symthaea:** coordinate with [PIE-001 interval-aware process closure #1606](https://github.com/Luminous-Dynamics/symthaea/issues/1606) and the [WATER-MFG engineering boundary #5966](https://github.com/Luminous-Dynamics/symthaea/issues/5966). Do not silently duplicate those semantics or turn a generic mass balance into a water-quality authority.
- **Sol-Atlas:** source-attributed regional utility/resource context and qualified model/run manifests; spatial context does not prove plant feasibility or treatment efficacy. The existing [water-engineering draft #112](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/112) keeps detailed pressurized hydraulics with EPANET and anticipates a distinct SWMM path for appropriate sewer/drainage cases.
- **Mycelix:** preserve ownership, permission, custody and actual return/recovery/disposal events through the existing [manufacturing/circularity lifecycle architecture #3095](https://github.com/Luminous-Dynamics/mycelix/issues/3095). A plan to recover sludge is not evidence that recovery occurred.
- **Accountable operator, qualified engineer, laboratory and regulator:** retain process operation, sampling, health, reuse and discharge approval authority.

The first goal is a stable, reviewable data contract and corpus. A later evaluator should consume the appropriate shared process-balance semantics rather than introduce a competing ontology.

## Independent accounting planes

### 1. Water-volume balance

For a declared process interval, use exact base units and explicit time alignment:

~~~text
inflow + opening storage
  = treated effluent + reuse output + backwash
    + sludge-associated water + reject/concentrate
    + declared losses + closing storage
~~~

This is a check of the declared accounting boundary, not a hydraulic calculation. A value may be unknown, simulated, measured or estimated; missing water must not be silently relabelled as loss. The tolerance is explicit and in the same unit as the compared quantities. Do not combine litres, flow rates and period totals without declared conversion and interval semantics.

### 2. Non-water material mass balance

Inputs and every output/residual stream use explicit mass intervals in a common declared unit/basis. Stream classes remain distinct: product, by-product, recycle material, reject, residual solids, tracked material and waste.

For point values, matching totals within an explicit tolerance may be ExactBalanced; point values outside tolerance are ExactUnbalanced. For uncertain intervals:
- overlapping admissible totals yield PossibleWithUncertainty, not exact closure;
- disjoint intervals yield ImpossibleWithinBounds;
- missing required streams yield Incomplete;
- invalid units / boundary semantics yield Invalid.

A wide uncertain range must never look like stronger conservation evidence than a narrow measured range. Electricity, heat and time are process requirements but are not matter to insert into a mass equation.

This tranche records mass interval inputs and output-stream intervals to make independent hand calculation possible. It does not implement the generic interval mass-balance algorithm.

### 3. Tracked tracer / species semantics

Concentration, total tracked mass and normalized water-quality/ecological indexes are different quantities. For a closed no-reaction dilution/mixing fixture, tracer mass can be conserved while concentration changes because carrier-water volume changes.

Do not emit a species-mass conclusion from concentration alone without compatible water volume/flow, physical tracer identity, time scope, composition and appropriate reaction boundary assumptions. If a reaction or transformation is expected but no qualified reaction model exists, mark the tracer result not assessable. This is not a universal aqueous-chemistry model.

Symtropy's existing [#476 tracer semantics](https://github.com/Luminous-Dynamics/symtropy/issues/476) already establishes the principle that tracer concentration/mass must not be confused with normalized ecological or toxicity indices.

### 4. Residual and lifecycle closure

Record residual route state separately from water and material balance. Qualified means only that the synthetic fixture declares an appropriate route and required supporting fields; it does not mean any real residual is safe or approved for beneficial use.

A residual can remain:
- not characterized;
- characterized but not eligible for the intended use;
- eligible in principle but without evidence of actual recovery;
- recovered with a measured quantity and required quality/permission evidence;
- explicitly disposed or of unknown disposition.

Prevent double-counting material as both harvested component/retained product and recycled constituent. Recovery quantity is not environmental benefit; the process energy, handling and downstream requirements remain separate.

### 5. Utilities and treatment / safety claims

Electricity, heat, chemical/consumable inventory and process time should be present for a complete-cycle assessment. Missing inputs yield incomplete resource accounting, not zero-burden technology. A single successful interval does not establish full-cycle performance if backwash, cleaning, media renewal, downtime, reject treatment or consumable replacement is omitted.

Treatment efficacy, potable safety, reuse eligibility, sanitation effectiveness, discharge compliance and environmental superiority are explicitly outside the claim ceiling of this corpus. They need their own profiles, qualified measurements, applicable local requirements and independent review. ISO 24516-2:2019, reviewed and confirmed current in 2025, is a useful asset-management anchor for treatment plants, sludge treatment facilities, pumping stations, reservoirs, tanks, dosing equipment and metering:
https://www.iso.org/standard/64667.html

## Frozen synthetic cases

Inputs: fixtures/water-process-closure-001-inputs-v1.json  
Expected outputs: fixtures/water-process-closure-001-oracle-v1.json  
Draft 2020-12 schemas: schemas/water-process-closure-001-inputs-v1.schema.json and schemas/water-process-closure-001-oracle-v1.schema.json.

| Case | Adversarial / control condition | Expected accounting lesson |
|---|---|---|
| WPC-01 | Complete point-valued positive control | Exact balance is an accounting-only pass |
| WPC-02 | Declared water outflow misses input | Water balance is unbalanced |
| WPC-03 | Internal recycle wrongly counted as external | Invalid boundary semantics |
| WPC-04 | Dewatering water plus dry cake, filtrate solids and captured residual | All declared outputs are included |
| WPC-05 | Membrane permeate plus reject/concentrate | Reject cannot disappear from the boundary |
| WPC-06 | Recovery claimed with no qualification / permission evidence | No recovery claim |
| WPC-07 | Required residual route missing | Lifecycle incomplete |
| WPC-08 | Exact material output lower than exact input | Material balance unbalanced |
| WPC-09 | Input/output intervals overlap | Possible with uncertainty, not exact |
| WPC-10 | Input/output intervals are disjoint | Impossible within bounds |
| WPC-11 | Dimensional-unit mismatch | Invalid before numeric interpretation |
| WPC-12 | Closed dilution fixture, tracer mass conserved while concentration changes | Mass—not index—supports the conclusion |
| WPC-13 | Concentration/index assertion without tracked mass | Tracer result not assessable |
| WPC-14 | Interval looks successful but cleaning/backwash/renewal incomplete | Full cycle incomplete |
| WPC-15 | Material stream inventory incomplete | Material closure incomplete |
| WPC-16 | Electricity/heat inventory missing | Full-process resource accounting incomplete |

All numbers are synthetic. The expected-output file is separate from the input corpus to support an independent future evaluator. No result is reported as having been computed by a production implementation.

## Acceptance gates for the future evaluator

- 16/16 outputs must agree with the separately frozen, independently hand-calculated oracle.
- Zero exact-closure passes for unbalanced or invalid water boundaries.
- Zero internal recycling counted again as external system input/output.
- Zero wide or overlapping intervals promoted to exact balance.
- Zero unknown streams, mismatched units or missing utilities silently defaulted to zero.
- Zero recovery claims based only on a plan, route label or concentration/index.
- Input/stream order cannot change canonical outcomes; duplicate stream IDs and mismatched time grids fail closed.
- Every result binds exact profile, schema, fixture and evaluator revisions and records a deterministic result digest.
- Accounting closure must not emit treatment, potable-safety, reuse, discharge or environmental-superiority claims.

### Proposed local accounting performance hypotheses (unmeasured)

After correctness is established, measure p50/p95/max for:
- 100 intervals / up to 1,000 streams: p95 ≤ 10 ms;
- 1,000 intervals / up to 10,000 streams: p95 ≤ 100 ms;
- 10,000 intervals / up to 100,000 streams: p95 ≤ 1 s.

These are local pure-accounting hypotheses, not hydraulics, process kinetics, treatment-plant runtime or real-world service targets. Record exact revision, host/compiler, counts, output size, memory and all corpus/input digests.

## Qualification sequence

1. Review the boundary, units and case oracle independently.
2. Add a pure evaluator composed with the existing process-closure semantics, not a copied engine.
3. Validate both JSON Schemas with a standards-compliant validator and test adversarial fixtures.
4. Produce a deterministic receipt on the exact revision and independently inspect the raw artifact.
5. Only later add solver-backed profiles or approved observations; record their source, vintage, permission, calibration/validation boundary and limitations.

A schema-valid fixture or synthetic PASS cannot establish physical treatment performance. An empty or queued workflow status is not a pass.
