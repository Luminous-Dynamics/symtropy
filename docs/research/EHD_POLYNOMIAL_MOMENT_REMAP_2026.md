# EHD Exact Polynomial-Moment Remap Oracle (2026)

**Benchmark:** EHD-POLYNOMIAL-MOMENTS-001  
**Status:** analytic P0/P1/P2 integral oracle and authored tests; Rust execution pending  
**As of:** 21 November 2026  
**Machine-readable form:** [ehd_polynomial_moments_v0_1.json](../../data/research/ehd_polynomial_moments_v0_1.json)

## 1. Why add this benchmark

A conservative remap can preserve the global integral while distributing the quantity incorrectly among destination cells. A useful higher-order qualification must test both local destination integrals and the global integral for known polynomial fields.

This reference oracle complements, but does not replace, the earlier components:

- `geometry.rs`: validates axis-aligned source/destination space-time partitions and computes overlap fractions.
- `exchange.rs`: remaps already-integrated vector impulse under a first-order, piecewise-constant source assumption.
- `gcl.rs`: checks the geometric conservation identity for linearly moving axis-aligned boxes (TPE(0)/constant-state reference).
- `exchange_receipt.rs`: checks event-level equal-and-opposite impulse, method exclusivity, and path/frame/solver identity.
- `polynomial_moments.rs`: exactly integrates caller-supplied P0/P1/P2 polynomial fields over axis-aligned overlap regions.

## 2. Implemented mathematical oracle

The Rust module is:

`tools/ehd-reference-audit/src/polynomial_moments.rs`

Each polynomial is represented as a sum of monomials:

\[
p(\mathbf x)=\sum_{m} c_m\prod_{i=1}^{N}x_i^{\alpha_{mi}}.
\]

For an axis-aligned box \(B=\prod_i[a_i,b_i]\), the monomial integral separates:

\[
\int_B \prod_i x_i^{\alpha_i}\,d\mathbf x
=\prod_i\frac{b_i^{\alpha_i+1}-a_i^{\alpha_i+1}}{\alpha_i+1}.
\]

The implementation uses exact antiderivative formulas (up to floating-point arithmetic) and compensated summation across monomial terms.

The remap oracle:
1. calls the existing geometry backend to reject overlap/gap defects and require complete source coverage;
2. constructs each source/destination intersection box;
3. integrates the same supplied polynomial over the complete source box and every destination intersection;
4. reports each destination integral separately;
5. checks both the global sum and the independently returned local values against known-answer fixtures.

The polynomial coefficients are supplied explicitly. This is an integration oracle, not a reconstruction method: it does not infer P1/P2 coefficients from cell averages, construct gradients/Hessians, limit reconstructed fields, or execute a production remapper.

## 3. Authored test matrix

Nine module tests are currently authored:

1. P0 constant-field split conservation.
2. P1 linear-field exact destination integrals.
3. P2 quadratic-field exact destination integrals.
4. Space-time P1 field integral across independent time slabs.
5. Reject a term above the declared degree.
6. Reject non-finite polynomial coefficients.
7. Reject geometric gaps before polynomial transfer.
8. Destination input-order determinism.
9. Offline reference self-check.

These are authored tests, not executed Rust tests.

## 4. Relationship to TPE and evolved geometric moments

This oracle is a prerequisite for a higher-order comparison, not a TPE implementation.

TPE(0) corresponds to constant-state preservation/GCL. TPE(1) and TPE(2) require a fully discrete transport method to preserve the transport of linear and quadratic polynomial states under the declared mesh-motion model. Integrating a supplied polynomial exactly over static AABB intersections does not establish TPE(1), TPE(2), or a designed convergence order.

Two research families should be compared rather than prematurely selecting one:
- **Intersection/moment remap:** reconstruct a polynomial field and integrate it over source/destination intersections.
- **Evolved-geometric-moment (EGM) transport:** evolve the geometric moments needed for polynomial transport, potentially avoiding repeated expensive geometric intersections under some mesh-motion regimes.

Relevant preprints:
- Cheng, Cai & Qiu (2026), *Transport Polynomial Exactness for Direct ALE-DG Methods with Nonsmooth Mesh Motion*, arXiv:2610.15650: https://arxiv.org/abs/2610.15650
- Cai, Cheng, Wu & Qiu (2026), *Beyond Free-Stream Preservation: Transport Polynomial Exactness for Moving-Mesh Methods under Arbitrary Mesh Motion*, arXiv:2602.09729: https://arxiv.org/abs/2602.09729

These are preprints and should be treated as research comparators pending independent reproduction. Neither paper establishes EHD-specific solver coupling or physical thrust validation.

## 5. Required next benchmark

A meaningful tournament should freeze identical source/destination meshes, polynomial fields, motion paths and error budgets, then compare:

1. P0 piecewise-constant conservative remap.
2. P1 reconstruction plus intersection integration.
3. P2 reconstruction plus intersection integration.
4. EGM/TPE transport where an independent implementation is available.
5. preCICE methods only where the mapped quantity, integral semantics and geometry are equivalent.

Measure:
- cell-local integral error and global conservation residual;
- polynomial coefficient/moment reproduction;
- L1/L2/L-infinity field errors;
- observed spatial and temporal convergence order;
- robustness under nonsmooth mesh motion and independent time subdivision;
- geometry setup, mapping, correction, fallback and total workflow cost;
- memory, communication and deterministic replay.

A small global residual is not sufficient: local cell errors and polynomial exactness must be reported independently.

## 6. Scope and limitations

The current oracle supports axis-aligned boxes in compile-time dimensions. It uses caller-supplied polynomial coefficients and does not:
- reconstruct a high-order field from cell averages;
- implement a production P1/P2 remapping algorithm;
- support rotated, curved or general unstructured cells;
- move or deform the cells during integration;
- implement EGM, an ALE-DG solver or TPE(1)/TPE(2);
- integrate species collision physics or perform a plasma-neutral solver exchange.

## 7. Claim ceiling

This module is an analytic P0/P1/P2 integration oracle for axis-aligned overlap boxes. It does not establish higher-order solver accuracy, TPE under moving meshes, production remapping, Rust test success, physical EHD validation or SOTA superiority.
