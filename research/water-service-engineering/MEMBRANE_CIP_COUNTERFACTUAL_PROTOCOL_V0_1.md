# MEMBRANE-CIP-COUNTERFACTUAL-001 — Cleaning Intervention Qualification v0.1

**Tracking issue:** [#5012](https://github.com/Luminous-Dynamics/symtropy/issues/5012)  
**Programme:** [#4977 MEMBRANE-FOULING-QUAL-001](https://github.com/Luminous-Dynamics/symtropy/issues/4977)  
**Dataset crosswalk:** [#4996](https://github.com/Luminous-Dynamics/symtropy/issues/4996)  
**Status:** causal evaluation contract only; no cleaning policy, recipe or membrane intervention is qualified.

## 1. Research question

Which cleaning intervention, at which decision point, produces the most profile-conforming permeate and membrane service life subject to water quality, integrity, safety and lifecycle-cost constraints?

Fouling prediction is not a cleaning decision. A lower TMP immediately after CIP is not proof of a lower lifecycle cost, more conforming water, reduced irreversible damage or longer membrane life.

## 2. Paradigm shift

Replace binary fouling alerts with counterfactual process management:
- compare the observed action with plausible alternative actions at the same decision point;
- quantify reversible recovery separately from irreversible deterioration;
- account for chemicals, cleaning water, heat, energy, labour, waste, downtime, replacement and quality outcomes;
- optimize profile-conforming water rather than raw flux alone;
- permit abstention when action support, causal identification or quality evidence is insufficient.

Historical cleaning data are observational. Operators intervene when they suspect fouling, creating confounding by indication. Time-varying feedwater, membrane age, module heterogeneity, seasonality, prior cleaning, operator selection and replacement censoring further bias naive comparisons.

## 3. Target-trial emulation contract

Before analysis, freeze:
1. **Eligibility:** module in service, critical sensors available, process state observed and within approved operating envelope, with no unresolved integrity/quality emergency.
2. **Time zero:** exact decision opportunity before the intervention begins.
3. **Strategies:** calendar schedule, SOP threshold, mechanism/physics-informed trigger, bounded early CIP, deferred CIP with approved rescue rules, or observation when safe.
4. **Treatment version:** chemical identity, concentration, pH, temperature, contact time, pressure/flow, rinse endpoint and sequence. Distinct recipes are distinct interventions.
5. **Follow-up:** 24h, 7d, 30d and 90d plus complete cycle until next cleaning or replacement.
6. **Outcomes:** conforming permeate, post-cleaning recovery, integrity/rejection, energy, chemicals, cleaning water, waste, downtime, labour and replacement.
7. **Estimand:** intention-to-treat policy effect first; per-protocol effect only with defensible adherence and confounding adjustment; subgroup effects by membrane/feed regime.

Never compare cleaned and uncleaned random time windows or use future information to define time zero.

## 4. Minimum causal data contract

- Membrane family, model, module/serial, area, age, accumulated service volume and complete prior CIP/replacement history.
- Timestamped TMP/stage differential pressure, feed/concentrate/permeate pressure, flow, flux, conductivity and temperature where actually measured.
- Feedwater state: conductivity/salinity, temperature, pH, turbidity/organic indicators and chemistry where available.
- Pre-decision process state, alarm, operator rationale, SOP/threshold, decision timestamp and execution timestamp.
- Full cleaning recipe and execution trace, chemical quantity, temperature, pH, contact time, water, energy/heat, rinse endpoint, waste and labour.
- Post-cleaning recovery at measured horizons; integrity/rejection; actual lab samples, methods, units and detection limits.
- Downtime, emergency intervention, replacement, failure, service outage, cost and censoring.
- Site/module/operator/controller/instrument identities and calibration status.

If key confounders, treatment versions or follow-up records are missing, the causal effect is NOT_IDENTIFIED or restricted to a justified subpopulation.

## 5. Causal methods — conditional, not automatic

Choose only after the data structure is audited:
- target-trial emulation;
- propensity weighting/matching for baseline treatment assignment, with overlap and effective sample size;
- marginal structural models or longitudinal g-formula for measured time-varying confounders affected by prior treatment;
- doubly robust estimation or TMLE when consistency, positivity and exchangeability assumptions are defensible;
- off-policy evaluation only within observed treatment support;
- survival/competing-risk methods for cleaning, replacement, integrity failure and outage;
- sensitivity bounds for unmeasured confounding and heterogeneous treatment versions.

Required diagnostics: propensity overlap, effective sample size, covariate balance, positivity, negative controls where defensible, pre-trend/pseudo-time-zero tests, sensitivity analysis, site/module/operator holdouts, calibration and abstention under out-of-domain conditions.

## 6. Baselines

1. Calendar-based CIP schedule.
2. Existing operator/SOP threshold.
3. TMP/flux trend or EWMA/CUSUM.
4. Mechanistic resistance/permeability threshold.
5. ML/hybrid trigger with calibrated uncertainty and abstention.
6. Perfect-information retrospective oracle only as an upper bound, never a deployable comparator.

Every method shares the same population, causal decision time, service boundary, follow-up and treatment support. No model wins by moving chemical, reject-water, downtime, replacement or quality costs outside the metric boundary.

## 7. Outcomes and metrics

- Profile-conforming permeate by horizon and per module-cycle.
- Post-CIP normalized permeability/flux recovery at 24h, 7d, 30d and 90d.
- Recovery retention and irreversible loss over repeated cycles.
- Lifecycle cost per cubic metre of conforming permeate, including water, energy, chemicals, labour, waste, downtime and replacement.
- Cleaning frequency, false intervention burden, missed intervention, emergency rescue and availability.
- Water recovery, reject/concentrate, chemical and energy intensity.
- Integrity/rejection and lab-confirmed quality only where measured with actual method and sample.
- Policy value/regret, uncertainty intervals and heterogeneous treatment effects.

Do not reduce the outcome to classification accuracy or a single TMP reduction.

## 8. Proposed stretch goals (not achieved)

- At matched quality/integrity and service constraints, seek ≥10% lower lifecycle cleaning cost per cubic metre of conforming permeate than the best existing scheduled/threshold policy.
- Seek ≥10% fewer unnecessary CIP interventions at equal or lower missed-event risk.
- Seek ≥10% more conforming permeate per cleaning cycle without increased membrane damage or quality risk.
- Demonstrate calibrated uncertainty, overlap and an explicit abstention region.

These are research goals, not achieved results or promises.

## 9. Safety boundary

Model output is advisory only. No autonomous chemical dose, cleaning sequence, pressure/temperature setpoint, valve, bypass, discharge or release is authorized. Existing local interlocks, equipment limits, manufacturer restrictions, operator approvals and regulatory obligations remain authoritative. Out-of-domain state, missing critical measurement, quality uncertainty or lack of causal support means abstain.

Physical T1 cleaning trials require manufacturer- and competent-engineer-approved procedures, risk assessment, independent metrology and approved waste handling.

## 10. Nonclaims

No CIP policy, causal treatment effect, membrane recovery, cleaning recipe, cost reduction, saved water, product integrity or autonomous control is qualified. Correlation between a cleaning event and improved TMP/flux is not a causal effect or proof of lifecycle benefit.
