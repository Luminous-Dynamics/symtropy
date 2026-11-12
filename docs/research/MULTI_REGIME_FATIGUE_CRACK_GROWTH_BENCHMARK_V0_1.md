# Multi-Regime Fatigue Crack-Growth Benchmark v0.1

**Tracking:** PHYS-FCG-REGIME-001 #4884  
**Parent references:** #4859 / PR #4860 (bounded constant-Delta-K Paris reference); #4862 (dataset admission); #4864 (load-history conditioning); #4867 / #4882 (component-condition and damage-receipt continuity).  
**Status:** research contract. No dataset workbook has yet been admitted or parsed by this benchmark; no model superiority or material validation is claimed.

## 1. Purpose

Qualify crack-growth models for high-strength structural steels without assuming one constant Paris-law fit describes near-threshold, stable propagation, and terminal acceleration.

The current material-damage-reference crate evaluates a deliberately bounded constant-Delta-K Paris law. It is a useful stable-regime analytical reference. It must not be extrapolated into a threshold or instability regime.

This benchmark owns the regime-model comparison and evaluation protocol. It does not replace the existing damage reference, own load-history conditioning, create a general fracture solver, or authorize structural decisions.

## 2. Why the next model must be regime-aware

The University of Strathclyde 22 September 2026 study, High Strength Steel Fatigue Crack Growth Database for Offshore Renewable Energy Applications (DOI 10.1111/ffe.70457), reports that:
- published high-strength-steel crack-growth data are sparse compared with S355;
- high-strength-steel data in corrosive offshore environments are particularly scarce;
- BS 7910:2019 two-stage crack-growth law was unconservative for high-strength steels in air in the reported analysis;
- three-stage fatigue crack-growth behaviour was observed for high-strength steels in air;
- crack-growth rate in that study was not correlated with yield strength.

This is a specific finding from the studied dataset and conditions, not a universal statement about every steel or every test. It is enough to prohibit an unqualified assumption that a two-stage or single Paris fit is universally conservative.

ASTM E647-24 spans near-threshold to Kmax-controlled instability. It emphasises that force ratio, temperature, aggressive environment, loading frequency, waveform and thickness can affect da/dN. Delta-K alone is not sufficient evidence of transferability.

## 3. Regime vocabulary and claim boundaries

A candidate model may distinguish:

1. **Near-threshold / low-growth region:** measured growth approaches a stated detection or threshold level. Preserve censored/non-detect values and the experimental threshold method. Do not turn zero, blank or below-resolution values into exact zero growth.
2. **Stable propagation region:** a Paris-type or other empirically justified relation may be evaluated only within its fitted and admitted interval. The existing constant-Delta-K Paris reference belongs here.
3. **High-growth / terminal region:** growth accelerates as Kmax approaches a material/condition-specific fracture-toughness or instability boundary. A terminal model requires compatible toughness, constraint, geometry and specimen validity evidence; the benchmark does not infer these from Delta-K alone.

Three regimes are a candidate representation, not a mandatory model structure. A curve may lack enough evidence to identify one or more regions. In that case the model must abstain or report the unsupported region, not invent a transition point.

## 4. Dataset owners and admission

Compose the existing dataset owner #4862. Do not create a second data registry or mirror workbook bytes into this repository before admission.

### 4.1 Primary paired dataset

University of Strathclyde, made available 26 October 2026:
- An open database of S-N fatigue data for high strength steels in offshore renewable energy structures
- DOI: 10.15129/d1c049e0-4550-4bed-a697-5fafef0ed626
- workbook: Database.xlsx, 840 KB, CC BY 4.0
- described sheets: source/test context, stress amplitude versus cycles to breakage, and crack-growth rate versus Delta-K
- steel grades described as medium/high strength, S355 and above, across air and corrosive environments.

The official dataset record establishes the stated licence and summary description. It does not replace workbook inspection. Before numerical admission, inspect exact sheets/headers, units, row-level publication identifiers, curve identifiers, material grades, heat/lot information where present, environment, force ratio, thickness, specimen geometry, test method, missingness, duplicate rows, corrections and author-derived columns. Retain source publication provenance for every digitised curve.

### 4.2 Complementary crack-growth dataset

University of Strathclyde, version September 2026, made available 11 September 2026:
- High Strength Steel Fatigue Crack Growth Database for Offshore Renewable Energy Applications
- DOI: 10.15129/983a9e5e-f8e2-4973-b217-9683ec62c965
- workbook: Database.xlsx, 811 KB, CC BY 4.0
- described contents: test/source context and fatigue crack-growth rate versus Delta-K for medium/high-strength steels S355 and above.

This is complementary to the October paired S-N/FCGR workbook. Do not assume that rows in the two workbooks are independent or use matching specimen identifiers until they are reconciled.

