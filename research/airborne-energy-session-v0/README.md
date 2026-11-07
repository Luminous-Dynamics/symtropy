# Aerial energy session v0

This is the initial schema/fixture seed for [AIRBORNE-ENERGY-SESSION-001 #4080](https://github.com/Luminous-Dynamics/symtropy/issues/4080).

## Ownership

This directory owns the aerial opportunity/session lifecycle only: discover, admit, acquire, track, energize, depart, confirm safe-off, reacquire and recover. It composes, but does not duplicate:

- #3980 site/PCC allocation and ground–air coexistence;
- #3885 aircraft mission energy;
- #3902 stationary aircraft docking;
- #3458 optical link physics/safety;
- #3472 source-to-load metrology;
- #3950 evidence identity and pass composition.

## Files

- `session.schema.json`: closed JSON Schema 2020-12 contract.
- `session.NOT_RUN.template.json`: schema-valid template which explicitly carries no evidence and cannot be interpreted as a pass.
- `session.synthetic-reacquire.example.json`: synthetic positive trace with two distinct power-active intervals separated by departure, safe-off and fresh admission/reacquisition.
- `session.adversarial-revoked-authorization.example.json`: intentionally hostile trace that attempts to enter POWER_ACTIVE with revoked authorization and local protection inhibited. The schema permits hostile evidence to be represented; the future semantic evaluator must reject it.

## Semantic invariants for the forthcoming Rust evaluator

1. Event sequence numbers are unique and contiguous; timestamps are monotonic.
2. Each event's `from_state` equals the previous state; the initial state is UNAVAILABLE.
3. Only the declared state-transition graph is admissible.
4. A transition into POWER_ACTIVE requires matched receiver identity, valid current authorization, valid site-allocation receipt, admitted current envelope, in-domain position, tracking, and `PERMIT_SYNTHETIC_ONLY` in this synthetic-only v0 corpus.
5. A transition into SAFE_OFF requires `CONFIRMED_SYNTHETIC`; an unconfirmed de-energization must not be silently treated as safe-off.
6. Following SAFE_OFF, reacquisition must repeat admission and tracking. An old POWER_ACTIVE permit is never reused across departure.
7. A transition cannot be made safe by an average score. Any invalid power-active admission rejects the whole trace.
8. The energy ledger is recomputed independently: source = source-to-channel loss + channel input; channel input = channel-to-receiver loss + receiver DC; receiver DC = receiver-to-battery loss + useful battery energy; aggregate source = total named losses + useful battery energy + residual.
9. Synthetic arithmetic and transition PASS are not physical qualification or flight authorization.

## Evidence and safety boundary

All current examples are synthetic. The `PERMIT_SYNTHETIC_ONLY` marker is not a hardware permit or interlock input. Local hardware protection must remain independent. No optical/RF power is authorized by this schema. No aircraft control, energized test, laser operation, aviation approval, spectrum authorization or safety certification is implied.
