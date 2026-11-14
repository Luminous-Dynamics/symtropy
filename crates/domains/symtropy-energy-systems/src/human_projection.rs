// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Role-specific, revision-bound human projections of the Electrical Service Graph.
//!
//! Projections change vocabulary and detail level only. They never turn declared
//! topology into live-state, isolation, safe-to-touch, or compliance claims.

use crate::{
    GraphEvidence, GraphEvidenceState, GraphMaturity, LoadCoverageStatus,
    PathWitnessPolicy, SourceMapAssessment,
};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionAudience {
    Resident,
    Operator,
    QualifiedPractitioner,
    EmergencyResponder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionRevisionStatus {
    CurrentRevision,
    StaleRevision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectedLoadStatus {
    DeclaredSourcePathListed,
    NoDeclaredSourcePathListed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionNoticeCode {
    DeclaredPathIsNotLiveFlow,
    NoDeclaredPathIsNotIsolation,
    NotSafeToTouchConclusion,
    NotInstallationApproval,
    GraphRevisionStale,
    DesignOnlyGraph,
    FieldReconciledWithinScope,
    IndependentlyReviewedWithinScope,
    StaleModuleConfiguration,
    LimitedPathWitnessPolicy,
}

impl ProjectionNoticeCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DeclaredPathIsNotLiveFlow => "PF-VIEW-0001",
            Self::NoDeclaredPathIsNotIsolation => "PF-VIEW-0002",
            Self::NotSafeToTouchConclusion => "PF-VIEW-0003",
            Self::NotInstallationApproval => "PF-VIEW-0004",
            Self::GraphRevisionStale => "PF-VIEW-0005",
            Self::DesignOnlyGraph => "PF-VIEW-0006",
            Self::FieldReconciledWithinScope => "PF-VIEW-0007",
            Self::IndependentlyReviewedWithinScope => "PF-VIEW-0008",
            Self::StaleModuleConfiguration => "PF-VIEW-0009",
            Self::LimitedPathWitnessPolicy => "PF-VIEW-0010",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionNoticeSeverity {
    Information,
    Caution,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectedEvidence {
    state: GraphEvidenceState,
    evidence_id: Option<String>,
}

impl ProjectedEvidence {
    fn from_graph_evidence(evidence: &GraphEvidence) -> Self {
        Self {
            state: evidence.state(),
            evidence_id: evidence.evidence_id().map(str::to_owned),
        }
    }
    pub const fn state(&self) -> GraphEvidenceState { self.state }
    pub fn evidence_id(&self) -> Option<&str> { self.evidence_id.as_deref() }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectedPath {
    source_id: String,
    node_ids: Vec<String>,
    edge_ids: Vec<String>,
    node_evidence: Vec<ProjectedEvidence>,
    edge_evidence: Vec<ProjectedEvidence>,
}

impl ProjectedPath {
    pub fn source_id(&self) -> &str { &self.source_id }
    pub fn node_ids(&self) -> &[String] { &self.node_ids }
    pub fn edge_ids(&self) -> &[String] { &self.edge_ids }
    pub fn node_evidence(&self) -> &[ProjectedEvidence] { &self.node_evidence }
    pub fn edge_evidence(&self) -> &[ProjectedEvidence] { &self.edge_evidence }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectedLoad {
    load_id: String,
    status: ProjectedLoadStatus,
    declared_source_count: usize,
    declared_source_ids: Option<Vec<String>>,
    path_witnesses: Option<Vec<ProjectedPath>>,
}

impl ProjectedLoad {
    pub fn load_id(&self) -> &str { &self.load_id }
    pub const fn status(&self) -> ProjectedLoadStatus { self.status }
    pub const fn declared_source_count(&self) -> usize { self.declared_source_count }
    pub fn declared_source_ids(&self) -> Option<&[String]> { self.declared_source_ids.as_deref() }
    pub fn path_witnesses(&self) -> Option<&[ProjectedPath]> { self.path_witnesses.as_deref() }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectionNotice {
    notice_id: String,
    code: ProjectionNoticeCode,
    severity: ProjectionNoticeSeverity,
    message: String,
    affected_id: Option<String>,
}

impl ProjectionNotice {
    pub fn notice_id(&self) -> &str { &self.notice_id }
    pub const fn code(&self) -> ProjectionNoticeCode { self.code }
    pub const fn severity(&self) -> ProjectionNoticeSeverity { self.severity }
    pub fn message(&self) -> &str { &self.message }
    pub fn affected_id(&self) -> Option<&str> { self.affected_id.as_deref() }
}

/// A presentation artifact bound to one exact source-map assessment.
/// projection_digest is supplied by the caller; this module does not hash UI
/// output or claim that a displayed projection is a physical safety control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoleProjection {
    audience: ProjectionAudience,
    graph_id: String,
    revision_id: String,
    graph_content_digest: String,
    projection_digest: String,
    scope_statement: String,
    revision_status: ProjectionRevisionStatus,
    graph_maturity: GraphMaturity,
    graph_maturity_evidence: ProjectedEvidence,
    path_witness_policy: PathWitnessPolicy,
    loads: Vec<ProjectedLoad>,
    stale_module_ids: Vec<String>,
    notices: Vec<ProjectionNotice>,
}

impl RoleProjection {
    pub const fn audience(&self) -> ProjectionAudience { self.audience }
    pub fn graph_id(&self) -> &str { &self.graph_id }
    pub fn revision_id(&self) -> &str { &self.revision_id }
    pub fn graph_content_digest(&self) -> &str { &self.graph_content_digest }
    pub fn projection_digest(&self) -> &str { &self.projection_digest }
    pub fn scope_statement(&self) -> &str { &self.scope_statement }
    pub const fn revision_status(&self) -> ProjectionRevisionStatus { self.revision_status }
    pub const fn graph_maturity(&self) -> GraphMaturity { self.graph_maturity }
    pub fn graph_maturity_evidence(&self) -> &ProjectedEvidence { &self.graph_maturity_evidence }
    pub const fn path_witness_policy(&self) -> PathWitnessPolicy { self.path_witness_policy }
    pub fn loads(&self) -> &[ProjectedLoad] { &self.loads }
    pub fn stale_module_ids(&self) -> &[String] { &self.stale_module_ids }
    pub fn notices(&self) -> &[ProjectionNotice] { &self.notices }

    pub fn notices_by_code(&self, code: ProjectionNoticeCode) -> impl Iterator<Item = &ProjectionNotice> {
        self.notices.iter().filter(move |notice| notice.code == code)
    }
}

pub fn compile_role_projection(
    source_map: &SourceMapAssessment,
    audience: ProjectionAudience,
    projection_digest: impl Into<String>,
) -> Result<RoleProjection, ProjectionError> {
    let projection_digest = projection_digest.into();
    require_non_empty("projection_digest", &projection_digest)?;

    let show_source_ids = audience != ProjectionAudience::Resident;
    let show_path_details = matches!(
        audience,
        ProjectionAudience::QualifiedPractitioner | ProjectionAudience::EmergencyResponder
    );

    let mut loads = Vec::with_capacity(source_map.load_coverage().len());
    let mut notices = Vec::new();

    for coverage in source_map.load_coverage() {
        let mut source_ids: Vec<String> = coverage.source_paths()
            .iter()
            .map(|path| path.source_id().to_owned())
            .collect();
        source_ids.sort();
        source_ids.dedup();

        let status = match coverage.status() {
            LoadCoverageStatus::DeclaredSourcePathFound => ProjectedLoadStatus::DeclaredSourcePathListed,
            LoadCoverageStatus::NoDeclaredSourcePath => {
                notices.push(notice(
                    ProjectionNoticeCode::NoDeclaredPathIsNotIsolation,
                    ProjectionNoticeSeverity::Caution,
                    no_path_message(audience),
                    Some(coverage.load_id().to_owned()),
                ));
                ProjectedLoadStatus::NoDeclaredSourcePathListed
            }
        };

        let path_witnesses = if show_path_details {
            Some(coverage.source_paths().iter().map(|path| ProjectedPath {
                source_id: path.source_id().to_owned(),
                node_ids: path.node_ids().to_vec(),
                edge_ids: path.edge_ids().to_vec(),
                node_evidence: path.node_evidence().iter()
                    .map(ProjectedEvidence::from_graph_evidence).collect(),
                edge_evidence: path.edge_evidence().iter()
                    .map(ProjectedEvidence::from_graph_evidence).collect(),
            }).collect())
        } else {
            None
        };

        loads.push(ProjectedLoad {
            load_id: coverage.load_id().to_owned(),
            status,
            declared_source_count: source_ids.len(),
            declared_source_ids: if show_source_ids { Some(source_ids) } else { None },
            path_witnesses,
        });
    }

    loads.sort_by(|left, right| left.load_id.cmp(&right.load_id));

    notices.push(notice(
        ProjectionNoticeCode::DeclaredPathIsNotLiveFlow,
        ProjectionNoticeSeverity::Caution,
        live_flow_message(audience),
        None,
    ));
    notices.push(notice(
        ProjectionNoticeCode::NotSafeToTouchConclusion,
        ProjectionNoticeSeverity::Caution,
        no_safe_touch_message(audience),
        None,
    ));
    notices.push(notice(
        ProjectionNoticeCode::NotInstallationApproval,
        ProjectionNoticeSeverity::Information,
        not_approval_message(audience),
        None,
    ));

    let revision_status = if source_map.revision_current() {
        ProjectionRevisionStatus::CurrentRevision
    } else {
        notices.push(notice(
            ProjectionNoticeCode::GraphRevisionStale,
            ProjectionNoticeSeverity::Caution,
            stale_revision_message(audience),
            Some(source_map.revision_id().to_owned()),
        ));
        ProjectionRevisionStatus::StaleRevision
    };

    match source_map.graph_maturity() {
        GraphMaturity::DesignOnly => notices.push(notice(
            ProjectionNoticeCode::DesignOnlyGraph,
            ProjectionNoticeSeverity::Caution,
            "This graph is design-only within its written scope; it is not a reconciled as-built record.",
            None,
        )),
        GraphMaturity::FieldReconciled => notices.push(notice(
            ProjectionNoticeCode::FieldReconciledWithinScope,
            ProjectionNoticeSeverity::Information,
            "Field reconciliation is limited to the graph's written scope and does not prove that hidden or undocumented sources are absent.",
            None,
        )),
        GraphMaturity::IndependentlyReviewed => notices.push(notice(
            ProjectionNoticeCode::IndependentlyReviewedWithinScope,
            ProjectionNoticeSeverity::Information,
            "Independent review applies only to its stated scope and evidence; this projection is not an installation certificate.",
            None,
        )),
    }

    let mut stale_module_ids = source_map.stale_module_configurations()
        .iter().map(|module| module.module_id().to_owned()).collect::<Vec<_>>();
    stale_module_ids.sort();
    stale_module_ids.dedup();

    for module in source_map.stale_module_configurations() {
        notices.push(notice(
            ProjectionNoticeCode::StaleModuleConfiguration,
            ProjectionNoticeSeverity::Caution,
            stale_module_message(audience, module.module_id(), module.configuration_digest()),
            Some(module.module_id().to_owned()),
        ));
    }

    if show_path_details && source_map.path_witness_policy() == PathWitnessPolicy::OneShortestHopPerSourceLoadPair {
        notices.push(notice(
            ProjectionNoticeCode::LimitedPathWitnessPolicy,
            ProjectionNoticeSeverity::Information,
            "Only one deterministic shortest-hop witness per source/load pair is shown. Alternate routes, edge-disjoint redundancy and common-cause dependencies require separate analysis.",
            None,
        ));
    }

    notices.sort_by(|left, right| left.notice_id.cmp(&right.notice_id));

    Ok(RoleProjection {
        audience,
        graph_id: source_map.graph_id().to_owned(),
        revision_id: source_map.revision_id().to_owned(),
        graph_content_digest: source_map.content_digest().to_owned(),
        projection_digest,
        scope_statement: source_map.scope_statement().to_owned(),
        revision_status,
        graph_maturity: source_map.graph_maturity(),
        graph_maturity_evidence: ProjectedEvidence::from_graph_evidence(source_map.graph_maturity_evidence()),
        path_witness_policy: source_map.path_witness_policy(),
        loads,
        stale_module_ids,
        notices,
    })
}

fn notice(
    code: ProjectionNoticeCode,
    severity: ProjectionNoticeSeverity,
    message: impl Into<String>,
    affected_id: Option<String>,
) -> ProjectionNotice {
    let message = message.into();
    let suffix = affected_id.as_deref().unwrap_or("document");
    ProjectionNotice {
        notice_id: format!("{}:{}", code.as_str(), suffix),
        code,
        severity,
        message,
        affected_id,
    }
}

fn live_flow_message(audience: ProjectionAudience) -> &'static str {
    match audience {
        ProjectionAudience::Resident =>
            "A listed supply path is part of the declared system record. It does not confirm that power is currently flowing or available.",
        ProjectionAudience::Operator =>
            "A listed path is declared topology only. It is not a live measurement, state estimate, source-availability indication, or capacity result.",
        ProjectionAudience::QualifiedPractitioner =>
            "A path witness represents declared directed topology only; it does not establish live flow, source availability, loading, or protection performance.",
        ProjectionAudience::EmergencyResponder =>
            "Treat source paths as declared topology, not proof of live flow or a complete physical source inventory.",
    }
}

fn no_path_message(audience: ProjectionAudience) -> &'static str {
    match audience {
        ProjectionAudience::Resident =>
            "No supply path is listed for this service in this graph revision. This does not mean the service is de-energized.",
        ProjectionAudience::Operator =>
            "No declared source path was found in this revision. Do not infer that the circuit is disconnected, de-energized, or unavailable.",
        ProjectionAudience::QualifiedPractitioner =>
            "No declared PowerPath witness was found within this graph scope. This is not proof of physical disconnection, absence of voltage, or isolation.",
        ProjectionAudience::EmergencyResponder =>
            "No declared path is listed in this revision. Do not treat this as proof that every physical source has been discovered or isolated.",
    }
}

fn no_safe_touch_message(audience: ProjectionAudience) -> &'static str {
    match audience {
        ProjectionAudience::Resident =>
            "This view cannot establish that equipment is safe to touch. Do not open electrical equipment or touch conductors; contact a registered or qualified electrical professional.",
        ProjectionAudience::Operator =>
            "No software state in this projection establishes absence of voltage or permission to work. Follow the site's approved isolation and test process.",
        ProjectionAudience::QualifiedPractitioner =>
            "This projection does not establish absence of voltage, effective isolation, or safe-to-work status. Apply the applicable approved isolation and verification procedure.",
        ProjectionAudience::EmergencyResponder =>
            "Do not use this projection as proof of de-energization or isolation. Follow incident command, site procedures, and qualified electrical response protocols.",
    }
}

fn not_approval_message(audience: ProjectionAudience) -> &'static str {
    match audience {
        ProjectionAudience::Resident =>
            "This is an information view, not an installation approval or Certificate of Compliance.",
        ProjectionAudience::Operator =>
            "This view is not an engineering approval, installation inspection, or Certificate of Compliance.",
        ProjectionAudience::QualifiedPractitioner =>
            "The projection is not a design approval, statutory verification, or Certificate of Compliance.",
        ProjectionAudience::EmergencyResponder =>
            "This information view is not an installation inspection, statutory approval, or safety certification.",
    }
}

fn stale_revision_message(audience: ProjectionAudience) -> &'static str {
    match audience {
        ProjectionAudience::Resident =>
            "This information may not match the latest recorded installation revision. Confirm with the responsible professional.",
        ProjectionAudience::Operator =>
            "The displayed source map is tied to a stale graph revision. Reconcile the as-built record before operational reliance.",
        ProjectionAudience::QualifiedPractitioner =>
            "The projection is historical: its graph revision is not marked current. Reconcile exact asset/configuration identities before qualification.",
        ProjectionAudience::EmergencyResponder =>
            "This source map is not marked current. Treat all listed and unlisted sources as unresolved until confirmed on scene.",
    }
}

fn stale_module_message(audience: ProjectionAudience, module_id: &str, digest: &str) -> String {
    match audience {
        ProjectionAudience::Resident =>
            format!("A recorded equipment configuration is out of date ({module_id}). Ask the responsible electrical professional to reconcile it."),
        ProjectionAudience::Operator =>
            format!("Module {module_id} configuration {digest} is stale; affected operating information must be reconciled."),
        ProjectionAudience::QualifiedPractitioner =>
            format!("Module {module_id} has stale configuration digest {digest}; dependent compatibility and documentation claims require reconciliation."),
        ProjectionAudience::EmergencyResponder =>
            format!("Module {module_id} has a stale recorded configuration ({digest}); do not rely on its documented capabilities without on-scene confirmation."),
    }
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), ProjectionError> {
    if value.trim().is_empty() {
        return Err(ProjectionError::EmptyField { field });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionError {
    EmptyField { field: &'static str },
}

impl core::fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ProjectionError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        GraphEvidence, GraphRevision, ModuleIdentity, ServiceEdge, ServiceEdgeKind,
        ServiceGraph, ServiceNode, ServiceNodeKind,
    };

    fn evidence(state: GraphEvidenceState, id: &str) -> GraphEvidence {
        GraphEvidence::new(
            state,
            if state == GraphEvidenceState::Unknown { None } else { Some(id.to_owned()) },
        ).unwrap()
    }

    fn graph(current: bool, with_path: bool, stale_module: bool) -> ServiceGraph {
        let revision = GraphRevision::new(
            "building:test", "revision:7", "sha256:graph:7",
            "Synthetic residential supply map scope",
            GraphMaturity::DesignOnly,
            evidence(GraphEvidenceState::DesignDeclared, "evidence:design"),
            current,
        ).unwrap();

        let mut source = ServiceNode::new(
            "source:grid", ServiceNodeKind::UtilitySource, "Utility source",
            evidence(GraphEvidenceState::DesignDeclared, "evidence:grid"),
        ).unwrap();
        if stale_module {
            source = source.with_module_identity(
                ModuleIdentity::new("module:inverter", "hw:R2", "sha256:old-config", false).unwrap()
            );
        }
        let load = ServiceNode::new(
            "load:lighting", ServiceNodeKind::Load, "Lighting",
            evidence(GraphEvidenceState::DesignDeclared, "evidence:load"),
        ).unwrap();
        let edges = if with_path {
            vec![ServiceEdge::new(
                "edge:grid-lighting", "source:grid", "load:lighting",
                ServiceEdgeKind::PowerPath,
                evidence(GraphEvidenceState::DesignDeclared, "evidence:path"),
            ).unwrap()]
        } else {
            Vec::new()
        };
        ServiceGraph::new(revision, vec![source, load], edges).unwrap()
    }

    fn projection(
        audience: ProjectionAudience,
        current: bool,
        with_path: bool,
        stale_module: bool,
    ) -> RoleProjection {
        let assessment = graph(current, with_path, stale_module).assess_declared_source_paths();
        compile_role_projection(&assessment, audience, "sha256:projection:test").unwrap()
    }

    #[test]
    fn all_roles_bind_to_the_same_graph_revision() {
        let roles = [
            ProjectionAudience::Resident,
            ProjectionAudience::Operator,
            ProjectionAudience::QualifiedPractitioner,
            ProjectionAudience::EmergencyResponder,
        ];
        for role in roles {
            let view = projection(role, true, true, false);
            assert_eq!(view.graph_id(), "building:test");
            assert_eq!(view.revision_id(), "revision:7");
            assert_eq!(view.graph_content_digest(), "sha256:graph:7");
            assert_eq!(view.projection_digest(), "sha256:projection:test");
            assert_eq!(view.revision_status(), ProjectionRevisionStatus::CurrentRevision);
        }
    }

    #[test]
    fn resident_view_uses_summary_without_exposing_technical_paths() {
        let view = projection(ProjectionAudience::Resident, true, true, false);
        let load = &view.loads()[0];
        assert_eq!(load.declared_source_count(), 1);
        assert_eq!(load.declared_source_ids(), None);
        assert_eq!(load.path_witnesses(), None);
    }

    #[test]
    fn operator_view_lists_sources_without_full_path_details() {
        let view = projection(ProjectionAudience::Operator, true, true, false);
        let load = &view.loads()[0];
        assert_eq!(load.declared_source_ids(), Some(&["source:grid".to_owned()][..]));
        assert_eq!(load.path_witnesses(), None);
    }

    #[test]
    fn practitioner_view_preserves_route_and_evidence_details() {
        let view = projection(ProjectionAudience::QualifiedPractitioner, true, true, false);
        let load = &view.loads()[0];
        let path = &load.path_witnesses().unwrap()[0];
        assert_eq!(path.source_id(), "source:grid");
        assert_eq!(path.node_ids(), &["source:grid".to_owned(), "load:lighting".to_owned()]);
        assert_eq!(path.edge_ids(), &["edge:grid-lighting".to_owned()]);
        assert_eq!(path.edge_evidence()[0].evidence_id(), Some("evidence:path"));
    }

    #[test]
    fn emergency_view_preserves_route_and_evidence_details() {
        let view = projection(ProjectionAudience::EmergencyResponder, true, true, false);
        assert!(view.loads()[0].path_witnesses().is_some());
        assert!(view.notices().iter().any(|notice| notice.code() == ProjectionNoticeCode::NotSafeToTouchConclusion));
    }

    #[test]
    fn every_role_discloses_that_topology_is_not_live_flow() {
        for role in [
            ProjectionAudience::Resident,
            ProjectionAudience::Operator,
            ProjectionAudience::QualifiedPractitioner,
            ProjectionAudience::EmergencyResponder,
        ] {
            let view = projection(role, true, true, false);
            let notice = view.notices_by_code(ProjectionNoticeCode::DeclaredPathIsNotLiveFlow).next().unwrap();
            assert_eq!(notice.severity(), ProjectionNoticeSeverity::Caution);
        }
    }

    #[test]
    fn no_declared_path_never_becomes_an_isolation_claim() {
        let view = projection(ProjectionAudience::QualifiedPractitioner, true, false, false);
        assert_eq!(view.loads()[0].status(), ProjectedLoadStatus::NoDeclaredSourcePathListed);
        assert!(view.notices_by_code(ProjectionNoticeCode::NoDeclaredPathIsNotIsolation).next().is_some());
        assert_eq!(view.notices_by_code(ProjectionNoticeCode::NoDeclaredPathIsNotIsolation).count(), 1);
    }

    #[test]
    fn stale_graph_revision_is_marked_in_every_projection() {
        for role in [
            ProjectionAudience::Resident,
            ProjectionAudience::Operator,
            ProjectionAudience::QualifiedPractitioner,
            ProjectionAudience::EmergencyResponder,
        ] {
            let view = projection(role, false, true, false);
            assert_eq!(view.revision_status(), ProjectionRevisionStatus::StaleRevision);
            assert!(view.notices_by_code(ProjectionNoticeCode::GraphRevisionStale).next().is_some());
        }
    }

    #[test]
    fn stale_module_configuration_is_never_filtered_out_by_audience() {
        for role in [
            ProjectionAudience::Resident,
            ProjectionAudience::Operator,
            ProjectionAudience::QualifiedPractitioner,
            ProjectionAudience::EmergencyResponder,
        ] {
            let view = projection(role, true, true, true);
            assert_eq!(view.stale_module_ids(), &["module:inverter".to_owned()]);
            assert!(view.notices_by_code(ProjectionNoticeCode::StaleModuleConfiguration).next().is_some());
        }
    }

    #[test]
    fn design_only_maturity_is_explicit() {
        let view = projection(ProjectionAudience::Resident, true, true, false);
        assert_eq!(view.graph_maturity(), GraphMaturity::DesignOnly);
        assert!(view.notices_by_code(ProjectionNoticeCode::DesignOnlyGraph).next().is_some());
    }

    #[test]
    fn practitioner_and_emergency_views_disclose_limited_path_witness_policy() {
        for role in [ProjectionAudience::QualifiedPractitioner, ProjectionAudience::EmergencyResponder] {
            let view = projection(role, true, true, false);
            assert!(view.notices_by_code(ProjectionNoticeCode::LimitedPathWitnessPolicy).next().is_some());
        }
        let resident = projection(ProjectionAudience::Resident, true, true, false);
        assert!(resident.notices_by_code(ProjectionNoticeCode::LimitedPathWitnessPolicy).next().is_none());
    }

    #[test]
    fn projections_are_deterministic_for_identical_inputs() {
        let assessment = graph(true, true, true).assess_declared_source_paths();
        let first = compile_role_projection(&assessment, ProjectionAudience::EmergencyResponder, "sha256:projection").unwrap();
        let second = compile_role_projection(&assessment, ProjectionAudience::EmergencyResponder, "sha256:projection").unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn empty_projection_digest_is_rejected() {
        let assessment = graph(true, true, false).assess_declared_source_paths();
        assert_eq!(
            compile_role_projection(&assessment, ProjectionAudience::Resident, " "),
            Err(ProjectionError::EmptyField { field: "projection_digest" })
        );
    }

    #[test]
    fn load_order_is_deterministic() {
        let assessment = graph(true, true, false).assess_declared_source_paths();
        let view = compile_role_projection(&assessment, ProjectionAudience::Resident, "sha256:p").unwrap();
        assert_eq!(view.loads().iter().map(ProjectedLoad::load_id).collect::<Vec<_>>(), vec!["load:lighting"]);
    }

    #[test]
    fn no_role_receives_installation_approval_from_projection() {
        for role in [
            ProjectionAudience::Resident,
            ProjectionAudience::Operator,
            ProjectionAudience::QualifiedPractitioner,
            ProjectionAudience::EmergencyResponder,
        ] {
            let view = projection(role, true, true, false);
            assert!(view.notices_by_code(ProjectionNoticeCode::NotInstallationApproval).next().is_some());
        }
    }

    #[test]
    fn graph_evidence_state_is_retained_in_projected_path() {
        let view = projection(ProjectionAudience::QualifiedPractitioner, true, true, false);
        let path = &view.loads()[0].path_witnesses().unwrap()[0];
        assert_eq!(path.node_evidence()[0].state(), GraphEvidenceState::DesignDeclared);
        assert_eq!(path.edge_evidence()[0].state(), GraphEvidenceState::DesignDeclared);
    }
}
