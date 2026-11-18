# Pump Cost per Verified Service v0.1

**Research snapshot:** 18 November 2026  
**Owner:** Symtropy #6023  
**Implementation:** dependency-free Stage 0 Rust evaluator.

## Paradigm: the useful service is the denominator

A released pump is not itself the economic product. Nor is total pumped volume a sufficient measure: a volume delivered below the required pressure or during a period when a required service area failed is not the declared service.

This evaluator consumes four existing owners rather than replacing them:
- manufacturing cost per released output: Symtropy #5966;
- pump acceptance-to-service transferability: Symtropy #6008;
- whole-system measured energy service: Symtropy #5103;
- per-area service floors and verified restoration: Symtropy #5027/#5069.

The functional unit is one m³ delivered within the declared hydraulic service envelope, during a study window where every required service area has current, complete, independently referenced MET evidence. This is a hydraulic claim only; it does not establish potable water.

## Why this is not a new cost or controller engine

- #5966 owns cost-item aggregation, allocation and amount-unit semantics. This evaluator consumes a pre-aggregated, period-aligned total cost interval.
- #5103 owns pump system energy boundaries and matched hydraulic duty. This evaluator consumes the energy interval and measurement/meter references; it does not calculate pump/system energy.
- #5027 owns service-floor evaluation. This evaluator consumes the per-area floor dispositions and never re-evaluates the hydraulics.
- #6008 owns factory-test-to-installed-duty transferability. This evaluator consumes its transfer disposition and exact product serial/configuration.
- #5069 owns restoration chronology and stable dwell. Its current verified-restoration receipt should be required by a production integration after a repair/outage.

## Industry/SOTA comparator snapshot

### DOE MEASUR / PSAT

The US DOE's MEASUR programme includes PSAT/pumping-system assessment and more than 70 calculators. The pump assessment uses field-measured data and system curves to assess potential energy improvements and compare what-if system modifications:
- https://www.energy.gov/cmei/ito/measur
- https://www.energy.gov/cmei/ito/pump-systems

This is a strong whole-system energy baseline. It is not, by itself, the cross-owner accepted-product, per-area service-floor, transferability and traceability composition tested here.

### Commercial digital pump/system capabilities

- Grundfos iSOLUTIONS advertises whole-system connected pump monitoring, remote control, fault prediction and system optimization: https://www.grundfos.com/us/learn/research-and-insights/grundfos-isolutions
- Siemens Opcenter Execution/MOM advertises production execution, resource tracking, genealogy, traceability and quality management: https://www.siemens.com/en-us/products/opcenter/execution/

These are vendor-published capability claims, not independent comparative validation. We should not claim they lack equivalent internal controls. The benchmark compares the same evidence and service boundaries and tests whether Luminous can add an interoperable evidence-composition layer.

### Digital-twin standards

- [ISO 23247-6:2026](https://www.iso.org/standard/87426.html) is published (14 July 2026) and specifies integrated, unified and federated manufacturing digital-twin composition.
- [ISO/AWI 23247-8](https://www.iso.org/cms/live/live/en/sites/isoorg/contents/data/standard/09/48/94858.html?browse=tc) is only an approved work item under development (stage 20.00; registered 23 October 2026), scoped to VVUQ throughout the digital-twin lifecycle. Do not describe it as published or normative yet.

The strategic implication is not another dashboard or MOM product. It is to qualify a common decision from owner-owned manufacturing, transferability, cost, energy, metrology and service-floor evidence—with exact provenance and explicit abstention.

## Decision contract

The 33-case corpus covers:
- accepted but non-transferable product;
- exact transfer serial/configuration join;
- one area failing while aggregate volume remains high;
- unknown areas, missing areas and duplicate area IDs;
- stale/missing floor, volume calibration or observation references;
- mismatched service profile, boundary or time horizon;
- missing energy meter/calibration/boundary evidence;
- cost source/allocation, period and currency/unit mismatches;
- zero or interval-uncertain qualified volume;
- potable claim misuse;
- cost and energy interval errors and exact positive controls.

The evaluator counts only MET service areas. A BREACHED status blocks the competitive score; if a different area is UNKNOWN, the disposition preserves both facts. No weighted average can hide a failed community.

## Metric and arithmetic

Cost/energy inputs are intervals and the service denominator is an interval. Ratios are outward-rounded and only computed if the lower bound of qualified service volume is positive.

- `cost_per_m3_milli_minor` = thousandths of minor currency unit per m³.
- `kwh_per_m3_milli` = thousandths of kWh/m³.

No silent FX, tariff conversion, inflation, discounting, asset life, capital amortization or period annualization is permitted. Those require an explicit owner-authorized policy with its own evidence.

## Proposed evaluation targets

- Zero competitive ratios for any non-transferable, breached, unknown, stale or incomplete episode.
- 100% exact joins across product, batch, currency, cost period, energy boundary, time window and service-floor policy.
- 100% agreement with independently authored interval/disposition oracles on held-out fixtures.
- Zero denominators and interval overflow are explicit states, not coerced numeric values.
- No volume enters the qualified denominator without all required per-area MET evidence.
- Report cost and energy intensity as intervals; report service coverage and breaches alongside them.

## Limits

The current evaluator runs only synthetic records. It does not prove real pump performance, field delivery, water quality, affordability, energy savings, reliability, site safety or deployment readiness. The production integration must consume the exact signed/verified owner receipts, not the synthetic fixture references.
