// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! SCALE-CELL-001F async/prewarm reference fixture, revision 2.
//!
//! This tranche treats the prior SCALE-CELL semantic subject as opaque typed
//! evidence and proves only async observation-hydration lifecycle semantics.
//! It does not implement domain evolution, canonical refinement, scheduler
//! execution, Bevy, networking, filesystem I/O, or production task-runtime
//! authority.
//!
//! The central theorem is:
//!
//! predictive hydration / cancellation / completion order / runtime restart
//! / executor choice != canonical world authority.

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
        bytes.extend_from_slice(b"symtropy.scale-cell-001f.canonical-cell.v2\0");
        push_string(&mut bytes, self.world.as_str());
        bytes.extend_from_slice(&self.authority_generation.to_le_bytes());
        bytes.extend_from_slice(&self.exact_total.to_le_bytes());
        bytes.extend_from_slice(&self.process_counter.to_le_bytes());
        push_digest(&mut bytes, &self.prior_scale_context);
        Ok(TypedDigest32::sha256(
            "symtropy.scale-cell-001f.canonical-cell.identity.v2",
            2,
            &bytes,
        )?)
    }

    fn advance_reference_generation(&self) -> Result<Self, RefError> {
        self.validate()?;
        let next = Self {
            world: self.world.clone(),
            authority_generation: self
                .authority_generation
                .checked_add(1)
                .ok_or(RefError::CounterOverflow)?,
            exact_total: self.exact_total,
            process_counter: self
                .process_counter
                .checked_add(1)
                .ok_or(RefError::CounterOverflow)?,
            prior_scale_context: self.prior_scale_context.clone(),
        };
        next.validate()?;
        Ok(next)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HydrationRequest {
    runtime_epoch: TypedDigest32,
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
        self.runtime_epoch.validate()?;
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

    fn lifecycle_identity(&self) -> Result<TypedDigest32, RefError> {
        self.validate()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"symtropy.scale-cell-001f.hydration-request.v2\0");
        push_digest(&mut bytes, &self.runtime_epoch);
        bytes.extend_from_slice(&self.request_id.to_le_bytes());
        push_string(&mut bytes, self.world.as_str());
        push_string(&mut bytes, self.scope.as_str());
        push_digest(&mut bytes, &self.source_commitment);
        push_digest(&mut bytes, &self.prior_scale_context);
        push_digest(&mut bytes, &self.recipe);
        bytes.push(self.execution_profile.code());
        Ok(TypedDigest32::sha256(
            "symtropy.scale-cell-001f.hydration-request.identity.v2",
            2,
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
        bytes.extend_from_slice(b"symtropy.scale-cell-001f.hydration-completion.v2\0");
        push_digest(&mut bytes, &self.request.lifecycle_identity()?);
        push_digest(&mut bytes, &self.content);
        Ok(TypedDigest32::sha256(
            "symtropy.scale-cell-001f.hydration-completion.identity.v2",
            2,
            &bytes,
        )?)
    }
}

fn hydration_content_digest(request: &HydrationRequest) -> Result<TypedDigest32, RefError> {
    request.validate()?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"symtropy.scale-cell-001f.hydrated-view.v2\0");
    push_string(&mut bytes, request.world.as_str());
    push_string(&mut bytes, request.scope.as_str());
    push_digest(&mut bytes, &request.source_commitment);
    push_digest(&mut bytes, &request.prior_scale_context);
    push_digest(&mut bytes, &request.recipe);
    Ok(TypedDigest32::sha256(
        "symtropy.scale-cell-001f.hydrated-view.content.v2",
        2,
        &bytes,
    )?)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InstalledView {
    request_id: u64,
    source_commitment: TypedDigest32,
    prior_scale_context: TypedDigest32,
    recipe: TypedDigest32,
    content: TypedDigest32,
    execution_receipt: TypedDigest32,
}

impl InstalledView {
    fn from_artifact(artifact: &HydrationArtifact) -> Result<Self, RefError> {
        Ok(Self {
            request_id: artifact.request.request_id,
            source_commitment: artifact.request.source_commitment.clone(),
            prior_scale_context: artifact.request.prior_scale_context.clone(),
            recipe: artifact.request.recipe.clone(),
            content: artifact.content.clone(),
            execution_receipt: artifact.execution_receipt()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingHydration {
    request: HydrationRequest,
    lifecycle_identity: TypedDigest32,
}

impl PendingHydration {
    fn new(request: HydrationRequest) -> Result<Self, RefError> {
        let lifecycle_identity = request.lifecycle_identity()?;
        Ok(Self {
            request,
            lifecycle_identity,
        })
    }

    fn exactly_matches(&self, request: &HydrationRequest) -> Result<bool, RefError> {
        Ok(self
            .lifecycle_identity
            .same_typed_value(&request.lifecycle_identity()?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CompletionOutcome {
    Installed,
    AlreadyInstalled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservationRuntime {
    epoch: TypedDigest32,
    next_request_id: u64,
    desired_request_by_scope: BTreeMap<ScopeId, u64>,
    pending: BTreeMap<u64, PendingHydration>,
    installed: BTreeMap<ScopeId, InstalledView>,
    completion_trace: Vec<TypedDigest32>,
}

impl ObservationRuntime {
    fn new(epoch: TypedDigest32) -> Result<Self, RefError> {
        epoch.validate()?;
        Ok(Self {
            epoch,
            next_request_id: 1,
            desired_request_by_scope: BTreeMap::new(),
            pending: BTreeMap::new(),
            installed: BTreeMap::new(),
            completion_trace: Vec::new(),
        })
    }

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

        self.reconcile_pending(current)?;

        let request_id = self.next_request_id;
        let next_request_id = request_id
            .checked_add(1)
            .ok_or(RefError::CounterOverflow)?;
        let previous_desired = self.desired_request_by_scope.get(&scope).copied();
        let previous_is_pending = previous_desired
            .map(|id| self.pending.contains_key(&id))
            .unwrap_or(false);
        let effective_pending = self.pending.len() - usize::from(previous_is_pending);
        if effective_pending >= MAX_PENDING_HYDRATIONS {
            return Err(RefError::TooManyPendingHydrations);
        }

        let request = HydrationRequest {
            runtime_epoch: self.epoch.clone(),
            request_id,
            world: current.world.clone(),
            scope: scope.clone(),
            source_commitment: current.commitment()?,
            prior_scale_context: current.prior_scale_context.clone(),
            recipe,
            execution_profile,
        };
        let pending = PendingHydration::new(request.clone())?;

        if let Some(previous) = previous_desired {
            self.pending.remove(&previous);
        }
        self.desired_request_by_scope.insert(scope, request_id);
        self.pending.insert(request_id, pending);
        self.next_request_id = next_request_id;
        Ok(request)
    }

    fn cancel_prewarm(&mut self, scope: &ScopeId) -> Result<bool, RefError> {
        scope.validate()?;
        let Some(request_id) = self.desired_request_by_scope.remove(scope) else {
            return Ok(false);
        };
        self.pending.remove(&request_id);
        Ok(true)
    }

    fn reconcile_pending(&mut self, current: &CanonicalCellState) -> Result<usize, RefError> {
        current.validate()?;
        let current_commitment = current.commitment()?;
        let stale_ids: Vec<u64> = self
            .pending
            .iter()
            .filter_map(|(request_id, pending)| {
                let request = &pending.request;
                let stale = request.world != current.world
                    || !request
                        .prior_scale_context
                        .same_typed_value(&current.prior_scale_context)
                    || !request
                        .source_commitment
                        .same_typed_value(&current_commitment);
                stale.then_some(*request_id)
            })
            .collect();

        for request_id in &stale_ids {
            if let Some(pending) = self.pending.remove(request_id) {
                if self
                    .desired_request_by_scope
                    .get(&pending.request.scope)
                    .is_some_and(|desired| desired == request_id)
                {
                    self.desired_request_by_scope.remove(&pending.request.scope);
                }
            }
        }
        Ok(stale_ids.len())
    }

    fn complete(
        &mut self,
        current: &CanonicalCellState,
        artifact: HydrationArtifact,
    ) -> Result<CompletionOutcome, RefError> {
        current.validate()?;
        artifact.validate()?;
        let lifecycle_identity = artifact.request.lifecycle_identity()?;
        self.completion_trace.push(lifecycle_identity);

        if !artifact
            .request
            .runtime_epoch
            .same_typed_value(&self.epoch)
        {
            return Err(RefError::WrongRuntimeEpoch);
        }
        if artifact.request.world != current.world {
            self.retire_exact_pending(&artifact.request)?;
            return Err(RefError::WrongWorld);
        }
        if !artifact
            .request
            .prior_scale_context
            .same_typed_value(&current.prior_scale_context)
        {
            self.retire_exact_pending(&artifact.request)?;
            return Err(RefError::StaleScaleContext);
        }
        if !artifact
            .request
            .source_commitment
            .same_typed_value(&current.commitment()?)
        {
            self.retire_exact_pending(&artifact.request)?;
            return Err(RefError::StaleCanonicalSource);
        }

        let Some(desired_id) = self.desired_request_by_scope.get(&artifact.request.scope) else {
            self.retire_exact_pending(&artifact.request)?;
            return Err(RefError::NoDesiredRequest);
        };
        if *desired_id != artifact.request.request_id {
            self.retire_exact_pending(&artifact.request)?;
            return Err(RefError::SupersededRequest);
        }

        if let Some(installed) = self.installed.get(&artifact.request.scope) {
            if installed.request_id == artifact.request.request_id {
                let receipt = artifact.execution_receipt()?;
                if installed.execution_receipt.same_typed_value(&receipt) {
                    return Ok(CompletionOutcome::AlreadyInstalled);
                }
                return Err(RefError::ConflictingRetry);
            }
        }

        let Some(pending) = self.pending.get(&artifact.request.request_id) else {
            return Err(RefError::UnknownPendingRequest);
        };
        if !pending.exactly_matches(&artifact.request)? {
            return Err(RefError::RequestIdentityCollision);
        }

        let installed = InstalledView::from_artifact(&artifact)?;
        self.pending.remove(&artifact.request.request_id);
        self.installed.insert(artifact.request.scope, installed);
        Ok(CompletionOutcome::Installed)
    }

    fn retire_exact_pending(&mut self, request: &HydrationRequest) -> Result<bool, RefError> {
        let Some(pending) = self.pending.get(&request.request_id) else {
            return Ok(false);
        };
        if !pending.exactly_matches(request)? {
            return Ok(false);
        }
        self.pending.remove(&request.request_id);
        Ok(true)
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
        bytes.extend_from_slice(b"symtropy.scale-cell-001f.installed-views.v2\0");
        bytes.extend_from_slice(&(self.installed.len() as u32).to_le_bytes());
        for (scope, view) in &self.installed {
            push_string(&mut bytes, scope.as_str());
            push_digest(&mut bytes, &view.source_commitment);
            push_digest(&mut bytes, &view.prior_scale_context);
            push_digest(&mut bytes, &view.recipe);
            push_digest(&mut bytes, &view.content);
        }
        Ok(TypedDigest32::sha256(
            "symtropy.scale-cell-001f.installed-views.identity.v2",
            2,
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

fn many_scopes(count: usize) -> BTreeSet<ScopeId> {
    (0..count)
        .map(|index| ScopeId::parse(format!("scope:{index}")).unwrap())
        .collect()
}

fn runtime_epoch(value: &str) -> TypedDigest32 {
    digest(
        "symtropy.scale-cell-001f.runtime-epoch.v2",
        value.as_bytes(),
    )
    .unwrap()
}

fn recipe(value: &str) -> TypedDigest32 {
    digest("symtropy.scale-cell-001f.view-recipe.v2", value.as_bytes()).unwrap()
}

fn digest(domain: &str, value: &[u8]) -> Result<TypedDigest32, RefError> {
    Ok(TypedDigest32::sha256(domain, 2, value)?)
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
    WrongRuntimeEpoch,
    WrongWorld,
    StaleCanonicalSource,
    StaleScaleContext,
    NoDesiredRequest,
    SupersededRequest,
    ConflictingRetry,
    UnknownPendingRequest,
    RequestIdentityCollision,
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
    let mut runtime = ObservationRuntime::new(runtime_epoch("runtime-a")).unwrap();

    let request = runtime
        .request_prewarm(
            &cell,
            &scopes,
            ScopeId::parse("micro:m0").unwrap(),
            recipe("micro-inspection-v2"),
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
        .current_view(&cell, &ScopeId::parse("micro:m0").unwrap())
        .unwrap()
        .is_some());
}

#[test]
fn serial_and_reordered_parallel_completion_have_same_semantic_views() {
    let cell = fixture();
    let scopes = known_scopes();
    let targets = ["body:a", "structure:s0", "micro:m0"];

    let mut serial = ObservationRuntime::new(runtime_epoch("serial-runtime")).unwrap();
    let mut serial_artifacts = Vec::new();
    for target in targets {
        let request = serial
            .request_prewarm(
                &cell,
                &scopes,
                ScopeId::parse(target).unwrap(),
                recipe(target),
                ExecutionProfile::ScaleReferenceSerialV1,
            )
            .unwrap();
        serial_artifacts.push(HydrationArtifact::build(request).unwrap());
    }
    for artifact in serial_artifacts {
        serial.complete(&cell, artifact).unwrap();
    }

    let mut parallel = ObservationRuntime::new(runtime_epoch("parallel-runtime")).unwrap();
    let mut parallel_artifacts = Vec::new();
    for target in targets {
        let request = parallel
            .request_prewarm(
                &cell,
                &scopes,
                ScopeId::parse(target).unwrap(),
                recipe(target),
                ExecutionProfile::ScaleRuntimeParallelV1,
            )
            .unwrap();
        parallel_artifacts.push(HydrationArtifact::build(request).unwrap());
    }
    parallel_artifacts.reverse();
    for artifact in parallel_artifacts {
        parallel.complete(&cell, artifact).unwrap();
    }

    assert_eq!(serial.installed_digest().unwrap(), parallel.installed_digest().unwrap());
    assert_ne!(serial.completion_trace, parallel.completion_trace);
}

#[test]
fn superseding_same_scope_releases_pending_capacity_immediately() {
    let cell = fixture();
    let scopes = known_scopes();
    let target = ScopeId::parse("micro:m0").unwrap();
    let mut runtime = ObservationRuntime::new(runtime_epoch("supersession-runtime")).unwrap();
    let mut first = None;

    for index in 0..(MAX_PENDING_HYDRATIONS * 2) {
        let request = runtime
            .request_prewarm(
                &cell,
                &scopes,
                target.clone(),
                recipe(&format!("recipe-{index}")),
                ExecutionProfile::ScaleRuntimeParallelV1,
            )
            .unwrap();
        if first.is_none() {
            first = Some(request.clone());
        }
        assert_eq!(runtime.pending.len(), 1);
    }

    let old_artifact = HydrationArtifact::build(first.unwrap()).unwrap();
    assert_eq!(
        runtime.complete(&cell, old_artifact),
        Err(RefError::SupersededRequest)
    );
    assert_eq!(runtime.pending.len(), 1);
}

#[test]
fn cancellation_releases_capacity_and_late_completion_cannot_install() {
    let cell = fixture();
    let scopes = known_scopes();
    let target = ScopeId::parse("structure:s0").unwrap();
    let mut runtime = ObservationRuntime::new(runtime_epoch("cancel-runtime")).unwrap();
    let request = runtime
        .request_prewarm(
            &cell,
            &scopes,
            target.clone(),
            recipe("structure-view-v2"),
            ExecutionProfile::ScaleRuntimeParallelV1,
        )
        .unwrap();
    let artifact = HydrationArtifact::build(request).unwrap();

    assert_eq!(runtime.pending.len(), 1);
    assert!(runtime.cancel_prewarm(&target).unwrap());
    assert!(runtime.pending.is_empty());
    assert_eq!(
        runtime.complete(&cell, artifact),
        Err(RefError::NoDesiredRequest)
    );
    assert!(runtime.installed.get(&target).is_none());
}

#[test]
fn runtime_epoch_prevents_request_id_aba_across_restart() {
    let cell = fixture();
    let scopes = known_scopes();
    let target = ScopeId::parse("micro:m0").unwrap();

    let mut old_runtime = ObservationRuntime::new(runtime_epoch("runtime-old")).unwrap();
    let old_request = old_runtime
        .request_prewarm(
            &cell,
            &scopes,
            target.clone(),
            recipe("same-semantic-view"),
            ExecutionProfile::ScaleRuntimeParallelV1,
        )
        .unwrap();
    assert_eq!(old_request.request_id, 1);
    let old_artifact = HydrationArtifact::build(old_request).unwrap();

    let mut new_runtime = ObservationRuntime::new(runtime_epoch("runtime-new")).unwrap();
    let new_request = new_runtime
        .request_prewarm(
            &cell,
            &scopes,
            target,
            recipe("same-semantic-view"),
            ExecutionProfile::ScaleRuntimeParallelV1,
        )
        .unwrap();
    assert_eq!(new_request.request_id, 1);
    let new_artifact = HydrationArtifact::build(new_request).unwrap();

    assert_eq!(old_artifact.content, new_artifact.content);
    assert_ne!(
        old_artifact.request.lifecycle_identity().unwrap(),
        new_artifact.request.lifecycle_identity().unwrap()
    );
    assert_eq!(
        new_runtime.complete(&cell, old_artifact),
        Err(RefError::WrongRuntimeEpoch)
    );
    assert_eq!(new_runtime.pending.len(), 1);
    assert_eq!(
        new_runtime.complete(&cell, new_artifact).unwrap(),
        CompletionOutcome::Installed
    );
}

#[test]
fn foreign_same_numeric_request_id_cannot_cancel_current_pending_work() {
    let cell = fixture();
    let scopes = known_scopes();
    let mut runtime = ObservationRuntime::new(runtime_epoch("runtime-current")).unwrap();
    let current_request = runtime
        .request_prewarm(
            &cell,
            &scopes,
            ScopeId::parse("local:l0").unwrap(),
            recipe("current"),
            ExecutionProfile::ScaleRuntimeParallelV1,
        )
        .unwrap();
    let current_artifact = HydrationArtifact::build(current_request).unwrap();

    let mut foreign_runtime = ObservationRuntime::new(runtime_epoch("runtime-foreign")).unwrap();
    let foreign_request = foreign_runtime
        .request_prewarm(
            &cell,
            &scopes,
            ScopeId::parse("local:l0").unwrap(),
            recipe("foreign"),
            ExecutionProfile::ScaleRuntimeParallelV1,
        )
        .unwrap();
    assert_eq!(foreign_request.request_id, 1);
    let foreign_artifact = HydrationArtifact::build(foreign_request).unwrap();

    assert_eq!(
        runtime.complete(&cell, foreign_artifact),
        Err(RefError::WrongRuntimeEpoch)
    );
    assert_eq!(runtime.pending.len(), 1);
    assert_eq!(
        runtime.complete(&cell, current_artifact).unwrap(),
        CompletionOutcome::Installed
    );
}

#[test]
fn stale_pending_work_is_reconciled_after_canonical_advance() {
    let cell = fixture();
    let scopes = known_scopes();
    let mut runtime = ObservationRuntime::new(runtime_epoch("reconcile-runtime")).unwrap();
    let old_request = runtime
        .request_prewarm(
            &cell,
            &scopes,
            ScopeId::parse("body:a").unwrap(),
            recipe("body-old"),
            ExecutionProfile::ScaleRuntimeParallelV1,
        )
        .unwrap();
    let old_artifact = HydrationArtifact::build(old_request).unwrap();
    assert_eq!(runtime.pending.len(), 1);

    let advanced = cell.advance_reference_generation().unwrap();
    assert_eq!(runtime.reconcile_pending(&advanced).unwrap(), 1);
    assert!(runtime.pending.is_empty());
    assert_eq!(
        runtime.complete(&advanced, old_artifact),
        Err(RefError::StaleCanonicalSource)
    );
}

#[test]
fn retained_installed_view_does_not_self_authenticate_after_source_advances() {
    let cell = fixture();
    let scopes = known_scopes();
    let target = ScopeId::parse("structure:s0").unwrap();
    let mut runtime = ObservationRuntime::new(runtime_epoch("retained-runtime")).unwrap();
    let request = runtime
        .request_prewarm(
            &cell,
            &scopes,
            target.clone(),
            recipe("structure-current"),
            ExecutionProfile::ScaleRuntimeParallelV1,
        )
        .unwrap();
    let artifact = HydrationArtifact::build(request).unwrap();
    runtime.complete(&cell, artifact.clone()).unwrap();
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
fn exact_retry_is_idempotent_only_while_bound_source_is_current() {
    let cell = fixture();
    let scopes = known_scopes();
    let target = ScopeId::parse("body:a").unwrap();
    let mut runtime = ObservationRuntime::new(runtime_epoch("retry-runtime")).unwrap();
    let request = runtime
        .request_prewarm(
            &cell,
            &scopes,
            target,
            recipe("body-view"),
            ExecutionProfile::ScaleReferenceSerialV1,
        )
        .unwrap();
    let artifact = HydrationArtifact::build(request).unwrap();

    assert_eq!(
        runtime.complete(&cell, artifact.clone()).unwrap(),
        CompletionOutcome::Installed
    );
    let installed_before = runtime.installed_digest().unwrap();
    assert_eq!(
        runtime.complete(&cell, artifact.clone()).unwrap(),
        CompletionOutcome::AlreadyInstalled
    );
    assert_eq!(runtime.installed_digest().unwrap(), installed_before);

    let advanced = cell.advance_reference_generation().unwrap();
    assert_eq!(
        runtime.complete(&advanced, artifact),
        Err(RefError::StaleCanonicalSource)
    );
}

#[test]
fn same_numeric_request_with_changed_identity_fails_without_evicting_pending() {
    let cell = fixture();
    let scopes = known_scopes();
    let mut runtime = ObservationRuntime::new(runtime_epoch("identity-runtime")).unwrap();
    let request = runtime
        .request_prewarm(
            &cell,
            &scopes,
            ScopeId::parse("body:a").unwrap(),
            recipe("body-view"),
            ExecutionProfile::ScaleRuntimeParallelV1,
        )
        .unwrap();
    let valid_artifact = HydrationArtifact::build(request.clone()).unwrap();

    let mut forged_request = request;
    forged_request.recipe = recipe("different-recipe");
    let forged_artifact = HydrationArtifact::build(forged_request).unwrap();
    assert_eq!(
        runtime.complete(&cell, forged_artifact),
        Err(RefError::RequestIdentityCollision)
    );
    assert_eq!(runtime.pending.len(), 1);
    assert_eq!(
        runtime.complete(&cell, valid_artifact).unwrap(),
        CompletionOutcome::Installed
    );
}

#[test]
fn pending_budget_backpressures_distinct_scopes_but_not_supersession() {
    let cell = fixture();
    let scopes = many_scopes(MAX_PENDING_HYDRATIONS + 1);
    let mut runtime = ObservationRuntime::new(runtime_epoch("budget-runtime")).unwrap();

    for index in 0..MAX_PENDING_HYDRATIONS {
        runtime
            .request_prewarm(
                &cell,
                &scopes,
                ScopeId::parse(format!("scope:{index}")).unwrap(),
                recipe(&format!("recipe-{index}")),
                ExecutionProfile::ScaleRuntimeParallelV1,
            )
            .unwrap();
    }
    assert_eq!(runtime.pending.len(), MAX_PENDING_HYDRATIONS);

    assert_eq!(
        runtime.request_prewarm(
            &cell,
            &scopes,
            ScopeId::parse(format!("scope:{MAX_PENDING_HYDRATIONS}")).unwrap(),
            recipe("overflow"),
            ExecutionProfile::ScaleRuntimeParallelV1,
        ),
        Err(RefError::TooManyPendingHydrations)
    );

    runtime
        .request_prewarm(
            &cell,
            &scopes,
            ScopeId::parse("scope:0").unwrap(),
            recipe("replacement"),
            ExecutionProfile::ScaleRuntimeParallelV1,
        )
        .unwrap();
    assert_eq!(runtime.pending.len(), MAX_PENDING_HYDRATIONS);
}

#[test]
fn executor_and_runtime_lifecycle_change_evidence_not_semantic_content() {
    let cell = fixture();
    let scopes = known_scopes();
    let target = ScopeId::parse("micro:m0").unwrap();
    let view_recipe = recipe("same-content");

    let mut serial = ObservationRuntime::new(runtime_epoch("serial-epoch")).unwrap();
    let serial_request = serial
        .request_prewarm(
            &cell,
            &scopes,
            target.clone(),
            view_recipe.clone(),
            ExecutionProfile::ScaleReferenceSerialV1,
        )
        .unwrap();

    let mut parallel = ObservationRuntime::new(runtime_epoch("parallel-epoch")).unwrap();
    let parallel_request = parallel
        .request_prewarm(
            &cell,
            &scopes,
            target,
            view_recipe,
            ExecutionProfile::ScaleRuntimeParallelV1,
        )
        .unwrap();

    let serial_artifact = HydrationArtifact::build(serial_request).unwrap();
    let parallel_artifact = HydrationArtifact::build(parallel_request).unwrap();
    assert_eq!(serial_artifact.content, parallel_artifact.content);
    assert_ne!(
        serial_artifact.request.lifecycle_identity().unwrap(),
        parallel_artifact.request.lifecycle_identity().unwrap()
    );
    assert_ne!(
        serial_artifact.execution_receipt().unwrap(),
        parallel_artifact.execution_receipt().unwrap()
    );
}
