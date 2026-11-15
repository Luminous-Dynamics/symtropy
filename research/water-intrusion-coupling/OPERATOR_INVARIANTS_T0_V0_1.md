# Positive and Mass-Accountable Mixing Operator Invariants T0 v0.1

**Programme:** WATER-MIXING-PHYSICAL-OPERATOR-INVARIANTS-001 / [#5294](https://github.com/Luminous-Dynamics/symtropy/issues/5294)  
**Dynamic operator:** [#5274](https://github.com/Luminous-Dynamics/symtropy/issues/5274), draft [#5288](https://github.com/Luminous-Dynamics/symtropy/pull/5288)  
**Evidence class:** synthetic invariant checker  
**Evidence ceiling:** no physical system identification or operator qualification

## 1. Motivation: constrain the model class before fitting

A dynamic junction model should not be allowed to produce impossible tracer behavior and then repair it with post-hoc clipping or normalization.

For a passive conservative tracer at fixed geometry and unchanged hydraulic state, the local operator is a positive causal input/output map. Its accepted model class should preserve:
- no response before an inlet excitation;
- no negative physical mass flux from a nonnegative input;
- no integrated output mass gain greater than the admitted input mass unless an explicit source is modeled;
- steady outlet transfer consistent with the integrated impulse response;
- linear superposition while the velocity/operating state is unchanged;
- time-shift equivariance for a time-invariant fixed-regime operator;
- port-permutation and unit-scale invariance;
- convergence of the discrete kernel's integrated area and temporal centroid under grid refinement.

These are conditional properties. They are not claimed across changing hydraulics, flow reversal, reactive chemistry, density-coupled species, or a regime transition. Those require a separate state transition model and corresponding experiments.

## 2. Positive impulse-response representation

For a fixed hydraulic regime, discretize the causal local operator as:

\[
y_j[k]=\sum_i\sum_{\ell=0}^{L-1} h_{ji}[\ell]u_i[k-\ell]
\]

where:
- \(u_i[k]\) is inlet tracer mass flux for inlet i;
- \(y_j[k]\) is predicted outlet tracer mass flux for outlet j;
- \(\ell\geq0\) is causal lag;
- \(h_{ji}[\ell]\geq0\) is the discrete outlet/inlet kernel.

The steady gain is the integrated kernel:

\[
P_{ji}=\sum_{\ell=0}^{L-1}h_{ji}[\ell]
\]

For a passive conservative tracer and a fully declared outlet set, a column's total outlet gain cannot exceed one:

\[
\sum_jP_{ji}\leq1
\]

The inequality may be strict when the finite operator support leaves mass in a modeled inventory or unresolved residual. It must never be closed by normalizing the observed outlets. The T0 checker reports synthetic input mass not returned by the kernel; it does not attribute that difference to a physical storage mechanism.

The dynamic operator's zero-delay limit is a nested static transfer matrix. The benchmark must verify that P equals the kernel's integrated gain. If it does not, the operator is internally inconsistent and should be rejected before comparison with physical data.

## 3. State-space positive realization

A finite-dimensional continuous-time realization may be written:

\[
\dot{x}=Ax+Bu,\qquad y=Cx+Du
\]

A sufficient structural condition for a positive linear system is:
- A is Metzler (all off-diagonal entries nonnegative);
- B, C and D are elementwise nonnegative.

For a Metzler A, the existence of a vector \(v>0\) satisfying:

\[
Av<-\eta\mathbf{1},\quad \eta>0
\]

is a sufficient certificate that A is Hurwitz. This T0 checks that supplied witness inequality. It does not identify A/B/C/D from data, prove they fit a junction, or prove that their transfer function equals a separate discrete kernel.

The positive realization check is an optional structural witness in v0.1. A future increment should link the realization's impulse response and DC gain to the actual local operator kernel and mass balance, rather than treating two independently supplied artifacts as if they were already consistent.

## 4. Invariants checked by T0

1. **Kernel positivity:** coefficients below the declared negative tolerance are rejected.
2. **Causality:** any declared negative-lag coefficient outside tolerance is rejected.
3. **Mass passivity:** the integrated outlet gain per inlet cannot exceed one.
4. **Static-gain consistency:** the supplied steady P matrix must match the integrated kernel gain.
5. **Superposition:** the output for two input histories combined must equal the sum of their individual outputs under the same fixed kernel.
6. **Time-shift equivariance:** delaying the input by one sample must delay the output by one sample.
7. **Port-permutation equivariance:** reversing output-port labels and kernel rows must reverse output histories correspondingly.
8. **Positive mass scaling:** scaling input mass by a positive factor must scale predicted output by the same factor.
9. **Grid-refinement consistency:** coarse and fine kernel representations must retain integrated area and temporal centroid within declared tolerances.
10. **Positive-state-space witness:** Metzler A, nonnegative B/C/D and positive-vector stability witness.

This is an invariant checker, not an estimator. The checker assumes upstream kernel, P matrix and state-space artifacts have already been produced. A future constrained estimator should fit within this admissible class using nonnegative least squares, positive realization or other justified optimization methods and should propagate parameter uncertainty.

## 5. Frozen synthetic controls

The v0.1 corpus has ten cases:
- valid positive fixed-regime response with superposition/time-shift/port-permutation/mass-scaling controls;
- negative impulse-response coefficient;
- noncausal negative-lag coefficient;
- integrated mass gain above one;
- disagreement between steady P and integrated kernel;
- positive stable Metzler realization with nonnegative B/C/D and a positive-vector witness;
- non-Metzler state matrix;
- Metzler but unstable/uncertified state matrix;
- coarse/fine kernel with matching area and centroid;
- coarse/fine kernel with an unacceptable area/centroid discrepancy.

The valid case deliberately returns exactly the injected mass across its two test inputs. The sub-stochastic kernel remains allowed in principle; the test is that it does not silently create mass. Separate open-boundary accounting belongs to the mass ledger in [#5242](https://github.com/Luminous-Dynamics/symtropy/issues/5242).

## 6. Benchmark strategy

Research targets (not achieved):
- 100% agreement with the frozen invariant oracle;
- zero accepted negative physical output coefficients beyond declared numerical tolerance;
- zero false mass-gain/false-closure decisions in the critical corpus;
- exact static-gain consistency within declared uncertainty;
- fixed-regime superposition and time-shift invariance within numerical tolerance;
- positive state-space witness for all accepted linear realizations;
- blind 90% predictive interval coverage on physical transient tests, with interval width and abstention by geometry/regime;
- publish model order, calibration runs, sensor bandwidth, runtime, memory and total experiment cost.

Compare:
- unconstrained subspace/state-space fitting;
- unconstrained fit with post-hoc clipping (must not be allowed to conceal negative coefficients);
- nonnegative least-squares identification;
- positive subspace/NMF realizations;
- series/parallel compartment and tanks-in-series models;
- geometry-specific transient CFD;
- directly measured physical pulse/step response.

All comparators require identical input histories, sensor-response correction, train/calibration/holdout splits and physical mass boundaries. CFD is a hypothesis comparator, not ground truth.

## 7. Evidence ceiling

The T0 evaluator checks discrete synthetic arithmetic and supplied state-space inequalities only. It does not fit a model, validate the supplied sensitivity or measurements, identify a physical positive system, prove junction memory, qualify sensors, qualify a tracer, validate CFD, establish potability, identify contamination or toxicology, establish SANS 241 compliance, or authorize water release.
