// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! # symthaea-bevy-brain
//!
//! Drop-in Bevy plugin that gives any entity a cognitive architecture.
//! Upgraded to canonical 8D Sovereign Profile and Passport-based prior injection.

use bevy::prelude::*;
use std::sync::{Arc, RwLock};
pub use symthaea::cognitive_loop::{
    CognitiveLoopBuilder, CognitiveLoopService, CycleMetadata, CycleResult,
};
pub use symthaea::symthaea_core::hdc::unified_hv::ContinuousHV;

pub mod art_capture;
pub mod art_cinema;
pub mod art_counterfactual;
pub mod art_depth;
pub mod art_depth_ghost;
#[cfg(feature = "realtime-art-render")]
pub mod art_depth_readback;
pub mod art_eye;
pub mod art_eye_ghost;
pub mod art_ghost_loop;
pub mod art_ghost_session;
pub mod art_observation;
#[cfg(feature = "realtime-art-render")]
pub mod art_offscreen;
pub mod art_port;
pub mod art_preview_scene;
pub mod art_runtime;
pub mod art_scene;
pub mod art_temporal;
pub mod art_temporal_window;
pub mod art_timeline;
pub mod art_visual;

pub use art_capture::{
    ArtCaptureEnqueueReceipt, ArtCaptureError, ArtCaptureOverflowPolicy, ArtCapturePurpose,
    ArtCaptureQueue, ArtCaptureReceipt, ArtCaptureRequest, ArtRenderChannel,
};
pub use art_cinema::{
    ArtCameraKeyframe, ArtCameraPose, ArtSequencePlan, ArtShotPlan, CinematicEvidence,
    CinematicHistory, CinematicPlanError, ExecutedShotRecord, ShotCandidate, ShotSelectionRecord,
};
pub use art_counterfactual::{
    CounterfactualError, CounterfactualRegistry, PreviewBranch, PreviewBranchState,
};
pub use art_depth::{
    ArtistDepthConfig, ArtistDepthConsequenceEvidence, ArtistDepthError, ArtistDepthEvidence,
    ArtistDepthObservation, DepthCentroidEvidence, DepthDiscontinuityEvidence,
    DepthDistributionEvidence, DepthLayerEvidence, DepthPlaneEncoding, analyze_depth_plane,
};
pub use art_depth_ghost::{
    FourGhostArtistDepthError, FourGhostArtistDepthEvidenceSet, GhostArtistDepthEvidence,
};
#[cfg(feature = "realtime-art-render")]
pub use art_depth_readback::{
    ArtDepthCopyTarget, ArtDepthGpuReadback, ArtDepthGpuReadbackQueue, ArtDepthReadbackError,
    ArtDepthReadbackFailure, ArtDepthReadbackOutcome, ArtDepthReadbackPlugin,
    BevyDepthProjection, PreparedArtDepthCapture, RenderedArtDepthCapture,
};
pub use art_eye::{
    ArtistEyeConfig, ArtistEyeConsequenceEvidence, ArtistEyeError, ArtistEyeLevelDelta,
    ArtistEyeObservation, ArtistEyePyramidLevel, ArtistEyeSpatialEvidence,
    EdgeOrientationEvidence, FocalHierarchyEvidence, FocalRegionEvidence, SilhouetteEvidence,
    SymmetryEvidence, ValueMassEvidence, analyze_artist_eye_pixel_plane,
};
pub use art_eye_ghost::{
    FourGhostArtistEyeError, FourGhostArtistEyeEvidenceSet, GhostArtistEyeEvidence,
};
pub use art_ghost_loop::{
    FourGhostCycleReceipt, FourGhostError, FourGhostRenderSet, FourGhostVisualEvidenceSet,
    GhostCandidateKind, GhostDecisionKind, GhostDecisionReceipt, GhostRenderObservation,
    GhostVisualEvidence,
};
pub use art_ghost_session::{
    ExpectedGhostCapture, FourGhostCandidatePlan, FourGhostPlan, FourGhostSession,
    FourGhostSessionError, FourGhostSessionPhase, GhostEvidenceFailure,
};
pub use art_observation::{
    AlignedCounterfactualObservationSet, FidelityTaggedCapture, ObservationError,
    RenderFidelity, RenderFidelityClass, SynchronizedViewSet, TemporalCaptureWindow,
};
#[cfg(feature = "realtime-art-render")]
pub use art_offscreen::{
    ArtGpuReadback, ArtGpuReadbackEnqueueReceipt, ArtGpuReadbackQueue, ArtOffscreenError,
    ArtRenderStamp, PreparedArtCaptureTarget, RenderedArtCaptureTarget,
};
pub use art_port::{
    ART_WORLD_SCHEMA_V1, ArtActionProposal, ArtAuthorityMode, ArtOperation, ArtParameterValue,
    ArtPerceptionFrame, ArtPort, ArtPortError, ArtPortEvent, ArtPortEventKind, ArtProposalState,
};
pub use art_preview_scene::{IsolatedPreviewScene, PreviewSceneError};
pub use art_runtime::{RealtimeArtStudioPlugin, StudioPluginError};
pub use art_scene::{
    ArtEntityId, ArtEntitySemantics, ArtSceneError, ArtSceneRecord,
    perception_frame_from_records, stable_scene_hash,
};
pub use art_temporal::{
    ArtistCameraPoseSample, ArtistTemporalConfig, ArtistTemporalError, ArtistTemporalFrame,
    ArtistTemporalTransition, CameraMotionEvidence, FocalMigrationEvidence,
    VisibilityChangeEvidence,
};
pub use art_temporal_window::{
    ArtistTemporalRhythmEvidence, ArtistTemporalWindow, ArtistTemporalWindowError,
};
pub use art_timeline::{
    FramePacingLedger, FramePacingSample, StudioClock, StudioFrame, StudioFrameRate,
    StudioTimelineError,
};
pub use art_visual::{
    ImagePlaneFeatures, PixelLayout, VisualConsequenceVector, VisualObservation,
    VisualPerceptionError, analyze_pixel_plane,
};

