# SIM-REL-030: experiment ordering under misspecification

Status: exact finite-channel benchmark and test specification. No external
benchmark score or SOTA claim is made by this file.

## 1. Problem statement

An experiment can be more informative under the assumed observation model and
still produce a worse real decision when the observation channel used by
inference differs from the channel generating the data.

SIM-REL-030 separates two axes:

- **Model misspecification:** inference is exact for the assumed channel, but
  the true data-generating channel differs.
- **Inferential misspecification:** the data-generating channel is correct, but
  inference uses an incorrect/approximate channel or approximation.

The interaction arm crosses both. It is important not to relabel all errors as
"model uncertainty"; otherwise a method that fixes one failure mode can appear
to fix the other without evidence.

## 2. Exact channel semantics

A finite observation channel is a stochastic matrix
`K[s, y] = P(Y=y | S=s)`. Each row is non-negative and sums exactly to one.
The prior over states and the terminal loss table are also exact reduced
rationals.

Given an *assumed* channel K and a distinct *true* channel T, the evaluator:

1. Forms the posterior under K for each public observation.
2. Chooses the minimum expected-loss decision under that assumed posterior,
   including explicit abstention.
3. Evaluates the chosen action under the true conditional state distribution
   induced by T.
4. Computes the true-channel Bayes oracle and regret.
5. Records observations with positive true probability but zero assumed
   probability as unsupported; those paths explicitly abstain and are never
   normalized into a fabricated posterior.

The receipt separates nominal model Bayes risk under its own observation
distribution from the assumed conditional risk reweighted by true observation
frequencies. It also reports realized policy loss, true Bayes oracle loss,
decision regret, unsupported-observation probability and the probability that
the chosen decision differs from the true Bayes decision.

Mutual information is computed in floating point for acquisition diagnostics
only. Decision, posterior and loss values remain exact rational quantities.

## 3. Exact nominal Blackwell dominance

A supplied more-informative channel A Blackwell-dominates B when there is a
row-stochastic garbling matrix G such that

`B[s,y] = sum_z A[s,z] * G[z,y]`

for every state s and output y. `verify_blackwell_dominance` checks this
identity exactly. It validates a supplied witness; it does not search for one
or claim to solve a linear feasibility problem.

This gives a clean nominal experiment-ordering test without conflating the
mathematical ordering with robustness under misspecification.

## 4. Executable two-state counterexample

Use prior P(S=0)=P(S=1)=1/2 and zero-one terminal losses with abstention loss
1/2.

- Experiment A is a perfect binary channel. Its nominal mutual information is
  exactly one bit.
- Experiment B is a symmetric 80%-accurate channel. A is more informative;
  B is a stochastic garbling of A. B's nominal mutual information is
  approximately 0.278 bits.
- Under a sensor inversion, A's true channel maps every observation to the
  opposite state while inference still assumes the perfect nominal channel.
  The selected decision is always wrong: assumed loss 0, realized loss 1,
  decision regret 1.
- B is left correctly specified. It yields realized loss 1/5 and zero regret.

Thus the nominal EIG ranking (A above B) is reversed by realized downstream
decision loss under model misspecification. These are exact finite probabilities
except for the displayed floating-point information diagnostic. This is a
constructed counterexample, not a physical experiment result.

The same channel evaluator also isolates inference-only error (true channel
correct, assumed channel inverted), model-only error (assumed channel correct,
true channel inverted), and a correctly specified negative control.

## 5. Required extensions

- Replace the perfect/noisy toy pair with a family of exact channels where
  Blackwell dominance can be certified by explicit garbling witnesses.
- Search for counterexamples where A and B have unequal cost, multiple terminal
  decisions, asymmetric loss and abstention.
- Cross model-only and inference-only perturbations with state-dependent
  readback failure, apparatus drift and partial execution.
- Include overlap and out-of-support observations; unsupported probability must
  be reported separately from decision error.
- Compare EIG, EGIG, the paper's inferential-error and model-error penalties,
  nominal decision-value DP, SIM-REL-028 minimax/TV planners, UAI-2026
  adversarial decision-aware design, COPEx and R-IDeA on identical channel
  instances and cost/risk budgets.
- Publish exact channel matrices, garbling certificates, priors, losses,
  inference versions, generator receipts, all seeds and every failed trial.

## 6. Acceptance and claim boundary

- All channel rows and priors validate exactly.
- Every claimed nominal Blackwell relation has a checked stochastic garbling
  witness.
- Tiny examples match independent enumeration of all terminal decisions.
- Model and inference perturbations can be enabled independently and together.
- Unsupported observations trigger explicit abstention and remain visible in
  the receipt.
- Report nominal EIG, nominal predicted loss, realized loss, regret,
  disagreement and unsupported-observation probability separately.

This benchmark does not prove that Blackwell monotonicity is universally false,
nor does it refute the theorem under its correct-model assumptions. It tests the
failure of nominal rankings under controlled misspecification. It is not a
measurement of physical transfer or superiority to published methods.

## 7. Literature anchor

Callahan & Catanach, *On the Misinformation in a Statistical Experiment*,
AISTATS 2026:
https://proceedings.mlr.press/v300/callahan26a.html

Harikumar et al., *Robust Bayesian Decision Making under Adversarial
Uncertainty*, UAI 2026:
https://proceedings.mlr.press/v337/harikumar26a.html
