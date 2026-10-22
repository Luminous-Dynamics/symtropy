# NAFEMS FV4 — Cantilever with Off-Centre Point Masses v0.1

**Tracking issue:** [#2627](https://github.com/Luminous-Dynamics/symtropy/issues/2627)  
**Benchmark ID:** `PHYS-NAFEMS-FV4-CLOSE-MODES-001`  
**Status:** frozen literature manifest; **no FV4 numerical solve is claimed**.

## Why this is a different stress test

The Ansys LS-DYNA verification manual describes this as a NAFEMS (1987) free-vibration case with two off-centre lump masses attached to a cantilever. Unlike the small axial-bar reference, this case stresses inertial-axis coupling and close eigenvalues. The first two published target frequencies are 1.723 and 1.727 Hz: just 0.004 Hz apart, a relative separation of approximately 0.232%. A solver that only sorts eigenvalues can keep the numbers in order yet silently swap or distort the corresponding mode identities as the model changes.

Primary source: [Ansys VM-LSDYNA-SOLVE-062](https://ansyshelp.ansys.com/public/views/secured/corp/v261/en/lsdyna_vm/vm-lsdyna-solve-062.html).

The source describes a 10 m main beam, a 4 m secondary beam/offset dimension, two off-centre lump masses (M_1=10{,}000) kg and (M_2=1{,}000) kg, and a fully restrained cantilever root. It lists material 1 as (E_1=200) GPa, (ho_1=8000) kg/m³, (
u_1=0.3); material 2 as (E_2=10^{16}) Pa, (ho_2=0), (
u_2=0). Its public page provides a sketch and a download-page link but the precise mass attachment coordinates, full section model and raw solver deck have not yet been parsed into an authoritative fixture. Those details remain unresolved, not guessed.

## Source-separated target table

| Mode index | NAFEMS target (Hz) | LS-DYNA result (Hz) | Published error |
|---:|---:|---:|---:|
| 1 | 1.723 | 1.722 | 0.058% |
| 2 | 1.727 | 1.725 | 0.116% |
| 3 | 7.413 | 7.410 | 0.040% |
| 4 | 9.971 | 9.960 | 0.110% |
| 5 | 18.155 | 18.079 | 0.419% |
| 6 | 26.957 | 26.843 | 0.423% |

These values are the **target/result pair published by Ansys for its LS-DYNA verification case**. They should not be silently merged with another solver's mode set. The Ansys Mechanical APDL reproduction page [VMP09-T4](https://ansyshelp.ansys.com/public/views/secured/corp/v261/en/ans_vm/vmp09t4.html) lists its own BEAM188 and BEAM189 results, including a different first pair. Those are cross-solver comparators, not replacements for the NAFEMS targets.

## Future numerical model contract

### FV4-M0 — source qualification

Before a numerical pass can be recorded, resolve and freeze:
- the exact attachment coordinates and orientation of each point mass;
- the cross-section dimensions, moments of area, torsional constant and mass matrix formulation;
- the element formulation and any rigid-link/offset constraints;
- the source input deck and its redistribution/license terms;
- the exact DOF restraint interpretation and solver/eigensolver controls.

If any unresolved item changes the mathematical model, the run must report `not-qualified:missing-reference-input` rather than pass.

### FV4-M1 — close-mode identity

For the 1.723/1.727 Hz pair:
- track eigenpairs by mode shape as well as frequency;
- use a predeclared MAC/assignment policy for isolated modes, and a mass-weighted subspace comparison if the pair becomes numerically repeated;
- record the pair's frequency gap in absolute Hz and relative terms at every mesh/parameter level;
- report mass-normalization, modal orthogonality and eigenpair residuals;
- preserve off-centre inertial-axis effects rather than reducing each mass to an on-axis scalar mass.

A frequency-only tolerance is insufficient to qualify mode identity.

### FV4-M2 — uncertainty and convergence

Vary only justified geometric/material/restraint uncertainties and track the same physical mode identities. Report uncertainty bands for both members of the close pair and the probability that ordering changes. Run the published input discretization first, then refine, while keeping NAFEMS target values, solver-published outputs and any newly computed results in separate fields.

## Proposed acceptance goals (not achieved)

- Six modes returned with all prescribed constraints applied and no unexplained omitted eigenvalues.
- Each eigenpair residual and mass-orthogonality error reported and gated.
- The first two modes are paired by shape/MAC, not only sorted frequency.
- Solver-relative errors are calculated against the NAFEMS column, and are not confused with cross-solver differences.
- A deterministic receipt binds canonical request bytes, source commit, input-deck hash (once obtained), solver version, DOF map, mode assignment, MAC, residuals and claim ceiling.
- Mutation tests detect missing point-mass offsets, swapped mass IDs, a dropped inertial coupling term, and reversed mode-pair assignment.

## Explicit claim ceiling

The present artifact records the literature reference values and acceptance plan only. It does not mean Symtropy solved FV4. No numerical FV4 pass, experimental physical validation, SOTA superiority, structural safety or code-compliance claim is made.
