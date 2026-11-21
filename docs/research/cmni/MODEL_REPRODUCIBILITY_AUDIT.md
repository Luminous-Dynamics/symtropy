# CMNI model-reproduction audit: 2026 two-environment screening model

**Snapshot:** 2026-11-21  
**Primary source:** Karahadian et al., *Nature Communications* 17, 8845 (2026), https://doi.org/10.1038/s41467-026-74421-1  
**Tracking issue:** https://github.com/Luminous-Dynamics/symtropy/issues/6482

## Executive finding

The 2026 paper reports a real experimental observation: a low-energy D–D yield plateau in beam-driven metallic hydrides, with proton measurements in the low-energy region and neutron measurements providing validation at higher energies and in the overlap region. The fitted heterogeneous screening description is explicitly phenomenological.

A line-by-line inspection of the published PDF identifies an internal ambiguity in the compact model specification. It affects faithful model reproduction, not the existence of the reported measured counts/yields.

We must preserve the source expression verbatim, implement its literal behavior as one candidate, and separately implement the direction described by the authors' prose/Fig. 3c. Neither candidate should be called the authors' original fit until the archived analysis code and supplementary fit parameters are inspected.

## 1. Source-faithful equations

### Equation 12: homogeneous enhancement

[
mathcal{E}_{mathrm{scr}}(E_{cm};U_e)=
sqrt{rac{E_{cm}}{E_{cm}+U_e}}
expleft[
sqrt{rac{E_G}{E_{cm}}}
-
sqrt{rac{E_G}{E_{cm}+U_e}}
ight].
]

This has the bare limit `E_scr(E; 0) = 1`.

### Equations 13–14: two-environment mixture

[
mathcal{E}_{env}(E)=
A_smathcal{E}_{scr}(E;U_{e,s})+
A_bmathcal{E}_{scr}(E;U_{e,b}),
qquad A_s+A_b=1.
]

This is a mixture in enhancement space. It has two amplitudes (one independent amplitude after the sum-to-one constraint), plus two effective screening potentials. The paper describes the surface and bulk components as candidate local environments, not directly measured microscopic fields.

### Equation 15: equivalent effective potential

[
mathcal{E}_{scr}(E;U_{e,eff}(E))=mathcal{E}_{env}(E).
]

This defines an equivalent homogeneous parameter for the mixture; it is not itself a direct measurement of a local screening potential.

### Equation 16: sigmoid as printed

[
U_{e,eff}(E)=U_{e,b}+
rac{U_{e,s}-U_{e,b}}{1+exp[-(E-E_0)/Delta]}.
]

## 2. Direction inconsistency

Let (E) be center-of-mass energy and (Delta>0).

For the printed Eq. 16:

- As (E	o-infty), the exponent (-(E-E_0)/Delta	o+infty), so (U_{e,eff}	o U_{e,b}).
- As (E	o+infty), the exponential tends to zero, so (U_{e,eff}	o U_{e,s}).

However, the paragraph immediately following Eq. 16 and the paper's Fig. 3c describe the opposite physical direction:

- Low energy / shallow implantation depth approaches the surface value (U_{e,s}).
- High energy / deeper implantation approaches the bulk value (U_{e,b}).

The direction consistent with that prose/figure would use:

[
U_{e,eff}^{prose}(E)=U_{e,b}+
rac{U_{e,s}-U_{e,b}}{1+exp[(E-E_0)/Delta]}.
]

The difference is not cosmetic: it reverses the low-/high-energy asymptotes. The Rust prototype therefore exposes both functions with explicit names:
- `sigmoid_screening_potential_printed_equation`
- `sigmoid_screening_potential_prose_direction`

It does not silently “correct” the paper's equation.

## 3. Parameterization ambiguity

There are two different functional forms in the methods:

| Form | Free parameters if (U_{e,b}) and (Delta) are fixed | Notes |
|---|---|---|
| Eq. 13 mixture | (A_s, U_{e,s}) | (A_b=1-A_s); no (E_0) appears |
| Eq. 16 sigmoid | (E_0, U_{e,s}) | No (A_s) appears in the printed sigmoid |
| Eq. 17 parameter vector as printed | ({A_s,E_0,U_{e,s}}) | Lists three parameters, and (A_s) is not present in Eq. 16 |

