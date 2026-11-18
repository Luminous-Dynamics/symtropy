# Lunar subsurface active survey selection by expected decision value

**Research update — 18 November 2026**

## Objective

Select the next geophysical survey by its expected contribution to a declared architecture decision, not by geometric uncertainty reduction alone.

The programme should maintain competing hypotheses for cavities/tubes, competent-rock corridors, layered or fractured structures, rubble false positives, and unsuitable host geology. A cavity detection is not a complete geometry reconstruction; geometry is not stability qualification; stability is not entry or construction approval.

## Observability evidence

Keil et al. (2026), *Evaluating Seismic Ambient Noise Techniques for Imaging Lava Tubes on the Moon*, compare seismic interferometry, H/V spectral ratios, distributed acoustic sensing and rotational measurements. Their synthetic modelling indicates distinct limits on detection, roof-thickness estimation and lower-boundary recovery; the latter is more difficult, and depth sensitivity depends on the available low-frequency ambient noise. This supports multimodal observability and explicit posterior/model-form uncertainty rather than one deterministic cavity mesh.

Source: https://doi.org/10.1029/2025JE009576

## Executable decision reference

The branch adds a synthetic-only scenario contract and deterministic jq calculator under tools/lunar-subsurface-survey-evsi/. For each candidate survey, the calculator reports:
- prior best architecture-study decision and expected utility;
- probability of each observation under the prior;
- posterior probabilities for each geological hypothesis;
- best decision and conditional expected utility after each observation;
- gross and net EVSI;
- prior and expected posterior entropy;
- separate net-EVSI-per-kg, per-Wh and per-hour ratios.

The key distinction is that entropy reduction is not the objective. A survey may make the posterior narrower without changing the decision. Decision-focused EVSI estimates whether the new information is expected to change the best available action enough to justify survey cost.

All likelihoods, priors, utility points and resource numbers in the committed example are fabricated arithmetic fixtures. They are not lunar sensor performance or a selected architecture.

## Benchmark tournament

Compare policies on identical blinded geophysical scenarios and resource budgets:
1. fixed survey schedule;
2. random / resource-matched policy;
3. greedy expected entropy reduction;
4. decision-focused EVSI;
5. preregistered expert geophysicist plan;
6. Symthaea next-survey policy with an explicit abstain option.

Measure cumulative decision regret, avoided bad commitments, false-positive architecture changes, posterior calibration, detection/geometry metrics, survey mass, energy, time, intervention count and safe abstention. Hold out entire analogue sites, geology, acquisition campaigns and sensor conditions.

## Critical protocol constraints

- Priors, likelihoods, decision utilities and costs each require their own evidence class and source references.
- For each hypothesis, outcome likelihoods sum to one.
- Survey cost utility is in the same declared utility unit as decision utility; physical resource figures remain independently reported.
- Correlated observations require a joint likelihood or validated conditional model; do not multiply marginal sensor likelihoods by assuming independence.
- Missing, stale, uncalibrated, corrupted or unavailable readings are UNKNOWN and do not become negative detections.
- Sequential/adaptive surveys, robust uncertainty over priors/likelihoods, and real sensor forward models are future work; the current calculator is one-step conditional arithmetic only.

## Authority boundary

Symtropy owns frozen scenarios and reproducible evaluation. Specialist geophysics solvers own signal processing and inversion. Rock-mechanics solvers own stability. Symthaea may propose a survey and abstain, but cannot certify its own likelihoods or survey outcomes. Independent physical analogue campaigns own ground truth and instrument calibration.

## Claim ceiling

This is a decision-analysis prototype with synthetic inputs. It does not detect a lunar void, determine a real site's geology, qualify a roof, authorize entry, or validate a construction architecture.
