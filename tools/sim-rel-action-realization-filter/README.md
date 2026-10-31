# SIM-REL-025 action-realization filter reference

This crate is the first bounded implementation slice for
[SIM-REL-025 / issue #3116](https://github.com/Luminous-Dynamics/symtropy/issues/3116).
It is deliberately a **filtering reference**, not yet a policy optimizer or an
actuator controller.

## Research contract

A command and the intervention that physically occurred are different variables.
The hidden state is the joint tuple:

`(mechanism, process state, sensor state, apparatus state)`.

For a command, a finite joint kernel supplies exact rational outcomes over:

- next mechanism/process/sensor/apparatus state;
- realized intervention class;
- optional execution readback;
- scientific measurement;
- terminal hazard.

The filter conditions on the public observation. Each observation field distinguishes
three cases: unobserved (`Any`), observed missing (`Missing`), and a concrete vocabulary
value (`Value(id)`). This prevents a missing execution acknowledgement from being
confused with telemetry that was simply not supplied to the inference process.
The posterior retains the correlation between the next joint state and the
realized intervention. Its next-step belief marginalizes the last action label only
because all persistent consequences must already be encoded in the next apparatus
and process state.

A model whose future depends on an earlier realized action must include sufficient
history in its apparatus/process state. This is the Markov-sufficiency obligation.

## Included falsification fixture

The binary fixture has `measurement = mechanism XOR execution_error`.
Without execution readback, the measurement leaves the mechanism posterior at
`(1/2, 1/2)`. With a reliable readback, the same measurement identifies the
mechanism exactly. This prevents the filter from treating a commanded intervention
as proof that the intervention occurred.

Additional tests cover dropped interventions, persistent apparatus-state effects
across repeated commands, impossible observations, exact normalization and
malformed kernels.

## Exactness and limits

All probabilities use checked rational arithmetic. Kernel rows and the initial
belief must normalize exactly. Zero-likelihood observations return a typed error;
the filter never normalizes an empty posterior. Arithmetic overflow and malformed
state/action references fail closed.

This reference is exact only for its finite declared kernel, prior and observation
semantics. It does not infer continuous action errors, validate a physical
apparatus, optimize experiment policies, authorize actuation, or certify safety.
The next stages are independent multi-step replay, exhaustive policy-tree
comparison, a bounded risk/cost/abstention optimizer, and integration with the
SIM-REL-013/016 contracts through an explicit versioned adapter.

## Qualification

The dedicated workflow pins Rust 1.96.0 and checks formatting, locked tests,
strict Clippy, exact checkout identity and a clean tree. A passing hosted run is
required before this source is described as qualified.

## Downstream decision-value reference

The `decision_value` module adds an exact one-step Bayes-risk evaluator over
finite terminal decisions. Its loss tensor is indexed by
`(decision, mechanism, physical state, sensor state, apparatus state)`.
Terminal decisions can therefore account directly for specimen condition,
calibration validity and apparatus degradation as well as mechanism identity.
Abstention has its own explicit loss. Every possible policy-visible outcome is replayed
through the exact action-realization filter, and the command cost is charged once
even on a hazard branch. An explicit observation protocol selects whether
readback and measurement channels are visible. If a channel is not acquired, its
outcomes are marginalized into one public branch; if its presence/absence is
visible, a missing value remains a distinct observation. The caller supplies an
explicit hazard loss and receives the exact hazard probability separately.

The counterexample uses prior probabilities `(0.80, 0.19, 0.01)`. The two
ordinary mechanisms share the same optimal terminal decision; a rare critical
mechanism requires a different decision and has a 1,000-unit false-release loss.
The high-information detail probe separates the two ordinary mechanisms but
does not reduce downstream Bayes risk: expected terminal loss remains 0.99.
The lower-information critical-regime screen isolates the rare mechanism and
reduces terminal Bayes risk to zero. Under the same 0.01 experiment cost, the
exact total losses are 1.00 for the detail probe and 0.01 for the critical screen.

For reference, the exact observation-channel entropies correspond to mutual
information of approximately 0.702 bits for the detail probe and 0.081 bits for
the critical screen. These entropy values are descriptive diagnostics; selection
uses the exact rational downstream loss, not a floating-point entropy score.
This is a constructed finite counterexample, not a measured benchmark victory.

## Finite-horizon decision-policy reference

The `decision_value::finite_horizon` module extends the one-step Bayes-risk
evaluator into exact public-observation policy trees. At every history the
planner may stop with the Bayes-optimal terminal decision (including explicit
abstention) or issue another command. Hazard outcomes terminate immediately.

The memoized dynamic programme retains the exact non-dominated frontier over
four separate quantities:

- expected cumulative experiment cost;
- expected terminal decision / hazard loss;
- campaign hazard probability;
- abstention probability.

It supports explicit hazard and abstention budgets when selecting a point from
the frontier. The policy is keyed only by public observations; it never sees
the hidden mechanism, process, sensor, apparatus or realized-action state.
Observation protocols control which channels are visible and ensure that
unacquired telemetry is marginalized.

For tiny cases, the crate includes a **non-memoized exhaustive policy-tree
enumerator** with independent pairwise frontier filtering, plus an independent
policy replay routine. The authored Rust tests compare the full objective
frontier, replay every optimized frontier policy, enforce hazard/abstention
budgets, and check horizon-zero and resource-limit behavior.

This remains an exact finite-model result conditional on the supplied kernel,
prior, full joint-state loss tensor, action costs, observation protocol, horizon
and policy class. It is not a robust multi-world decision optimizer,
open-world model adequacy proof, continuous physical solver, or hardware safety
certificate.


## Exact experiment-ordering under misspecification (SIM-REL-030)

The `decision_value::experiment_ordering` module provides a finite-channel
oracle that keeps the assumed inference channel distinct from the true
data-generating channel. It verifies a supplied Blackwell garbling witness
exactly, updates posteriors under the assumed stochastic channel, evaluates the
selected terminal decision under the true channel, and reports true-channel
Bayes regret and decision disagreement. Unsupported observations trigger
explicit abstention instead of fabricated posterior normalization.

Mutual information is provided only as a floating-point acquisition
diagnostic. All posterior, expected-loss, oracle-regret and unsupported-mass
receipts use exact rational arithmetic.

The authored binary fixture certifies that a perfect channel Blackwell-dominates
a symmetric 80%-accurate channel under the nominal model. The more informative
channel has one bit of nominal information, but when its sensor labels are
inverted in the true world, its nominally zero-loss policy suffers realized loss
1. The weaker channel, left correctly specified, has realized loss 1/5. The
same evaluator separately tests inference-only mismatch, model-only mismatch
and a correctly specified negative control. This is an exact synthetic
counterexample, not empirical physical evidence.

The module validates explicit garbling witnesses; it does not solve the
Blackwell-dominance feasibility problem. EGIG and the paper's model/inference
error penalties remain external comparator implementations to be added under
SIM-REL-030 / issue #3189.

## Fresh-fold sequential conformal calibration

The `decision_value::sequential_conformal` module implements the exact
augmented order-statistic bookkeeping for multistage calibration under
recursive selection. At stage k, with M_k survivor scores and target
conditional coverage beta_k, it computes
`r_k = ceil((M_k + 1) * beta_k)` using exact rational arithmetic. A finite
threshold is the r_k-th smallest survivor score; when r_k = M_k + 1, the
receipt marks the threshold unbounded rather than inventing a finite one.

The routine requires distinct fresh-fold IDs, contiguous stages, declared
score versions, survivor counts bounded by each fold's original size, and
score-fit fold provenance. A stage score may use only calibration folds from
strictly earlier stages; current, future or undeclared fold reuse fails closed.
Each receipt records the augmented rank, threshold (or unbounded status),
sample/survivor count, score version, fit-fold IDs and exact prefix-coverage
lower bound, equal to the product of conditional targets through that stage.

This is only the finite-sample rank calculation under the sequential conformal
paper's sampling and exchangeability assumptions. Its prefix-coverage lower
bound is marginal over repeated training/calibration samples and a new test
case; it is not conditional coverage for one realized calibration set. In
particular, calibration folds must be independent of the score function fitted
at that stage, and fresh-fold survivors must be exchangeable with a surviving
test case conditional on prior history. The result is not a guarantee under
arbitrary nonstationary physical shift, and it is not a calibrated TV ambiguity
radius.
An unbounded threshold is not automatically equivalent to abstention; a
downstream safety policy must decide how to handle an unavailable finite
threshold. The module records provenance but cannot prove that caller-supplied
fold lineage is truthful.

## Deterministic model-family falsification (SIM-REL-029 precursor)

The `decision_value::counterexample_search` module provides a deterministic,
bounded attack surface against one fixed shared policy. It supports explicit
`PerturbationSpec` lists and an auto-generated `BoundedPerturbationGrid`.
The grid enumerates declared split probabilities, mass-shift magnitudes,
measurement inversions, readback dropout, hazard injection, apparatus
degradation and realized-action remapping across a sorted source-world list.
It returns a deterministic ordered set of applicable candidate specs plus
attempted/skipped counts. Input-order permutation does not change candidate
IDs or ordering. A hard attempt cap fails closed rather than silently
truncating a large Cartesian product. Each spec names an immutable source
world, a unique candidate-world ID and an exact mutation.

The grid preflight skips obvious no-op/inapplicable parameter combinations
without cloning the full kernel. The search stage constructs and validates each
candidate kernel, preserving the source state's, action's and observation's
vocabularies so candidates can be appended without silently changing the
meaning of the loss table or public protocol. Search replays the *same fixed policy* over the expanded
family. A newly reachable observation for which the policy has no branch is
recorded as `PolicyInvalidated`; it is never silently normalized or discarded.
Other witnesses distinguish hazard-budget violation, abstention-budget
violation and increased downstream loss relative to the previous family
worst-case. Non-violating candidates and rejected mutations are recorded
separately.

An end-to-end test exercises the loop rather than testing its components only:
a one-world nominal policy is attacked by a persistent sensor-calibration
inversion; the generated world is explicitly appended to the family; the
shared robust planner then chooses a world-identifying probe followed by a
science measurement. In this finite fixture, terminal loss is zero in both
worlds and exact worst-case expected experiment cost plus terminal loss is
`3/20`. This is a constructed oracle fixture, not a measured physical result.

The report retains the original worlds, loss table, observation protocol,
command costs, hazard loss, policy, horizon and risk budgets, alongside the
exact mutation specification and candidate kernel. This makes a discovered
counterexample replayable and allows an explicit later admission/re-plan step.
Candidate IDs are unique within a search report, and candidate insertion is
never performed implicitly.
The `search_policy_against_bounded_grid` entry point connects grid generation
and frozen-policy replay while retaining both the grid and search receipts.
The `search_policy_against_bounded_grid` entry point connects grid generation
and frozen-policy replay while retaining both the grid and search receipts.

This is a finite, enumerated mutation-family falsifier. It is not a learned
adversary, does not cover arbitrary unknown physics, does not establish
statistical model adequacy and does not confer a physical safety guarantee.
The next increment is to add calibrated search domains, provenance/split
receipts, held-out generator families and an explicit admission ledger rather
than to overstate finite mutation coverage.

## Shared-policy robust downstream decision planning (SIM-REL-028)

The `decision_value::robust_finite_horizon` module extends the finite-horizon
planner over a finite family of **persistent complete model worlds**. A world
contains its own exact action-realization kernel and normalized joint initial
belief. The family must share the same hypothesis vocabulary, process/sensor/
apparatus dimensions, command IDs, realized-action IDs, readback IDs and
measurement IDs. One terminal loss table, observation protocol and command-cost
contract apply across the family.

At every public observation history, each still-active world updates its own
joint posterior under its own kernel. The optimizer chooses one command and one
continuation policy for the whole active family. It does not receive a world ID,
a hidden state, or a realized action. Worlds that assign zero likelihood to an
observation become inactive on that branch only; their persistent model identity
is not switched or discarded from other histories.

The exact Pareto frontier retains a full per-world vector of:
- expected cumulative experiment cost;
- expected terminal downstream decision loss;
- campaign hazard probability;
- explicit-abstention probability.

The root can minimize worst-case cost plus terminal loss under worst-case hazard
and abstention budgets. A second selector minimizes nominal cost plus loss while
preserving worst-case hazard/abstention limits and optionally imposing a
worst-case terminal-loss ceiling. Terminal decisions are shared across all
active worlds at each public history. The optimizer uses branch-local model
elimination only where a world assigns exactly zero probability; no independent
world-specific policies are admitted.

A finite-support total-variation ambiguity selector is also provided. Given
nominal categorical weights $q$ over the canonical sorted world IDs and a
radius $\epsilon \in [0, 1]$, it computes the exact worst expected
cost-plus-terminal loss:
`max p · L` subject to `p ∈ simplex` and
`0.5 * sum_i |p_i - q_i| <= epsilon`.
It transfers probability mass from the lowest-loss worlds to the highest-loss
worlds, subject to exact rational arithmetic, simplex capacity and explicit
weight normalization. Radius zero recovers the nominal mixture objective;
radius one recovers the maximum over supplied worlds. Hazard and abstention
budgets remain worst-case across every supplied world, even when the loss
objective uses a TV ambiguity set. An optional terminal-loss ceiling can
additionally bound the loss in every single supplied world, independent of its
nominal weight.

This is finite-support distributionally robust optimization over a declared
world list, not a Wasserstein or continuous-model ambiguity set. It does not
supply a calibrated TV radius: that must be derived and pre-registered from
separate calibration regimes, with adaptive-selection leakage controlled.

A separate shared-policy replay propagates raw, unnormalized forward path
mass directly through kernel outcomes. It does not call the Bayesian filter or
normalize posterior states, so it independently reconstructs per-world
observation-branch mass, command cost, terminal loss, hazard and abstention
receipts. It checks exact public-observation branch coverage, horizon limits
and hazard termination. Tiny-domain exhaustive enumeration has its own
quadratic pairwise dominance implementation rather than calling the production
frontier pruner. Authored fixtures include:
- an instrument whose measurement mapping is reversed in one plausible world;
- a low-cost probe that identifies which mapping is present, followed by a
  shared policy that selects the correct terminal action;
- nominal planning that is optimal in its own world but wrong in the shifted
  world;
- exact robust-DP versus non-memoized exhaustive-frontier equality;
- independent replay of every optimized frontier policy;
- singleton-world reduction to the one-world finite-horizon DP;
- model-world input permutation invariance;
- rejection of incompatible observation and state semantics.

This is exact only for the supplied finite world family, kernels, priors, loss
tensor, command costs, observation protocol, horizon and policy class. It does
not prove that the true physical process belongs to that family, establish
coverage of unmodeled worlds, or certify apparatus safety. A trace impossible
under every supplied world must be treated as out-of-family evidence and sent
to model-family falsification/refinement; it must never be normalized into an
empty posterior. The shared-policy robust planner is a decision-value extension
of SIM-REL-016, not a measured SOTA victory.
