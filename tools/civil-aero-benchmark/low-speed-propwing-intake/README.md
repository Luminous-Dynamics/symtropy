# AER-01 low-speed propeller / wing reference intake v0.1

**Tracking:** [#4280 — low-Re propeller maps and installed wing-interaction benchmarks](https://github.com/Luminous-Dynamics/symtropy/issues/4280)  
**Parent performance owner:** [#4055 — STOL / hot-high / blown-lift performance](https://github.com/Luminous-Dynamics/symtropy/issues/4055)  
**Composes:** #4111 reference admission, #3495 applicability, #4112 spatial transfer, #4236 observation operators, #4093 aeroelastic bridge and #3237 solver interoperability.

## Scope and claim ceiling

This is a source-discovery catalog plus an independent fail-closed metadata validator. It is **not** an admitted physical reference corpus: all six entries remain unacquired or preview-only, there are no exact-byte digests and terms/redistribution remain under review.

It deliberately distinguishes:
- isolated propeller component data;
- installed low-Re propeller/wing tests;
- full-scale propeller-stand tests;
- installed high-lift propeller geometry-sensitivity studies.

An isolated propeller performance map does not validate the installed wing interaction. A component-stand test does not establish an aircraft lift distribution. No item validates Wren 2 or Wayfarer 4.

## Key source-backed findings

### UIUC propeller databases — low-Re component-map baseline

The UIUC Propeller Database landing page describes wind-tunnel data for small UAV/model-aircraft propellers and four volumes. Volume 1 lists nearly 140 propellers; Volume 2 nearly 70. Both index thrust and torque coefficient versus advance ratio at specified RPM, static RPM sweeps and approximate digitized geometry for some entries.

Two non-obvious source semantics must be preserved:
- Volume 2 coefficients use the true measured propeller diameter, which may differ from the nominal label.
- Volume 1 online records include an updated fixture-drag correction and the UIUC page recommends the current online records over old thesis plots/tables.

The UIUC database is a **component performance** reference only. It contains no universal installed wing correction. APC geometry is separately sourced; inspect the latest geometry package and reuse terms before ingesting it.

### Low-Re propeller-induced MAV wing flow

Durai (2014) publisher abstract reports a low-speed wind-tunnel test at 9 m/s, root-chord Reynolds number 135,000 and 8,000–10,000 rpm. It compared an attached motor/propeller arrangement with a decoupled arrangement to separate propeller thrust from the aircraft-model force measurement. The abstract reports increased lift at higher angle of attack and increased drag under propeller-induced flow, with stronger effects at higher RPM.

The abstract is not the raw dataset. Exact geometry, AoA matrix, uncertainty, calibration, full tables and data reuse remain to be inspected.

### NASA X-57 high-lift propeller stand tests

The NASA 2021 full-scale high-lift propeller test paper describes 12 X-57 high-lift propellers upstream of the wing leading edge and a NASA Langley Low Speed Aeroacoustic Wind Tunnel campaign. Its test matrix includes freestream speeds of 0, 35, 58, 75 and 90 KEAS, with multiple RPM and power/torque settings. It reports 439 time-averaged data points across more than 15 hours of powered operation.

The paper documents a particularly important measurement issue:
- the thrust channel suffered significant force cross-loading and thermal drift;
- the torque channel remained much more reliable;
- therefore thrust and torque must not be given identical uncertainty or promoted as equally precise.

This is a component-stand source, not integrated installed-wing validation.

### NASA high-lift propeller installation geometry

NASA's X-57 technical-paper index describes a wind-tunnel experiment that varied propeller height, offset and inclination. The abstract says height was the most critical variable and the best height depended on angle of attack and flap deflection, with a relationship to unblown wing lift coefficient.

The abstract does not supply the numerical data needed for reproduction; the exact paper, geometry and dataset must be admitted before using it as a numerical benchmark.

## Files

- `reference-catalog-v0.1.json` — six source entries, four distinct benchmark tracks and global claim constraints.
- `verify_reference_catalog.py` — independent standard-library catalog validator and six hostile mutations.

Run:

~~~sh
python3 tools/civil-aero-benchmark/low-speed-propwing-intake/verify_reference_catalog.py
~~~

The validator proves catalog consistency only. It does not download, hash or admit the source data.

## Benchmark order

1. Admit selected UIUC component files and propeller geometry; independently recompute CT/CQ/CP using the exact source definitions. Validate static and wind-on records separately.
2. Admit the low-Re MAV paper/data if available and terms permit; reproduce one attached/decoupled propeller-induced wing point.
3. Admit NASA X-57 high-lift propeller test and retain channel-specific thrust/torque metrology, thermal drift and uncertainty.
4. Admit NASA installation-geometry reference; compare propeller height/offset/inclination across supported AoA/flap settings.
5. Extend to multiple-propeller interference only after a source provides sufficient geometry, operating conditions and measured outputs.
6. Feed only the qualified low-speed model domain into #4055; use out-of-domain abstention rather than extrapolating a component map into a distributed blown-lift claim.

## Paradigm shifts this work enables

- Optimize the installed aero-propulsive state, not a propeller coefficient in isolation.
- Treat propeller/wing installation geometry as a first-class discrete/continuous design space.
- Co-design measurement systems and experiments: load-cell cross-talk, thermal drift, bandwidth and synchronization are part of the observation operator.
- Select wind-tunnel/CFD tests by expected change in aircraft architecture decision, not simply the highest lift or largest model residual.
- Optimize robust mission service, including cruise drag, mass, thermal, failure, landing/go-around and maintenance penalties, rather than peak blown-lift alone.

## Claim ceiling

Public source discovery and partial paper inspection are not dataset admission. No exact bytes or hashes are currently acquired. UIUC data are isolated component maps. NASA X-57 material is an X-57 high-lift propeller stand test. The 2014 MAV paper is a low-Re method/reference candidate. None is candidate-specific physical validation, airworthiness, certification or permission to manufacture/test/fly.
