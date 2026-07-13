# Symtropy Presentation Architecture & 3D Pivot (v0.1)

## Executive Summary
This document formalizes the pivot of the Symtropy presentation model. The current 2D top-down implementation is officially frozen as a **Debug Harness / Systems Sandbox / CI Validation Layer**. It is not the target player experience.

Symtropy is redefined as a high-fidelity **Embodied 3D civilization survival/repair game** integrated with a **2.5D/isometric Civic Planning & Governance layer**. Both presentation layers (along with a headless validation layer) sit atop a unified, deterministic shared simulation core.

---

## 1. The Three-Tier Presentation Architecture

```
                    +----------------------------------+
                    |      User Interface Options      |
                    +----------------------------------+
                                     |
         +---------------------------+---------------------------+
         |                           |                           |
         v                           v                           v
+------------------+       +-------------------+       +------------------+
| Embodied 3D      |       | City Ops 2.5D     |       | Headless Sim     |
| (1st/3rd Person) |       | (Strategic/Civic) |       | (CI/Validation)  |
+------------------+       +-------------------+       +------------------+
         |                           |                           |
         +---------------------------+---------------------------+
                                     |
                                     v
                    +----------------------------------+
                    |      Shared Simulation Core      |
                    | (ECS State, Layout, Metrics, HUD)|
                    +----------------------------------+
```

### 1.1 Embodied 3D Layer (First/Third-Person)
- **Role**: Physical interaction, mechanical repair, immediate hazard survival, real-time combat, vehicle operation, exploration, and spatial scanning.
- **Focus**: High-fidelity immersion, local real-time physics, spatial sound design, and micro-scale tasks.
- **Output**: Generates real-time state changes, device transactions, and emits signed outcomes upon completing significant protocols.

### 1.2 City Ops 2.5D Layer (Strategic/Planning)
- **Role**: High-level logistics planning, power/water grid overlays, road planning, district zoning, public works queues, emergency authorization monitoring, and vote cast interfaces.
- **Focus**: Clear visual telemetry, long-term civic strategy, faction pressure maps, and history visualization.
- **Output**: Issues zone planning orders, grid routing mandates, and initiates public votes.

### 1.3 Headless Simulation Layer (Testing/Server)
- **Role**: Continuous integration, server-side simulation authority, deterministic validation of settlement metrics, and automated Mycelix/Symtheae behavior experiments.
- **Focus**: Headless correctness, maximum validation speed, and regression prevention.
- **Output**: Test suite assertions, performance metrics, and transaction validation receipts.

---

## 2. The Shared Simulation Core

All three layers must read from and write to the same simulation structures. Presentation layers project these structures into their respective rendering modes (e.g., 3D meshes, 2D icons, or JSON validation streams).

### 2.1 Shared Layout & Anchor Resource
```rust
pub struct SiteLayout {
    pub site_id: String,
    pub anchors: HashMap<String, Transform3d>,
    pub rooms: Vec<RoomVolume>,
    pub infrastructure_nodes: Vec<InfrastructureNodeId>,
}
```
- **3D Render**: Instantiates meshes and colliders at the calculated `Transform3d` coordinate anchors.
- **2.5D Render**: Renders nodes as interactive schematic symbols on a projection grid.
- **Headless**: Simulates distances and paths using numerical coordinates, with no visual representation.

### 2.2 Shared Simulation State
- **SettlementMetrics**: Unified metrics (Power, Water, Trust, Entropy, Safety, Legitimacy).
- **Chronicle Events**: Structured record of milestones.
- **NPC Cog State**: Cognition parameters (caution, learning rate, epistemic tags).
- **Resource Flow Network**: Direction and volume of water/energy grids.

---

## 3. Mycelix (Holochain) & Symtheae (AI) Boundaries

### 3.1 Mycelix Separation (Civic vs. Hot-Path)
Real-time gameplay runs locally to maintain high-fidelity 3D responsiveness. Real-time events do not interact with Mycelix.

- **Local Simulation Hot-Path (Real-Time)**: Footsteps, bullet physics, player movement, scanner frequency adjustments, tool heat, and raw machinery repairs.
- **Device Bus Transactions (Deterministic)**: Access control checks, door locking mechanisms, and material reserve consumption.
- **Chronicle (Civic Records)**: Signed outcomes, waterworks restoration protocols, public decree outcomes, and credential generation.
- **Mycelix Bridge Runtime (Persistence)**: Asynchronously commits local Chronicle scars and voter signatures to the Holochain source chain.

### 3.2 Symtheae Cognitive Embodiment
- **Crew NPCs** run enactive active inference loops (`ActiveInferenceAgent`) mapped to real world inputs.
- NPCs read spatial fields (e.g., consciousness gradients, hazard volumes, machine outages) and execute pathfinding targets.
- Their internal psychological stress states directly scale their FEP caution variables, modulating movement speed and repair accuracy.

---

## 4. Scaffold Role of the 2D Launcher
The current 2D codebase (`symtropy-launcher`) is frozen as the **Debug Harness / Systems Sandbox**.

- **Allowed Tasks**:
  - Running automated check suites and simulation cycles.
  - Validating settlement metric algorithms.
  - Testing Chronicle event generation.
  - Benchmarking NPC FEP parameters.
- **Prohibited Tasks**:
  - Visual decoration, sprite polishing, or pixel-art tuning.
  - Adding game-feel tweaks, camera animations, or real-time control adjustments.
  - Incorporating assets not intended for shared core validation.

---

## 5. Next Milestones (Horizon 1.5)
1. **Simulation Extraction**: Decouple layout and metric calculations from the 2D rendering pipeline into shared core structures.
2. **Setup Seedworks3D**: Define the initial 3D entry point in the launcher or a dedicated `crates/apps/symtropy-3d` package.
3. **Old Waterworks 3D Blockout**: Assemble a basic 3D room volume with a player controller, static mesh pump, and interactive terminal.
4. **Chronicle Mycelix Decoupling**: Implement a backend abstraction layer that keeps Holochain bridge updates asynchronously detached from the real-time simulation thread.
