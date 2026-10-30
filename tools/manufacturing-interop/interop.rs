//! Typed MTConnect/QIF semantic mapping and explicit cycle-to-inspection joins.
//!
//! Parser-independent reference kernel. XML parsing and XSD validation are upstream
//! responsibilities; this kernel requires an explicit schema-validation receipt.
//!
//! It preserves native identities, units, timestamps, sequence numbers, source
//! locators and opaque extensions. It never infers part joins from text or time.
//!
//! Compile:
//!   rustc --edition=2024 -D warnings --test tools/manufacturing-interop/interop.rs -o /tmp/mfg-interop-tests
//!   /tmp/mfg-interop-tests
//!   rustc --edition=2024 -D warnings tools/manufacturing-interop/interop.rs -o /tmp/mfg-interop
//!   /tmp/mfg-interop --verify
//!   /tmp/mfg-interop --csv

use std::collections::BTreeMap;

const HASH_LEN: usize = 64;
const MTCONNECT_VERSION: &str = "2.8";
const QIF_VERSION: &str = "3.0";

fn valid_sha(value: &str) -> bool {
    value.len() == HASH_LEN && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn present(value: &str) -> bool {
    !value.trim().is_empty()
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn finite(value: f64) -> bool {
    value.is_finite()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Standard {
    MtConnect,
    Qif,
}

impl Standard {
    fn code(self) -> &'static str {
        match self {
            Self::MtConnect => "MTCONNECT",
            Self::Qif => "QIF",
        }
    }

    fn supported_version(self) -> &'static str {
        match self {
            Self::MtConnect => MTCONNECT_VERSION,
            Self::Qif => QIF_VERSION,
        }
    }
}

#[derive(Clone, Debug)]
struct OpaqueExtension {
    namespace: String,
    local_name: String,
    raw_xml_sha256: String,
    source_locator: String,
}

#[derive(Clone, Debug)]
struct SchemaAdmission {
    standard: Standard,
    version: String,
    namespace: String,
    schema_bundle_sha256: String,
    parser_id: String,
    parser_version: String,
    validation_authority_id: String,
    validation_receipt_sha256: String,
    xsd_validation_passed: bool,
}

#[derive(Clone, Debug)]
struct SourceDocument {
    document_id: String,
    document_sha256: String,
    source_uri: String,
    schema: SchemaAdmission,
    opaque_extensions: Vec<OpaqueExtension>,
}

impl SourceDocument {
    fn validate(&self, expected: Standard) -> Result<(), &'static str> {
        if self.schema.standard != expected {
            return Err("SOURCE_STANDARD_MISMATCH");
        }
        if self.schema.version != expected.supported_version() {
            return Err("UNSUPPORTED_SCHEMA_VERSION");
        }
        if !present(&self.document_id)
            || !present(&self.source_uri)
            || !present(&self.schema.namespace)
            || !present(&self.schema.parser_id)
            || !present(&self.schema.parser_version)
            || !present(&self.schema.validation_authority_id)
        {
            return Err("SOURCE_DOCUMENT_IDENTITY_MISSING");
        }
        if !valid_sha(&self.document_sha256)
            || !valid_sha(&self.schema.schema_bundle_sha256)
            || !valid_sha(&self.schema.validation_receipt_sha256) {
            return Err("SOURCE_DOCUMENT_DIGEST_INVALID");
        }
        if !self.schema.xsd_validation_passed {
            return Err("SOURCE_XSD_VALIDATION_NOT_PASSED");
        }
        for extension in &self.opaque_extensions {
            if !present(&extension.namespace)
                || !present(&extension.local_name)
                || !present(&extension.source_locator)
                || !valid_sha(&extension.raw_xml_sha256)
            {
                return Err("OPAQUE_EXTENSION_RECEIPT_INVALID");
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
enum MtValue {
    Sample {
        value: f64,
        unit: String,
        statistic: Option<String>,
        subtype: Option<String>,
        coordinate_system: Option<String>,
    },
    Event {
        value: String,
        subtype: Option<String>,
        native_code: Option<String>,
    },
    Condition {
        level: String,
        code: Option<String>,
        native_code: Option<String>,
        description: Option<String>,
    },
    Asset {
        asset_type: String,
        asset_id: String,
    },
}

impl MtValue {
    fn kind_code(&self) -> &'static str {
        match self {
            Self::Sample { .. } => "SAMPLE",
            Self::Event { .. } => "EVENT",
            Self::Condition { .. } => "CONDITION",
            Self::Asset { .. } => "ASSET",
        }
    }

    fn value_text(&self) -> String {
        match self {
            Self::Sample { value, .. } => format!("{value:.17e}"),
            Self::Event { value, .. } => value.clone(),
            Self::Condition { level, code, description, .. } => {
                format!("level={level};code={};description={}",
                    code.as_deref().unwrap_or(""),
                    description.as_deref().unwrap_or(""))
            }
            Self::Asset { asset_type, asset_id } => format!("{asset_type}:{asset_id}"),
        }
    }

    fn validate(&self) -> Result<(), &'static str> {
        match self {
            Self::Sample { value, unit, statistic, subtype, coordinate_system } => {
                if !finite(*value) { return Err("MTCONNECT_SAMPLE_NONFINITE"); }
                if !present(unit) { return Err("MTCONNECT_SAMPLE_UNIT_MISSING"); }
                for optional in [statistic, subtype, coordinate_system] {
                    if optional.as_ref().is_some_and(|value| value.trim().is_empty()) {
                        return Err("MTCONNECT_SAMPLE_METADATA_EMPTY");
                    }
                }
            }
            Self::Event { value, subtype, native_code } => {
                if !present(value) { return Err("MTCONNECT_EVENT_VALUE_MISSING"); }
                if subtype.as_ref().is_some_and(|v| v.trim().is_empty())
                    || native_code.as_ref().is_some_and(|v| v.trim().is_empty()) {
                    return Err("MTCONNECT_EVENT_METADATA_EMPTY");
                }
            }
            Self::Condition { level, code, native_code, description } => {
                if !present(level) { return Err("MTCONNECT_CONDITION_LEVEL_MISSING"); }
                if code.as_ref().is_some_and(|v| v.trim().is_empty())
                    || native_code.as_ref().is_some_and(|v| v.trim().is_empty())
                    || description.as_ref().is_some_and(|v| v.trim().is_empty()) {
                    return Err("MTCONNECT_CONDITION_METADATA_EMPTY");
                }
            }
            Self::Asset { asset_type, asset_id } => {
                if !present(asset_type) || !present(asset_id) {
                    return Err("MTCONNECT_ASSET_IDENTITY_MISSING");
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct MtObservation {
    device_uuid: String,
    stream_id: String,
    data_item_id: String,
    component_id: String,
    sequence: u64,
    source_timestamp_ns: i64,
    reference_timestamp_ns: i64,
    reference_clock_id: String,
    clock_mapping_sha256: String,
    clock_uncertainty_ns: u64,
    source_locator: String,
    document_order: usize,
    value: MtValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StreamDiagnosticKind {
    DuplicateSequence,
    SequenceGap,
    TimestampRegression,
    ConflictingDuplicateSequence,
}

impl StreamDiagnosticKind {
    fn code(self) -> &'static str {
        match self {
            Self::DuplicateSequence => "DUPLICATE_SEQUENCE",
            Self::SequenceGap => "SEQUENCE_GAP",
            Self::TimestampRegression => "TIMESTAMP_REGRESSION",
            Self::ConflictingDuplicateSequence => "CONFLICTING_DUPLICATE_SEQUENCE",
        }
    }

    fn invalidates_composition(self) -> bool {
        true
    }
}

#[derive(Clone, Debug)]
struct StreamDiagnostic {
    kind: StreamDiagnosticKind,
    sequence: u64,
    source_locator: String,
    detail: String,
}

#[derive(Clone, Debug)]
struct MappedObservation {
    device_uuid: String,
    stream_id: String,
    data_item_id: String,
    component_id: String,
    sequence: u64,
    source_timestamp_ns: i64,
    timestamp_ns: i64,
    reference_clock_id: String,
    clock_mapping_sha256: String,
    clock_uncertainty_ns: u64,
    document_order: usize,
    native_kind: &'static str,
    native_unit: String,
    value_text: String,
    source_document_sha256: String,
    source_locator: String,
    extension_count: usize,
}

#[derive(Clone, Debug)]
struct MappedStream {
    document_id: String,
    document_sha256: String,
    source_uri: String,
    schema_version: String,
    schema_namespace: String,
    schema_bundle_sha256: String,
    parser_id: String,
    parser_version: String,
    validation_authority_id: String,
    validation_receipt_sha256: String,
    standard: Standard,
    extension_manifest: String,
    device_uuid: String,
    stream_id: String,
    observations: Vec<MappedObservation>,
    diagnostics: Vec<StreamDiagnostic>,
}

impl MappedStream {
    fn composable(&self) -> bool {
        self.diagnostics.iter().all(|d| !d.kind.invalidates_composition())
    }

    fn diagnostic_manifest(&self) -> String {
        self.diagnostics.iter().map(|d| format!("{}:{}:{}:{}",
            d.kind.code(), d.sequence, d.source_locator, d.detail
        )).collect::<Vec<_>>().join(";")
    }

    fn csv_rows(&self) -> Vec<String> {
        self.observations.iter().map(|observation| {
            [
                self.standard.code().to_string(),
                csv_escape(&observation.device_uuid),
                csv_escape(&observation.stream_id),
                csv_escape(&observation.data_item_id),
                csv_escape(&observation.component_id),
                self.document_id.clone(),
                self.document_sha256.clone(),
                csv_escape(&self.source_uri),
                observation.sequence.to_string(),
                observation.source_timestamp_ns.to_string(),
                observation.timestamp_ns.to_string(),
                csv_escape(&observation.reference_clock_id),
                observation.clock_mapping_sha256.clone(),
                observation.clock_uncertainty_ns.to_string(),
                observation.document_order.to_string(),
                observation.native_kind.to_string(),
                csv_escape(&observation.native_unit),
                csv_escape(&observation.value_text),
                csv_escape(&observation.source_locator),
                self.schema_version.clone(),
                csv_escape(&self.schema_namespace),
                self.schema_bundle_sha256.clone(),
                csv_escape(&self.parser_id),
                csv_escape(&self.parser_version),
                csv_escape(&self.validation_authority_id),
                self.validation_receipt_sha256.clone(),
                csv_escape(&self.extension_manifest),
                self.diagnostic_manifest(),
            ].into_iter().map(|field| csv_escape(&field)).collect::<Vec<_>>().join(",")
        }).collect()
    }
}

fn native_unit(value: &MtValue) -> String {
    match value {
        MtValue::Sample { unit, .. } => unit.clone(),
        MtValue::Event { .. } => String::new(),
        MtValue::Condition { .. } => String::new(),
        MtValue::Asset { .. } => String::new(),
    }
}

fn map_mtconnect(
    source: &SourceDocument,
    device_uuid: &str,
    stream_id: &str,
    complete_unfiltered_stream: bool,
    observations: &[MtObservation],
) -> Result<MappedStream, &'static str> {
    source.validate(Standard::MtConnect)?;
    if !present(device_uuid) || !present(stream_id) {
        return Err("MTCONNECT_STREAM_IDENTITY_MISSING");
    }
    if observations.is_empty() {
        return Err("MTCONNECT_STREAM_EMPTY");
    }

    let mut mapped = Vec::with_capacity(observations.len());
    let mut diagnostics = Vec::new();
    let mut sequence_index: BTreeMap<u64, &MtObservation> = BTreeMap::new();
    let mut chronological: Vec<&MtObservation> = observations.iter().collect();

    for observation in observations {
        if observation.device_uuid != device_uuid || observation.stream_id != stream_id {
            return Err("MTCONNECT_OBSERVATION_STREAM_IDENTITY_MISMATCH");
        }
        if !present(&observation.data_item_id)
            || !present(&observation.component_id)
            || !present(&observation.source_locator)
            || !present(&observation.reference_clock_id)
            || !valid_sha(&observation.clock_mapping_sha256)
            || observation.clock_uncertainty_ns > 100_000 {
        
            return Err("MTCONNECT_OBSERVATION_OR_CLOCK_IDENTITY_INVALID");
        }
        observation.value.validate()?;

        if let Some(previous) = sequence_index.insert(observation.sequence, observation) {
            let kind = if previous.value == observation.value
                && previous.data_item_id == observation.data_item_id
                && previous.component_id == observation.component_id
                && previous.reference_timestamp_ns == observation.reference_timestamp_ns
            {
                StreamDiagnosticKind::DuplicateSequence
            } else {
                StreamDiagnosticKind::ConflictingDuplicateSequence
            };
            diagnostics.push(StreamDiagnostic {
                kind,
                sequence: observation.sequence,
                source_locator: observation.source_locator.clone(),
                detail: format!("sequence {} already appeared at {}", observation.sequence, previous.source_locator),
            });
        }

        mapped.push(MappedObservation {
            device_uuid: device_uuid.to_string(),
            stream_id: stream_id.to_string(),
            data_item_id: observation.data_item_id.clone(),
            component_id: observation.component_id.clone(),
            sequence: observation.sequence,
            source_timestamp_ns: observation.source_timestamp_ns,
            timestamp_ns: observation.reference_timestamp_ns,
            reference_clock_id: observation.reference_clock_id.clone(),
            clock_mapping_sha256: observation.clock_mapping_sha256.clone(),
            clock_uncertainty_ns: observation.clock_uncertainty_ns,
            document_order: observation.document_order,
            native_kind: observation.value.kind_code(),
            native_unit: native_unit(&observation.value),
            value_text: observation.value.value_text(),
            source_document_sha256: source.document_sha256.clone(),
            source_locator: observation.source_locator.clone(),
            extension_count: source.opaque_extensions.len(),
        });
    }

    chronological.sort_by_key(|observation| (observation.sequence, observation.document_order));
    for pair in chronological.windows(2) {
        let previous = pair[0];
        let current = pair[1];
        if current.reference_timestamp_ns < previous.reference_timestamp_ns {
            diagnostics.push(StreamDiagnostic {
                kind: StreamDiagnosticKind::TimestampRegression,
                sequence: current.sequence,
                source_locator: current.source_locator.clone(),
                detail: format!("reference timestamp {} precedes sequence {} timestamp {}",
                    current.reference_timestamp_ns, previous.sequence, previous.reference_timestamp_ns),
            });
        }
        if complete_unfiltered_stream && current.sequence > previous.sequence.saturating_add(1) {
            diagnostics.push(StreamDiagnostic {
                kind: StreamDiagnosticKind::SequenceGap,
                sequence: current.sequence,
                source_locator: current.source_locator.clone(),
                detail: format!("missing sequence range {}..{}",
                    previous.sequence.saturating_add(1), current.sequence.saturating_sub(1)),
            });
        }
    }

    mapped.sort_by(|a, b| (
        a.device_uuid.as_str(), a.stream_id.as_str(), a.sequence, a.source_locator.as_str()
    ).cmp(&(
        b.device_uuid.as_str(), b.stream_id.as_str(), b.sequence, b.source_locator.as_str()
    )));

    let extension_manifest = source.opaque_extensions.iter().map(|extension| format!(
        "{}:{}:{}:{}",
        extension.namespace, extension.local_name, extension.raw_xml_sha256, extension.source_locator
    )).collect::<Vec<_>>().join(";");

    Ok(MappedStream {
        document_id: source.document_id.clone(),
        document_sha256: source.document_sha256.clone(),
        source_uri: source.source_uri.clone(),
        schema_version: source.schema.version.clone(),
        schema_namespace: source.schema.namespace.clone(),
        schema_bundle_sha256: source.schema.schema_bundle_sha256.clone(),
        parser_id: source.schema.parser_id.clone(),
        parser_version: source.schema.parser_version.clone(),
        validation_authority_id: source.schema.validation_authority_id.clone(),
        validation_receipt_sha256: source.schema.validation_receipt_sha256.clone(),
        standard: source.schema.standard,
        extension_manifest,
        device_uuid: device_uuid.to_string(),
        stream_id: stream_id.to_string(),
        observations: mapped,
        diagnostics,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum QifResultStatus {
    Measured,
    NominalOnly,
    NotMeasured,
    Failed,
}

#[derive(Clone, Debug)]
struct QifInspectionResult {
    source_document_id: String,
    source_document_sha256: String,
    qif_document_uuid: String,
    result_id: String,
    part_id: String,
    lot_id: String,
    operation_id: String,
    measurement_plan_id: String,
    characteristic_id: String,
    feature_id: String,
    specification_id: String,
    specification_revision: String,
    instrument_resource_id: String,
    actual_value: Option<f64>,
    original_unit: String,
    source_measured_at_ns: i64,
    measured_at_reference_ns: i64,
    reference_clock_id: String,
    clock_mapping_sha256: String,
    clock_uncertainty_ns: u64,
    result_status: QifResultStatus,
    source_locator: String,
}

impl QifInspectionResult {
    fn validate(&self, source: &SourceDocument) -> Result<(), &'static str> {
        source.validate(Standard::Qif)?;
        if self.source_document_id != source.document_id
            || self.source_document_sha256 != source.document_sha256 {
            return Err("QIF_RESULT_SOURCE_DOCUMENT_MISMATCH");
        }
        for id in [
            self.qif_document_uuid.as_str(), self.result_id.as_str(), self.part_id.as_str(),
            self.lot_id.as_str(), self.operation_id.as_str(), self.measurement_plan_id.as_str(),
            self.characteristic_id.as_str(), self.feature_id.as_str(), self.specification_id.as_str(),
            self.specification_revision.as_str(), self.instrument_resource_id.as_str(),
            self.original_unit.as_str(), self.source_locator.as_str(),
            self.reference_clock_id.as_str(),
        ] {
            if !present(id) {
                return Err("QIF_RESULT_IDENTITY_MISSING");
            }
        }
        if self.result_status != QifResultStatus::Measured {
            return Err("QIF_RESULT_NOT_MEASURED");
        }
        if !valid_sha(&self.clock_mapping_sha256) || self.clock_uncertainty_ns > 100_000 {
            return Err("QIF_CLOCK_MAPPING_INVALID");
        }
        let actual = self.actual_value.ok_or("QIF_ACTUAL_VALUE_MISSING")?;
        if !finite(actual) {
            return Err("QIF_ACTUAL_VALUE_NONFINITE");
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct OperationCycle {
    cycle_id: String,
    part_id: String,
    lot_id: String,
    operation_id: String,
    start_ns: i64,
    end_ns: i64,
    reference_clock_id: String,
    clock_mapping_sha256: String,
    clock_uncertainty_ns: u64,
    mtconnect_document_sha256: String,
    device_uuid: String,
    stream_id: String,
    source_locator: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AssociationState {
    Verified,
    Missing,
    Unverified,
    Revoked,
}

#[derive(Clone, Debug)]
struct AssociationReceipt {
    receipt_id: String,
    state: AssociationState,
    relation_type: String,
    mtconnect_document_sha256: String,
    cycle_id: String,
    qif_document_sha256: String,
    qif_result_id: String,
    part_id: String,
    lot_id: String,
    operation_id: String,
    characteristic_id: String,
    reference_clock_id: String,
    valid_from_ns: i64,
    valid_until_ns: i64,
    authority_id: String,
    source_sha256: String,
    source_locator: String,
}

#[derive(Clone, Debug)]
struct UnitTransform {
    source_unit: String,
    target_unit: String,
    scale: f64,
    offset: f64,
    revision: String,
}

#[derive(Clone, Debug)]
struct DerivedValue {
    original_value: f64,
    original_unit: String,
    derived_value: f64,
    derived_unit: String,
    transform_revision: String,
}

fn apply_unit_transform(value: f64, unit: &str, transform: &UnitTransform) -> Result<DerivedValue, &'static str> {
    if !finite(value) || !finite(transform.scale) || !finite(transform.offset) || transform.scale <= 0.0 {
        return Err("UNIT_TRANSFORM_NUMERIC_INVALID");
    }
    if !present(unit) || !present(&transform.source_unit) || !present(&transform.target_unit)
        || !present(&transform.revision) {
        return Err("UNIT_TRANSFORM_IDENTITY_MISSING");
    }
    if unit != transform.source_unit {
        return Err("UNIT_TRANSFORM_SOURCE_MISMATCH");
    }
    let converted = value.mul_add(transform.scale, transform.offset);
    if !finite(converted) {
        return Err("UNIT_TRANSFORM_RESULT_NONFINITE");
    }
    Ok(DerivedValue {
        original_value: value,
        original_unit: unit.to_string(),
        derived_value: converted,
        derived_unit: transform.target_unit.clone(),
        transform_revision: transform.revision.clone(),
    })
}

#[derive(Clone, Debug)]
struct ComposedEvidence {
    cycle_id: String,
    part_id: String,
    lot_id: String,
    operation_id: String,
    qif_document_uuid: String,
    qif_result_id: String,
    measurement_plan_id: String,
    characteristic_id: String,
    feature_id: String,
    instrument_resource_id: String,
    specification_id: String,
    specification_revision: String,
    actual_value: f64,
    original_unit: String,
    derived_value: Option<DerivedValue>,
    source_measured_at_ns: i64,
    measured_at_ns: i64,
    reference_clock_id: String,
    measurement_clock_mapping_sha256: String,
    cycle_clock_mapping_sha256: String,
    combined_clock_uncertainty_ns: u64,
    association_receipt_id: String,
    association_authority_id: String,
    association_valid_from_ns: i64,
    association_valid_until_ns: i64,
    mtconnect_document_sha256: String,
    qif_document_sha256: String,
    association_receipt_sha256: String,
    qif_source_locator: String,
    cycle_source_locator: String,
}

impl ComposedEvidence {
    fn csv_row(&self) -> String {
        let (derived_value, derived_unit, transform_revision) = self.derived_value.as_ref()
            .map(|v| (format!("{:.17e}", v.derived_value), v.derived_unit.clone(), v.transform_revision.clone()))
            .unwrap_or_default();
        [
            csv_escape(&self.cycle_id), csv_escape(&self.part_id), csv_escape(&self.lot_id),
            csv_escape(&self.operation_id), csv_escape(&self.qif_document_uuid),
            csv_escape(&self.qif_result_id), csv_escape(&self.measurement_plan_id),
            csv_escape(&self.characteristic_id), csv_escape(&self.feature_id),
            csv_escape(&self.instrument_resource_id), csv_escape(&self.specification_id),
            csv_escape(&self.specification_revision), format!("{:.17e}", self.actual_value),
            csv_escape(&self.original_unit), self.source_measured_at_ns.to_string(),
            self.measured_at_ns.to_string(), csv_escape(&self.reference_clock_id),
            self.measurement_clock_mapping_sha256.clone(), self.cycle_clock_mapping_sha256.clone(),
            self.combined_clock_uncertainty_ns.to_string(),
            csv_escape(&self.association_receipt_id), csv_escape(&self.association_authority_id),
            self.association_valid_from_ns.to_string(), self.association_valid_until_ns.to_string(),
            self.mtconnect_document_sha256.clone(), self.qif_document_sha256.clone(),
            self.association_receipt_sha256.clone(), csv_escape(&self.qif_source_locator),
            csv_escape(&self.cycle_source_locator), derived_value, derived_unit, transform_revision,
        ].join(",")
    }
}

#[derive(Clone, Debug)]
struct JoinResult {
    composed: Option<ComposedEvidence>,
    status: &'static str,
    reason: &'static str,
}

fn join_qif_to_cycle(
    stream: &MappedStream,
    cycle: &OperationCycle,
    qif_source: &SourceDocument,
    result: &QifInspectionResult,
    association: &AssociationReceipt,
    decision_time_ns: i64,
    transform: Option<&UnitTransform>,
) -> JoinResult {
    let fail = |reason| JoinResult { composed: None, status: "INCONCLUSIVE", reason };
    if !stream.composable() {
        return fail("MTCONNECT_STREAM_DIAGNOSTICS_PRESENT");
    }
    if stream.document_sha256 != cycle.mtconnect_document_sha256
        || stream.device_uuid != cycle.device_uuid
        || stream.stream_id != cycle.stream_id {
        return fail("MTCONNECT_CYCLE_SOURCE_MISMATCH");
    }
    if !valid_sha(&stream.document_sha256) || !valid_sha(&cycle.mtconnect_document_sha256) {
        return fail("MTCONNECT_SOURCE_DIGEST_INVALID");
    }
    if !present(&cycle.cycle_id) || !present(&cycle.part_id) || !present(&cycle.lot_id)
        || !present(&cycle.operation_id) || !present(&cycle.source_locator)
        || !present(&cycle.reference_clock_id) || !valid_sha(&cycle.clock_mapping_sha256)
        || cycle.clock_uncertainty_ns > 100_000 || cycle.start_ns >= cycle.end_ns {
        return fail("OPERATION_CYCLE_IDENTITY_INVALID");
    }
    if let Err(_) = qif_source.validate(Standard::Qif) {
        return fail("QIF_SOURCE_NOT_ADMITTED");
    }
    if let Err(_) = result.validate(qif_source) {
        return fail("QIF_RESULT_NOT_ADMITTED");
    }
    if decision_time_ns < association.valid_from_ns || decision_time_ns > association.valid_until_ns
        || association.valid_from_ns > association.valid_until_ns {
        return fail("ASSOCIATION_RECEIPT_NOT_EFFECTIVE");
    }
    if association.state != AssociationState::Verified {
        return fail("ASSOCIATION_RECEIPT_NOT_VERIFIED");
    }
    if association.relation_type != "inspection_result_of_operation_cycle" {
        return fail("ASSOCIATION_RELATION_TYPE_UNSUPPORTED");
    }
    if !present(&association.receipt_id) || !present(&association.authority_id)
        || !present(&association.source_locator) || !valid_sha(&association.source_sha256) {
        return fail("ASSOCIATION_RECEIPT_EVIDENCE_INVALID");
    }
    if association.mtconnect_document_sha256 != stream.document_sha256
        || association.cycle_id != cycle.cycle_id
        || association.qif_document_sha256 != qif_source.document_sha256
        || association.qif_result_id != result.result_id
        || association.part_id != cycle.part_id
        || association.lot_id != cycle.lot_id
        || association.operation_id != cycle.operation_id
        || association.characteristic_id != result.characteristic_id {
        return fail("ASSOCIATION_RECEIPT_SCOPE_MISMATCH");
    }
    if result.part_id != cycle.part_id || result.lot_id != cycle.lot_id
        || result.operation_id != cycle.operation_id {
        return fail("PART_LOT_OPERATION_IDENTITY_MISMATCH");
    }
    if result.reference_clock_id != cycle.reference_clock_id
        || association.reference_clock_id != cycle.reference_clock_id
        || result.clock_uncertainty_ns.saturating_add(cycle.clock_uncertainty_ns) > 100_000 {
        return fail("REFERENCE_CLOCK_ID_OR_UNCERTAINTY_MISMATCH");
    }
    if association.valid_from_ns > result.measured_at_reference_ns
        || association.valid_until_ns < result.measured_at_reference_ns {
        return fail("ASSOCIATION_NOT_VALID_AT_MEASUREMENT_TIME");
    }
    let relevant_observations: Vec<&MappedObservation> = stream.observations.iter()
        .filter(|obs| obs.timestamp_ns >= cycle.start_ns && obs.timestamp_ns <= cycle.end_ns)
        .collect();
    if relevant_observations.is_empty() {
        return fail("NO_MACHINE_OBSERVATIONS_IN_CYCLE_WINDOW");
    }
    if relevant_observations.iter().any(|obs|
        obs.reference_clock_id != cycle.reference_clock_id
        || obs.clock_mapping_sha256 != cycle.clock_mapping_sha256
        || obs.clock_uncertainty_ns > cycle.clock_uncertainty_ns
        || result.clock_uncertainty_ns.saturating_add(obs.clock_uncertainty_ns) > 100_000) {
        return fail("MACHINE_OBSERVATION_CLOCK_MAPPING_MISMATCH");
    }
    if result.measured_at_reference_ns < cycle.start_ns || result.measured_at_reference_ns > cycle.end_ns {
        return fail("QIF_MEASUREMENT_OUTSIDE_CYCLE_WINDOW");
    }
    let actual_value = result.actual_value.unwrap_or(f64::NAN);
    let derived_value = match transform {
        Some(spec) => match apply_unit_transform(actual_value, &result.original_unit, spec) {
            Ok(value) => Some(value),
            Err(_) => return fail("UNIT_TRANSFORM_NOT_ADMITTED"),
        },
        None => None,
    };

    JoinResult {
        composed: Some(ComposedEvidence {
            cycle_id: cycle.cycle_id.clone(),
            part_id: cycle.part_id.clone(),
            lot_id: cycle.lot_id.clone(),
            operation_id: cycle.operation_id.clone(),
            qif_document_uuid: result.qif_document_uuid.clone(),
            qif_result_id: result.result_id.clone(),
            measurement_plan_id: result.measurement_plan_id.clone(),
            characteristic_id: result.characteristic_id.clone(),
            feature_id: result.feature_id.clone(),
            instrument_resource_id: result.instrument_resource_id.clone(),
            specification_id: result.specification_id.clone(),
            specification_revision: result.specification_revision.clone(),
            actual_value,
            original_unit: result.original_unit.clone(),
            derived_value,
            source_measured_at_ns: result.source_measured_at_ns,
            measured_at_ns: result.measured_at_reference_ns,
            reference_clock_id: result.reference_clock_id.clone(),
            measurement_clock_mapping_sha256: result.clock_mapping_sha256.clone(),
            cycle_clock_mapping_sha256: cycle.clock_mapping_sha256.clone(),
            combined_clock_uncertainty_ns: result.clock_uncertainty_ns.saturating_add(cycle.clock_uncertainty_ns),
            association_receipt_id: association.receipt_id.clone(),
            association_authority_id: association.authority_id.clone(),
            association_valid_from_ns: association.valid_from_ns,
            association_valid_until_ns: association.valid_until_ns,
            mtconnect_document_sha256: stream.document_sha256.clone(),
            qif_document_sha256: qif_source.document_sha256.clone(),
            association_receipt_sha256: association.source_sha256.clone(),
            qif_source_locator: result.source_locator.clone(),
            cycle_source_locator: cycle.source_locator.clone(),
        }),
        status: "COMPOSED",
        reason: "EXPLICIT_VERIFIED_ASSOCIATION_AND_MATCHED_SCOPE",
    }
}

fn fixture_source(standard: Standard) -> SourceDocument {
    let (version, namespace, prefix, source_sha) = match standard {
        Standard::MtConnect => ("2.8", "urn:mtconnect.org:MTConnectStreams:2.8", "MT", "a"),
        Standard::Qif => ("3.0", "http://qifstandards.org/xsd/qif3", "QIF", "b"),
    };
    SourceDocument {
        document_id: format!("{prefix}-DOC-0001"),
        document_sha256: prefix_hash(source_sha),
        source_uri: format!("fixture://{prefix}/document.xml"),
        schema: SchemaAdmission {
            standard,
            version: version.into(),
            namespace: namespace.into(),
            schema_bundle_sha256: prefix_hash(if standard == Standard::MtConnect { "1" } else { "2" }),
            parser_id: "fixture-parser".into(),
            parser_version: "0.1".into(),
            validation_authority_id: "fixture-xsd-validator".into(),
            validation_receipt_sha256: prefix_hash("5"),
            xsd_validation_passed: true,
        },
        opaque_extensions: vec![OpaqueExtension {
            namespace: "urn:vendor:extension:v1".into(),
            local_name: "VendorTrace".into(),
            raw_xml_sha256: prefix_hash("3"),
            source_locator: "/Document/Extensions/VendorTrace[1]".into(),
        }],
    }
}

fn prefix_hash(value: &str) -> String {
    format!("{value}{}", "0".repeat(HASH_LEN - 1))
}

fn fixture_observations(with_gap: bool) -> Vec<MtObservation> {
    let sequences = if with_gap { vec![10, 11, 13] } else { vec![10, 11, 12] };
    sequences.into_iter().enumerate().map(|(index, sequence)| {
        let value = match index {
            0 => MtValue::Sample {
                value: 1200.0, unit: "rpm".into(), statistic: None,
                subtype: Some("ACTUAL".into()), coordinate_system: None,
            },
            1 => MtValue::Event {
                value: "CYCLE_ACTIVE".into(), subtype: Some("STATE".into()),
                native_code: Some("ACTIVE".into()),
            },
            _ => MtValue::Condition {
                level: "NORMAL".into(), code: Some("OK".into()),
                native_code: Some("0".into()), description: Some("No fault".into()),
            },
        };
        MtObservation {
            device_uuid: "urn:uuid:machine-001".into(),
            stream_id: "STREAM-001".into(),
            data_item_id: format!("DID-{index:03}"),
            component_id: if index == 0 { "SPINDLE-01" } else { "CONTROLLER-01" }.into(),
            sequence,
            source_timestamp_ns: 1_750_000_100 + index as i64 * 100,
            reference_timestamp_ns: 1_750_000_100 + index as i64 * 100,
            reference_clock_id: "PLANT-UTC-001".into(),
            clock_mapping_sha256: prefix_hash("6"),
            clock_uncertainty_ns: 100,
            source_locator: format!("/MTConnectStreams/Streams/ComponentStream/Observations/Observation[{index}]"),
            document_order: index,
            value,
        }
    }).collect()
}

fn fixture_cycle(stream: &MappedStream) -> OperationCycle {
    OperationCycle {
        cycle_id: "CYCLE-001".into(),
        part_id: "PART-001".into(),
        lot_id: "LOT-001".into(),
        operation_id: "OP-MILL-10".into(),
        start_ns: 1_750_000_000,
        end_ns: 1_750_001_000,
        reference_clock_id: "PLANT-UTC-001".into(),
        clock_mapping_sha256: prefix_hash("6"),
        clock_uncertainty_ns: 100,
        mtconnect_document_sha256: stream.document_sha256.clone(),
        device_uuid: stream.device_uuid.clone(),
        stream_id: stream.stream_id.clone(),
        source_locator: "/MES/Cycle[1]".into(),
    }
}

fn fixture_qif_result(source: &SourceDocument) -> QifInspectionResult {
    QifInspectionResult {
        source_document_id: source.document_id.clone(),
        source_document_sha256: source.document_sha256.clone(),
        qif_document_uuid: "QIF-UUID-001".into(),
        result_id: "QIF-RESULT-001".into(),
        part_id: "PART-001".into(),
        lot_id: "LOT-001".into(),
        operation_id: "OP-MILL-10".into(),
        measurement_plan_id: "PLAN-001".into(),
        characteristic_id: "SHAFT-DIAMETER-01".into(),
        feature_id: "FEATURE-CYLINDER-01".into(),
        specification_id: "SPEC-SHAFT-01".into(),
        specification_revision: "REV-7".into(),
        instrument_resource_id: "CMM-01".into(),
        actual_value: Some(15.02),
        original_unit: "mm".into(),
        source_measured_at_ns: 1_750_000_490,
        measured_at_reference_ns: 1_750_000_500,
        reference_clock_id: "PLANT-UTC-001".into(),
        clock_mapping_sha256: prefix_hash("7"),
        clock_uncertainty_ns: 100,
        result_status: QifResultStatus::Measured,
        source_locator: "/QIFDocument/QIFResults/Results[1]".into(),
    }
}

fn fixture_association(stream: &MappedStream, qif: &SourceDocument) -> AssociationReceipt {
    AssociationReceipt {
        receipt_id: "ASSOC-001".into(),
        state: AssociationState::Verified,
        relation_type: "inspection_result_of_operation_cycle".into(),
        mtconnect_document_sha256: stream.document_sha256.clone(),
        cycle_id: "CYCLE-001".into(),
        qif_document_sha256: qif.document_sha256.clone(),
        qif_result_id: "QIF-RESULT-001".into(),
        part_id: "PART-001".into(),
        lot_id: "LOT-001".into(),
        operation_id: "OP-MILL-10".into(),
        characteristic_id: "SHAFT-DIAMETER-01".into(),
        reference_clock_id: "PLANT-UTC-001".into(),
        valid_from_ns: 1_700_000_000,
        valid_until_ns: 1_800_000_000,
        authority_id: "MES-QUALITY-LINKER-01".into(),
        source_sha256: prefix_hash("4"),
        source_locator: "/MES/AssociationReceipt[1]".into(),
    }
}

fn verify() -> Result<(), String> {
    let mt = fixture_source(Standard::MtConnect);
    let qif = fixture_source(Standard::Qif);
    let observations = fixture_observations(false);
    let stream = map_mtconnect(&mt, "urn:uuid:machine-001", "STREAM-001", true, &observations)
        .map_err(str::to_string)?;
    if stream.observations.len() != 3
        || stream.observations[0].native_kind != "SAMPLE"
        || stream.observations[1].native_kind != "EVENT"
        || stream.observations[2].native_kind != "CONDITION"
        || stream.observations.iter().any(|x| x.extension_count != 1) {
        return Err("MTCONNECT_SEMANTIC_PRESERVATION_FAILED".into());
    }

    let cycle = fixture_cycle(&stream);
    let result = fixture_qif_result(&qif);
    let association = fixture_association(&stream, &qif);
    let transform = UnitTransform {
        source_unit: "mm".into(), target_unit: "um".into(),
        scale: 1000.0, offset: 0.0, revision: "UNIT-MAP-1".into(),
    };
    let joined = join_qif_to_cycle(&stream, &cycle, &qif, &result, &association, 1_750_000_800, Some(&transform));
    let composed = joined.composed.ok_or("EXPECTED_EXPLICIT_JOIN_FAILED")?;
    if joined.status != "COMPOSED"
        || composed.original_unit != "mm"
        || (composed.actual_value - 15.02).abs() > 1.0e-12
        || (composed.derived_value.as_ref().ok_or("DERIVED_VALUE_MISSING")?.derived_value - 15020.0).abs() > 1.0e-9 {
        return Err("QIF_MTCONNECT_JOIN_KNOWN_ANSWER_FAILED".into());
    }
    let composed_row = composed.csv_row();
    for expected in ["QIF-UUID-001", "PLAN-001", "FEATURE-CYLINDER-01", "CMM-01",
        "ASSOC-001", "MES-QUALITY-LINKER-01", &mt.document_sha256, &qif.document_sha256] {
        if !composed_row.contains(expected) {
            return Err("COMPOSED_EVIDENCE_RECEIPT_INCOMPLETE".into());
        }
    }

    let gap_stream = map_mtconnect(&mt, "urn:uuid:machine-001", "STREAM-001", true, &fixture_observations(true))
        .map_err(str::to_string)?;
    if !gap_stream.diagnostics.iter().any(|d| d.kind == StreamDiagnosticKind::SequenceGap) {
        return Err("MTCONNECT_SEQUENCE_GAP_NOT_REPORTED".into());
    }
    let gap_cycle = fixture_cycle(&gap_stream);
    if join_qif_to_cycle(&gap_stream, &gap_cycle, &qif, &result,
        &fixture_association(&gap_stream, &qif), 1_750_000_800, None).status != "INCONCLUSIVE" {
        return Err("MTCONNECT_SEQUENCE_GAP_DID_NOT_ABSTAIN".into());
    }

    Ok(())
}

fn emit_csv() {
    println!("standard,device_uuid,stream_id,data_item_id,component_id,source_document_id,source_document_sha256,source_uri,sequence,source_timestamp_ns,reference_timestamp_ns,reference_clock_id,clock_mapping_sha256,clock_uncertainty_ns,document_order,native_kind,native_unit,value_text,source_locator,schema_version,schema_namespace,schema_bundle_sha256,parser_id,parser_version,validation_authority_id,validation_receipt_sha256,extension_manifest,diagnostic_manifest");
    let source = fixture_source(Standard::MtConnect);
    let mapped = map_mtconnect(&source, "urn:uuid:machine-001", "STREAM-001", true, &fixture_observations(false))
        .expect("frozen valid MTConnect fixture");
    for row in mapped.csv_rows() { println!("{row}"); }
}

fn emit_joined_csv() -> Result<(), String> {
    let mt = fixture_source(Standard::MtConnect);
    let qif = fixture_source(Standard::Qif);
    let stream = map_mtconnect(&mt, "urn:uuid:machine-001", "STREAM-001", true,
        &fixture_observations(false)).map_err(str::to_string)?;
    let cycle = fixture_cycle(&stream);
    let result = fixture_qif_result(&qif);
    let association = fixture_association(&stream, &qif);
    let transform = UnitTransform {
        source_unit: "mm".into(), target_unit: "um".into(),
        scale: 1000.0, offset: 0.0, revision: "UNIT-MAP-1".into(),
    };
    let joined = join_qif_to_cycle(&stream, &cycle, &qif, &result, &association,
        1_750_000_800, Some(&transform));
    let evidence = joined.composed.ok_or(joined.reason)?;
    println!("cycle_id,part_id,lot_id,operation_id,qif_document_uuid,qif_result_id,measurement_plan_id,characteristic_id,feature_id,instrument_resource_id,specification_id,specification_revision,actual_value,original_unit,source_measured_at_ns,measured_at_reference_ns,reference_clock_id,measurement_clock_mapping_sha256,cycle_clock_mapping_sha256,combined_clock_uncertainty_ns,association_receipt_id,association_authority_id,association_valid_from_ns,association_valid_until_ns,mtconnect_document_sha256,qif_document_sha256,association_receipt_sha256,qif_source_locator,cycle_source_locator,derived_value,derived_unit,transform_revision");
    println!("{}", evidence.csv_row());
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [] => verify().map(|()| println!("MFG_INTEROP_OK: typed MTConnect/QIF semantic preservation and explicit join")),
        [arg] if arg == "--verify" => verify().map(|()| println!("MFG_INTEROP_OK: typed MTConnect/QIF semantic preservation and explicit join")),
        [arg] if arg == "--csv" => { emit_csv(); Ok(()) },
        [arg] if arg == "--joined-csv" => emit_joined_csv(),
        [arg] if arg == "--help" || arg == "-h" => {
            println!("Usage: mfg-interop [--verify|--csv|--joined-csv|--help]");
            Ok(())
        }
        _ => Err("expected --verify, --csv or --help".into()),
    };
    if let Err(error) = result {
        eprintln!("MFG_INTEROP_ERROR: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mtconnect_sample_event_condition_asset_types_are_preserved() {
        let source = fixture_source(Standard::MtConnect);
        let mut observations = fixture_observations(false);
        observations.push(MtObservation {
            device_uuid: "urn:uuid:machine-001".into(),
            stream_id: "STREAM-001".into(),
            data_item_id: "TOOL-ASSET".into(),
            component_id: "TOOL-01".into(),
            sequence: 13,
            source_timestamp_ns: 1_750_000_400,
            reference_timestamp_ns: 1_750_000_400,
            reference_clock_id: "PLANT-UTC-001".into(),
            clock_mapping_sha256: prefix_hash("6"),
            clock_uncertainty_ns: 100,
            source_locator: "/MTConnectAssets/Assets/Asset[1]".into(),
            document_order: 3,
            value: MtValue::Asset { asset_type: "CuttingTool".into(), asset_id: "TOOL-0001".into() },
        });
        let mapped = map_mtconnect(&source, "urn:uuid:machine-001", "STREAM-001", true, &observations).unwrap();
        let kinds: Vec<_> = mapped.observations.iter().map(|o| o.native_kind).collect();
        assert_eq!(kinds, vec!["SAMPLE", "EVENT", "CONDITION", "ASSET"]);
    }

    #[test]
    fn unfiltered_sequence_gap_is_reported_and_blocks_composition() {
        let source = fixture_source(Standard::MtConnect);
        let mapped = map_mtconnect(&source, "urn:uuid:machine-001", "STREAM-001", true,
            &fixture_observations(true)).unwrap();
        assert!(mapped.diagnostics.iter().any(|d| d.kind == StreamDiagnosticKind::SequenceGap));
        let qif = fixture_source(Standard::Qif);
        let result = fixture_qif_result(&qif);
        let cycle = fixture_cycle(&mapped);
        let association = fixture_association(&mapped, &qif);
        let joined = join_qif_to_cycle(&mapped,&cycle,&qif,&result,&association,1_750_000_800,None);
        assert_eq!(joined.status,"INCONCLUSIVE");
        assert_eq!(joined.reason,"MTCONNECT_STREAM_DIAGNOSTICS_PRESENT");
    }

    #[test]
    fn filtered_stream_does_not_claim_sequence_gaps_as_loss() {
        let source = fixture_source(Standard::MtConnect);
        let mapped = map_mtconnect(&source, "urn:uuid:machine-001", "STREAM-001", false,
            &fixture_observations(true)).unwrap();
        assert!(!mapped.diagnostics.iter().any(|d| d.kind == StreamDiagnosticKind::SequenceGap));
    }

    #[test]
    fn duplicate_sequence_and_conflicting_duplicate_are_distinct() {
        let source = fixture_source(Standard::MtConnect);
        let mut observations = fixture_observations(false);
        observations.push(observations[0].clone());
        let mapped = map_mtconnect(&source,"urn:uuid:machine-001","STREAM-001",false,&observations).unwrap();
        assert!(mapped.diagnostics.iter().any(|d| d.kind == StreamDiagnosticKind::DuplicateSequence));
        observations.last_mut().unwrap().value = MtValue::Sample {
            value: 1201.0, unit: "rpm".into(), statistic: None,
            subtype: Some("ACTUAL".into()), coordinate_system: None,
        };
        let mapped = map_mtconnect(&source,"urn:uuid:machine-001","STREAM-001",false,&observations).unwrap();
        assert!(mapped.diagnostics.iter().any(|d| d.kind == StreamDiagnosticKind::ConflictingDuplicateSequence));
    }

    #[test]
    fn timestamp_regression_is_reported_without_rewriting_source_times() {
        let source = fixture_source(Standard::MtConnect);
        let mut observations = fixture_observations(false);
        observations[2].reference_timestamp_ns = observations[0].reference_timestamp_ns - 1;
        let mapped = map_mtconnect(&source,"urn:uuid:machine-001","STREAM-001",false,&observations).unwrap();
        assert!(mapped.diagnostics.iter().any(|d| d.kind == StreamDiagnosticKind::TimestampRegression));
        assert_eq!(mapped.observations[2].timestamp_ns,observations[2].reference_timestamp_ns);
        assert_eq!(mapped.observations[2].source_timestamp_ns,observations[2].source_timestamp_ns);
    }

    #[test]
    fn explicit_qif_join_composes_and_retains_original_and_derived_units() {
        let source = fixture_source(Standard::MtConnect);
        let stream = map_mtconnect(&source,"urn:uuid:machine-001","STREAM-001",true,&fixture_observations(false)).unwrap();
        let qif = fixture_source(Standard::Qif);
        let result = fixture_qif_result(&qif);
        let cycle = fixture_cycle(&stream);
        let association = fixture_association(&stream,&qif);
        let transform = UnitTransform {
            source_unit:"mm".into(),target_unit:"um".into(),scale:1000.0,offset:0.0,revision:"UNIT-MAP-1".into()
        };
        let join = join_qif_to_cycle(&stream,&cycle,&qif,&result,&association,1_750_000_800,Some(&transform));
        assert_eq!(join.status,"COMPOSED");
        let evidence=join.composed.unwrap();
        assert_eq!(evidence.original_unit,"mm");
        assert_eq!(evidence.derived_value.unwrap().derived_unit,"um");
        assert_eq!(evidence.association_receipt_sha256,association.source_sha256);
    }

    #[test]
    fn qif_nominal_not_measured_and_failed_records_never_compose() {
        let mt=fixture_source(Standard::MtConnect);
        let stream=map_mtconnect(&mt,"urn:uuid:machine-001","STREAM-001",true,&fixture_observations(false)).unwrap();
        let qif=fixture_source(Standard::Qif);
        let cycle=fixture_cycle(&stream);
        let association=fixture_association(&stream,&qif);
        for status in [QifResultStatus::NominalOnly,QifResultStatus::NotMeasured,QifResultStatus::Failed] {
            let mut result=fixture_qif_result(&qif);
            result.result_status=status;
            let join=join_qif_to_cycle(&stream,&cycle,&qif,&result,&association,1_750_000_800,None);
            assert_eq!(join.status,"INCONCLUSIVE");
            assert_eq!(join.reason,"QIF_RESULT_NOT_ADMITTED");
        }
    }

    #[test]
    fn missing_and_revoked_association_states_never_compose() {
        let mt=fixture_source(Standard::MtConnect);
        let stream=map_mtconnect(&mt,"urn:uuid:machine-001","STREAM-001",true,&fixture_observations(false)).unwrap();
        let qif=fixture_source(Standard::Qif);
        let result=fixture_qif_result(&qif);
        let cycle=fixture_cycle(&stream);
        for state in [AssociationState::Missing,AssociationState::Revoked] {
            let mut association=fixture_association(&stream,&qif);
            association.state=state;
            let join=join_qif_to_cycle(&stream,&cycle,&qif,&result,&association,1_750_000_800,None);
            assert_eq!(join.status,"INCONCLUSIVE");
            assert_eq!(join.reason,"ASSOCIATION_RECEIPT_NOT_VERIFIED");
        }
    }

    #[test]
    fn missing_or_unverified_association_never_joins() {
        let mt = fixture_source(Standard::MtConnect);
        let stream = map_mtconnect(&mt,"urn:uuid:machine-001","STREAM-001",true,&fixture_observations(false)).unwrap();
        let qif = fixture_source(Standard::Qif);
        let result = fixture_qif_result(&qif);
        let cycle = fixture_cycle(&stream);
        let mut association = fixture_association(&stream,&qif);
        association.state = AssociationState::Unverified;
        let join = join_qif_to_cycle(&stream,&cycle,&qif,&result,&association,1_750_000_800,None);
        assert_eq!(join.status,"INCONCLUSIVE");
        assert_eq!(join.reason,"ASSOCIATION_RECEIPT_NOT_VERIFIED");
    }

    #[test]
    fn wrong_part_lot_operation_characteristic_and_document_fail_closed() {
        let mt = fixture_source(Standard::MtConnect);
        let stream = map_mtconnect(&mt,"urn:uuid:machine-001","STREAM-001",true,&fixture_observations(false)).unwrap();
        let qif = fixture_source(Standard::Qif);
        let result = fixture_qif_result(&qif);
        let cycle = fixture_cycle(&stream);
        let mut association = fixture_association(&stream,&qif);
        association.part_id = "PART-OTHER".into();
        let join = join_qif_to_cycle(&stream,&cycle,&qif,&result,&association,1_750_000_800,None);
        assert_eq!(join.reason,"ASSOCIATION_RECEIPT_SCOPE_MISMATCH");

        let mut wrong_result = result.clone();
        wrong_result.lot_id = "LOT-OTHER".into();
        let association = fixture_association(&stream,&qif);
        let join = join_qif_to_cycle(&stream,&cycle,&qif,&wrong_result,&association,1_750_000_800,None);
        assert_eq!(join.reason,"PART_LOT_OPERATION_IDENTITY_MISMATCH");
    }

    #[test]
    fn different_or_unqualified_clock_reference_cannot_join() {
        let mt=fixture_source(Standard::MtConnect);
        let stream=map_mtconnect(&mt,"urn:uuid:machine-001","STREAM-001",true,&fixture_observations(false)).unwrap();
        let qif=fixture_source(Standard::Qif);
        let mut result=fixture_qif_result(&qif);
        result.reference_clock_id="CMM-LOCAL-CLOCK".into();
        let cycle=fixture_cycle(&stream);
        let association=fixture_association(&stream,&qif);
        let join=join_qif_to_cycle(&stream,&cycle,&qif,&result,&association,1_750_000_800,None);
        assert_eq!(join.status,"INCONCLUSIVE");
        assert_eq!(join.reason,"REFERENCE_CLOCK_ID_OR_UNCERTAINTY_MISMATCH");

        let mut cycle=fixture_cycle(&stream);
        cycle.clock_uncertainty_ns=100_000;
        let result=fixture_qif_result(&qif);
        let join=join_qif_to_cycle(&stream,&cycle,&qif,&result,&association,1_750_000_800,None);
        assert_eq!(join.reason,"REFERENCE_CLOCK_ID_OR_UNCERTAINTY_MISMATCH");
    }

    #[test]
    fn association_must_cover_measurement_time_not_only_decision_time() {
        let mt=fixture_source(Standard::MtConnect);
        let stream=map_mtconnect(&mt,"urn:uuid:machine-001","STREAM-001",true,&fixture_observations(false)).unwrap();
        let qif=fixture_source(Standard::Qif);
        let result=fixture_qif_result(&qif);
        let cycle=fixture_cycle(&stream);
        let mut association=fixture_association(&stream,&qif);
        association.valid_from_ns=1_750_000_600;
        let join=join_qif_to_cycle(&stream,&cycle,&qif,&result,&association,1_750_000_800,None);
        assert_eq!(join.reason,"ASSOCIATION_NOT_VALID_AT_MEASUREMENT_TIME");
    }

    #[test]
    fn cycle_clock_uncertainty_must_cover_observation_clock_uncertainty() {
        let mt=fixture_source(Standard::MtConnect);
        let stream=map_mtconnect(&mt,"urn:uuid:machine-001","STREAM-001",true,&fixture_observations(false)).unwrap();
        let qif=fixture_source(Standard::Qif);
        let result=fixture_qif_result(&qif);
        let mut cycle=fixture_cycle(&stream);
        cycle.clock_uncertainty_ns=50;
        let association=fixture_association(&stream,&qif);
        let join=join_qif_to_cycle(&stream,&cycle,&qif,&result,&association,1_750_000_800,None);
        assert_eq!(join.status,"INCONCLUSIVE");
        assert_eq!(join.reason,"MACHINE_OBSERVATION_CLOCK_MAPPING_MISMATCH");
    }

    #[test]
    fn machine_observation_must_use_cycle_clock_mapping() {
        let mt=fixture_source(Standard::MtConnect);
        let mut observations=fixture_observations(false);
        observations[0].clock_mapping_sha256=prefix_hash("8");
        let stream=map_mtconnect(&mt,"urn:uuid:machine-001","STREAM-001",true,&observations).unwrap();
        let qif=fixture_source(Standard::Qif);
        let result=fixture_qif_result(&qif);
        let cycle=fixture_cycle(&stream);
        let association=fixture_association(&stream,&qif);
        let join=join_qif_to_cycle(&stream,&cycle,&qif,&result,&association,1_750_000_800,None);
        assert_eq!(join.reason,"MACHINE_OBSERVATION_CLOCK_MAPPING_MISMATCH");
    }

    #[test]
    fn measurement_outside_cycle_window_cannot_join() {
        let mt=fixture_source(Standard::MtConnect);
        let stream=map_mtconnect(&mt,"urn:uuid:machine-001","STREAM-001",true,&fixture_observations(false)).unwrap();
        let qif=fixture_source(Standard::Qif);
        let mut result=fixture_qif_result(&qif);
        result.measured_at_ns=1_750_002_000;
        let cycle=fixture_cycle(&stream);
        let association=fixture_association(&stream,&qif);
        let join=join_qif_to_cycle(&stream,&cycle,&qif,&result,&association,1_750_000_800,None);
        assert_eq!(join.reason,"QIF_MEASUREMENT_OUTSIDE_CYCLE_WINDOW");
    }

    #[test]
    fn unsupported_schema_and_missing_xsd_validation_reject() {
        let mut mt=fixture_source(Standard::MtConnect);
        mt.schema.version="2.7".into();
        assert!(mt.validate(Standard::MtConnect).is_err());
        let mut qif=fixture_source(Standard::Qif);
        qif.schema.xsd_validation_passed=false;
        assert!(qif.validate(Standard::Qif).is_err());
    }

    #[test]
    fn source_digests_and_extension_payloads_are_preserved() {
        let mt=fixture_source(Standard::MtConnect);
        let mapped=map_mtconnect(&mt,"urn:uuid:machine-001","STREAM-001",true,&fixture_observations(false)).unwrap();
        assert!(mapped.observations.iter().all(|o| o.source_document_sha256==mt.document_sha256));
        assert!(mapped.observations.iter().all(|o| o.extension_count==1));
        let row = mapped.csv_rows()[0].clone();
        assert!(row.contains(&mt.opaque_extensions[0].raw_xml_sha256));
        let mut changed=mt.clone();
        changed.document_sha256=prefix_hash("9");
        changed.opaque_extensions[0].raw_xml_sha256=prefix_hash("8");
        let mapped_changed=map_mtconnect(&changed,"urn:uuid:machine-001","STREAM-001",true,&fixture_observations(false)).unwrap();
        let changed_row=mapped_changed.csv_rows()[0].clone();
        assert!(changed_row.contains(&changed.opaque_extensions[0].raw_xml_sha256));
        assert_ne!(mapped_changed.observations[0].source_document_sha256,mt.document_sha256);
    }

    #[test]
    fn observation_csv_header_and_rows_have_identical_column_counts() {
        let source=fixture_source(Standard::MtConnect);
        let mapped=map_mtconnect(&source,"urn:uuid:machine-001","STREAM-001",true,&fixture_observations(false)).unwrap();
        let header="standard,device_uuid,stream_id,data_item_id,component_id,source_document_id,source_document_sha256,source_uri,sequence,source_timestamp_ns,reference_timestamp_ns,reference_clock_id,clock_mapping_sha256,clock_uncertainty_ns,document_order,native_kind,native_unit,value_text,source_locator,schema_version,schema_namespace,schema_bundle_sha256,parser_id,parser_version,validation_authority_id,validation_receipt_sha256,extension_manifest,diagnostic_manifest";
        let expected_columns=header.split(',').count();
        assert!(mapped.csv_rows().iter().all(|row| row.split(',').count()==expected_columns));
        assert!(mapped.csv_rows()[0].starts_with("MTCONNECT,"));
    }

    #[test]
    fn joined_evidence_csv_has_stable_complete_column_count() {
        let mt=fixture_source(Standard::MtConnect);
        let qif=fixture_source(Standard::Qif);
        let stream=map_mtconnect(&mt,"urn:uuid:machine-001","STREAM-001",true,&fixture_observations(false)).unwrap();
        let cycle=fixture_cycle(&stream);
        let result=fixture_qif_result(&qif);
        let association=fixture_association(&stream,&qif);
        let transform=UnitTransform { source_unit:"mm".into(),target_unit:"um".into(),scale:1000.0,offset:0.0,revision:"UNIT-MAP-1".into() };
        let joined=join_qif_to_cycle(&stream,&cycle,&qif,&result,&association,1_750_000_800,Some(&transform));
        let row=joined.composed.unwrap().csv_row();
        assert_eq!(row.split(',').count(),32);
        assert!(row.contains("QIF-UUID-001"));
        assert!(row.contains("MES-QUALITY-LINKER-01"));
    }

    #[test]
    fn unit_transform_never_discards_the_original_measurement() {
        let transform=UnitTransform {
            source_unit:"in".into(),target_unit:"mm".into(),scale:25.4,offset:0.0,revision:"IN-MM-1".into()
        };
        let converted=apply_unit_transform(1.0,"in",&transform).unwrap();
        assert_eq!(converted.original_value,1.0);
        assert_eq!(converted.original_unit,"in");
        assert!((converted.derived_value-25.4).abs()<1.0e-12);
        assert_eq!(converted.derived_unit,"mm");
        assert!(apply_unit_transform(1.0,"mm",&transform).is_err());
    }

    #[test]
    fn deterministic_mapping_preserves_document_order_and_sorted_sequence() {
        let mt=fixture_source(Standard::MtConnect);
        let mut observations=fixture_observations(false);
        observations.reverse();
        let mapped=map_mtconnect(&mt,"urn:uuid:machine-001","STREAM-001",false,&observations).unwrap();
        assert_eq!(mapped.observations[0].sequence,10);
        assert_eq!(mapped.observations[0].document_order,0);
        assert_eq!(mapped.observations[1].sequence,11);
    }
}
