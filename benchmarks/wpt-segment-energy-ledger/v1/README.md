# WPT segment and route energy ledger v1

**Status:** research software with a synthetic known-truth route fixture. It is not a test report from a vehicle or an IEC compliance certificate.

## Purpose

Dynamic WPT systems are often compared using percentages that refer to different physical boundaries. This crate records and reports these as distinct, non-interchangeable metrics.

1. **Dynamic segment transfer:** supply-power-circuit (SPC) input energy to EV power-control-circuit (EVPC) output energy over the declared inverter-rise to inverter-fall segment window. This is aligned to the metric described in IEC PAS 61980-5:2024 section 7.3.
2. **Roadway power electronics boundary:** roadway inverter DC input to vehicle DC-bus output. INDOT states approximately 85% at rated power for its Purdue–INDOT testbed at this boundary. This is not grid-to-battery efficiency.
3. **Infrastructure-to-battery route service:** grid AC input, including declared idle/waiting energy, divided into battery-terminal input over the complete route pass. This is intentionally broader than the IEC segment metric.

The crate does not authenticate meter calibration or source artifact signatures. It verifies that content digests have the expected format and carries them into the input record.

## Event model

Each route is an event stream over integer nanoseconds:
- route identity, geometry digest, evidence class and power-flow direction;
- one or more energized segment records;
- exactly six ordered phase records per segment: inverter rise, pre-power, transfer ramp-up, active transfer, transfer ramp-down and inverter fall;
- independent whole-segment SPC input and EVPC output meter readings;
- explicit idle intervals for energy consumed outside active segment transfer;
- separate boundary readings for grid AC input, optional BESS DC output, roadway inverter DC input, vehicle DC-bus output and battery-terminal input.

Every phase has its own SPC/EVPC Wh values and deterministic absolute uncertainty bounds. The six phase sums must reconcile with independent whole-segment meter totals within the combined bounds. A route is rejected when segments/idle intervals overlap, leave an unaccounted time gap, or fail to cover the full route interval.

The measurement-plane identifier is checked against the expected plane. Each measurement has meter ID, measurement ID, calibration receipt digest and raw source artifact digest. Measurement IDs must be globally unique within a route to prevent accidental double-counting. Reports preserve route/segment geometry digests, evidence class, per-phase meter IDs, per-segment boundary measurements and a sorted provenance index for every meter reading, calibration receipt and source artifact. If BESS DC output is present in any route interval, it must be metered in every segment and idle interval (use an explicit zero where no BESS energy is supplied); partial BESS coverage is rejected.

## Metric definitions

### Standard-aligned segment transfer efficiency

For each active segment and for the route total:

\[
\eta_{SPC\to EVPC}=\frac{\sum E_{EVPC,out}}{\sum E_{SPC,in}}.
\]

The route score is a **ratio of summed energies**, not the arithmetic mean of per-segment percentages. The input/output uncertainty bounds are aggregated conservatively. If the denominator's lower uncertainty bound crosses zero, or the output's lower bound becomes negative, the metric is UNRESOLVED.

### Roadway inverter to vehicle DC bus

\[
\eta_{INV\to VDC}=\frac{\sum E_{vehicle\ DC,out}}{\sum E_{roadway\ inverter\ DC,in}}.
\]

Only active segment transfer windows enter this metric. Idle inverter draw is reported separately so waiting/standby power does not silently degrade or contaminate the matched rated-power transfer metric.

### Direct grid-AC input / battery-terminal input energy ratio

\[
r_{grid\,AC/battery}=\frac{\sum E_{battery\ terminal,in}}
{\sum E_{grid\ AC,in}+\sum E_{idle\ grid,in}}.
\]

This is a **direct quotient of two measured energy totals**, not a conversion-efficiency claim. It spans a wider route boundary than the IEC SPC-to-EVPC segment metric, and it does not prove that all battery input originated at the grid meter. BESS DC output and its uncertainty are reported separately. When BESS contributes energy, this direct quotient is not a complete upstream-source-to-battery efficiency; v1 does not calculate a BESS-to-battery efficiency without a separately metered downstream path.

## Uncertainty semantics

All measurements use deterministic absolute energy bounds in Wh. For a ratio E_out/E_in, v1 computes:

\[
\eta_{low}=\frac{E_o-\epsilon_o}{E_i+\epsilon_i},\qquad
\eta_{high}=\frac{E_o+\epsilon_o}{E_i-\epsilon_i}.
\]

The result is UNRESOLVED when the input lower bound is non-positive or the output lower bound is negative. It does not clamp values or invent uncertainty bounds.

## Synthetic known truth

The synthetic-highway-pass.json fixture contains two synthetic six-phase segments and one idle interval. Its expected ratios are:
- SPC to EVPC: 108 / 120 = 90%;
- roadway inverter DC to vehicle DC bus: 130 / 140 ≈ 92.86%;
- grid AC to battery terminal across the route: 96 / 185 ≈ 51.89%.

The route contains 5 Wh of idle grid input and 20 Wh of idle roadway-inverter draw. The latter is separately reported with its uncertainty and is intentionally not included in the active inverter-to-vehicle transfer denominator. The fixture also includes BESS DC output values to demonstrate that this is another measurement boundary, not an alias for grid energy. The CLI report carries the BESS energy and summed uncertainty separately, alongside the full meter-provenance index.

These values are synthetic regression data, not Purdue, Electreon, IEC or vehicle test results.

## CLI and tests

    cargo test --locked -p wpt-segment-energy-ledger
    cargo run --locked -p wpt-segment-energy-ledger -- benchmarks/wpt-segment-energy-ledger/v1/synthetic-highway-pass.json

## Industry and standards comparators

- IEC PAS 61980-5:2024 section 7.3 defines dynamic-segment transfer efficiency on SPC input versus EVPC output over the inverter-rise to inverter-fall interval and describes the inverter-rise, pre-power, ramp-up, transfer, ramp-down and inverter-fall phases.
- INDOT's Purdue–INDOT dynamic WPT testbed reports approximately 85% roadway inverter DC bus to vehicle DC bus at rated power. It does not state a grid-to-battery ratio at that boundary.
- Route-service studies may publish WPT-supplied energy divided by propulsion energy consumed; that is not an electrical transfer efficiency.

This implementation keeps all three lanes distinct. See issue #3587 for the research plan and issue #3472 for the wider source-to-load accounting programme.

## Proposed targets (not achieved)

- zero silent gaps/overlaps in the route time stream;
- phase-to-independent-segment-total reconciliation residual inside declared metrology bounds;
- route aggregation always by ratio-of-sums;
- zero denominator-crossing false passes;
- all results carry calibration, meter-plane and source-artifact digests;
- deterministic JSON replay and under 10ms evaluation per 1,000 segments on a declared reference CPU.

## Claim ceiling

This crate performs event validation and energy-ratio arithmetic. It does not prove meter calibration authenticity, measurement-plane traceability, power-quality compliance, electromagnetic exposure, FOD performance, thermal durability, interoperability, vehicle-roadway safety, hardware reliability or standards compliance.
