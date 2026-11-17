# Living World Process Activation Atomicity V0

Status: normative design contract; documentation only.

## Purpose

Once representation fidelity is determined by enabled authoritative processes, the process set itself becomes future-bearing policy state.

Enabling a process and obtaining the information/authority it requires cannot be separate observable transitions.

## Core invariant

> A newly enabled authoritative process becomes schedulable only in the same canonical commit that establishes a representation/context sufficient for that process set.

## Unsafe sequence

Forbidden:

1. mark process enabled;
2. scheduler can execute it;
3. discover current representation is insufficient;
4. promote afterward.

Even a one-tick interval violates authority semantics.

## Process-set generation

The enabled semantic process set has a monotonic/non-reused identity, conceptually `ProcessSetGeneration`.

Any prepared operation whose sufficiency proof depends on the process set binds that generation, including:

- Level-A realization;
- population promotion/refinement;
- collapse/demotion;
- reaction/settlement plans where applicable;
- canonical fidelity selection.

A committed semantic process-set change invalidates plans prepared under the previous generation.

## Enable transaction

Conceptually:

`REQUEST -> RESOLVE -> EVALUATE -> SELECT -> PREPARE -> COMMIT -> PUBLISH`

Before COMMIT:

- resolve registered process/profile authority;
- compute the candidate final process set deterministically;
- evaluate its information/spatiotemporal requirements;
- select a legal/reachable representation/context;
- prepare required promotion/refinement without publishing new process authority.

COMMIT atomically installs:

- new process-set generation;
- required representation/context authority changes;
- any required provenance/policy identity.

Only afterward is the process schedulable.

## Failure atomicity

Any failure leaves the old process set and old representation/context authoritative.

A failed activation MUST NOT leave:

- speculative promoted state published;
- partial exact-share transfers;
- new active owners;
- incremented process generation;
- partially changed registry/fidelity policy state.

## Disable transaction

Removing a process may reduce information demand, but it does not authorize immediate information destruction.

Any collapse/demotion remains subject to its own admissibility proof: surviving process requirements, history, exact settlement, persistent identity, and future reconstruction constraints.

`process removed` therefore means only `some richer state may no longer be required`.

## Concurrent changes

Same-tick enable/disable requests are canonically composed/arbitrated using stable process identities and explicit policy. Thread completion order cannot determine the final process set.

Retries after acknowledgement loss are idempotent and cannot create multiple semantic generations for one committed request.

## Qualification fixtures

- marginals sufficient for process set P;
- request enabling exact age×condition process;
- process remains unschedulable while richer context is only prepared;
- process generation and sufficient authority appear together at commit;
- simulated promotion failure leaves old process set/context untouched;
- old Level-A plan rejects after process generation changes;
- disabling a process does not auto-collapse an organism carrying surviving history;
- same-tick request ordering does not change final process set;
- retry is idempotent;
- camera/FPS cannot activate/deactivate canonical ecological processes.

## Relationship

This contract makes dynamic process-set changes transactional. Registry/sufficiency contracts answer whether a candidate set *could* execute; fidelity-selection and promotion-provenance contracts determine how a sufficient state can be reached.

Relates to #210, #252, #269, #270, #271, #272, #280, #281.
