# Human circadian model register v0.1

**Status:** source/equation register; no new circadian model has been implemented by this document.  
**Programme:** [HUMAN-SYSTEMS-SIM-001 / #5625](https://github.com/Luminous-Dynamics/symtropy/issues/5625)  
**Implementation track:** [HUMAN-CIRCADIAN-LIGHT-001 / #5666](https://github.com/Luminous-Dynamics/symtropy/issues/5666)

## 1. Decision: model family, not a single magic circadian equation

The current Symtropy Process C is a fixed-period harmonic waveform. Keep it as an explicitly simple periodic baseline. It has no light input, no phase resetting, and no entrainment dynamics.

The literature contains several related but non-identical human circadian pacemaker models:

| Model family | Core structure | Useful distinction | Status for Symtropy |
|---|---|---|---|
| Fixed harmonic Process C | Prescribed periodic waveform | Transparent, cheap, no entrainment | Existing synthetic mathematical baseline |
| Forger–Jewett–Kronauer 1999 (FJK99 / Forger99) | Two-state van der Pol-type Process P plus one Process-L phototransduction state | Compact three-state light-responsive oscillator | First implementation candidate, parameter profile still to be source-pinned |
| Jewett–Kronauer 1999 (JK99 / Jewett99) | Higher-order limit-cycle Process P plus Process L | Different amplitude recovery and phase response from the simpler oscillator | Separate reference candidate; do not collapse into Forger99 |
| Hannay, Booth & Forger 2019 | Low-dimensional models systematically reduced from a higher-dimensional SCN neural-network model; includes a two-population extension | More explicit physiological derivation and multiscale linkage | Important independent comparator; not automatically the next implementation |

Primary references:
- Kronauer, Forger & Jewett (1999), *Quantifying human circadian pacemaker response to brief, extended, and repeated light stimuli over the photopic range*, DOI: https://doi.org/10.1177/074873099129001073.
- Forger, Jewett & Kronauer (1999), *A simpler model of the human circadian pacemaker*, DOI: https://doi.org/10.1177/074873099129000867.
- Jewett, Forger & Kronauer (1999), *Revised limit cycle oscillator model of human circadian pacemaker*, DOI: https://doi.org/10.1177/074873049901400608.
- Hannay, Booth & Forger (2019), *Macroscopic Models for Human Circadian Rhythms*, DOI: https://doi.org/10.1177/0748730419878298.

The 1999 Process-L paper reports that the prior models did not adequately describe brief bright-light pulses and moderate-intensity light. Its Process L introduces a dynamic stimulus processor before the pacemaker oscillator. It reports characteristic stimulus timescales of roughly 5–10 minutes for bright-light pulse build-up and 30–80 minutes for dark gaps that can be tolerated without an excessive phase-shift penalty. These are source-reported model characteristics, not universal sensor or person parameters.

## 2. Equation register: simplified FJK99 family

The three-state model has two oscillator states, (x) and (x_c), and one light-processing state, (n).

### Process P: oscillator

[
rac{dx}{dt} = rac{pi}{12}(x_c + B)
]

[
rac{dx_c}{dt} =
rac{pi}{12}
left{
muleft(x_c-rac{4x_c^3}{3}ight)
-
xleft[
left(rac{24}{0.99669	au_x}ight)^2+kB
ight]
ight}
]

### Process L: light processing

[
alpha(I)=alpha_0left(rac{I}{I_0}ight)^p
]

[
rac{dn}{dt}=60left[alpha(I)(1-n)-eta night]
]

[
widehat B=G(1-n)alpha(I)
]

[
B=widehat B(1-0.4x)(1-0.4x_c)
]

Here (I) is the model's light-intensity input, (n) is the fraction of photoreceptor elements in the model's activated/used state, and (B) is the phase-modulated drive into Process P. The variables (x), (x_c), and (n) are model states; they are not direct measurements of core body temperature, melatonin, or retinal photoreceptor chemistry.

A public reference implementation documents this equation family and explicitly states that its Forger99 parameter set is taken from Serkh & Forger (2014), rather than the original 1999 paper. Its currently documented reference parameter values are: (	au_x=24.2), (mu=0.23), (G=33.75), (alpha_0=0.05), (eta=0.0075), (p=0.5), (I_0=9500), (k=0.55). These are a named reference implementation profile, not universal human defaults and not to be attributed to the 1999 paper without checking the original paper's parameter table.

Reference implementation and parameter provenance note: https://arcascope.github.io/circadian/api/models.html.

## 3. Equation register: higher-order JK99 family

The revised Jewett–Kronauer model uses a higher-order oscillator. A public reference implementation documents the form as:

[
rac{dx}{dt}=
rac{pi}{12}
left[
x_c+muleft(rac{x}{3}+rac{4x^3}{3}-rac{256x^7}{105}ight)+B
ight]
]

[
rac{dx_c}{dt}=
rac{pi}{12}
left[
qBx_c-
left{
left(rac{24}{0.99729	au_x}ight)^2+kB
ight}x
ight]
]

Process L retains the three-state light-processing equations above, with the parameter set selected for this model family. It is not valid to transplant the Forger99 (mu,G,alpha_0,eta,p,	au_x) values into this model without source justification.

The revised model was developed to better capture amplitude recovery, phase-dependent light sensitivity and Aschoff's rule. Its equation and parameter set must remain separately versioned.

Reference equation/parameter listing: https://arcascope.github.io/circadian/api/models.html.  
Primary paper: https://doi.org/10.1177/074873049901400608.

## 4. Later model family: Hannay, Booth & Forger (2019)

The 2019 paper describes a paradigm in which low-dimensional macroscopic models are systematically derived from a higher-dimensional model of the circadian neural network, with variables and parameters intended to have a clearer physiological interpretation. It fits and validates reduced models against a library of experimental measurements, and also presents a two-population extension representing ventral and dorsal SCN populations.

This is important for our long-term direction: a better model may come from principled model reduction and identifiability, not simply from adding more oscillator terms to an older phenomenological model. We should compare at least one reduced-neural-network alternative to FJK99 and JK99 before declaring the higher-order oscillator the best available model for our context.

Source: https://doi.org/10.1177/0748730419878298.

## 5. Light exposure is a physical measurement problem

A room's horizontal lux value is not equivalent to the light incident at a person's eye. Light-response modelling needs a declared exposure pipeline:

1. environmental spectral power distribution / spectral irradiance;
2. source geometry and location in Symtropy;
3. observer position, head orientation and gaze direction;
4. vertical illuminance at the plane of the eye;
5. obstructions, eyelids, spectacles and pupil assumptions when known;
6. photopic illuminance and relevant alpha-opic quantities, preserving the original spectrum and calculation method;
7. model-specific transformation from the declared light input to Process L.

The FJK99 equation family is commonly written with photopic light intensity (I). Do not silently substitute melanopic EDI, melanopic irradiance, or an arbitrary "circadian lux" scalar for (I). Such a substitution changes the input model and requires an equation/parameter refit or a specifically validated model variant.

CIE's 2024 position statement recommends assessing/predicting light exposure at the plane of the observer's eye (typically vertical and facing gaze direction) and recommends melanopic EDI for describing ipRGC-effective stimulus. ISO/CIE 8995-1:2025 also uses melanopic EDI for ipRGC stimulation assessment. These measurement conventions are useful exposure metadata; they do not automatically change the input semantics of the historical FJK equations.

Sources:
- CIE Position Statement (2024), *Recommending Proper Light at the Proper Time*: https://doi.org/10.25039/PS.b2twa77g.
- ISO/CIE 8995-1:2025 information: https://www.iso.org/standard/8995-1.html.
- CIE S 026 / alpha-opic quantities overview: https://cie.co.at/publications/cie-system-metrology-optical-radiation-iprgc-influenced-responses-light.

## 6. Required model passport

Before implementation, each variant must carry:

- exact model family, publication, equations and errata;
- a source equation/page/section register;
- time variable and unit convention for every derivative;
- state variables, definitions, units and interpretation limits;
- light input quantity, units, spectral assumptions and reference plane;
- complete parameter set with publication, version and calibration provenance;
- initial-condition / entrainment protocol;
- solver, tolerances, maximum step and event handling;
- supported light intensity, duration and timing domain;
- known limitations, unsupported photic/non-photic inputs and model discrepancy;
- exact known-answer / comparison cases and source code or reference implementation version.

No hidden defaults. No mixing parameters across model families. Unknown equation or unit semantics block execution rather than invite plausible guesses.

## 7. Qualification experiments

Synthetic software verification:
- equation-level comparison against an independent source transcription;
- constant-dark free-running period and limit-cycle recovery;
- brief-pulse, extended-pulse and repeated-pulse responses;
- dark-gap recovery and response saturation;
- phase-response curve and phase-dependent sensitivity;
- entrainment and phase shift under fixed, alternating and irregular light schedules;
- convergence under step-size refinement and deterministic replay;
- sensitivity and identifiability of each fitted parameter;
- unit rescaling and invalid-input fail-closed controls.

Experimental/model validation, only with appropriate independent data:
- compare predicted phase markers to measured DLMO or core-body-temperature phase under source-compatible protocols;
- preserve the exact light exposure spectrum, timing, duration and eye-plane geometry;
- use participant-held-out and protocol-held-out validation;
- report prediction intervals, calibration, systematic phase bias and failure conditions;
- do not tune on the held-out participants;
- compare FJK99, JK99, Hannay-derived models and the fixed-harmonic baseline on identical evidence.

A fitted model must not be promoted to an individual's circadian twin or light-therapy recommendation without separate prospective and clinical qualification.

## 8. Initial implementation recommendation

1. Freeze the fixed-harmonic baseline unchanged.
2. Finish equation and unit audit for FJK99 and JK99.
3. Select one parameter profile with exact provenance; do not use "default human".
4. Implement FJK99 as an explicit reference adapter with the original light-input semantics and expose Process P and Process L separately.
5. Add JK99 as a second adapter only after its own equation and parameter audit.
6. Add the 2019 reduced-neural-network model as an independent comparator when its equations and parameter source are fully pinned.
7. Keep physical light exposure and the retinal/input transform outside the oscillator kernel so that the environment model and the biological model remain independently testable.

The first successful software milestone only demonstrates a correct implementation of the declared mathematical model. It does not establish circadian phase prediction accuracy for any individual.
