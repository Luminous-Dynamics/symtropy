# Grid convergence and GCI protocol v0.1

## Purpose and limits

This protocol adds a numerical verification diagnostic for a scalar output on a systematically refined grid family. It implements an observed-order estimate, Richardson extrapolation and the Grid Convergence Index (GCI) on sliding triples of grids. The output is a numerical uncertainty estimate under the method's assumptions. It does not authenticate the solver, prove that the mesh family is systematically refined, estimate model-form error, validate against an experiment or establish aircraft safety.

The caller must provide ordered coarse-to-fine mesh identifiers, a positive characteristic spacing for each grid in consistent units, a disclosed method for deriving that spacing, one solution value for a named scalar quantity on each grid, and a safety factor with an explicit justification. For three-or-more-grid studies this implementation requires F_s >= 1.25, consistent with NASA's published recommendation; the factor remains recorded in every report. Node count alone is not characteristic spacing. Do not infer h from GRID_SIZE without a documented mesh-family geometry and a valid spacing definition.

## Method

For three grids coarse, medium, fine:
- r_cm = h_coarse / h_medium > 1
- r_mf = h_medium / h_fine > 1
- R = abs((phi_coarse - phi_medium) / (phi_medium - phi_fine))

Require both solution differences to be non-zero and have the same sign (monotonic convergence). The observed order p > 0 solves:

R = (r_cm^p - 1) / (1 - r_mf^(-p))

For equal refinement ratios, the equation reduces to p = ln(R) / ln(r). Otherwise the implementation brackets and bisects a positive solution. If no positive order is supported or a sequence is oscillatory/non-monotone, the module fails closed.

Richardson extrapolation from medium and fine grids:

phi_ext = phi_fine + (phi_fine - phi_medium) / (r_mf^p - 1)

Absolute medium/fine GCI estimates:

GCI_medium = Fs * abs(phi_coarse - phi_medium) / (r_cm^p - 1)

GCI_fine = Fs * abs(phi_medium - phi_fine) / (r_mf^p - 1)

Relative GCI percentages normalize the respective absolute estimate by abs(phi_medium) or abs(phi_fine). When a normalization value is zero, the relative metric is absent and the absolute GCI remains reported.

The asymptotic consistency ratio is GCI_medium / (GCI_fine * r_mf^p) using absolute GCI estimates and a common caller-supplied safety factor. Values near one are consistent with asymptotic behaviour; this is a diagnostic, not proof that all discretization or modelling errors are small.

For six grids, the implementation computes four consecutive three-grid windows. It does not collapse them into one universal order or uncertainty number.

## Known-answer validation

A synthetic function phi(h) = 2 + 0.8*h^2 on six spacings [4, 2, 1, 0.5, 0.25, 0.125] has exact order 2 and exact zero-spacing limit 2. Rust tests and an independent standard-library Python implementation must recover four sliding triplets, order 2 on every triplet, extrapolated limit 2, asymptotic ratio 1, and deterministic CSV output.

This test proves only implementation behaviour on an exact synthetic power law.

## Minimum real-case admission checklist

Before a DPW8 V1a result can use this method:
1. authenticate and retain exact solver configuration, executable/build, geometry, mesh and output bytes;
2. verify each grid is a consistent refinement of the same physical problem and record mesh family, dimensions, cell counts and characteristic-spacing method;
3. establish iterative convergence and conservation/consistency for every grid;
4. use at least three suitable systematically refined grids, preferably the full workshop-recommended six where available;
5. predeclare monitored quantities (CL, CD, CM, Cp stations/sections) and convergence criteria;
6. report each triplet, not just a selected favourable value;
7. investigate non-monotonic convergence rather than silently discarding grids;
8. compare outputs against the appropriate experimental data and its uncertainty separately from GCI.

## References

- NASA CFD verification and validation guidance: https://www.grc.nasa.gov/www/wind/valid/tutorial/verassess.html
- NIST IR 8298, Industrial Verification, Validation, and Uncertainty Quantification: https://nvlpubs.nist.gov/nistpubs/ir/2020/NIST.IR.8298.pdf
- Official AIAA DPW-8/AePW-4 test cases: https://www.aiaa-dpw.org/TestCases/testcases.html


NASA's spatial-convergence guidance recommends a safety factor F_s=1.25 for studies with three or more grids (and 3.0 for comparisons of only two grids). This implementation requires at least three grids and therefore fails closed for F_s<1.25. See https://www.grc.nasa.gov/www/wind/valid/tutorial/spatconv.html.
