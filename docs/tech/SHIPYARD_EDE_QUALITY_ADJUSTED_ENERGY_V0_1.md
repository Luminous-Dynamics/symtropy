# Tandem-GMAW EDE ledger v0.1

Tracking: [SHIPYARD-EDE-QUALITY-ADJUSTED-ENERGY-001 #5073](https://github.com/Luminous-Dynamics/symtropy/issues/5073)

## Source and industry reference

Chaurasia et al. (2026), *Tandem gas metal arc welding of thick steel plates: Progress in productivity and properties using advanced process modes*, DOI [10.1177/13621718261472548](https://doi.org/10.1177/13621718261472548), describes an electrical deposition efficiency (EDE) metric for multi-pass T-GMAW of 30 mm structural plate. It reports a specific study envelope of about 674 g/kWh (PMC/PMC) to 1,051 g/kWh (CMT/CMT), with about a 25% mean improvement for the mixed PMC-leading/CMT-trailing mode against its PMC/PMC comparison. It also reports per-pass WFR/arc-power/energy relationships, t8/5, laser-profiled inter-layer cross-sections and 180–220 HV hardness in its five tested conditions.

Those are study-specific reference values, not a universal SOTA range and not our measured results. The article's data-availability statement says raw data can be provided on request. Separately, the Zenodo v2.0.0 record for the BAM raw archive still renders a blank licence field and “Copyright (C) 2026 The Authors.” This crate uses public article-level methodology and synthetic input only. It does not download, redistribute or score raw archive files.

## Formula and discrete integration policy

For the leading and trailing wires separately:

`E_j = integral(V_j(t) I_j(t) dt)` in joules

`L_j = integral(v_wj(t) dt)` in metres

For density `rho` in kg/m³ and wire diameter `d` in metres:

`A = pi*d^2/4`

`m_j[g] = rho * A * L_j * 1000`

`EDE[g/kWh] = (m_L + m_T) / ((E_L + E_T) / 3,600,000)`

V0 uses trapezoidal integration of sampled instantaneous power products `V_i*I_i`, and trapezoidal integration of each wire's feed speed. Each of the four streams keeps its own native sample cadence. The four streams must span the exact same declared window, have strictly increasing integer-nanosecond timestamps and obey the maximum sample-gap policy. No implicit gap bridging, endpoint extrapolation, missing-to-zero, mean(V)*mean(I) substitution or universal-rate resampling is permitted.

The calculator requires a non-zero source trace SHA-256 and a non-zero alignment receipt SHA-256. Synthetic tests use symbolic digests only. A future physical adapter must bind actual source bytes and a separately qualified event-time alignment receipt.

## Conservative uncertainty

V0 uses a worst-case multiplicative interval. The relative mass bound is the conservative sum of density + twice the diameter bound + wire-feed + feed-quadrature + clock-alignment uncertainty. The relative energy bound is the conservative sum of voltage + current + power-quadrature + clock-alignment uncertainty.

`EDE_low = EDE_nominal*(1-u_mass)/(1+u_energy)`

`EDE_high = EDE_nominal*(1+u_mass)/(1-u_energy)`

The contract rejects either combined bound at or above 100%. These are bounded intervals, not probabilistic confidence intervals.

## Synthetic known answer

The committed fixture uses 2 seconds of two-wire signals with distinct leading/trailing voltage, current and feed series:
- Leading arc energy: 12,720 J.
- Trailing arc energy: 7,760 J.
- Total arc energy: 20,480 J.
- Estimated leading/trailing filler masses: 3.5513 g / 2.1308 g.
- Total estimated filler mass: 5.6820 g.
- Nominal EDE: 998.7908 g/kWh.
- Conservative interval: 967.5481–1,031.5046 g/kWh.
- Arc energy per weld length: 0.0682667 kJ/mm.

These are synthetic symbolic known answers, not BAM pass data or industrial performance claims. Rust unit tests and an independent JavaScript replay use the same declared fixture values.

## Why EDE alone is not the optimization objective

EDE is an electrical-input-to-estimated-filler-wire-mass proxy. It does not directly weigh retained deposited metal, measure spatter/other deposition losses, prove fusion or penetration, or establish NDT acceptance. Do not call it actual deposition efficiency.

The field-level comparison must preserve separate objectives and gates:
- EDE (g/kWh);
- arc energy per weld length (kJ/mm);
- fill/geometry and scan uncertainty;
- t8/5, preheat/interpass temperature and hardness;
- independent macrosection/NDT disposition and acceptance basis;
- rework/reinspection energy and time;
- kWh and consumables per independently accepted metre/joint;
- full-cell cycle time including fit-up, scans, cooling, cleaning and inspection.

A high-EDE option that violates geometry, fusion, thermal, procedure or quality requirements is infeasible. Do not disguise hard quality constraints inside a weighted sustainability score.

## Tests and claim ceiling

The Rust suite covers sampled V*I rather than product-of-means, independent leading/trailing streams, irregular native cadence, source/alignment digests, exact window endpoints, timestamp ordering, maximum gaps, missing/non-finite/negative values, density/diameter unit mistakes, zero energy/feed, uncertainty bounds and identity. The crate is dependency-free and has its own nested Cargo workspace/lockfile.

A PASS proves the declared synthetic calculator and interval rules only. It does not prove physical source calibration, raw BAM access rights, measured EDE on a BAM pass, retained filler mass, weld quality, NDT acceptance, procedure qualification, reduced carbon footprint or improved shipyard productivity.
