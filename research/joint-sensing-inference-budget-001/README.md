# JOINT-SENSING-INFERENCE-BUDGET-001

**Status:** deterministic runtime co-scheduler reference; no camera driver, inference engine, simulator, trained policy or safety authority  
**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4623  
**Compute allocator dependency:** [#4577 / draft PR #4581](https://github.com/Luminous-Dynamics/symtropy/pull/4581), consumed via a path dependency. This branch is stacked on `research/anytime-driving-imagination-v0-1`; the dependent PR must target that branch until #4581 is merged.

## Purpose

Choose whether the next scarce resource unit should buy a fresh/richer observation or another world-model inference slice, while preserving the control deadline and mandatory actuation/recovery reserve.

This crate is a coordinator. It does not implement an inference allocator: it calls the existing anytime-driving-imagination allocator for one eligible next rollout. The parent allocator remains the authority for marginal-rollout validation, evidence admission, exact-ratio ordering, and actuation-reserve enforcement.

## Decision order

1. Validate the common decision deadline, resource-unit identity, decision-value unit, clock domain and action/recovery reserve.
2. Ask the existing allocator whether one more rollout is eligible using the current observation and remaining resource budget.
3. Validate an optional sensor-refresh candidate against the same episode context, checkpoint, observation, policy-visible source role, SAM evidence and resource unit.
4. If the current observation is stale, choose a fresh observation when it can arrive before the action deadline and reserve; otherwise require the external fallback.
5. If the current observation is fresh and both candidates are eligible, compare their decision-regret-reduction-per-resource-unit ratios using 128-bit cross multiplication. Both gains must share the same declared decision-value unit, costs must share the same declared resource unit, and latency bounds must share the same clock domain. No floating-point tie instability.
6. If ratios tie, prefer the already-qualified inference slice; stable behavior is explicit.
7. If neither resource has positive admissible value, stop deliberation and return commit-now (not an actuator command).

## Common comparison units

There are three units that must be shared before the scheduler compares candidates:

- **Resource unit:** the same quantity for sensor work and inference work, e.g. measured microjoules on the exact target hardware.
- **Decision-value unit:** the same regret-reduction scale for sensor refresh and inference, e.g. caller-defined micro-loss units. The scheduler cannot compare scores from different utility scales.
- **Clock domain:** the same declared clock/tick domain for sensor acquisition-to-delivery and inference latency bounds. Wall-clock nanoseconds cannot be mixed with simulation ticks.

The identifiers are passed explicitly and are bound into the joint decision receipt. The scheduler checks identity equality; it does **not** independently calibrate a measurement, prove the utility estimates are well calibrated, or authenticate the source digests. The inference allocator's `compute_units_per_rollout` value must be calibrated into the same resource unit by the caller and bound by its compute-cost digest. FLOPs, tokens, GPU utilization and model steps are proxies, not measured energy.

Sensor grants carry the exact SAM evidence digest that admitted their estimated decision value. This establishes a reference to the evidence used, not proof that the estimate is correct or an authenticated signature.

Sensor and inference costs must use the same caller-declared resource unit, e.g. measured microjoules on the exact target hardware. The inference allocator's compute_units_per_rollout value must be calibrated into that same unit by the caller and bound by its compute-cost digest. FLOPs, tokens, GPU utilization and model steps are proxies, not measured energy.

## Tests and validation boundary

Twenty-nine known-answer/adversarial tests are authored for exact-ratio selection and ties, stale/missing observation refresh, fallback on unavailable evidence, sensor/inference deadlines and recovery reserve, source-role and SAM/context rejection, resource/value/clock unit mismatches, no-positive-value stopping, and JSON receipt/schema round trips.

This authoring environment has no `cargo` or `rustc`; no local formatting, compilation, test or Clippy pass is claimed. The exact-head workflow is the first compilation authority. A queued or pending workflow is not pass evidence.

## Claim ceiling

Passing tests establish deterministic scheduling over finite supplied estimates and identities. They do not prove the marginal value estimates are calibrated, that more sensing improves driving, that extra inference improves the chosen action, that a resource unit equals physical energy unless independently measured, or that any vehicle is safe. Empirical claims require matched sensor-only, inference-only and joint closed-loop comparisons on a sealed split.
