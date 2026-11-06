# Industry performance comparison benchmark v0.1

**Status:** source-attributed public evidence plus a strict comparison gate. No pooled industry ranking or Luminous superiority claim is produced.

## 1. Public comparison data available as of 6 November 2026

### Komatsu MC51 product-page figures

The official regional MC51 pages publish distinct quantities:

| Published claim | Region/source scope | Metric semantics |
|---|---|---|
| 20 m³/h typical maximum cutting rate | Australia | Cutting-rate claim |
| 30 m³/h in-situ material handling | Australia | Material handling claim |
| Up to 60 m³/h cut-material removal | United States | Removal claim |
| 320 kW | US product material | Machine power figure |
| Up to 250 MPa stated rock-hardness capability | US product material | Advertised capability |

Sources:
- https://www.komatsu.com/en-au/products/underground-hard-rock/mining-and-tunneling-equipment/mc51-mechanical-cutter/
- https://www.komatsu.com/en-us/products/underground-hard-rock/mining-and-tunneling-equipment/mc51-mechanical-cutter/

Do not combine the 20, 30 and 60 m³/h figures into one productivity result. Their metric definitions differ. Do not equate 250 MPa advertised hardness capability with a measured mean UCS without a shared test definition and context.

### Cementation / Niobec / Komatsu MC51 field validation

Cementation's official 2 September 2026 report states that nearly ten months of underground development-drift validation led to cutting-head refinement, a 25% reduction in downtime and peak advance greater than 3.8 m per shift and greater than 5.7 m in one day.

Primary source:
https://www.cementation.com/innovation-in-action/

International Mining's 3 September 2026 story reports the same collaboration and figures:
https://im-mining.com/2026/09/03/komatsu-mc51-mechanical-cutter-progresses-with-commercial-sale-validation-with-cementation-at-niobec/

The secondary article is not an independent second field trial. The 25% is a relative downtime reduction against a prior cutting-head baseline, not absolute machine availability. Advance per shift and advance per day have different denominators.

### South African undercutting/vacuum extraction trial

Raju & Esterhuyse, *Mining, Metallurgy & Exploration*, published 1 October 2026, report:
- seven-month on-mine validation;
- average 1.20 m per cutting hour and peak 1.4 m/cutting hour;
- business-case advance target 1.05 m/h;
- predominantly anorthosite averaging 237 MPa UCS;
- 88% extraction efficiency for fragments below 20 mm and 95% for fines below 10 mm;
- material removal rather than breakage constrained the sustainable operating setting during evaluated productive periods.

Source:
https://doi.org/10.1007/s42461-026-01722-w

These numbers are not directly comparable with MC51 shift-based advance without matched task, geometry, geology, machine, work-cycle and denominator definitions.

## 2. Paradigm shift: benchmark evidence, not headline numbers

A credible comparator requires exact agreement across:
- canonical metric definition and version;
- unit;
- numerator basis;
- denominator basis;
- operating boundary;
- task profile;
- geology/material cohort;
- excavation geometry;
- accepted-output/quality profile;
- comparison protocol;
- machine configuration class.

The exact machine model/configuration may differ; that is the object of comparison. However, the work domain and boundary must be aligned.

Unknown context is not equal context. The evaluator returns INSUFFICIENT_CONTEXT where no known mismatch exists but required context is absent. It returns NON_COMPARABLE when known definitions/context differ. A valid outcome is "no ranking".

## 3. Executable v0.1 comparison semantics

The Rust crate `symtropy-industry-comparison-bench` validates source records and explicitly requested pairs.

- Value intervals may be bounded or have an explicit null upper bound for a reported lower-bound claim.
- Only exact metric/context matches pass the comparability gate.
- Same physical trial IDs are not independent observations even when a secondary article republishes the same figures.
- Reference-only physical capability/strength data are not ranked as throughput metrics.
- For comparable bounded intervals, difference is calculated conservatively:
  - difference lower = A lower − B upper;
  - difference upper = A upper − B lower.
- A definite winner is reported only when intervals are strictly separated in the direction specified by the metric.
- Overlapping value intervals remain NOT_RESOLVED.
- No hidden normalization, TOPSIS weights, global score or pooled industry rank is produced.

## 4. What the current public corpus demonstrates

The fixture includes 11 public evidence records and 2 synthetic records. Structural comparison confirms:

| Requested pair | Result | Why |
|---|---|---|
| MC51 >3.8 m/shift vs SA 1.20 m/cutting-hour | NON_COMPARABLE | Different unit, denominator and metric definition |
| MC51 20 m³/h cutting vs 30 m³/h in-situ handling | NON_COMPARABLE | Different numerator and metric definition |
| SA 237 MPa mean UCS vs MC51 250 MPa hardness claim | NON_COMPARABLE | Different measurement/capability definitions |
| Cementation vs secondary report of same MC51 trial | NON_COMPARABLE | Same trial ID, not independent evidence |
| Synthetic matched full-cycle pair | COMPARABLE | Exact declared contexts match; A wins only in this fabricated example |

Public evidence remains source-attributed, not independently replicated. The synthetic pair exists only to exercise the interval arithmetic.

## 5. The real competitive benchmark we should build toward

The future primary metric is:

**Accepted solid volume per full-cycle elapsed hour**, with a frozen and independently audited boundary that includes cutting, capture/cleaning, regripping/repositioning, scheduled and unscheduled downtime, consumables and maintenance.

It must be accompanied by:
- total-boundary kWh per accepted solid m³;
- cutter and transport wear per accepted volume;
- spatial residual and unrecovered mass;
- sustained throughput and event availability;
- product/fragmentation acceptance;
- operating cost per accepted solid m³;
- transfer performance across rock cohorts and installed configurations.

The comparison protocol must match task, geology, geometry, machine class, acceptance criteria and full-cycle accounting. We should publish all component metrics and uncertainty, not one weighted composite.

## 6. Target gates

- 100% rejection of planted unit/denominator/definition mismatch controls.
- 100% of pairs with missing required context remain unranked.
- Correct conservative interval ordering on golden matched pairs.
- No secondary source about one trial counted as independent confirmation.
- No composite rank produced from heterogeneous metric families.

These are software conformance gates, not evidence that a Luminous machine outperforms the industry.

## 7. Integration

The comparator consumes source-attributed measurements and claims from #3864/#3876, #3814, #3825, #3853, #3890, #3910, #3913 and #3927. It does not replace any measurement authority or select machine operating conditions.

## 8. Non-claims

This benchmark does not certify manufacturer claims, prove product performance, predict economics or authorize operations. A COMPARABLE result means only that the declared comparison definitions and context pass the gate. Prospective independently measured matched trials are still required for any superiority claim.
