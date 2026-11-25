# RRVR Safety-Case Interoperability v0.1

**Status:** research crosswalk; not a compliance assessment, safety function, PL/SIL determination, or certification.

**Tracking:** [#7465 — map RRVR evidence to machinery safety and industrial security assurance](https://github.com/Luminous-Dynamics/symtropy/issues/7465)

**Register checked:** 2026-11-25. Standards lifecycle status can change; revalidate the official catalogue before deployment or a formal conformity assessment.

## 1. Position

RRVR should be an evidence-producing subsystem that supports, but does not replace, a manufacturer's or integrator's risk assessment, safety lifecycle, safety-related control system, validation, or certification work.

A passing RRVR model or checker does not establish that a safety function meets a Performance Level (PL), Safety Integrity Level (SIL), safe-stop requirement, stopping-distance requirement, or application-specific safeguarding requirement.

The crosswalk must keep these claims separate:

- **Functional safety:** does the complete safety-related control system perform its specified safety function within its required risk reduction?
- **Cybersecurity:** can an adversary compromise the identity, integrity, availability, update path, clock, observer, or control path in a way that invalidates a safety assumption?
- **Execution/recovery assurance:** can the system reconstruct intent, acceptance, possible physical effects, downstream closure, and the authority for the next action?
- **Physical validation:** did the selected hardware/firmware and installation behave as required under the applicable fault, environmental and operating conditions?

RRVR primarily supports the third category and supplies evidence inputs to the others. It does not make the other categories true by itself.

## 2. Standards register and status

| Reference | Status checked 2026-11-25 | Relevance to RRVR | Claim boundary |
|---|---|---|---|
| ISO 12100:2010 | Published/current; last reviewed and confirmed 2022. ISO/DIS 12100.3 is under development. | Hazard identification, risk estimation/evaluation and risk reduction; use to scope which RRVR failure modes matter. | RRVR does not perform or replace the deployment risk assessment. |
| ISO 10218-1:2025 | Published, 2025-02. | Robot manufacturer/incomplete-machine boundary; device identity, operating envelope and endpoint assumptions. | Industrial robot scope only; not a universal service/mobile robot standard. |
| ISO 10218-2:2025 | Published, 2025-02. | Industrial application/cell integration, commissioning, operation, maintenance and decommissioning; useful for integrator evidence. | Part 2 has explicit application limits, including mobile-platform and public-access cases; check scope before applying. |
| ISO 13849-1:2023 | Published, 2023-04. | Safety-related parts of control systems, including software, in high-demand/continuous modes. | RRVR does not calculate or establish PL, category, PFHd, MTTFd or DC. |
| IEC 62061:2021 + AMD1:2024 + AMD2:2026 | Consolidated edition published 2026-03-20. | Machinery functional-safety control-system design, integration and validation. | It does not replace application-level risk analysis or security-specific controls. |
| IEC 61508-1:2026 PRV | IEC catalogue lists pre-release/FDIS material with voting 2026-10-02 to 2026-11-13; final publication state must be rechecked. | Generic E/E/PE functional safety context; edition-change monitoring. | Do not cite the PRV as a final published edition without confirming the IEC lifecycle record. |
| ISO 3691-4:2023 | Published, 2023-06. ISO/DIS 3691-4 is under development and will replace it when published. | Driverless industrial truck systems, including AGVs and AMRs within the standard's defined scope. | Use 2023 as the published edition until the successor is published; inspect operating-environment exclusions. |
| IEC 62443-4-2:2019 | Published. | Component security requirements: identity/authentication, use control, system integrity, restricted data flow, timely response and resource availability. | A component capability requirement is not proof of the deployment's target or achieved security level. |
| IEC TR 63074:2019 | IEC catalogue marks it replaced. | Historical guidance connecting cybersecurity threats to loss of functional-safety capability. | Do not treat as current guidance; identify and review the applicable successor before formal use. |
| ROS 2 Control Lyrical documentation | Current released documentation at time of review. | Exact implementation behavior for hardware lifecycle and command interfaces. | Implementation documentation, not a functional-safety standard or proof that a deployed plugin/firmware is safe. |
| ROS 2 DDS QoS design | Middleware design documentation. | Deadline, Lifespan and Liveliness behavior can supply transport-health evidence. | Middleware events do not prove user consumption, driver cancellation, endpoint rejection, physical NoEffect or safe state. |

Official references:
- ISO 12100:2010: https://www.iso.org/standard/51528.html
- ISO/DIS 12100.3: https://www.iso.org/standard/88578.html
- ISO 10218-1:2025: https://www.iso.org/standard/73933.html
- ISO 10218-2:2025: https://www.iso.org/standard/73934.html
- ISO 13849-1:2023: https://www.iso.org/standard/73481.html
- IEC 62061 consolidated 2026 edition: https://webstore.iec.ch/en/publication/112847
- IEC 61508-1:2026 PRV: https://webstore.iec.ch/en/publication/117296
- ISO 3691-4:2023: https://www.iso.org/standard/83545.html
- ISO/DIS 3691-4: https://www.iso.org/standard/88615.html
- IEC 62443-4-2:2019: https://webstore.iec.ch/en/publication/34421
- IEC TR 63074:2019: https://webstore.iec.ch/en/publication/31572
- ROS 2 Control Lyrical lifecycle: https://control.ros.org/lyrical/doc/ros2_control/hardware_interface/doc/lifecycle_of_a_hardware_component.html

## 3. RRVR evidence-to-claim crosswalk

| RRVR owner | Evidence it can supply | Potential safety-case use | Cannot establish alone |
|---|---|---|---|
| Coordinator journal / #7403 | Durable intent, crash-cut model, suppression/reconciliation history | Configuration- and attempt-scoped lifecycle evidence | Physical effect, actual durability under target power-loss/storage conditions |
| Injector reservation / #7404 | Key/payload reservation model and concurrency invariants | Duplicate-dispatch prevention argument | Atomicity between a remote ledger and physical actuation |
| Endpoint effect fence / #7405 | One-shot token and authority-epoch model | Device-boundary replay/stale-epoch design argument | Firmware implementation, NVM anti-rollback, physical effect cardinality |
| Journaled Gazebo adapter / #7409 | Intent-before-mutator ordering, restart suppression and owner-lookup behavior | Simulator integration and software execution trace | Live remote injector, real ROS/Gazebo, HIL or physical result |
| Downstream graph / #6817 and #6866 | Declared command-holding graph and per-layer quiescence evidence | Completeness and fencing preconditions for a recovery claim | Proof that the supplied inventory omitted no real device/firmware queue |
| Observer assurance / #6877 | Fault-model-relative observer dependencies and evidence scope | Measurement validity and common-cause analysis | Universal sensor independence or physical truth without platform validation |
| Temporal closure / #7433 and #7435 | Fenced-and-drained or bounded-expiry closure evaluation | No-late-command argument within an exact graph/profile | Hardware bound, clock accuracy, receiver expiry or endpoint behavior not measured |
| Native authority / #6874 | Owner permit, freshness and consume-once admission | Authorization lineage for the next attempt | Safety function design or proof that physical hazard is controlled |
| ROS 2 Control assay / #7415 | Exact distro/plugin/firmware lifecycle and stale-command observations | Platform-specific integration evidence | General ROS lifecycle safety or applicability to other firmware/plugins |

Every artifact must retain its exact source revision, profile, logical-effect/attempt scope, boot/controller/authority epochs, test environment, raw evidence digest, verifier version and qualification tier.

## 4. Safety/security threat cases RRVR should explicitly cover

- Replay of an authentic but stale command or receipt.
- Endpoint NVM rollback, replacement or loss of one-shot token/epoch history.
- Unsigned, downgraded or unauthorized endpoint firmware.
- Compromised injector, bridge, driver, ROS node, gateway or update service.
- Revoked/stolen device or observer credentials.
- Forged, backdated or wrong-scope observation evidence.
- Shared clock, power, network or calibration failure that compromises multiple observers.
- Denial of service that suppresses fencing, safety observations or authority reconciliation.
- Re-provisioning after a security incident where the previous endpoint history is no longer trustworthy.

A valid signature proves who signed bytes under a key assumption; it does not by itself prove freshness, completeness, physical truth, calibration or safe execution. A correct safety argument must account for compromised keys and rollback of the state that enforces freshness.

## 5. Evidence tiers

Every assertion must name exactly one tier and never promote a lower tier into a higher one:

1. Specification/schema review.
2. Bounded formal abstraction and expected-mutant counterexamples.
3. Implementation compile, unit tests and static analysis at an exact commit.
4. Process restart, crash-cut and storage-fault injection.
5. Simulator/digital-twin execution on a pinned profile.
6. Hardware-in-the-loop on exact hardware/firmware.
7. Physical evaluation with independent metrology.
8. Independent safety/security assessment and any required conformity/certification process.

A TLA+ PASS validates the checked abstraction. It is not an ISO compliance result. A simulator PASS is not firmware or physical qualification. A signed receipt is not physical NoEffect.

## 6. Deployment profiles

Maintain separate applicability profiles:

- **Fixed industrial manipulator/cell:** evaluate ISO 10218-1/-2 and applicable ISO 13849-1 or IEC 62061 safety-function design.
- **Driverless industrial truck / AMR:** evaluate ISO 3691-4:2023 and its operating-zone/system scope; track ISO/DIS 3691-4 revision.
- **Experimental ROS 2 Control robot:** freeze the exact ROS distribution, RMW/DDS, hardware plugin, driver, firmware, bus, endpoint, sensors, calibration and safety-related control architecture. ROS documentation is not a substitute for a machinery risk assessment or safety standard.

Do not collapse these into one universal compliance profile. Other deployments (public-space, service, medical, airborne, underwater, hazardous-area or military systems) require their own standards/applicability analysis.

## 7. Next implementation

The tracking issue #7465 should produce:
- strict JSON Schema and versioned machine-readable standards register;
- standard lifecycle/status review date and update triggers;
- requirement-to-RRVR-owner mapping with supported/unsupported claims;
- platform-specific security/fault scenario matrix;
- evidence-tier and change-impact validators;
- an independent reviewer checklist for safety-case reuse.

The first implementation should be descriptive and fail closed on unknown standard status, unknown deployment scope, missing source revision or missing claim ceiling. It must not generate compliance declarations.
