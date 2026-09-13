# Thermodynamic Friction Reservation Design v0.1

Status: design precursor for #809; source/static evidence only.

## Goal

Preserve the strongest exactly-once friction theorem when the production world solver is integrated with the private thermodynamic runtime:

`duplicate identity rejection must happen before mechanical mutation`.

The runtime therefore needs an explicit non-terminal reservation phase before the solver applies an impulse.

## Intended lifecycle

`Absent -> RuntimeReserved -> Applied -> Promoted | DiagnosticOnly`

`RuntimeReserved` is not physical evidence and is not written into the core friction receipt. It is an in-flight runtime authority state. A fixed tick MUST NOT finalize while any reservation remains unresolved.

## Reservation binding

A reservation binds all of:

- runtime-minted `fixed_tick`;
- typed `FrictionSolverCoordinates`;
- body A handle;
- body B handle;
- contact point;
- impulse-on-B vector.

The caller may supply only solver-local coordinates and mechanical inputs. It cannot supply `fixed_tick`.

The reservation token must be non-cloneable. Applying the reservation consumes it. A body-handle mismatch, missing reservation, wrong fixed tick, or duplicate coordinates fails before mechanics.

## Transaction rules

1. `reserve_friction_impulse_at(...)` requires an open non-finalizing fixed tick.
2. Reservation rejects if the same `FrictionTransactionId` is already reserved or already present in the private core friction journal.
3. `prepare_finalize`, `validate_prepared_finalize`, and `commit_finalize_and_rotate` all reject while reservations remain.
4. `apply_reserved_friction_impulse(...)` consumes the exact reservation and delegates the mechanical transition to the existing core `apply_friction_impulse_once` primitive.
5. If core evidence construction rejects, its mechanical rollback semantics remain authoritative and the runtime reservation is released.
6. A reservation can be explicitly cancelled only before mechanics; cancellation leaves no physical or diagnostic evidence.
7. Successful application moves authority from the runtime reservation set into the private core friction journal as `Applied`.
8. Existing `apply_friction_impulse_at(...)` becomes a compatibility composition of reserve + apply.

## Non-claims

This design does not yet edit `PhysicsWorld::resolve_contact`, does not replace `j_t.abs() * 0.1`, and does not physically promote off-center or static-boundary friction. It prepares the authority boundary required for that later hot-loop integration.
