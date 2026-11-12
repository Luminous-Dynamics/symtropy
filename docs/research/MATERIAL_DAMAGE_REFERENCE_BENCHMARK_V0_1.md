# Material Damage Reference Benchmark v0.1

**Tracking:** PHYS-DAMAGE-REFERENCE-001  
**Implementation:** `research/material-damage-reference/`  
**Status:** initial software and analytical qualification contract; no measured validation claimed.

## 1. Purpose

Provide a minimal, reproducible reference for cumulative fatigue damage and stable crack extension that can later feed Symtropy's revisioned physical lifecycle model. The initial scope intentionally begins after cycle counting and after stress-intensity-factor calculation. It verifies two constitutive calculation surfaces without pretending to own the upstream measurement or solver problems.

## 2. Models and assumptions

### 2.1 Stress-life plus linear Miner accumulation

For a supplied S-N curve, cycles to failure are interpolated linearly in log(stress amplitude) / log(cycles). For load block i, damage increment is:

    D_i = n_i / N_i,     D = sum_i D_i

The curve must be strictly ordered and the query must remain inside the supplied stress range. No extrapolation is allowed. The load blocks are assumed to have already been counted; this benchmark does not implement rainflow or recover cycle reversals from raw time series.

The conventional D=1 threshold is reported as a model disposition only. It does not establish physical failure for a real component.

### 2.2 Constant-Delta-K Paris law

For one declared constant stress-intensity-factor range:

    da/dN = C * (Delta-K)^m

The reference computes the constant growth rate and multiplies it by the caller-supplied cycle count. Delta-K is supplied in MPa*sqrt(m), crack length in metres, and the Paris coefficient must use the corresponding units. Inputs outside the caller-declared Delta-K interval are rejected.

This is not a geometry solver. It does not compute Delta-K from stress, crack size, specimen geometry, residual stress or load ratio, and it does not model threshold or instability regions.

## 3. Software qualification

The exact-source workflow runs formatting, known-answer tests, metamorphic tests and strict Clippy. It also runs a mutation-sensitivity audit in disposable source copies.

The 12 November 2026 exact-head mutation campaign killed four selected formula defects:
- reversed S-N interpolation direction;
- multiplication instead of division in the Miner increment;
- omission of the Paris exponent;
- inverse rather than direct scaling with cycle count.

The unmutated full test suite is checked first. The workflow binds and uploads a deterministic mutation receipt to the exact checked-out commit. This is software test-sensitivity evidence only; it does not establish material validity or prove the absence of all possible defects.

## 4. Industry comparison

| Reference | Publicly documented scope | Appropriate comparison |
|---|---|---|
| Ansys nCode DesignLife | Stress-life, strain-life, vibration, thermo-mechanical, weld, material and crack-growth workflows integrated with FEA | Future matched-model comparison; not comparable to this small reference on breadth |
| SIMULIA fe-safe | Multiaxial fatigue, duty-cycle damage, manufacturing effects, probability-of-failure/warranty curves and FE integration | Future fatigue workflow baseline after upstream stresses, cycle counting and material inputs are matched |
| ASTM E647-24 | Experimental measurement of fatigue crack-growth rate versus Delta-K, including influence of load ratio, temperature/environment, thickness and residual stress | Experimental-method anchor for later crack-growth validation |
| 2026 computational-fracture review | Reviews GNN, CNN, PINN, neural-operator and generative fracture approaches; identifies generalization, data, scale and uncertainty as challenges | Research frontier for later physics/ML hybrid studies, not a substitute for experimental validation |

Sources:
- https://ansys.synopsys.com/Products/Structures/Ansys-nCode-DesignLife
- https://www.3ds.com/products/simulia/fe-safe
- https://store.astm.org/standards/e647
- https://doi.org/10.1016/j.engfracmech.2025.111778

## 5. Future benchmark targets

These are programme targets, not achieved results:

1. Data admission: specimen/material-lot identity, source rights, units, stress ratio, environment, thickness, manufacturing state, crack-size measurement and uncertainty are complete.
2. Blind validation: calibration and holdout split by specimen and material lot, never by adjacent frames or windows from the same specimen.
3. Prediction: report log-life error, crack-growth-rate error, interval coverage, false-safe rates, abstention and domain coverage separately.
4. Transfer: test leave-one-material-lot, leave-one-specimen-geometry and leave-one-environment-out conditions.
5. Lifecycle: preserve component identity and damage provenance across inspection, repair, deconstruction and material recovery.

Do not set universal numeric error targets before the dataset, measurement uncertainty and eligible baseline are selected. Pre-register thresholds for each declared material/model regime before opening the blind set.

## 6. Research frontier and differentiation

The recent fracture-ML literature includes physics-informed neural methods, GNNs, neural operators and transformer-based crack-path prediction. The field still faces generalization, dataset quality, scalability and uncertainty challenges. Symtropy should not chase a model leaderboard before it has admissible measurements.

The stronger differentiator is an evidence-qualified lifecycle path:

    load history -> admitted stress/Delta-K -> bounded damage model
    -> uncertainty/applicability -> component-state revision
    -> repair/deconstruction -> verified material fate

Symthaea can propose predictions and interventions, but only admitted physical-model results and observations can update the physical evidence state. A learned latent score must never be promoted to a measured material property.

## 7. Claim ceiling

The initial crate is a software/formula reference. It is not a material database, fatigue certification, fracture solver, structural-health monitor, remaining-useful-life predictor, building-code checker or repair/release authority.
