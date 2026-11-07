# Continuous-Time Oscillator Kernel V0

Standalone, dependency-free Rust reference implementation for Symtropy issue #3811.

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

## Multichannel observation filter

The filter augments the two oscillator coordinates with one static animal-level random intercept per named observation channel:

    x = [z1, z2, b_left, b_right, ...]

    y_c(t) = intercept_c + loading_c^T z(t) + b_c + epsilon_c(t)

It provides:

- Irregular-time prediction using the exact continuous-time A(dt) and Q(dt).
- Sequential Kalman innovations for a joint multichannel predictive likelihood. A measurement in one channel updates the shared oscillator posterior and therefore changes forecasts in the other channels.
- Channel-specific observation noise and static animal effects, with effect posteriors retained across sessions.
- Session reset of oscillator state while retaining animal effects and their covariance; oscillator/effect cross-covariance is cleared at the boundary.
- Omission-based missingness: absent channels are not zero-filled. An empty observation batch advances time without inventing measurements.
- An explicit instantaneous phase-perturbation operation for controlled phase-response experiments. The caller must bind it to an independently verified assignment/delivery receipt.
- Posterior phase and delta-method phase uncertainty, with abstention when latent amplitude is too small relative to its uncertainty.
- Atomic timestamp updates: invalid input or numerical failure does not leave a partially conditioned state behind.
- Strict duplicate-channel, channel identity, timestamp, input finiteness and numerical validation.

The API time unit is declared by the caller and must match angular frequency (radians per time unit) and damping (inverse time unit).

## What this qualifies

- Exact continuous-time mean transition and process covariance under the declared isotropic stationary covariance.
- Mean and covariance semigroup composition.
- Multichannel Gaussian filtering with a shared latent oscillator and channel-specific static random effects.
- Joint predictive log likelihood through sequential innovations.
- Irregular elapsed-time propagation, explicit session reset, missing-channel omission and phase-perturbation handling.
- Known-answer and negative-control tests for update algebra, cross-channel conditioning, session persistence, missingness, phase abstention and numerical atomicity.
- No external dependencies; standalone Cargo manifest and dedicated Rust 1.96.0 formatting/test/strict-Clippy workflow.

The current implementation contains 24 deterministic tests across the exact transition and observation filter. CI must verify the current exact branch head before any pass is claimed.

## Scientific boundary

This is a mathematical and inference kernel, not a fitted locomotor dataset, connectome model, adaptive experiment planner, embodied simulation or biological result.

A two-dimensional oscillator is one candidate mechanism family. It must compete against scalar AR(1), VAR(p), ordinary linear Gaussian state-space models, switching dynamical systems, sensory-feedback attractors without intrinsic oscillation, and per-leg or coupled-oscillator architectures.

A periodic kinematic fit does not prove the existence of a biological central pattern generator. Phase and frequency are not identifiable in all short, noisy, aliased or low-amplitude recordings. Those cases require explicit abstention or an equivalence set, not a manufactured point estimate.

The transition is stationary and isotropic in the latent plane. The observation filter currently uses fixed oscillator frequency and damping, fixed channel loadings, Gaussian observation noise and static additive animal effects. It does not yet implement fitted parameter inference, frequency modulation, continuous forcing, nonlinear phase-response curves, switching regimes, random slopes/session effects, non-ignorable missingness or coupled per-leg oscillator populations. The phase perturbation rotates the latent state as an explicitly declared idealized action; it is not a biological perturbation model.

## Run

    cargo fmt --manifest-path research/continuous-time-oscillator-v0/Cargo.toml -- --check
    cargo test --manifest-path research/continuous-time-oscillator-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/continuous-time-oscillator-v0/Cargo.toml --locked --all-targets -- -D warnings
