# ev-battery-second-life-audit

Dependency-free Rust 1.96 reference audit for mission-specific second-life routing of retired EV packs.

Checks cell-to-pack membership and configuration, evidence registry and rights admission, explicit complete first-life incident categories, physically evidenced `VERIFIED_ABSENT` history, physical cell-health evidence, BMS strategy compatibility, mission binding, lower-bound energy/power/reserve and thermal-margin feasibility, and non-compensable hard gates.

Second-life and third-life backup are distinct mission classes. UNKNOWN hard gates never permit an eligible reuse route. Physical cell, incident and BMS evidence is required where the claim depends on real hardware state; synthetic fixtures may only validate the software rules.

All decisions are read-only. The audit does not estimate SOH/RUL, simulate degradation, operate BMS firmware, authorize disassembly/HV testing/transport/commissioning, certify safety, or submit a Battery Passport. Unit tests are authored but require exact-head workflow execution.