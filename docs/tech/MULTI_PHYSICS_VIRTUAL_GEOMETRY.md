# Multi-Physics Virtual Geometry (MPVG)

## 1. Architectural Concept

Traditional real-time rendering and physical simulation pipelines operate over completely disjoint geometry formats:
- **Visuals:** GPU-optimized mesh representation (e.g., Bevy's native Compute Meshlet pipeline) designed to rasterize billions of triangles using cluster-based DAGs, software rasterizers, and visibility buffers.
- **Physics:** Low-resolution collision proxy shapes (capsules, bounding boxes, or simplified convex hulls) that approximate collision to preserve CPU performance.

**Multi-Physics Virtual Geometry (MPVG)** unifies these domains. By embedding structural, thermodynamic, and acoustic parameters directly inside the vertex buffer strides of rendering meshlets, the system achieves:
1. **Simulation Dignity:** GJK/EPA narrowphase checks execute directly against high-fidelity meshlet clusters instead of low-poly proxies.
2. **Unified Data Substrate:** Telemetry and material attributes are shared zero-copy between CPU physical constraints, WebGPU compute shaders, and dynamic visual deformations.

---

## 2. Data Structure & Serialization (.mpvg)

The MPVG data format is serialized to binary byte streams via `bincode` for zero-copy VRAM streaming and headless asset cooking.

### MultiPhysicsVertex
Stores standard visual components alongside structural stress, thermal, and acoustic strides:
```rust
#[repr(C)]
pub struct MultiPhysicsVertex {
    // --- Visual Strides (32 bytes) ---
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],

    // --- Physical Strides (16 bytes) ---
    pub mass_density: f32,          // kg/m^3 (steel: 7800, aluminum: 2700)
    pub elastic_modulus: f32,       // Young's modulus (Pa) driving compliance
    pub thermal_conductivity: f32,  // W/(m*K)
    pub acoustic_impedance: f32,    // Pa*s/m
}
```

### MultiPhysicsMeshlet
Represents a cluster of triangles (typically up to 64 vertices and 126 indices) with pre-computed cluster-level physical bounds:
```rust
pub struct MultiPhysicsMeshlet {
    pub vertex_offset: u32,
    pub vertex_count: u32,
    pub index_offset: u32,
    pub index_count: u32,

    // --- Aggregated Physics Bounds ---
    pub center_of_mass: SVector<f32, 3>, // Centroid mass-weighted center
    pub total_mass: f32,                // Total mass computed from vertex volumes/densities
    pub average_elasticity: f32,        // Average Young's modulus for TGS Soft compliance
}
```

---

## 3. Two-Level Narrowphase GJK/EPA Collision

To execute narrowphase collision against high-resolution meshes at 500 Hz, `symtropy-mesh` implements a hierarchical pruning solver:

1. **Midphase Cluster Pruning:** A sphere-sphere intersection check is performed between the colliding shape's bounding sphere and each `MultiPhysicsMeshlet`'s pre-computed bounding radius:
   $$\text{Radius} = \max_{v \in \text{vertices}} \|v - \text{center\_of\_mass}\|$$
   Meshlet clusters that do not overlap the incoming collider are instantly pruned.
2. **Narrowphase GJK/EPA Check:** For overlapping clusters, individual triangles are reconstructed on the fly (casting `f32` vertex strides to double-precision `f64` vectors) and solved via GJK/EPA:
   - **GJK (Gilbert-Johnson-Keerthi):** Evaluates convex intersection.
   - **EPA (Expanding Polytope Algorithm):** Determines penetration depth and contact normals.

---

## 4. Material-Aware TGS Soft Constraint Compliance

During contact manifold generation, the `MultiPhysicsMeshlet`'s `average_elasticity` (Young's modulus $E$ in Pa) is bound directly to the resulting `ContactManifold` struct. 

In the **Temporal Gauss-Seidel (TGS) Soft solver** inside `symtropy-physics`, contact compliance is modified dynamically. Physical compliance $C$ represents inverse stiffness ($C = 1/K$). The solver converts Young's modulus $E$ to compliance:
$$C_{\text{material}} = \frac{1}{E}$$

This compliance is added directly to the constraint's diagonal solver matrix $\alpha$:
$$\alpha_{\text{total}} = \frac{\text{compliance}_{\text{global}} + C_{\text{material}}}{dt^2}$$

The constraint impulse update step then calculates:
$$\Delta\lambda = \frac{-v_{\text{rel}} \cdot n + \text{bias}}{M_{\text{inv}} + \alpha_{\text{total}}}$$

### Solver Behavior:
- **Stiff Materials ($E \approx 200\text{ GPa}$):** $C_{\text{material}} \to 0$, producing instantaneous elastic rebounds.
- **Soft Materials ($E \approx 100\text{ kPa}$):** $\alpha_{\text{total}}$ increases, absorbing kinetic energy, dampening peak contact forces, and distributing the impulse resolution smoothly over multiple ticks.

---

## 5. Visual Observability Matrix (GPU SSBO)

To close the visual loop, active inference and physical strain metrics are synced to the GPU using WebGPU Storage Buffers (SSBOs). 

### Node & Link Telemetry Strides (std430 Aligned)
```rust
#[derive(ShaderType)]
pub struct NodeTelemetryGpu {
    pub position: Vec3,
    pub variational_free_energy: f32, // drives vertex displacement shaders
    pub bandwidth_bps: f32,
    pub latency_ms: f32,
    pub tunnel_state: u32,
    pub dht_holding_completeness: f32,
    pub gossip_frequency_hz: f32,
    pub validation_failure_count: u32,
    pub wasm_memory_fraction: f32,
    pub last_hot_reload_time: f32,
    pub holographic_coherence: f32,
    pub thermal_gradient: f32,
    pub circuit_load: f32,
    pub _padding: f32, // Perfect 64-byte alignment
}
```

Bevy's custom WGSL vertex shaders read this buffer directly. If a local node's variational free energy ($\mathcal{E}$) spikes (triggered by cyber events or kinetic impact impulses from the TGS solver), the shader dynamically warps the meshlet vertices along normal vectors, visualising computational stress as geometric topology.

---

## 6. Implementation References

- **Data layouts:** [meshlet_physics.rs](../../crates/symtropy-mesh/src/meshlet_physics.rs)
- **Narrowphase collision:** [narrowphase.rs](../../crates/symtropy-mesh/src/narrowphase.rs)
- **Soft TGS solver:** [world.rs](../../crates/core/symtropy-physics/src/world.rs)
- **GPU buffers:** [telemetry_ssbo.rs](../../crates/bridges/symtropy-render-bridge/src/telemetry_ssbo.rs)
- **Sandboxed asset baking CLI:** [mycelix-asset-bake.rs](../../crates/symtropy-mesh/src/bin/mycelix-asset-bake.rs)
