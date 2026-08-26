# Adaptive Fidelity Research Protocol

## Research question

Can semantic/continuous-time supervision reduce simulation cost while keeping physical error inside a declared envelope and escalating unfamiliar, unstable, incompletely-accounted, or lifecycle-changing states to higher-fidelity physics early enough?

This protocol evaluates the controller in `advisory::fidelity::AdaptiveFidelityPolicy`.

## Architecture under test

The controller consumes a **complete assessment input** from two classes of evidence.

### Authoritative evidence

- numerical-health status;
- accounting completeness for the active modeled reservoirs;
- world/body/reservoir/representation lifecycle stability;
- conservation/reconciliation residual ratio;
- contact/constraint error;
- physical activity/instability.

### Advisory evidence

- causal/gameplay importance;
- semantic novelty from HDC or another representation;
- calibrated predicted physical error from CfC/LTC or another temporal model;
- calibrated model confidence.

The controller computes an explainable minimum fidelity floor using a max-lattice. No learned weighted sum may suppress an authoritative safety signal.

The controller then emits a `PhysicsAdvisory`. Promotions may jump directly to the required floor. Demotions are limited to one tier per proposal and still must pass `EpistemicFirewallPolicy` independently.

## Complete evidence versus unknown evidence

`FidelityEvidence` is deliberately a complete controller input, not a partially-known telemetry record. A caller must not manufacture a favorable value merely because live telemetry is unavailable.

In particular:

- unknown numerical health is not `true`;
- unknown accounting completeness is not `true`;
- unknown lifecycle stability is not `true`;
- an unavailable conservation/reconciliation residual is not `0.0`;
- an uncalibrated novelty proxy is not automatically a low novelty score;
- an unavailable physical-error prediction is not zero predicted error.

Shadow-mode collection should preserve missing signals explicitly until a later readiness/calibration layer can construct complete `FidelityEvidence`. No production fidelity reduction should depend on guessed defaults.

## Why a max-lattice

A weighted score can hide a severe failure signal behind many benign signals. For example, high confidence and low semantic novelty must never average away a failed invariant, incomplete reservoir accounting, an unresolved reservoir appearance/disappearance, or a large conservation residual.

Independent evidence floors make the decision monotonic and inspectable:

- failed numerical health -> highest-certified tier;
- incomplete accounting -> highest-certified tier;
- unresolved lifecycle/representation transition -> highest-certified tier;
- large conservation residual -> High/highest-certified;
- large constraint error -> High/highest-certified;
- high activity -> High;
- high causal importance -> Standard/High;
- high novelty -> High/highest-certified;
- high predicted error -> Standard/High/highest-certified.

The final required tier is the maximum of those floors and the configured absolute baseline floor.

`FidelityTier::Exact` is the enum label for this highest-certified/reference path; it is not a claim of mathematically exact floating-point dynamics.

## Policy validity

`AdaptiveFidelityPolicy` is serializable/versionable configuration and must be revalidated after construction or deserialization.

The controller fails closed when threshold bands are non-finite, out of range, negative where prohibited, or inverted. Required ordering includes:

- `high_conservation_residual <= exact_conservation_residual`;
- `high_constraint_error <= exact_constraint_error`;
- `high_novelty <= exact_novelty`;
- `standard_causal_importance <= high_causal_importance`;
- `standard_predicted_error <= high_predicted_error <= exact_predicted_error`.

Unit-interval thresholds must remain in `[0, 1]`.

Policy revision/fingerprint must eventually be included in every intervention evidence artifact so a result cannot be detached from the thresholds that produced it.

## Phase A: deterministic policy tests

Required properties:

1. numerical-health failure always selects the highest-certified tier;
2. incomplete accounting selects the highest-certified tier even when numeric residual is exactly zero;
3. unresolved lifecycle selects the highest-certified tier even when numeric residual is exactly zero;
4. high novelty selects the highest-certified tier even with perfect model confidence;
5. high causal importance raises fidelity even for spatially distant/calm state;
6. no calibrated error prediction means no demotion proposal;
7. a demotion proposal changes at most one tier at a time;
8. every semantic proposal carries an exact-state digest;
9. every reduction still passes the epistemic firewall independently;
10. malformed/NaN/inverted adaptive-policy thresholds fail closed.

