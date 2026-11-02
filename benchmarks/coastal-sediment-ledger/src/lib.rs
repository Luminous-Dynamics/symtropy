//! Deterministic synthetic-first sediment material accounting.
//!
//! This is a conservation ledger, not a sediment-transport or morphodynamics
//! model. Dry mass and wet volume are intentionally distinct types.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct DryMassGrams(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct WetVolumeMillilitres(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GrainClass { Clay, Silt, Sand, Gravel, Mixed, Unknown }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContaminationStatus { Unknown, ScreenedCompatible, NeedsTreatment, Contaminated }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SedimentLot {
    pub id: String,
    pub grain_class: GrainClass,
    pub contamination: ContaminationStatus,
    pub characterization_evidence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Endpoint { Boundary(String), Store(String) }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transfer {
    pub event_id: String,
    pub lot_id: String,
    pub from: Endpoint,
    pub to: Endpoint,
    pub dry_mass: DryMassGrams,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerError {
    InvalidIdentifier, DuplicateLot, DuplicateEvent, UnknownLot, ZeroMass,
    SameEndpoint, BoundaryToBoundary, InsufficientInventory, ArithmeticOverflow,
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for LedgerError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MassBalance {
    pub external_inflow_g: u64,
    pub external_outflow_g: u64,
    pub internal_stock_g: u64,
    /// Positive means unexplained mass; negative means over-accounted mass.
    pub residual_g: i128,
    pub balanced: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ledger {
    lots: BTreeMap<String, SedimentLot>,
    event_ids: BTreeSet<String>,
    events: Vec<Transfer>,
    inventory_g: BTreeMap<(String, String), u64>,
    external_in_g: BTreeMap<String, u64>,
    external_out_g: BTreeMap<String, u64>,
}

impl Ledger {
    pub fn new() -> Self { Self::default() }

    pub fn register_lot(&mut self, lot: SedimentLot) -> Result<(), LedgerError> {
        if lot.id.trim().is_empty()
            || lot.characterization_evidence.as_ref().is_some_and(|v| v.trim().is_empty())
        { return Err(LedgerError::InvalidIdentifier); }
        if self.lots.contains_key(&lot.id) { return Err(LedgerError::DuplicateLot); }
        self.lots.insert(lot.id.clone(), lot);
        Ok(())
    }

    /// Apply one sequential transfer. Every next value is checked before mutation.
    pub fn record_transfer(&mut self, transfer: Transfer) -> Result<(), LedgerError> {
        if transfer.event_id.trim().is_empty() || transfer.lot_id.trim().is_empty()
            || endpoint_invalid(&transfer.from) || endpoint_invalid(&transfer.to)
        { return Err(LedgerError::InvalidIdentifier); }
        if self.event_ids.contains(&transfer.event_id) { return Err(LedgerError::DuplicateEvent); }
        if transfer.dry_mass.0 == 0 { return Err(LedgerError::ZeroMass); }
        if transfer.from == transfer.to { return Err(LedgerError::SameEndpoint); }
        if matches!(&transfer.from, Endpoint::Boundary(_))
            && matches!(&transfer.to, Endpoint::Boundary(_))
        { return Err(LedgerError::BoundaryToBoundary); }
        if !self.lots.contains_key(&transfer.lot_id) { return Err(LedgerError::UnknownLot); }

        let amount = transfer.dry_mass.0;
        let lot_id = transfer.lot_id.clone();

        let source_next = match &transfer.from {
            Endpoint::Boundary(_) => None,
            Endpoint::Store(store) => {
                let key = (lot_id.clone(), store.clone());
                let available = self.inventory_g.get(&key).copied().unwrap_or(0);
                if available < amount { return Err(LedgerError::InsufficientInventory); }
                Some((key, available - amount))
            }
        };
        let target_next = match &transfer.to {
            Endpoint::Boundary(_) => None,
            Endpoint::Store(store) => {
                let key = (lot_id.clone(), store.clone());
                let current = self.inventory_g.get(&key).copied().unwrap_or(0);
                Some((key, current.checked_add(amount).ok_or(LedgerError::ArithmeticOverflow)?))
            }
        };
        let inflow_next = match &transfer.from {
            Endpoint::Boundary(_) => {
                let current = self.external_in_g.get(&lot_id).copied().unwrap_or(0);
                Some(current.checked_add(amount).ok_or(LedgerError::ArithmeticOverflow)?)
            }
            Endpoint::Store(_) => None,
        };
        let outflow_next = match &transfer.to {
            Endpoint::Boundary(_) => {
                let current = self.external_out_g.get(&lot_id).copied().unwrap_or(0);
                Some(current.checked_add(amount).ok_or(LedgerError::ArithmeticOverflow)?)
            }
            Endpoint::Store(_) => None,
        };

        if let Some((key, value)) = source_next { self.inventory_g.insert(key, value); }
        if let Some((key, value)) = target_next { self.inventory_g.insert(key, value); }
        if let Some(value) = inflow_next { self.external_in_g.insert(lot_id.clone(), value); }
        if let Some(value) = outflow_next { self.external_out_g.insert(lot_id, value); }
        self.event_ids.insert(transfer.event_id.clone());
        self.events.push(transfer);
        Ok(())
    }

    pub fn inventory(&self, lot_id: &str, store_id: &str) -> DryMassGrams {
        DryMassGrams(self.inventory_g.get(&(lot_id.to_owned(), store_id.to_owned())).copied().unwrap_or(0))
    }

    pub fn events(&self) -> &[Transfer] { &self.events }
    pub fn lot(&self, lot_id: &str) -> Option<&SedimentLot> { self.lots.get(lot_id) }

    /// Recompute the global dry-mass conservation equation from materialized state.
    pub fn mass_balance(&self) -> Result<MassBalance, LedgerError> {
        let inflow = checked_sum(self.external_in_g.values().copied())?;
        let outflow = checked_sum(self.external_out_g.values().copied())?;
        let stock = checked_sum(self.inventory_g.values().copied())?;
        let accounted = outflow.checked_add(stock).ok_or(LedgerError::ArithmeticOverflow)?;
        let residual = i128::from(inflow) - i128::from(accounted);
        Ok(MassBalance {
            external_inflow_g: inflow,
            external_outflow_g: outflow,
            internal_stock_g: stock,
            residual_g: residual,
            balanced: residual == 0,
        })
    }

    /// Narrow synthetic placement check. It is not a permit or environmental approval.
    pub fn assess_placement(&self, lot_id: &str, rule: &PlacementRule, evidence: &SuitabilityEvidence)
        -> PlacementDecision
    {
        if rule.id.trim().is_empty() || rule.destination_id.trim().is_empty()
            || rule.permitted_grain_classes.is_empty()
        { return PlacementDecision::ReviewRequired(PlacementReason::InvalidRule); }
        let Some(lot) = self.lots.get(lot_id) else {
            return PlacementDecision::ReviewRequired(PlacementReason::UnknownLot);
        };
        if lot.grain_class == GrainClass::Unknown {
            return PlacementDecision::ReviewRequired(PlacementReason::UnknownGrainClass);
        }
        if evidence.lot_id != lot.id || evidence.evidence_id.trim().is_empty()
            || lot.characterization_evidence.as_deref() != Some(evidence.evidence_id.as_str())
        { return PlacementDecision::ReviewRequired(PlacementReason::MissingOrMismatchedEvidence); }
        if !rule.permitted_grain_classes.contains(&lot.grain_class) {
            return PlacementDecision::Rejected(PlacementReason::GrainClassNotPermitted);
        }
        match (lot.contamination, evidence.state) {
            (ContaminationStatus::ScreenedCompatible, SuitabilityState::Compatible) =>
                PlacementDecision::EligibleForSyntheticRule,
            (ContaminationStatus::Unknown, _) | (_, SuitabilityState::Unknown) =>
                PlacementDecision::ReviewRequired(PlacementReason::UnknownSuitability),
            (ContaminationStatus::NeedsTreatment, _) | (ContaminationStatus::Contaminated, _) =>
                PlacementDecision::Rejected(PlacementReason::CharacterizationNotCompatible),
            (_, SuitabilityState::Incompatible) =>
                PlacementDecision::Rejected(PlacementReason::SuitabilityEvidenceRejectsPlacement),
        }
    }
}

fn endpoint_invalid(value: &Endpoint) -> bool {
    match value { Endpoint::Boundary(id) | Endpoint::Store(id) => id.trim().is_empty() }
}
fn checked_sum(values: impl Iterator<Item = u64>) -> Result<u64, LedgerError> {
    values.try_fold(0_u64, |sum, value| sum.checked_add(value).ok_or(LedgerError::ArithmeticOverflow))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementRule {
    pub id: String,
    pub destination_id: String,
    pub permitted_grain_classes: Vec<GrainClass>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuitabilityState { Unknown, Compatible, Incompatible }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuitabilityEvidence {
    pub lot_id: String,
    pub evidence_id: String,
    pub state: SuitabilityState,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementReason {
    InvalidRule, UnknownLot, UnknownGrainClass, MissingOrMismatchedEvidence,
    UnknownSuitability, CharacterizationNotCompatible, GrainClassNotPermitted,
    SuitabilityEvidenceRejectsPlacement,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementDecision {
    EligibleForSyntheticRule, ReviewRequired(PlacementReason), Rejected(PlacementReason),
}

#[cfg(test)]
mod tests {
    use super::*;
    fn lot(id: &str) -> SedimentLot {
        SedimentLot { id: id.into(), grain_class: GrainClass::Sand,
            contamination: ContaminationStatus::ScreenedCompatible,
            characterization_evidence: Some(format!("lab-{id}")) }
    }
    fn tx(event: &str, lot: &str, from: Endpoint, to: Endpoint, grams: u64) -> Transfer {
        Transfer { event_id: event.into(), lot_id: lot.into(), from, to, dry_mass: DryMassGrams(grams) }
    }
    fn ledger() -> Ledger {
        let mut l = Ledger::new();
        l.register_lot(lot("sand-1")).unwrap();
        l.record_transfer(tx("in-1", "sand-1", Endpoint::Boundary("river".into()),
            Endpoint::Store("estuary".into()), 1_000_000)).unwrap();
        l
    }

    #[test]
    fn closed_transfer_chain_conserves_exact_dry_mass() {
        let mut l = ledger();
        l.record_transfer(tx("dredge", "sand-1", Endpoint::Store("estuary".into()),
            Endpoint::Store("channel-stockpile".into()), 250_000)).unwrap();
        l.record_transfer(tx("place", "sand-1", Endpoint::Store("channel-stockpile".into()),
            Endpoint::Store("beach-cell".into()), 150_000)).unwrap();
        l.record_transfer(tx("dispose", "sand-1", Endpoint::Store("channel-stockpile".into()),
            Endpoint::Boundary("confined-disposal".into()), 50_000)).unwrap();
        let b = l.mass_balance().unwrap();
        assert_eq!((b.external_inflow_g, b.external_outflow_g, b.internal_stock_g), (1_000_000, 50_000, 950_000));
        assert_eq!(b.residual_g, 0);
        assert!(b.balanced);
    }

    #[test]
    fn overdraw_is_rejected_without_partial_mutation() {
        let mut l = ledger();
        let before = l.clone();
        assert_eq!(l.record_transfer(tx("overdraw", "sand-1", Endpoint::Store("estuary".into()),
            Endpoint::Store("stockpile".into()), 1_000_001)), Err(LedgerError::InsufficientInventory));
        assert_eq!(l, before);
    }

    #[test]
    fn duplicate_event_and_duplicate_lot_ids_are_rejected() {
        let mut l = ledger();
        assert_eq!(l.record_transfer(tx("in-1", "sand-1", Endpoint::Boundary("other".into()),
            Endpoint::Store("estuary".into()), 1)), Err(LedgerError::DuplicateEvent));
        assert_eq!(l.register_lot(lot("sand-1")), Err(LedgerError::DuplicateLot));
    }

    #[test]
    fn unknown_lot_zero_mass_and_invalid_endpoints_are_rejected() {
        let mut l = Ledger::new();
        assert_eq!(l.record_transfer(tx("unknown", "x", Endpoint::Boundary("src".into()),
            Endpoint::Store("dst".into()), 1)), Err(LedgerError::UnknownLot));
        l.register_lot(lot("sand-1")).unwrap();
        assert_eq!(l.record_transfer(tx("zero", "sand-1", Endpoint::Boundary("src".into()),
            Endpoint::Store("dst".into()), 0)), Err(LedgerError::ZeroMass));
        assert_eq!(l.record_transfer(tx("blank", "sand-1", Endpoint::Boundary(" ".into()),
            Endpoint::Store("dst".into()), 1)), Err(LedgerError::InvalidIdentifier));
    }

    #[test]
    fn boundary_hops_and_self_transfers_are_rejected() {
        let mut l = ledger();
        assert_eq!(l.record_transfer(tx("hop", "sand-1", Endpoint::Boundary("a".into()),
            Endpoint::Boundary("b".into()), 1)), Err(LedgerError::BoundaryToBoundary));
        assert_eq!(l.record_transfer(tx("self", "sand-1", Endpoint::Store("estuary".into()),
            Endpoint::Store("estuary".into()), 1)), Err(LedgerError::SameEndpoint));
    }

    #[test]
    fn different_lots_remain_separately_tracked() {
        let mut l = ledger();
        let mut clay = lot("clay-1");
        clay.grain_class = GrainClass::Clay;
        l.register_lot(clay).unwrap();
        l.record_transfer(tx("clay-in", "clay-1", Endpoint::Boundary("river".into()),
            Endpoint::Store("estuary".into()), 300)).unwrap();
        l.record_transfer(tx("sand-out", "sand-1", Endpoint::Store("estuary".into()),
            Endpoint::Store("stockpile".into()), 100)).unwrap();
        assert_eq!(l.inventory("sand-1", "estuary"), DryMassGrams(999_900));
        assert_eq!(l.inventory("clay-1", "estuary"), DryMassGrams(300));
        assert_eq!(l.mass_balance().unwrap().residual_g, 0);
    }

    #[test]
    fn overflow_does_not_mutate_inventory() {
        let mut l = Ledger::new();
        l.register_lot(lot("sand-1")).unwrap();
        l.record_transfer(tx("almost-max", "sand-1", Endpoint::Boundary("src".into()),
            Endpoint::Store("stock".into()), u64::MAX)).unwrap();
        let before = l.clone();
        assert_eq!(l.record_transfer(tx("overflow", "sand-1", Endpoint::Boundary("src".into()),
            Endpoint::Store("stock".into()), 1)), Err(LedgerError::ArithmeticOverflow));
        assert_eq!(l, before);
    }

    #[test]
    fn placement_requires_matching_lot_and_evidence() {
        let l = ledger();
        let rule = PlacementRule { id: "beach-rule".into(), destination_id: "beach-1".into(),
            permitted_grain_classes: vec![GrainClass::Sand] };
        let evidence = SuitabilityEvidence { lot_id: "sand-1".into(), evidence_id: "lab-sand-1".into(),
            state: SuitabilityState::Compatible };
        assert_eq!(l.assess_placement("sand-1", &rule, &evidence), PlacementDecision::EligibleForSyntheticRule);
        let wrong = SuitabilityEvidence { lot_id: "other-lot".into(), ..evidence.clone() };
        assert_eq!(l.assess_placement("sand-1", &rule, &wrong),
            PlacementDecision::ReviewRequired(PlacementReason::MissingOrMismatchedEvidence));
    }

    #[test]
    fn unknown_or_contaminated_material_never_passes_placement() {
        let mut l = Ledger::new();
        let mut unknown = lot("unknown");
        unknown.grain_class = GrainClass::Unknown;
        unknown.contamination = ContaminationStatus::Unknown;
        l.register_lot(unknown).unwrap();
        let rule = PlacementRule { id: "r".into(), destination_id: "d".into(),
            permitted_grain_classes: vec![GrainClass::Sand, GrainClass::Mixed] };
        let evidence = SuitabilityEvidence { lot_id: "unknown".into(), evidence_id: "lab-unknown".into(),
            state: SuitabilityState::Compatible };
        assert_eq!(l.assess_placement("unknown", &rule, &evidence),
            PlacementDecision::ReviewRequired(PlacementReason::UnknownGrainClass));
        let mut contaminated = lot("contaminated");
        contaminated.contamination = ContaminationStatus::Contaminated;
        l.register_lot(contaminated).unwrap();
        let contaminated_evidence = SuitabilityEvidence { lot_id: "contaminated".into(),
            evidence_id: "lab-contaminated".into(), state: SuitabilityState::Compatible };
        assert_eq!(l.assess_placement("contaminated", &rule, &contaminated_evidence),
            PlacementDecision::Rejected(PlacementReason::CharacterizationNotCompatible));
    }

    #[test]
    fn wet_volume_has_no_implicit_mass_conversion() {
        let wet = WetVolumeMillilitres(2_000_000);
        let dry = DryMassGrams(500_000);
        assert_ne!(wet.0, dry.0);
        // Deliberately no conversion API: density and solids fraction must be qualified.
    }
}
