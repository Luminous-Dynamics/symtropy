# 1D sparse flag-update enthalpy reimplementation

This standalone Rust benchmark independently implements the 1D isothermal flag-update algorithm described in Peters et al. (2025), IJHMT 249, 127166, Sections 4.1–4.4. It is **not** a port of the authors' MATLAB supplementary source.

## Numerical method

At each implicit Euler step, the shifted enthalpy is split as E = S + gamma(f) v, where S is latent enthalpy, v is the Kirchhoff potential and f ∈ {-1,0,1} is a solid/mixed/liquid flag. The phase flag makes each step a linear block system. In 1D, mixed nodes set v=0 and the remaining non-mixed nodes form contiguous tridiagonal segments that can be solved with a Thomas sweep. The mixed-node latent enthalpy is then recovered from the PDE row. Flags are updated according to the paper's consistency rules until stable.

This implementation uses a tridiagonal-segment reduction rather than allocating a dense 2N×2N matrix. A test-only dense Gaussian-elimination oracle checks the reduced solve for several mixed/solid/liquid flag patterns.

## Grid convention

The paper's general notation gives N gridpoints and dx=D/(N-1), but its 1D table says N=48, D=6m, dx=0.125m. The benchmark therefore interprets N as intervals and uses N+1 endpoint-inclusive points to match the tabulated spacings. This is an explicit reproduction interpretation, not a claim that the article's notation is internally consistent or that the MATLAB implementation has been reproduced.

## Build and run

    rustc --edition=2024 -D warnings --test tools/casting-phase-reference/flag_update_1d.rs -o /tmp/flag-update-tests
    /tmp/flag-update-tests

    rustc --edition=2024 -D warnings tools/casting-phase-reference/flag_update_1d.rs -o /tmp/flag-update
    /tmp/flag-update --verify
    /tmp/flag-update --csv

## Metrics

Reports paper-style mean absolute temperature error and mean absolute interface error over all grid points/timesteps. The primary interface is the linear interpolation where latent component S=rho*L/2. Also reports final interface, mean/max flag iterations, tridiagonal segment solves, PDE residual, wall heat and global enthalpy conservation.

The paper explicitly reports that interface error can be zero by construction when a variable time-step pins the front at a node, while temperature error is nonzero. Therefore no single metric is treated as a substitute for the others.

## Claim ceiling

This is an independent 1D reimplementation from published equations. It is not a port or reproduction of the authors' supplementary MATLAB implementation, not physical foundry validation, not an alloy model, and not a production-qualified solver.
