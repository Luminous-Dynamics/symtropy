//! Read-only evaluator for behavior-conditioned construction work packages.
//!
//! Accepted-output, WIP and evidence admissibility remain with their upstream
//! owners. This crate derives benchmark requirements from an admitted snapshot;
//! it never accepts work, issues commands or grants movement authority.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Panel { NorthWest, NorthEast, SouthWest, SouthEast }

impl Panel {
    pub const ALL: [Self; 4] = [Self::NorthWest, Self::NorthEast, Self::SouthWest, Self::SouthEast];
    pub const fn code(self) -> &'static str {
        match self { Self::NorthWest => "NW", Self::NorthEast => "NE", Self::SouthWest => "SW", Self::SouthEast => "SE" }
    }
    fn parse(code: &str) -> Option<Self> {
        match code { "NW" => Some(Self::NorthWest), "NE" => Some(Self::NorthEast), "SW" => Some(Self::SouthWest), "SE" => Some(Self::SouthEast), _ => None }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Seam { North, South, West, East }

impl Seam {
    pub const ALL: [Self; 4] = [Self::North, Self::South, Self::West, Self::East];
    pub const fn code(self) -> &'static str {
        match self { Self::North => "NORTH", Self::South => "SOUTH", Self::West => "WEST", Self::East => "EAST" }
    }
    fn panels(self) -> (Panel, Panel) {
        match self {
            Self::North => (Panel::NorthWest, Panel::NorthEast),
            Self::South => (Panel::SouthWest, Panel::SouthEast),
            Self::West => (Panel::NorthWest, Panel::SouthWest),
            Self::East => (Panel::NorthEast, Panel::SouthEast),
        }
    }
    fn parse(code: &str) -> Option<Self> {
        match code { "NORTH" => Some(Self::North), "SOUTH" => Some(Self::South), "WEST" => Some(Self::West), "EAST" => Some(Self::East), _ => None }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WipDisposition { Partial, Rejected, InProgress, Unknown }

/// This record must already have been admitted by the #3605 owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceptedOutput {
    pub unit_id: String,
    pub package_id: String,
    /// Opaque upstream reference; this crate does not authenticate it.
    pub acceptance_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkInProgress {
    pub unit_id: String,
    pub package_id: String,
    pub disposition: WipDisposition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessState { Available, Blocked, Unknown }

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccessObservation {
    pub target: String,
    pub state: AccessState,
    pub evidence_ref: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MissionSnapshot {
    pub accepted_outputs: Vec<AcceptedOutput>,
    pub work_in_progress: Vec<WorkInProgress>,
    pub access_observations: Vec<AccessObservation>,
    /// Adversarial input only. Self-reported completion is never authoritative.
    pub robot_reported_completed: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkPackageKind {
    InstallAnchorFrame,
    InstallPanel(Panel),
    InspectSeam(Seam),
    VerifyPanelAssembly,
    ReconcileWip,
    ReviewRejectedWork,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockReason { AccessBlocked, RequiredObservationUnknown, WipOutcomeUnknown }

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanDisposition { Ready, Blocked(BlockReason), ReconcileRequired, ReviewRequired, InProgress }

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanItem {
    pub id: String,
    pub kind: WorkPackageKind,
    pub target: String,
    pub source_package_id: Option<String>,
    pub disposition: PlanDisposition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvaluationStatus { Inconsistent, Complete, Actionable, Held }

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Evaluation {
    pub status: EvaluationStatus,
    pub items: Vec<PlanItem>,
    pub error: Option<String>,
}

impl Evaluation {
    pub fn ready_package_ids(&self) -> Vec<String> {
        self.items.iter()
            .filter(|item| item.disposition == PlanDisposition::Ready)
            .map(|item| item.id.clone())
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PackageKey { Anchor, Panel(Panel), Seam(Seam), FinalCheck }

impl PackageKey {
    fn id(self) -> String {
        match self {
            Self::Anchor => "anchor-frame:install".to_owned(),
            Self::Panel(panel) => format!("panel:{}:install", panel.code()),
            Self::Seam(seam) => format!("seam:{}:inspect", seam.code()),
            Self::FinalCheck => "panel-wall:functional-check".to_owned(),
        }
    }
    fn target(self) -> String {
        match self {
            Self::Anchor => "anchor-frame".to_owned(),
            Self::Panel(panel) => format!("panel:{}", panel.code()),
            Self::Seam(seam) => format!("seam:{}", seam.code()),
            Self::FinalCheck => "panel-wall".to_owned(),
        }
    }
    fn kind(self) -> WorkPackageKind {
        match self {
            Self::Anchor => WorkPackageKind::InstallAnchorFrame,
            Self::Panel(panel) => WorkPackageKind::InstallPanel(panel),
            Self::Seam(seam) => WorkPackageKind::InspectSeam(seam),
            Self::FinalCheck => WorkPackageKind::VerifyPanelAssembly,
        }
    }
    fn parse(id: &str) -> Option<Self> {
        if id == "anchor-frame:install" { return Some(Self::Anchor); }
        if id == "panel-wall:functional-check" { return Some(Self::FinalCheck); }
        let parts: Vec<&str> = id.split(':').collect();
        match parts.as_slice() {
            ["panel", code, "install"] => Panel::parse(code).map(Self::Panel),
            ["seam", code, "inspect"] => Seam::parse(code).map(Self::Seam),
            _ => None,
        }
    }
}

fn inconsistent(message: impl Into<String>) -> Evaluation {
    Evaluation { status: EvaluationStatus::Inconsistent, items: Vec::new(), error: Some(message.into()) }
}

struct SnapshotIndex {
    accepted_packages: BTreeSet<String>,
    wip_by_package: BTreeMap<String, WipDisposition>,
    access_by_target: BTreeMap<String, AccessState>,
}

fn validate_snapshot(snapshot: &MissionSnapshot) -> Result<SnapshotIndex, String> {
    let mut accepted_packages = BTreeSet::new();
    let mut accepted_units = BTreeSet::new();

    for output in &snapshot.accepted_outputs {
        if output.unit_id.trim().is_empty() || output.acceptance_ref.trim().is_empty()
            || PackageKey::parse(&output.package_id).is_none() {
            return Err("accepted output has missing identity/reference or unknown package".to_owned());
        }
        if !accepted_units.insert(output.unit_id.clone()) {
            return Err(format!("duplicate accepted unit id: {}", output.unit_id));
        }
        if !accepted_packages.insert(output.package_id.clone()) {
            return Err(format!("multiple accepted outputs claim package: {}", output.package_id));
        }
    }

    let mut wip_by_package = BTreeMap::new();
    let mut wip_units = BTreeSet::new();
    for wip in &snapshot.work_in_progress {
        if wip.unit_id.trim().is_empty() || PackageKey::parse(&wip.package_id).is_none() {
            return Err("WIP record has missing identity or unknown package".to_owned());
        }
        if accepted_units.contains(&wip.unit_id) || !wip_units.insert(wip.unit_id.clone()) {
            return Err(format!("duplicate or cross-domain WIP unit id: {}", wip.unit_id));
        }
        if wip_by_package.insert(wip.package_id.clone(), wip.disposition).is_some() {
            return Err(format!("multiple WIP records claim package: {}", wip.package_id));
        }
        if accepted_packages.contains(&wip.package_id) {
            return Err(format!("package is both accepted and WIP: {}", wip.package_id));
        }
    }

    let allowed_targets = [
        "anchor-frame", "panel:NW", "panel:NE", "panel:SW", "panel:SE",
        "seam:NORTH", "seam:SOUTH", "seam:WEST", "seam:EAST", "panel-wall",
    ];
    let mut access_by_target = BTreeMap::new();
    for observation in &snapshot.access_observations {
        if observation.evidence_ref.trim().is_empty() || !allowed_targets.contains(&observation.target.as_str()) {
            return Err("access observation has unknown target or missing evidence reference".to_owned());
        }
        if access_by_target.insert(observation.target.clone(), observation.state).is_some() {
            return Err(format!("duplicate access observation for {}", observation.target));
        }
    }

    // No accepted or started operation may bypass its accepted prerequisites.
    for package_id in accepted_packages.iter().chain(wip_by_package.keys()) {
        let key = PackageKey::parse(package_id).expect("package IDs were validated above");
        let prerequisites_met = match key {
            PackageKey::Anchor => true,
            PackageKey::Panel(_) => accepted_packages.contains(&PackageKey::Anchor.id()),
            PackageKey::Seam(seam) => {
                let (first, second) = seam.panels();
                accepted_packages.contains(&PackageKey::Panel(first).id())
                    && accepted_packages.contains(&PackageKey::Panel(second).id())
            }
            PackageKey::FinalCheck => {
                Panel::ALL.iter().all(|panel| accepted_packages.contains(&PackageKey::Panel(*panel).id()))
                    && Seam::ALL.iter().all(|seam| accepted_packages.contains(&PackageKey::Seam(*seam).id()))
            }
        };
        if !prerequisites_met {
            return Err(format!("{} exists without accepted prerequisites", package_id));
        }
    }

    Ok(SnapshotIndex {
        accepted_packages,
        wip_by_package,
        access_by_target,
    })
}

fn push_candidate(
    key: PackageKey,
    accepted: &BTreeSet<String>,
    wip: &BTreeMap<String, WipDisposition>,
    access: &BTreeMap<String, AccessState>,
    items: &mut Vec<PlanItem>,
) {
    let package_id = key.id();
    if accepted.contains(&package_id) { return; }

    let target = key.target();
    if let Some(disposition) = wip.get(&package_id) {
        let (id, kind, plan_disposition) = match disposition {
            WipDisposition::Partial => (
                format!("reconcile:{package_id}"), WorkPackageKind::ReconcileWip,
                PlanDisposition::ReconcileRequired,
            ),
            WipDisposition::Rejected => (
                format!("review-rejected:{package_id}"), WorkPackageKind::ReviewRejectedWork,
                PlanDisposition::ReviewRequired,
            ),
            WipDisposition::InProgress => (
                package_id.clone(), key.kind(), PlanDisposition::InProgress,
            ),
            WipDisposition::Unknown => (
                package_id.clone(), key.kind(),
                PlanDisposition::Blocked(BlockReason::WipOutcomeUnknown),
            ),
        };
        items.push(PlanItem { id, kind, target, source_package_id: Some(package_id), disposition: plan_disposition });
        return;
    }

    let disposition = match access.get(&target).copied().unwrap_or(AccessState::Unknown) {
        AccessState::Available => PlanDisposition::Ready,
        AccessState::Blocked => PlanDisposition::Blocked(BlockReason::AccessBlocked),
        AccessState::Unknown => PlanDisposition::Blocked(BlockReason::RequiredObservationUnknown),
    };
    items.push(PlanItem {
        id: package_id, kind: key.kind(), target, source_package_id: None, disposition,
    });
}

/// Derive a deterministic package set from admitted accepted outputs, WIP and
/// scoped access observations. Robot self-reports are intentionally ignored.
pub fn evaluate(snapshot: &MissionSnapshot) -> Evaluation {
    let index = match validate_snapshot(snapshot) {
        Ok(validated) => validated,
        Err(error) => return inconsistent(error),
    };
    let accepted = index.accepted_packages;
    let wip = index.wip_by_package;
    let access = index.access_by_target;

    if accepted.contains(&PackageKey::FinalCheck.id()) {
        return Evaluation { status: EvaluationStatus::Complete, items: Vec::new(), error: None };
    }

    let mut items = Vec::new();
    if !accepted.contains(&PackageKey::Anchor.id()) {
        push_candidate(PackageKey::Anchor, &accepted, &wip, &access, &mut items);
    } else {
        for panel in Panel::ALL {
            push_candidate(PackageKey::Panel(panel), &accepted, &wip, &access, &mut items);
        }
        for seam in Seam::ALL {
            let (first, second) = seam.panels();
            if accepted.contains(&PackageKey::Panel(first).id()) && accepted.contains(&PackageKey::Panel(second).id()) {
                push_candidate(PackageKey::Seam(seam), &accepted, &wip, &access, &mut items);
            }
        }
        let all_panels = Panel::ALL.iter().all(|panel| accepted.contains(&PackageKey::Panel(*panel).id()));
        let all_seams = Seam::ALL.iter().all(|seam| accepted.contains(&PackageKey::Seam(*seam).id()));
        if all_panels && all_seams {
            push_candidate(PackageKey::FinalCheck, &accepted, &wip, &access, &mut items);
        }
    }

    items.sort_by(|left, right| left.id.cmp(&right.id));
    let status = if items.iter().any(|item| item.disposition == PlanDisposition::Ready) {
        EvaluationStatus::Actionable
    } else {
        EvaluationStatus::Held
    };
    Evaluation { status, items, error: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accepted(package_id: String) -> AcceptedOutput {
        AcceptedOutput {
            unit_id: format!("unit:{package_id}"),
            package_id,
            acceptance_ref: "upstream-3605-acceptance-ref".to_owned(),
        }
    }
    fn accept(snapshot: &mut MissionSnapshot, ids: &[String]) {
        snapshot.accepted_outputs.extend(ids.iter().cloned().map(accepted));
    }
    fn ready(snapshot: &MissionSnapshot) -> Vec<String> { evaluate(snapshot).ready_package_ids() }
    fn panel_ids(panels: &[Panel]) -> Vec<String> {
        panels.iter().map(|panel| PackageKey::Panel(*panel).id()).collect()
    }
    fn seam_ids(seams: &[Seam]) -> Vec<String> {
        seams.iter().map(|seam| PackageKey::Seam(*seam).id()).collect()
    }
    fn sorted(mut ids: Vec<String>) -> Vec<String> { ids.sort(); ids }
    fn available_access() -> Vec<AccessObservation> {
        [
            "anchor-frame",
            "panel:NW", "panel:NE", "panel:SW", "panel:SE",
            "seam:NORTH", "seam:SOUTH", "seam:WEST", "seam:EAST",
            "panel-wall",
        ]
        .into_iter()
        .map(|target| AccessObservation {
            target: target.to_owned(),
            state: AccessState::Available,
            evidence_ref: format!("fixture-access:{target}"),
        })
        .collect()
    }

    fn anchored() -> MissionSnapshot {
        let mut snapshot = MissionSnapshot::default();
        snapshot.access_observations = available_access();
        accept(&mut snapshot, &[PackageKey::Anchor.id()]);
        snapshot
    }
    fn fully_installed() -> MissionSnapshot {
        let mut snapshot = anchored();
        accept(&mut snapshot, &panel_ids(&Panel::ALL));
        snapshot
    }
    fn fully_seamed() -> MissionSnapshot {
        let mut snapshot = fully_installed();
        accept(&mut snapshot, &seam_ids(&Seam::ALL));
        snapshot
    }

    #[test]
    fn missing_access_evidence_holds_the_anchor_package() {
        let result = evaluate(&MissionSnapshot::default());
        assert_eq!(result.status, EvaluationStatus::Held);
        assert!(result.ready_package_ids().is_empty());
        assert_eq!(
            result.items[0].disposition,
            PlanDisposition::Blocked(BlockReason::RequiredObservationUnknown)
        );
    }

    #[test]
    fn explicit_available_access_allows_anchor_package() {
        let mut snapshot = MissionSnapshot::default();
        snapshot.access_observations = available_access();
        assert_eq!(ready(&snapshot), vec![PackageKey::Anchor.id()]);
    }

    #[test]
    fn robot_self_report_does_not_satisfy_accepted_output() {
        let mut snapshot = MissionSnapshot::default();
        snapshot.access_observations = available_access();
        snapshot.robot_reported_completed.push(PackageKey::Anchor.id());
        assert_eq!(ready(&snapshot), vec![PackageKey::Anchor.id()]);
    }

    #[test]
    fn accepted_anchor_exposes_available_panels() {
        assert_eq!(ready(&anchored()), sorted(panel_ids(&Panel::ALL)));
    }

    #[test]
    fn north_first_path_creates_north_seam_obligation() {
        let mut snapshot = anchored();
        accept(&mut snapshot, &panel_ids(&[Panel::NorthWest, Panel::NorthEast]));
        assert_eq!(ready(&snapshot), sorted(vec![
            PackageKey::Panel(Panel::SouthEast).id(),
            PackageKey::Panel(Panel::SouthWest).id(),
            PackageKey::Seam(Seam::North).id(),
        ]));
    }

    #[test]
    fn west_first_path_creates_west_seam_obligation() {
        let mut snapshot = anchored();
        accept(&mut snapshot, &panel_ids(&[Panel::NorthWest, Panel::SouthWest]));
        assert_eq!(ready(&snapshot), sorted(vec![
            PackageKey::Panel(Panel::NorthEast).id(),
            PackageKey::Panel(Panel::SouthEast).id(),
            PackageKey::Seam(Seam::West).id(),
        ]));
    }

    #[test]
    fn partial_wip_requires_reconciliation_not_duplicate_issue() {
        let mut snapshot = anchored();
        let id = PackageKey::Panel(Panel::NorthWest).id();
        snapshot.work_in_progress.push(WorkInProgress {
            unit_id: "wip:nw".to_owned(), package_id: id.clone(), disposition: WipDisposition::Partial,
        });
        let result = evaluate(&snapshot);
        assert!(result.items.iter().any(|item| item.id == format!("reconcile:{id}")
            && item.disposition == PlanDisposition::ReconcileRequired));
        assert!(!result.ready_package_ids().contains(&id));
    }

    #[test]
    fn rejected_wip_requires_review() {
        let mut snapshot = anchored();
        let id = PackageKey::Panel(Panel::SouthEast).id();
        snapshot.work_in_progress.push(WorkInProgress {
            unit_id: "wip:se".to_owned(), package_id: id.clone(), disposition: WipDisposition::Rejected,
        });
        let result = evaluate(&snapshot);
        assert!(result.items.iter().any(|item| item.id == format!("review-rejected:{id}")
            && item.disposition == PlanDisposition::ReviewRequired));
        assert!(!result.ready_package_ids().contains(&id));
    }

    #[test]
    fn unknown_anchor_wip_holds_downstream_progress() {
        let mut snapshot = MissionSnapshot::default();
        snapshot.work_in_progress.push(WorkInProgress {
            unit_id: "wip:anchor".to_owned(), package_id: PackageKey::Anchor.id(),
            disposition: WipDisposition::Unknown,
        });
        let result = evaluate(&snapshot);
        assert_eq!(result.status, EvaluationStatus::Held);
        assert!(result.ready_package_ids().is_empty());
        assert_eq!(result.items[0].disposition, PlanDisposition::Blocked(BlockReason::WipOutcomeUnknown));
    }

    #[test]
    fn orphan_seam_acceptance_is_inconsistent() {
        let mut snapshot = MissionSnapshot::default();
        accept(&mut snapshot, &[PackageKey::Seam(Seam::North).id()]);
        assert_eq!(evaluate(&snapshot).status, EvaluationStatus::Inconsistent);
    }

    #[test]
    fn blocked_panel_does_not_block_independent_panel_work() {
        let mut snapshot = anchored();
        snapshot.access_observations.push(AccessObservation {
            target: "panel:NE".to_owned(), state: AccessState::Blocked, evidence_ref: "site-access-ref".to_owned(),
        });
        let result = evaluate(&snapshot);
        assert_eq!(result.ready_package_ids(), sorted(vec![
            PackageKey::Panel(Panel::NorthWest).id(),
            PackageKey::Panel(Panel::SouthWest).id(),
            PackageKey::Panel(Panel::SouthEast).id(),
        ]));
        assert!(result.items.iter().any(|item| item.id == PackageKey::Panel(Panel::NorthEast).id()
            && item.disposition == PlanDisposition::Blocked(BlockReason::AccessBlocked)));
    }

    #[test]
    fn unknown_access_blocks_only_the_affected_branch() {
        let mut snapshot = anchored();
        snapshot.access_observations.push(AccessObservation {
            target: "panel:NE".to_owned(), state: AccessState::Unknown, evidence_ref: "unknown-access-ref".to_owned(),
        });
        let result = evaluate(&snapshot);
        assert_eq!(result.status, EvaluationStatus::Actionable);
        assert_eq!(result.ready_package_ids(), sorted(vec![
            PackageKey::Panel(Panel::NorthWest).id(),
            PackageKey::Panel(Panel::SouthWest).id(),
            PackageKey::Panel(Panel::SouthEast).id(),
        ]));
        assert!(result.items.iter().any(|item| item.disposition
            == PlanDisposition::Blocked(BlockReason::RequiredObservationUnknown)));
    }

    #[test]
    fn three_accepted_seams_leave_only_the_missing_seam() {
        let mut snapshot = fully_installed();
        accept(&mut snapshot, &seam_ids(&[Seam::North, Seam::South, Seam::West]));
        assert_eq!(ready(&snapshot), vec![PackageKey::Seam(Seam::East).id()]);
    }

    #[test]
    fn final_check_requires_four_accepted_seams() {
        let snapshot = fully_seamed();
        assert_eq!(ready(&snapshot), vec![PackageKey::FinalCheck.id()]);
        let mut complete = snapshot;
        accept(&mut complete, &[PackageKey::FinalCheck.id()]);
        let result = evaluate(&complete);
        assert_eq!(result.status, EvaluationStatus::Complete);
        assert!(result.items.is_empty());
    }

    #[test]
    fn duplicate_accepted_package_is_inconsistent() {
        let mut snapshot = anchored();
        snapshot.accepted_outputs.push(AcceptedOutput {
            unit_id: "second-anchor-unit".to_owned(),
            package_id: PackageKey::Anchor.id(),
            acceptance_ref: "second-ref".to_owned(),
        });
        assert_eq!(evaluate(&snapshot).status, EvaluationStatus::Inconsistent);
    }

    #[test]
    fn input_order_does_not_change_the_plan() {
        let mut first = anchored();
        accept(&mut first, &panel_ids(&[Panel::NorthWest, Panel::NorthEast]));
        let mut second = first.clone();
        second.accepted_outputs.reverse();
        assert_eq!(evaluate(&first), evaluate(&second));
    }

    #[test]
    fn in_progress_work_is_not_issued_again() {
        let mut snapshot = anchored();
        let id = PackageKey::Panel(Panel::NorthEast).id();
        snapshot.work_in_progress.push(WorkInProgress {
            unit_id: "wip:ne".to_owned(), package_id: id.clone(), disposition: WipDisposition::InProgress,
        });
        let result = evaluate(&snapshot);
        assert!(!result.ready_package_ids().contains(&id));
        assert!(result.items.iter().any(|item| item.id == id && item.disposition == PlanDisposition::InProgress));
    }

    #[test]
    fn duplicate_access_observation_is_inconsistent() {
        let mut snapshot = anchored();
        for state in [AccessState::Available, AccessState::Blocked] {
            snapshot.access_observations.push(AccessObservation {
                target: "panel:NE".to_owned(), state, evidence_ref: "access-ref".to_owned(),
            });
        }
        assert_eq!(evaluate(&snapshot).status, EvaluationStatus::Inconsistent);
    }

    #[test]
    fn accepted_panel_without_anchor_is_inconsistent() {
        let mut snapshot = MissionSnapshot::default();
        accept(&mut snapshot, &[PackageKey::Panel(Panel::NorthWest).id()]);
        assert_eq!(evaluate(&snapshot).status, EvaluationStatus::Inconsistent);
    }
}