## Phase B: offline shadow scheduling

Run the highest-certified physics path unchanged. At each scheduling interval, collect the authoritative state plus frozen semantic/model outputs and record what fidelity the controller *would* request only when complete evidence is available.

Do not enact proposals yet.

When evidence is incomplete, record the missing signal rather than inventing a controller input. Missingness itself is part of the research result.

Report:

- complete-evidence readiness rate;
- missing-evidence reason distribution;
- requested tier distribution on complete observations;
- promotion/demotion frequency;
- highest-certified fallback rate;
- novelty distribution and calibration status;
- predicted-error calibration;
- time spent in each reason class;
- reservoir/lifecycle instability frequency;
- hypothetical compute savings using measured per-tier costs.

This phase tests decision quality without contaminating physical trajectories.

## Phase C: paired intervention

Phase C is not eligible to start merely because the policy unit tests pass. The core Phase-Zero convergence gates must first establish authoritative world/session lifetime, reservoir lifecycle provenance, canonical angular dynamics for the studied regime, and measured rather than heuristic physical work for any coupled dissipation path.

For each complete held-out scenario, then run:

1. highest-certified reference;
2. fixed lower-fidelity baseline;
3. adaptive controller with HDC/CfC signals;
4. adaptive controller with semantic signals ablated;
5. adaptive controller with temporal error prediction ablated;
6. adaptive controller with only authoritative diagnostics.

Use identical initial conditions, seeds, inputs, command streams, lifecycle events, accounting boundaries, and deterministic mode.

## Required metrics

### Physical and accounting error

Choose scenario-appropriate authoritative metrics, including as applicable:

- position/orientation trajectory error;
- contact impulse error;
- penetration/constraint error;
- mass, momentum, angular-momentum, and energy residuals;
- state-versus-ledger reservoir reconciliation residuals;
- unresolved reservoir/body/representation lifecycle events;
- thermal/entropy residuals;
- topology/phase-transition discrepancies.

A small total energy residual does not override an unresolved reservoir lifecycle event or an unrepresented internal ledger port.

### Scheduling quality

- false-demotion rate;
- missed-promotion rate;
- highest-certified fallback precision/recall;
- anomaly lead time before an authoritative failure condition;
- dwell time per fidelity tier;
- representation/fidelity thrash rate;
- demotion attempts blocked by incomplete accounting/lifecycle evidence;
- intervention rollback/fallback success rate.

### Performance

- wall time;
- CPU/GPU utilization;
- peak memory;
- scheduling overhead;
- semantic/model inference overhead;
- exact/highest-certified shadow-reference overhead where used;
- net compute saved after all overhead.

## Acceptance gate for an adaptive-fidelity claim

An adaptive policy may be called **validated** only when it:

1. stays inside a pre-registered physical-error envelope on held-out scenarios;
2. never treats incomplete accounting, unresolved lifecycle, or unknown evidence as favorable demotion evidence;
3. produces no worse catastrophic-failure rate than the highest-certified reference within the studied validity domain;
4. demonstrates positive net compute savings after inference/scheduling/reference overhead;
5. reports all highest-certified fallbacks and missed escalations;
6. remains deterministic for the same frozen advisory stream and lifecycle stream;
7. retains exact provenance for every enacted learned/semantic proposal;
8. records the adaptive-policy and firewall-policy versions/thresholds used;
9. preserves the authoritative accounting/reconciliation contract across every enacted transition.

A claim of being **competitive** or **leading** additionally requires matched external-engine or fixed-policy baselines under the Physics Excellence Program.

## First target scenarios

Start with workloads where fidelity is naturally sparse in time or space **and** the authoritative lower layers are already validated:

- many sleeping rigid bodies with localized impacts after rigid/contact Phase-Zero gates;
- large terrain with one evolving instability zone after representation/lifecycle accounting exists;
- a vehicle scene with localized high-speed contacts after character/vehicle/contact validation;
- a fracture/collapse sequence after fracture work and topology transitions have explicit reservoirs/provenance;
- thermal diffusion after spatial thermal state and its stability/accounting campaign exist.

The eventual flagship remains a coupled terrain failure where the controller raises fidelity before fracture/collapse, then safely reduces it after the event while mass, momentum, energy/reservoir identity, causal ledgers, and representation transitions remain reconciled.
