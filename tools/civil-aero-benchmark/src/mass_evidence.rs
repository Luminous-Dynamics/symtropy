//! Source-bound mass evidence and immutable aircraft loading-state snapshots.
//!
//! This layer composes with `mass_properties`; it does not replace its arithmetic.
//! It binds each reported loading-state value to explicit evidence, aircraft
//! effectivity, source units and a separately digested state manifest.
//!
//! Important boundaries:
//! - source SHA-256 identifies bytes claimed by the source record; this module
//!   does not fetch, authenticate, or independently verify those source bytes;
//! - `AdmittedForScreening` is a local review state, not regulatory approval;
//! - uncertainty is propagated to total mass only when every installed item
//!   declares a separately justified independence model;
//! - CG and inertia uncertainty propagation is not implemented in v0.1;
//! - every result remains screening-only.

use crate::integrity::sha256_hex;
use crate::mass_properties::{
    MassGeometry, MassItem, MassItemCategory, MassLedger, MassPropertiesReport,
    MassPropertiesStatus,
};
use std::collections::{BTreeMap, HashMap, HashSet};

pub const MASS_EVIDENCE_SCHEMA_VERSION: &str = "aircraft-mass-evidence/0.1.0";
pub const LOADING_STATE_SCHEMA_VERSION: &str = "aircraft-loading-state/0.1.0";
pub const MASS_STATE_REPORT_SCHEMA_VERSION: &str = "aircraft-mass-properties-state-report/0.1.0";
const MASS_MATCH_RELATIVE_TOLERANCE: f64 = 1.0e-9;
const POSITION_MATCH_RELATIVE_TOLERANCE: f64 = 1.0e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MassEvidenceClass {
    DirectlyMeasured,
    ManufacturerReported,
    ApprovedManualOrTypeData,
    DerivedFromMeasuredComponents,
    EngineeringEstimate,
    SyntheticFixture,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceAdmission {
    AdmittedForScreening,
    PendingReview,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UncertaintyRelationship {
    /// Independence has been explicitly justified for this finite model.
    IndependentByDeclaredModel,
    /// Correlation is known to exist or has not been bounded.
    CorrelatedOrUnknown,
    /// No uncertainty assessment was supplied.
    NotAssessed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountingRole {
    /// Item mass is a leaf; its children must not also represent this same mass.
    Leaf,
    /// Item is an aggregate; child IDs identify mass records it already includes.
    Aggregate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MassUnit {
    Kilogram,
    Gram,
    Pound,
}

impl MassUnit {
    pub fn to_kilograms(self, value: f64) -> Result<f64, String> {
        if !value.is_finite() || value <= 0.0 {
            return Err("source mass must be finite and > 0".to_owned());
        }
        let converted = match self {
            Self::Kilogram => value,
            Self::Gram => value / 1_000.0,
            Self::Pound => value * 0.453_592_37,
        };
        if !converted.is_finite() || converted <= 0.0 {
            return Err("source mass conversion produced an invalid kg value".to_owned());
        }
        Ok(converted)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthUnit {
    Metre,
    Centimetre,
    Millimetre,
    Foot,
    Inch,
}

impl LengthUnit {
    pub fn to_metres(self, value: f64) -> Result<f64, String> {
        if !value.is_finite() {
            return Err("source position coordinate must be finite".to_owned());
        }
        let converted = match self {
            Self::Metre => value,
            Self::Centimetre => value / 100.0,
            Self::Millimetre => value / 1_000.0,
            Self::Foot => value * 0.3048,
            Self::Inch => value * 0.0254,
        };
        if !converted.is_finite() {
            return Err("source position conversion produced a non-finite metre value".to_owned());
        }
        Ok(converted)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceMassValue {
    pub value: f64,
    pub unit: MassUnit,
}

impl SourceMassValue {
    pub fn kilograms(self) -> Result<f64, String> {
        self.unit.to_kilograms(self.value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourcePosition {
    pub values: [f64; 3],
    pub unit: LengthUnit,
}

impl SourcePosition {
    pub fn metres(self) -> Result<[f64; 3], String> {
        Ok([
            self.unit.to_metres(self.values[0])?,
            self.unit.to_metres(self.values[1])?,
            self.unit.to_metres(self.values[2])?,
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRecord {
    /// Stable document URI, artifact reference, or controlled source identifier.
    pub reference: String,
    /// Exact document/model/data revision; do not use a mutable label such as "latest".
    pub revision: String,
    /// Section, table, row, instrument record, or other locator within the source.
    pub locator: String,
    /// SHA-256 asserted for the exact source bytes. This crate does not fetch those bytes.
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AircraftEffectivity {
    pub aircraft_family: String,
    pub aircraft_model: String,
    pub configuration_sha256: String,
    /// None means all serials of the declared family/model/configuration.
    /// Some(values) is an explicit allow-list.
    pub serial_numbers: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MassItemEvidence {
    pub item: MassItem,
    pub parent_id: Option<String>,
    pub accounting_role: AccountingRole,
    /// For an Aggregate record, these are included in its mass total.
    /// The aggregate and any of these children must not both be installed.
    pub included_child_ids: Vec<String>,
    pub evidence_class: MassEvidenceClass,
    pub admission: EvidenceAdmission,
    pub source: SourceRecord,
    pub effectivity: AircraftEffectivity,
    /// Original source mass and unit, retained alongside canonical kg.
    pub source_mass: Option<SourceMassValue>,
    /// Original source position and unit for point/rigid-body CG location.
    pub source_position: Option<SourcePosition>,
    pub standard_uncertainty_kg: Option<f64>,
    pub uncertainty_relationship: UncertaintyRelationship,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MassEvidenceLedger {
    pub schema_version: &'static str,
    pub configuration_sha256: String,
    pub aircraft_family: String,
    pub aircraft_model: String,
    pub serial_number: String,
    /// Records must be strictly sorted by item ID for canonical generation identity.
    pub records: Vec<MassItemEvidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadingStateKind {
    Dispatch,
    Takeoff,
    IntermediateFlight,
    Landing,
    Diversion,
    GoAround,
    DeclaredFailure,
    Maintenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallationState {
    Installed,
    Removed,
    Unknown,
}

/// Immutable item state. Installed items carry their full state mass and geometry;
/// source values and units are retained to verify the conversion into SI.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadingItemState {
    pub item_id: String,
    pub installation: InstallationState,
    pub mass_kg: Option<f64>,
    pub source_mass: Option<SourceMassValue>,
    pub geometry: MassGeometry,
    pub source_position: Option<SourcePosition>,
    /// Evidence backing the values in this particular state, e.g. fuel quantity,
    /// occupant loading, or a configuration-specific equipment record.
    pub value_source: Option<SourceRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AircraftLoadingState {
    pub schema_version: &'static str,
    pub state_id: String,
    pub state_kind: LoadingStateKind,
    pub configuration_sha256: String,
    pub source_generation_sha256: String,
    pub aircraft_family: String,
    pub aircraft_model: String,
    pub serial_number: String,
    /// Must contain every evidence record, exactly once, sorted by item ID.
    /// Removed items remain explicit; they are not silently omitted.
    pub items: Vec<LoadingItemState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateEvidenceStatus {
    CompleteForScreening,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TotalMassUncertaintyStatus {
    RssUnderDeclaredIndependence,
    CorrelationOrUncertaintyNotAssessed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MassPropertiesStateReport {
    pub schema_version: &'static str,
    pub evidence_level: &'static str,
    pub state_id: String,
    pub state_kind: LoadingStateKind,
    pub configuration_sha256: String,
    pub source_generation_sha256: String,
    pub state_sha256: String,
    pub status: StateEvidenceStatus,
    pub mass_properties: MassPropertiesReport,
    pub total_mass_standard_uncertainty_kg: Option<f64>,
    pub total_mass_uncertainty_status: TotalMassUncertaintyStatus,
    /// Deliberately absent in v0.1; no uncertainty propagation for CG/inertia is claimed.
    pub cg_uncertainty_m: Option<[f64; 3]>,
    pub inertia_uncertainty_kg_m2: Option<[f64; 6]>,
    pub pending_item_ids: Vec<String>,
    pub rejected_item_ids: Vec<String>,
    pub unresolved_installation_item_ids: Vec<String>,
    pub unknown_mass_item_ids: Vec<String>,
    pub unknown_geometry_item_ids: Vec<String>,
}

impl MassEvidenceLedger {
    pub fn source_generation_sha256(&self) -> Result<String, String> {
        self.validate_identity()?;
        let mut bytes = Vec::new();
        push_str(&mut bytes, MASS_EVIDENCE_SCHEMA_VERSION);
        push_str(&mut bytes, &self.configuration_sha256);
        push_str(&mut bytes, &self.aircraft_family);
        push_str(&mut bytes, &self.aircraft_model);
        push_str(&mut bytes, &self.serial_number);
        push_u64(&mut bytes, self.records.len() as u64);
        for record in &self.records {
            encode_evidence_record(&mut bytes, record);
        }
        sha256_hex(&bytes)
    }

    pub fn evaluate_state(
        &self,
        state: &AircraftLoadingState,
        expected_configuration_sha256: &str,
    ) -> Result<MassPropertiesStateReport, String> {
        self.validate_identity()?;
        validate_sha256(expected_configuration_sha256, "expected_configuration_sha256")?;
        if self.configuration_sha256 != expected_configuration_sha256 {
            return Err("stale mass evidence ledger: aircraft configuration mismatch".to_owned());
        }
        if state.schema_version != LOADING_STATE_SCHEMA_VERSION {
            return Err("unsupported aircraft loading-state schema version".to_owned());
        }
        if state.configuration_sha256 != expected_configuration_sha256 {
            return Err("stale loading state: aircraft configuration mismatch".to_owned());
        }
        if state.aircraft_family != self.aircraft_family
            || state.aircraft_model != self.aircraft_model
            || state.serial_number != self.serial_number
        {
            return Err("loading state aircraft identity/effectivity mismatch".to_owned());
        }

        let generation = self.source_generation_sha256()?;
        if state.source_generation_sha256 != generation {
            return Err("stale source generation: loading state references another evidence generation".to_owned());
        }
        validate_state_order_and_completeness(&self.records, &state.items)?;

        let evidence_by_id: HashMap<&str, &MassItemEvidence> = self
            .records
            .iter()
            .map(|record| (record.item.id.as_str(), record))
            .collect();

        let mut pending_item_ids = Vec::new();
        let mut rejected_item_ids = Vec::new();
        let mut unresolved_installation_item_ids = Vec::new();
        let mut mass_items = Vec::new();
        let mut installed_ids = HashSet::new();
        let mut uncertainty_variances = Vec::new();
        let mut all_uncertainties_independent = true;

        for state_item in &state.items {
            let evidence = evidence_by_id
                .get(state_item.item_id.as_str())
                .ok_or_else(|| format!("missing mass evidence record: {}", state_item.item_id))?;
            validate_evidence_record(evidence, self)?;

            if evidence.admission == EvidenceAdmission::PendingReview {
                pending_item_ids.push(state_item.item_id.clone());
            }
            if evidence.admission == EvidenceAdmission::Rejected {
                rejected_item_ids.push(state_item.item_id.clone());
            }
            if evidence.evidence_class == MassEvidenceClass::Unknown {
                pending_item_ids.push(state_item.item_id.clone());
            }

            match state_item.installation {
                InstallationState::Removed => {
                    if state_item.mass_kg.is_some()
                        || state_item.source_mass.is_some()
                        || !matches!(state_item.geometry, MassGeometry::Unknown)
                        || state_item.source_position.is_some()
                        || state_item.value_source.is_some()
                    {
                        return Err(format!(
                            "removed item {} must not carry installed mass/geometry values",
                            state_item.item_id
                        ));
                    }
                }
                InstallationState::Unknown => {
                    unresolved_installation_item_ids.push(state_item.item_id.clone());
                    if state_item.mass_kg.is_some()
                        || state_item.source_mass.is_some()
                        || !matches!(state_item.geometry, MassGeometry::Unknown)
                        || state_item.source_position.is_some()
                        || state_item.value_source.is_some()
                    {
                        return Err(format!(
                            "unknown-installation item {} must remain unresolved",
                            state_item.item_id
                        ));
                    }
                    mass_items.push(MassItem {
                        id: state_item.item_id.clone(),
                        category: evidence.item.category,
                        mass_kg: None,
                        geometry: MassGeometry::Unknown,
                    });
                }
                InstallationState::Installed => {
                    installed_ids.insert(state_item.item_id.as_str());
                    if let Some(source) = &state_item.value_source {
                        validate_source_record(source, &state_item.item_id)?;
                    } else {
                        pending_item_ids.push(state_item.item_id.clone());
                    }

                    if let Some(mass_kg) = state_item.mass_kg {
                        if !mass_kg.is_finite() || mass_kg <= 0.0 {
                            return Err(format!(
                                "loading item {}: mass_kg must be finite and > 0",
                                state_item.item_id
                            ));
                        }
                        let source_mass = state_item.source_mass.ok_or_else(|| {
                            format!("loading item {}: known mass requires source value and unit", state_item.item_id)
                        })?;
                        let converted = source_mass.kilograms()?;
                        if !close_enough(mass_kg, converted, MASS_MATCH_RELATIVE_TOLERANCE) {
                            return Err(format!(
                                "loading item {}: source mass conversion does not match canonical kg",
                                state_item.item_id
                            ));
                        }
                        if let Some(uncertainty) = evidence.standard_uncertainty_kg {
                            if !uncertainty.is_finite() || uncertainty < 0.0 {
                                return Err(format!(
                                    "mass item {}: standard uncertainty must be finite and >= 0",
                                    state_item.item_id
                                ));
                            }
                            if evidence.uncertainty_relationship
                                == UncertaintyRelationship::IndependentByDeclaredModel
                            {
                                uncertainty_variances.push(uncertainty * uncertainty);
                            } else {
                                all_uncertainties_independent = false;
                            }
                        } else {
                            all_uncertainties_independent = false;
                        }
                    } else if state_item.source_mass.is_some() {
                        return Err(format!(
                            "loading item {}: source mass present while canonical mass is unknown",
                            state_item.item_id
                        ));
                    } else {
                        all_uncertainties_independent = false;
                    }

                    validate_state_geometry(state_item)?;
                    mass_items.push(MassItem {
                        id: state_item.item_id.clone(),
                        category: evidence.item.category,
                        mass_kg: state_item.mass_kg,
                        geometry: state_item.geometry.clone(),
                    });
                }
            }
        }

        validate_aggregate_double_counting(&self.records, &installed_ids)?;

        let mass_ledger = MassLedger {
            configuration_sha256: expected_configuration_sha256.to_owned(),
            items: mass_items,
        };
        let mass_properties = mass_ledger.evaluate()?;
        let state_sha256 = self.state_sha256(state)?;

        pending_item_ids.sort();
        pending_item_ids.dedup();
        rejected_item_ids.sort();
        rejected_item_ids.dedup();
        unresolved_installation_item_ids.sort();

        let complete_evidence = pending_item_ids.is_empty()
            && rejected_item_ids.is_empty()
            && unresolved_installation_item_ids.is_empty()
            && mass_properties.status == MassPropertiesStatus::Complete;

        let total_mass_standard_uncertainty_kg =
            if complete_evidence && all_uncertainties_independent {
                Some(uncertainty_variances.iter().sum::<f64>().sqrt())
            } else {
                None
            };

        Ok(MassPropertiesStateReport {
            schema_version: MASS_STATE_REPORT_SCHEMA_VERSION,
            evidence_level: "screening_only",
            state_id: state.state_id.clone(),
            state_kind: state.state_kind,
            configuration_sha256: expected_configuration_sha256.to_owned(),
            source_generation_sha256: generation,
            state_sha256,
            status: if complete_evidence {
                StateEvidenceStatus::CompleteForScreening
            } else {
                StateEvidenceStatus::Indeterminate
            },
            mass_properties,
            total_mass_standard_uncertainty_kg,
            total_mass_uncertainty_status: if total_mass_standard_uncertainty_kg.is_some() {
                TotalMassUncertaintyStatus::RssUnderDeclaredIndependence
            } else {
                TotalMassUncertaintyStatus::CorrelationOrUncertaintyNotAssessed
            },
            cg_uncertainty_m: None,
            inertia_uncertainty_kg_m2: None,
            pending_item_ids,
            rejected_item_ids,
            unresolved_installation_item_ids,
            unknown_mass_item_ids: mass_ledger_unknown_mass(&mass_properties),
            unknown_geometry_item_ids: mass_properties.unknown_geometry_item_ids.clone(),
        })
    }

    pub fn state_sha256(&self, state: &AircraftLoadingState) -> Result<String, String> {
        self.validate_identity()?;
        if state.configuration_sha256 != self.configuration_sha256 {
            return Err("state digest configuration does not match evidence ledger".to_owned());
        }
        let expected_generation = self.source_generation_sha256()?;
        if state.source_generation_sha256 != expected_generation {
            return Err("state digest source generation does not match evidence ledger".to_owned());
        }
        let mut bytes = Vec::new();
        push_str(&mut bytes, LOADING_STATE_SCHEMA_VERSION);
        push_str(&mut bytes, &state.state_id);
        encode_state_kind(&mut bytes, state.state_kind);
        push_str(&mut bytes, &state.configuration_sha256);
        push_str(&mut bytes, &state.source_generation_sha256);
        push_str(&mut bytes, &state.aircraft_family);
        push_str(&mut bytes, &state.aircraft_model);
        push_str(&mut bytes, &state.serial_number);
        push_u64(&mut bytes, state.items.len() as u64);
        for item in &state.items {
            encode_loading_item(&mut bytes, item);
        }
        sha256_hex(&bytes)
    }

    fn validate_identity(&self) -> Result<(), String> {
        if self.schema_version != MASS_EVIDENCE_SCHEMA_VERSION {
            return Err("unsupported mass-evidence schema version".to_owned());
        }
        validate_sha256(&self.configuration_sha256, "mass_evidence.configuration_sha256")?;
        validate_nonempty(&self.aircraft_family, "aircraft_family")?;
        validate_nonempty(&self.aircraft_model, "aircraft_model")?;
        validate_nonempty(&self.serial_number, "serial_number")?;
        if self.records.is_empty() {
            return Err("mass evidence ledger must contain at least one record".to_owned());
        }
        let mut previous: Option<&str> = None;
        for record in &self.records {
            if let Some(prev) = previous {
                if prev >= record.item.id.as_str() {
                    return Err("mass evidence records must be strictly sorted by item ID".to_owned());
                }
            }
            previous = Some(&record.item.id);
        }
        Ok(())
    }
}

fn validate_evidence_record(
    evidence: &MassItemEvidence,
    ledger: &MassEvidenceLedger,
) -> Result<(), String> {
    if evidence.item.id.trim().is_empty() {
        return Err("mass evidence item ID must not be empty".to_owned());
    }
    validate_source_record(&evidence.source, &evidence.item.id)?;
    validate_nonempty(&evidence.effectivity.aircraft_family, "effectivity.aircraft_family")?;
    validate_nonempty(&evidence.effectivity.aircraft_model, "effectivity.aircraft_model")?;
    validate_sha256(
        &evidence.effectivity.configuration_sha256,
        "effectivity.configuration_sha256",
    )?;
    if evidence.effectivity.aircraft_family != ledger.aircraft_family
        || evidence.effectivity.aircraft_model != ledger.aircraft_model
        || evidence.effectivity.configuration_sha256 != ledger.configuration_sha256
    {
        return Err(format!(
            "mass item {}: aircraft effectivity does not match evidence ledger",
            evidence.item.id
        ));
    }
    if let Some(serials) = &evidence.effectivity.serial_numbers {
        if serials.is_empty() || !serials.iter().any(|serial| serial == &ledger.serial_number) {
            return Err(format!(
                "mass item {}: serial-number effectivity mismatch",
                evidence.item.id
            ));
        }
        if serials.iter().any(|serial| serial.trim().is_empty()) {
            return Err(format!(
                "mass item {}: serial-number effectivity contains an empty value",
                evidence.item.id
            ));
        }
    }
    if let Some(parent_id) = &evidence.parent_id {
        validate_nonempty(parent_id, "mass_item.parent_id")?;
        if parent_id == &evidence.item.id {
            return Err(format!("mass item {} cannot parent itself", evidence.item.id));
        }
    }
    let mut child_ids = HashSet::new();
    for child in &evidence.included_child_ids {
        validate_nonempty(child, "mass_item.included_child_id")?;
        if child == &evidence.item.id || !child_ids.insert(child.as_str()) {
            return Err(format!(
                "mass item {} has a self/duplicate included child",
                evidence.item.id
            ));
        }
    }
    match evidence.accounting_role {
        AccountingRole::Leaf if !evidence.included_child_ids.is_empty() => {
            return Err(format!(
                "leaf mass item {} must not declare included child masses",
                evidence.item.id
            ));
        }
        AccountingRole::Aggregate if evidence.included_child_ids.is_empty() => {
            return Err(format!(
                "aggregate mass item {} must declare its included child IDs",
                evidence.item.id
            ));
        }
        _ => {}
    }
    if let Some(uncertainty) = evidence.standard_uncertainty_kg {
        if !uncertainty.is_finite() || uncertainty < 0.0 {
            return Err(format!(
                "mass item {}: standard uncertainty must be finite and >= 0",
                evidence.item.id
            ));
        }
    }
    if evidence.item.mass_kg.is_some() != evidence.source_mass.is_some() {
        return Err(format!(
            "mass item {}: base source-mass presence must match canonical mass presence",
            evidence.item.id
        ));
    }
    if let (Some(canonical), Some(source)) = (evidence.item.mass_kg, evidence.source_mass) {
        if !close_enough(canonical, source.kilograms()?, MASS_MATCH_RELATIVE_TOLERANCE) {
            return Err(format!(
                "mass item {}: base source mass does not convert to canonical kg",
                evidence.item.id
            ));
        }
    }
    match &evidence.item.geometry {
        MassGeometry::PointMass { position_m } => {
            let source = evidence.source_position.ok_or_else(|| {
                format!("mass item {}: base point mass requires source position", evidence.item.id)
            })?;
            validate_position_match(*position_m, source, &evidence.item.id)?;
        }
        MassGeometry::RigidBody { center_of_mass_m, .. } => {
            let source = evidence.source_position.ok_or_else(|| {
                format!("mass item {}: rigid body requires source centre-of-mass position", evidence.item.id)
            })?;
            validate_position_match(*center_of_mass_m, source, &evidence.item.id)?;
        }
        MassGeometry::Unknown => {
            if evidence.source_position.is_some() {
                return Err(format!(
                    "mass item {}: unknown base geometry must not claim a source position",
                    evidence.item.id
                ));
            }
        }
    }
    Ok(())
}

fn validate_state_order_and_completeness(
    evidence: &[MassItemEvidence],
    items: &[LoadingItemState],
) -> Result<(), String> {
    if evidence.len() != items.len() {
        return Err("loading-state manifest must explicitly cover every mass evidence record".to_owned());
    }
    let mut previous: Option<&str> = None;
    for (record, item) in evidence.iter().zip(items) {
        if item.item_id != record.item.id {
            return Err(format!(
                "loading-state items must be sorted and complete; expected {}, got {}",
                record.item.id, item.item_id
            ));
        }
        if let Some(prev) = previous {
            if prev >= item.item_id.as_str() {
                return Err("loading-state item IDs must be strictly sorted".to_owned());
            }
        }
        previous = Some(&item.item_id);
    }
    Ok(())
}

fn validate_state_geometry(item: &LoadingItemState) -> Result<(), String> {
    match &item.geometry {
        MassGeometry::PointMass { position_m } => {
            let source = item.source_position.ok_or_else(|| {
                format!("loading item {}: point geometry requires source position", item.item_id)
            })?;
            validate_position_match(*position_m, source, &item.item_id)?;
        }
        MassGeometry::RigidBody { center_of_mass_m, .. } => {
            let source = item.source_position.ok_or_else(|| {
                format!("loading item {}: rigid-body geometry requires source position", item.item_id)
            })?;
            validate_position_match(*center_of_mass_m, source, &item.item_id)?;
        }
        MassGeometry::Unknown => {
            if item.source_position.is_some() {
                return Err(format!(
                    "loading item {}: unknown geometry must not have source position",
                    item.item_id
                ));
            }
        }
    }
    Ok(())
}

fn validate_position_match(
    canonical_m: [f64; 3],
    source: SourcePosition,
    item_id: &str,
) -> Result<(), String> {
    let converted = source.metres()?;
    for axis in 0..3 {
        if !close_enough(canonical_m[axis], converted[axis], POSITION_MATCH_RELATIVE_TOLERANCE) {
            return Err(format!(
                "mass item {item_id}: source position unit conversion does not match canonical metres"
            ));
        }
    }
    Ok(())
}

fn validate_aggregate_double_counting(
    records: &[MassItemEvidence],
    installed_ids: &HashSet<&str>,
) -> Result<(), String> {
    let evidence_by_id: HashMap<&str, &MassItemEvidence> = records
        .iter()
        .map(|record| (record.item.id.as_str(), record))
        .collect();
    for record in records {
        if installed_ids.contains(record.item.id.as_str())
            && record.accounting_role == AccountingRole::Aggregate
        {
            for child in &record.included_child_ids {
                if installed_ids.contains(child.as_str()) {
                    return Err(format!(
                        "aggregate/child double counting: {} includes installed child {}",
                        record.item.id, child
                    ));
                }
                if !evidence_by_id.contains_key(child.as_str()) {
                    return Err(format!(
                        "aggregate {} references missing child evidence {}",
                        record.item.id, child
                    ));
                }
            }
        }
    }
    Ok(())
}

fn mass_ledger_unknown_mass(report: &MassPropertiesReport) -> Vec<String> {
    report.unknown_mass_item_ids.clone()
}

fn validate_source_record(source: &SourceRecord, item_id: &str) -> Result<(), String> {
    validate_nonempty(&source.reference, "source.reference")?;
    validate_nonempty(&source.revision, "source.revision")?;
    validate_nonempty(&source.locator, "source.locator")?;
    if source.revision.eq_ignore_ascii_case("latest") {
        return Err(format!(
            "mass item {item_id}: source revision must be immutable, not 'latest'"
        ));
    }
    validate_sha256(&source.sha256, &format!("mass item {item_id} source.sha256"))
}

fn validate_sha256(value: &str, field: &str) -> Result<(), String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("{field} must be 64 hexadecimal characters"));
    }
    Ok(())
}

fn validate_nonempty(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    Ok(())
}

fn close_enough(left: f64, right: f64, relative_tolerance: f64) -> bool {
    left.is_finite()
        && right.is_finite()
        && (left - right).abs() <= relative_tolerance * left.abs().max(right.abs()).max(1.0)
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn push_str(bytes: &mut Vec<u8>, value: &str) {
    push_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn push_f64(bytes: &mut Vec<u8>, value: f64) {
    bytes.extend_from_slice(&value.to_bits().to_be_bytes());
}

fn push_option_f64(bytes: &mut Vec<u8>, value: Option<f64>) {
    match value {
        Some(value) => {
            bytes.push(1);
            push_f64(bytes, value);
        }
        None => bytes.push(0),
    }
}

fn encode_category(bytes: &mut Vec<u8>, value: MassItemCategory) {
    bytes.push(match value {
        MassItemCategory::PrimaryStructure => 0,
        MassItemCategory::SecondaryStructure => 1,
        MassItemCategory::Propulsion => 2,
        MassItemCategory::EnergyStorage => 3,
        MassItemCategory::ElectricalDistribution => 4,
        MassItemCategory::ThermalManagement => 5,
        MassItemCategory::FlightControls => 6,
        MassItemCategory::Avionics => 7,
        MassItemCategory::LandingGear => 8,
        MassItemCategory::InteriorAndFurnishings => 9,
        MassItemCategory::SafetyEquipment => 10,
        MassItemCategory::FuelAndFluids => 11,
        MassItemCategory::Payload => 12,
        MassItemCategory::Other => 13,
    });
}

fn encode_geometry(bytes: &mut Vec<u8>, geometry: &MassGeometry) {
    match geometry {
        MassGeometry::PointMass { position_m } => {
            bytes.push(0);
            for value in position_m {
                push_f64(bytes, *value);
            }
        }
        MassGeometry::RigidBody {
            center_of_mass_m,
            inertia_tensor_about_center_kg_m2,
        } => {
            bytes.push(1);
            for value in center_of_mass_m {
                push_f64(bytes, *value);
            }
            for value in inertia_tensor_about_center_kg_m2 {
                push_f64(bytes, *value);
            }
        }
        MassGeometry::Unknown => bytes.push(2),
    }
}

fn encode_source_record(bytes: &mut Vec<u8>, source: &SourceRecord) {
    push_str(bytes, &source.reference);
    push_str(bytes, &source.revision);
    push_str(bytes, &source.locator);
    push_str(bytes, &source.sha256.to_ascii_lowercase());
}

fn encode_mass_value(bytes: &mut Vec<u8>, value: SourceMassValue) {
    push_f64(bytes, value.value);
    bytes.push(match value.unit {
        MassUnit::Kilogram => 0,
        MassUnit::Gram => 1,
        MassUnit::Pound => 2,
    });
}

fn encode_position(bytes: &mut Vec<u8>, value: SourcePosition) {
    for coordinate in value.values {
        push_f64(bytes, coordinate);
    }
    bytes.push(match value.unit {
        LengthUnit::Metre => 0,
        LengthUnit::Centimetre => 1,
        LengthUnit::Millimetre => 2,
        LengthUnit::Foot => 3,
        LengthUnit::Inch => 4,
    });
}

fn encode_evidence_record(bytes: &mut Vec<u8>, record: &MassItemEvidence) {
    push_str(bytes, &record.item.id);
    encode_category(bytes, record.item.category);
    push_option_f64(bytes, record.item.mass_kg);
    encode_geometry(bytes, &record.item.geometry);
    match &record.parent_id {
        Some(parent) => {
            bytes.push(1);
            push_str(bytes, parent);
        }
        None => bytes.push(0),
    }
    bytes.push(match record.accounting_role {
        AccountingRole::Leaf => 0,
        AccountingRole::Aggregate => 1,
    });
    let mut children = record.included_child_ids.clone();
    children.sort();
    push_u64(bytes, children.len() as u64);
    for child in children {
        push_str(bytes, &child);
    }
    bytes.push(record.evidence_class as u8);
    bytes.push(record.admission as u8);
    encode_source_record(bytes, &record.source);
    push_str(bytes, &record.effectivity.aircraft_family);
    push_str(bytes, &record.effectivity.aircraft_model);
    push_str(bytes, &record.effectivity.configuration_sha256.to_ascii_lowercase());
    match &record.effectivity.serial_numbers {
        Some(serials) => {
            bytes.push(1);
            let mut serials = serials.clone();
            serials.sort();
            push_u64(bytes, serials.len() as u64);
            for serial in serials {
                push_str(bytes, &serial);
            }
        }
        None => bytes.push(0),
    }
    match record.source_mass {
        Some(value) => {
            bytes.push(1);
            encode_mass_value(bytes, value);
        }
        None => bytes.push(0),
    }
    match record.source_position {
        Some(value) => {
            bytes.push(1);
            encode_position(bytes, value);
        }
        None => bytes.push(0),
    }
    push_option_f64(bytes, record.standard_uncertainty_kg);
    bytes.push(record.uncertainty_relationship as u8);
}

fn encode_state_kind(bytes: &mut Vec<u8>, kind: LoadingStateKind) {
    bytes.push(match kind {
        LoadingStateKind::Dispatch => 0,
        LoadingStateKind::Takeoff => 1,
        LoadingStateKind::IntermediateFlight => 2,
        LoadingStateKind::Landing => 3,
        LoadingStateKind::Diversion => 4,
        LoadingStateKind::GoAround => 5,
        LoadingStateKind::DeclaredFailure => 6,
        LoadingStateKind::Maintenance => 7,
    });
}

fn encode_loading_item(bytes: &mut Vec<u8>, item: &LoadingItemState) {
    push_str(bytes, &item.item_id);
    bytes.push(match item.installation {
        InstallationState::Installed => 0,
        InstallationState::Removed => 1,
        InstallationState::Unknown => 2,
    });
    push_option_f64(bytes, item.mass_kg);
    match item.source_mass {
        Some(value) => {
            bytes.push(1);
            encode_mass_value(bytes, value);
        }
        None => bytes.push(0),
    }
    encode_geometry(bytes, &item.geometry);
    match item.source_position {
        Some(value) => {
            bytes.push(1);
            encode_position(bytes, value);
        }
        None => bytes.push(0),
    }
    match &item.value_source {
        Some(source) => {
            bytes.push(1);
            encode_source_record(bytes, source);
        }
        None => bytes.push(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const CONFIG_REMOVED_MODULE: &str =
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const SOURCE_A: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const SOURCE_B: &str = "2222222222222222222222222222222222222222222222222222222222222222";

    fn source(reference: &str, sha: &str) -> SourceRecord {
        SourceRecord {
            reference: reference.to_owned(),
            revision: "rev-1".to_owned(),
            locator: "table-1/row-1".to_owned(),
            sha256: sha.to_owned(),
        }
    }

    fn effectivity(config: &str) -> AircraftEffectivity {
        AircraftEffectivity {
            aircraft_family: "Wren 2".to_owned(),
            aircraft_model: "Wren 2 research fixture".to_owned(),
            configuration_sha256: config.to_owned(),
            serial_numbers: Some(vec!["SYNTH-001".to_owned()]),
        }
    }

    fn evidence_record(
        id: &str,
        category: MassItemCategory,
        mass_kg: Option<f64>,
        position_m: Option<[f64; 3]>,
        config: &str,
    ) -> MassItemEvidence {
        let geometry = position_m
            .map(|position_m| MassGeometry::PointMass { position_m })
            .unwrap_or(MassGeometry::Unknown);
        let source_mass = mass_kg.map(|value| SourceMassValue {
            value,
            unit: MassUnit::Kilogram,
        });
        let source_position = position_m.map(|values| SourcePosition {
            values,
            unit: LengthUnit::Metre,
        });
        MassItemEvidence {
            item: MassItem {
                id: id.to_owned(),
                category,
                mass_kg,
                geometry,
            },
            parent_id: None,
            accounting_role: AccountingRole::Leaf,
            included_child_ids: Vec::new(),
            evidence_class: MassEvidenceClass::SyntheticFixture,
            admission: EvidenceAdmission::AdmittedForScreening,
            source: source(&format!("fixture:{id}"), SOURCE_A),
            effectivity: effectivity(config),
            source_mass,
            source_position,
            standard_uncertainty_kg: Some(0.1),
            uncertainty_relationship: UncertaintyRelationship::IndependentByDeclaredModel,
        }
    }

    fn fixture_ledger(config: &str) -> MassEvidenceLedger {
        let mut records = vec![
            evidence_record(
                "airframe",
                MassItemCategory::PrimaryStructure,
                Some(500.0),
                Some([1.0, 0.0, 0.0]),
                config,
            ),
            evidence_record(
                "fuel_tank",
                MassItemCategory::FuelAndFluids,
                Some(100.0),
                Some([4.0, 0.0, 0.0]),
                config,
            ),
            evidence_record(
                "occupant",
                MassItemCategory::Payload,
                Some(80.0),
                Some([2.0, 0.0, 0.5]),
                config,
            ),
            evidence_record(
                "removable_module",
                MassItemCategory::Propulsion,
                Some(40.0),
                Some([0.5, 0.0, 0.0]),
                config,
            ),
        ];
        records.sort_by(|left, right| left.item.id.cmp(&right.item.id));
        MassEvidenceLedger {
            schema_version: MASS_EVIDENCE_SCHEMA_VERSION,
            configuration_sha256: config.to_owned(),
            aircraft_family: "Wren 2".to_owned(),
            aircraft_model: "Wren 2 research fixture".to_owned(),
            serial_number: "SYNTH-001".to_owned(),
            records,
        }
    }

    fn state_item(
        id: &str,
        installation: InstallationState,
        mass_kg: Option<f64>,
        geometry: MassGeometry,
        source_mass: Option<SourceMassValue>,
        source_position: Option<SourcePosition>,
        value_source: Option<SourceRecord>,
    ) -> LoadingItemState {
        LoadingItemState {
            item_id: id.to_owned(),
            installation,
            mass_kg,
            source_mass,
            geometry,
            source_position,
            value_source,
        }
    }

    fn installed_state_item(record: &MassItemEvidence) -> LoadingItemState {
        state_item(
            &record.item.id,
            InstallationState::Installed,
            record.item.mass_kg,
            record.item.geometry.clone(),
            record.source_mass,
            record.source_position,
            Some(record.source.clone()),
        )
    }

    fn removed_state_item(record: &MassItemEvidence) -> LoadingItemState {
        state_item(
            &record.item.id,
            InstallationState::Removed,
            None,
            MassGeometry::Unknown,
            None,
            None,
            None,
        )
    }

    fn make_state(ledger: &MassEvidenceLedger, kind: LoadingStateKind) -> AircraftLoadingState {
        let generation = ledger.source_generation_sha256().expect("generation");
        AircraftLoadingState {
            schema_version: LOADING_STATE_SCHEMA_VERSION,
            state_id: format!("synthetic-{kind:?}"),
            state_kind: kind,
            configuration_sha256: ledger.configuration_sha256.clone(),
            source_generation_sha256: generation,
            aircraft_family: ledger.aircraft_family.clone(),
            aircraft_model: ledger.aircraft_model.clone(),
            serial_number: ledger.serial_number.clone(),
            items: ledger.records.iter().map(installed_state_item).collect(),
        }
    }

    #[test]
    fn takeoff_and_landing_fuel_burn_are_distinct_and_replayable() {
        let ledger = fixture_ledger(CONFIG);
        let mut takeoff = make_state(&ledger, LoadingStateKind::Takeoff);
        let mut landing = make_state(&ledger, LoadingStateKind::Landing);
        let fuel_index = takeoff
            .items
            .iter()
            .position(|item| item.item_id == "fuel_tank")
            .expect("fuel row");
        for state in [&mut takeoff, &mut landing] {
            state.items[fuel_index].value_source = Some(source("fixture:fuel-state", SOURCE_B));
        }
        landing.items[fuel_index].mass_kg = Some(50.0);
        landing.items[fuel_index].source_mass = Some(SourceMassValue {
            value: 50.0,
            unit: MassUnit::Kilogram,
        });
        let takeoff_report = ledger
            .evaluate_state(&takeoff, CONFIG)
            .expect("takeoff state");
        let landing_report = ledger
            .evaluate_state(&landing, CONFIG)
            .expect("landing state");
        assert!((takeoff_report.mass_properties.total_mass_kg.unwrap() - 720.0).abs() < 1.0e-12);
        assert!((landing_report.mass_properties.total_mass_kg.unwrap() - 670.0).abs() < 1.0e-12);
        assert_ne!(
            takeoff_report.mass_properties.center_of_gravity_m,
            landing_report.mass_properties.center_of_gravity_m
        );
        assert_ne!(takeoff_report.state_sha256, landing_report.state_sha256);
    }

    #[test]
    fn payload_relocation_changes_cg_and_inertia_without_changing_total_mass() {
        let ledger = fixture_ledger(CONFIG);
        let state_a = make_state(&ledger, LoadingStateKind::Takeoff);
        let mut state_b = make_state(&ledger, LoadingStateKind::Takeoff);
        let index = state_b
            .items
            .iter()
            .position(|item| item.item_id == "occupant")
            .expect("occupant row");
        state_b.items[index].geometry = MassGeometry::PointMass {
            position_m: [3.0, 0.0, 0.5],
        };
        state_b.items[index].source_position = Some(SourcePosition {
            values: [3.0, 0.0, 0.5],
            unit: LengthUnit::Metre,
        });
        state_b.items[index].value_source = Some(source("fixture:relocated-seat", SOURCE_B));
        let a = ledger.evaluate_state(&state_a, CONFIG).expect("state A");
        let b = ledger.evaluate_state(&state_b, CONFIG).expect("state B");
        assert_eq!(a.mass_properties.total_mass_kg, b.mass_properties.total_mass_kg);
        assert_ne!(
            a.mass_properties.center_of_gravity_m,
            b.mass_properties.center_of_gravity_m
        );
        assert_ne!(
            a.mass_properties.inertia_tensor_about_cg_kg_m2,
            b.mass_properties.inertia_tensor_about_cg_kg_m2
        );
    }

    #[test]
    fn removed_module_is_explicit_and_changes_configuration_identity() {
        let ledger = fixture_ledger(CONFIG);
        let mut state = make_state(&ledger, LoadingStateKind::Maintenance);
        let index = state
            .items
            .iter()
            .position(|item| item.item_id == "removable_module")
            .expect("module row");
        state.items[index] = removed_state_item(
            ledger.records.iter().find(|record| record.item.id == "removable_module").unwrap(),
        );
        let removed_config_ledger = fixture_ledger(CONFIG_REMOVED_MODULE);
        let mut removed_state = make_state(&removed_config_ledger, LoadingStateKind::Maintenance);
        let removed_index = removed_state
            .items
            .iter()
            .position(|item| item.item_id == "removable_module")
            .expect("module row");
        removed_state.items[removed_index] = removed_state_item(
            removed_config_ledger
                .records
                .iter()
                .find(|record| record.item.id == "removable_module")
                .unwrap(),
        );
        let report = removed_config_ledger
            .evaluate_state(&removed_state, CONFIG_REMOVED_MODULE)
            .expect("removed module under new configuration");
        assert!((report.mass_properties.total_mass_kg.unwrap() - 680.0).abs() < 1.0e-12);
        assert_ne!(state.configuration_sha256, removed_state.configuration_sha256);
    }

    #[test]
    fn unknown_mass_with_known_location_withholds_complete_mass_properties() {
        let mut ledger = fixture_ledger(CONFIG);
        let target = ledger.records.iter_mut().find(|record| record.item.id == "fuel_tank").unwrap();
        target.item.mass_kg = None;
        target.source_mass = None;
        target.standard_uncertainty_kg = None;
        let mut state = make_state(&ledger, LoadingStateKind::Takeoff);
        let index = state.items.iter().position(|item| item.item_id == "fuel_tank").unwrap();
        state.items[index].mass_kg = None;
        state.items[index].source_mass = None;
        let report = ledger.evaluate_state(&state, CONFIG).expect("indeterminate report");
        assert_eq!(report.status, StateEvidenceStatus::Indeterminate);
        assert_eq!(report.mass_properties.total_mass_kg, None);
        assert_eq!(report.mass_properties.center_of_gravity_m, None);
    }

    #[test]
    fn known_mass_with_unknown_location_reports_mass_but_withholds_cg_and_inertia() {
        let mut ledger = fixture_ledger(CONFIG);
        let target = ledger.records.iter_mut().find(|record| record.item.id == "fuel_tank").unwrap();
        target.item.geometry = MassGeometry::Unknown;
        target.source_position = None;
        let mut state = make_state(&ledger, LoadingStateKind::Takeoff);
        let index = state.items.iter().position(|item| item.item_id == "fuel_tank").unwrap();
        state.items[index].geometry = MassGeometry::Unknown;
        state.items[index].source_position = None;
        let report = ledger.evaluate_state(&state, CONFIG).expect("indeterminate geometry");
        assert_eq!(report.mass_properties.total_mass_kg, Some(720.0));
        assert_eq!(report.mass_properties.center_of_gravity_m, None);
        assert_eq!(report.mass_properties.inertia_tensor_about_cg_kg_m2, None);
        assert_eq!(report.status, StateEvidenceStatus::Indeterminate);
    }

    #[test]
    fn source_mass_units_are_converted_and_checked_against_canonical_kg() {
        let converted = SourceMassValue {
            value: 220.462_262_184_877_57,
            unit: MassUnit::Pound,
        }
        .kilograms()
        .expect("lb conversion");
        assert!((converted - 100.0).abs() < 1.0e-10);
        let mut ledger = fixture_ledger(CONFIG);
        let target = ledger.records.iter_mut().find(|record| record.item.id == "fuel_tank").unwrap();
        target.source_mass = Some(SourceMassValue {
            value: 50.0,
            unit: MassUnit::Kilogram,
        });
        assert!(ledger.source_generation_sha256().is_ok());
        let state = make_state(&ledger, LoadingStateKind::Takeoff);
        let error = ledger.evaluate_state(&state, CONFIG).expect_err("unit mismatch");
        assert!(error.contains("source mass conversion"));
    }

    #[test]
    fn stale_source_generation_is_rejected() {
        let ledger = fixture_ledger(CONFIG);
        let mut state = make_state(&ledger, LoadingStateKind::Takeoff);
        state.source_generation_sha256 = SOURCE_B.to_owned();
        let error = ledger.evaluate_state(&state, CONFIG).expect_err("stale generation");
        assert!(error.contains("stale source generation"));
    }

    #[test]
    fn wrong_aircraft_effectivity_is_rejected() {
        let mut ledger = fixture_ledger(CONFIG);
        ledger.records[0].effectivity.aircraft_model = "different model".to_owned();
        let mut state = make_state(&ledger, LoadingStateKind::Takeoff);
        state.source_generation_sha256 = ledger.source_generation_sha256().expect("new generation");
        let error = ledger.evaluate_state(&state, CONFIG).expect_err("wrong effectivity");
        assert!(error.contains("effectivity mismatch"));
    }

    #[test]
    fn pending_evidence_keeps_state_indeterminate() {
        let mut ledger = fixture_ledger(CONFIG);
        ledger.records[0].admission = EvidenceAdmission::PendingReview;
        let mut state = make_state(&ledger, LoadingStateKind::Takeoff);
        state.source_generation_sha256 = ledger.source_generation_sha256().expect("new generation");
        let report = ledger.evaluate_state(&state, CONFIG).expect("report retained");
        assert_eq!(report.status, StateEvidenceStatus::Indeterminate);
        assert_eq!(report.pending_item_ids, vec!["airframe"]);
    }

    #[test]
    fn aggregate_and_included_child_cannot_both_be_installed() {
        let mut ledger = fixture_ledger(CONFIG);
        let mut aggregate = evidence_record(
            "propulsion_assembly",
            MassItemCategory::Propulsion,
            Some(60.0),
            Some([0.5, 0.0, 0.0]),
            CONFIG,
        );
        aggregate.accounting_role = AccountingRole::Aggregate;
        aggregate.included_child_ids = vec!["removable_module".to_owned()];
        ledger.records.push(aggregate);
        ledger.records.sort_by(|left, right| left.item.id.cmp(&right.item.id));
        let mut state = make_state(&ledger, LoadingStateKind::Takeoff);
        state.items.push(installed_state_item(ledger.records.last().unwrap()));
        state.items.sort_by(|left, right| left.item_id.cmp(&right.item_id));
        let error = ledger.evaluate_state(&state, CONFIG).expect_err("double count");
        assert!(error.contains("aggregate/child double counting"));
    }

    #[test]
    fn missing_manifest_item_is_rejected() {
        let ledger = fixture_ledger(CONFIG);
        let mut state = make_state(&ledger, LoadingStateKind::Takeoff);
        state.items.pop();
        let error = ledger.evaluate_state(&state, CONFIG).expect_err("missing manifest row");
        assert!(error.contains("explicitly cover every"));
    }

    #[test]
    fn uncertainty_is_reported_only_under_explicit_independence() {
        let ledger = fixture_ledger(CONFIG);
        let state = make_state(&ledger, LoadingStateKind::Takeoff);
        let report = ledger.evaluate_state(&state, CONFIG).expect("complete state");
        let expected = (4.0_f64 * 0.1_f64.powi(2)).sqrt();
        assert!((report.total_mass_standard_uncertainty_kg.unwrap() - expected).abs() < 1.0e-12);
        assert_eq!(report.cg_uncertainty_m, None);
        assert_eq!(report.inertia_uncertainty_kg_m2, None);
    }

    #[test]
    fn state_digest_changes_when_payload_location_changes() {
        let ledger = fixture_ledger(CONFIG);
        let first = make_state(&ledger, LoadingStateKind::Takeoff);
        let mut second = first.clone();
        let index = second.items.iter().position(|item| item.item_id == "occupant").unwrap();
        second.items[index].geometry = MassGeometry::PointMass {
            position_m: [3.0, 0.0, 0.5],
        };
        second.items[index].source_position = Some(SourcePosition {
            values: [3.0, 0.0, 0.5],
            unit: LengthUnit::Metre,
        });
        second.items[index].value_source = Some(source("fixture:relocated-seat", SOURCE_B));
        assert_ne!(
            ledger.state_sha256(&first).unwrap(),
            ledger.state_sha256(&second).unwrap()
        );
    }
}
