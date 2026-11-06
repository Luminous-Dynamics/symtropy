# ANIMA Conditional Trajectory Kernels V0

Standalone, dependency-free Rust inference kernels for [Symtropy #3762](https://github.com/Luminous-Dynamics/symtropy/issues/3762) and the interventional state-space programme in [#3547](https://github.com/Luminous-Dynamics/symtropy/issues/3547).

## Scalar reference kernel

The root module implements a Gaussian AR(1) observation model with an animal-level random intercept shared across sessions:

    y[t] = intercept + action_effect[t]
           + phi * (y[t-1] - intercept)
           + (1 - phi) * animal_effect
           + innovation[t]

It analytically integrates the Gaussian animal random intercept, scores each new observation with its conditional Gaussian predictive density, updates competing mechanism probabilities in log space, and resets AR state at session boundaries while retaining the animal effect posterior.

## Multichannel shared-latent kernel

The `multivariate` module adds a linear-Gaussian model with one shared latent coordination state and channel-specific static animal effects:

    z[t] = phi * z[t-1] + process_noise[t]
    y[c,t] = intercept[c] + action_effect[c,action]
             + loading[c] * z[t] + animal_effect[c] + observation_noise[c,t]

The implementation:
- supports multiple named output channels (for example joint angle and contact force);
- exposes the full multivariate predictive covariance induced by the shared latent state;
- updates the shared latent posterior when any observed channel arrives, changing forecasts for the other channels;
- analytically filters channel-specific animal random intercepts that persist across sessions;
- resets the shared latent state at session boundaries while retaining animal effects;
- computes the joint conditional likelihood by a canonical sequence of Kalman innovations;
- accepts partial-channel observations by omitting missing channels under an explicit ignorable-missingness assumption;
- validates channel identities, action identities, assignment/delivery, fidelity, interference, carryover, sample identity and contiguous session-local time;
- binds forecast inputs to a deterministic FNV-1a drift fingerprint (not a cryptographic receipt).

Channel identity is independent of the order in which channels are declared. Predictive means and covariance are returned in sorted channel-ID order.

## Explicit model limitations

These are **synthetic inference kernels**, not complete state-space models, causal estimators, connectome adapters, biological controllers or biological discoveries.

The multichannel V0 model has one scalar shared AR(1) coordination state, channel-specific observation loadings, independent channel-level animal random-intercept priors, Gaussian observation/process noise and action-specific observation offsets. It does not yet implement:
- a two-dimensional oscillator with explicit phase/frequency;
- multiple/coupled leg-specific latent oscillators;
- action-dependent latent-state forcing or phase resets;
- random slopes or session random effects;
- irregular-time transition dynamics;
- nonlinear or behaviour-switching state equations;
- an informative/missing-not-at-random observation model;
- full intervention identifiability or model-adequacy analysis.

Time indices are discrete, contiguous and session-local. Irregular timestamps are rejected rather than treated as one time step. Missing outputs are omitted only under the declared ignorable-missingness assumption; this does not justify treating MNAR dropout as ignorable.

The next model family is specified in [#3811 — continuous-time latent oscillator state-space model](https://github.com/Luminous-Dynamics/symtropy/issues/3811). It must be compared against these AR(1) references, VAR(p), linear and switching state-space models, and must abstain when phase/frequency is not identifiable. A fitted oscillator is not evidence of a biological CPG.

The kernels provide conditional-likelihood seams to #3547, not a replacement for its treatment of intervention validity, temporal identification, observability, parameter identification, latent-coordinate equivalence and model adequacy.

## Tests

Nineteen deterministic tests cover:
- scalar AR(1) history dependence and animal-effect uncertainty;
- animal effects retained across sessions while the scalar AR state resets;
- conditional mechanism-posterior updates;
- assignment/delivery, duplicate-sample, unknown-action, session and time validation;
- cross-channel predictive covariance from the shared latent state;
- updating one channel and changing the other channel's forecast;
- partial-channel observation without zero imputation;
- session-specific reset with persistent channel random effects;
- multichannel conditional likelihood for competing mechanisms;
- unknown-channel/action rejection and atomic update failure;
- channel-order invariance for named forecasts and covariance.

## Run

    cargo fmt --manifest-path research/anima-conditional-trajectory-v0/Cargo.toml -- --check
    cargo test --manifest-path research/anima-conditional-trajectory-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/anima-conditional-trajectory-v0/Cargo.toml --locked --all-targets -- -D warnings
