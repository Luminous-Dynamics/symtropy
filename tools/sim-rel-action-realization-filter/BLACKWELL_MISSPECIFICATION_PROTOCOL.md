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

## 2A. Loss-based generalized-Bayes inference baseline

For a finite discrete model, a caller may supply exact rational Gibbs factors
`w(s,y) \propto exp(-eta * loss(s,y))`. The posterior rule is
`q(s|y) \propto prior(s) * w(s,y)`, normalized independently for each
observation. The implementation accepts these factors directly so arithmetic
and posterior receipts remain exact; the factor-generating loss and temperature
must be calibrated on a development/calibration split and separately versioned.

A negative control sets `w(s,y)=K(y|s)`, which must reproduce ordinary Bayes
exactly. A misspecification arm uses a non-likelihood Gibbs factor and reports
its posterior deviation from the exact working-model posterior and from the
true posterior. This is a finite Gibbs-posterior update primitive, not a
reimplementation of GBOED's full Gibbs expected-information-gain acquisition
or its statistical guarantees.

## 2. Exact channel semantics

A finite observation channel is a stochastic matrix
`K[s, y] = P(Y=y | S=s)`. Each row is non-negative and sums exactly to one.
The prior over states and the terminal loss table are also exact reduced
rationals.

Given a working/assumed channel K, true generating channel T and a
separately supplied posterior rule q(y), the evaluator:

1. Validates that q has the same state/observation vocabularies as K and that
   every posterior is a normalized exact distribution.
2. Chooses the terminal decision from q(y), including explicit abstention.
3. Independently computes the exact working posterior p_K(s|y) and true
   posterior p_T(s|y).
4. Evaluates the q-selected decision under p_T and computes the true-channel
   Bayes oracle/regret.
5. Records observations with positive true probability but zero K probability
   as unsupported; those paths explicitly abstain and are never normalized
   into a fabricated posterior.

This creates three independently switchable conditions:
- **Inference-only:** K=T, but q differs from the exact Bayes posterior under K.
- **Model-only:** q is exact Bayes under K, but K differs from T.
- **Interaction:** both q differs from exact Bayes under K and K differs from T.

For each supported observation, the receipt retains q, p_K and p_T and computes
TV(q,p_K) (inference approximation), TV(p_K,p_T) (model-induced posterior
shift), and TV(q,p_T) (total posterior error). These are weighted by the true
observation frequencies; unsupported mass is reported separately. The
expected multiclass Brier score is computed as
E_{S~p_T}[sum_j (q_j - 1[S=j])^2], not as the squared distance between posterior
vectors.

The receipt also separates nominal model Bayes risk under K's own observation
distribution from q's conditional risk reweighted by T's observation
frequencies. It reports realized policy loss, true Bayes oracle loss, decision
regret, unsupported-observation probability, decision disagreement and
disagreement between q's terminal choice and the exact working-model Bayes
choice.

Mutual information is computed in floating point for acquisition diagnostics
only. Decision, posterior and loss values remain exact rational quantities.

## 3. Exact nominal Blackwell dominance

A supplied more-informative channel A Blackwell-dominates B when there is a
row-stochastic garbling matrix G such that

`B[s,y] = sum_z A[s,z] * G[z,y]`

for every state s and output y. `verify_blackwell_dominance` checks this
identity exactly. The module includes a closed-form signed-rational solver for
the two-state/two-observation case and a bounded general solver for small
channels using rational Fourier-Motzkin elimination and reverse witness
reconstruction.

The general solver eliminates the final target-output column using each
garbling row's sum-to-one constraint. It then solves the remaining bounded
linear feasibility problem, reconstructs an exact rational witness, and
rechecks it with the general verifier. Hard limits are 12 free variables,
25,000 retained constraints and 500,000 elimination pair products. Exceeding a
limit returns an unsupported-size error, never a negative dominance result.
This is an exact small-domain method, not a scalable general-purpose LP solver.

This gives a clean nominal experiment-ordering test without conflating the
mathematical ordering with robustness under misspecification.

The experiment-pair audit reports information preference, nominal model-risk
preference and realized cost-plus-terminal-loss preference. Its
`cost_to_loss_exchange_rate` explicitly converts one experiment-cost unit to
terminal-decision-loss units; the default is one, and sensitivity analysis
should report how choices change over a preregistered range of rates. It also
reports the two-dimensional Pareto relation over (experiment cost, decision
loss) separately for nominal and realized outcomes. For strict cost/risk
trade-offs it derives the exact rational exchange-rate crossover at which the
scalarized preference changes, separately for nominal and realized loss. A
scalar preference must not conceal a Pareto trade-off; report the crossover
and Pareto relation together.

The audit runs the bounded exact feasibility solver in both directions and
retains either a checked garbling certificate, a supported non-dominance
result, or an explicit unsupported-size/arithmetic error. This prevents a
pairwise benchmark from silently treating solver limits as proof of
incomparability. A dominance certificate proves only the nominal channel
ordering; its consequences under a shifted true generator are measured by the
separate decision-loss evaluator.

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

## 8. Constructive decision transfer under exact Blackwell dominance

The new `decision_value::blackwell_transfer` oracle makes the Blackwell
comparison constructive. Given an exact stochastic garbling witness
`G(z,y)` and any terminal rule `delta_B(d | y)` for the less-informative
channel, it builds the randomized rule for the richer observation:

`delta_A(d | z) = sum_y G(z,y) delta_B(d | y)`.

Abstention is transferred as an explicit additional outcome, so the rule's
probability row remains exactly normalized. The evaluator computes expected
loss directly from unnormalized joint masses
`prior(s) K(s,y) delta(d | y) loss(d,s)`; zero-probability symbols are
accounted for without posterior normalization.

For every accepted witness the certificate requires exact equality between
the source rule's risk on the less-informative channel and the transferred
rule's risk on the more-informative channel. It also computes both exact Bayes
risks and verifies that the more-informative channel cannot have greater Bayes
risk. The exact non-negative difference is reported as the decision-specific
value of refinement. This is not an information-theoretic scalar and depends
on the supplied prior and terminal loss table.

Required negative controls include identity garbling (zero refinement value),
malformed randomized rules, incompatible vocabularies, unsupported observation
symbols and a false dominance witness. These certificates establish the
finite-channel theorem under the supplied model; they do not imply monotonicity
when the data-generating or inferential model is misspecified.
