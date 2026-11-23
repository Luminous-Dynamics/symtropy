# Orbital Observation-Process Identifiability Benchmark v0.1

**Status:** synthetic mathematical reference implementation; exact-head hosted validation pending.  
**Owner:** [#6722 — observation-process identifiability](https://github.com/Luminous-Dynamics/symtropy/issues/6722).  
**Consumes:** #6659 capture observability / HIL POD evidence, #6617/#6618 model admissibility, #6636 PLP diagnostics and #6598 typed claim composition.  
**Claim ceiling:** synthetic independent-thinning PLP identifiability only.  
**Physical reliability / sensor qualification / mission authority claims:** always false.

## The identifiability problem

A reliability estimator sees the events admitted to the observed event history. It does not directly observe physical events that the sensor missed upstream.

Assume the physical event process is a power-law process (PLP), with intensity:

\[
\lambda_{\mathrm{phys}}(t)=\eta_{\mathrm{phys}}\beta_{\mathrm{phys}}t^{\beta_{\mathrm{phys}}-1},
\qquad t>0.
\]

Assume each physical event is independently detected with probability:

\[
p(t)=p_0(t/T)^\gamma,\qquad 0<p_0\le1,\quad \gamma\ge0,\quad 0<t\le T.
\]

This is a restricted independent-thinning model: the detection decision is conditionally independent for each event given its time, p(t) is deterministic within the declared exposure window, and no false-positive events are added. It does not model event-history-dependent dead time, burst-correlated loss, mark-dependent POD, sensor saturation, adaptive thresholds or shared-failure reference paths.

Independent thinning gives:

\[
\begin{aligned}
\lambda_{\mathrm{obs}}(t)
&=p(t)\lambda_{\mathrm{phys}}(t)\\
&=\eta_{\mathrm{phys}}p_0(t/T)^\gamma
  \beta_{\mathrm{phys}}t^{\beta_{\mathrm{phys}}-1}\\
&=\eta_{\mathrm{phys}}p_0
  \frac{\beta_{\mathrm{phys}}}{\beta_{\mathrm{phys}}+\gamma}
  T^{-\gamma}
  (\beta_{\mathrm{phys}}+\gamma)
  t^{\beta_{\mathrm{phys}}+\gamma-1}.
\end{aligned}
\]

Therefore the observed history is itself exactly a PLP, with:

\[
\boxed{\beta_{\mathrm{obs}}=\beta_{\mathrm{phys}}+\gamma}
\]

and:

\[
\boxed{
\eta_{\mathrm{obs}}=
\eta_{\mathrm{phys}}p_0
\frac{\beta_{\mathrm{phys}}}{\beta_{\mathrm{phys}}+\gamma}
T^{-\gamma}
}.
\]

This is the key result. A time-varying detection probability can imitate a physical reliability trend while leaving the observed process perfectly inside the PLP family. A better PLP goodness-of-fit statistic cannot detect this confounding: the two models generate the same observed point-process law under the stated assumptions.

## Important special cases

**Constant independent thinning:** gamma = 0. Then beta_obs = beta_phys and eta_obs = p0 eta_phys. The timestamp shape is preserved, but the physical scale is confounded with baseline detection probability.

**Increasing detection probability:** gamma > 0. The observed beta is larger than the physical beta. A sensor whose probability of capture improves with time can make the observed failure process appear to deteriorate more slowly or improve less than it actually does.

**Decreasing detection probability:** the power-law POD profile above restricts gamma to nonnegative values. A decreasing detection trend requires a different model domain and is a future extension.

## Exact observational equivalence

For fixed observed parameters (eta_obs, beta_obs), every admissible pair (p0, gamma) with 0 < p0 <= 1 and 0 <= gamma < beta_obs induces a physical PLP:

\[
\beta_{\mathrm{phys}}=\beta_{\mathrm{obs}}-\gamma
\]

and:

\[
\eta_{\mathrm{phys}}=
\eta_{\mathrm{obs}}
\frac{\beta_{\mathrm{obs}}}{\beta_{\mathrm{phys}}}
\frac{T^\gamma}{p_0}.
\]

Substituting these values into the thinning equations reproduces exactly the same eta_obs and beta_obs.

Thus one observed event history can be explained by many different physical beta values and detection-trend values. The ambiguity is structural, not a finite-sample estimation error. More events reduce sampling uncertainty around beta_obs; they do not separate beta_phys from gamma.

The benchmark constructs these equivalent decompositions and checks that they reproduce both observed PLP parameters, not just the shape.

## Sensitivity bounds from external POD evidence

Suppose an independent HIL / metrology programme bounds the detection-trend parameter:

\[
\gamma\in[\gamma_{\min},\gamma_{\max}].
\]

Then the physical shape is restricted to:

\[
\beta_{\mathrm{phys}}\in
[\beta_{\mathrm{obs}}-\gamma_{\max},
 \beta_{\mathrm{obs}}-\gamma_{\min}]
\cap(0,\infty).
\]

If gamma_max reaches or exceeds beta_obs, the lower endpoint is open at zero. It is not valid to report a closed interval whose lower physical beta is zero, because beta_phys must remain positive.

If the external gamma interval collapses to a point, physical beta is identified conditional on this restricted thinning model. If the interval has nonzero width, physical beta remains set-identified, not point-identified.

The external detection evidence must be scoped to the same:
- sensor / transducer and mounting;
- firmware, acquisition configuration, gain, filter and event detector;
- event class and measurand definition;
- amplitude, duration, waveform and rate regime;
- environment and operating mode;
- calibration validity and uncertainty;
- reference-system independence and common-cause review.

A scalar POD estimate from a different event class or configuration cannot bound gamma for the current event stream.

## Why inverse-probability correction is not automatic

Dividing every observed event by a modelled p(t), or applying inverse-probability weights, is not a general fix.

Such correction requires a validated, externally supported detection-probability model and the assumed conditional independent thinning structure. Where p(t) is very small, inverse weighting has high variance and the physical process may be practically unobservable. If detection depends on amplitude, pulse width, overlap, rate, temperature or event history, the simple one-dimensional p(t) model is misspecified.

The v0.1 policy is therefore:
- use independently measured POD evidence to bound the plausible gamma region;
- report the resulting physical-beta sensitivity interval;
- preserve OBSERVATION_PROCESS_UNIDENTIFIED or OBSERVATION_BIAS_POSSIBLE where external evidence is absent or too weak;
- do not let the reliability estimator infer or self-authorize its own missing-event correction.

## Executable benchmark

The standalone Rust crate implements:
1. the exact observed PLP shape / scale transformation;
2. the inverse family of physically distinct but observationally equivalent decompositions;
3. physical-beta sensitivity intervals from an externally supplied gamma interval;
4. deterministic thinning simulations using separate PRNG streams for physical event generation and capture decisions;
5. a strict frozen corpus, receipt CLI, JSON Schemas and an independently implemented JavaScript oracle.

The simulator generates normalized physical event times from the conditional PLP law:

\[
U_i\sim\mathrm{Uniform}(0,1),\qquad
t_i/T=U_i^{1/\beta_{\mathrm{phys}}},
\]

then independently retains each event with probability p(t_i). Conditional on the retained event count, the observed timestamps follow the PLP shape beta_phys + gamma. The two random streams are separately seeded to avoid introducing artificial dependence between physical event generation and detector decisions.

## Frozen synthetic corpus

The v0.1 corpus includes:
- constant thinning, which preserves beta but changes eta;
- time-varying thinning, which changes observed beta;
- a detection trend that masks an improving physical process;
- physical deterioration combined with a separate detection trend;
- narrow and broad externally bounded gamma intervals;
- a gamma interval whose lower physical-beta endpoint is open at zero;
- an exact gamma bound that identifies physical beta conditional on the assumed model;
- deterministic physical-PLP-plus-thinning simulations with independent random streams.

The synthetic oracle checks the analytic identities, the inverse decompositions, the sensitivity intervals and the simulation's recovery of the theoretical observed PLP shape. Agreement proves only the declared mathematics and simulator implementation; it does not establish a real sensor's POD curve or physical reliability.

## HIL and metrology interface

The physical evidence path is described in [draft HIL protocol #6715](https://github.com/Luminous-Dynamics/symtropy/pull/6715), which extends the #6256 ground test cell and is owned jointly with #6659.

The required sequence is:
1. independently characterize the reference instrument and DUT calibration scope;
2. qualify the DAQ boundary with electronic injection (downstream path only);
3. physically stimulate the actual sensor / mounting and measure the measurand with independent reference metrology;
4. measure POD separately across the preregistered event class, amplitude, duration, rate, environment and concurrency strata;
5. estimate an uncertainty region for the time-dependence of POD (and, in future versions, for mark-/rate-dependent POD);
6. feed that external region into the physical-beta sensitivity calculation;
7. preserve INDETERMINATE when the POD evidence does not bound the observation process sufficiently.

The capture evaluator #6676 currently checks synthetic reference/observation reconciliation. It is not yet a physical POD estimator or authenticated HIL receipt producer. The HIL profile schema and protocol in #6715 are proposed planning artifacts, not executed tests.

## Standards and research comparators

- **NIST Technical Note 2119** — probability-of-detection confidence intervals/bounds and false-alarm analysis from hit/miss detection data: [NIST publication](https://doi.org/10.6028/NIST.TN.2119).
- **ASTM E2862-23** — hit/miss POD analysis for a declared examination configuration, including model validation and interpretation: [ASTM standard](https://doi.org/10.1520/E2862-23). This is a probability-of-detection method comparator from nondestructive examination; it does not automatically make an aerospace sensor campaign ASTM-conforming.
- **Mottier et al. (2026)** — identifiability of thinned point-process observation models and the role of temporal replication / partial pooling: [arXiv:2610.29365](https://arxiv.org/abs/2610.29365).

The differentiator is the explicit end-to-end link between externally qualified sensor POD, physical-parameter sensitivity and event-history inference. The next competitive benchmark should compare:
- nominal and misspecified POD surfaces;
- structural and practical identifiability;
- sensitivity-interval coverage under true gamma;
- false physical-trend attribution when POD changes;
- estimator bias when a POD trend is ignored;
- stability and variance of any validated correction;
- cost and trial counts required to bound POD per stratum.

Do not claim superiority to NIST / ASTM methods without a matched POD simulation and physical-data comparison. These references are methodological comparators, not proof of standards compliance.

## Ownership and claim boundary

- #6722 owns the observation-process identifiability benchmark.
- #6659 owns event-capture completeness and POD/HIL evidence.
- #6715 is the proposed HIL campaign adapter to #6256.
- #6617/#6618 own model-input admissibility and should consume external POD bounds.
- #6636/#6683/#6717 own PLP diagnostics and cannot self-certify observation completeness.
- #6598 composes independently qualified evidence and cannot upgrade synthetic identifiability results into physical truth or authority.

No physical reliability, sensor qualification, flight readiness, mission authorization or HIL result is granted by this crate.

## Commands

~~~bash
cargo fmt --manifest-path tools/orbital-observation-process-identifiability-bench/Cargo.toml -- --check
cargo test --locked --manifest-path tools/orbital-observation-process-identifiability-bench/Cargo.toml --all-targets
cargo clippy --locked --manifest-path tools/orbital-observation-process-identifiability-bench/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path tools/orbital-observation-process-identifiability-bench/Cargo.toml -- \
  tools/orbital-observation-process-identifiability-bench/fixtures/v0.1/identifiability-cases.json \
  --source-commit <40-lowercase-hex-commit>
node tools/orbital-observation-process-identifiability-bench/tests/reference_oracle.mjs \
  tools/orbital-observation-process-identifiability-bench/fixtures/v0.1/identifiability-cases.json
~~~
