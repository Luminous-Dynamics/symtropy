# Global black-hole admissibility protocol: independent regularity gates

**Purpose:** replace a single “regular black hole” boolean with a proof-carrying vector of independent geometric, analytic, thermodynamic and physical outcomes.

## Gate outcomes

Every gate returns exactly one of:
- **Pass:** the specified property is established over its declared domain and with stated conventions.
- **Fail:** an explicit counterexample or non-integrable/singular behavior is established.
- **Unresolved:** evidence is incomplete or inconclusive. Unresolved is neither Pass nor evidence of nonexistence.

Never promote one gate's result into another.

## Required independent gates

1. **Theory and source identity:** action, coupling normalization, dimension, signature, Riemann convention, ansatz, and parameterization are pinned.
2. **Field equations:** full theory field equations or independently derived reduced equations are evaluated; constraints and Bianchi identities remain acceptance conditions.
3. **Lorentzian signature:** metric non-degeneracy and inertia are checked in regular frames, including all off-diagonal blocks.
4. **Local curvature:** declared scalar invariants and, where needed, frame components are bounded over the domain. A sampled grid alone is not a full-domain proof.
5. **Local axes/orbits:** each shrinking angular orbit has the correct period and no conical/orbifold defect.
6. **Global identifications:** all non-shrinking angular cycles, bundle data, topology and periodicities are specified by a compatible global completion.
7. **Compactness and measure:** horizon cross-sections are compact and their induced measure has finite total volume. Prove integrability, not merely pointwise smoothness of curvature.
8. **Thermodynamic/charge integrability:** Wald entropy and covariant phase-space charges are finite, integrable and independent of admissible integration surface; verify first-law/Smarr relations only after these prerequisites.
9. **Physical characteristics:** physical polarization cones, gauge/constraint quotient and characteristic eigenbasis are qualified independently of curvature.
10. **Evolution well-posedness and stability:** strong hyperbolicity of a declared gauge-fixed PDE system and mode stability are separate outcomes.
11. **Global exterior:** integrate/match to the requested asymptotics, demonstrate a regular horizon and exclude naked singularities/branch endpoints.
12. **EFT/physical domain:** expansion parameter, omitted operators, observer-frame tidal quantities and observational regime are controlled where applicable.

## Adversarial cross-gate test cases

- Finite curvature with divergent horizon area: curvature Pass, finite measure Fail, thermodynamic integrability Fail/Unresolved.
- Lorentzian signature but no pinned global angular period: signature Pass, global identifications Unresolved.
- Smooth local horizon data with no global exterior: local gates may Pass, global exterior Unresolved.
- Finite scalar invariants but diverging freely falling tidal components: scalar-invariant gate Pass, observer-frame validity Fail.
- Real characteristic roots but defective/incomplete eigenvectors: real-root diagnostic Pass, strong hyperbolicity Fail.
- Tiny collocation residual with unresolved spectral tail: sampled residual Pass, approximation qualification Unresolved.

## Benchmark reporting

Publish a vector/status table for every solution candidate; include input/action digest, chart and domain, exact bounds, independent implementation, replay receipt, and reason for every Fail or Unresolved outcome. Report Pareto trade-offs across branch coverage, residual/error, certified domain, compute cost and proof cost. Do not compress the vector into a weighted aggregate score that hides a failing gate.

## Claim language

Use precise labels such as “source-reported local solution,” “independently field-equation replayed,” “Lorentzian on the declared patch,” “curvature bounded,” “finite-area horizon,” “charges integrable,” “strongly hyperbolic in formulation X,” and “global asymptotically-flat branch.” “Regular black hole” is permitted only when the intended gates and global scope are explicitly stated.
