# LOCO-SETVAL-001 — Augmented assurance state and certificate validity

Status: research architecture specification; no hardware safety certificate is claimed.

Parent: https://github.com/Luminous-Dynamics/symtropy/issues/4083

Certificate lifecycle implementation: [LOCO-ASSURANCE-001 / #4117](https://github.com/Luminous-Dynamics/symtropy/issues/4117), implemented as a separate evidence gate in [PR #4119](https://github.com/Luminous-Dynamics/symtropy/pull/4119).

## 1. Paradigm shift: the robot's evidence substrate is part of the operational state

A conventional locomotion state may contain body pose, joint position/velocity and contact mode. That is not enough to determine whether a safety argument remains valid.

A robot can be instantaneously balanced while:
- its only contact sensor is stale or stuck;
- its contact calibration is invalid on the current surface;
- an actuator has thermally derated and cannot deliver the assumed torque;
- command latency or bus jitter invalidates the assumed action timing;
- a control/reachability deadline was missed;
- model residuals have moved outside the certified parameter envelope.

These are not merely logging metadata. They change the admissible physical successor set or invalidate the evidence used to justify an action.

We therefore propose an augmented cyber-physical assurance state:

\[
z_t = (x_t, q_t, \Theta_t, b_t, H_t, A_t, C_t, M_t)
\]

where:

- \(x_t\): physical configuration and velocity state;
- \(q_t\): discrete contact/gait/actuation mode;
- \(\Theta_t\): bounded individual, terrain and actuator parameter set;
- \(b_t\): probabilistic belief over latent contact/terrain hypotheses (when used);
- \(H_t\): sensor/evidence health (freshness, dropout, saturation, calibration, OOD and clock mapping);
- \(A_t\): actuator condition (thermal state, voltage, derating, deadband, tracking error and limits);
- \(C_t\): computation/communication condition (deadline slack, measured latency, jitter, queue delay and command age);
- \(M_t\): model and certificate validity context (model digest, calibration identity, validated envelope and assumption status).

This extends the physical hybrid state; it does not turn metadata into physical forces. The added variables must affect the transition relation only through explicit, justified sensor, actuator, timing and model-validity relationships.

## 2. Keep physical viability and assurance validity distinct

Maintain at least two separate predicates:

1. **Physical viability** \(V_{phys}\): all declared admissible physical successors preserve the hard physical constraints or reach an allowed physical recovery set.
2. **Assurance validity** \(V_{assure}\): the assumptions, sensor health, actuator envelope, timing bounds and model identity required by the certificate remain valid, or a separately qualified fallback exists.

Operational certification requires both:

\[
V_{operational}=V_{phys}\cap V_{assure}
\]

A system can be physically viable but not certifiable with current evidence. That state must not be described as physically unsafe by definition; it is an **assurance-degraded / unknown** state that triggers conservative re-estimation, re-certification or fallback.

Conversely, a valid sensor/calibration record does not imply physical viability. Neither channel may substitute for the other.

## 3. State-dependent actuator and timing bounds

Actuator feasibility must depend on condition:

\[
u_t\in U(A_t)
\]

For example, thermal derating, voltage sag, backlash or tracking error may shrink the deliverable torque/position set. A nominal actuator envelope must not remain active after the measured hardware condition leaves the range in which it was validated.

Control timing must be represented explicitly:

\[
\tau_{sense}+\tau_{infer}+\tau_{cert}+\tau_{comm}+\tau_{act}
\leq \tau_{deadline}
\]

The terms must be measured or conservatively bounded. If a certificate is not available before its deadline, the system must not apply a stale certificate to a later action. It must use a separately certified fast fallback or return Unknown/abstain.

A delayed action is a different hybrid transition, not merely a slower version of the nominal action.

## 4. Evidence expiry and model invalidation

Every certificate is conditional on a model and evidence envelope. Bind:
- model/parameter/terrain digest;
- sensor calibration and freshness bounds;
- actuator calibration and current thermal/voltage state;
- clock alignment and latency bounds;
- solver/version, tolerance and resource budget;
- state-space and disturbance envelope;
- certificate creation time, validity horizon and explicit invalidation conditions.

Invalidate or widen the model when:
- observations are stale, missing, clipped, saturated or contradictory;
- contact evidence is outside its validated material/surface class;
- actuator temperature, current or tracking residual exits its validated envelope;
- measured transition residuals violate the assumed dynamics bounds;
- timestamps or command-delivery deadlines cannot be established;
- resource exhaustion or a missed compute deadline prevents complete certification.

Do not silently preserve a narrow viable set after its assumptions become invalid.

## 5. Runtime architecture: fast guard, slower certificate, independent monitor

Use a hierarchical execution path:

1. **Fast runtime guard:** deterministic, low-latency check against a conservative certified envelope; has a known deadline.
2. **Slower viability/reachability service:** recomputes phase-conditioned recovery options, set-valued transitions and model validity.
3. **Independent monitor/fallback:** detects stale certificates, missed deadlines, sensor/actuator-health changes and invalid assumptions; triggers a prequalified stop, stance reorganization or conservative recovery.

This is compatible with established runtime-assurance / Simplex ideas; those concepts are not claimed as novel. The proposed research is to make certificate validity, sensor/actuator health and deadline state explicit components of the viability and recovery benchmark.

## 6. Frozen adversarial tests

- identical physical pose with healthy versus stale contact observation must produce different assurance status;
- contact switch stuck-high, stuck-low, delayed, dropped and saturated;
- soft terrain where binary switches can false-negative;
- thermal derating changes the actuator torque set and may remove a previously admissible action;
- voltage sag and actuator tracking error;
- command delay crosses a gait/contact guard;
- control deadline met versus missed;
- reachability timeout returns Unknown and cannot reuse the expired result;
- calibration/model digest mismatch invalidates the certificate;
- model residual exceeds the validated envelope;
- an assurance-degraded but physically recoverable state selects an independently certified fallback;
- fresh evidence may restore certificate validity only after the declared revalidation step;
- input ordering and event ordering preserve deterministic outputs.

## 7. Additional scorecard

Report separately:
- time inside a certified operational envelope;
- sensor-health detection latency;
- stale-certificate detection latency;
- actuator derating prediction error;
- deadline miss rate and worst-case end-to-end latency;
- fallback activation correctness and time to fallback;
- viable-set shrinkage caused by actuator/timing constraints;
- unnecessary certificate invalidation rate;
- recovery success after evidence degradation;
- false-valid certificate count (target zero on the frozen corpus).

Do not combine physical safety, certificate validity, controller reward, energy or compute cost into one score.

## 8. Benchmark motivation

SPINDER (Maoudj et al., Scientific Reports 16, 26147, 2026) documents hardware-relevant effects that make these tests concrete: binary foot switches can produce false negatives on soft/compliant terrain; servo backlash and compliance create stance-exchange mismatch; sustained walking can trigger thermal torque reduction; and the authors identify contact-loss/fault-tolerant gait reorganization as future work. DOI: https://doi.org/10.1038/s41598-026-56856-0.

The contact-sensing survey by Jia et al. (Artificial Intelligence Review, 2026) explicitly motivates information-health monitoring and morphology–sensing–representation co-design: https://doi.org/10.1007/s10462-026-11644-0.

These motivate the benchmark, not a claim that the proposed architecture has demonstrated superior hardware results.

## 9. Claim ceiling

An augmented-state specification is not a validated state model. Each sensor-health, actuator-thermal and deadline transition must be calibrated and validated on the target platform. A runtime guard that is too conservative may be safe but unusable; its usable operational-envelope coverage must be reported.

A complete certificate is conditional on:
1. sound physical transition overapproximation;
2. validated actuator and timing bounds;
3. valid sensor/estimator health and calibration;
4. complete computation before its deadline;
5. independent monitoring of assumption violations.

Only after these obligations are discharged may the system claim an operational certificate for the declared envelope.
