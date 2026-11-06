# Continuous-Time Oscillator Kernel V0

Standalone, dependency-free Rust reference kernel for Symtropy issue #3811.

## Model

For a two-dimensional latent state z = [z1, z2]^T:

    dz = F z dt + L dW
    F = [[-kappa, -omega], [omega, -kappa]]

The exact transition for elapsed time dt > 0 is:

    A(dt) = exp(-kappa * dt) * [[cos(omega*dt), -sin(omega*dt)],
                                [sin(omega*dt),  cos(omega*dt)]]

Under stationary isotropic covariance P_inf = sigma_inf^2 I:

    Q(dt) = P_inf - A(dt) P_inf A(dt)^T
          = sigma_inf^2 * (1 - exp(-2*kappa*dt)) I

The process variance uses exp_m1 to preserve precision for small elapsed times. Damping may be zero, in which case the transition is a deterministic rotation and process covariance is exactly zero.

## What this qualifies

- Exact continuous-time mean transition.
- Exact process covariance under the declared isotropic stationary covariance.
- Semigroup composition for transition matrices and process covariance.
- Explicit units at the API boundary: angular frequency in rad/time; damping in 1/time; variance in state units squared; dt in the matching time unit.
- Fail-closed parameter and numerical validation.
- Ten deterministic known-answer/property tests.
- No external dependencies; standalone Cargo manifest and dedicated Rust 1.96.0 formatting/test/strict-Clippy workflow.

## Scientific boundary

This is a mathematical transition kernel, not a fitted oscillator, neural state-space model, connectome model, adaptive intervention planner, embodied simulation or biological result.

A two-dimensional oscillator is one candidate mechanism family. It must compete against scalar AR(1) and VAR(p), ordinary linear Gaussian state-space models, switching dynamical systems, sensory-feedback attractors with no intrinsic oscillator, and per-leg/coupled-oscillator architectures.

A periodic kinematic fit does not prove the existence of a biological central pattern generator. Phase and frequency are not identifiable in all short, noisy, aliased or low-amplitude recordings. Those cases require explicit abstention.

The transition is stationary and isotropic in the latent plane. Action-dependent phase resets, forcing, frequency modulation, coupled oscillators, multichannel observation filters, random animal/session effects and missingness models are not implemented in V0.

## Run

    cargo fmt --manifest-path research/continuous-time-oscillator-v0/Cargo.toml -- --check
    cargo test --manifest-path research/continuous-time-oscillator-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/continuous-time-oscillator-v0/Cargo.toml --locked --all-targets -- -D warnings
