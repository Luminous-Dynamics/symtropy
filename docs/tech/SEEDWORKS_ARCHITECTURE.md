# Symtropy: Seedworks Architectural Blueprint

## Overview
This document outlines the architectural integration points for the **Seedworks** vertical slice. This slice is a self-contained proof-of-concept located in `crates/symtropy-bevy-core/examples/old_waterworks_micro_slice.rs`.

## 1. Subsystem Mapping

| Slice Component | Engine Subsystem | Responsibility |
| :--- | :--- | :--- |
| `SiteHistory` | `symtropy-world` | Persistence of site state/metadata. |
| `PlayerOrigin` | `symtropy-bevy::biometrics` | Persona-specific scan/UI modifiers. |
| `RepairPath` | `symtropy-robotics-bridge` | Motor authority and system intervention. |
| `Chronicle` | `symtropy-chronicle` (new) | Append-only history ledger. |

## 2. API Integration Hooks

To promote the micro-slice to a core engine experience, the following integration APIs must be exposed:

### A. The Chronicle Logger (Service Interface)
```rust
pub trait ChronicleLogger {
    fn record<T: Serialize>(event: ChronicleEventEnvelope<T>) -> Result<(), ChronicleError>;
}
```
*   *Implementation:* Needs to integrate with `symtropy-net` for eventual distributed synchronization, but initially can be a local append-only JSONL file.

### B. Origin-Aware Scan Service
The `FieldDeck` should query an `OriginService` to determine the foreground/background of its scan UI.
```rust
pub trait ScanInterface {
    fn get_contextual_note(origin: &PlayerOrigin, site_id: &str) -> String;
}
```

### C. Repair-Authority Bridge
The `DeadAuthorityLock` needs to resolve against the engine's `AuthorityRegistry`.
```rust
pub struct RepairResolution {
    pub path: RepairPath,
    pub outcome: Outcome,
    pub legitimacy_impact: i32,
}
```

## 3. Future Expansion Path
1. **Normalization:** Resolve systemic path inconsistencies (workspace-wide) before attempting crate promotion.
2. **Device Bus:** Replace the current console interaction with a proper `symtropy-DeviceBus` implementation for component-based messaging.
3. **Null Loop:** Integrate the `Null Reinforcement Loop` into the `symtropy-consciousness-physics` update cycle, making it a state-based system effect rather than a stubbed UI warning.

## 4. Repository Guidelines
*   **Containment:** The `Seedworks` slice logic must remain within `symtropy-bevy-core` or a dedicated `symtropy-seedworks` crate to prevent cross-workspace contamination.
*   **Hygiene:** All new modules must satisfy `cargo check` in isolation before integration.
EOF
