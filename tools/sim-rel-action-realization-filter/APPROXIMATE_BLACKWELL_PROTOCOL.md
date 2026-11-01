# SIM-REL-031: approximate Blackwell refinement with decision-specific bounds

Status: exact finite-channel oracle and acceptance protocol. No continuous physical abstraction or SOTA result is claimed.

## 1. Purpose

Exact Blackwell dominance supplies a strong order: if channel B is an exact stochastic post-processing of channel A, every randomized decision rule for B can be simulated after A with identical expected loss.

The exact identity is often too strict for validated numerical abstractions, approximate simulators and estimated observation kernels. SIM-REL-031 makes the approximation quantitative rather than either silently treating it as exact or discarding it.

## 2. Finite-channel definitions

Let A and B be finite channels with the same ordered hidden-state vocabulary. They may have different observation vocabularies. Let G(z,y) be a row-stochastic post-processing kernel from A's observation z to B's observation y.

Construct the post-processed channel:

K_tilde_B(y|s) = sum_z K_A(z|s) G(y|z).

For each hidden state s, compute the exact total-variation residual:

epsilon_s = 1/2 sum_y |K_B(y|s) - K_tilde_B(y|s)|.

The audit also reports max_s epsilon_s and whether it is at most the declared maximum radius. The radius is an explicit input, not a fitted guarantee. Residuals outside the declared radius remain visible in the audit receipt but cannot produce an accepted decision-transfer certificate.

## 3. Decision-specific transfer theorem

Let the terminal action set contain every concrete decision and explicit abstention. Let L(d,s) be the loss of action d in hidden state s, and define its statewise span:

rho_s = max_d L(d,s) - min_d L(d,s).

For any randomized policy delta_B(d|y), construct:

delta_A(d|z) = sum_y G(y|z) delta_B(d|y).

For fixed s, the conditional expected action loss as a function of y lies in an interval of width rho_s. The total-variation expectation inequality therefore gives:

| E_A[ L(delta_A,S) | S=s ] - E_B[ L(delta_B,S) | S=s ] |
<= epsilon_s rho_s.

Averaging over the prior pi yields:

| R_A(delta_A) - R_B(delta_B) |
<= sum_s pi(s) epsilon_s rho_s.

Let delta_B* be a Bayes-optimal policy for B. Its lift is a feasible policy for A, so:

R_A* <= R_B* + sum_s pi(s) epsilon_s rho_s.

This is a decision-specific guarantee. A rare state can have a small prior weight but a very large loss span; the bound makes that trade-off explicit. A global max-TV radius alone hides this structure.

The proof is finite and exact. All channel entries, prior weights, loss values, TV residuals, spans and bounds are reduced rationals. Checked arithmetic overflow is an explicit failure.

## 4. Implementation and receipts

The module decision_value::approximate_blackwell provides:

- Exact validation of both stochastic channels, shared ordered state vocabulary, row-stochastic G, prior, loss table and randomized decision rule.
- Exact statewise TV residuals, maximum residual, declared radius and a boolean radius-acceptance result.
- Lifted randomized policies that preserve the explicit abstention mass.
- Raw-joint-mass risk evaluation for source and transferred policies, including zero-probability observation symbols without posterior normalization.
- Per-state loss spans and prior-weighted bound terms.
- Exact absolute risk gap and a check against the decision-specific bound.
- The exact Bayes rule for B, its lifted policy and risk on A, and the Bayes-optimal risk on A.
- The additive Bayes-risk ceiling R_B* + sum_s pi(s) epsilon_s rho_s.

The verifier and certificate builder are separate: callers can inspect exact residuals for a witness outside a declared radius, but the certificate builder rejects it.

## 5. Conformance fixtures

- Exact garbling is the epsilon=0 boundary and must reproduce the exact transfer equality from SIM-REL-030 / PR #3219.
- One-percent row perturbation has statewise TV residuals exactly 1/100.
- Under unit zero-one loss, the deterministic guess policy has risk 21/100 on B and 1/5 after lift to A; the exact transfer gap and decision-specific bound are both 1/100.
- Scaling the terminal loss span by 1/10 scales the bound from 1/100 to 1/1000 without changing the channel residual.
- Randomized policies and explicit abstention must satisfy the same bound.
- A declared radius of 1/200 must not accept an actual residual of 1/100.
- Invalid stochastic kernels, invalid radii, incompatible vocabularies and malformed inputs fail closed.
- Zero-probability source symbols remain represented without fabricated posterior normalization.

## 6. Benchmark programme

The exact oracle suite should enumerate tiny channel matrices, row-stochastic garblings, priors, randomized policies and bounded loss tables. Use rational grids first, then scale state/observation/action counts until a declared resource cap is reached.

Measure:
- exact row-TV residual correctness;
- exact statewise loss-span and weighted-bound correctness;
- observed transfer-gap / theoretical-bound ratio;
- tightness against adversarially selected randomized policies;
- improvement over the global bound max_s epsilon_s times max_s rho_s;
- exact epsilon-zero agreement with the Blackwell transfer module;
- false acceptance outside the declared radius;
- runtime, memory, rational numerator/denominator growth and explicit unsupported-size counts.

Keep all seeds and cases. For larger randomized suites, use paired policy/channel instances and report the full distribution of bound tightness; do not treat random examples as proof.

## 7. Bridge to physical abstraction

The intended longer-term connection is SIM-REL-026's sound continuous-to-finite abstraction. A validated abstraction should provide conservative output-distribution enclosures or a provable channel residual. SIM-REL-031 can then translate that residual into a downstream decision-loss bound.

Do not assign epsilon from empirical fit error and call it a validated physical guarantee. The future continuous integration must derive residuals from validated reachability, interval or set-based propagation, preserve persistent parameter identity and account for unmodeled outcomes.

## 8. Claim boundary

This module establishes an exact finite discrete-channel transfer bound conditional on the supplied channels, G, prior and loss table. It does not prove that the true physical process is covered, that a supplied radius is statistically calibrated, that safety is guaranteed under arbitrary omitted shifts, or that the method beats published systems.

## 9. Relation to classical Le Cam deficiency

The approximate-simulation idea is classical. Le Cam deficiency for a directed pair of experiments minimizes the worst-state total-variation residual over Markov post-processing kernels. Its bounded-loss comparison theorem gives a uniform decision-risk transfer bound. Depending on the convention, sources define total variation as the supremum event difference or as the full L1 norm; constants must be normalized before comparing formulas.

SIM-REL-031 does not claim to introduce approximate Blackwell comparison, Le Cam deficiency, or the general TV-to-risk inequality. It implements a fixed supplied witness and retains the complete statewise residual vector. For a supplied prior and terminal loss table, it computes the sharper task-specific quantity sum_s prior(s) epsilon_s rho_s, where rho_s is the action-loss span in state s. It also constructs and replays randomized policies with explicit abstention using exact rational arithmetic.

Relevant references:
- Le Cam, Comparison of Statistical Experiments, Cambridge University Press (1991): https://www.cambridge.org/core/books/comparison-of-statistical-experiments/405F64D2A2FB3A4E6A1893C1C8A4A60B
- A concise formulation of deficiency and bounded-loss transfer: https://www.statslab.cam.ac.uk/~nickl/Site/__files/MSL.pdf
- Raginsky, Shannon meets Blackwell and Le Cam: Channels, codes, and statistical experiments: https://experts.illinois.edu/en/publications/shannon-meets-blackwell-and-le-cam-channels-codes-and-statistical/
