// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! SCALE-CELL-001F async/prewarm reference fixture.
//!
//! This tranche treats the prior SCALE-CELL semantic subject as opaque typed
//! evidence and proves only async observation-hydration semantics. It does not
//! implement domain evolution, canonical refinement, scheduler execution, Bevy,
//! networking, filesystem I/O, or production task-runtime authority.
//!
//! The central theorem is:
//!
//! predictive hydration / task completion order / runtime parallelism
//! != canonical world authority.

use std::collections::{BTreeMap, BTreeSet};

use symtropy_sim_contracts::{
    ContractError, DigestAlgorithm, ScopeId, TypedDigest32, WorldInstanceId,
};

const MAX_PENDING_HYDRATIONS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExecutionProfile {
    ScaleReferenceSerialV1,
    ScaleRuntimeParallelV1,
}

impl ExecutionProfile {
    const fn code(self) -> u8 {
        match self {
            Self::ScaleReferenceSerialV1 => 0,
            Self::ScaleRuntimeParallelV1 => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CanonicalCellState {
    world: WorldInstanceId,
    authority_generation: u64,
    exact_total: u64,
    process_counter: u64,
    prior_scale_context: TypedDigest32,
}

impl CanonicalCellState {
    fn validate(&self) -> Result<(), RefError> {
        self.world.validate()?;
        self.prior_scale_context.validate()?;
        Ok(())
    }

    fn commitment(&self) -> Result<TypedDigest32, RefError> {
        self.validate()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"symtropy.scale-cell-001f.canonical-cell.v1\0");
        push_string(&mut bytes, self.world.as_str());
        bytes.extend_from_slice(&self.authority_generation.to_le_bytes());
        bytes.extend_from_slice(&self.exact_total.to_le_bytes());
        bytes.extend_from_slice(&self.process_counter.to_le_bytes());
        push_digest(&mut bytes, &self.prior_scale_context);
        Ok(TypedDigest32::sha256(
            "symtropy.scale-cell-001f.canonical-cell.identity.v1",
            1,
            &bytes,
        )?)
    }

    fn advance_reference_generation(&self) -> Result<Self, RefError> {
        self.validate()?;
        let authority_generation = self
            .authority_generation
            .checked_add(1)
            .ok_or(RefError::CounterOverflow)?;
        let process_counter = self
            .process_counter
            .checked_add(1)
            .ok_or(RefError::CounterOverflow)?;
        let next = Self {
            world: self.world.clone(),
            authority_generation,
            exact_total: self.exact_total,
            process_counter,
            prior_scale_context: self.prior_scale_context.clone(),
        };
        next.validate()?;
        Ok(next)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HydrationRequest {
    request_id: u64,
    world: WorldInstanceId,
    scope: ScopeId,
    source_commitment: TypedDigest32,
    prior_scale_context: TypedDigest32,
    recipe: TypedDigest32,
    execution_profile: ExecutionProfile,
}

impl HydrationRequest {
    fn validate(&self) -> Result<(), RefError> {
        if self.request_id == 0 {
            return Err(RefError::InvalidRequestId);
        }
        self.world.validate()?;
        self.scope.validate()?;
        self.source_commitment.validate()?;
        self.prior_scale_context.validate()?;
        self.recipe.validate()?;
        Ok(())
    }

    fn execution_evidence(&self) -> Result<TypedDigest32, RefError> {
        self.validate()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"symtropy.scale-cell-001f.hydration-request.v1\0");
        bytes.extend_from_slice(&self.request_id.to_le_bytes());
        push_string(&mut bytes, self.world.as_str());
        push_string(&mut bytes, self.scope.as_str());
        push_digest(&mut bytes, &self.source_commitment);
        push_digest(&mut bytes, &self.prior_scale_context);
        push_digest(&mut bytes, &self.recipe);
        bytes.push(self.execution_profile.code());
        Ok(TypedDigest32::sha256(
            "symtropy.scale-cell-001f.hydration-request.identity.v1",
            1,
            &bytes,
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HydrationArtifact {
    request: HydrationRequest,
    content: TypedDigest32,
}

impl HydrationArtifact {
    fn build(request: HydrationRequest) -> Result<Self, RefError> {
        request.validate()?;
        let content = hydration_content_digest(&request)?;
        Ok(Self { request, content })
    }

    fn validate(&self) -> Result<(), RefError> {
        self.request.validate()?;
        self.content.validate()?;
        if !self
            .content
            .same_typed_value(&hydration_content_digest(&self.request)?)
        {
            return Err(RefError::ArtifactContentMismatch);
        }
        Ok(())
    }

    fn execution_receipt(&self) -> Result<TypedDigest32, RefError> {
        self.validate()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"symtropy.scale-cell-001f.hydration-completion.v1\0");
        push_digest(&mut bytes, &self.request.execution_evidence()?);
        push_digest(&mut bytes, &self.content);
        Ok(TypedDigest32::sha256(
            "symtropy.scale-cell-001f.hydration-completion.identity.v1",
            1,
            &bytes,
        )?)
    }
}

fn hydration_content_digest(request: &HydrationRequest) -> Result<TypedDigest32, RefError> {
    request.validate()?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"symtropy.scale-cell-001f.hydrated-view.v1\0");
    push_string(&mut bytes, request.world.as_str());
    push_string(&mut bytes, request.scope.as_str());
    push_digest(&mut bytes, &request.source_commitment);
    push_digest(&mut bytes, &request.prior_scale_context);
    push_digest(&mut bytes, &request.recipe);
    Ok(TypedDigest32::sha256(
        "symtropy.scale-cell-001f.hydrated-view.content.v1",
        1,
        &bytes,
    )?)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InstalledView {
    source_commitment: TypedDigest32,
    prior_scale_context: TypedDigest32,
    recipe: TypedDigest32,
    content: TypedDigest32,
}

impl InstalledView {
    fn from_artifact(artifact: &HydrationArtifact) -> Self {
        Self {
            source_commitment: artifact.request.source_commitment.clone(),
            prior_scale_context: artifact.request.prior_scale_context.clone(),
            recipe: artifact.request.recipe.clone(),
            content: artifact.content.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CompletionOutcome {
    Installed,
    AlreadyInstalled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservationRuntime {
    next_request_id: u64,
    desired_request_by_scope: BTreeMap<ScopeId, u64>,
    pending_request_ids: BTreeSet<u64>,
    installed: BTreeMap<ScopeId, InstalledView>,
    accepted_by_request: BTreeMap<u64, TypedDigest32>,
    completion_trace: Vec<u64>,
}

impl Default for ObservationRuntime {
    fn default() -> Self {
        Self {
            next_request_id: 1,
            desired_request_by_scope: BTreeMap::new(),
            pending_request_ids: BTreeSet::new(),
            installed: BTreeMap::new(),
            accepted_by_request: BTreeMap::new(),
            completion_trace: Vec::new(),
        }
    }
}

impl ObservationRuntime {
    fn request_prewarm(
        &mut self,
        current: &CanonicalCellState,
        known_scopes: &BTreeSet<ScopeId>,
        scope: ScopeId,
        recipe: TypedDigest32,
        execution_profile: ExecutionProfile,
    ) -> Result<HydrationRequest, RefError> {
        current.validate()?;
        scope.validate()?;
        recipe.validate()?;
        if !known_scopes.contains(&scope) {
            return Err(RefError::UnknownScope);
        }
        if self.pending_request_ids.len() >= MAX_PENDING_HYDRATIONS {
            return Err(RefError::TooManyPendingHydrations);
        }

        let request_id = self.next_request_id;
        self.next_request_id = self
            .next_request_id
            .checked_add(1)
            .ok_or(RefError::CounterOverflow)?;
        let request = HydrationRequest {
            request_id,
            world: current.world.clone(),
            scope: scope.clone(),
            source_commitment: current.commitment()?,
            prior_scale_context: current.prior_scale_context.clone(),
            recipe,
            execution_profile,
        };
        request.validate()?;

        self.desired_request_by_scope.insert(scope, request_id);
        self.pending_request_ids.insert(request_id);
        Ok(request)
    }

    fn complete(
        &mut self,
        current: &CanonicalCellState,
        artifact: HydrationArtifact,
    ) -> Result<CompletionOutcome, RefError> {
        current.validate()?;
        artifact.validate()?;
        self.completion_trace.push(artifact.request.request_id);

        let receipt = artifact.execution_receipt()?;
        if artifact.request.world != current.world {
            self.pending_request_ids.remove(&artifact.request.request_id);
            return Err(RefError::WrongWorld);
        }
        if !artifact
            .request
            .prior_scale_context
            .same_typed_value(&current.prior_scale_context)
        {
            self.pending_request_ids.remove(&artifact.request.request_id);
            return Err(RefError::StaleScaleContext);
        }
        if !artifact
            .request
            .source_commitment
            .same_typed_value(&current.commitment()?)
        {
            self.pending_request_ids.remove(&artifact.request.request_id);
            return Err(RefError::StaleCanonicalSource);
        }

        if let Some(previous) = self.accepted_by_request.get(&artifact.request.request_id) {
            if previous.same_typed_value(&receipt) {
                return Ok(CompletionOutcome::AlreadyInstalled);
            }
            return Err(RefError::ConflictingRetry);
        }
        if !self
            .pending_request_ids
            .contains(&artifact.request.request_id)
        {
            return Err(RefError::UnknownPendingRequest);
        }

        let Some(desired_id) = self.desired_request_by_scope.get(&artifact.request.scope) else {
            self.pending_request_ids.remove(&artifact.request.request_id);
            return Err(RefError::NoDesiredRequest);
        };
        if *desired_id != artifact.request.request_id {
            self.pending_request_ids.remove(&artifact.request.request_id);
            return Err(RefError::SupersededRequest);
        }

        self.pending_request_ids.remove(&artifact.request.request_id);
        self.accepted_by_request
            .insert(artifact.request.request_id, receipt);
        self.installed.insert(
            artifact.request.scope.clone(),
            InstalledView::from_artifact(&artifact),
        );
        Ok(CompletionOutcome::Installed)
    }

    fn current_view<'a>(
        &'a self,
        current: &CanonicalCellState,
        scope: &ScopeId,
    ) -> Result<Option<&'a InstalledView>, RefError> {
        current.validate()?;
        scope.validate()?;
        let Some(view) = self.installed.get(scope) else {
            return Ok(None);
        };
        if !view
            .prior_scale_context
            .same_typed_value(&current.prior_scale_context)
            || !view
                .source_commitment
                .same_typed_value(&current.commitment()?)
        {
            return Ok(None);
        }
        Ok(Some(view))
    }

    fn installed_digest(&self) -> Result<TypedDigest32, RefError> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"symtropy.scale-cell-001f.installed-views.v1\0");
        bytes.extend_from_slice(&(self.installed.len() as u32).to_le_bytes());
        for (scope, view) in &self.installed {
            push_string(&mut bytes, scope.as_str());
            push_digest(&mut bytes, &view.source_commitment);
            push_digest(&mut bytes, &view.prior_scale_context);
            push_digest(&mut bytes, &view.recipe);
            push_digest(&mut bytes, &view.content);
        }
        Ok(TypedDigest32::sha256(
            "symtropy.scale-cell-001f.installed-views.identity.v1",
            1,
            &bytes,
        )?)
    }
}

fn fixture() -> CanonicalCellState {
    CanonicalCellState {
        world: WorldInstanceId::parse("world:scale-cell-001").unwrap(),
        authority_generation: 7,
        exact_total: 100,
        process_counter: 11,
        prior_scale_context: digest(
            "symtropy.scale-cell-001e.continuation-frame-context.identity.v1",
            b"opaque-001e-reference-context",
        )
        .unwrap(),
    }
}

fn known_scopes() -> BTreeSet<ScopeId> {
    [
        "system:fixture",
        "body:a",
        "region:r0",
        "local:l0",
        "structure:s0",
        "micro:m0",
    ]
    .into_iter()
    .map(|value| ScopeId::parse(value).unwrap())
    .collect()
}

fn recipe(value: &str) -> TypedDigest32 {
    digest("symtropy.scale-cell-001f.view-recipe.v1", value.as_bytes()).unwrap()
}

fn digest(domain: &str, value: &[u8]) -> Result<TypedDigest32, RefError> {
    Ok(TypedDigest32::sha256(domain, 1, value)?)
}

fn push_string(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn push_digest(bytes: &mut Vec<u8>, digest: &TypedDigest32) {
    push_string(bytes, &digest.domain);
    match &digest.algorithm {
        DigestAlgorithm::Sha256 => bytes.push(0),
        DigestAlgorithm::Other(name) => {
            bytes.push(1);
            push_string(bytes, name);
        }
    }
    bytes.extend_from_slice(&digest.schema_version.to_le_bytes());
    bytes.extend_from_slice(&digest.value);
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RefError {
    Contract,
    InvalidRequestId,
    CounterOverflow,
    UnknownScope,
    TooManyPendingHydrations,
    ArtifactContentMismatch,
    WrongWorld,
    StaleCanonicalSource,
    StaleScaleContext,
    NoDesiredRequest,
    SupersededRequest,
    ConflictingRetry,
    UnknownPendingRequest,
}

impl From<ContractError> for RefError {
    fn from(_: ContractError) -> Self {
        Self::Contract
    }
}

#[test]
fn predictive_prewarm_changes_only_observation_runtime() {
    let cell = fixture();
    let before = cell.commitment().unwrap();
    let scopes = known_scopes();
    let mut runtime = ObservationRuntime::default();

    let request = runtime
        .request_prewarm(
            &cell,
            &scopes,
            ScopeId::parse("micro:m0").unwrap(),
            recipe("micro-inspection-v1"),
            ExecutionProfile::ScaleRuntimeParallelV1,
        )
        .unwrap();
    let artifact = HydrationArtifact::build(request).unwrap();
    assert_eq!(
        runtime.complete(&cell, artifact).unwrap(),
        CompletionOutcome::Installed
    );

    assert_eq!(cell.commitment().unwrap(), before);
    assert!(runtime
        .installed
        .contains_key(&ScopeId::parse("micro:m0").unwrap()));
}

#[test]
fn serial_and_reordered_parallel_completion_have_same_semantic_result() {
    let cell = fixture();
    let canonical_before = cell.commitment().unwrap();
    let scopes = known_scopes();
    let targets = [
        ("body:a", "body-view-v1"),
        ("region:r0", "region-view-v1"),
        ("structure:s0", "structure-view-v1"),
        ("micro:m0", "micro-view-v1"),
    ];

    let mut serial = ObservationRuntime::default();
    let mut serial_artifacts = Vec::new();
    for (scope, recipe_name) in targets {
        let request = serial
            .request_prewarm(
                &cell,
                &scopes,
                ScopeId::parse(scope).unwrap(),
                recipe(recipe_name),
                ExecutionProfile::ScaleReferenceSerialV1,
            )
            .unwrap();
        serial_artifacts.push(HydrationArtifact::build(request).unwrap());
    }
    for artifact in serial_artifacts {
        assert_eq!(
            serial.complete(&cell, artifact).unwrap(),
            CompletionOutcome::Installed
        );
    }

    let mut parallel = ObservationRuntime::default();
    let mut parallel_artifacts = Vec::new();
    for (scope, recipe_name) in targets {
        let request = parallel
            .request_prewarm(
                &cell,
                &scopes,
                ScopeId::parse(scope).unwrap(),
                recipe(recipe_name),
                ExecutionProfile::ScaleRuntimeParallelV1,
            )
            .unwrap();
        parallel_artifacts.push(HydrationArtifact::build(request).unwrap());
    }
    for index in [2_usize, 0, 3, 1] {
        assert_eq!(
            parallel
                .complete(&cell, parallel_artifacts[index].clone())
                .unwrap(),
            CompletionOutcome::Installed
        );
    }

    assert_eq!(cell.commitment().unwrap(), canonical_before);
    assert_eq!(
        serial.installed_digest().unwrap(),
        parallel.installed_digest().unwrap()
    );
    assert_ne!(serial.completion_trace, parallel.completion_trace);
}

#[test]
fn stale_result_cannot_win_by_finishing_last() {
    let old_cell = fixture();
    let scopes = known_scopes();
    let mut runtime = ObservationRuntime::default();
    let target = ScopeId::parse("micro:m0").unwrap();

    let old_request = runtime
        .request_prewarm(
            &old_cell,
            &scopes,
            target.clone(),
            recipe("micro-view-v1"),
            ExecutionProfile::ScaleRuntimeParallelV1,
        )
        .unwrap();
    let old_artifact = HydrationArtifact::build(old_request).unwrap();

    let new_cell = old_cell.advance_reference_generation().unwrap();
    let new_request = runtime
        .request_prewarm(
            &new_cell,
            &scopes,
            target.clone(),
            recipe("micro-view-v1"),
            ExecutionProfile::ScaleRuntimeParallelV1,
        )
        .unwrap();
    let new_artifact = HydrationArtifact::build(new_request).unwrap();

    assert_eq!(
        runtime.complete(&new_cell, new_artifact.clone()).unwrap(),
        CompletionOutcome::Installed
    );
    let installed_after_new = runtime.installed.get(&target).unwrap().clone();

    assert_eq!(
        runtime.complete(&new_cell, old_artifact),
        Err(RefError::StaleCanonicalSource)
    );
    assert_eq!(runtime.installed.get(&target), Some(&installed_after_new));
    assert!(installed_after_new
        .source_commitment
        .same_typed_value(&new_cell.commitment().unwrap()));
}

#[test]
fn retained_view_does_not_self_authenticate_after_source_advances() {
    let cell = fixture();
    let scopes = known_scopes();
    let mut runtime = ObservationRuntime::default();
    let target = ScopeId::parse("structure:s0").unwrap();
    let request = runtime
        .request_prewarm(
            &cell,
            &scopes,
            target.clone(),
            recipe("structure-view-v1"),
            ExecutionProfile::ScaleRuntimeParallelV1,
        )
        .unwrap();
    let artifact = HydrationArtifact::build(request).unwrap();

    assert_eq!(
        runtime.complete(&cell, artifact.clone()).unwrap(),
        CompletionOutcome::Installed
    );
    assert!(runtime.current_view(&cell, &target).unwrap().is_some());

    let advanced = cell.advance_reference_generation().unwrap();
    assert!(runtime.current_view(&advanced, &target).unwrap().is_none());
    assert!(runtime.installed.contains_key(&target));
    assert_eq!(
        runtime.complete(&advanced, artifact),
        Err(RefError::StaleCanonicalSource)
    );
}

#[test]
fn same_source_superseded_request_cannot_replace_newer_observation_intent() {
    let cell = fixture();
    let scopes = known_scopes();
    let mut runtime = ObservationRuntime::default();
    let target = ScopeId::parse("local:l0").unwrap();

    let old_request = runtime
        .request_prewarm(
            &cell,
            &scopes,
            target.clone(),
            recipe("local-overview-v1"),
            ExecutionProfile::ScaleRuntimeParallelV1,
        )
        .unwrap();
    let old_artifact = HydrationArtifact::build(old_request).unwrap();
    let new_request = runtime
        .request_prewarm(
            &cell,
            &scopes,
            target.clone(),
            recipe("local-inspection-v2"),
            ExecutionProfile::ScaleRuntimeParallelV1,
        )
        .unwrap();
    let new_artifact = HydrationArtifact::build(new_request).unwrap();

    assert_eq!(
        runtime.complete(&cell, new_artifact.clone()).unwrap(),
        CompletionOutcome::Installed
    );
    let installed = runtime.installed.get(&target).unwrap().clone();
    assert_eq!(
        runtime.complete(&cell, old_artifact),
        Err(RefError::SupersededRequest)
    );
    assert_eq!(runtime.installed.get(&target), Some(&installed));
    assert!(installed
        .recipe
        .same_typed_value(&recipe("local-inspection-v2")));
}

#[test]
fn exact_completion_retry_is_idempotent() {
    let cell = fixture();
    let scopes = known_scopes();
    let mut runtime = ObservationRuntime::default();
    let request = runtime
        .request_prewarm(
            &cell,
            &scopes,
            ScopeId::parse("structure:s0").unwrap(),
            recipe("structure-view-v1"),
            ExecutionProfile::ScaleRuntimeParallelV1,
        )
        .unwrap();
    let artifact = HydrationArtifact::build(request).unwrap();

    assert_eq!(
        runtime.complete(&cell, artifact.clone()).unwrap(),
        CompletionOutcome::Installed
    );
    let installed = runtime.installed_digest().unwrap();
    assert_eq!(
        runtime.complete(&cell, artifact).unwrap(),
        CompletionOutcome::AlreadyInstalled
    );
    assert_eq!(runtime.installed_digest().unwrap(), installed);
}

#[test]
fn conflicting_retry_with_same_request_id_fails_closed() {
    let cell = fixture();
    let scopes = known_scopes();
    let mut runtime = ObservationRuntime::default();
    let request = runtime
        .request_prewarm(
            &cell,
            &scopes,
            ScopeId::parse("region:r0").unwrap(),
            recipe("region-view-v1"),
            ExecutionProfile::ScaleRuntimeParallelV1,
        )
        .unwrap();
    let artifact = HydrationArtifact::build(request.clone()).unwrap();
    assert_eq!(
        runtime.complete(&cell, artifact).unwrap(),
        CompletionOutcome::Installed
    );

    let mut conflicting_request = request;
    conflicting_request.recipe = recipe("region-view-conflict");
    let conflicting = HydrationArtifact::build(conflicting_request).unwrap();
    assert_eq!(
        runtime.complete(&cell, conflicting),
        Err(RefError::ConflictingRetry)
    );
}

#[test]
fn unknown_scope_prewarm_fails_without_mutating_runtime_or_world() {
    let cell = fixture();
    let before_cell = cell.commitment().unwrap();
    let scopes = known_scopes();
    let mut runtime = ObservationRuntime::default();
    let before_runtime = runtime.clone();

    assert_eq!(
        runtime.request_prewarm(
            &cell,
            &scopes,
            ScopeId::parse("micro:missing").unwrap(),
            recipe("missing-view"),
            ExecutionProfile::ScaleRuntimeParallelV1,
        ),
        Err(RefError::UnknownScope)
    );
    assert_eq!(runtime, before_runtime);
    assert_eq!(cell.commitment().unwrap(), before_cell);
}

#[test]
fn execution_profile_changes_execution_evidence_not_hydrated_semantics() {
    let cell = fixture();
    let scope = ScopeId::parse("body:a").unwrap();
    let source_commitment = cell.commitment().unwrap();
    let view_recipe = recipe("body-view-v1");

    let serial = HydrationRequest {
        request_id: 1,
        world: cell.world.clone(),
        scope: scope.clone(),
        source_commitment: source_commitment.clone(),
        prior_scale_context: cell.prior_scale_context.clone(),
        recipe: view_recipe.clone(),
        execution_profile: ExecutionProfile::ScaleReferenceSerialV1,
    };
    let parallel = HydrationRequest {
        request_id: 1,
        world: cell.world.clone(),
        scope,
        source_commitment,
        prior_scale_context: cell.prior_scale_context.clone(),
        recipe: view_recipe,
        execution_profile: ExecutionProfile::ScaleRuntimeParallelV1,
    };

    assert_ne!(
        serial.execution_evidence().unwrap(),
        parallel.execution_evidence().unwrap()
    );
    assert_eq!(
        HydrationArtifact::build(serial).unwrap().content,
        HydrationArtifact::build(parallel).unwrap().content
    );
}
