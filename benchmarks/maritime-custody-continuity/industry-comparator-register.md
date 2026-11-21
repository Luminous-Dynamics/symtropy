# Industry and Standards Comparator Register — Maritime Custody Continuity

## Standards to reuse, not replace

- DCSA Track & Trace provides standardized event exchange for container-shipping visibility. The benchmark must test exact event semantics and implementation compatibility rather than inventing a competing global event vocabulary. Public standard documentation is maintained in the DCSA Standards Public Reference: https://dcsa.org/standards/track-and-trace
- GS1 EPCIS 2.0 provides interoperable visibility-event structures for what, when, where, why and how. A future adapter should map the benchmark's custody assertions to EPCIS/CBV only when the source semantics genuinely align; an EPCIS event is not automatically proof of legal custody or authenticated receipt.
- DCSA Port Call / Just-in-Time and IAPH–IHMA PCO describe port-call operational milestones, not a complete source-independent custody-chain proof.
- Existing terminal operating systems and visibility platforms may already manage container status, gate transactions, interchange and custody-related records. This benchmark is an evidence reconciliation layer alongside them, not a claim that incumbents lack those features.

## What could differentiate this work

The research contribution is the explicit separation and testing of:
1. canonical physical-box identity (upstream #6631);
2. custody state and paired physical handoff;
3. event publication/revision/replay semantics;
4. evidence artifact authenticity and actor authority (not yet implemented);
5. final receiver receipt;
6. aggregate cargo service and intervention impact (downstream #6578/#3683).

## Required competitive comparison

At P1/P2, evaluate the same permissioned trace sets against:
- source-native TOS / interchange history, where the owner provides a lawful export;
- DCSA Track & Trace event reconstruction;
- EPCIS 2.0 event reconstruction;
- a plain relational reconciliation baseline;
- the Symtropy custody state machine.

Use identical source data, event cutoffs, identity crosswalks and adjudication. Report precision, recall, false-complete rate, unresolved fraction, delay sensitivity, correction/retraction handling, explainability, runtime and memory. Do not claim superiority until matched reproduction is complete.
