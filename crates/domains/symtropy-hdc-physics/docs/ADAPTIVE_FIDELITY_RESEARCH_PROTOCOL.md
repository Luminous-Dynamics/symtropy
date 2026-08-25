# Adaptive Fidelity Research Protocol

## Research question

Can semantic/continuous-time supervision reduce simulation cost while keeping
physical error inside a declared envelope and escalating unfamiliar or unstable
states to higher-fidelity physics early enough?

This protocol evaluates the controller in
`advisory::fidelity::AdaptiveFidelityPolicy`.

## Architecture under test

The controller consumes normalized evidence from two classes:

### Exact evidence

- numerical-health status;
- conservation/reconciliation residual ratio;
- contact/constraint error;
- physical activity/instability.

### Advisory evidence

- causal/gameplay importance;
- semantic novelty from HDC or another representation;
- calibrated predicted physical error from CfC/LTC or another temporal model;
- calibrated model confidence.

The controller computes an explainable minimum fidelity floor using a max-lattice.
No learned weighted sum may suppress an exact safety signal.

The controller then emits a `PhysicsAdvisory`. Promotions may jump directly to
the required floor. Demotions are limited to one tier per proposal and still
must pass `EpistemicFirewallPolicy`.

## Why a max-lattice

A weighted score can hide a severe failure signal behind many benign signals.
For example, high confidence and low semantic novelty must never average away a
large conservation residual. Independent evidence floors make the decision
monotonic and inspectable:

- failed numerical health -> Exact;
- large conservation residual -> High/Exact;
- large constraint error -> High/Exact;
- high activity -> High;
- high causal importance -> Standard/High;
- high novelty -> High/Exact;
- high predicted error -> Standard/High/Exact.

The final required tier is the maximum of those floors and the configured
absolute baseline floor.

## Phase A: deterministic policy tests

Required properties:

1. exact numerical-health failure always selects `Exact`;
2. high novelty selects `Exact` even with perfect model confidence;
3. high causal importance raises fidelity even for spatially distant/calm state;
4. no calibrated error prediction means no demotion proposal;
5. a demotion proposal changes at most one tier at a time;
6. every semantic proposal carries an exact-state digest;
7. every reduction still passes the epistemic firewall independently.

## Phase B: offline shadow scheduling

Run the highest-certified physics path unchanged. At each scheduling interval,
feed the controller the same exact state plus frozen semantic/model outputs and
record what fidelity it *would* have requested.

Do not enact proposals yet.

Report:

- requested tier distribution;
- promotion/demotion frequency;
- exact-fallback rate;
- novelty distribution;
- predicted-error calibration;
- time spent in each reason class;
- hypothetical compute savings using measured per-tier costs.

This phase tests decision quality without contaminating physical trajectories.

## Phase C: paired intervention

For each complete held-out scenario, run:

1. exact/highest-certified reference;
2. fixed lower-fidelity baseline;
3. adaptive controller with HDC/CfC signals;
4. adaptive controller with semantic signals ablated;
5. adaptive controller with temporal error prediction ablated;
6. adaptive controller with only exact diagnostics.

Use identical initial conditions, seeds, inputs, and deterministic command
streams.

## Required metrics

### Physical error

Choose scenario-appropriate exact metrics, including as applicable:

- position/orientation trajectory error;
- contact impulse error;
- penetration/constraint error;
- mass, momentum, angular-momentum, and energy residuals;
- thermal/entropy/reconciliation residuals;
- topology/phase-transition discrepancies.

### Scheduling quality

- false-demotion rate;
- missed-promotion rate;
- exact-fallback precision/recall;
- anomaly lead time before an exact failure condition;
- dwell time per fidelity tier;
- representation/fidelity thrash rate.

### Performance

- wall time;
- CPU/GPU utilization;
- peak memory;
- scheduling overhead;
- semantic/model inference overhead;
- net compute saved after overhead.

## Acceptance gate for an adaptive-fidelity claim

An adaptive policy may be called **validated** only when it:

1. stays inside a pre-registered physical-error envelope on held-out scenarios;
2. produces no worse catastrophic-failure rate than the exact/highest-certified
   reference within the studied validity domain;
3. demonstrates positive net compute savings after inference/scheduling cost;
4. reports all exact-fallbacks and missed escalations;
5. remains deterministic for the same frozen advisory stream;
6. retains exact provenance for every enacted learned/semantic proposal.

A claim of being **competitive** or **leading** additionally requires matched
external-engine or fixed-policy baselines under the Physics Excellence Program.

## First target scenarios

Start with workloads where fidelity is naturally sparse in time or space:

- many sleeping rigid bodies with localized impacts;
- large terrain with one evolving instability zone;
- a vehicle scene with localized high-speed contacts;
- a fracture/collapse sequence with long quiet precursors;
- thermal diffusion with a localized phase-transition front.

The eventual flagship should be a coupled terrain failure where the controller
raises fidelity before fracture/collapse, then safely reduces it after the event
while the conservation and causal ledgers remain reconciled.
