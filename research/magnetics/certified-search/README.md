# Certified robust magnetic design search (v0.1)

A small branch-and-bound reference that uses interval forward-mode AD from the adjacent interval-ad crate to search bounded magnetic design parameters under a declared nuisance/operating envelope.

## Scope

The objective and compliance expression have separate provenance IDs. The objective expression ID is bound to the root parameter-domain record; the compliance expression ID is an explicit policy input and is copied into every margin evaluation and report.

This v0.1 crate solves a deliberately narrow problem:
- outer prefix parameters are design variables;
- remaining suffix parameters are a fixed bounded nuisance/operating domain;
- one scalar objective is minimized;
- one separately identified scalar robust compliance-margin expression must remain nonnegative over the full design × nuisance box.

The design search subdivides only design variables. The nuisance/operating domain remains whole and is conservatively enclosed at every node. This makes v0.1 simple and safe but can be loose. A later increment can partition nuisance space into a validated exact cover and tighten minimax bounds.

## Semantics

For objective f(x,u) and compliance margin g(x,u), positive g means compliant. Over design box X and nuisance box U:

- objective lower bound on the worst-case loss is the lower endpoint of the interval enclosure f(X,U), since for every x, sup_u f(x,u) is at least every f(x,u), and hence at least the enclosure's lower endpoint;
- a point design x* is a robust-feasible incumbent only when the lower endpoint of g({x*},U) is nonnegative;
- its worst-case objective upper bound is the upper endpoint of f({x*},U);
- a whole design box X is robust-feasible only when the lower endpoint of g(X,U) is nonnegative;
- a whole design box X is safely excluded as robust-infeasible only when the upper endpoint of g(X,U) is strictly negative for the complete product box;
- a design box is objective-dominated only when its objective lower bound is at least the incumbent upper bound.

This uses the complete nuisance envelope in every evaluation. Because v0.1 does not subdivide U, the lower bound can be conservative and the best possible gap may be limited by interval overestimation or nuisance-range width. An unresolved box remains in the report.

## Search policy

- deterministic binary split of one design variable at a time;
- split axis is chosen by the largest normalized contribution from objective or compliance-margin derivative supremum multiplied by design width;
- fixed ID-order tie-break;
- maximum node count, maximum depth and per-design-variable minimum width;
- absolute-plus-relative global gap target;
- separate objective-expression and compliance-expression identities;
- exact node IDs and parent/child path identities.

If no incumbent exists, the report says so. If budget/width limits stop the search, unresolved boxes remain explicit. No queue-empty shortcut is labelled global optimality.

## Global lower bound and gap

The global lower bound is the minimum objective lower bound across the **terminal design partition**:
- objective-dominated leaves (their lower bounds are at least the incumbent upper bound);
- unresolved leaves, including those retained at maximum depth, minimum width, unbounded-expression subdivision limits or node-budget exhaustion.

Split parent boxes are excluded because their children replace them. Robust-infeasible leaves are excluded because they contain no robust-feasible design. The feasible incumbent supplies an upper bound only; its objective lower enclosure is not substituted for the global lower bound. If any terminal non-infeasible leaf has no valid objective lower bound, the global lower bound is absent and no optimality gap is claimed.

Every split child inherits its parent's valid objective lower bound until a tighter child enclosure is obtained. This keeps node-budget exhaustion conservative: queued boxes remain unresolved but still contribute an inherited lower bound when one is available.

A certified gap is reported only when an incumbent exists and:

    incumbent_upper - global_lower
      <= absolute_gap_tolerance
       + relative_gap_tolerance * max(abs(incumbent_upper), abs(global_lower))

This is a global bound for the declared scalar objective and one scalar constraint, conditional on the interval arithmetic/expression contract and nuisance box. It does not imply that the global optimum is attained by the reported candidate.

## Test programme

Analytic one-dimensional minimax cases exercise:
- robust feasible incumbent discovery;
- invalid design-box pruning controls;
- monotone improvement in lower/upper bounds under split;
- no-optimality claims without a feasible incumbent;
- budget/depth/width exhaustion retaining unresolved leaves;
- deterministic split selection and replay.

## Limitations

- Single scalar compliance margin and single scalar objective only.
- One unsplit nuisance/operating box in v0.1.
- No Pareto front, multiobjective dominance, stochastic yield or manufacturing probability.
- No FEM/Magpylib execution and no physical measurement.
- The interval-AD crate supports only its declared algebraic/rational subset plus square root; transcendental and nonsmooth design objectives require separate validated implementations.
- This crate is a research reference, not a formally verified global optimizer or component qualification.

The method is established interval branch-and-bound/validated optimization, not a new invention. The research contribution sought is a reproducible, evidence-bound application to installed magnetic encoder function with operating and manufacturing envelopes.
