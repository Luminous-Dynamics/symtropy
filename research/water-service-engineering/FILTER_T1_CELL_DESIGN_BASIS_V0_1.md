# FILTER-T1-CELL-001 — Modular Clean-Water Filter Hydraulic and Inspection Cell v0.1

**Tracking issue:** [#5087](https://github.com/Luminous-Dynamics/symtropy/issues/5087)  
**Filter observability:** [#5084](https://github.com/Luminous-Dynamics/symtropy/issues/5084)  
**Pump reference design:** [PUMP-T1-CW-01 #4994](https://github.com/Luminous-Dynamics/symtropy/issues/4994)  
**Status:** parametric design basis and synthetic arithmetic only.

## 1. Objective

Build a reusable filter-test spool and fixture set on the existing clean-water pump platform. The cell measures pressure loss, flow, bypass leakage, throughput and service history. It does not infer contaminant removal from hydraulic signals.

## 2. Key compatibility result

The current PUMP-T1-CW-01 reference envelope is 0.2–1.0 L/s, equivalent to **12–60 L/min**. The proposed household POU test envelope is **0.1–10 L/min**. These intervals do not overlap. Therefore:
- household POU testing needs a separate calibrated low-flow pump/loop and suitably ranged reference flow meter;
- community modular testing at 12–60 L/min can reuse the existing pump T1 duty envelope, subject to the selected pump's actual curve and component ratings;
- a municipal pilot needs a separate project-specific flow/head/pressure design. It cannot be obtained by simply scaling the household or community spool.

These are test-profile proposals, not universal service demand values.

## 3. Candidate topology

Vented reservoir → suction isolation/strainer → pump/VFD → mechanical pressure-relief return → reference flow meter → inlet pressure tap → filter DUT spool → outlet pressure tap → controlled resistance valve → reservoir.

A separately instrumented bypass is provided around the DUT, with isolation/valve-position verification and a way to measure or independently quantify leakage. Include:
- upstream/downstream pressure transmitters and an independent differential-pressure reference where the test objective requires it;
- flow and totalized-volume channels;
- temperature and reservoir level;
- uniquely identified feed/outlet sample ports for later T2 laboratory work (capped and unused in T1);
- replaceable housing adapters and documented actual bore/connection geometry;
- leak/spill detection, guard, drains/vents, secondary containment, local manual stop and lockout provisions.

No pressure rating, relief setpoint, cable size or protective-device rating is frozen here. Each depends on the final pump curve, shut-off head, weakest wetted component and competent engineering review.

## 4. Measurement boundary

The T1 measurement boundary covers water entering the DUT, treated outlet, measured bypass, pressure before/after DUT, throughput totalizer, temperature, power where declared, and any captured waste/backwash. Flow imbalance must be reconciled within the measurement uncertainty; it cannot be silently labelled filter leakage or media loss.

Turbidity, conductivity, pH, chlorine residual and UV intensity may be logged where appropriate, but are contextual operational signals only. They are not universal substitutes for contaminant-specific laboratory analysis.

## 5. Synthetic sizing arithmetic

At the community flow endpoints:
- 12 L/min = 0.2 L/s.
- 60 L/min = 1.0 L/s.
- In an illustrative 40 mm **actual internal diameter** line, mean velocity is approximately 0.159 m/s at 12 L/min and 0.796 m/s at 60 L/min.
- A 250 L usable reservoir turns over ideally in 20.83 min at 12 L/min and 4.17 min at 60 L/min.
- At the PUMP-T1-CW-01 boundary, ideal hydraulic power ranges from 9.81 W at 0.2 L/s and 5 m head to 245.25 W at 1.0 L/s and 25 m head.

These are idealized arithmetic checks. They do not include pipe/fitting losses, pump efficiency, NPSH, transients or a selected DUT pressure drop. A nominal DN40 is not assumed to have 40 mm ID.

## 6. T1 test sequence

1. Freeze filter/cartridge/media/housing revision and actual geometry.
2. Complete mechanical/electrical/safety design review before fabrication, filling or energization.
3. Verify range and calibration of flow, pressure, temperature and power channels.
4. Conduct an approved static leak check and verify relief return/containment.
5. Record clean-water baseline pressure-flow curves at registered points.
6. Test bypass closed/open states and valve-position versus measured bypass flow.
7. Test controlled flow excursions, stagnation/restart and shutdown within approved equipment limits.
8. Exercise throughput counter, cartridge identity, replacement event and counter reconciliation.
9. Repeat over multiple assemblies/batches where available; preserve failures and no-results.
10. Publish raw observations, calibration, uncertainty, configuration and exact analysis digest.

T1 is clean-water hydraulic/inspection work only. No microbial or hazardous chemical challenge is performed here.

## 7. Scale-specific approach

### Household POU
Use a dedicated low-flow loop and low-range calibrated meter. The current 12 L/min pump minimum is above the proposed 10 L/min household maximum. Do not use pump throttling alone as a substitute for a correctly ranged and validated low-flow test source.

### Community modular
Use the PUMP-T1-CW-01 platform only after checking the exact selected pump curve and DUT pressure drop. Add accessible isolation, safe cartridge changeout, bypass metering and captured waste/backwash.

### Municipal pilot
Freeze its own flow/head/pressure and sample plan, including train redundancy, pipe supports, relief, power, metering and laboratory sampling. A bench spool is not a municipal skid design.

## 8. Hold points and nonclaims

Competent mechanical and electrical reviewers must approve MAWP, shut-off pressure, relief capacity/setting, restraint, water-hammer/trapped-pressure hazards, wet-zone separation, overcurrent/RCD protection, earthing, local emergency stop and depressurization before energizing. No pressure-system safety, contaminant removal, potability, certification, filter life or safe-to-energize status is established by this design basis.
