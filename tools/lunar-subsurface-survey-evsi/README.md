# Lunar subsurface survey selection by decision value

**Research reference — 18 November 2026**

This directory defines a source-scoped decision model for choosing the next geophysical/site-characterization survey. It is not a seismic inversion, radar processor, gravity solver, cave mapper, stability model, or site approval system.

## Files

- scenario.schema.json: JSON Schema Draft 2020-12 for priors, hypotheses, architecture decisions, survey options, outcomes, likelihoods and resource costs.
- examples/synthetic-site-decision.json: synthetic three-hypothesis / three-decision scenario with two candidate surveys and an uninformative negative control.
- scripts/calculate-lunar-survey-evsi.sh: deterministic one-step EVSI and entropy calculator, implemented with jq.
- scripts/check-lunar-subsurface-survey-evsi.sh: fixture and negative-control regression tests.

Run the calculator from the repository root:

    bash scripts/calculate-lunar-survey-evsi.sh \
      tools/lunar-subsurface-survey-evsi/examples/synthetic-site-decision.json

## Why this is needed

Keil et al. (2026), *Evaluating Seismic Ambient Noise Techniques for Imaging Lava Tubes on the Moon*, compare seismic interferometry, H/V spectral ratios, distributed acoustic sensing and rotational measurements. Their synthetic modelling indicates that roof-thickness estimation and detection are not equivalent to recovering the lower boundary / full cavity depth; sensitivity depends on frequency and depth, and multiple complementary techniques may be required. A single best-fit mesh therefore hides important observability and model-form uncertainty.

Source: https://doi.org/10.1029/2025JE009576

The programme must preserve competing hypotheses, including at least: cavity / tube, fractured or layered competent rock, rubble or fracture false positive, and no suitable host. The hypothesis set is study-specific; do not force every site into a "cave / no cave" binary.

## Decision-theoretic formulation

Let H be the set of mutually exclusive and collectively exhaustive hypotheses, D the candidate architecture decisions, and S a candidate survey. Let p(h) be the prior probability, P(o|h,S) the likelihood of outcome o, U(d,h) the declared decision utility, and C(S) the survey cost expressed in the same utility unit.

Current best expected utility:

\[
EU_0 = \max_{d\in D} \sum_{h\in H} p(h) U(d,h)
\]

Probability of an outcome:

\[
P(o|S) = \sum_{h\in H} p(h) P(o|h,S)
\]

Posterior after a valid observation:

\[
p(h|o,S) = \frac{p(h)P(o|h,S)}{P(o|S)}
\]

Expected best utility after the survey:

\[
EU_S = \sum_o P(o|S) \max_{d\in D} \sum_h p(h|o,S) U(d,h)
\]

Gross and net expected value of sample information:

\[
EVSI_{gross}(S) = EU_S - EU_0
\]

\[
EVSI_{net}(S) = EVSI_{gross}(S) - C(S)
\]

The calculator also reports expected entropy reduction, but does not rank by entropy alone. A survey may reduce entropy while leaving the preferred decision unchanged; a different survey may be more valuable because it changes an expensive architecture choice or avoids a poor commitment.

## Input and evidence rules

1. Hypotheses must be mutually exclusive and collectively exhaustive for the declared decision context; prior probabilities sum to one.
2. Every decision supplies one utility value for every hypothesis. No missing hypothesis is silently assigned utility zero.
3. For each survey and each hypothesis, the likelihoods of all listed outcomes sum to one.
4. Outcome likelihoods are P(o|h,S), not P(h|o,S). Keep that distinction explicit.
5. synthetic_assumption, published, and measured inputs stay visibly distinct. Published/measured priors, utilities and likelihoods require source references.
6. Survey costs are expressed in the same utility unit as decision utility. Instrument mass, energy and duration are shown separately; do not convert them to utility with arbitrary unreviewed coefficients.
7. A missing, stale, uncalibrated, corrupted or unavailable observation is not a "no detection". It yields no posterior update unless the scenario explicitly models an observation process for that failure state.
8. Correlated sensor observations cannot be multiplied as independent likelihoods. A joint multimodal survey needs a joint outcome likelihood or a separately validated conditional model.
9. The calculator evaluates each survey option as a one-step candidate from the same prior. It does not yet model sequential adaptive surveys, correlated survey sequences, dynamic resource constraints, or uncertainty over the prior/likelihood parameters.

## Synthetic example: known answers

The committed scenario intentionally uses arbitrary utility points and likelihood assumptions. It is not a lunar sensor-accuracy estimate. Its deterministic known answers are:

