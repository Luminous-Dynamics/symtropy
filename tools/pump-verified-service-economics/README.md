# Pump Cost per Verified Service — Stage 0

**Owner:** Symtropy [PUMP-COST-PER-VERIFIED-SERVICE-001 #6023](https://github.com/Luminous-Dynamics/symtropy/issues/6023)  
**Composes:** pump energy service #5103, transferability #6008, released-output economics #5966, service floors #5027 and restoration #5069.  
**Status:** dependency-free Rust synthetic composition evaluator; no physical pump or tariff claim.

## Paradigm

Do not compare only:
- cost per manufactured/released pump;
- kWh per total volume pumped;
- total volume regardless of pressure/flow service;
- system average pressure when a specific service area fails.

Compare **lifecycle cost and electrical energy per m³ delivered only in study windows where every required service area is independently verified as meeting its declared service floors**.

A manufactured pump must first be transferable to its declared installed duty. Cost and energy windows must match the service study horizon. A breached, unknown, stale or incomplete service area cannot be hidden by another area's surplus.

The output is an interval and categorical disposition, never a single point estimate or weighted KPI.

## Frozen Stage-0 corpus

- 33 synthetic cases.
- 65 service-area records, with explicit two-area population and one missing-area negative control.
- 33 independently authored expected dispositions.
- 28 authored Rust unit tests.
- 37-column deterministic evidence-carrying TSV report.
- Pinned Rust 1.96.0 format/Clippy/test/report workflow and 30-day evidence bundle.

Expected dispositions:

| Disposition | Cases |
|---|---:|
| QUALIFIED_COST_PER_SERVICE | 2 |
| NOT_TRANSFERABLE | 1 |
| IDENTITY_MISMATCH | 1 |
| SERVICE_FLOOR_BREACHED | 2 |
| SERVICE_FLOOR_BREACHED_WITH_UNKNOWN | 1 |
| INDETERMINATE | 3 |
| EVIDENCE_INCOMPLETE | 12 |
| PROFILE_INCOMPLETE | 6 |
| ZERO_QUALIFIED_SERVICE | 1 |
| UNSUPPORTED_CLAIM | 1 |
| INVALID | 3 |

## Interval calculations

Inputs:
- (C=[C_L,C_U]): total lifecycle cost already allocated by the #5966 owner, in declared currency minor units.
- (E=[E_L,E_U]): measured electrical energy over the matched study window, in Wh.
- (V_q=[V_L,V_U]): sum of delivered volume intervals only for required areas with MET service-floor evidence, in mL.

The evaluator does not calculate cost items, capital amortization, tariffs, FX, discounting, inflation or service floors. It consumes owner-authorized, source-bound values.

Output scales:
- `cost_per_m3_milli_minor`: thousandths of the declared minor currency unit per m³.
- `kwh_per_m3_milli`: thousandths of kWh/m³.

Conservative bounds:

`cost = [floor(C_L × 10^9 / V_U), ceil(C_U × 10^9 / V_L)]`

`energy = [floor(E_L × 10^6 / V_U), ceil(E_U × 10^6 / V_L)]`

The upper bounds require (V_L>0). A zero or uncertain lower volume never becomes a finite optimistic ratio.

## Non-compensatory rules

- A non-transferable pump cannot receive a service-cost score.
- Transfer serial/configuration must match the manufactured asset.
- Every required service area must be observed in the same study window and boundary.
- Any known floor breach blocks the competitive score; if another area is unknown, both states are retained.
- Unknown/stale/missing floor, volume, observation or calibration evidence cannot become MET.
- Missing transfer, cost, energy or allocation evidence fails closed.
- Cost period, energy window and study horizon must match exactly.
- Currency and amount-unit prefix must match (e.g. ZAR + ZAR_CENT); no FX is guessed.
- A hydraulic service record never establishes potability. A potable claim is UNSUPPORTED_CLAIM without the independent water-quality/barrier owners.
- Zero qualified service returns ZERO_QUALIFIED_SERVICE, never divide-by-zero.
- No weighted average or aggregate mean can compensate for a failed area.

## Run

```sh
cargo fmt --manifest-path tools/pump-verified-service-economics/Cargo.toml -- --check
cargo clippy --locked --manifest-path tools/pump-verified-service-economics/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path tools/pump-verified-service-economics/Cargo.toml --all-targets
cargo run --locked --manifest-path tools/pump-verified-service-economics/Cargo.toml -- --report
```

## Claim ceiling

The evaluator qualifies only synthetic evidence composition, exact joins and interval arithmetic. It does not establish actual lifecycle cost, tariff, energy saving, delivered water service, uptime, water quality, pump reliability, legal compliance, installation safety or product release.
