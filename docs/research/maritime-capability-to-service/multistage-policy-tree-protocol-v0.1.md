# Multi-epoch non-anticipativity validator v0.1

Companion to [MCSC-R1-POLICY-TREE-001 #7848](https://github.com/Luminous-Dynamics/symtropy/issues/7848), following the one-epoch R1 reference in [#7834](https://github.com/Luminous-Dynamics/symtropy/issues/7834).

## Purpose

Before implementing a multistage R1 optimizer, validate the central mathematical requirement independently: at every decision epoch, scenarios with identical observable histories must choose the same action.

This is a **policy validator**, not a multistage optimizer.

## Input contract

Each scenario path contains:
- stable, unique scenario ID;
- frozen instance/service context;
- ordered decision stages;
- one shared epoch/cutoff schedule across the scenario set;
- per-stage SHA-256 commitment to the complete canonical observable state;
- per-stage available-action catalogue, with evidence admissibility, hard-constraint and protected-floor status;
- selected action ID.

The observable-state digest must commit to the decision-relevant evidence payload/value digests, source/vintage identities, rights, authority, freshness and admissibility dispositions. The validator checks digest encoding and uses it as a grouping key; the evidence owner must calculate and authenticate the commitment.

## Information sets and non-anticipativity

An information set is keyed by the tuple (decision_epoch, decision_cutoff, observable_state_digest).

All scenario-stage decisions in one information set must expose the same action catalogue and select the same action. Scenario identity and hidden future outcomes are not part of the key.

A later decision may diverge when the committed observable state changes. A hidden shock that has not become visible must not cause a different earlier action.

## Validation gates

The Rust API validate_non_anticipative_policy_tree checks:
1. non-empty unique scenario identifiers;
2. a non-empty, identical frozen service/instance context;
3. non-empty scenario paths;
4. identical epoch/cutoff schedules across scenarios;
5. strictly increasing epochs and cutoffs within each path;
6. 64-character lowercase hexadecimal SHA-256 observable-state commitments;
7. non-empty, unique action IDs in each stage;
8. selected action exists in the stage catalogue;
9. selected action is explicitly admissible;
10. selected action passes hard constraints and has no protected-floor breach;
11. identical action catalogues for indistinguishable scenarios;
12. identical selected action for every member of an information set;
13. deterministic output independent of scenario input ordering.

The report includes scenario count, stage-decision count, information-set count, shared-information-set count, maximum scenarios per set and sorted per-set membership/action summaries.

## Synthetic negative controls

- Same first-stage observation, different hidden future shock: common first-stage action required.
- Divergent second-stage observable states: distinct second-stage actions permitted.
- Same information-set digest with different selected actions: rejected.
- Same information-set digest with different action catalogues: rejected.
- Selected action absent, indeterminate, hard-constraint failing or protected-floor violating: rejected.
- Missing stage or mismatched cutoff schedule: rejected.
- Repeated/non-monotone epochs or cutoffs: rejected.
- Malformed evidence-state digest: rejected.
- Reversing scenario input order: byte-equivalent semantic report.

The unit fixtures are synthetic. The public integration test exercises the exported API; no physical port history or operational policy is claimed.

## Scope ceiling

The validator checks a supplied policy tree. It does not generate an optimal policy, compute scenario probabilities, validate transitions or causal effects, aggregate multi-stage service outcomes, prove that a digest represents truthful source information, or authorize a live action.

The next step is a bounded extensive-form R1 enumerator over these validated information sets, with explicit state transitions, cumulative service accounting and a deterministic exhaustive reference for tiny trees.