// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Typed GPU storage-buffer records for telemetry visualization.
//!
//! These layouts are consumed by the Bevy 0.20 telemetry WESL material. The current
//! shader reads node records for read-only colour/emission modulation; it does not
//! implement compute-driven vertex deformation. Buffer builders return initialized
//! ShaderBuffer assets, but callers own asset handles, update cadence and render binding.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;
use bevy::render::storage::ShaderBuffer;
use bytemuck::{Pod, Zeroable};

/// Shared telemetry record consumed by the read-only telemetry visualization material.
///
/// Its exact 64-byte field order is pinned by offset tests and mirrored by
/// `NodeTelemetryGpu` in `telemetry_visualization.wesl`. The current material uses
/// position, variational free energy, coherence, circuit load and DHT completeness.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, ShaderType, Reflect, Pod, Zeroable)]
pub struct NodeTelemetryGpu {
    // --- Layer 4: Cognitive & Spatial Coordinates ---
    /// Spatial 3D coordinates (position) derived from t-SNE/HDC projections or bioregional terrain.
    /// In std430, a vec3 occupies 12 bytes but is aligned to 16 bytes.
    pub position: Vec3,
    /// Variational Free Energy surprise metric (contributes to visual activity intensity).
    pub variational_free_energy: f32,

    // --- Layer 1: Crypto-Transport ---
    /// Throughput / bandwidth in bits per second (drives line thickness of transport links).
    pub bandwidth_bps: f32,
    /// Network latency in milliseconds (drives particle velocity along connection lines).
    pub latency_ms: f32,
    /// Cryptographic handshake tunnel state:
    /// - 0: Disconnected
    /// - 1: Sovereign Secure (X25519MLKEM768 hybrid)
    /// - 2: Fallback Secure
    /// - 3: Unvetted / Untrusted / Malicious
    pub tunnel_state: u32,
    /// Data completeness fraction (0.0 to 1.0) of local DHT neighborhood holdings.
    pub dht_holding_completeness: f32,

    // --- Layer 2: Distributed Ledger & Gossip ---
    /// Gossip update frequency (sync events per second in Hz).
    pub gossip_frequency_hz: f32,
    /// Validation failures or cryptographic slashing count (drives ring fracture effect).
    pub validation_failure_count: u32,
    /// Consumed memory fraction of the WebAssembly container sandbox (0.0 to 1.0).
    pub wasm_memory_fraction: f32,
    /// Relative timestamp of the last hot-swap / bytecode reload.
    pub last_hot_reload_time: f32,

    // --- Layer 4 & 5: Consciousness Integration & Physical Twin ---
    /// Information integration coherence metric (Phi/IIT) driving luminance core intensity.
    pub holographic_coherence: f32,
    /// Thermal gradient (Kelvin delta) from bioregional hardware monitoring.
    pub thermal_gradient: f32,
    /// Electrical/computational load line coefficient (0.0 to 1.0).
    pub circuit_load: f32,
    /// Padding field to preserve perfect 16-byte size structure (64 bytes total).
    pub _padding: f32,
}

/// Shared telemetry layout representing a post-quantum cryptographic link between two nodes.
///
/// Maps to WebGPU Storage Buffers for particle simulation systems (similar to bevy_hanabi)
/// that draw packets flowing between federated networks.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, ShaderType, Reflect, Pod, Zeroable)]
pub struct LinkTelemetryGpu {
    /// Index of the source node inside the `nodes` storage buffer.
    pub source_node_idx: u32,
    /// Index of the target node inside the `nodes` storage buffer.
    pub target_node_idx: u32,
    /// Width of the connecting line.
    pub link_thickness: f32,
    /// Wave propagation velocity for packets traveling along this edge.
    pub particle_velocity: f32,

    /// RGBA color representation (mapped directly to WGSL vec4<f32>).
    pub link_color: Vec4,
}

/// CPU-side telemetry records used to build GPU ShaderBuffer assets.
///
/// This is a data carrier, not an automatic render-extraction system. Call
/// `build_node_shader_buffer` / `build_link_shader_buffer` and add the returned
/// assets to Bevy's `Assets<ShaderBuffer>`; callers own updating the asset and
/// assigning its handle to the material. Empty vectors return `None`, because
/// zero-sized storage buffers cannot be bound.
#[derive(Resource, Default, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct TelemetryBufferResource {
    /// Array of active telemetry nodes.
    pub nodes: Vec<NodeTelemetryGpu>,
    /// Array of active transport links.
    pub links: Vec<LinkTelemetryGpu>,
}

impl TelemetryBufferResource {
    /// Creates a typed node storage buffer from the current records.
    ///
    /// Returns `None` for an empty vector because a zero-sized storage buffer
    /// cannot be used as a binding resource.
    pub fn build_node_shader_buffer(&self) -> Option<ShaderBuffer> {
        if self.nodes.is_empty() {
            return None;
        }
        Some(ShaderBuffer::new(
            self.nodes.clone(),
            RenderAssetUsages::default(),
        ))
    }

