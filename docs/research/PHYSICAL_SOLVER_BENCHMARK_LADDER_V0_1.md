# Physical Solver Benchmark Ladder v0.1

**Purpose:** define how Symtropy's physical-transformation reference work progresses from reproducible software evidence to qualified model verification, external benchmark reproduction and finally physical validation.  
**Status:** proposed qualification ladder; the targets below are gates to earn, not achieved claims.

## The paradigm shift: evidence-carrying model classes

Do not ask one global question—"is the solver accurate?"—when the software contains different physics models. Every result should carry its model class, exact source/request digest, governing assumptions, independent oracle, residual diagnostics, domain limits and claim ceiling. Keep four kinds of evidence separate:

1. **Verification:** does the implementation solve the stated equations and numerical model correctly?
2. **Benchmark reproduction:** was a published problem reproduced with matching inputs, units, boundary conditions, element/mass formulation and observable?
3. **Validation:** does the chosen model represent the physical system, using observations and measurement uncertainty?
4. **Uncertainty/model discrepancy:** how does uncertainty in inputs, measurement, discretization and missing physics affect the result?

A pass in an earlier class does not automatically transfer to a later class. A deterministic receipt can prove what code and inputs were run; it cannot prove those inputs represent the real structure.

## Industry reference ladder

| Track | External reference | Published purpose and constraint | Symtropy status / next gate |
|---|---|---|---|
| Static truss | [SOFiSTiK BE1](https://docs.sofistik.com/2026/en/verification/index.html) | Joint deflection of a plane truss; static truss element | Use as a named external static case after the input archive and exact quantity of interest are frozen. Existing 2-bar oracle is not a reproduction of BE1. |
| Axial modal table | [NAFEMS Challenge 5](https://www.nafems.org/downloads/nbc5_challenge.pdf) and [published solution](https://www.nafems.org/downloads/nbr5v3.pdf) | The reported coarse CS1/CS2 axial-bar table can be independently derived; solution warns that axial-only modes are not the complete physical truss response | [PR #2556](https://github.com/Luminous-Dynamics/symtropy/pull/2556) passed its exact-head CI; the claim is limited to the reported two-DOF, single-element-per-member axial table. |
| Beam eigenfrequencies | [SOFiSTiK BE25 PDF](https://docs.sofistik.com/2026/en/verification/_static/verification/pdf/be25.pdf) | Four beam boundary conditions, a 10-element consistent-mass model and published analytical/numerical frequencies | Input audit in [PR #2659](https://github.com/Luminous-Dynamics/symtropy/pull/2659) intentionally blocks quantitative reproduction until the PDF's mass-per-length inconsistency is resolved against its input deck. |
| Beam modal model class | [NAFEMS Challenge 5 solution](https://www.nafems.org/downloads/nbr5v3.pdf) | Released-joint beam model includes bending modes and rotary-inertia choices; the source reports 22 beam modes below 1600 Hz versus two axial-bar modes and uses 200 elements/member for approximately 1% accuracy through that band | Local Euler-Bernoulli element and global transformation are draft implementation layers; private pin rotations and mode classification are being qualified separately. A complete sparse, subdivided mesh model is still required. |
| SDOF dynamics | [SOFiSTiK BE23 / BE24](https://docs.sofistik.com/2026/en/verification/index.html) | Undamped free vibration and under-critically damped free vibration, including time stepping | Add before claiming transient dynamics, damping or time-integration accuracy. |
| Geometric nonlinearity | [SOFiSTiK BE31](https://docs.sofistik.com/2026/en/verification/index.html) | Truss snap-through with geometric nonlinearity and load stepping | Separate future model family; the current linear, small-displacement truss code cannot claim this capability. |
| Solid-mechanics VVUQ | [ASME V&V 10-2019 (R2025)](https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-solid-mechanics/2019) | Current reaffirmed framework for verification, validation and uncertainty quantification in computational solid mechanics | Use its distinctions to structure the evidence dossier; do not collapse these into one quality score. |

The BE25 table values must not be silently used as ground truth while input interpretation is unresolved. The audit compares the literal PDF values (E=200\,\mathrm{MPa}), (I=0.1\,\mathrm{cm^4}), (\mu=25\,\mathrm{kg/m}) with the value (\mu=\gamma A/g\approx0.254929\,\mathrm{kg/m}); neither candidate reproduces the published 0.457 Hz first cantilever reference. The attached `eigenvalue analysis.dat` must be inspected before the source discrepancy can be resolved.

## Qualification gates and proposed targets

### Q0 — Reproducibility and provenance

- 100% of accepted benchmark runs must bind the exact source commit, canonical request SHA-256, source-tree digest, toolchain and model claims.
- Two independent executions of a deterministic fixture must produce byte-identical canonical receipts.
- Any unresolved unit, boundary-condition or element/mass formulation ambiguity yields **blocked / not-qualified**, not a pass.
- Receipts must distinguish `audit-complete` from `benchmark-pass`; rejected/blocked inputs are successful audit outcomes but failed benchmark qualification.

### Q1 — Analytical element/model correctness

- Closed-form element matrix entries, analytical two-DOF eigenvalues, cantilever compliance and frequency are checked against independent derivations.
- Normalized generalized eigen-residual, modal-mass normalization and mass-orthogonality targets: each below (10^{-8}) on well-conditioned fixtures.
- Preserve every requested mode; explicitly classify negative and near-zero/mechanism modes rather than silently filtering them.
- Named defects for every critical formula/unit/assembly branch must be killed by the mutation suite; report the tested mutation denominator, not a vague "100% test coverage" claim.

### Q2 — Property-based / metamorphic verification

- Fixed-seed parameter sweeps cover the predeclared geometric/material envelope and every returned mode, not only the first frequency.
- Tests include scaling area/material/density/length where physics gives an exact invariance or scaling law; storage order should not affect canonical assembly.
- Proposed maximum relative error for analytically equivalent metamorphic transformations: (10^{-7}) for the current small, well-conditioned synthetic model sweep.
- A 64-case beam-modal parameter-envelope sweep is proposed in [PR #2662](https://github.com/Luminous-Dynamics/symtropy/pull/2662); its current exact-head receipt still has to pass before these targets count as evidence.

### Q3 — External numerical benchmark reproduction

- Freeze original source input files and their hashes; record source licence/access terms.
- Match geometry, units, supports, releases, free-DOF map, element model, mass matrix (including any rotary inertia), mesh and requested observable.
- For a published eigenfrequency table, proposed acceptance is at most 1% relative frequency error for each named mode once the input deck and precise reference are reconciled. Use a stricter tolerance only where exact inputs and rounding justify it.
- For a refined modal model, also require mode correspondence (MAC, with subspace treatment for closely spaced/repeated eigenvalues), a convergence curve and mechanism/rigid-mode accounting. Frequency-only agreement is insufficient.
- Never confuse the 2-DOF NAFEMS axial table with the released-joint beam spectrum, or SOFiSTiK BE25 with NAFEMS NBC05.

### Q4 — Numerical convergence and performance

- Publish frequency versus mesh-size curves and frequency/modal-subspace changes between consecutive refinements.
- Target relative frequency change below 1% for the requested physical band, with mode tracking using MAC/subspace pairing; this is a proposed gate, not a result already achieved.
- Scale past the 128-total-DOF dense solver cap using sparse assembly/eigensolvers before attempting hundreds of beam elements per member.
- Compare throughput and memory only at matched model, boundary conditions, mode count and frequency error. A 2× performance gain against a named open-source baseline is a stretch goal, not currently demonstrated.

### Q5 — Physical validation and uncertainty

- Use experimental datasets with reproducible provenance, units, calibration and licence; record measurement uncertainty and environmental/state conditions.
- Proposed initial goal: median frequency error within 5% and P95 within 10% only when those limits are consistent with reference uncertainty and model assumptions. Otherwise set thresholds from the uncertainty budget before results are examined.
- Report verification error, input uncertainty, measurement uncertainty, discretization uncertainty and model-form discrepancy separately.
- No experimental modal dataset has yet been ingested or used to validate this work. Therefore no current physical-accuracy or structural-safety claim is available.

## What would actually outperform the industry?

Do not claim a higher-accuracy solver based on unit tests or a handful of matching frequencies. The credible differentiation target is **auditability plus model-class honesty**: every numerical claim is linked to its source/request, independent oracle, mutation sensitivity, model boundary and uncertainty budget. To beat a named commercial or open-source result, run the same source input and element/mass formulation, compare the same modes and residuals, independently reproduce results, and include compute/memory on pinned hardware.

A future benchmark leaderboard should show per-case absolute/relative error, mode MAC, convergence rate, residuals, rejected-input behaviour, runtime and peak memory side by side. No single composite score should allow a fast but physically incomplete model to outrank a slower model that captures the correct physics.

## Explicit non-claims

The existing tests cover synthetic small-displacement linear models and software invariants. They do not establish a full SOFiSTiK BE25 or NAFEMS NBC05 beam reproduction, experimental validation, nonlinear snap-through, buckling, fracture, code compliance or structural safety.