The methods text says that fixing (U_{e,b}) leaves two free parameters, while Eq. 17 lists three and mixes a parameter from Eq. 13 with the sigmoid variables of Eq. 16. The paper may contain a typographical omission or the prose may conflate two candidate forms; the public text alone does not establish which implementation produced the plotted fit.

A weighted log-space objective is stated:

[
chi^2(	heta)=sum_i
left[
rac{ln R_i-ln mathcal{E}_{env}(E_i;	heta)}
{sigma_{ln R,i}}
ight]^2.
]

The observable/model naming in Eq. 17 should also be checked against the archived analysis code: the objective is written with (mathcal{E}_{env}), while the surrounding discussion also describes the equivalent (U_{e,eff}) sigmoid. These are not generally algebraically identical if an amplitude is introduced into only one representation.

## 4. Independent implementations to retain

Do not collapse these into one class until the source implementation is resolved:

1. **Homogeneous ALR baseline:** Eq. 12, constant (U_e), fitted to the paper's stated higher-energy range.
2. **Two-environment mixture:** Eq. 13, independent amplitude (A_s), (A_b=1-A_s).
3. **Printed sigmoid:** literal Eq. 16, with its literal asymptotes.
4. **Prose/figure sigmoid:** reverse the exponent direction to match the stated physical limits.
5. **Future depth-resolved model:** integrate local reaction probability through the measured depth profile and implantation/transport fields. This is a new model, not an implementation of Eqs. 13–16.

The code records all five conceptual variants as separate research hypotheses. The Rust prototype currently implements the first four at the enhancement level; it does not yet implement a measured depth profile, a material transport model, or a fitted parameter optimizer.

## 5. Required evidence to resolve ambiguity

1. Inspect Supplementary Information, especially fit tables and the exact definition of parameter (	heta).
2. Inspect the deposited data/fit code after the 2026 Zenodo embargo ends on 2026-12-01, if the files are actually released.
3. Compare the code's equation, parameter vector, parameter bounds, optimizer variables, and generated curves with Fig. 3c and the printed methods.
4. Request author clarification if code and printed methods remain inconsistent.
5. Preserve both literal and prose-direction variants in the benchmark until one is source-verified.

## 6. Numerical tests added to the prototype

- Bare limit (U_e=0 implies mathcal{E}=1).
- Positive screening increases the closed-form enhancement for the tested positive-energy regime.
- Two-environment mixture returns unity when both component potentials are zero.
- Mixture reduces to the surface component at (A_s=1).
- Printed and prose-direction sigmoids have opposite asymptotes.
- Prose-direction sigmoid approaches (U_{e,s}) at low energy and (U_{e,b}) at high energy.
- The crossover energy yields the midpoint between the two potentials.
- Weighted log-space objective is zero for an exact prediction and rejects mismatched input lengths.

The Rust test suite is included in the branch but has not been run in the current environment because Cargo/rustc are unavailable. The equations and limiting cases were independently checked numerically in a separate calculation; this is not a Rust test pass or reproduction of the 2026 data.

## 7. Other method details that the simulator must preserve

From the 2026 Methods:

- (E_{cm}) is the center-of-mass energy.
- The thick-target integral is stopping-weighted:
  [
  Y(E)=N_0int_0^E sigma(E')[-dE'/dx]^{-1}dE'.
  ]
  The stopping power passed to a numerical implementation must be defined against the same energy coordinate. If a source supplies projectile lab-frame stopping for D on stationary D, convert consistently to the (E_{cm}=E_{lab}/2) coordinate.
- The authors report an approximately 5% ion-energy spread. A nominal energy-only fit is therefore an approximation; the beam-energy distribution should be convolved with yield predictions in the next fidelity tier.
- Proton spectra cover 0.25–4.5 keV; neutron spectra cover about 4.5–7 keV, with the overlap used for cross-checking. Reaction yields below 3.0 keV were quantified using the proton channel only.
- Background measurements exceeded 350 h; the reported channel background rates differ substantially. The observation model must use channel-specific background and efficiency.
- (R_{exp}(E_{cm})=Y_{exp}(E_{cm})/Y_{exp}(6.5,keV)). The current reference kernel now includes a relative-yield function, but still has no channel observation model or beam-spread convolution.

