# ev-battery-second-life-audit

Dependency-free Rust 1.96 reference auditor for second-life EV battery routing.

Checks physical-cell-to-pack closure, exact configuration identity, evidence and rights admission, incident-history completeness, BMS strategy/compatibility, mission binding, and hard-gate eligibility. A second/third-life eligible disposition requires every declared hard gate to pass, complete first-life history, admitted cell evidence, physical evidence for physical safety gates, BMS compatibility, and an explicitly feasible mission envelope.

The auditor is read-only. It does not estimate SOH/RUL, simulate degradation, execute BMS updates, authorize disassembly/HV testing/transport, certify safety, or submit a Battery Passport. Unit tests are authored but require exact-head workflow execution.