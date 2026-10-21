# Planar Truss Mutation Qualification Protocol v0.1

**Tracking:** [PHYS-TRUSS-001 #2361](https://github.com/Luminous-Dynamics/symtropy/issues/2361)  
**Implementation:** [planar truss reference solver PR #2363](https://github.com/Luminous-Dynamics/symtropy/pull/2363)  
**Purpose:** test sensitivity of the truss reference tests to named numerical/formula defects. This is not physical validation.

## Metamorphic and known-answer roles

The symmetric two-bar V-truss is checked against an independently derived closed-form solution for apex displacement, signed axial forces, support reactions, and normalized equilibrium residuals. The solver also checks input-order invariance, load and stiffness scaling, mechanisms/singular matrices, invalid topology/geometry, overflow, and the invariance of the pivot tolerance under a joint scaling of stiffness and loads.

These tests give different kinds of evidence:
- Known-answer checks pin an independently derived result for one declared model.
- Metamorphic checks constrain how outputs must change under specified transformations.
- Negative controls verify that underconstrained or malformed inputs do not yield a PASS.
- Mutation checks establish that selected tests detect selected classes of implementation defects.

No one layer establishes that the 2D axial-bar model represents a real structure.

## Energy consistency is a separate gate

For the current static, linear-elastic truss with proportional loading and zero-displacement supports, the solver now reports:
- Member strain energy: (U = \frac{1}{2}\sum_e N_e\delta_e).
- External work: (W = \frac{1}{2}\sum_i \mathbf{F}_i\cdot\mathbf{u}_i).
- Absolute residual (U-W) in joules and a scale-normalized residual.

Support reactions participate in the force/moment equilibrium calculation, but are not included in external work because restrained degrees of freedom have zero prescribed displacement. The frozen V-truss test compares (U) and (W) with the independent closed-form value (\frac{1}{2}P|u_y|); all 128 parameter-sweep cases also check energy agreement and the normalized residual gate. The request-bound truss receipt cannot report PASS if the known-answer energy or normalized energy gate fails.

Energy agreement is a separate software/numerical consistency invariant, not an external physical observation. It can detect certain defects that force equilibrium alone may miss, but the two invariants still share the same model and implementation. Neither establishes a real structure's physical validity.

## Asymmetric geometry and off-axis loading

The additional `asymmetric_two_bar_truss_matches_closed_form_oracle` case uses a free apex at (1,3), supports at (0,0) and (4,0), unequal member axial stiffnesses, and applied load (600,-1200) N. Its independently derived reduced stiffness matrix is (10^9\begin{bmatrix}1.1&-0.7\\-0.7&1.9\end{bmatrix}) N/m. It pins both displacement components, signed member forces, reactions, residuals and energy/work agreement. This avoids relying only on the symmetric 45-degree vertical-load fixture, where off-diagonal stiffness terms can cancel.

The (cs=0) mutation is specifically assigned to the 128-case asymmetric parameter sweep: it should catch a defect that the symmetric known-answer alone can miss, across a range of geometry, unequal axial stiffness, and mixed-sign off-axis loads. The sweep receipt uses seed `0x4153594D54525553`, 128 cases, 21 scalar output checks per case (2,688 total), and independently defined parameter bounds; its evidence class is test execution, not physical validation.

The asymmetric sweep bounds are: span 2–6 m; apex x between 20% and 80% of span; apex height 0.5–3 m; Young's modulus 50–210 GPa; bar axial stiffnesses 1e7–1e10 and 7e6–1.3e10 N/m; and both apex force components from −3,000 to +3,000 N. The fixed LCG seed and constants are checked into the test; the CI metadata receipt names the independent 2x2 inverse oracle and its source hash. These bounds are deterministic regression coverage, not a distributional uncertainty model.

## Named mutation set

The CI script `research/physical-transformation-reference/scripts/verify_planar_truss_mutations.sh` first runs the unmutated `planar_truss::tests` control. It then copies the crate manifest, lockfile, source files and frozen input fixture into fresh temporary directories, applies one explicit mutation per copy, and runs the intended test by exact name.

| Mutation ID | Deliberate defect | Required detecting test |
|---|---|---|
| `PHYS-TRUSS-MUT-STIFFNESS-AREA-001` | Omit area from (EA/L) during stiffness assembly | `load_and_stiffness_scaling_obey_metamorphic_relations` |
| `PHYS-TRUSS-MUT-STIFFNESS-LENGTH-001` | Divide assembled axial stiffness by (L^2) instead of (L) | `fixed_seed_parameter_sweep_matches_symmetric_truss_closed_form` |
| `PHYS-TRUSS-MUT-MEMBER-FORCE-AREA-001` | Omit area from recovered axial force (N=EAepsilon) | `symmetric_two_bar_truss_matches_independent_closed_form_oracle` |
| `PHYS-TRUSS-MUT-REACTION-SIGN-001` | Reverse the sign of vertical support-reaction recovery | `symmetric_two_bar_truss_matches_independent_closed_form_oracle` |
| `PHYS-TRUSS-MUT-ENERGY-HALF-FACTOR-001` | Omit the one-half factor from member strain-energy accumulation | `symmetric_two_bar_truss_matches_independent_closed_form_oracle` |
| `PHYS-TRUSS-MUT-OFFDIAGONAL-CS-001` | Set the element stiffness off-diagonal direction-cosine product (cs) to zero | `fixed_seed_asymmetric_sweep_matches_independent_two_dof_oracle` |
| `PHYS-TRUSS-MUT-PIVOT-SCALE-001` | Reintroduce an absolute `max(1.0)` floor in the pivot tolerance | `uniform_stiffness_and_load_rescaling_does_not_change_displacement` |

A mutant counts as detected only when its named test appears as FAILED. A compile failure, unrelated Cargo failure, failed source transform, or surviving mutant invalidates the audit. After the isolated runs, the script re-checks the source hash. CI then checks the entire production source remains unchanged, re-runs formatting, the full crate suite and Clippy, and uploads the JSON mutation receipt with the reference receipt.

## Receipt fields and interpretation

The receipt binds the exact 40-character source commit, SHA-256 of `planar_truss.rs`, Rust/Cargo versions, control outcome, stable mutation IDs, intended test names, per-mutant disposition and counts. The summary is limited to these seven deliberately selected mutations. The third mutation targets the 128-case closed-form parameter sweep, whose bounds, fixed seed, and scalar-check count are published separately in `planar-truss-sweep-receipt-v0.1`; the fourth mutation targets the strain-energy half-factor; the fifth zeros the off-diagonal `cs` coupling and is required to fail the asymmetric oracle. It must not be represented as a global mutation score or a physical-accuracy metric.

## Claim boundary

The solver handles only static small-displacement linear-elastic 2D pin-jointed axial bars under its declared input contract. It excludes member bending, buckling, geometric/material nonlinearity, contact, fracture, dynamics, and structural code/safety checks. Passing this audit shows only software-test sensitivity for the seven named defects; it neither reproduces a proprietary catalog case nor establishes sim-to-real validity.