## 8. Scientific status

The paper's reported low-energy plateau and beam-driven D–D product measurements remain the observations to explain. The ambiguity above means only that the exact compact curve parameterization is not fully reproducible from the printed equations alone. It does not establish that the measurements are wrong, nor does it justify a cold-fusion/net-energy claim.

**Release rule:** no “reproduces the 2026 heterogeneous fit” label until source code, parameter definitions, raw/processed inputs and figure curves agree.


## 9. Beam-species / molecular-ion energy-mixture confound

The same Methods section reports a typical incident mixture of approximately 85% D⁺ and 15% D₂⁺ and computes the incident deuteron flux by counting two D nuclei per D₂⁺ ion. It also reports an approximately 5% beam-energy spread based on a retarding-potential analyzer.

A single scalar energy spread does not automatically establish the mass-resolved energy distribution of the individual deuterons. For a singly charged ion accelerated through (V) kV:
- D⁺ carries approximately (V) keV total ion energy, so (E_{cm}approx V/2) keV for D on stationary D.
- D₂⁺ carries approximately (V) keV total molecular energy. In the **conditional prompt-dissociation** limit, if its fragments inherit molecular velocity, each D fragment carries approximately (V/2) keV and has (E_{cm}approx V/4) keV.

With 85/15 ion-current fractions, the corresponding prompt-dissociation limiting case places (2(0.15)/(0.85+2(0.15))approx26.1%) of the deuteron flux in the lower-energy fragment component. This is a kinematic sensitivity hypothesis, not a measured fragment distribution and not an explanation of the observed plateau.

The isolated Rust prototype now separates:
- ion-current fractions from deuteron-flux fractions;
- the D⁺ and prompt-D₂⁺ fragment center-of-mass energy components;
- weighted component-yield evaluation;
- arbitrary weighted energy-spectrum samples, suitable for a future measured histogram or quadrature representation without assuming a Gaussian.

Remaining missing physics includes measured mass-resolved fractions vs operating point, molecular survival/dissociation probability and depth, fragment energy/angular distributions, species-specific stopping and implantation, and the detector observation model.

Tracking:
- https://github.com/Luminous-Dynamics/symtropy/issues/6483
- https://github.com/Luminous-Dynamics/symthaea/issues/13034

Do not promote the prompt-dissociation model as the actual source distribution until a mass-resolved beam characterization supports it.


## 10. Eq. 16 / Eq. 17 source-level clarification

A reinspection of the PMC full text (Methods lines 312–334) confirms the exact mismatch:

- Eq. 13: mixture model E_env(E)=A_s E_scr(E;U_e,s)+A_b E_scr(E;U_e,b), with A_s+A_b=1.
- Eq. 16: a sigmoid effective potential with fixed Delta=0.5 keV; after fixing U_e,b, this leaves two variables (E_0 and U_e,s).
- Eq. 17: the printed parameter vector is theta={A_s,E_0,U_e,s} (three parameters), and the residual prediction is written as E_env(E_i;theta).

Thus the prose says two free parameters, Eq. 17 lists three, and Eq. 17 uses the Eq. 13 E_env notation while the preceding text describes fitting the Eq. 16 sigmoid through Eq. 12. The original text does not uniquely specify whether the authors fitted:
1. the Eq. 13 two-environment mixture with A_s,U_e,s;
2. the Eq. 16 sigmoid effective potential with E_0,U_e,s; or
3. a composite model not fully specified in the methods.

The printed sigmoid also has opposite low-/high-energy asymptotes from its adjacent physical description (see Section 2).

Keep these as distinct variants. The Rust reference kernel now exposes both literal Eq. 16 and prose-direction sigmoid enhancement functions, and a separate Eq. 13 mixture. It does not combine A_s with the Eq. 16 sigmoid by assumption.

The public full-text source establishes the mismatch, but the 4.8 MB Supplementary Information PDF / Table 2 and archived fit implementation were not successfully opened as visual/source artifacts in this tool session. Therefore, the actual fit parameter values and optimizer objective still require source-file inspection. The Zenodo data record 10.5281/zenodo.20435774 remains embargoed until 2026-12-01:
https://zenodo.org/records/20435774

Source text: https://pmc.ncbi.nlm.nih.gov/articles/PMC13500673/
