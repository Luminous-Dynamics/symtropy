# Common-cause-aware sequential sensing v0.1

**Issue:** [NAV-COMMON-CAUSE-VOI-001 #6708](https://github.com/Luminous-Dynamics/symtropy/issues/6708)  
**Parent:** [NAV-SEQUENTIAL-VOI-001 #6689](https://github.com/Luminous-Dynamics/symtropy/issues/6689)

## Model

Two hidden binary variables are represented jointly:
- S: outside vs inside the declared navigation service envelope;
- C: nominal vs shared common-cause regime.

The prior is a four-component joint distribution, not a product of assumed independent marginals. Each sensing action supplies P(outcome | S,C) and P(acquisition succeeds | S,C).

For latent z=(S,C), prior p(z), acquisition q_a(z) and outcome likelihood l_a(o|z):

P(o, success) = Σ_z p(z) q_a(z) l_a(o|z)

P(no observation) = Σ_z p(z) (1-q_a(z))

Both successful observations and no-observation events update the posterior over S and C. Repeated observations are conditionally independent only after conditioning on both latent variables. Marginalizing C generally induces dependence between modalities.

## Implemented

- Joint prior/outcome normalization and fail-closed validation.
- Four-state outcome and acquisition models.
- Posterior updates for measurement and state-dependent missingness.
- Exact contingent horizon-two policy and greedy-first comparator.
- Hard energy, latency and bandwidth constraints.
- Diagnostic comparing the actual two-sensor joint outcome probability with the naive independent approximation.

## Known-answer negative control

The demo constructs UWB-like and camera-like sensors that may both emit a safe reading during a shared common-cause regime while service is actually outside its envelope. Under the synthetic prior and likelihoods, the actual joint probability of two safe readings conditional on unsafe service is over three times the independent approximation. This is a constructed test, not a measured sensor property.

## Limits

One binary service state, one binary shared regime, static state over two steps, finite actions/outcomes and known likelihoods. It does not identify real dependencies, calibrate regime priors, handle arbitrary copulas or multiple interacting failure domains, or represent continuous state evolution. The conditional-independence assumption must be checked against actual source-dependency evidence. This is not an integrity certificate.
