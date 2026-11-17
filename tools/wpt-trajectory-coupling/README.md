# WPT synchronized trajectory-coupling contract v0

Tracks [WPT trajectory coupling #3905](https://github.com/Luminous-Dynamics/symtropy/issues/3905). It creates one common coordinate and time record for receiver alignment, wheel paths, pass-level electrical metering and thermal observations.

## Paradigm shift: one trajectory, several evidence channels

The same vehicle trajectory must drive the geometric inputs to both the electromagnetic and pavement analyses. Do not give the electromagnetic model one lateral-offset summary and the civil model a separately averaged wheel-path trace.

For each ordered sample, the evaluator derives:

- receiver longitudinal coordinate = vehicle longitudinal coordinate + receiver mount offset;
- receiver lateral coordinate = vehicle centre lateral coordinate + receiver mount offset;
- receiver-to-coil offsets in the declared common frame;
- receiver-in-active-zone flag using the declared coil longitudinal zone;
- every wheel's lateral path relative to the embedded coil;
- total vertical wheel load for that sample.

No alignment, wheel path or time series is silently sorted, shifted or imputed.

## Meter integration

The two power channels are deliberately named by their measurement planes:

- `roadway_dc_input_power_w`: roadway converter DC input;
- `vehicle_receiver_dc_output_power_w`: vehicle receiver DC output.

Energy is integrated with a trapezoidal rule over strictly increasing sample times and quantized to micro-Wh. The reported conversion ratio is only produced when total receiver-side DC output does not exceed total roadway-side DC input and the input integral is non-zero.

This is **not** grid-to-battery efficiency. It is not battery-terminal energy. Any DC-link storage, auxiliary load, meter-clock offset or unsynchronized plane must be separately accounted for before applying the ratio to measured field traces.

## Mechanical and thermal descriptors

The evaluator reports:
- minimum lateral distance between any declared wheel path and the coil centre;
- integrated total vertical wheel-load impulse;
- maximum mismatch between odometry-derived interval speed and the mean of endpoint speed samples, checked against an explicit protocol tolerance;
- maximum recorded coil and pavement-interface temperatures;
- counts of missing thermal samples.

Wheel-load impulse is an exposure descriptor only. It is not pavement stress, strain, fatigue, rutting, damage, remaining life or an allowable-load calculation. Temperatures are maxima of supplied observations, not thermal-dose or material-response predictions. Null observations remain explicit and are never interpolated.

## Known-truth fixture

The synthetic three-sample pass declares:
- receiver lateral offsets of 20, 120 and 220 mm;
- one of three samples inside the declared longitudinal active zone;
- two of three samples inside the declared 150 mm alignment envelope (666,667 ppm);
- minimum wheel-path distance 600 mm;
- roadway DC input 27.777778 Wh and receiver DC output 23.611111 Wh;
- pass-level DC conversion ratio 850,000 ppm;
- vertical wheel-load impulse 84 kN·s;
- maximum coil/interface temperatures 45 °C / 35 °C.

The fixtures are hand-computable software controls, not measurements or performance claims.

## Exactness and limits

- Synthetic known-truth evidence only.
- 2–1,024 ordered samples and 2–16 wheel channels.
- Strict sample index, monotonically increasing time and forward progress.
- Speed/odometry mismatch is checked against an explicit protocol tolerance.
- Declared maximum sample gap; gaps above it fail closed.
- Exact wheel identity coverage at every sample.
- Signed integer millimetre coordinates and explicit longitudinal/lateral axis directions.
- Row-order and coordinate-origin invariance tests.
- Unknown fields, duplicate identifiers and non-synthetic evidence are rejected.

## Validation

`cargo test --locked -p wpt-trajectory-coupling`

Twelve tests cover known-truth outputs, coordinate translation, sample order/progress, speed/odometry agreement, single-sample clock skew, lateral-axis inversion, meter swaps, sample gaps, wheel-channel completeness, ambiguous energy balance, explicit missing thermal channels and fail-closed identifiers/fields/evidence.

## Claim ceiling

This package proves only deterministic coordinate transforms, sample-contract integrity and trapezoidal integration for synthetic traces. It does not calculate electromagnetic mutual inductance/coupling, power transfer from geometry, pavement stress/strain/fatigue, heat transfer, battery-side energy, human exposure, safety, standards compliance or field performance.

## Physical comparators and measurement gaps

- **Purdue–INDOT 2026:** 85 roadway coils embedded approximately 2.4 in below a rigid-pavement surface; a retrofitted Class-8 truck received 50–190 kW at speeds up to 65 mph, with efficiency approaching 85%; the programme reports strong misalignment tolerance without active road/vehicle alignment. This is a physical roadway demonstration, not a published joint trajectory/wheel-load/thermal trace in this v0. [USDOT ROSA P report FHWA/IN/JTRP-2026/18](https://rosap.ntl.bts.gov/view/dot/93168), [DOI](https://doi.org/10.5703/1288284318633).
- **Honda / Taisei / Taisei Rotec, 5 October 2026:** disclosed a DC-distributed road architecture with an integrated inverter and coil per embedded ground assembly. Its one-million-wheel-load / 49 kN durability campaign, tests up to 150 kW and road demonstration from Japanese FY2027 onward are announced future activities, not completed results. [Honda](https://global.honda/en/topics/2026/c_2026-10-05beng.html).
- **Asphalt thermo-mechanics, August 2026:** a binder-grade study reports the modelled response to a 10 °C surface rise differed materially between PG 64–22 and PG 76–22 asphalt; distributed fibre-optic observations located strain peaks in hot zones above pads. These are study-specific findings, not universal material specifications. [DOI 10.1016/j.applthermaleng.2026.132099](https://doi.org/10.1016/j.applthermaleng.2026.132099).

The public comparators do not yet supply a common frozen record with receiver pose, coil-relative alignment, wheel loads, interface strain/temperature and all electrical meter planes synchronized to one clock. That missing linked trace is the research gap this contract addresses. A future measured benchmark must report transfer power, conversion boundary, receiver DC energy, battery-terminal energy, alignment tolerance, wheel-path load, temperature and pavement response as separate scoreboards.

