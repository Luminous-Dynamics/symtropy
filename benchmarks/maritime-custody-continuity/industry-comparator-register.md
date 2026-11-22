# Industry and Standards Comparator Register — Maritime Custody Continuity

## Standards to reuse, not replace

- **DCSA Track & Trace** provides the container-shipping industry's shared event vocabulary and API model across shipment, transport, equipment, IoT and reefer domains. Current public documentation is hosted in the DCSA Standards Public Reference: https://dcsa.org/standards/track-and-trace
- **Version discipline matters.** DCSA announced Track & Trace 3.0.0 on 30 September 2026, extending visibility to IoT and reefer events. DCSA says the 3.0.0 release is initially available to members and DCSA+ partners, with public release planned for H2 2027. Until that public release, do not claim general public conformance to 3.0.0; pin the latest publicly accessible profile actually used by each adapter.
- **GS1 EPCIS 2.0.1 / CBV 2.0.0** provide a mature event-exchange and vocabulary baseline for cross-enterprise visibility. Preserve the distinction between event time and record time; a valid EPCIS event is not automatically authenticated physical truth or proof of legal custody. References: https://ref.gs1.org/standards/epcis/2.0.1/ and https://ref.gs1.org/standards/cbv/2.0.0/
- **IAPH–IHMA PCO and DCSA Port Call/JIT** provide port-call operational milestone semantics. They are not by themselves an independent source-reconciliation or container-custody theorem.
- **Existing TOS platforms are capable systems.** Kaleris N4 describes coordinated vessel, yard, rail, gate and control-room operations. Treat terminal-native equipment interchange, gate and custody-adjacent records as important comparators rather than assuming these capabilities are missing. https://kaleris.com/terminal-operating-system/kaleris-n4-tos/
- **Global shipment visibility is also an established category.** project44 describes carrier/forwarder integrations, container-level tracking, ETA prediction and stitching shipments across modes. Its site advertises 150+ carrier/forwarder connections and 350K+ containers monitored daily; treat those as vendor-published claims, not independently audited benchmark results. https://www.project44.com/platform/visibility/ocean/
- **Port performance has a recognized global comparator.** The World Bank / S&P Global CPPI 2025 is the sixth edition and compares vessel time in port using vessel movement data. The published release describes 403 ports, more than 175,000 vessel calls and 247 million container moves. It is a macro port-efficiency diagnostic, not a custody-verification benchmark or a complete cargo-to-inland service measure. https://www.worldbank.org/en/topic/transport/publication/cppi

