# EHD-DPM-002 — randomized drift-balanced diagnostic counterfactuals

**Status:** standalone Rust reference authored; 13 unit tests authored; local Rust execution pending; exact-head hosted CI pending.  
**Branch:** research/ehd-dpm-counterfactual-v0-1  
**Parent:** EHD-DPM-001 observer-perturbation receipt

## Research trigger

Zwolińska-Glądys, Cardona Murillo and Jaszczur (2026), *Experiments in Fluids* 67, 168, DOI 10.1007/s00348-026-04322-y, directly demonstrate that EHD PIV seeding is an intervention on the system, not merely passive measurement noise. For a 14 kV needle-to-ring ionic fan, the paper reports DEHS versus incense-smoke peak axial velocity of 3.8 versus 5.9 m/s (36% lower under DEHS) and volumetric flow of approximately 0.89 versus 1.07 L/s (about 17% lower). The paper attributes the difference to ionization/effective-conductivity suppression and dielectric collector deposition.

The paper also reports 15-minute current-stability tests at 8 kV and describes the sequential test conditions as clean air, DEHS, and incense smoke. It observed current decay under DEHS deposition and faster visible incense deposition on the collector. Those are important physical findings, but they also motivate a methodological question: how much of an apparent between-condition difference is attributable to the intervention, how much to temporal drift, and how much to persistent surface-state carryover? The source reports generated data as available from the corresponding author on reasonable request, so this programme has not independently reprocessed its raw time series or images.

## Paradigm shift: from before/after comparison to randomized intervention identification

EHD-DPM-001 compares a baseline, diagnostic-on observation, and optional post-removal observation. That is a useful conservative receipt, but a simple sequence can confound treatment with time, environmental drift, electrode conditioning, and irreversible deposits.

EHD-DPM-002 introduces a pre-registered randomized switchback:

- **ABBA:** sham/control → diagnostic → diagnostic → sham/control.
- **BAAB:** diagnostic → sham/control → sham/control → diagnostic.

For symmetric period timing, both sequences give the control and intervention groups the same mean observation time. The paired contrast therefore cancels a first-order linear drift within a block. Balanced assignment of ABBA and BAAB over independent clusters also counterbalances a common quadratic drift term in the campaign mean; it does not cancel arbitrary non-stationarity.

Every period requires its own:
- outcome evidence identity;
- verified collector/electrode surface-reset receipt;
- verified chamber purge / tracer-clearance receipt;
- matched operating-condition identity.

These controls are intentionally strict. A clean-air/control period following a deposited tracer is not a clean baseline unless the surface state and residual aerosol state have been restored and documented. A failed reset or purge prevents passivity qualification.

## Statistical estimand and uncertainty

For block b, with two diagnostic and two control periods:

`D_b = mean(Y | diagnostic) - mean(Y | control)`

The block-level conservative absolute measurement bound is:

`U_b = 0.5 * sum(U_period_1, ..., U_period_4)`

Blocks are first averaged within the declared highest-level independent acquisition/common-cause cluster. The campaign mean gives each independent cluster equal weight; many repeated blocks in one laboratory session do not create independent replications.

A cluster-level sampling standard error and approximate 95% Student-t margin are computed across independent cluster means. The fixed critical value 2.093 is the df=19 two-sided 95% value, used conservatively for n≥20 under an approximately normal cluster-mean assumption. The measurement absolute bound is then added to the sampling margin. This is an explicit conservative reference rule, not a distribution-free coverage theorem.

The diagnostic is:
- **Detected** when the complete uncertainty interval lies outside the allowed absolute effect;
- **WithinTolerance** only when the complete uncertainty interval lies inside the predeclared equivalence tolerance;
- **Unresolved** when the interval overlaps the tolerance boundary.

Failure of physical ledgers, validation envelope, randomization verification, sequence balance, surface reset, purge, or independent-cluster minimum takes precedence over a small point estimate.

## Prospective experiment targets

These are design targets, not measured results:

| Design element | Initial target |
|---|---:|
| Independent acquisition/common-cause clusters | ≥20 |
| Assignment | independently randomized and receipt-verified |
| Sequence balance | ABBA and BAAB counts differ by at most one cluster |
| Period schedule | symmetric time midpoints within declared tolerance |
| Reset/purge | independently receipted before every period |
| Reference controls | sham condition with same handling / elapsed time |
| Outcome streams | velocity profile, integrated flow, current, voltage, surface state |
| Repetition | repeat across voltage, humidity, tracer concentration and geometry |
| Effect tolerance | pre-registered per observable and engineering decision |

Twenty clusters are a minimum screening floor, not a universal sample-size recommendation. A real campaign must use a pre-run power analysis based on between-session variance and the smallest effect of engineering significance.

## How this complements the field

PIVPostProcess evaluates vector validity. Classical PIV uncertainty methods estimate displacement uncertainty from correlation statistics or image matching. PIVtools (Taylor, Lawson and Ganapathisubramani, 2026) provides a comprehensive open-source end-to-end PIV platform, including planar/stereo interrogation, ensemble correlation, direct Reynolds-stress extraction, GUI/CLI workflows, and hybrid Python/C+Dask scaling; the paper reports 77% parallel efficiency at 128 cores. EHD-DPM-002 does not attempt to replace those processing systems or claim their throughput. It qualifies whether the measurement intervention changes the physical system before any image-processing result is accepted as passive-flow evidence.

The larger opportunity is a diagnostic-specific causal record: every tracer, probe, illumination, surface preparation or acquisition protocol becomes an intervention with an assigned control, effect estimate, uncertainty, recovery evidence and physical-ledger status.

## Authored adversarial tests

Thirteen Rust tests cover:
- causal intervention detection despite strong linear drift;
- equivalence under no intervention with linear drift;
- balanced sequences counterbalancing manufactured common quadratic drift;
- asymmetric timing rejection;
- surface-reset and purge failures;
- independently verified randomization requirement;
- sequence-allocation imbalance;
- repeated blocks in one session not inflating independent sample count;
- physical-ledger and validation-envelope precedence;
- duplicate evidence and invalid sequence rejection.

## Claim boundary and pending work

This is a manufactured scalar counterfactual evaluator, not a laboratory protocol validation. It does not generate or verify the randomization itself, control chamber purge, verify electrode cleaning, measure surface conductivity, identify nonlinear drift, model particle charge/transport, process PIV images, calculate momentum flux or measure thrust.

Pending: local cargo fmt/test/Clippy (cargo/rustc/rustfmt are unavailable in this implementation environment), exact-head hosted CI, external review, pre-registered lab protocol, raw synchronized current/voltage/flow/surface-state data, and independent laboratory replication.

## References

- Zwolińska-Glądys, K., Cardona Murillo, J.P. & Jaszczur, M. (2026). Impact of relative humidity and PIV seeding on the electro-aerodynamic performance of needle-to-ring ionic fans. *Experiments in Fluids* 67, 168. DOI: 10.1007/s00348-026-04322-y.
- Srinivasan, V. (2026). On Methods of Uncertainty Quantification in Computational Modeling and Simulation: Harmonizing Modeling and Metrology Practices. NIST Advanced Manufacturing Series 100-85. DOI: 10.6028/NIST.AMS.100-85.
- Taylor, M.T., Lawson, J.M. & Ganapathisubramani, B. (2026). PIVtools: A comprehensive open source software for particle image velocimetry. *SoftwareX* 35, 102967. DOI: 10.1016/j.softx.2026.102967.
