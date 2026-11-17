# Power Fabric transport-session binding v0.1

**Owner issue:** [#5930 — POWER-FABRIC-SESSION-BINDING-001](https://github.com/Luminous-Dynamics/symtropy/issues/5930)  
**Transport-path owner:** [#5900 / PR #5906](https://github.com/Luminous-Dynamics/symtropy/pull/5906)  
**Session-protocol owner:** [WPT Energy Interface PR #3636](https://github.com/Luminous-Dynamics/symtropy/pull/3636)

## Why this sidecar exists

The WPT Energy Interface already specifies session discovery, service offers, authentication, receiver positioning, authorization, safe energization, metering and termination. Power Fabric's new transport contract separately describes the physical carrier, mechanism, medium, ordered primitive chain, conversion stages and useful-service comparison dimensions.

This sidecar binds a particular session event to a particular transport-path revision. It does not reproduce the WPT session state machine and does not authorize the session.

## Binding graph

The evidence join is:

`session event digest → source/receiver configuration → transport path digest → ordered primitive digests → useful-service profile → exact boundary IDs`

The session event remains the authority for its service request and state. The path contract remains the authority for its ordered primitive description. The sidecar records that the identities, digests and boundaries agree.

## Important physics distinction

Service type is the output service, not the transport carrier. A free-space optical path can provide battery-side DC energy after photovoltaic conversion and rectification. The transport carrier remains electromagnetic; the useful endpoint is electrical DC. Likewise, a wireless energy path can provide shaft work or process heat through declared conversions. Never relabel the path carrier as the final service type.

## Fail-closed semantics

- Matching identifiers and digests may produce METADATA_BOUND only.
- Known contradictions produce NOT_COMPARABLE.
- Missing evidence, stale digests or ambiguous identity produce UNRESOLVED.
- Primitive ports must form a continuous ordered chain.
- Source and receiver identities/configurations must match the exact session event.
- Source-input and useful-output boundary IDs must match the session service contract exactly.
- Unknown evidence is never promoted to measured or independently verified.
- The authority object is closed and constant: evidence-only, no energization authority, no interlock override.

A sidecar never changes DISCOVERED/OFFERED/AUTHENTICATED/RESERVED/POSITION_CONFIRMED/ENERGIZED/RAMPING_DOWN/DEENERGIZED/VERIFIED state. Existing local protection and the #3636 session validator remain authoritative.

## Synthetic example

`transport-session-binding.example.json` demonstrates a laser-to-photovoltaic-to-battery path. All IDs and hashes are synthetic placeholders; the example is not a valid cryptographic or hardware attestation.

## Next integration

The intended implementation location is the existing `wpt-energy-interface-validator` on the #3636 branch. It should consume the path contract from #5906 rather than define a second carrier/mechanism/medium type system. The frozen negative controls are event digest mismatch, source or receiver mismatch, boundary mismatch, stale path digest, operating-regime mismatch, disconnected primitive chain, omitted conversion, unknown evidence and attempted authority escalation.

## Claim ceiling

The synthetic optical-to-battery path explicitly contains an electrical feed and electro-optic conversion, a free-space optical propagation primitive, and a photovoltaic/rectifier-side conversion followed by receiver-side electrical conduction. This is still only a metadata example; the declared conversion stages are not measurements or physical evidence.

The sidecar schema records a metadata binding only. It does not prove that a physical path exists, that an optical beam is present, that a coil is energized, that a service has been delivered, that a receiver is safe or authorized, or that the system meets any standard. It is not permission to energize.
