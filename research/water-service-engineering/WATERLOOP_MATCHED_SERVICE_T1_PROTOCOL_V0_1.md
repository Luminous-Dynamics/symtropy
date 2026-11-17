# WaterLoop matched-service T1 protocol v0.1

**Tracking issue:** #5908 WATERLOOP-MATCHED-SERVICE-BENCH-001  
**Parent:** #5335 WaterLoop physical demonstrator  
**Accounting contract:** #5859 / draft PR #5862  
**Status:** design-only; no physical data acquired and no trial authorized.

## 1. Objective

Qualify whether recovered water supplies one declared end use and displaces mains water at the same service boundary. Gross treatment output, use-qualified output, delivered recovered water, actual fixture use and matched-control mains displacement are different quantities.

The first template uses a *non-potable toilet-flushing* profile only as a synthetic design example. It is not a quality approval or a plumbing design. The use-profile ID must match exactly between the declared use, quality evidence and measured run before any credit could be considered.

## 2. Trial design: randomized, counterbalanced paired crossover

For each frozen demand block, execute:
- Control arm: mains-only supply to the declared fixture.
- Treatment arm: recovered supply only when an independent, current quality release applies to the exact declared use profile.

Randomize/counterbalance arm order. Hold the exogenous fixture demand trace, pressure/flow envelope, initial storage, temperature band and run duration comparable. Record interruptions, fallback supply and missing intervals; do not silently discard them.

The initial template proposes at least six paired blocks as a planning hypothesis only. It is not a sample-size/power calculation. Run duration, stopping rule and uncertainty target remain pending until metrology and demand profiles are frozen.

## 3. Measurement boundary

All quantities bind to instrument ID, calibration reference, method, unit, physical boundary, timebase and uncertainty. Minimum channels in the plan:
- mains inlet and recovered-source inlet;
- treatment feed and gross product;
- recovered delivery and mains delivery at the same fixture boundary;
- reject/concentrate, purge/backwash and overflow/drain;
- fixture pressure and flow;
- whole treatment/pump train electrical energy.

Tank level is not volume without a separately validated geometry/calibration relation. Gross product volume is not end-use service. A bypass or mains fallback interval is not recovered mains displacement.

Where feasible, use an independent gravimetric or reference-volume check for critical flow/volume instruments. Do not force water-balance closure by inventing an unmeasured sink.

## 4. Use-quality qualification

The evidence package must bind the exact use profile, source-water profile, analyte panel, sample IDs, custody, laboratory, method revision, detection/quantitation limits, uncertainty, instrument calibration and release decision.

A quality result for toilet flushing cannot qualify irrigation or potable use. Missing, stale, contradictory, out-of-scope, out-of-envelope or custody-invalid critical evidence => no release and no service credit.

The initial plan contains pending placeholders and therefore cannot authorize operation or credit.

## 5. Service-credit estimator

For each matched block and one exact use profile, the provisional upper bound is:

`credit_l <= min(quality_qualified_recovered_delivery_l, metered_recovered_fixture_use_l, matched_control_mains_volume_displaced_l)`

Also require distinct, correctly bound meter IDs; non-overlapping windows; comparable demand; same service boundary; time alignment; and propagated instrument uncertainty.

No credit for gross treatment output, unused stored water, reject, purge, backwash, overflow, bypass or fallback mains. If the control is absent or not comparable, displacement is indeterminate and credit is zero.

Report paired block differences and their predeclared uncertainty interval. Do not report a single number without paired-run count, missing-run accounting and uncertainty.

## 6. Positive / negative controls

Positive controls:
- mains-only reference reproduces the frozen fixture demand within its uncertainty;
- recovered-source meter agrees with the independent reference check;
- seeded bypass/reject is measured and excluded from fixture credit.

Negative controls:
- high gross output with zero fixture use => zero credit;
- toilet-flushing quality evidence applied to irrigation => reject;
- stale/missing analyte or custody => no release;
- missing/unmatched mains control => indeterminate / zero credit;
- duplicate/swapped/time-shifted meters => reject;
- shared source credited twice => reject;
- reject/backwash/overflow counted as storage or delivery => reject balance;
- below-envelope pressure/flow => no credit for affected interval;
- invalid timebase or missing power/flow => indeterminate;
- recovered stream never reaches the fixture => zero credit.

## 7. Benchmarks and proposed stretch target

Report: qualified litres delivered, recovered litres actually used, net mains litres displaced, kWh/m³ qualified delivery, kWh/m³ net displacement, residual fractions, pressure/flow violations, treatment differential pressure, consumables, maintenance/operator time, quality disposition, paired uncertainty and lifecycle cost per 1,000 L of profile-conforming net displacement.

Research hypothesis (not achieved or an industry standard): at least 20% lower lifecycle cost per 1,000 L of independently verified use-qualified net displacement in one matched profile without quality, service, energy, residual-handling or maintenance regression.

Compare at the same declared function and service envelope. Keep storage-only, correctly selected commercial pump/storage, recovery without treatment, justified treatment and Luminous candidate separate. Do not compare product functions outside their precise certified/intended scope.

## 8. Safety hold points and nonclaims

The initial intent is an unoccupied, clean-water recirculating cell only, subject to competent facility and engineering review. This protocol is not a P&ID, pressure-system or electrical design. Air-gap/backflow, pressure ratings/relief, containment/drainage, wet-side electrical isolation, manual isolation and shutdown require independent review before any assembly or energization.

No biological challenge, chemical dosing, potable connection, public-network interconnection, occupied use, automatic actuation or procurement SKU freeze is authorized. No savings, treatment effectiveness, potability, SANS 241 compliance, product superiority, regulatory approval or physical deployment is claimed.