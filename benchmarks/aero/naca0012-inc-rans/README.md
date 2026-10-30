# NACA 0012: incompressible turbulent SU2 reference

**Qualification contract:** AERO-QUAL-001 (#2960)  
**External reference:** NASA Turbulence Modeling Resource (TMR)  
**Execution adapter:** CFD-REF-001 (#2894)

This directory freezes the first aerodynamic reference case. It is a benchmark input definition, not a result and not evidence that Symtropy has validated aerodynamics.

## Source artifacts

The upstream SU2 tutorial publishes:
- configuration: `incompressible_flow/Inc_Turbulent_NACA0012/turb_naca0012.cfg`
- mesh: `incompressible_flow/Inc_Turbulent_NACA0012/n0012_897-257.su2`

Upstream repository: https://github.com/su2code/Tutorials  
Source ref inspected: `master`  
Config Git blob SHA: `38d60d0ff09853a9fcf51559c8bd1e56a829ebe7`  
Mesh Git blob SHA: `dc088bd5a6e9897e60b17da9c744e36b2c719187`

The config header identifies itself as SU2 6.1.0. Treat that as an artifact-generation/version-history note, not a claim that the current executable is 6.1.0. An actual run must bind the executable version, executable SHA-256, build options, and OS separately. The mesh file's Git blob digest is recorded even though the repository connector did not return its text payload.

## Physical setup

The SU2 tutorial describes steady 2D incompressible RANS, Spalart–Allmaras turbulence, FDS with second-order MUSCL, implicit Euler, far-field and no-slip/adiabatic airfoil-wall conditions.

- Chord: 1 m
- Reynolds number per chord: 6,000,000
- Constant density: 2.13163 kg/m³
- Constant dynamic viscosity: 1.853e-5 kg/(m·s)
- 0° velocity: (52.1572, 0, 0) m/s
- 10° velocity: (51.36481493540834, 9.0570027322096198, 0) m/s
- Mesh: 897 airfoil-normal × 257 airfoil-tangential nodes
- Far-field: approximately 500 chords
- Near-wall spacing: designed for y+ < 1

The 0° and 10° cases are separate case instances. Do not mutate one case in-place and then reuse stale output. Generate separate, digest-bound run directories and configs.

## Observables and evidence classes

| Quantity | Reference source | Evidence type / caveat |
|---|---|---|
| Cp(x/c), 0° and 10° | TMR-linked pressure data, including Gregory & O'Reilly | Experimental pressure measurements; pressure coordinate and surface orientation must be aligned |
| Cf(x/c) | TMR-published CFL3D profile | Code-to-code comparison only; TMR says no known experimental Cf data for this case |
| CL/CD | TMR experimental curves | Must match trip state and Reynolds number; TMR recommends Ladson tripped force data for fully turbulent CFD at Re=6M |
| Grid convergence | TMR numerical-analysis grid family | Verification of discretization error, not experimental validation |

TMR notes that different wind-tunnel datasets disagree, especially near stall. Gregory & O'Reilly pressure data are identified as comparatively two-dimensional, while their force data are at Re=3M. Ladson's tripped force data are recommended for comparison with fully turbulent CFD at Re=6M. These datasets are not interchangeable and must not be averaged into one synthetic truth curve.

TMR also warns that the initial validation grids may be inadequate for high-accuracy work and refers users to finer nested grids in its numerical-analysis case. The 897×257 SU2 tutorial grid is therefore a reproduction fixture, not by itself a convergence study.

## Run protocol

Before the run:
1. Verify the config and mesh blob identities above.
2. Derive separate 0° and 10° configs from the pinned upstream config, changing only the free-stream vector and an explicitly documented case label.
3. Record all config diffs and derived config digests.
4. Pin the SU2 executable version, binary digest, build, platform, command arguments, and run environment.
5. Use a new empty output directory. Never reuse prior history/restart files.

After the run:
1. Preserve stdout, stderr, SU2 history, surface output, restart/solution output, and raw hashes.
2. Require finite fields, force history convergence, residual criteria and continuity/force checks. Exit code zero is not enough.
3. Compare Cp with surface-coordinate alignment and a frozen interpolation rule.
4. Compare Cf only against the named CFL3D reference.
5. Use at least three nested mesh levels from TMR before publishing grid-convergence order or GCI.
6. Preserve failed, unstable and unconverged runs in the benchmark ledger.

## Claim ceiling

This case only qualifies one steady 2D incompressible RANS configuration at Re=6M, 0°/10°, a specified mesh family and SA turbulence model. It does not validate transonic shock flows, stall prediction, arbitrary airfoils, 3D wings, transition modeling, aeroacoustics or engineering design generally.
