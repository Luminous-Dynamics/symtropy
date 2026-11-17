# WPT energy-service session state machine v1

This is a proposed protocol state machine. It is not an IEC/ISO/SAE state machine and does not supersede any safety control.

## Normal path

DISCOVERED -> OFFERED -> AUTHENTICATED -> RESERVED -> POSITION_CONFIRMED -> ENERGIZED -> RAMPING_DOWN -> DEENERGIZED -> VERIFIED

Each event is authenticated and sequence-bound to one session. A record must carry a monotonically increasing sequence number, timestamp, previous-event digest and session identity.

## Safety and abort rules

- An unknown, unauthenticated, unauthorized, expired, stale or replayed receiver/session cannot enter ENERGIZED.
- ENERGIZED additionally requires confirmed positioning where positioning is applicable, healthy sensors, ready protection, foreign-object clearance and all hard limits clear.
- Missing calibration, unresolved capability, out-of-envelope position, failed sensor health or a failed safety interlock must inhibit energization.
- A system already ENERGIZED must ramp down and verify de-energization before it can be marked ABORTED or VERIFIED. Do not model an instantaneous software state jump as proof that physical energy/current is zero.
- Loss of a network coordinator must not disable local protection or safe de-energization.
- Direction reversal is a new authorized mode transition. It must not occur while uncontrolled resonant current remains or receiver authority is unverified.
- Receiver departure and segment handover must bind the old and new segment sessions. Overlap is prohibited unless the physical multi-segment mode has a separate qualified profile.

## State transition table

| Current state | Allowed next state | Gate |
|---|---|---|
| DISCOVERED | OFFERED, ABORTED | Source and receiver capability records exist |
| OFFERED | AUTHENTICATED, ABORTED | Typed service, boundaries and capability intersection valid |
| AUTHENTICATED | RESERVED, ABORTED | Mutual identity, authorization scope, freshness and expiry valid |
| RESERVED | POSITION_CONFIRMED, ABORTED | Resource/session reservation valid |
| POSITION_CONFIRMED | ENERGIZED, ABORTED | Position confidence, FOD, sensor health, protection and hard limits pass |
| ENERGIZED | RAMPING_DOWN | Local controller remains within declared operating envelope |
| RAMPING_DOWN | DEENERGIZED | Electrical/field safe-state evidence confirms transfer stopped |
| DEENERGIZED | VERIFIED, ABORTED | Energy ledger and evidence package complete or explicitly unresolved |
| VERIFIED | — | Terminal state |
| ABORTED | — | Terminal state; safe-state evidence required if transfer was previously energized |

## Measurement semantics

The service quantity determines the useful-output meter boundary and method. Battery DC energy, shaft work, process heat and optical radiant energy must remain separate typed results. The unit joule does not make their measurement methods interchangeable.

The service ledger uses one synchronized time window for source input, useful output, auxiliary power and stored-energy change. Missing or uncalibrated terms remain unknown and force UNRESOLVED; the residual must never be allocated to a convenient loss category to make the balance close.
