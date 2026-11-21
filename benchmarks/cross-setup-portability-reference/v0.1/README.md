# Cross-Setup Portability Scorecard v0.1

**Tracking issue:** [SIM2REAL-PORTABILITY-001 #6531](https://github.com/Luminous-Dynamics/symtropy/issues/6531)  
**Single-setup fidelity reference:** [#6529](https://github.com/Luminous-Dynamics/symtropy/pull/6529)

**Status:** M1 deterministic comparison reference. All fixtures are synthetic; no policy benchmark or robot trial has been run.

## Why this differs from sim-to-real fidelity

A simulator may predict its own matched robot's rankings while rankings still change across labs, articles, embodiments or engines. This package measures *winner agreement for the same immutable policy checkpoints on the same task contract* across two declared setups.

A shared task name is insufficient. If the policy checkpoint differs, this is not a paired transfer comparison and the scorecard rejects it.

## Comparison tiers

- `WithinSetupRepeatability`: repeated evaluation of the same team/article/embodiment/kind.
- `MatchedSimulatorTwin`: simulation and physical setup of the same team/article/embodiment.
- `WithinTeamArticle`: different articles of the same robot design at one team.
- `CrossTeamOrLab`: different teams/labs.
- `CrossEmbodiment`: different robot embodiment identifiers.
- `CrossEngine`: two simulated setups with different engine identifiers.

The tier is explicit and cannot be relabeled when setup identities do not satisfy its definition.

## Decision rule

For each common task and each pair of policies, compute 90% Wilson intervals from the success/trial counts. A winner is called only if the intervals do not overlap. Otherwise the matchup is undecided.

The report includes:
- source and target decision coverage;
- joint decision coverage;
- number of paired matchups that agree or reverse;
- winner-agreement and rank-reversal rates over jointly decided matchups;
- Spearman policy-rank correlation across equal-task macro success;
- deterministic 90% percentile bootstrap interval over whole task clusters.

If fewer than ten task clusters contain at least one jointly decided matchup, the inferential interval is absent. An undecided result is not forced into a win or loss.

## Invariants

- Both setups must have the same task-contract digest.
- Policy IDs common to both setups must have the same immutable checkpoint digest.
- Each setup may provide only one aggregate success/trial outcome per task/policy.
- Every common task must contain all common policies in both setups.
- At least ten common task clusters and two common policy checkpoints are required.
- Setup comparison tier must match setup identities.
- Bootstrap resamples tasks, not individual policy pairs, to retain the within-task dependence of multiple pairwise comparisons.

## Limits

Wilson interval non-overlap is a conservative decision rule, not a general paired randomization test. The cluster bootstrap interval describes agreement conditional on the jointly decided matchups and included tasks. It is not a claim of universal ranking stability.

The source/target policy set must be the same immutable checkpoints. This reference does not compare independently trained policies that share a name, nor does it correct for task difficulty or embodiment differences. Those differences remain visible in the setup identity and comparison tier.
