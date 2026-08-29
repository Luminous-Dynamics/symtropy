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
pub mod art_motion_attribution;
pub mod art_object_id;
pub mod art_object_render_plan;
pub mod art_object_temporal;
pub mod art_object_window;
pub mod art_observation;
#[cfg(feature = "realtime-art-render")]
pub mod art_offscreen;
pub mod art_port;
pub mod art_preview_scene;
#[cfg(feature = "reality-ledger-adapter")]
pub mod art_reality_adapter;
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
pub use art_motion_attribution::{
    MotionAttributionConfig, MotionAttributionError, ObjectMotionAttribution,
    ObjectMotionAttributionEvidence, attribute_transition_motion,
};
pub use art_object_id::{
    ObjectBoundingBox, ObjectIdError, ObjectIdObservation, ObjectIdPlaneEvidence,
    ObjectIdRegistry, ObjectRasterEvidence, analyze_object_id_plane,
};
pub use art_object_render_plan::{
    ObjectIdRenderAssignment, ObjectIdRenderPlan, ObjectIdRenderPlanError,
};
pub use art_object_temporal::{
    ObjectCameraMotionEvidence, ObjectIdentityEvent, ObjectIdentityTransition,
    ObjectTemporalError, PersistentObjectFrame, PersistentObjectTransition,
    ScreenTrajectoryEvidence, SemanticObjectFrame, SemanticObjectState,
    SemanticTransformDelta,
};
pub use art_object_window::{
    ObjectTrackSummary, ObjectWindowError, PersistentObjectWindow,
    PersistentObjectWindowEvidence,
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
#[cfg(feature = "reality-ledger-adapter")]
pub use art_reality_adapter::{SymtropyRealityAdapterError, SymtropyRealityBinding};
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
    /// Create a new cognitive brain with default settings.
    pub fn new() -> Self {
        let service = CognitiveLoopBuilder::new()
            .max_cycles(1)
            .build()
            .expect("CognitiveLoopService construction should succeed");

        Self {
            handle: BrainHandle(Arc::new(RwLock::new(service))),
            last_result: None,
            profile: SovereignProfile8D::default(),
            motor_output: Vec::new(),
            perception_input: String::new(),
            perception_hv: None,
            ticks_since_cycle: 0,
            cycle_interval: 1,
        }
    }

    /// Create a cognitive brain with a custom cycle interval.
    pub fn with_cycle_interval(interval: u32) -> Self {
        let mut brain = Self::new();
        brain.cycle_interval = interval.max(1);
        brain
    }
}

impl Default for CognitiveBrain {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin that installs the cognitive brain update system.
pub struct SymthaeaBrainPlugin;

impl Plugin for SymthaeaBrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, cognitive_brain_system);
    }
}

/// System that runs each entity's cognitive cycle.
fn cognitive_brain_system(mut query: Query<&mut CognitiveBrain>) {
    for mut brain in &mut query {
        brain.ticks_since_cycle += 1;
        if brain.ticks_since_cycle < brain.cycle_interval {
            continue;
        }
        brain.ticks_since_cycle = 0;

        let mut service = brain.handle.0.write().expect("brain lock poisoned");
        let result = if let Some(ref hv) = brain.perception_hv {
            service.cycle_hv(hv.clone())
        } else {
            service.cycle(&brain.perception_input)
        };

        match result {
            Ok(cycle_result) => {
                brain.profile = SovereignProfile8D {
                    epistemic_integrity: cycle_result.metadata.epistemic_integrity,
                    thermodynamic_yield: cycle_result.metadata.thermodynamic_yield,
                    network_resilience: cycle_result.metadata.network_resilience,
                    economic_velocity: cycle_result.metadata.economic_velocity,
                    civic_participation: cycle_result.metadata.civic_participation,
                    stewardship_care: cycle_result.metadata.stewardship_care,
                    semantic_resonance: cycle_result.metadata.semantic_resonance,
                    domain_competence: cycle_result.metadata.domain_competence,
                };
                brain.motor_output = cycle_result.motor_output.clone();
                brain.last_result = Some(cycle_result);
            }
            Err(error) => {
                warn!("Symthaea cognitive cycle failed: {error}");
            }
        }
    }
}
