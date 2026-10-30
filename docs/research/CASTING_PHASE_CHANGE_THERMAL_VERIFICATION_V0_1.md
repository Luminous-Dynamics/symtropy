# Casting phase-change thermal verification ladder v0.1

**Status:** Research protocol / benchmark specification. No phase-change solver implementation or physical validation is claimed by this document.

**Parent:** [CASTING-PHASE-001](https://github.com/Luminous-Dynamics/symtropy/issues/2925)

**Purpose:** Establish a small, independently checkable physical reference before attempting coupled casting fill-and-solidification simulation.

## 1. The research shift

The first manufacturing physics target is not a visually convincing casting animation. It is a solver that can predict a moving solid/liquid interface, thermal history and latent-heat release while preserving energy accounting and disclosing uncertainty.

The target workflow is:

```text
versioned material-property evidence
  -> phase-change model profile
  -> geometry + initial/boundary conditions
  -> independently verified thermal solver
  -> phase-front / temperature / enthalpy fields
  -> independent reference comparison
  -> held-out measured-coupon validation
  -> decision evidence for process planning
```

Three capabilities must remain distinct:

```reduced-order freezing-time estimate
!= spatial thermal/phase-change solution
!= coupled flow + solidification + defect prediction
```

Symthaea's calibrated Chvorinov-style model in [CASTING-001A](https://github.com/Luminous-Dynamics/symthaea/pull/8351) is a useful reduced-order baseline. It is not a spatial phase-front solver. Symtropy's ongoing [WCSPH reference implementation](https://github.com/Luminous-Dynamics/symtropy/pull/2891) is a fluid-side foundation; it does not by itself establish thermal or phase-change fidelity.

## 2. Canonical ownership and reuse

Do not create another manufacturing ontology, material authority, temperature/entropy owner, energy ledger, observation store, or machine-execution authority.

| Responsibility | Canonical owner / consumer |
|---|---|
| Process family, recipe, transformation and process-plan identity | Symthaea MFG-PROC ([#5686](https://github.com/Luminous-Dynamics/symthaea/issues/5686)) |
| Material identity, grade/condition and property evidence | Existing Symthaea materials owners |
| Physical thermal state, energy accounting and solver execution | Existing Symtropy physics owners; reuse the established thermal authority |
| Fluid reference model and later advection coupling | Symtropy fluid / [HYDRO-SPH-CORE-001 #2879](https://github.com/Luminous-Dynamics/symtropy/issues/2879) |
| Physical observation, calibration and inspection | FIELD / QIF / metrology owners |
| Lots, production events, custody and cross-organization provenance | Mycelix manufacturing |
| Candidate selection, model calibration, discrepancy and decision analysis | Symthaea |
| Deterministic scenario execution and replay | Symtropy |

An initial code audit is mandatory. If an existing solver already represents an energy or thermal quantity, this work must extend or qualify that owner rather than instantiate a competing state authority.

## 3. Material-profile contract

The solver input must bind a specific material grade, condition and evidence revision. A generic label such as “aluminium” or “steel” is not a process-ready material profile.

At minimum, a profile must identify:

- density and its reference temperature / state;
- solid and liquid heat capacity, including units and applicable temperature range;
- solid and liquid thermal conductivity, including units and applicable temperature range;
- solidus, liquidus and equilibrium/nominal transition temperature semantics;
- latent heat of fusion and its measurement/source basis;
- any composition-dependent or temperature-dependent property law;
- uncertainty, validity range, source record, lot/condition applicability and property revision.

An alloy mushy-zone interpolation law is part of the model profile, not a hidden solver constant. If inputs are missing, stale, contradictory or outside their declared range, the run is BLOCKED or OUT_OF_DOMAIN; it must not silently substitute generic values.

## 4. Governing model and phase semantics

The minimum enthalpy form is:

```text
∂H/∂t = ∇·(k∇T) + q
```

where H includes sensible and latent enthalpy, and any liquid-fraction relation is explicit and bounded to [0, 1]. The inverse enthalpy-to-temperature / phase-state relation must be deterministic, monotone over the admitted profile and testable independently.

The first reference problem is one-dimensional and conduction-dominated. Fluid advection is deliberately disabled until the independent thermal and WCSPH verification lanes pass.

For the sharp-interface reference, report the interface location as a function of time and compare the entire temperature profile against the analytical/semi-analytical reference. Do not infer solidification from an arbitrary post-processing temperature threshold when the model already carries phase state.

The first alloy/mushy-zone model is a later extension. It must not be tuned to make the sharp-interface reference pass.

## 5. Verification ladder

### Tier 0 — input and units

- SI-only internal representation with explicit input-unit conversion at adapters.
- Reject NaN, infinity, invalid sign/range, missing property evidence and incompatible material state.
- Known-answer checks for enthalpy, sensible heat, latent heat and phase-fraction endpoints.
- Check scale consistency and exact model/material/input digests.

### Tier 1 — pure conduction

Use a manufactured solution and/or an exact one-dimensional conduction solution before adding phase change. Verify boundary-flux sign, source-term sign, spatial order, timestep order and integrated energy change.

### Tier 2 — one-phase Stefan similarity problem

Use the semi-infinite one-dimensional freezing/melting reference with a known moving interface. Compare:
- front position at predeclared times;
- temperature profile in the active phase;
- total sensible + latent enthalpy change;
- boundary heat flow and energy residual;
- observed convergence under spatial and temporal refinement.

The interface trajectory follows the classical square-root time structure, (s(t) propto sqrt{t}), with its coefficient fixed by the declared Stefan number and boundary/material conditions. All assumptions and the selected analytical branch must be recorded.

### Tier 3 — more demanding phase-change references

- Two-phase Stefan cases with distinct solid/liquid thermal diffusivities.
- Convective boundary conditions.
- Finite-domain cases where the semi-infinite solution no longer applies.
- Published semi-analytical enthalpy benchmarks in more than one spatial dimension.

### Tier 4 — alloys and variable properties

Only after Tiers 0–3 pass:
- temperature-dependent density, heat capacity and conductivity;
- declared solidus/liquidus range and liquid-fraction function;
- mushy-zone enthalpy regularization / smoothing sensitivity;
- property uncertainty and out-of-domain stress tests.

### Tier 5 — fluid/thermal coupling

Only after the phase-change and WCSPH lanes independently pass:
- advective enthalpy transport;
- fluid/mold contact and heat-transfer boundary conditions;
- consistent transfer of temperature, enthalpy and phase state;
- no double-counting between viscous dissipation, mechanical work and heat sources;
- coupling-step conservation and timestep-stability studies.

Do not add porosity, oxide inclusion, shrinkage, microstructure or defect claims until the underlying flow, thermal and material submodels each have their own validation evidence.

## 6. Independent references and industry comparators

### Analytical and community verification references

1. Neumann / Stefan freezing and energy-conserving phase-change benchmark: [Cryosphere / Copernicus (2021)](https://tc.copernicus.org/articles/15/2541/2021/).
2. Semi-analytical Stefan benchmark for enthalpy methods in arbitrary dimension: [HFF (2022), DOI 10.1108/HFF-09-2021-0647](https://doi.org/10.1108/HFF-09-2021-0647).
3. WCSPH analytical flow check for the later fluid lane: [SPHERIC Test 22 — Taylor–Green vortex](https://www.spheric-sph.org/tests/test-22).

### Measurement programmes

[NIST AM-Bench](https://www.nist.gov/ambench) is a strong model-validation exemplar: controlled measurements, uncertainty-aware challenge problems and public data. AM-Bench 2025 includes metal-AM and polymer-AM measurements; its 2028 planning includes more industrially relevant components. Those observations are an additive-manufacturing evidence population. They may validate an AM profile but must not be relabelled as foundry casting evidence.

For casting, the physical-validation gate needs independently measured foundry coupons or lawful published datasets with exact alloy/lot, mold, geometry, pouring and boundary conditions, synchronized thermocouples or thermal imaging, and microstructure/defect measurements. Public availability alone does not establish data reuse rights or label validity.

### Commercial and external baselines

Commercial tools such as [MAGMASOFT investment casting](https://www.magmasoft.com/en/solutions/investment-casting/) advertise filling, thermal history, solidification and defect-prediction capabilities. This is an industry capability reference, not a ground truth or a claim that Symtropy is equivalent.

Compare tools only on matched geometries, material/property profiles, boundary conditions, physical outputs, compute budgets and validation data. If proprietary result access is unavailable, report the comparison as qualitative capability mapping—not a quantitative win.

Where appropriate, pin and qualify an external FEM/finite-volume solver as a separate reference. Its executable identity, input deck, output semantics, convergence and failure state must be audited. Solver agreement is useful but does not replace experimental validation.

## 6A. Manufacturing digital-twin standards alignment

Use the current ISO 23247 catalogue as an interoperability and assurance mapping, not as a claim of conformity or certification:

- [ISO 23247-5:2026](https://www.iso.org/standard/87425.html), published June 2026, addresses the digital thread across design, planning, production and testing.
- [ISO 23247-6:2026](https://www.iso.org/standard/87426.html), published July 2026, addresses digital-twin composition, including integrated, unified and federated composition.
- [ISO/AWI 23247-7](https://www.iso.org/standard/94859.html) is the approved work item for digital-twin commissioning. Its scope includes initialization/configuration, connection to digital threads, and verification of frequency, latency, fidelity and operational readiness. It is under development.
- [ISO/AWI 23247-8](https://committee.iso.org/standard/94858.html) is the approved work item for manufacturing digital-twin VVUQ. It was approved and registered on 23 October 2026 and is under development.

Keep the current numbering: Part 7 is commissioning; Part 8 is VVUQ. Some older project descriptions still refer to VVUQ as Part 7, but that is not the current ISO work-item mapping.

[NISTIR 8620](https://doi.org/10.6028/NIST.IR.8620), published 21 July 2026, highlights interoperability, VVUQ, cybersecurity and workforce readiness as persistent manufacturing digital-twin challenges. Map Symtropy's evidence receipts, validation lifecycle, exact configuration lineage, cybersecurity controls and operator-facing readiness states against those concerns.

A useful later cross-process interoperability reference is [ISO/TR 23247-101:2026](https://www.iso.org/standard/93278.html), a published robotic multilayer and multipass gas-shielded metal arc welding digital-twin use case. Use it to test whether a common digital thread can represent another process family without duplicating the casting physics model. It is a welding-specific use case, not casting evidence.

## 7. Metrics and acceptance logic

Every case reports raw data and all of the following independently:

| Dimension | Measurement |
|---|---|
| Phase front | Position error versus time; front trajectory error |
| Thermal field | L1, L2 and L∞ temperature error on an explicitly matched grid / interpolation rule |
| Phase state | Liquid/solid fraction bounds, phase mass, latent-heat closure |
| Conservation | Energy residual including boundary heat flux and volumetric sources |
| Numerics | Spatial and temporal refinement, observed order, timestep stability |
| Transfer | Error by held-out material, geometry, boundary regime and campaign |
| Uncertainty | Interval coverage, interval width, calibration drift and abstention |
| Practical utility | Cost and elapsed time per validated decision; not just runtime per time step |
| Reproducibility | Exact input, material, solver, checker and raw-output digests |

A single mean RMSE is not an adequate result. The primary question is whether the model preserves a useful process decision under refinement and held-out conditions—for example, the ordering of freeze times or whether a declared thermal window is violated.

Do not specify one universal front/temperature tolerance for every material and geometry. Freeze tolerances per case from its analytical solution, measurement uncertainty, discretization order and intended decision before looking at candidate outputs.

## 8. Negative, metamorphic and adversarial controls

Required rejection or falsification cases include:

- mixed units or incorrect unit scale;
- missing, stale or wrong-grade material properties;
- non-positive density, heat capacity or conductivity;
- missing/negative latent heat or invalid solidus/liquidus ordering;
- enthalpy inversion that is non-monotone or non-finite;
- liquid fraction below zero or above one;
- wrong boundary-flux sign;
- latent heat counted twice;
- source / advection / viscous heating silently omitted or duplicated;
- unconverged solve presented as a valid result;
- phase front under-resolved by the mesh or timestep;
- held-out alloy/lot outside the admitted property envelope;
- geometry/time scaling that violates the declared similarity relation;
- model revision or raw-input tampering after the run.

The verifier independently recomputes energy closure, phase bounds, digests and score metrics. Candidate code does not certify its own output.

## 9. Evidence classes and promotion gates

Never silently pool these evidence classes:

1. **Analytical:** closed-form or semi-analytical known answer.
2. **Synthetic:** generated oracle cases; useful for software tests, not physical claims.
3. **Independent numerical:** a separate qualified solver or discretization.
4. **Measured coupon:** calibrated physical measurement with traceable conditions and uncertainty.
5. **Production:** repeated production observations, inspection outcomes and operating context.

Promotion:

- **A — analytical correctness:** known-answer cases and independent oracle parity.
- **B — numerical credibility:** conservation, bounds, metamorphic checks and mesh/time refinement.
- **C — independent solver comparison:** matched equations, conditions and output semantics.
- **D — physical validation:** held-out measured coupons grouped by material lot, geometry and campaign.
- **E — shadow-mode usefulness:** prospective decision comparison before process recommendations influence production.

Missing, failed, queued, skipped, stale or unsupported evidence is never PASS. Valid outcomes include BLOCKED, OUT_OF_DOMAIN and INCONCLUSIVE.

## 10. Symthaea ↔ Symtropy research loop

```text
Symthaea
  material/profile evidence + reduced-order hypotheses
  + experiment selection + uncertainty/discrepancy analysis
        |
        | exact versioned process/material/model inputs
        v
Symtropy
  deterministic thermal/phase solver
  + reference execution + replay + raw fields/energy trace
        |
        | exact output and run digests
        v
Independent verifier
  analytical oracle + conservation + convergence + metrics
        |
        | validated residuals / holdout outcomes
        v
Symthaea
  recalibration proposal + transfer assessment + next experiment
```

The optimizer must not use hidden reference outcomes, validation labels or physical holdout data to select candidates. Calibration, validation and final test populations remain separated by material lot, geometry family, campaign and source as appropriate.

Mycelix stores the cross-organization provenance and production-event context; it does not turn a model prediction into a physical observation or confer execution authority.

## 11. Claim ceiling and first exit criterion

The first deliverable is a standalone analytical reference/qualification corpus and a small enthalpy solver only if the Phase 0 audit confirms there is no existing owner that can be extended.

Exit for this initial tranche:

- deterministic Tier 0–2 cases;
- an independent oracle and receipt checker;
- explicit energy/phase-state accounting;
- observed convergence under predeclared refinement;
- all negative controls rejected;
- exact source/data/configuration hashes and reproducible raw output.

This does **not** establish a foundry-qualified casting simulator, certified material model, machine capability, defect-prediction capability, autonomous operation, industry/SOTA superiority or physical production improvement. Those require later independent numerical and physical validation.