- **Mycelix already owns loss-aware logistics interoperability and institutional authority boundaries.** Compose with:
  - [#437 — loss-aware EPCIS, DCSA, ONE Record and UN/CEFACT profiles](https://github.com/Luminous-Dynamics/mycelix/issues/437);
  - [#1073 — electronic bill-of-lading interoperability](https://github.com/Luminous-Dynamics/mycelix/issues/1073);
  - [#1109 — customs and Single Window regulatory-evidence bridge](https://github.com/Luminous-Dynamics/mycelix/issues/1109);
  - [#1110 — supply-chain traceability and transparency bridge](https://github.com/Luminous-Dynamics/mycelix/issues/1110);
  - [#1121 — staged Digital Port Gateway admission](https://github.com/Luminous-Dynamics/mycelix/issues/1121);
  - [#1293 — cross-sovereign partial-effect and finality contract](https://github.com/Luminous-Dynamics/mycelix/issues/1293).

## Semantic separation that must survive every adapter

`event conformance != source authentication != independent physical observation != custody transfer != legal title/control != customs clearance != carrier/terminal release != receiver delivery`.

In particular:
- An EPCIS/DCSA record may describe an event without proving the reporting actor had authority to transfer custody.
- A receiver's digital acknowledgement is not automatically proof the correct receiver identity, mandate, or artifact signature was validated.
- eBL data conformance, transferable-record control, legal title and cargo-release authority are separate questions (#1073).
- Customs submission, acceptance, clearance, release authority and physical release are separate (#1109).
- Cross-standard translation may be lossy and must preserve unmapped fields and nonclaims (#437/#1110).
- The Digital Port Gateway should return staged, purpose-bound admission evidence and no effect authority in the reference profile (#1121).
- A multi-party release/settlement workflow can partially commit; timeout or compensation cannot erase an external effect (#1293).

## Timestamp semantics and temporal-order baselines

- DCSA Information Model 2.0 separates Event DateTime (when an event occurred or will occur) from Event Created DateTime (when the event entry was created), and carries an event classifier such as Actual. The adapter must preserve both rather than use message creation as physical event time. https://reference.dcsa.org/files/content/standards/guidelines/information-model-2-0/dcsa-information-model-2-0.pdf
- GS1 EPCIS separates eventTime from recordTime. eventTime is the capture application's assertion of when the event occurred; recordTime is repository bookkeeping and does not describe the real-world event. EPCIS also carries timezone-offset semantics, while its sensor guidance recommends clock synchronization (IEEE 1588/PTP) for distributed observations. This does not itself establish the quality of a particular device's synchronization. https://ref.gs1.org/epcis/ObjectEvent and https://ref.gs1.org/epcis/SensorReport
- Distributed-systems theory distinguishes a partial happens-before relation from a physical-time total ordering. Lamport's classic treatment is a conceptual baseline: some events are not orderable from available causal information alone. The maritime implementation adds bounded physical-time enclosures and explicit clock-basis identifiers; it should not call overlap simultaneity. https://lamport.azurewebsites.net/pubs/time-clocks.pdf
- Reuse the existing Symtropy interval/event-order research: [#3410 — partial event order under uncertain near-simultaneous impacts](https://github.com/Luminous-Dynamics/symtropy/issues/3410) and [#4113 / closed draft #4107 — exact-rational interval and event-order reference](https://github.com/Luminous-Dynamics/symtropy/issues/4113). This maritime verifier is not a replacement for their exact-rational arithmetic or hybrid contact/reset semantics.

Keep these times distinct in every real adapter: physical event/observation time, source event-entry/creation time, repository record time, source publication time, local ingestion/receipt time, evidence retrieval time, and decision/as-of time. Do not silently substitute one for another.

## What could differentiate this work

The research contribution is the explicit composition and testing of:
1. canonical physical-box identity (upstream Symtropy #6631);
2. source-native event versioning and as-of visibility;
3. paired physical custody handoffs and single-custodian continuity (this benchmark);
4. evidence-artifact authenticity, actor identity, role and exact action authority (owned by Mycelix; not implemented here);
5. transferable-document control and local regulatory recognition (owned by Mycelix; not implemented here);
6. receiver-receipted delivery;
7. aggregate cargo service and intervention impact (downstream Symtropy #6578/#3683).

## Required competitive comparison

At P1/P2, evaluate the same permissioned, adjudicated trace sets against:
- source-native terminal TOS / equipment interchange history, where the owner provides a lawful export;
- DCSA Track & Trace reconstruction using the public version/profile actually available to the evaluator;
- GS1 EPCIS 2.0.1 / CBV 2.0.0 reconstruction;
- a plain relational join/reconciliation baseline;
- a deterministic reference custody state machine;
- the Symtropy composition of #6631 identity resolution plus this custody verifier.

Use identical source data, event cutoffs, identity crosswalks, receiver declarations and adjudication. Report precision, recall, false-complete rate, unresolved fraction, correction/retraction handling, delayed-event sensitivity, field-level mismatch reasons, explainability, runtime and memory. Include separate issuer/authority authentication results where a lawful credential source exists. Do not claim superiority until matched reproduction is complete.

## Claim ceiling

This v0.1 does not demonstrate vendor superiority, standards certification, legal chain of custody, customs compliance, document title, or real-world cargo delivery. It is a finite semantic verifier and a protocol for fair future comparison.
