# Beam-Theory Applicability Screen v0.1

**Benchmark ID:** PHYS-BEAM-THEORY-SCREEN-001  
**Status:** heuristic model-selection aid; not a verified solver or physical validation result.  
**Scope:** distinguish when a slender Euler–Bernoulli beam model is a reasonable candidate from when a shear-flexible Timoshenko comparison is warranted or the current input is insufficient.

## Motivation

The model class is part of the numerical answer. A small residual from an Euler–Bernoulli eigenproblem only verifies that the discretized equations were solved consistently; it does not prove that transverse shear or the relevant modal wavelength are negligible. A structurally long member may still be short relative to a high-frequency mode's wavelength.

The screen deliberately uses two separate dimensionless indicators rather than one boolean:

1. **Cross-section-to-axial-scale ratio**

   `r = d_max / L_eff`

   where `d_max` is the user-declared maximum cross-section dimension and `L_eff = min(member_length, shortest_relevant_wavelength)` when a modal wavelength is supplied. The cross-section annotation is not inferred from its symbol: the user must state the dimension unambiguously.

2. **Shear slenderness**

   `S = G A L_eff² / (E I)`

   where `G` is shear modulus, `A` area, `E` Young's modulus and `I` the bending second moment. This follows the dimensionless indicator described for the ANSYS BEAM188 element.

## Source interpretation

- [Abaqus beam-element overview](https://3dexperience.fiir.upb.ro/internal/docv6/english/SIMA3DXELMRefMap/simaelm-c-beamelem.htm) describes Euler–Bernoulli elements as suitable for slender beams and gives cross-section dimensions below about 1/15 of typical axial distances or participating-mode wavelengths as a useful criterion.
- [ANSYS BEAM188 Element Reference](https://ansyshelp.ansys.com/public/Views/Secured/corp/v261/en/ans_elem/Hlp_E_BEAM188.html) uses `G A L²/(E I)` and recommends values above 30 for its Timoshenko element. Its worked cantilever example reports Timoshenko/Euler–Bernoulli deflection ratios of approximately 1.12, 1.06 and 1.03 at indicator values 25, 50 and 100 respectively.
- These are source-specific screening heuristics, **not universal error guarantees** and not a replacement for an independent Timoshenko solve.

## Decision semantics

The executable uses four explicit statuses:

| Status | Gate | Meaning |
|---|---|---|
| `MissingDynamicWavelength` | modal analysis requested but no shortest relevant wavelength is supplied | Unqualified for a modal theory recommendation; no frequency claim |
| `EulerBernoulliOutsideReferenceScreen` | (r > 1/8) or (S le 30) | Outside the cited slender-beam screen; consider Timoshenko or a higher-fidelity continuum model |
| `EulerBernoulliCandidate` | (r le 1/15) and (S > 100) | Euler–Bernoulli is a candidate for further numerical verification, not physically validated |
| `TimoshenkoComparisonRecommended` | valid input falls between the above gates | Run a matched Timoshenko comparison and quantify the model-form difference |

The gap between (1/15) and (1/8), and between shear indicators 30 and 100, is deliberate. The screen does not collapse uncertain/intermediate cases into a pass.

For static/quasi-static cases the report includes an **uncorrected** `3/S` cantilever shear-deflection proxy, but deliberately omits an assumed section shear-correction factor. It is a rough indicator only—not a frequency error prediction, not a structural design formula, and not evidence that the chosen theory meets a particular error tolerance.

## Tests and acceptance

- Independently construct inputs to target shear slenderness indicators 25, 50 and 150; verify the expected three distinct decision outcomes.
- Supply a globally slender member with a short modal wavelength; the shorter wavelength must control the cross-section ratio.
- Omit the modal wavelength; return an explicit unqualified status rather than silently using the full member length.
- Uniformly rescale the geometry (and section (A,I) consistently); both dimensionless indicators and the decision must remain invariant.
- Reject non-finite or non-positive SI inputs.
- Mutation-audit the wavelength selection, missing-wavelength gate, and (L^2) exponent. Each mutant must fail its named assertion in an isolated crate copy, while the unmutated control passes.
- CI must pin the source commit, source digest, screen-case request digest, and generated receipt. No validation credit is inherited from the axial-bar table or beam element-matrix tests.

## Limits / next experiment

This screen does not yet run two beam theories on the same assembled structure. The next experimental deliverable should solve Euler–Bernoulli and Timoshenko cantilevers / released-joint beam meshes with the same (E,G,A,I,ho), boundary conditions and mesh ladder, then report frequency and shape differences separately. A screen recommendation is not a pass until that paired model-form experiment—and, later, an eligible external reference or measured experiment—supports the intended claim.

**Claim ceiling:** model-class screening only. No solver superiority, experimental validation, structural safety or code-compliance claim.
