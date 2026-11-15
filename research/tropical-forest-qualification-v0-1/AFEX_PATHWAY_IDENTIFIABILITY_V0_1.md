# AFEX pathway observability and structural-identifiability benchmark v0.1

**Issue:** [#5449 — AFEX-ROOT-PATHWAY-IDENTIFIABILITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/5449)  
**Parent strategy tournament:** [#5445](https://github.com/Luminous-Dynamics/symtropy/issues/5445)  
**Structural flux owner:** [#5349](https://github.com/Luminous-Dynamics/symtropy/issues/5349)  
**Practical uncertainty owner:** [#5360](https://github.com/Luminous-Dynamics/symtropy/issues/5360)  
**Date:** 15 November 2026  
**Status:** exact synthetic known-truth; no AFEX pathway fluxes measured or inferred

## 1. Why this benchmark matters

The ELM-MYCI model architecture distinguishes direct inorganic nutrient uptake by uncolonised roots, mycorrhizal inorganic uptake and mycorrhizal organic uptake. The AFEX trait records include AM colonisation, morphology, phosphatase and tissue nutrient concentration—but none is a direct measurement of those three source-specific fluxes or host-to-fungal carbon allocation.

A common modelling error is to treat a measured colonisation percentage as though it measured fungal nutrient transfer. It does not. It is, at most, a covariate or proxy used in an observation model.

## 2. A deliberately simplified observation model

Unknown coefficients:

`theta = [k_root, k_myc,inorganic, k_myc,organic, c_host→fungal]`

For plot (i), with known AM colonisation fraction (f_i), the synthetic shared-rate model is:

`U_i = (1-f_i) k_root + f_i k_myc,inorganic + f_i k_myc,organic`

where (U_i) is a hypothetical measurement of total nutrient uptake. This operator assumes common pathway coefficients across plots and a simple proportional dependence on colonisation fraction. It is a mathematical test case, not a biological model validated for AFEX.

For (f={0.25,0.50,0.75}), scaling each row by four gives:

[
A =
egin{bmatrix}
3 & 1 & 1 & 0 \\
2 & 2 & 2 & 0 \\
1 & 3 & 3 & 0
end{bmatrix}
]

The exact rank is 2 for four unknowns, leaving nullity 2. The fungal inorganic and organic columns are identical; host-to-fungal carbon cost has a zero column. The model can at best distinguish the direct-root coefficient from a combined fungal contribution, under its shared-rate assumption.

Adding a source-specific mycorrhizal inorganic tracer observation ([0,1,0,0]) raises rank to 3. The remaining null direction is carbon cost. Adding an independent host-to-fungal carbon allocation observation ([0,0,0,1]) raises rank to 4.

## 3. What current AFEX data can and cannot identify

The current AFEX trait records establish:
- AM root-length colonisation percentage in 0–10 cm only;
- morphology by depth;
- potential phosphatase activity by depth;
- tissue nutrient concentrations bulked over four intervals.

These are not direct nutrient-transfer equations. In this observation matrix they provide no direct row on the four unknown pathway rates, so the current trait-only set has rank 0 for this flux vector. Even a hypothetical aggregate total-uptake series with varied colonisation fractions would have rank only 2; it cannot distinguish mycorrhizal inorganic from organic uptake or estimate carbon cost.

This is a **structural** result for the declared observation model. Real AFEX may have additional constraints or different pathway relationships; no raw flux data are ingested.

## 4. Measurement design implied by the rank deficiency

The exact synthetic cases show which additional observations resolve which unknowns:
1. A source-specific inorganic fungal uptake tracer distinguishes fungal inorganic acquisition from the combined uptake.
2. A host-to-fungal carbon allocation tracer identifies fungal carbon cost separately.
3. To test the organic pathway as distinct from the inorganic route, retain source-specific organic nutrient uptake or labeled organic-substrate observations and verify the assumed common-rate response.
4. Repeated plot-paired stock/productivity and depth-matched uptake observations are still required for turnover and process closure.

Rank alone does not rank survey value. The existing decision-value owner (#5223) should choose among candidate measurements using expected improvement in a stated ecological decision and the full field/assay cost. Practical identifiability under measurement error and correlated samples composes with #5360.

## 5. Executable v0.1

The machine-readable fixture [cases/afex-root-pathway-identifiability-synthetic-v0.1.json](cases/afex-root-pathway-identifiability-synthetic-v0.1.json) has four matrices with expected exact ranks 0, 2, 3 and 4, nullities 4, 2, 1 and 0, and explicit nullspace witnesses. The Rust validator computes exact rational rank and verifies every declared witness. It refuses claims that current AFEX colonisation/trait records establish pathway fluxes.

Run from repository root:

```bash
cargo run --manifest-path research/tropical-forest-qualification-v0.1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0.1/cases/afex-root-pathway-identifiability-synthetic-v0.1.json

cargo test --manifest-path research/tropical-forest-qualification-v0.1/validator/Cargo.toml
cargo fmt --manifest-path research/tropical-forest-qualification-v0.1/validator/Cargo.toml -- --check
cargo clippy --manifest-path research/tropical-forest-qualification-v0.1/validator/Cargo.toml --all-targets -- -D warnings
```

## 6. Claim ceiling

A synthetic pass proves exact matrix rank and the declared nullspace structure only. It does not estimate AFEX pathway fractions, prove the shared-rate model, establish fungal organic uptake, estimate mycorrhizal carbon cost, or validate the utility of a particular field assay. All pathway-specific measurements described here are future measurement candidates, not current AFEX evidence.
