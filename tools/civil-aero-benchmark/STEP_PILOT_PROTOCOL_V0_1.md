# STEP7 / STEP24 clean-room pilot protocol v0.1

## Scope and evidence ceiling

This pilot implements two formula-level tasks from the pinned STEP suite: STEP7 C2-DTLZ2 (M=2, D=5) and STEP24 C1-DTLZ1 (M=3, D=5). It tests objective/constraint definitions, variable bounds, constraint sign convention and independent replay. It does not implement an optimizer, reproduce AUC-ERD, qualify any baseline, or demonstrate superiority. The reference manifest covers 30 task mappings, but only two formulas are implemented here. External GPL-3.0 code and external .pf data remain outside this crate.

## Constraint convention

The Rust pilot exposes a signed constraint margin: nonnegative means feasible. Aggregate violation is the sum of max(-margin, 0), but is not itself a feasibility certificate.

### STEP24: C1-DTLZ1

For M=3, D=5, k=D-M+1=3. The standard DTLZ1 distance term is:

g = 100 * (k + sum((x_i - 0.5)^2 - cos(20*pi*(x_i - 0.5))))

where the sum covers the final D-M+1 variables. Objectives:

f1 = 0.5*(1+g)*x1*x2
f2 = 0.5*(1+g)*x1*(1-x2)
f3 = 0.5*(1+g)*(1-x1)

Published C1-DTLZ1 feasible-positive margin:

margin = 1 - f3/0.6 - (f1+f2)/0.5

### STEP7: C2-DTLZ2

For M=2, D=5, base objectives are spherical DTLZ2:

g = sum((x_i - 0.5)^2) over the final D-M+1 variables
f1 = (1+g)*cos(pi*x1/2), f2 = (1+g)*sin(pi*x1/2)

The feasible set is the union of radius-r balls around the M unit axes and the centre (1/sqrt(M), ..., 1/sqrt(M)), with r=0.1 for this instance. The code uses:

margin = r^2 - min(distance_squared_to_each_axis, distance_squared_to_centre)

so nonnegative margin means feasible. A text-extracted version of one supplementary inequality can be read as max where its accompanying figure and independent formulations describe a union of balls (min of distances). This pilot follows the union-of-balls interpretation and discloses the ambiguity rather than hiding it. The objective and constraint formula should remain subject to independent reference-oracle confirmation before other STEP tasks are added.

## Known-answer and negative controls

- STEP7 with tail variables all 0.5 and x1 in {0, 0.5, 1} must lie in three distinct feasible components with margin +0.01.
- STEP7 x1 in {0.25, 0.75}, with tail variables 0.5, must fall into gaps and be infeasible.
- A tail perturbation away from 0.5 increases g and is checked independently.
- STEP24 [0.2, 0.3, 0.5, 0.5, 0.5] has g=0, objectives [0.03, 0.07, 0.4] and positive margin 2/15.
- STEP24 [0.2, 0.3, 0, 0.5, 0.5] activates a positive multimodal distance penalty and is infeasible.
- Rust unit tests and the separate Python oracle replay objective values, margins, feasibility signs and row identities.

## Next gate

Before using either formula in the full STEP lane, independently verify formula/sign against another source or a separate licensed oracle, including boundaries. Implement and validate all 30 task definitions incrementally. Algorithm comparisons require at least 30 paired seeds, feasible-only HV/IGD+, time-to-first-feasible, false-feasible rate, complete objective/constraint/high-fidelity cost accounting and family-clustered statistics.