### 4.3 Admission rules

- Pin DOI, dataset version, record snapshot, retrieval date, file name/size, licence text, source URL and SHA-256 once bytes are obtained.
- Record whether each value is primary experimental data, digitised literature, author regression, or metadata.
- Preserve missing, rounded, censored, below-resolution and disputed points with explicit dispositions.
- Keep air, seawater/free-corrosion, cathodic protection, temperature and other environments as separate strata when metadata permit.
- Bind material grade/subgrade, product form, weld/HAZ/base metal, heat/lot, force ratio R, Delta-K or Kmax, thickness, waveform/frequency, specimen geometry and measurement method wherever available.
- Do not infer missing test conditions from adjacent rows or publication title.
- No random point-level train/test split. Adjacent observations from one curve/specimen/publication must remain in the same evaluation group.
- CC BY 4.0 requires attribution for public derivatives; retain original source-publication attribution as well as dataset DOI attribution.

## 4.4A Exact NASA/SwRI equation semantics and curve-level uncertainty

NASA/CR-2005-213585 Section 2.7 distinguishes two formulations:

- Its NASGRO-compatible equation uses an effective cyclic driving-force term with a threshold factor involving Delta-K-th and a terminal acceleration factor involving K-max/K-c.
- Its Saxena-Hudak three-component equation combines three reciprocal-rate contributions: a near-threshold Paris-like term, a classic Paris-regime term, and a term that permits acceleration near instability.

The NASA report states that the three-component equation does **not** require a mathematical threshold. In that report, an operational Delta-K-th is inferred at a declared da/dN reporting level (10^-9 inch/cycle, following the cited ASTM practice). Therefore an operational threshold at a stated reporting level and a mathematical no-growth cutoff are different claims. The model can produce very small growth below the reported operational Delta-K-th.

The same report notes that neighbouring da/dN observations are not independent: crack-growth-rate points are derived from a(N) measurements, and the transformation introduces substantial serial correlation. It examines an autoregressive residual model rather than assuming independent errors.

Benchmark consequences:
- Keep the NASA/SwRI three-component equation and NASGRO equation as distinct candidate families; record their exact formula/version, parameters and assumptions.
- Distinguish an operational threshold at a stated da/dN reporting/detection level from a true no-growth cutoff.
- Preserve original crack-length-versus-cycle observations wherever licensing/schema permits; retain the finite-difference/window method used to derive da/dN and Delta-K.
- Propagate crack-length, cycle/time, geometry and load/stress-intensity measurement uncertainties through the derived response where supported.
- Treat a test curve/specimen/publication as the statistical unit for split and resampling. Do not treat adjacent derived points as independent replicate specimens.
- Evaluate curve-level covariance/autocorrelation (for example a declared AR(1) residual profile) against independent residual diagnostics. Do not assume AR(1) is universal; compare alternatives only where data support them.
- Report both pointwise predictive errors and curve-integrated crack-growth/life consequences, retaining the covariance model and its version in the receipt.

## 5. Candidate model tournament

Evaluate all candidate models on identical admitted curve groups and the same fitting budget.

| Candidate | Scope | Prohibited claim |
|---|---|---|
| Single-regime Paris | Stable propagation interval only | No threshold or instability extrapolation |
| Two-regime piecewise law | Two declared intervals with frozen transition rule | Not assumed conservative because it is conventional |
| Three-regime law | Near-threshold, stable propagation, terminal acceleration where data support all three | Not forced onto curves lacking regime coverage |
| Author-reported regression | Reproduced only where input columns, exclusions and fit procedure are reconstructable | Not treated as an independent measurement |
| Monotone/nonparametric baseline | Diagnostic check for fit bias and transition over-regularisation | Not a physical law or extrapolation authority |

For every candidate, report fitting family, parameter count, objective/loss, weights, constraints, transition selection method, parameter uncertainty, valid interval and exact data curves admitted. Transitions are estimated on training groups and frozen before evaluating blind groups.

A piecewise law must declare whether continuity is imposed at a transition. If continuity is imposed, report the constraint and any resulting fit penalty; if discontinuity is admitted, justify it with physical/test evidence. Model selection must penalise unsupported extra regimes and report sensitivity to transition placement.

## 6. Validation design

### 6.1 Leakage-safe group holdouts

Required where support permits:
- leave-one-source-publication-out;
- leave-one-specimen/test-campaign-out;
- leave-one-material grade/subgrade or heat/lot out;
- leave-one-environment-out;
- leave-one-force-ratio band out.

If the data are too sparse for a stratum, report NOT_ESTIMABLE; do not loosen group definitions until a favourable score appears. Pre-register primary and secondary metrics, the split, eligible curves and exclusions before opening the blind set.

