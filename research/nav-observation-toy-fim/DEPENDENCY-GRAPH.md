# Multi-cause failure-domain hypergraph planner v0.1

**Issue:** [NAV-DEPENDENCY-GRAPH-VOI-001 #6720](https://github.com/Luminous-Dynamics/symtropy/issues/6720)

## Why this goes beyond one common-cause flag

A navigation stack can have overlapping dependencies: UWB and camera may share site power; UWB and IMU may share a clock; all sources may share compute; GNSS may share an antenna mount or RF environment. The model represents multiple binary cause variables, allowing overlapping source memberships.

## State and declaration contract

The latent state is the binary service state plus up to four binary failure-domain regimes. With K causes, there are 2 × 2^K joint states; the cap is 32 states, keeping the exact small-instance oracle tractable.

Cause IDs are sorted and uniquely named. Each source declares a sorted, nonempty list of failure-domain IDs. Its acquisition probability and outcome likelihoods are full vectors over the joint state. The validator checks that these vectors are invariant to any cause bit the source did not declare. If dependency inventory is incomplete, include an explicit shared unknown-dependency cause instead of treating missing metadata as independence.

## What is measured

- Shared causes are marginalized only after conditioning on service state, inducing dependence between sources that share a cause.
- Sources whose declared cause sets are disjoint remain independent only under a factorized prior across those latent causes; this assumption can be tested directly.
- An observation updates the posterior over service state and every failure-domain bit.
- Horizon-two planning chooses a different second source per observation branch under hard per-path budgets.

## Known-answer controls

The tests create independent 10% clock and 10% power latent faults. UWB and camera share power; IMU shares clock. The joint probability of two safe readings under unsafe service is more than three times the product of marginal probabilities for the UWB/camera pair, while the UWB/IMU pair factorizes under the independent cause prior. A source made sensitive to an undeclared clock cause is rejected.

## Limits

This is a bounded static latent-factor model, not a general factor graph, real source inventory, empirical common-cause probability estimate or safety case. It validates declared model consistency only. It does not discover dependencies or prove that all common causes are represented.