    /// Creates a typed link storage buffer from the current records.
    ///
    /// Returns `None` for an empty vector because a zero-sized storage buffer
    /// cannot be used as a binding resource.
    pub fn build_link_shader_buffer(&self) -> Option<ShaderBuffer> {
        if self.links.is_empty() {
            return None;
        }
        Some(ShaderBuffer::new(
            self.links.clone(),
            RenderAssetUsages::default(),
        ))
    }

    /// Clears all CPU-side telemetry records.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.links.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{offset_of, size_of};

    #[test]
    fn node_telemetry_layout_matches_the_wesl_storage_struct() {
        assert_eq!(size_of::<NodeTelemetryGpu>(), 64);
        assert_eq!(offset_of!(NodeTelemetryGpu, position), 0);
        assert_eq!(offset_of!(NodeTelemetryGpu, variational_free_energy), 12);
        assert_eq!(offset_of!(NodeTelemetryGpu, bandwidth_bps), 16);
        assert_eq!(offset_of!(NodeTelemetryGpu, latency_ms), 20);
        assert_eq!(offset_of!(NodeTelemetryGpu, tunnel_state), 24);
        assert_eq!(offset_of!(NodeTelemetryGpu, dht_holding_completeness), 28);
        assert_eq!(offset_of!(NodeTelemetryGpu, gossip_frequency_hz), 32);
        assert_eq!(offset_of!(NodeTelemetryGpu, validation_failure_count), 36);
        assert_eq!(offset_of!(NodeTelemetryGpu, wasm_memory_fraction), 40);
        assert_eq!(offset_of!(NodeTelemetryGpu, last_hot_reload_time), 44);
        assert_eq!(offset_of!(NodeTelemetryGpu, holographic_coherence), 48);
        assert_eq!(offset_of!(NodeTelemetryGpu, thermal_gradient), 52);
        assert_eq!(offset_of!(NodeTelemetryGpu, circuit_load), 56);
        assert_eq!(offset_of!(NodeTelemetryGpu, _padding), 60);
    }

    #[test]
    fn link_telemetry_layout_matches_the_wesl_storage_struct() {
        assert_eq!(size_of::<LinkTelemetryGpu>(), 32);
        assert_eq!(offset_of!(LinkTelemetryGpu, source_node_idx), 0);
        assert_eq!(offset_of!(LinkTelemetryGpu, target_node_idx), 4);
        assert_eq!(offset_of!(LinkTelemetryGpu, link_thickness), 8);
        assert_eq!(offset_of!(LinkTelemetryGpu, particle_velocity), 12);
        assert_eq!(offset_of!(LinkTelemetryGpu, link_color), 16);
    }

    #[test]
    fn buffer_builders_return_none_for_empty_records() {
        let telemetry = TelemetryBufferResource::default();
        assert!(telemetry.build_node_shader_buffer().is_none());
        assert!(telemetry.build_link_shader_buffer().is_none());
    }

    #[test]
    fn buffer_builders_bind_exact_record_bytes() {
        let telemetry = TelemetryBufferResource {
            nodes: vec![NodeTelemetryGpu {
                position: Vec3::new(1.0, 2.0, 3.0),
                ..default()
            }],
            links: vec![LinkTelemetryGpu {
                source_node_idx: 0,
                target_node_idx: 1,
                ..default()
            }],
        };

        let nodes = telemetry.build_node_shader_buffer().expect("one node");
        let links = telemetry.build_link_shader_buffer().expect("one link");
        assert_eq!(nodes.buffer_size(), size_of::<NodeTelemetryGpu>() as u64);
        assert_eq!(links.buffer_size(), size_of::<LinkTelemetryGpu>() as u64);
    }

    #[test]
    fn bevy_020_shader_buffer_accepts_typed_telemetry_records() {
        let nodes = vec![
            NodeTelemetryGpu {
                position: Vec3::new(1.0, 2.0, 3.0),
                variational_free_energy: 0.25,
                holographic_coherence: 0.75,
                ..default()
            },
            NodeTelemetryGpu {
                position: Vec3::new(4.0, 5.0, 6.0),
                variational_free_energy: 0.5,
                holographic_coherence: 0.9,
                ..default()
            },
        ];

        let buffer = ShaderBuffer::new(nodes, RenderAssetUsages::default());
        let readback = buffer
            .cast_slice::<NodeTelemetryGpu>()
            .expect("typed initialized buffer");

        assert_eq!(readback.len(), 2);
        assert_eq!(readback[0].position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(readback[1].position, Vec3::new(4.0, 5.0, 6.0));
        assert_eq!(buffer.buffer_size(), (2 * size_of::<NodeTelemetryGpu>()) as u64);
    }
}