### 6.2 Required metrics

Report per curve, regime, grade, environment and force-ratio stratum:
- log10(da/dN) MAE/RMSE and signed bias;
- curve-wise error distributions and worst-group performance;
- transition-location error and transition stability across resamples;
- residual autocorrelation diagnostics and grouped curve-level uncertainty (not an IID point-count surrogate);
- propagated uncertainty from original a(N), cycle/time, geometry and load/SIF measurements where source metadata support it;
- curve-integrated crack-growth/life discrepancy under the declared covariance model;
- predictive interval coverage and width when an uncertainty method is justified;
- false-slow / unconservative error separately from overprediction;
- regime-domain coverage, abstention rate and out-of-domain detection;
- sensitivity to curve digitisation resolution, threshold censoring, sample density and transition selection;
- calibration/validation gap and parameter variability across publication/specimen groups.

Do not publish one pooled average that hides poor corrosive-environment or high-strength-grade results. Report denominators and every excluded/abstained curve.

### 6.3 Metamorphic and negative controls

- Duplicate a curve point: no silent increase in independent evidence count.
- Permute input row order: fitted result and receipt remain deterministic when ordering is explicitly irrelevant.
- Remove or alter a source-publication ID: grouped evaluation must fail closed or quarantine the row.
- Move a holdout specimen into the training group: leakage audit must detect it.
- Put a query Delta-K below/above the admitted regime: explicit out-of-domain response, never extrapolated numeric output.
- Introduce missing R/environment/thickness where required by the selected applicability profile: abstain.
- Force two candidate models to share all data but alter only the terminal boundary: score differences must remain attributable to that change.
- Add an unsupported third regime to a curve with no evidence of transition: complexity penalty / model selection must not reward it as a spurious improvement.

## 7. Integration into Symtropy lifecycle

A future admitted regime-conditioned output should bind:
- physical article/component/configuration/material lot;
- exact source history and geometry/stress-intensity input;
- material/test condition (grade, environment, R, thickness, waveform/frequency where available);
- model/profile/version digest;
- regime identity, fitting interval and transition evidence;
- parameter provenance and uncertainty;
- output artifact digest and applicability disposition.

Compose with:
- #4862 for rights-aware source/data admission and grouped model validation;
- #4864 for source-bound load history/cycle conditioning;
- #4860 for the existing bounded S-N/Miner and constant-Delta-K Paris reference;
- #4882 / PR #4883 for damage receipt admission;
- #4867 / PR #4869 for revisioned component condition.

A crack-growth forecast is a model estimate. It is not a measured crack length, residual-strength assessment, inspection result, remaining-life guarantee, failure verdict, or maintenance authorization.

## 7.3A Distinguish the NASA three-component equation from a piecewise three-region fit

NASA/CR-2005-213585, Section 2.7, presents two different full-range growth-rate formulations:

1. A NASGRO-compatible relation with an effective cyclic stress-intensity term, a threshold factor and a terminal Kmax/Kc acceleration factor.
2. A NASA/SwRI three-component growth-rate equation (attributed there to Saxena and Hudak, 1979/1980), formed from three terms: a near-threshold contribution, a classic Paris-regime contribution, and an acceleration contribution associated with monotonic failure modes near instability.

The report states that this three-component equation does not require a threshold to be used and was considered better suited for probabilistic fatigue-life calculations because it avoids asymptotic difficulties in statistical characterization of near-threshold uncertainty. This is one specified composite equation, not a generic name for all piecewise three-region fits.

The tournament therefore distinguishes:
- **NASGRO full-range equation:** one combined relation with threshold, stress-ratio/crack-opening and Kmax/toughness terms;
- **NASA/SwRI three-component equation:** the particular composite formulation and parameter semantics documented in NASA/CR-2005-213585;
- **empirical piecewise two-/three-region regressions:** separately fitted regimes and training-derived transition points.

These candidates are mathematically different. Pin the exact equation variant, parameter convention, closure/stress-ratio assumptions, transition handling and source implementation before comparison. Do not compare only by calling both “three-stage”.

## 7.4 Specialist solver reference profiles

The comparison set must include the following model families where the solver/version and data semantics can be configured equivalently:

- **Paris:** stable region-II baseline. This is the current bounded #4860 reference; do not treat it as a full-range model.
- **Walker and Forman:** include as candidate conventional alternatives where inputs and assumptions are supported.
- **NASGRO v3/v4:** high-value reference for a combined growth relationship that accounts for threshold, stress ratio/crack closure effects and Kmax relative to fracture toughness. Pin the exact version, material parameters and options.
- **Tabular law:** use where measured curves are supplied directly; preserve interpolation, range and transition conventions.
- **NASA/SwRI three-component equation:** include as its specific composite growth-rate law, distinct from NASGRO and empirical piecewise fits; pin equation variant and parameters.
- **Empirical two-/three-regime piecewise fits:** include only where each region and transition is supported by training curves; do not force regimes on unsupported data.

