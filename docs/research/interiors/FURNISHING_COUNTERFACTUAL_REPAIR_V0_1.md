# Counterfactual furnishing repair v0.1

**Status:** synthetic known-truth prototype. This module generates advisory single-item translation candidates. It does not approve its own repairs.

## Paradigm shift

The current audit identifies collisions and blocked task/clear zones. The next user-visible step is to suggest the smallest explainable edit that may fix the failure, then rerun the normal independent oracle on the exact revised plan.

This slice targets one common case: a furniture footprint blocks a mandatory, known-truth \`zone_must_remain_clear\` requirement. It enumerates deterministic candidate translations around room, furniture, and protected-zone boundaries. It preserves the item identity, dimensions, and rotation; keeps other placements fixed; filters candidate moves using the supported hard constraints; and ranks them by Euclidean movement distance with stable tie-breaking.

## Fail-closed contract

- This v0.1 search accepts only synthetic-known-truth room geometry and mandatory hard constraints with recognized, supported semantics.
- An unknown room, unknown mandatory requirement, unsupported mandatory requirement kind, invalid fixture, or zero result limit does not produce a recommendation.
- It searches only single-item translations and preserves the current rotation. No global optimum or completeness claim is made.
- Candidate search is bounded to 100,000 position checks. A capped search reports \`SearchCapped\` rather than presenting a partial result as complete.
- Returned suggestions are advisory and explicitly require normal oracle revalidation. Passing the local move filter is not a safety/accessibility/code certificate.
- The objective is least anchor displacement within this finite proposal set, not a universal measure of resident preference, installation effort, or design quality.

## Tests and qualification

The focused tests exercise deterministic replay, relocation away from a blocked route, rechecking the edited fixture with the existing layout oracle, no-op abstention for an already clear zone, unknown real-site geometry, malformed fixture input, and zero search limit.

Acceptance targets are 0 false-safe promotions, exact reproducibility for identical inputs, preservation of all unedited item state, and explicit abstention under unknown mandatory evidence. The fixture is synthetic; the current implementation does not evaluate arbitrary polygons, correlated dimensions, functional-envelope v0.2 shapes, accessibility standards, daylight, cost, or real product/catalogue truth.
