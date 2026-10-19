# MINE-FLEX-PUMP-SUMP-001 — bounded pump/sump trajectories v0.1

## Purpose

This follow-on to MINE-FLEX-EXEC-001 exercises a stateful water-removal asset rather than only an aggregate process-batch schedule. It models a synthetic pump command trace, response delay, fixed declared operating flow/energy, an upper supported-head envelope, and a bounded sump inventory.

This is a contract test, not a calibrated pump curve, hydraulic model, operational controller, or mine safety case.

## Explicit synthetic units

- Inflow, sump volume and pump flow: liters and liters per scheduling period.
- Required and supported head: kPa.
- Energy consumption and budget: Wh per scheduling period.
- Every bound is inclusive and represented as a minimum/maximum interval.
- A period is a benchmark interval; it is not implicitly a minute, hour, or real equipment control cycle.

The input case declares an interval for inflow and required head in every period. The synthetic pump envelope declares minimum/maximum operating flow, maximum supported head, operating energy, startup energy, startup delay, and minimum on/off durations.

## State and execution contract

For every period, the evaluator calculates a bounded sump-state interval from the previous state, inflow interval, and pump-flow interval. It checks lower and upper sump limits, energy budgets, required head against the declared maximum head, startup delay, and minimum on/off times. Unit semantics are encoded in typed field names, with no implicit unit conversion; malformed numeric bounds and invalid time grids are rejected.

A separate conservation recomputation reconstructs the sump interval from the original inputs and each emitted flow interval. The receipt is deterministic and binds the exact source commit and generated case-set digest.

## Adversarial synthetic controls

The suite contains one feasible trace and controls that must be rejected for:
- sump drawdown below the lower operating limit;
- inflow filling beyond the upper sump limit;
- required head above the declared operating envelope;
- minimum-on-time violation;
- minimum-off-time violation; and
- energy-budget violation.

The exact-head workflow checks the control classifications, zero accepted hard violations, zero conservation mismatches, receipt digests and source SHA.

## Claim boundary

A passing suite proves only that this finite synthetic contract and its declared controls behave as specified. It does not prove real pump performance, hydraulic stability, the correctness of a field model, safety, cost or carbon improvements, or physical execution. The fixed flow/head/energy envelope is illustrative. Future work must introduce a calibrated pump curve, variable-speed behavior, response/ramp limits, telemetry freshness and provenance, observed sump measurements, uncertainty propagation, shared-resource allocation, and controller-in-the-loop testing before any site claim.

## Research context

This benchmark is motivated by existing methods, not a claim that asset dynamics or mine-pump scheduling are new:

- Romero et al., *A comparison of the economic benefits of centralized and distributed model predictive control strategies for optimal and sub-optimal mine dewatering system designs*, Applied Thermal Engineering (2015), DOI: https://doi.org/10.1016/j.applthermaleng.2015.01.031. This is relevant prior work on mine dewatering scheduling and control architecture.
- Wang et al., *Distributionally robust hybrid energy management in smart mining using process-coupled primal-dual mirror descent*, Scientific Reports (2025), https://www.nature.com/articles/s41598-025-11013-x. It studies process-coupled, uncertain multi-period mining energy management involving renewables, ventilation, dewatering and blasting.
- Li et al., *Unlocking lithium mine load flexibility: A multi-granularity dispatch disaggregation strategy for isolated microgrid*, Energy 360 (2026), DOI: https://doi.org/10.1016/j.energy.2026.141872. It explicitly addresses heterogeneous response power, response time, lag and response rate when translating aggregate dispatch requests into asset setpoints.
- NIST, *Digital Twins Workshops Summary Report* (NISTIR 8620, 2026), https://doi.org/10.6028/NIST.IR.8620, identifies interoperability, verification/validation, uncertainty quantification and cybersecurity as priorities for trustworthy industrial digital twins.

The results from those studies are not directly comparable with this finite synthetic suite: the plant models, datasets, baselines, objectives and evaluation protocols differ. The appropriate differentiator here is a separately testable, source-bound feasibility receipt and explicit rejection of infeasible trajectories. A real performance comparison must use matched site data, declared operating envelopes, held-out time windows and then blind site-separated evaluation. No cross-paper cost, tracking or emissions advantage is asserted by this benchmark.

## Cross-project composition

- Symtropy owns this dynamic asset and trajectory-validation experiment.
- Symthaea utility accounting (PIE-002 / #1610) remains separate from physical execution feasibility.
- Sol-Atlas water–power resilience (#147) owns network/service consequences and distributional service floors; modeled capacity is not proof of delivered water.
- Mycelix operational lineage (#2953) connects decision, authority, attempt and outcome; a simulator receipt is not operating authorization.
