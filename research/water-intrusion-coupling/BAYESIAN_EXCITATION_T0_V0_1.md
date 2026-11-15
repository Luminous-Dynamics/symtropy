# Prior-Robust Bayesian Excitation Design T0 v0.1

**Programme:** WATER-MIXING-BAYESIAN-EXPERIMENT-DESIGN-001 / [#5324](https://github.com/Luminous-Dynamics/symtropy/issues/5324)  
**Active-excitation baseline:** [#5309](https://github.com/Luminous-Dynamics/symtropy/issues/5309), [draft PR #5320](https://github.com/Luminous-Dynamics/symtropy/pull/5320)  
**Constrained kernel identification:** [#5298](https://github.com/Luminous-Dynamics/symtropy/issues/5298), [draft PR #5306](https://github.com/Luminous-Dynamics/symtropy/pull/5306)  
**Evidence class:** synthetic finite-prior Gaussian-mixture information-bound arithmetic  
**Evidence ceiling:** no physical junction qualification

## 1. Research shift

The prior active-excitation T0 computes local Fisher information at one reference kernel and checks rank/conditioning after nuisance adjustment. That is useful, but local derivatives can be fragile when the physical kernel, actuator dynamics and sensor response are uncertain.

This increment evaluates each command sequence against a finite prior ensemble of *joint* physical-kernel, injector-response and sensor-response hypotheses. It uses a tractable mutual-information lower bound for the resulting Gaussian mixture. Bania & Wójcik (2025) explicitly motivate mutual-information input design as an alternative when local Fisher design is inadequate under substantial model uncertainty or nonlinear dynamics, and use a pairwise-distance lower bound for finite Gaussian mixtures. Their paper compares the approach against average D-optimal and semi-Bayesian designs. This T0 implements the equal-measurement-covariance special case of that lower bound; it does not claim to reproduce their entire method.

Reference: [Bania & Wójcik, *An Approximate Bayesian Approach to Optimal Input Signal Design for System Identification* (2025)](https://pmc.ncbi.nlm.nih.gov/articles/PMC12562801/).

## 2. Forward measurement ensemble

For prior particle i with weight p_i, physical kernel K_i, calibrated injector response H_a,i and calibrated sensor response H_s,i:

\[
u_{m,i}=H_{a,i}*u_c,\qquad
\mu_i(u_c)=H_{s,i}*(K_i*u_{m,i}).
\]

Each particle represents a joint plausible realization of the physical operator and the two calibration transfer functions. All particles use one common, declared, full SPD observation covariance Sigma in v0.1. The covariance is not diagonalized.

Prior weights must be strictly positive and sum to one. Duplicate particle IDs, invalid passive kernels, non-unit-area actuator/sensor impulse responses and non-SPD covariance are rejected.

## 3. Mutual-information lower bound

For common Gaussian measurement covariance Sigma, the pairwise Bhattacharyya distance between prior-particle observation distributions simplifies to:

\[
d_{ij}(u_c)=\frac{1}{8}
(\mu_i-\mu_j)^T\Sigma^{-1}(\mu_i-\mu_j).
\]

The evaluator calculates the Kolchinsky–Tracey lower bound:

\[
I_L(u_c)=-\sum_i p_i
\log\left(\sum_jp_j e^{-d_{ij}(u_c)}\right),
\qquad
0\leq I_L\leq H(p).
\]

The implementation uses Cholesky solves against the full covariance and stable log-sum-exp arithmetic. It reports prior entropy in nats, mutual-information lower bound in nats, normalized lower-bound fraction of prior entropy, all pairwise Bhattacharyya distances, and worst-case injected mass, inlet rate, inlet slew and experiment duration.

This is a *lower bound*, not an exact mutual-information estimate, a posterior distribution, a probability of contamination or a confidence interval.

## 4. Guard against information/identifiability confusion

A Bayesian model-discrimination score can be high even when the requested physical parameters cannot be separately estimated. We therefore retain an independent parameter-identifiability gate.

For each prior particle, the evaluator propagates two declared target-parameter kernel directions through that particle's actuator and sensor responses, forms its local Fisher information under the same full covariance, and checks rank and minimum singular value. A candidate is eligible only when **every** prior particle passes:
- full rank for both target parameters;
- the declared minimum singular-value floor;
- the Bayesian mutual-information lower-bound floor.

A high MI lower bound cannot rescue a rank-deficient candidate. This is intentionally stricter than maximizing model-separation information alone.

The evaluator also reports the prior-weighted average Fisher matrix and its log determinant as a comparator to the Bayesian lower-bound ranking.

## 5. Frozen eleven-candidate controls

The prior weights are 0.5, 0.3 and 0.2 (prior entropy 1.02965 nats). The three particles vary their kernel, actuator FIR and sensor FIR.

| Candidate | Bayesian lower bound | Robust rank | Result |
|---|---:|---:|---|
| Orthogonal pulses | 0.16433 nats | 2 | Eligible |
| Synchronous pulses | 0.30725 nats | 1 | Robustly unidentifiable |
| Low-dose orthogonal | 0.000696 nats | 2 | Insufficient Bayesian information |
| Zero excitation | 0 nats | 0 | Robustly unidentifiable |
| Excessive dose | 0.66634 nats before constraint refusal | 2 | Operational violation |

The synchronous sequence is the critical negative control: its MI lower bound is nearly twice that of the orthogonal sequence, but it cannot identify two inlet-specific parameters. It is not selected. The orthogonal sequence is the only eligible candidate in this frozen corpus.

These are synthetic mirror/oracle results, not measurements or achieved physical performance.

## 6. Selection policy

Only ELIGIBLE_BAYESIAN_DESIGN candidates enter the Pareto frontier. The frontier maximizes the MI lower bound while minimizing worst-case delivered mass and duration. The deterministic selector chooses the Pareto candidate with the largest MI lower bound, then lower worst-case mass.

All candidates are refused before information scoring if they have command-only provenance, uncalibrated injector or sensor response, missing inlet/outlet observation, incomplete temporal tail, insufficient sensor bandwidth, excessive dose, excessive flow/slew, or excessive duration.

## 7. Comparators and benchmark programme

### Baselines

Evaluate under the *same* prior particles, candidate waveforms, full covariance and operating constraints:
1. local reference-kernel D/E-optimal design from #5320;
2. prior-weighted average-FIM / average D-optimal ranking reported by this T0;
3. mutual-information lower bound from this T0;
4. pairwise model-discrimination-only ranking (diagnostic, not sufficient for parameter eligibility);
5. random, greedy and fixed pulse/step candidates.

### Proposed targets (not achieved)

- 100% frozen-oracle agreement and zero invalid prior/covariance contracts accepted.
- Zero rank-deficient prior-particle cases selected, regardless of MI lower-bound score.
- No over-limit experiment receives selection credit, even if its information score is highest.
- Demonstrate candidate-rank stability across geometry, flow-ratio, injector and sensor-response prior strata.
- At equal delivered dose/time/sensor set, improve prior-weighted blind parameter prediction and interval coverage relative to local FIM-only design.
- Publish score-versus-dose and score-versus-duration Pareto curves, prior particle effective sample size, entropy fraction, worst-particle singular spectrum, average-FIM logdet, prediction error, abstention rate, runtime and memory.
- For blind physical T1 runs, preregister 90% predictive-interval coverage as a target and report empirical coverage by geometry/flow/calibration stratum.

## 8. Limitations / next increments

- The prior is finite and discrete. Its particle discretization and prior misspecification errors are not quantified.
- All particles share one observation covariance. Particle-dependent covariance and uncertain noise spectra require a separate derivation and validation.
- The MI lower bound may be conservative and need not tightly approximate exact MI. The published method itself notes that the bound is not always tight and depends on suitable prior nodes/weights.
- Two target kernel directions and one fixed measurement time grid are used in v0.1.
- The local FIM gate still uses local target directions; future robust design should propagate parameter uncertainty across nonlinear model particles and evaluate posterior parameter recovery.
- Injector/sensor FIR particles encode assumed prior uncertainty, not measured calibration evidence. Physical fitting must use independently measured inlet mass flux.

## 9. Safety and evidence boundary

This T0 is synthetic arithmetic only. A physical T1 campaign requires a facility-approved closed-loop non-potable rig, approved tracer and disposal plan, calibration traceability, synchronized clocks, independently measured inlet mass flux, all relevant outlet sensors and full physical/sensor tail capture. Do not inject tracer into a live drinking-water system under this research item.

No output qualifies a physical junction, tracer, sensor, water network, contamination-warning system, potability, toxicology, SANS 241 compliance or release authority.
