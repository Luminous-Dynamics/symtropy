# AV Margin-Qualified Pareto Confidence Sequences v0

**Parent:** Symtropy #6840  
**Programme:** Symtropy #6806  
**Consumes:** #6770 / PR #6771 matched metric manifest, hard constraints and point-estimate frontier; #6834 scalar rank confidence sequence; #6753 EVSI, #6564 executor and #6754 blind custody.

## Objective

Add uncertainty-aware, anytime-valid evidence to the policy resource frontier. This is a paired directional e-process evaluator for **margin-qualified Pareto dominance**. It does not compute a weighted aggregate and it does not turn an ordinary point-estimate frontier into a statistical certificate.

## Metric contract

Every shared metric axis must provide:
- raw unit and source direction (maximize/minimize);
- a frozen monotone transform ID and SHA-256 digest mapping raw measurements into a utility score in [0,1], with higher always better;
- provenance: MEASURED, DERIVED_FROM_MEASUREMENT, SYNTHETIC, ESTIMATED or NOT_MEASURED;
- measurement_method and evidence digest when provenance is MEASURED or DERIVED_FROM_MEASUREMENT;
- no-worse tolerance delta in [0,1);
- material-improvement margin epsilon in [0,1).

NOT_APPLICABLE axes must be removed from the common comparison rather than imputed. Missing or unmeasured physical resources cannot be relabeled as zero cost. If any axis is synthetic/estimated/unmeasured, the scorecard labels the result as containing non-measured evidence; it is not a measured-resource frontier claim.

All policies are scored on the same episode/source clusters and fixed outer item order. The first implementation supports finite-cohort random-order mode only. The evaluator verifies the order commitment and replay identity, but independent custody under #6754 must attest that the permutation was truly random and committed before any results were observed.

## Margin-qualified Pareto semantics

For ordered candidate/reference pair A,B and metric k, let D[t] = U[t,A,k] - U[t,B,k] where utility is higher-is-better.

Two hypotheses are tested:

**No-worse within tolerance**

    Z_minus[t] = (D[t] + delta[k]) / (1 + delta[k])

Rejecting the null mean(D) <= -delta[k] certifies that A is not worse than B by more than the declared tolerance.

**Material improvement**

    Z_plus[t] = (D[t] - epsilon[k]) / (1 + epsilon[k])

Rejecting the null mean(D) <= epsilon[k] certifies a material improvement on that axis.

Both transformed values are in [-1,1]. For every ordered pair, issue CERTIFIED_DOMINATES only when the no-worse process crosses on every metric and at least one material-improvement process crosses. The e-process uses the same betting grid and finite-cohort offset as the RCS construction:

    b[t] = max(-0.99, min(1, -S[t-1] / (N-t+1)))
    E[lambda,t] = E[lambda,t-1] * (1 + lambda * (Z[t]-b[t])/(1+b[t]))

Use log wealth throughout. Crossing is recorded permanently when the running maximum reaches 1/alpha_local.

This is **margin-qualified** Pareto dominance, not exact Pareto dominance. Positive tolerances are practical-equivalence assumptions; they must be declared before evaluating candidates. Approximate dominance edges can form cycles when margins are nonzero. The evaluator therefore reports only directed pair certificates and does not infer transitive closure or force a total order.

## Familywise error

For M policies and K axes, there are 2 * K * M * (M-1) directional e-processes. v0 uses:

    alpha_local = alpha / (2 * K * M * (M-1))

The union bound covers every directional false certificate over all policy pairs, all metrics and all sequential looks, regardless of dependence between axes or policy scores on an item. This is intentionally conservative. More efficient closed-testing or structured alpha allocation requires independent statistical derivation and validation.

## Frontier output

- CERTIFIED_DOMINATES: all no-worse tests and at least one strict-improvement test crossed.
- UNRESOLVED: no directed certificate yet.
- INELIGIBLE: at least one policy fails a preregistered hard gate.
- CERTIFIED_DOMINATED: a policy has at least one eligible incoming certified edge.
- POSSIBLY_NONDOMINATED: no incoming edge is certified. This is **not proof** that the policy is truly Pareto-optimal.

The point-estimate frontier from #6771 remains a separate output. The inferential frontier does not replace it.

## Input and execution

The v1 input requires:
- benchmark ID and integer schema version;
- unique policies and metric IDs;
- an exact finite episode cohort, committed random order and order digest;
- hard-gate eligibility and reason codes for each policy;
- one complete [0,1] transformed utility score per eligible policy, metric and episode;
- preregistered alpha, betting grid, per-axis margins and transform digests.

Run:

    python3 -m unittest discover -s tools/av-anytime-valid-evaluator -p 'test_margin_qualified_pareto.py' -v
    python3 tools/av-anytime-valid-evaluator/margin_qualified_pareto.py --input pareto-workload.json

Hard-gate-ineligible policies do not need fabricated metric scores; they remain in the policy roster and failure ledger, while score rows contain eligible policies only. No certificate may rescue an ineligible policy.

## Explicit v0 limits

- Finite-cohort, fixed-order mode only; no i.i.d. superpopulation mode yet.
- One fixed common metric set per cohort; no axis missingness or metric substitution mid-run.
- Normalization/transforms are digest-bound but their correctness/physical calibration must be independently qualified by the #6771 metrology and provenance contract.
- No streaming job orchestration or live stop control yet; evaluator replays a completed score matrix and emits all looks.
- No exact Pareto optimality claims, no scalar ranking, no transitive closure of tolerance-qualified edges.
- No multi-metric model-rank confidence sequence; #6834 remains the single-scalar rank owner.

## Claim boundary

Passing known-answer tests validates evaluator mechanics under the stated bounded-score, finite-population and custody assumptions. It does not establish physical energy truth, universal superiority, exact Pareto optimality, or an industry SOTA result.
