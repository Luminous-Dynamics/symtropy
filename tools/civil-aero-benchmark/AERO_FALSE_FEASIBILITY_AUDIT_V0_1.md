# Independent false-feasible audit v0.2

## Purpose

The policy benchmark scorecard currently consumes a caller-supplied feasibility boolean and an admission receipt for a successful high-fidelity output. Its own evidence level correctly says that this is a caller-bound admission-receipt scorecard, not solver validation.

This module adds a separate audit over the exact frame of candidates admitted by one policy. Each candidate can be independently adjudicated as:

- `confirmed_feasible`: the independent assessment supports the declared constraints within the candidate's exact scope;
- `falsified_infeasible`: the independent assessment finds a violated constraint or otherwise rejects the prior feasible assertion;
- `inconclusive`: an assessment was attempted but cannot decide;
- `unassessed`: no independent adjudication was supplied.

The module does not change or reinterpret the original admission result. It evaluates the reliability of admitted feasibility claims after the fact.

## Exact candidate frame

The frame includes, for every admitted candidate:

- task, replicate, policy, candidate and high-fidelity operation IDs;
- exact output SHA-256;
- original admission receipt digest;
- original admission verifier ID and executable/source digest;
- exact claim-scope digest.

Candidate IDs and operation identities must be unique. A separate canonical frame digest is computed from the full sorted candidate set. The selection design must reference that exact digest.

Each independent adjudication must match the candidate's exact output and scope. It must name a different adjudicator identity and verifier digest, and use a distinct adjudication receipt from the original admission receipt. This is structural separation only: identities and independence declarations are not authenticated by a trust registry in v0.1.

## Selection designs

- `full_census`: every admitted candidate must have an adjudication record. An adjudication may still be inconclusive; that candidate remains unresolved.
- `pre_registered_probability_sample`: bind the complete candidate sampling frame, selection-plan digest and selection-receipt digest.
- `targeted_challenge_set`: bind the same three elements for a deliberate stress/adversarial subset.

The two sampled designs do not cause an extrapolated point estimate. They merely identify how the selected set was chosen. Unselected candidates remain unassessed and are included in the worst-case upper bound. A future version may add authenticated randomization receipts and design-based confidence intervals, but must not infer representativeness from a label alone.

## False-feasible bounds

Let:

- (N): all candidates admitted as feasible by the policy;
- (F): independently falsified infeasible candidates;
- (C): independently confirmed feasible candidates;
- (I): inconclusive adjudications;
- (U): unassessed candidates.

Then the exact bounds over this frame are:

[
	ext{lower} = rac{F}{N}
]

[
	ext{upper} = rac{F+I+U}{N}
]

The lower bound counts only known false-feasible admissions. The upper bound conservatively treats every unresolved candidate as potentially false-feasible. If (N=0), the rates are undefined, not zero.

The resolved-only fraction is reported separately:

[
	ext{resolved-only} = rac{F}{F+C}
]

This conditional fraction can be selection-biased and must not be presented as the frame-wide false-feasible rate unless the adjudication design justifies that interpretation. The conservative bounds remain primary.

## Output and replay

The report binds:
- exact candidate-frame digest;
- selection-design digest and source selection receipt;
- adjudication-manifest digest;
- counts for confirmed, falsified, inconclusive, unassessed and resolved candidates;
- lower, upper and resolved-only false-feasible rates;
- one CSV row per candidate, including explicit unassessed rows.

Replay recomputes the frame digest, verifies candidate/output/scope links, checks declared adjudicator separation, recomputes the adjudication manifest and rebuilds the report digest.

## Output and replay

The report binds:
- exact candidate-frame digest;
- selection-design digest and source selection receipt;
- adjudication-manifest digest;
- counts for confirmed, falsified, inconclusive, unassessed and resolved candidates;
- lower, upper and resolved-only false-feasible rates;
- one CSV row per candidate, including explicit unassessed rows.

Replay recomputes the frame digest, verifies candidate/output/scope links, checks declared adjudicator separation, recomputes the adjudication manifest and rebuilds the report digest.

## Upstream source-scorecard binding (v0.2)

The downstream audit requires the exact `PolicyBenchmarkRow` produced by `evaluate_policy_benchmark` and the complete operation-ledger slice for that row's task, replicate and policy. A free-standing, self-consistent candidate list is not sufficient source evidence.

The scorecard row carries crate-private canonical fields:
- admitted candidate-frame SHA-256;
- full operation-ledger SHA-256 (successful and failed operations, high- and low-fidelity);
- an admission-frame receipt digest binding those hashes to the group identity, admitted count and scorecard evidence ceiling.

The falsification evaluator:
1. checks that the source row's policy, task, replicate and admitted count match the downstream request;
2. replays the full operation-ledger digest and source admission-frame receipt;
3. independently rebuilds the admitted-candidate frame from the downstream entries and requires an exact digest match;
4. retains source task, replicate, frame, ledger, operation count and receipt digests in the audit report and canonical report digest.

The independent Python oracle receives three separate inputs: the audit CSV, the upstream scorecard CSV and the complete source operation-ledger CSV. It independently recomputes the admitted-frame digest, operation-ledger digest and source admission-frame receipt before verifying the false-feasible bounds and final report digest.

This detects candidate omission and same-count candidate substitution relative to the exact upstream scorecard row, and detects operation-ledger deletion, addition or mutation relative to the source receipt. It does **not** authenticate the original ledger's storage, solver output, admission verifier executable, adjudicator identity or verifier independence. The scorecard and input ledger remain caller-supplied source records; stronger authenticity requires a separately trusted signing/attestation and storage policy.

The schema is `aero-policy-feasibility-audit/0.2.0`; v0.1 reports do not satisfy the source-binding contract.

## What the metric means — and does not mean

A falsified candidate is evidence that the policy admitted at least one false-feasible design under the independent adjudication method and declared scope. A confirmed candidate is not proof of universal feasibility outside that scope. An inconclusive or unassessed candidate is unresolved, not a pass.

The module does not:
- establish independence through cryptographic identity authentication;
- prove that a higher-fidelity solver is truth or that a physical test is representative;
- replace grid, temporal, model-form, uncertainty or experimental validation;
- certify an aircraft, authorize manufacturing or authorize flight;
- create a statistically valid population estimate from a targeted challenge set.

The selection design, adjudication method and validity domain must be preregistered. Physical tests require calibration, repeatability and uncertainty evidence. High-fidelity comparisons require the same scoped operating conditions and a separately verified numerical protocol.

## Benchmark use

For every optimizer policy, export the exact admitted candidate frame and collect independent adjudications at a predeclared budget. Compare policies using:
- false-feasible lower and upper bounds;
- number and fraction of unresolved candidates;
- feasible yield over all high-fidelity attempts, including failed solves;
- validated feasible hypervolume only after the relevant independent validation gate;
- decision regret and worst-decile feasibility;
- full cost of adjudication, solver retries, physical testing and reviewer time.

Use paired tasks and replicates. Keep challenge selection blind to the policy's own ranking where practical. Do not reward a policy for submitting fewer candidates to the adjudication pool without accounting for its unresolved candidates and reduced validated hypervolume.

## Synthetic tests

The tests cover a complete census with an inconclusive result, a sampled frame with unassessed candidates, full-census completeness, exact frame/scope/output matching, verifier/receipt separation and undefined rates for an empty admitted frame.

All test records are synthetic. They qualify the ledger and bounds logic only; they are not aircraft feasibility evidence.
