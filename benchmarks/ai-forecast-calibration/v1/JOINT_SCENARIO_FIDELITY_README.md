# Joint scenario-fidelity known-truth fixtures

**Claim class:** synthetic known truth only. These fixtures are not real workload, renewable, market, weather, storage or thermal observations.

These cases exercise the Energy Score, Variogram Score (p = 0.5), marginal CRPS and Brier score for conjunction events implemented in `symtropy-energy-systems::evaluate_joint_scenario_forecasts`.

## Core adversarial control

Two ensembles have identical marginal distributions for renewable output and workload, but different joint dependence:

- **Dependent scarcity ensemble:** paths (renewable=0, workload=10) and (renewable=10, workload=0), each with probability 0.5.
- **Broken-dependence ensemble:** paths (0,0) and (10,10), each with probability 0.5.

The held-out realized path is (0,10). The event is renewable < 1 AND workload > 9.

| Metric | Dependent scarcity | Broken dependence |
|---|---:|---:|
| Energy Score | 2.5 | 4.5710678119 |
| Variogram Score (p=0.5) | 0 | 10 |
| Renewable marginal CRPS | 2.5 | 2.5 |
| Workload marginal CRPS | 2.5 | 2.5 |
| Joint scarcity probability | 0.5 | 0 |
| Joint scarcity Brier score | 0.25 | 1.0 |

This is the key negative control: marginal CRPS is identical while dependence-sensitive path and event metrics distinguish the ensembles.

## Other vectors

- Tolerated probability-mass drift: raw weights sum to 0.9999999995 under tolerance 1e-9; receipts must preserve raw values and normalized values summing to 1.
- Probability-mass mismatch: raw weights sum to 0.9 and must be rejected.
- Event calibration over two cases: both forecast a 0.25 event probability; observed outcomes are true and false; aggregate frequency is 0.5 and mean Brier score is 0.3125.

## Interpretation limits

Proper scores compare forecast quality over a held-out collection; a single case is only a deterministic arithmetic oracle. The fixtures do not establish that the case set is representative, that the scenario generator is calibrated under distribution shift, or that a downstream optimizer is safe. Preserve matched forecast vintages, actual evidence lineage, dimension scales and event definitions in any real benchmark.
