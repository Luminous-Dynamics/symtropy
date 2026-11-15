# AFEX practical pathway identifiability and cost-aware measurement design v0.1

**Issue:** [#5464 — AFEX-PRACTICAL-DESIGN-001](https://github.com/Luminous-Dynamics/symtropy/issues/5464)  
**Structural prerequisite:** [#5449](https://github.com/Luminous-Dynamics/symtropy/issues/5449)  
**Composes with:** [#5360 practical identifiability](https://github.com/Luminous-Dynamics/symtropy/issues/5360), [#5223 decision value](https://github.com/Luminous-Dynamics/symtropy/issues/5223), [#5445 pathway tournament](https://github.com/Luminous-Dynamics/symtropy/issues/5445)  
**Date:** 15 November 2026  
**Status:** synthetic linear-Gaussian design benchmark; no AFEX assay optimum identified

## 1. The next paradigm shift

Structural identifiability asks whether noiseless observations can distinguish parameter combinations. Practical identifiability asks whether finite, noisy observations can estimate them precisely enough to be useful. These are not interchangeable.

Liu, Maini & Baker (2026) explicitly separate experiment design for practical parameter identifiability (profile likelihood) from design for model discrimination, and formulate the latter as an optimal-control problem: [DOI 10.1016/j.mbs.2026.109710](https://doi.org/10.1016/j.mbs.2026.109710).

A 2026 ecological sampling study shows that allocating samples between replication and environmental coverage is task-dependent: systematic coverage performed well when response shape was unknown, while replication could help when simple response shapes were known. It studies gradient designs, not AFEX's randomized factorial experiment, so we use it to motivate a synthetic trade-off benchmark, not to prescribe AFEX sample allocation: [DOI 10.1038/s41598-026-38541-4](https://doi.org/10.1038/s41598-026-38541-4).

A 2026 review of ecological calibration, sensitivity and uncertainty highlights equifinality, parameter trade-offs, structural uncertainty and the need to explore response surfaces rather than relying only on a point optimum: [DOI 10.1111/ele.70375](https://doi.org/10.1111/ele.70375).

## 2. Synthetic measurement model

Use the four pathway parameters from #5449. The base design contains three total-uptake observations across synthetic AM-colonisation fractions 0.25, 0.50 and 0.75. Rows are scaled by four to remain integer-valued:

`[3,1,1,0], [2,2,2,0], [1,3,3,0]`

Each has synthetic observation standard deviation 0.8 in the scaled units. The base design has exact data-only rank 2.

For independent Gaussian observation errors, data Fisher information is:

`F = Σ_j (a_j a_jᵀ / σ_j²)`

With prior covariance `C0`, the linear-Gaussian posterior covariance is:

`C_post = (C0⁻¹ + F)⁻¹`

Report them separately:
- **data-only rank/Fisher:** what the experiment contributes;
- **posterior covariance:** what remains after both experiment and prior assumptions;
- **information gain:** `0.5 log(det(C0)/det(Cpost))`, in nats for the Gaussian linear approximation;
- **decision-weighted variance:** a declared diagonal loss proxy, not an ecological decision until #5223 supplies a real loss function.

A tight prior must never be reported as data identification.

## 3. Synthetic experiment-design result

Under a synthetic total budget of 24 effort units (base design costs 6), the fixture enumerates all candidate-measurement subsets and returns the cost–uncertainty Pareto frontier.

The minimum weighted posterior variance in this declared synthetic scenario is obtained by adding:
- one mycorrhizal inorganic source-specific tracer;
- one host-to-fungal carbon-allocation tracer.

That portfolio costs 24 synthetic effort units, reaches data-only rank 4, and has weighted posterior variance 0.28047. This is **not** a real field recommendation: costs, measurement noise, sensitivity rows, priors and decision weights are invented for the test fixture. A mycorrhizal organic tracer plus the carbon tracer is also a full-rank alternative, with a slightly higher synthetic decision-weighted risk.

The result illustrates a useful principle: measurement selection depends on which parameter combinations matter to the decision. A generic “measure the most uncertain variable” heuristic can be inferior to a pathway-specific measurement that breaks a structurally meaningful alias.

## 4. Adversarial tests

- Same structural rank, different tracer precision: high-noise fungal tracer leaves the corresponding pathway parameter practically weak even though rank rises from 2 to 3.
- Repeating total uptake improves aggregate precision but does not separate fungal inorganic/organic uptake.
- Repeating AM colonisation contributes no direct flux Fisher information in this fixed-`f` four-rate model. It may improve uncertainty in `f`, which this simplified operator does not yet propagate.
- An artificially tight prior makes posterior standard deviations small while the data-only rank remains 2. This must not be called experimental identification.
- The budgeted optimizer must return the Pareto frontier, not hide the cost/precision trade-off behind one score.

## 5. Scope and integration

The Rust validator composes with #5449's exact integer rank function, calculates a bounded four-parameter Fisher/posterior calculation, enumerates a six-package design set, and checks the expected Pareto portfolio. It is not a general Bayesian engine or a substitute for profile likelihood, bootstrap, hierarchical/block uncertainty or #5223 decision value.

The actual AFEX design has four blocks and 32 plot-level treatment units. The current four-pathway matrices are synthetic and do not use AFEX field flux measurements. A field translation must model plot/block dependence, covariate uncertainty in AM colonisation, depth mismatch, assay-specific error, practical field costs and model discrimination between mechanism alternatives.

## 6. Claim ceiling

Every prior, cost, noise level, sensitivity coefficient and utility weight in the fixture is synthetic. The optimum is conditional on that synthetic objective and budget. No AFEX parameter is practically identified, no field assay price/performance is established, no real experimental allocation is recommended, and no ecological mechanism is selected.
