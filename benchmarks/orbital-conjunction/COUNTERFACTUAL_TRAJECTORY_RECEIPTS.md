# Shared-exogenous counterfactual trajectory receipts

Status: structural research contract and finite identity validator. This does not authenticate simulators, validate force models, establish orbit accuracy, or authorize manoeuvres.

## Why row-level pairing is insufficient

Common-random-number (CRN) comparisons are valid only to the extent that portfolio outcomes are evaluated under the intended common exogenous realization. Reusing a seed, atom label or row number is not enough. A simulator could preserve the same atom ID while silently changing the initial state, covariance, environment realization, process noise, coordinate frame, epoch or density semantics.

The paired-objective oracle in [#7701](https://github.com/Luminous-Dynamics/symtropy/pull/7701) therefore has a stronger qualification path:

1. Freeze one exogenous world receipt per epistemic-scenario × latent-atom pair.
2. Apply each eligible portfolio's own intervention to that common pre-action world.
3. Produce one portfolio-specific post-action state, propagated trajectory and encounter inventory.
4. Bind each trajectory receipt to the mission manifest, event inventory, risk envelope, post-action horizon and policy used by the hard mission-risk gate.
5. Only then calculate paired objective contrasts and attach the trajectory-receipt digest to the Pareto report.

## Common cause versus intervention effect

The shared receipt records the exogenous inputs:
- pre-manoeuvre state and covariance;
- latent uncertainty draw;
- environment realization;
- exogenous process-noise realization;
- target measure and density-space identities;
- reference frame, time scale and reference epoch.

The portfolio receipt records what changes under intervention:
- manoeuvre plan (including an explicit no-op plan for a baseline arm);
- portfolio-specific manoeuvre execution/actuation-noise realization;
- post-action initial state;
- propagator and integrator configuration;
- complete post-action trajectory;
- post-action encounter inventory and risk-policy linkage.

Actuation error is not automatically a common random variable. The receipt binds a separate nonzero execution-noise digest per portfolio. Equal execution-noise digests are allowed only when the experiment explicitly justifies a shared actuation error; otherwise the same seed must not be reused across physically different action mechanisms.

Different post-action trajectories and event inventories are expected. The common pre-action state and exogenous disturbance are not.

The physical-world/force-model configuration identity is scenario-specific and caller-supplied. The hard-risk gate's mission-support manifest is portfolio-specific because a manoeuvre may introduce or remove encounter events. The implementation intentionally does not force these portfolio manifests to be equal or reinterpret them as the physical-model identity.

## Receipt validation

The campaign freezes the upstream hard-risk gate, scenario contracts, complete finite atom support, and eligible/ineligible portfolio partition. It checks:

- exact scenario coverage and unique latent atoms;
- target-measure and density-space agreement with the hard-risk gate within each scenario;
- frame, time-scale and epoch agreement with the frozen scenario contract;
- one shared-world receipt for every scenario/atom row;
- exactly one trajectory receipt for each eligible portfolio on every row;
- an identical shared-world receipt digest referenced by every portfolio receipt in that row;
- the scenario physics, propagator and integrator identities for each portfolio;
- each portfolio's own mission manifest, event inventory, horizon, risk policy and risk envelope against the corresponding hard-risk check;
- canonical row/portfolio ordering and deterministic digests.

The paired-objective campaign's evaluate_with_trajectory_receipts path additionally requires exact equality of gate digest, scenario universe, eligibility partition, scenario/atom universe, and scenario physics configuration identities before embedding the receipt report digest in the Pareto result. An arithmetic-only call to evaluate does not claim this qualification and leaves the trajectory-receipt digest absent.

## What the digest proves—and does not prove

A digest proves only content identity relative to the supplied bytes. It does not prove:
- that the issuer is authentic;
- that the declared propagator actually executed;
- that the force model is physically adequate;
- that all catalogue objects or conjunction windows are present;
- that the simulator is deterministic or correctly implemented;
- that two supposedly common latent atoms were sampled from the correct target measure;
- that the environment or process-noise realization is physically representative.
- that portfolio-specific actuation-noise realizations are physically representative.

A subsequent independent-replay service must re-run the same scenario/atom/portfolio campaign through a separately implemented propagator path, compare full trajectory and encounter outputs, and retain build, binary, kernel, input-product and numerical-tolerance provenance. This repository module is only the structural contract for that future service.

## Required benchmark campaign

### Synthetic exhaustive oracle

Use simple analytically solvable trajectories (two-body, constant-acceleration and piecewise-linear fixtures) with known action deltas and encounter windows. Compare:
- exact pre-action state/covariance identity;
- exact latent atom and exogenous disturbance identity;
- post-action state and trajectory differences;
- encounter inventory differences;
- mission-risk gate bindings;
- replay digest equality.

### Decoy and mutation controls

For every valid campaign, generate decoys that preserve IDs but alter exactly one field:
- pre-action state;
- covariance;
- latent uncertainty draw;
- atmosphere/environment;
- process-noise realization;
- target measure;
- density coordinate space;
- reference frame;
- time scale;
- reference epoch;
- scenario physics model;
- propagator/integrator configuration;
- manoeuvre plan;
- post-action state;
- trajectory;
- event inventory;
- mission manifest;
- post-action horizon;
- risk policy or risk envelope.

Expected result: shared-world mutations invalidate the common receipt or scenario contract; post-action mutations change the portfolio receipt digest; any risk-linked change mismatching the hard-risk gate is rejected.

### Coupling efficiency benchmark

Compare the same objective frontier under:
1. true shared exogenous trajectory realizations;
2. independent per-arm trajectory realizations;
3. same atom labels with intentionally mismatched underlying worlds.

Measure paired-difference variance by objective/scenario, propagation calls to resolve a frontier, false dominance, frontier precision/recall, and regret. CRN should be reported as an empirical effect, not assumed to help: negative cross-arm covariance can inflate variance.

### External physical reference

Later compare against matched NASA CARA and independent orbit-propagation products. NOAA TraCSS can support geometry/screening reproduction but is not a universal Pc truth set or operational certification dataset. All physical comparison must freeze object pair, epoch, frame, covariance, hard-body radius, force model, kernel/version and tolerance semantics.

## Acceptance targets

- zero acceptance of mismatched shared-world or scenario/atom identities in exhaustive negative controls;
- zero acceptance of missing/duplicate/substituted eligible portfolio receipts;
- zero acceptance of receipt references not matching the exact shared-world digest;
- exact agreement with an independently implemented receipt canonicalizer;
- exact digest invariance to row and arm ordering;
- zero false qualification when any hard-risk-linked per-portfolio manifest, event inventory, post-action horizon, risk policy or risk-envelope identity is substituted;
- no assertion of physical accuracy or operational safety from receipt validation alone.

## Related work

- [#7701 — exact paired-objective Pareto oracle](https://github.com/Luminous-Dynamics/symtropy/pull/7701)
- [#7721 — shared-exogenous counterfactual trajectory coupling research](https://github.com/Luminous-Dynamics/symtropy/issues/7721)
- [#7578 — exact multi-arm shared-atom comparisons](https://github.com/Luminous-Dynamics/symtropy/pull/7578)
- [#7637 — hard mission-risk feasibility gate](https://github.com/Luminous-Dynamics/symtropy/pull/7637)