| Candidate | Gross EVSI | Cost | Net EVSI | Expected entropy reduction |
|---|---:|---:|---:|---:|
| Passive seismic fusion | 15.3125 | 8.0 | 7.3125 | ~0.5622 bits |
| Orbital radar sounding | 7.6250 | 5.0 | 2.6250 | ~0.3132 bits |
| Uninformative sensor control | 0.0000 | 0.5 | -0.5000 | 0.0000 bits |

In the toy scenario, the no-survey decision is **engineered_rock_tunnel**. The passive seismic outcome changes the preferred study pathway for cavity-like and no-suitable-host observations, while the uninformative control preserves the prior and baseline decision exactly. These values validate the arithmetic only.

## Paradigm shift: active observability, not passive mapping

The benchmark should compare policies, not only sensors:

- **Fixed schedule:** a predeclared survey order independent of observations.
- **Random/resource-matched:** a controlled baseline for survey budget allocation.
- **Entropy greedy:** choose the next survey with greatest expected entropy reduction.
- **Decision-focused EVSI:** choose the survey with greatest positive net decision value.
- **Expert plan:** a geophysicist's pre-registered survey plan.
- **Symthaea policy:** proposes a next survey or abstains; the independent evaluator computes the score.

A robust trial freezes the geology and ground-truth geometry, hides it from each policy, supplies the same starting evidence and resource budget, reveals valid outcomes from a blind analogue/simulation, and scores the change in decision regret and total resource expenditure. Symthaea cannot supply its own likelihood truth or accept its own recommendation.

## Benchmark dimensions

Report every metric separately; do not hide failures inside a single score.

**Geophysical inference**
- cavity detection precision/recall and false-positive rate;
- roof-thickness error and calibrated interval coverage;
- lateral width, depth and lower-boundary error separately;
- resolution versus depth, spatial scale and survey coverage;
- sensitivity to priors, inversion method and model-form choices.

**Decision quality**
- cumulative expected decision regret against the ground-truth-optimal declared action;
- rate of decision changes when a new observation is actually decision-relevant;
- rate of unnecessary architecture changes under false-positive signals;
- expected utility captured versus fixed-schedule, random and entropy-greedy baselines;
- safe abstention / "insufficient evidence" frequency.

**Resource and operations**
- instrument mass, energy, survey duration and intervention count;
- area/volume surveyed per mission-hour and mission-Wh;
- expected net EVSI per kg, Wh and hour, shown as separate ratios;
- missed-data, sensor-dropout, recalibration and recovery burden.

**Calibration and robustness**
- prior and posterior calibration on held-out analogue sites;
- prediction interval coverage and sharpness;
- performance under sensor coupling degradation, noise-spectrum shifts and correlated faults;
- robustness to prior misspecification and missing modalities;
- worst-site and worst-geology-regime behavior.

## Physical validation ladder

1. **Arithmetic fixture:** validate Bayes/EVSI calculations and negative controls (this increment).
2. **Synthetic forward model:** freeze cavity, layering, fracture, rubble, roof thickness, depth, noise, coupling and source spectrum; create hidden ground truth.
3. **Earth analogue:** blind cavities and discontinuities with surveyed geometry/lithology and independently measured sensor calibration.
4. **Cross-site transfer:** hold out whole geology, site and acquisition campaigns, not random traces from the same cavity.
5. **Lunar application:** only after source-grounded sensor performance and mission constraints exist; keep lunar transfer uncertainty explicit.

A cavity detection is not a full geometry reconstruction. A geometry reconstruction is not a stability qualification. A stability result is not entry approval or construction authorization.

## Authority boundaries

- **Symtropy:** frozen scenario inputs, deterministic calculation, run receipts and reproducible policy comparisons.
- **Specialist geophysics tools:** sensor processing, inversion and physical measurement interpretation.
- **Specialist rock-mechanics tools:** stability and support analyses.
- **Symthaea:** candidate survey selection, hypothesis tracking, anomaly prioritization and abstention; no self-certification.
- **Mycelix:** provenance, permission and evidence custody; not sensor truth.
- **Independent operators/reviewers:** ground truth, instrument calibration and acceptance.

## Claim ceiling

The committed example is synthetic. The tool calculates conditional decision value from supplied priors, likelihoods, utilities and costs. It does not demonstrate that a survey can detect a lunar cavity, that a candidate void exists, that a roof is stable, or that entry, excavation or construction is safe.
