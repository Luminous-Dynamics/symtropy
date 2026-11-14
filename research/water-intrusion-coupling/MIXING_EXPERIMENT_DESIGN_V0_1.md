# Mixing Experiment Identifiability and Sensor Design v0.1

**Parent qualification:** [WATER-MIXING-FIDELITY-QUAL-001 / #5216](https://github.com/Luminous-Dynamics/symtropy/issues/5216)  
**Implementation proposal:** [draft PR #5220](https://github.com/Luminous-Dynamics/symtropy/pull/5220)  
**Evidence class:** analytical design contract and literature synthesis; no physical experiment has been run.

## 1. Design for identifiability, not sensor count

A sensor layout can have broad spatial coverage while leaving the parameters of interest unidentifiable. Conversely, several redundant sensors can add cost without adding useful information. The experiment must identify the junction transfer operator or tank RTD under stated noise, calibration and flow conditions.

This is related to, but distinct from, the established water-network sensor-placement problem. Published information-theoretic approaches use value of information and transinformation entropy to rank locations and reduce redundant information. A recent 2026 review describes the placement problem as multi-objective, with cost, coverage, detection, uncertainty and practical constraints. Our extension is to optimize the **identifiability of local mixing physics and competing mixing hypotheses**, not only contamination detection or leak localization.

References:
- Khorshidi, Nikoo & Sadegh (2018), *Optimal and objective placement of sensors in water distribution systems using information theory*, Water Research 143, 218–228, DOI [10.1016/j.watres.2018.06.050](https://doi.org/10.1016/j.watres.2018.06.050). PubMed record: https://pubmed.ncbi.nlm.nih.gov/29960176/
- Yao, Wallner & Klawonn (2026), *Optimal Sensor and Sampling Placement for Contaminant Detection: A Comprehensive Review Across Water Distribution and Wastewater Collection Systems*, Eng 7(3), 121, DOI [10.3390/eng7030121](https://doi.org/10.3390/eng7030121).

## 2. Junction transfer-matrix identifiability

Let there be (n_i) inlets and (n_o) outlets. Define (P\in\mathbb{R}^{n_o\times n_i}), where (P_{ji}) is the fraction of conservative tracer mass flux entering at inlet (i) that leaves through outlet (j).

For experiment (r), let (u_r\in\mathbb{R}^{n_i}) be the measured inlet tracer mass-flux vector, and (y_r\in\mathbb{R}^{n_o}) the measured outlet tracer mass-flux vector:

[
y_r=P u_r+\epsilon_r
]

Stacking experiments gives (Y=PU+E), where (U=[u_1,\ldots,u_R]). If (P) is not further parameterized, a necessary condition for unique identification of all inlet columns is:

[
\operatorname{rank}(U)=n_i
]

With noiseless, fully observed outlet fluxes and full row rank (U), the least-squares estimate is (\hat P=YU^T(UU^T)^{-1}). In a real campaign use a constrained, uncertainty-weighted estimator that enforces non-negative, column-stochastic fractions, propagates flow/concentration uncertainty, and reports residuals rather than silently projecting poor fits into validity.

Each inlet's tracer input must therefore be independently excited. Sequential pulses at individual inlets are the simplest design where use of one tracer is appropriate. Distinct temporal codes are possible if their input signals remain linearly independent after accounting for dispersion, sampling and sensor response. If every inlet is injected with the same proportional tracer pattern, (U) is rank-deficient; a unique mixing matrix cannot be recovered.

### Minimal 2-inlet, 2-outlet example

Suppose the unknown operator is:

[
P=\begin{bmatrix}0.8&0.2\\0.2&0.8\end{bmatrix}
]

Use two independently excited experiments:

[
U_{\mathrm{identified}}=\begin{bmatrix}1&0\\0&1\end{bmatrix}
\quad\Rightarrow\quad
Y=PU_{\mathrm{identified}}=P
]

By contrast, a single simultaneous equal excitation gives:

[
U_{\mathrm{ambiguous}}=\begin{bmatrix}1\\1\end{bmatrix}
\quad\Rightarrow\quad
Y=PU_{\mathrm{ambiguous}}=\begin{bmatrix}1\\1\end{bmatrix}
]

That same observation is produced by every 2×2 column-stochastic operator, including complete segregation, partial mixing and complete mixing. The experiment is structurally incapable of distinguishing them. More repeats of the same proportional excitation improve precision along the observed direction but do not recover the missing degree of freedom.

This is a hard preflight condition: the design evaluator must report **UNIDENTIFIABLE_EXCITATION_RANK** before a fitted operator can be reported.

## 3. Minimum observation topology

For a conservative junction mass-routing campaign:

- Measure inlet flow and inlet tracer concentration/time history at every inlet.
- Measure outlet flow and tracer concentration/time history at every outlet.
- Synchronize all channels and characterize clock offsets, sample-line delay, sensor response time, resolution and detection limits.
- Recover and report tracer mass at every measured outlet; retain an explicit unresolved residual if the measurement plane does not close.
- Repeat the excitation under the same declared geometry and flow regime; then deliberately vary the flow split/momentum ratio in a separate factor.
- Hold out entire geometry or flow-regime families from calibration.

Missing an outlet measurement may make its mass fraction algebraically inferable only by imposing exact closure. That is not sufficient physical evidence where leakage, unobserved storage, adsorption or sampling losses are possible. Therefore a physically measured operator needs observation coverage for every outlet and an independent mass-recovery assessment.

## 4. Tank RTD experimental design

For a tank, record the **actual input waveform and mass**, not just the nominal injector command. Collect synchronized outlet tracer concentration and volumetric flow, so each outlet's tracer mass flux is observable. A single outlet RTD characterizes only that outlet response under the tested fill/flow schedule; it does not establish the complete internal concentration field or a universal tank model.

Use a pulse input for RTD estimation and, where useful, a step input to check the inferred response. Repeat across relevant tank levels, inlet/outlet configurations and operating schedules. Stratification hypotheses require vertical or multi-depth observations; a single outlet sensor cannot verify them. Preserve late samples long enough to resolve the tail, and report unrecovered tracer mass rather than normalizing it away.

Sampling frequency is selected from the shortest feature the trial claims to resolve, the sensor bandwidth and response time, the tank residence-time scale and uncertainty. There is no universal sampling rate that is correct for every tank or transient.

## 5. Cost-aware information design

For a parameterized model (y=f(\theta,d)+\epsilon), where (d) encodes injection patterns, flow setpoints, sensor locations and sample cadence, local information is summarized by:

[
F(d)=J_\theta(d)^T\Sigma(d)^{-1}J_\theta(d),\qquad
J_\theta=\frac{\partial f}{\partial\theta}
]

Candidate design objectives:

- **D-optimal:** maximize (\log\det(F(d)+F_0)) to reduce joint parameter uncertainty (with a declared prior-information term (F_0), or a constrained estimable subspace).
- **E-optimal:** maximize the smallest supported information eigenvalue to avoid a poorly observed parameter direction.
- **Model discrimination:** maximize expected separation between competing model predictions under the measurement noise model, rather than choosing sensors that all models predict similarly.
- **Robust design:** optimize expected or worst-case information across plausible flow, geometry, calibration and noise scenarios.
- **Budget frontier:** report information gain and identifiability against sensor count, calibration burden, installation/maintenance cost, tracer/test time and compute.

Do not collapse those objectives into one unexplained weighted score. Publish the trade-offs, parameter correlations, singular values/condition number, and the designs that remain unidentifiable.

## 6. Pre-registration and holdout controls

Before test execution, freeze:
- target parameters and competing hypotheses;
- candidate injection codes / sequence and declared operating regimes;
- measurement locations, sample interval, sensor bandwidth and calibration state;
- uncertainty model and minimum rank/singular-value threshold;
- acceptance/rejection rule and missing-stream treatment;
- calibration runs and blind holdout geometry/conditions;
- raw-data custody and output digests.

A rank test on (U) is necessary but not sufficient: outlet observability, measurement noise, sensor cross-sensitivity, time alignment, mass recovery and parameter conditioning still matter. Report numerical rank with a stated scale-aware threshold and the singular spectrum, not a bare boolean.

## 7. Proposed future executable T0/T1 checks

The next evaluator increment should add:
1. a full-rank, independently excited inlet design that identifies a known transfer matrix;
2. a rank-deficient equal-excitation negative control that must abstain;
3. a noisy near-collinear excitation that reports poor conditioning;
4. an omitted-outlet control that blocks physical operator qualification;
5. an RTD input-waveform-missing case that abstains;
6. a redundant-sensor case where extra sensors add little independent information;
7. a budget-constrained design comparison with an explicit information/cost Pareto set.

These are not yet implemented as fixtures or executable checks. The equations above define the intended contract for that work.

## 8. Nonclaims

This note is analytical design, not a physical result. It does not infer an operator for any real junction or tank, establish instrument requirements for a particular asset, or authorize potable-system tracer injection. A physical campaign requires a competent operator, an approved non-potable loop, tracer compatibility/safety review, calibration, raw-data custody and a waste/disposal plan.
