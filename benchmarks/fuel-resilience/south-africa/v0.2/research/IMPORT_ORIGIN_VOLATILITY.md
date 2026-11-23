# Import-origin variability — source-qualified research note

**Research cut:** 23 November 2026  
**Purpose:** show why supplier concentration must be time-varying and why non-equivalent source series must not be silently concatenated.

## Observation A — official Q2 2026 trade report

The South African Energy Trade Report for Q2 2026 reports diesel imports of approximately **3.3 billion litres** and the following origin mix:

| Origin | Reported share | Reported litres |
|---|---:|---:|
| India | 27% | 891 million L |
| United States | 21% | 712 million L |
| Oman | 15% | 481 million L |
| Togo | 9% | 311 million L |
| Netherlands | 5% | 174 million L |

The top three contribute 63% and the top five 77% of the report's Q2 origin mix. The source is based on SARS customs data as reproduced in the published trade report.

**Internal reconciliation warning:** the five country-level volumes printed in the report sum to **2.569 billion litres**. The five printed shares sum to **77%**; 77% of the report's rounded 3.3-billion-litre diesel total is **2.541 billion litres**, a **28-million-litre difference**. This may reflect rounding, aggregation or source-table inconsistency; the publication does not resolve it. The table therefore stores the source-reported shares and source-reported litres as separate observations. Do not recompute one field from the other or treat this as a calibrated probability distribution.

## Observation B — secondary monthly dashboard, September 2026

Econo Energy Tools' public customs dashboard (updated 3 November 2026) reports approximately **1.048 billion litres** of diesel imports in September 2026, with a different origin mix:

| Origin | Reported share | Reported litres |
|---|---:|---:|
| India | 27.7% | 290.1 million L |
| Oman | 19.4% | 203.3 million L |
| China | 15.1% | 158.1 million L |
| Malaysia | 14.5% | 151.7 million L |
| Singapore | 10.4% | 108.8 million L |

The top three sum to 62.2%, while the top five sum to 87.1% of that dashboard's September observation.

The dashboard identifies its underlying source as SARS Merchandise Trade Statistics under tariff heading 27.10, Additional Note 1(g), and warns that SARS may revise historical data. Its customs-district breakdown is a statistical entry point and does **not** necessarily identify the physical port of discharge. Do not use customs-district shares as a physical berth, terminal or pipeline routing map.

## What we can and cannot infer

These two observations suggest that supplier origin is not a safe static parameter. They do **not** constitute a statistically valid time-series comparison because:
- one is a published Q2 report and the other a third-party monthly dashboard;
- exact classification, customs-district treatment and data processing may differ;
- quarter and month are different aggregation windows;
- the secondary dashboard's method and revisions policy need independent review.

Do not combine, interpolate or extrapolate these observations as one series. Do not use either to claim a fixed annual dependency.

## Required SAFRB data model

A supplier-origin observation should carry:
- source identifier and publisher;
- source publication/version and raw data digest;
- observation start/end time and aggregation period;
- country-of-origin semantics versus shipment/dispatch/flag state;
- product grade and customs commodity code;
- gross imports, exports and net flow kept separate;
- declared volume, mass/density conversion if present, unit and uncertainty;
- revision status and reconciliation state;
- source quality tier and confidence.

The benchmark should model country/supplier exposure as a **time-varying, evidence-weighted concentration**. Preserve top-N shares, residual share and a lower-bound HHI when the full distribution is unavailable. A partial HHI must never be described as the full index.

## Research directions

1. Compare monthly, quarterly and annual origin distributions only after semantic and unit reconciliation.
2. Use rolling HHI, top-three/top-five share, supplier count, Herfindahl effective supplier count and route/port concentration.
3. Model common-cause concentration: countries sharing export terminals, refinery regions, shipping lanes, insurers or geopolitical chokepoints.
4. Use Bayesian or interval estimates when origin data are incomplete; do not fill unknown shares with zero.
5. Stress supplier disruption using multiple plausible origin snapshots and correlated route failures.
6. Report robustness to source selection and revisions.

## Sources

- [South African Energy Trade Report — Quarter 2, 2026](https://fuelsindustry.org.za/wp-content/uploads/2026/10/SOUTH-AFRICAN-ENERGY-TRADE-REPORT-QUARTER-2-2026.pdf) — published customs-derived Q2 origin mix.
- [Econo Energy Tools — Fuel Imports](https://tools.econoenergy.co.za/fuel-imports) — secondary dashboard, September 2026 monthly observation, updated 6 November 2026.

**Claim ceiling:** source comparison and research hypothesis only. This note does not certify either dataset, establish current supplier concentration, or predict future import availability.
