# Non-Ideal Mixing Fidelity Qualification — Benchmark Protocol v0.1

**Programme:** WATER-MIXING-FIDELITY-QUAL-001  
**Tracking:** [Issue #5216](https://github.com/Luminous-Dynamics/symtropy/issues/5216)  
**Snapshot:** 14 November 2026  
**Evidence ceiling:** synthetic mass-routing and discrete RTD arithmetic only

## 1. Why a separate mixing qualification is necessary

A network-scale solver needs tractable mixing assumptions. Those assumptions are not automatically a physical description of every junction or tank.

EPANET 2.2 explicitly models complete, instantaneous mixing at pipe junctions. For tanks it offers complete mixing, two-compartment mixing, FIFO plug flow and LIFO plug flow. This is a useful baseline family, not proof that a particular asset follows one of those idealizations.

Published work gives us a concrete challenge set:
- Ho (2008) reports an analytical bulk-mixing model as a lower bound to mixing in the studied junctions and complete mixing as an upper bound, with an intermediate scaling approach.
- Cross-junction CFD and experimental studies report incomplete mixing and concentration differences tied to flow conditions.
- A multi-geometry study found mixing at tested crosses related to flow momentum ratio, while double-tee configurations behaved differently and could contain stagnant zones.
- A clear-water tank fluoride-tracer study found CFD and a series/parallel compartment model close to measured concentrations; a single CSTR fit worst among the compared tank models.

Primary sources are pinned in the comparator register. This programme has not independently reproduced those experiments; they are research comparators, not project validation.

## 2. Paradigm shift: qualify an operator, not a global mixing flag

The network model should remain computationally useful. The innovation is to make local mixing fidelity an explicit, replaceable, evidence-bearing operator.

### 2.1 Junction transfer operator

For inlet (i), let flow be (Q_i), tracer concentration be (C_i), and conservative tracer mass flux be (m_i=Q_i C_i). Define (P_{ji}) as the fraction of inlet (i)'s tracer mass flux delivered to outlet (j).

Required constraints:

[
P_{ji}\geq0,\qquad \sum_j P_{ji}=1
]

[
\dot m_j^{out}=\sum_i P_{ji}Q_iC_i,\qquad
C_j^{out}=\frac{\dot m_j^{out}}{Q_j^{out}}
]

The columns of (P) conserve each inlet's tracer mass. The complete-mixing baseline is the special case (P_{ji}=Q_j^{out}/\sum_k Q_k^{out}), yielding the same concentration at every outlet. A segregated or partially mixed transfer matrix can produce different outlet concentrations while still conserving total tracer mass.

This is an algebraic contract for T0. It does **not** claim that a transfer matrix can be inferred from topology alone. Physical T1 must estimate the operator from geometry-specific tracer measurements and flow conditions.

### 2.2 Tank residence-time distribution

A conservative pulse tracer experiment estimates a residence-time distribution (E(t)). From discrete collection bins with recovered tracer masses (m_k) at representative times (t_k), compute the conditional recovered-mass weights only after separately reporting total recovery:

[
r=\frac{\sum_k m_k}{M_{in}}
]

[
\bar t=\frac{\sum_k m_k t_k}{\sum_k m_k},\qquad
\sigma_t^2=\frac{\sum_k m_k(t_k-\bar t)^2}{\sum_k m_k}
]

Also report early-arrival fraction and long-tail fraction against declared time thresholds. Do not silently renormalize a poor-recovery tracer run into a valid RTD. The unobserved mass remains unresolved and the RTD disposition is indeterminate outside the declared recovery tolerance.

RTD moments and quantiles are descriptors, not a complete hydrodynamic state. For operational use, preserve the measured curve, confidence bands, injection waveform, tank level/flow schedule, and sensor response.

## 3. Comparator hierarchy

| Comparator | What it tests | Qualification boundary |
|---|---|---|
| EPANET 2.2 | Standard complete junction mixing; four tank mixing options | Mandatory network-scale baseline; not universal physical truth |
| Ho bulk-mixing / complete-mixing bounds | Efficient reduced-order junction alternatives | Reproduce within tested geometry and flow domain |
| Cross / tee / double-tee CFD | Geometry-resolved velocity and concentration fields | Mesh, time-step, turbulence and scalar-diffusivity sensitivity required |
| Physical junction tracer rig | Actual outlet-wise mixing | Calibrated, synchronized, blinded measurements |
| Single / multi-CSTR, series-parallel and empirical RTD | Tank reduced-order alternatives | Fit on training runs, evaluate on held-out injection and operating schedules |
| Tank CFD + tracer experiment | High-fidelity physical hypothesis | Independent tracer validation; CFD is not ground truth |

Do not compare models on different geometry, flow split, tracer diffusivity, injection waveform or sensor bandwidth and call the result a leaderboard.

## 4. Benchmark design

### Junction strata
- Tee, cross, double-tee A and double-tee B.
- Symmetric and asymmetric flow splits.
- Flow momentum ratio sweep and relevant Reynolds-number regimes.
- Branch orientation and connection-length variants.
- Reversal/transient conditions only where the chosen solver supports and qualifies them.

### Tank strata
- Fill-and-draw and through-flow.
- Pulse and step tracer injection.
- Complete mix, two-compartment, FIFO and LIFO baselines.
- One CSTR, series CSTR, series/parallel compartments and empirical RTD.
- Short-circuiting, stagnant/dead volume and long-tail cases.
- Variable level, inflow/outflow schedule and inlet/outlet arrangement.

### Blind split
Hold out entire geometries, facilities, junction families or operating regimes. Randomly splitting samples from the same physical run is leakage, not a meaningful generalization test. Freeze raw-data hashes and calibration metadata before exposing the holdout.

### Confounders
Temperature/viscosity, tracer diffusivity, injection duration, flow variability, geometry uncertainty, sensor response, sample-line delay, clock offset, detection limit, tracer adsorption and incomplete recovery must be measured, controlled or represented as uncertainty.

## 5. Metrics and research targets

Report all metrics per geometry family and condition, not only a pooled average.

- Junction: outlet-wise concentration MAE/RMSE, peak concentration error, arrival-time error, outlet cross-contamination matrix error, mass residual and uncertainty interval coverage/width.
- Tank: RTD mean and variance error, early-arrival fraction error, tail-mass fraction error, quantile error, recovered-mass fraction and curve distance.
- Calibration: number and type of fitted parameters, calibration data volume, stability across replicates and failure under blind holdout.
- Cost: runtime, peak memory, mesh/solver cost, calibration effort, tracer/sensor cost and data-processing burden.
- Integrity: false-safe rate on frozen high-consequence controls; out-of-domain/indeterminate rate; percentage of results with complete raw provenance.

Research targets (not achieved): nominal 90% prediction-interval coverage on blind physical conditions; no false-safe decision in the frozen critical corpus; conservative tracer recovery within combined measurement uncertainty; time-alignment uncertainty below one tenth of the shortest resolved event feature; no transfer of fitted geometry-specific coefficients outside a validated domain.

Do not optimise only a scalar score. Publish the accuracy–uncertainty–compute–instrumentation Pareto frontier and the abstention rate.

## 6. T0 fixtures and T1 physical plan

The T0 corpus contains 15 synthetic vectors:
- 8 junction mass-routing known-answer/refusal cases;
- 7 tank RTD known-answer/refusal cases.

T0 checks arithmetic, conservation, normalization, flow balance, time ordering, calibration-state and refusal semantics only.

T1 should use a non-potable closed loop with:
1. rights-cleared, dimensioned junction/tank geometries;
2. conservative tracer with validated compatibility and safe handling;
3. calibrated flow meters and pressure/level instrumentation;
4. multi-outlet concentration sensors or synchronized sample collection;
5. known injection mass and waveform;
6. independent mass recovery and raw data custody;
7. blinded repeats and held-out conditions.

For tanks, acquire pulse response first, then step response if useful. For junctions, measure every outlet and vary flow momentum ratio while preserving known geometry. Do not infer the full outlet concentration field from one sensor.

No tracer injection into a potable asset is proposed. Physical work requires a competent operator, approved non-potable rig, risk assessment and waste/disposal procedure.

## 7. Interface to the parent intrusion programme

Mixing fidelity is a sub-qualification of network transport, not a new sixth causal stage. The parent chain remains initial state → transient pressure → defect ingress → network transport → chemical fate. The network-transport stage must identify the mixing operator and its evidence domain. If mixing is not qualified for the geometry/flow regime, downstream concentration intervals widen or the result abstains; the chain cannot silently fall back to ideal mixing.

Conservative tracer qualification precedes reactive chemistry. This separates transport/mixing discrepancy from reaction-rate discrepancy.

## 8. Nonclaims

All T0 rows are synthetic. No measured physical data, site mixing operator, tank RTD, validated solver, potability result, toxicological conclusion, SANS 241 compliance claim or release authority is created by this protocol.