Current Ansys Mechanical APDL documentation states that its cited built-in XFEM-based fatigue crack-growth implementation is restricted to region II, while the SMART method exposes Paris, Walker, Forman, tabular fatigue law and NASGRO v3/v4 options. It also documents temperature- and stress-ratio-dependent data options and crack-closure models for supported criteria. NASA's current public NASGRO report explains threshold and Kmax/toughness treatment. These product features are baseline context, not proof that every product/version supports every geometry or context in the same way.

The tournament must record exact solver/version/module, license/module availability, input format, geometry/SIF solution, material card, closure and R-ratio options, temperature/environment support, integration method, solver tolerances, cycle integration strategy, output units and applicability constraints. “Software supports NASGRO” is not a reproducible configuration.

## 8. Industry comparison and differentiation

Commercial Ansys nCode DesignLife and SIMULIA fe-safe support much broader integrated fatigue workflows, including FEA coupling and multiple fatigue methods. This benchmark does not claim solver parity or replacement.

The focused differentiator is not broader constitutive-model coverage than specialist tools. It is a reproducible, rights-aware and leakage-safe comparison plus solver-independent evidence/lifecycle continuity that:
- exposes unconservative error instead of hiding it in mean error;
- reports curve/publication/grade/environment uncertainty;
- tests transition stability rather than presenting one fitted curve as universal;
- refuses out-of-domain extrapolation;
- carries exact provenance into component condition and repair history.

A claimed improvement must beat the strongest matched eligible baseline on the pre-registered blind metric without degrading the false-slow risk, calibration, domain-coverage or abstention gates. No numeric SOTA target is set until data admission and holdout support are established.

## 9. Advancement gates (targets, not achieved results)

1. 100% correct admission/disposition on the frozen metadata, units, identity, rights and invalid-input controls.
2. Zero unsupported extrapolation outside the admitted regime/condition envelope.
3. Zero specimen/publication leakage in group split verification.
4. All transition points selected on training data only and frozen for blind evaluation.
5. Report the denominator, error, uncertainty, exclusions and abstentions per regime/condition.
6. Any superiority claim requires an improvement on the pre-registered blind metric with no worsening of the false-slow/unconservative gate.
7. Every output composes with the lifecycle damage receipt without changing evidence class from ModelEstimate to Measured.

## Research and solver references

- Strathclyde 2026 high-strength-steel FCGR database: https://doi.org/10.15129/983a9e5e-f8e2-4973-b217-9683ec62c965
- Strathclyde 2026 paired S-N / FCGR database: https://doi.org/10.15129/d1c049e0-4550-4bed-a697-5fafef0ed626
- Devine et al., 2026, high-strength steel FCGR database and single-/two-/three-stage analysis: https://doi.org/10.1111/ffe.70457
- ASTM E647-24: https://store.astm.org/standards/e647
- Ansys SMART fatigue crack-growth options and temperature/R-ratio/closure features: https://ansyshelp.ansys.com/public/Views/Secured/corp/v261/en/ans_frac/fracSMART.html
- Ansys Mechanical APDL Fracture Analysis Guide (region-II restriction stated for its cited XFEM fatigue-growth implementation): https://ansyshelp.ansys.com/public/Views/Secured/corp/v252/en/pdf/Ansys_Mechanical_APDL_Fracture_Analysis_Guide.pdf
- NASA/CR-2005-213585, *Probabilistic Fracture Mechanics Analysis of the Orbiter's LH2 Feedline Flowliner*, Section 2.7: NASGRO-compatible and NASA/SwRI three-component equations: https://ntrs.nasa.gov/api/citations/20050186899/downloads/20050186899.pdf (Sections 2.7 and 4.3.4 discuss the two equation families and correlated crack-growth-rate residuals)
- NASA NASGRO 2025 fracture/fatigue report: https://ntrs.nasa.gov/api/citations/20250011200/downloads/NASGRO%20FFS%20Final%20Report%20July%202025%20formatted.pdf
- NASA Probabilistic Risk Assessment Guide, NASGRO equation and crack-growth parameter definitions: https://ntrs.nasa.gov/api/citations/20120001369/downloads/20120001369.pdf
- NAFEMS code verification: https://www.nafems.org/publications/code-verification/

## 10. Claim ceiling

This is a research benchmark and bounded model-profile specification. It does not certify steel grades, establish a universal threshold, produce a universal offshore design curve, replace fracture assessment, provide validated inspection intervals, approve continued service or authorize repair. A passing software/benchmark gate does not constitute physical validation beyond the admitted curves and context of use.