/// Thread-safe wrapper for the cognitive loop.
#[derive(Clone)]
pub struct BrainHandle(pub Arc<RwLock<CognitiveLoopService>>);

/// The canonical 8D Sovereign Profile (Mycelix standard).
#[derive(Debug, Clone, Default, Reflect)]
pub struct SovereignProfile8D {
    pub epistemic_integrity: f64,
    pub thermodynamic_yield: f64,
    pub network_resilience: f64,
    pub economic_velocity: f64,
    pub civic_participation: f64,
    pub stewardship_care: f64,
    pub semantic_resonance: f64,
    pub domain_competence: f64,
}

/// Bevy component: attach to any entity to give it a cognitive loop.
#[derive(Component)]
pub struct CognitiveBrain {
    pub handle: BrainHandle,
    /// Result from the last cognitive cycle.
    pub last_result: Option<CycleResult>,
    /// The 8D profile extracted from the last cycle metadata.
    pub profile: SovereignProfile8D,
    /// Raw motor commands for joint actuation.
    pub motor_output: Vec<f32>,
    /// Perception input for the next cycle (Legacy String path).
    pub perception_input: String,
    /// High-dimensional perception input (Canonical HDC path).
    /// If Some, this overrides `perception_input`.
    pub perception_hv: Option<ContinuousHV>,
    ticks_since_cycle: u32,
    pub cycle_interval: u32,
}

impl CognitiveBrain {
    pub fn new(cfc_neurons: usize, genesis_phrase: &str) -> Self {
        let service = CognitiveLoopBuilder::new()
            .with_cfc_neurons(cfc_neurons)
            .with_genesis_phrase(genesis_phrase)
            .build()
            .expect("failed to build cognitive loop");

        Self {
            handle: BrainHandle(Arc::new(RwLock::new(service))),
            last_result: None,
            profile: SovereignProfile8D::default(),
            motor_output: Vec::new(),
            perception_input: String::new(),
            perception_hv: None,
            ticks_since_cycle: 0,
            cycle_interval: 3,
        }
    }

    pub fn new_with_hv_input(cfc_neurons: usize, genesis_phrase: &str, hv_dim: usize) -> Self {
        let mut brain = Self::new(cfc_neurons, genesis_phrase);
        brain.perception_hv = Some(ContinuousHV::zero(hv_dim));
        brain
    }

    /// Current scalar Phi estimate (epistemic integrity proxy).
    pub fn phi(&self) -> f64 {
        self.profile.epistemic_integrity
    }

    /// Inject explicit FEP priors (Passport Route).
    pub fn inject_priors(&self, mean: Vec<f64>, precision: Vec<f64>) {
        if let Ok(mut service) = self.handle.0.write() {
            service.inject_priors(mean, precision);
        }
    }

    fn cycle(&mut self) {
        if let Ok(mut service) = self.handle.0.write() {
            let result = if let Some(ref hv) = self.perception_hv {
                service.cycle_with_hv(hv)
            } else {
                service.cycle(&self.perception_input)
            };

            self.motor_output = result.output.clone();

            let mce = &result.metadata;
            self.profile = SovereignProfile8D {
                epistemic_integrity: service.consciousness_level() as f64,
                thermodynamic_yield: mce.mce_softmin,
                network_resilience: mce.mce_weighted_sum,
                economic_velocity: mce.perception_attention_sensitivity as f64,
                civic_participation: mce.mce_social,
                stewardship_care: mce.embodied.body_phi_modulation,
                semantic_resonance: mce.mce_narrative,
                domain_competence: mce.memory_recall_quality as f64,
            };

            self.last_result = Some(result);
        }
    }
}

pub struct SymthaeaBrainPlugin {
    pub default_neurons: usize,
    pub telemetry: bool,
}

impl Default for SymthaeaBrainPlugin {
    fn default() -> Self {
        Self {
            default_neurons: 64,
            telemetry: false,
        }
    }
}

impl Plugin for SymthaeaBrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, cognitive_cycle_system);
        if self.telemetry {
            app.add_systems(Update, telemetry_system);
        }
    }
}

fn cognitive_cycle_system(mut brains: Query<&mut CognitiveBrain>) {
    for mut brain in &mut brains {
        brain.ticks_since_cycle += 1;
        if brain.ticks_since_cycle >= brain.cycle_interval {
            brain.ticks_since_cycle = 0;
            brain.cycle();
        }
    }
}

fn telemetry_system(brains: Query<(Entity, &CognitiveBrain)>) {
    for (entity, brain) in &brains {
        debug!(
            "Brain {:?}: Phi={:.4}, Competence={:.4}, Resonance={:.4}",
            entity,
            brain.phi(),
            brain.profile.domain_competence,
            brain.profile.semantic_resonance,
        );
    }
}
