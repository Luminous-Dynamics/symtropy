//! E4 typed, loss-accounted projections for provider-native evidence records.
//!
//! This is a synthetic-only semantic portability benchmark. DataCite metadata,
//! OGC catalog records and OpenLineage run events remain distinct tagged
//! variants. It never flattens them into a universal record or upgrades source
//! metadata into provider authentication, rights authorization, model truth,
//! or physical resilience.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::process;

const DATACITE: &[u8] = include_bytes!(
    "../../../fixtures/protective-evidence-multiprovider-e3-v0.1/datacite/doi-response.json"
);
const OGC: &[u8] = include_bytes!(
    "../../../fixtures/protective-evidence-multiprovider-e3-v0.1/ogc/record-feature.json"
);
const OPENLINEAGE: &[u8] = include_bytes!(
    "../../../fixtures/protective-evidence-multiprovider-e3-v0.1/openlineage/e3-run-events.json"
);
const E1_CONTEXT: &[u8] = include_bytes!(
    "../../../fixtures/protective-evidence-multiprovider-e3-v0.1/context/e1-envelope-context.json"
);
const E1_STATUS: &str = "NOT_EXECUTED";
const E1_ANALYSIS_ID: &str = "synthetic.analysis.water-continuity.001";
const CLAIM_CEILING: &str = "MULTI_PROVIDER_METADATA_PORTABILITY_ONLY";
const TERMINAL_RULE: &str = "E4-OPENLINEAGE-TERMINAL-STATE-001";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum MappingClass {
    ExactNative,
    NormalizedEquivalent,
    Derived,
    PreservedRawOnly,
    UnknownOrUnsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct MappingDecision {
    provider: String,
    source_path: String,
    target_path: Option<String>,
    classification: MappingClass,
    rule_id: Option<String>,
    note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SourceReceipt {
    provider: String,
    native_profile: String,
    source_identity: String,
    source_path: String,
    source_byte_length: u64,
    source_sha256: String,
    retrieval_mode: String,
    rights_status: String,
    permitted_use: String,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct DataCiteCreator {
    name: String,
    name_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct DataCiteRelatedIdentifier {
    identifier: String,
    identifier_type: String,
    relation_type: String,
    relation_type_information: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct DataCiteRights {
    rights: String,
    rights_uri: Value,
    language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct DataCiteView {
    doi: String,
    title: String,
    publisher: String,
    publication_year: u64,
    resource_type_general: String,
    schema_version_uri: String,
    creators: Vec<DataCiteCreator>,
    related_identifiers: Vec<DataCiteRelatedIdentifier>,
    rights: Vec<DataCiteRights>,
    preserved_json_api_meta: Value,
    preserved_unmapped_attributes: Value,
    native_record: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct OgcLink {
    relation: String,
    href: String,
    media_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct OgcSpatialExtent {
    bbox: [f64; 4],
    crs_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct OgcRecordView {
    record_id: String,
    conforms_to: Vec<String>,
    record_type: String,
    title: String,
    description: String,
    created: String,
    updated: String,
    language: String,
    languages: Vec<String>,
    keywords: Vec<String>,
    geometry: Value,
    spatial_extent: OgcSpatialExtent,
    links: Vec<OgcLink>,
    preserved_property_extensions: Value,
    preserved_unmapped_properties: Value,
    native_record: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct OpenLineageDataset {
    namespace: String,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct OpenLineageRunView {
    run_id: String,
    job_namespace: String,
    job_name: String,
    producer_uri: String,
    run_event_schema_url: String,
    start_event_type: String,
    start_event_time: String,
    terminal_event_type: String,
    derived_terminal_status: String,
    terminal_event_time: String,
    inputs: Vec<OpenLineageDataset>,
    outputs: Vec<OpenLineageDataset>,
    preserved_run_facets: Vec<Value>,
    preserved_native_events: Vec<Value>,
    native_record: Value,
    source_e1_analysis_id: String,
    source_e1_execution_status: String,
    source_e1_relationship: String,
    source_e1_not_source_execution: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "provider", content = "native_view")]
enum ProviderNativeView {
    DataCite(DataCiteView),
    OgcRecords(OgcRecordView),
    OpenLineage(OpenLineageRunView),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ProjectionEnvelope {
    schema_version: String,
    claim_ceiling: String,
    context_source_receipt: SourceReceipt,
    source_receipts: Vec<SourceReceipt>,
    provider_views: Vec<ProviderNativeView>,
    mapping_decisions: Vec<MappingDecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ProjectionReport {
    schema_version: String,
    claim_ceiling: String,
    disposition: String,
    provider_count: usize,
    source_bytes_retained: bool,
    projection_roundtrip_equal: bool,
    mapping_decision_count: usize,
    class_counts: BTreeMap<String, usize>,
    envelope_sha256: String,
    context_source_receipt: SourceReceipt,
    source_receipts: Vec<SourceReceipt>,
    provider_views: Vec<ProviderNativeView>,
    mapping_decisions: Vec<MappingDecision>,
}

fn parse(bytes: &[u8], label: &str) -> Result<Value, String> {
    serde_json::from_slice(bytes).map_err(|error| format!("{label} JSON invalid: {error}"))
}

fn required_text<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| format!("{path}.{key} must be non-empty text"))
}

fn string_array(value: &Value, path: &str) -> Result<Vec<String>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("{path} must be an array"))?
        .iter()
        .enumerate()
        .map(|(index, item)| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{path}[{index}] must be text"))
        })
        .collect()
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn receipt(
    provider: &str,
    profile: &str,
    identity: &str,
    source_path: &str,
    bytes: &[u8],
) -> SourceReceipt {
    SourceReceipt {
        provider: provider.to_owned(),
        native_profile: profile.to_owned(),
        source_identity: identity.to_owned(),
        source_path: source_path.to_owned(),
        source_byte_length: bytes.len() as u64,
        source_sha256: hash(bytes),
        retrieval_mode: "SYNTHETIC_STATIC_FIXTURE".to_owned(),
        rights_status: "SYNTHETIC_OPEN_FIXTURE".to_owned(),
        permitted_use: "SYNTHETIC_TEST_ONLY".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
    }
}

fn decision(
    provider: &str,
    source_path: &str,
    target_path: Option<&str>,
    classification: MappingClass,
    rule_id: Option<&str>,
    note: &str,
) -> MappingDecision {
    MappingDecision {
        provider: provider.to_owned(),
        source_path: source_path.to_owned(),
        target_path: target_path.map(str::to_owned),
        classification,
        rule_id: rule_id.map(str::to_owned),
        note: note.to_owned(),
    }
}

fn project_datacite(value: &Value) -> Result<(ProviderNativeView, Vec<MappingDecision>), String> {
    let data = value
        .get("data")
        .ok_or_else(|| "DataCite data is required".to_owned())?;
    let attrs = data
        .get("attributes")
        .ok_or_else(|| "DataCite attributes are required".to_owned())?;
    let doi_attribute = required_text(attrs, "doi", "DataCite attributes")?;
    let data_id = required_text(data, "id", "DataCite data")?;
    if doi_attribute != data_id {
        return Err("DataCite JSON:API data.id and attributes.doi must match".to_owned());
    }
    let creators = attrs
        .get("creators")
        .and_then(Value::as_array)
        .ok_or_else(|| "DataCite creators must be an array".to_owned())?
        .iter()
        .map(|creator| {
            Ok(DataCiteCreator {
                name: required_text(creator, "name", "DataCite creator")?.to_owned(),
                name_type: creator
                    .get("nameType")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let related_identifiers = attrs
        .get("relatedIdentifiers")
        .and_then(Value::as_array)
        .ok_or_else(|| "DataCite relatedIdentifiers must be an array".to_owned())?
        .iter()
        .map(|item| {
            Ok(DataCiteRelatedIdentifier {
                identifier: required_text(item, "relatedIdentifier", "DataCite relatedIdentifier")?
                    .to_owned(),
                identifier_type: required_text(
                    item,
                    "relatedIdentifierType",
                    "DataCite relatedIdentifier",
                )?
                .to_owned(),
                relation_type: required_text(item, "relationType", "DataCite relatedIdentifier")?
                    .to_owned(),
                relation_type_information: item
                    .get("relationTypeInformation")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let rights = attrs
        .get("rightsList")
        .and_then(Value::as_array)
        .ok_or_else(|| "DataCite rightsList must be an array".to_owned())?
        .iter()
        .map(|item| {
            Ok(DataCiteRights {
                rights: required_text(item, "rights", "DataCite rights")?.to_owned(),
                rights_uri: item.get("rightsUri").cloned().unwrap_or(Value::Null),
                language: item.get("lang").and_then(Value::as_str).map(str::to_owned),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let consumed_attributes = [
        "doi",
        "schemaVersion",
        "types",
        "creators",
        "titles",
        "publisher",
        "publicationYear",
        "relatedIdentifiers",
        "rightsList",
    ];
    let mut preserved_unmapped_attributes = Map::new();
    for (key, item) in attrs
        .as_object()
        .ok_or_else(|| "DataCite attributes must be an object".to_owned())?
    {
        if !consumed_attributes.contains(&key.as_str()) {
            preserved_unmapped_attributes.insert(key.clone(), item.clone());
        }
    }
    let title = attrs
        .get("titles")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .ok_or_else(|| "DataCite titles[0] is required".to_owned())?;
    let data_type = attrs
        .get("types")
        .ok_or_else(|| "DataCite types are required".to_owned())?;
    let view = DataCiteView {
        doi: data_id.to_owned(),
        title: required_text(title, "title", "DataCite title")?.to_owned(),
        publisher: required_text(attrs, "publisher", "DataCite attributes")?.to_owned(),
        publication_year: attrs
            .get("publicationYear")
            .and_then(Value::as_u64)
            .ok_or_else(|| "DataCite publicationYear must be an unsigned integer".to_owned())?,
        resource_type_general: required_text(data_type, "resourceTypeGeneral", "DataCite types")?
            .to_owned(),
        schema_version_uri: required_text(attrs, "schemaVersion", "DataCite attributes")?
            .to_owned(),
        creators,
        related_identifiers,
        rights,
        preserved_json_api_meta: value.get("meta").cloned().unwrap_or(Value::Null),
        preserved_unmapped_attributes: Value::Object(preserved_unmapped_attributes.clone()),
        native_record: value.clone(),
    };
    let mut decisions = vec![
        decision("DATACITE", "/data/id+/data/attributes/doi", Some("/native_view/doi"), MappingClass::ExactNative, None, "JSON:API id and DataCite doi are required to agree; registration/resolution is not asserted."),
        decision("DATACITE", "/data/attributes/titles/0/title", Some("/native_view/title"), MappingClass::ExactNative, None, "Title value copied without semantic reinterpretation."),
        decision("DATACITE", "/data/attributes/creators", Some("/native_view/creators"), MappingClass::ExactNative, None, "Creator name and name type remain provider-native values."),
        decision("DATACITE", "/data/attributes/publisher", Some("/native_view/publisher"), MappingClass::ExactNative, None, "Publisher string is preserved as supplied."),
        decision("DATACITE", "/data/attributes/publicationYear", Some("/native_view/publication_year"), MappingClass::ExactNative, None, "Publication year remains an integer year."),
        decision("DATACITE", "/data/attributes/types/resourceTypeGeneral", Some("/native_view/resource_type_general"), MappingClass::ExactNative, None, "DataCite controlled resource type remains separate from OGC record type."),
        decision("DATACITE", "/data/attributes/relatedIdentifiers", Some("/native_view/related_identifiers"), MappingClass::ExactNative, None, "Identifier type, value, relation type and relationTypeInformation remain distinct."),
        decision("DATACITE", "/data/attributes/rightsList", Some("/native_view/rights"), MappingClass::ExactNative, None, "Rights statement, URI null/unknown and language are preserved; no license grant is inferred."),
        decision("DATACITE", "/data/attributes/schemaVersion", Some("/native_view/schema_version_uri"), MappingClass::ExactNative, None, "Generic kernel-4 URI is retained; the 4.7 minor-version profile remains separate metadata."),
        decision("DATACITE", "/meta/e3FixtureExtension", Some("/native_view/preserved_json_api_meta"), MappingClass::PreservedRawOnly, None, "JSON:API extension is preserved as native JSON rather than coerced into a cross-provider field."),
        decision("DATACITE", "/", Some("/native_view/native_record"), MappingClass::PreservedRawOnly, None, "Complete provider-native JSON:API record is retained to guarantee reconstruction of every unprojected field."),
    ];
    for key in ["identifiers", "subjects", "dates", "language", "descriptions", "url"] {
        if preserved_unmapped_attributes.contains_key(key) {
            decisions.push(decision(
                "DATACITE",
                &format!("/data/attributes/{key}"),
                Some("/native_view/preserved_unmapped_attributes"),
                MappingClass::PreservedRawOnly,
                None,
                "Known native DataCite field is retained raw because E4 does not flatten its provider-specific semantics.",
            ));
        }
    }
    for key in preserved_unmapped_attributes.keys() {
        if !["identifiers", "subjects", "dates", "language", "descriptions", "url"]
            .contains(&key.as_str())
        {
            decisions.push(decision(
                "DATACITE",
                &format!("/data/attributes/{key}"),
                Some("/native_view/preserved_unmapped_attributes"),
                MappingClass::UnknownOrUnsupported,
                None,
                "Unknown provider field is retained in the source-specific projection and is not silently discarded.",
            ));
        }
    }
    Ok((ProviderNativeView::DataCite(view), decisions))
}

fn project_ogc(value: &Value) -> Result<(ProviderNativeView, Vec<MappingDecision>), String> {
    let conforms_to = string_array(
        value.get("conformsTo").ok_or_else(|| "OGC conformsTo is required".to_owned())?,
        "OGC conformsTo",
    )?;
    let properties = value
        .get("properties")
        .ok_or_else(|| "OGC properties are required".to_owned())?;
    let geometry = value
        .get("geometry")
        .cloned()
        .ok_or_else(|| "OGC geometry is required".to_owned())?;
    let spatial = value
        .get("extent")
        .and_then(|extent| extent.get("spatial"))
        .ok_or_else(|| "OGC extent.spatial is required".to_owned())?;
    let bbox_values = spatial
        .get("bbox")
        .and_then(Value::as_array)
        .filter(|items| items.len() == 4)
        .ok_or_else(|| "OGC spatial bbox must contain four coordinates".to_owned())?
        .iter()
        .map(|item| item.as_f64().ok_or_else(|| "OGC bbox values must be numeric".to_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    let bbox: [f64; 4] = bbox_values
        .try_into()
        .map_err(|_| "OGC bbox must contain four values".to_owned())?;
    let links = value
        .get("links")
        .and_then(Value::as_array)
        .ok_or_else(|| "OGC links must be an array".to_owned())?
        .iter()
        .map(|link| {
            Ok(OgcLink {
                relation: required_text(link, "rel", "OGC link")?.to_owned(),
                href: required_text(link, "href", "OGC link")?.to_owned(),
                media_type: link.get("type").and_then(Value::as_str).map(str::to_owned),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let consumed_properties = [
        "type",
        "title",
        "description",
        "created",
        "updated",
        "language",
        "languages",
        "keywords",
        "e3Extension",
    ];
    let mut preserved_unmapped_properties = Map::new();
    for (key, item) in properties
        .as_object()
        .ok_or_else(|| "OGC properties must be an object".to_owned())?
    {
        if !consumed_properties.contains(&key.as_str()) {
            preserved_unmapped_properties.insert(key.clone(), item.clone());
        }
    }
    let view = OgcRecordView {
        record_id: required_text(value, "id", "OGC Feature")?.to_owned(),
        conforms_to,
        record_type: required_text(properties, "type", "OGC properties")?.to_owned(),
        title: required_text(properties, "title", "OGC properties")?.to_owned(),
        description: required_text(properties, "description", "OGC properties")?.to_owned(),
        created: required_text(properties, "created", "OGC properties")?.to_owned(),
        updated: required_text(properties, "updated", "OGC properties")?.to_owned(),
        language: required_text(properties, "language", "OGC properties")?.to_owned(),
        languages: string_array(
            properties.get("languages").ok_or_else(|| "OGC languages are required".to_owned())?,
            "OGC properties.languages",
        )?,
        keywords: string_array(
            properties.get("keywords").ok_or_else(|| "OGC keywords are required".to_owned())?,
            "OGC properties.keywords",
        )?,
        geometry,
        spatial_extent: OgcSpatialExtent {
            bbox,
            crs_uri: required_text(spatial, "crs", "OGC spatial extent")?.to_owned(),
        },
        links,
        preserved_property_extensions: properties
            .get("e3Extension")
            .cloned()
            .unwrap_or(Value::Null),
        preserved_unmapped_properties: Value::Object(preserved_unmapped_properties.clone()),
        native_record: value.clone(),
    };
    let mut decisions = vec![
        decision("OGC_API_RECORDS", "/id", Some("/native_view/record_id"), MappingClass::ExactNative, None, "Server-assigned catalog record identity remains a record ID, not a DOI or payload digest."),
        decision("OGC_API_RECORDS", "/conformsTo", Some("/native_view/conforms_to"), MappingClass::ExactNative, None, "Selected conformance classes remain explicit rather than being inferred from content."),
        decision("OGC_API_RECORDS", "/properties/type", Some("/native_view/record_type"), MappingClass::ExactNative, None, "Catalog record type is distinct from DataCite resourceTypeGeneral."),
        decision("OGC_API_RECORDS", "/properties/title", Some("/native_view/title"), MappingClass::ExactNative, None, "Catalog title preserved as supplied."),
        decision("OGC_API_RECORDS", "/properties/description", Some("/native_view/description"), MappingClass::ExactNative, None, "Catalog description is preserved as descriptive text, not payload content."),
        decision("OGC_API_RECORDS", "/properties/created", Some("/native_view/created"), MappingClass::ExactNative, None, "Record creation timestamp preserved without timezone normalization."),
        decision("OGC_API_RECORDS", "/properties/updated", Some("/native_view/updated"), MappingClass::ExactNative, None, "Record update timestamp preserved without timezone normalization."),
        decision("OGC_API_RECORDS", "/properties/language", Some("/native_view/language"), MappingClass::ExactNative, None, "Primary metadata language is preserved."),
        decision("OGC_API_RECORDS", "/properties/languages", Some("/native_view/languages"), MappingClass::ExactNative, None, "Additional metadata languages remain a list."),
        decision("OGC_API_RECORDS", "/properties/keywords", Some("/native_view/keywords"), MappingClass::ExactNative, None, "Catalog keywords remain a list of terms."),
        decision("OGC_API_RECORDS", "/geometry", Some("/native_view/geometry"), MappingClass::ExactNative, None, "GeoJSON geometry remains a geometry object; it is not converted into a dataset payload."),
        decision("OGC_API_RECORDS", "/extent/spatial/bbox", Some("/native_view/spatial_extent/bbox"), MappingClass::NormalizedEquivalent, Some("E4-OGC-NUMERIC-BBOX-001"), "Numeric coordinates are represented as f64 values; CRS URI is preserved unchanged and no reprojection occurs."),
        decision("OGC_API_RECORDS", "/extent/spatial/crs", Some("/native_view/spatial_extent/crs_uri"), MappingClass::ExactNative, None, "CRS declaration remains an explicit URI, not an inferred coordinate system."),
        decision("OGC_API_RECORDS", "/links", Some("/native_view/links"), MappingClass::ExactNative, None, "Self, collection and profile links retain their native relations and targets."),
        decision("OGC_API_RECORDS", "/properties/e3Extension", Some("/native_view/preserved_property_extensions"), MappingClass::PreservedRawOnly, None, "Foreign property extension remains native JSON and is not flattened."),
        decision("OGC_API_RECORDS", "/", Some("/native_view/native_record"), MappingClass::PreservedRawOnly, None, "Complete provider-native catalog record is retained so unprojected top-level/profile fields survive."),
    ];
    for key in ["themes", "contacts", "providers"] {
        if preserved_unmapped_properties.contains_key(key) {
            decisions.push(decision(
                "OGC_API_RECORDS",
                &format!("/properties/{key}"),
                Some("/native_view/preserved_unmapped_properties"),
                MappingClass::PreservedRawOnly,
                None,
                "Known OGC catalog field is retained raw because this projection does not reinterpret its profile-specific structure.",
            ));
        }
    }
    for key in preserved_unmapped_properties.keys() {
        if !["themes", "contacts", "providers"].contains(&key.as_str()) {
            decisions.push(decision(
                "OGC_API_RECORDS",
                &format!("/properties/{key}"),
                Some("/native_view/preserved_unmapped_properties"),
                MappingClass::UnknownOrUnsupported,
                None,
                "Unknown catalog property is retained in the native projection and is not silently dropped.",
            ));
        }
    }
    Ok((ProviderNativeView::OgcRecords(view), decisions))
}

fn project_openlineage(
    value: &Value,
    e1_context: &Value,
) -> Result<(ProviderNativeView, Vec<MappingDecision>), String> {
    let events = value
        .get("events")
        .and_then(Value::as_array)
        .filter(|items| items.len() == 2)
        .ok_or_else(|| "OpenLineage must contain the two fixture events".to_owned())?;
    let start = &events[0];
    let terminal = &events[1];
    if required_text(start, "eventType", "OpenLineage first event")? != "START" {
        return Err("OpenLineage first event must be START".to_owned());
    }
    let terminal_event_type = required_text(terminal, "eventType", "OpenLineage terminal event")?;
    if !["COMPLETE", "FAIL", "ABORT"].contains(&terminal_event_type) {
        return Err(format!("unsupported OpenLineage terminal event type: {terminal_event_type}"));
    }
    let start_time = required_text(start, "eventTime", "OpenLineage START")?;
    let terminal_time = required_text(terminal, "eventTime", "OpenLineage terminal")?;
    if terminal_time <= start_time {
        return Err("OpenLineage terminal event must follow START".to_owned());
    }
    let start_run = start.get("run").ok_or_else(|| "OpenLineage START run missing".to_owned())?;
    let terminal_run = terminal.get("run").ok_or_else(|| "OpenLineage terminal run missing".to_owned())?;
    let job = start.get("job").ok_or_else(|| "OpenLineage job missing".to_owned())?;
    let inputs = start
        .get("inputs")
        .and_then(Value::as_array)
        .ok_or_else(|| "OpenLineage inputs must be an array".to_owned())?
        .iter()
        .map(|dataset| {
            Ok(OpenLineageDataset {
                namespace: required_text(dataset, "namespace", "OpenLineage input")?.to_owned(),
                name: required_text(dataset, "name", "OpenLineage input")?.to_owned(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let outputs = start
        .get("outputs")
        .and_then(Value::as_array)
        .ok_or_else(|| "OpenLineage outputs must be an array".to_owned())?
        .iter()
        .map(|dataset| {
            Ok(OpenLineageDataset {
                namespace: required_text(dataset, "namespace", "OpenLineage output")?.to_owned(),
                name: required_text(dataset, "name", "OpenLineage output")?.to_owned(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let start_facet = start_run
        .get("facets")
        .and_then(|facets| facets.get("e3_evidence_context"))
        .cloned()
        .ok_or_else(|| "OpenLineage START E3 facet missing".to_owned())?;
    let terminal_facet = terminal_run
        .get("facets")
        .and_then(|facets| facets.get("e3_evidence_context"))
        .cloned()
        .ok_or_else(|| "OpenLineage terminal E3 facet missing".to_owned())?;
    let e1_analysis = e1_context
        .get("analysis")
        .ok_or_else(|| "E1 context analysis is required".to_owned())?;
    let e1_analysis_id = required_text(e1_analysis, "analysis_id", "E1 context")?;
    let e1_execution_status = required_text(e1_analysis, "execution_status", "E1 context")?;
    if e1_execution_status != E1_STATUS || !e1_analysis.get("run_id").is_some_and(Value::is_null) {
        return Err("E4 E1 context must remain NOT_EXECUTED with null run_id".to_owned());
    }
    if required_text(&start_facet, "sourceE1AnalysisId", "OpenLineage E3 facet")?
        != e1_analysis_id
        || required_text(&start_facet, "sourceE1ExecutionStatus", "OpenLineage E3 facet")?
            != e1_execution_status
        || required_text(&start_facet, "sourceBinding", "OpenLineage E3 facet")?
            != "CONTEXT_ONLY"
        || start_facet.get("notASourceExecution").and_then(Value::as_bool) != Some(true)
    {
        return Err("OpenLineage E3 facet must match the exact E1 NOT_EXECUTED source context".to_owned());
    }
    let view = OpenLineageRunView {
        run_id: required_text(start_run, "runId", "OpenLineage START run")?.to_owned(),
        job_namespace: required_text(job, "namespace", "OpenLineage job")?.to_owned(),
        job_name: required_text(job, "name", "OpenLineage job")?.to_owned(),
        producer_uri: required_text(start, "producer", "OpenLineage START")?.to_owned(),
        run_event_schema_url: required_text(start, "schemaURL", "OpenLineage START")?.to_owned(),
        start_event_type: required_text(start, "eventType", "OpenLineage START")?.to_owned(),
        start_event_time: required_text(start, "eventTime", "OpenLineage START")?.to_owned(),
        terminal_event_type: terminal_event_type.to_owned(),
        derived_terminal_status: match terminal_event_type {
            "COMPLETE" => "COMPLETED",
            "FAIL" => "FAILED",
            "ABORT" => "ABORTED",
            other => return Err(format!("unsupported OpenLineage terminal event type: {other}")),
        }
        .to_owned(),
        terminal_event_time: required_text(terminal, "eventTime", "OpenLineage terminal")?.to_owned(),
        inputs,
        outputs,
        preserved_run_facets: vec![start_facet.clone(), terminal_facet.clone()],
        preserved_native_events: events.to_vec(),
        native_record: value.clone(),
        source_e1_analysis_id: e1_analysis_id.to_owned(),
        source_e1_execution_status: e1_execution_status.to_owned(),
        source_e1_relationship: required_text(&start_facet, "sourceBinding", "OpenLineage facet")?
            .to_owned(),
        source_e1_not_source_execution: start_facet
            .get("notASourceExecution")
            .and_then(Value::as_bool)
            .ok_or_else(|| "OpenLineage E3 facet notASourceExecution must be boolean".to_owned())?,
    };
    if required_text(start_run, "runId", "OpenLineage START run")?
        != required_text(terminal_run, "runId", "OpenLineage terminal run")?
        || start_facet != terminal_facet
    {
        return Err("OpenLineage projection requires stable run identity and E3 context facet".to_owned());
    }
    let mut decisions = vec![
        decision("OPENLINEAGE", "/events/0/run/runId", Some("/native_view/run_id"), MappingClass::ExactNative, None, "Run ID identifies the synthetic E3 adapter-test execution only."),
        decision("OPENLINEAGE", "/events/0/job", Some("/native_view/job_namespace+job_name"), MappingClass::ExactNative, None, "Job namespace and name stay distinct from a dataset or DOI."),
        decision("OPENLINEAGE", "/events/0/producer", Some("/native_view/producer_uri"), MappingClass::ExactNative, None, "Producer URI is kept distinct from the job and run identities."),
        decision("OPENLINEAGE", "/events/0/schemaURL", Some("/native_view/run_event_schema_url"), MappingClass::ExactNative, None, "Exact OpenLineage schema URL is retained."),
        decision("OPENLINEAGE", "/events/0/eventTime", Some("/native_view/start_event_time"), MappingClass::ExactNative, None, "START observation timestamp remains the exact source string."),
        decision("OPENLINEAGE", "/events/1/eventTime", Some("/native_view/terminal_event_time"), MappingClass::ExactNative, None, "Terminal observation timestamp remains the exact source string."),
        decision("OPENLINEAGE", "/events/0/inputs", Some("/native_view/inputs"), MappingClass::ExactNative, None, "Input dataset namespace and name remain distinct values."),
        decision("OPENLINEAGE", "/events/0/outputs", Some("/native_view/outputs"), MappingClass::ExactNative, None, "Output dataset namespace and name remain distinct values."),
        decision("OPENLINEAGE", "/events/*/run/facets/e3_evidence_context/sourceE1AnalysisId", Some("/native_view/source_e1_analysis_id"), MappingClass::ExactNative, None, "E1 analysis identity is cross-checked against the exact E1 context receipt."),
        decision("OPENLINEAGE", "/events/*/run/facets/e3_evidence_context/sourceE1ExecutionStatus", Some("/native_view/source_e1_execution_status"), MappingClass::ExactNative, None, "E1 NOT_EXECUTED status is retained and is not converted into an E3 run state."),
        decision("OPENLINEAGE", "/events/*/run/facets/e3_evidence_context/sourceBinding", Some("/native_view/source_e1_relationship"), MappingClass::ExactNative, None, "Context-only relationship remains explicit."),
        decision("OPENLINEAGE", "/events/0/eventType", Some("/native_view/start_event_type"), MappingClass::ExactNative, None, "START is the directly observed run event, not a predicted state."),
        decision("OPENLINEAGE", "/events/1/eventType", Some("/native_view/terminal_event_type"), MappingClass::ExactNative, None, "COMPLETE is the directly observed terminal event, retained separately from a derived lifecycle label."),
        decision("OPENLINEAGE", "/events/0/eventType+/events/1/eventType", Some("/native_view/derived_terminal_status"), MappingClass::Derived, Some(TERMINAL_RULE), "A local lifecycle label is derived from the observed START and terminal events; it does not describe the E1 model run."),
        decision("OPENLINEAGE", "/events/*/run/facets/e3_evidence_context/notASourceExecution", Some("/native_view/source_e1_not_source_execution"), MappingClass::ExactNative, None, "Explicit boolean prevents the embedded E1 context from being interpreted as this OpenLineage run."),
        decision("OPENLINEAGE", "/events/*/run/facets/e3_evidence_context", Some("/native_view/preserved_run_facets"), MappingClass::PreservedRawOnly, None, "Custom facet is retained as native facet JSON, including E1 NOT_EXECUTED context-only assertion."),
        decision("OPENLINEAGE", "/events/*", Some("/native_view/preserved_native_events"), MappingClass::PreservedRawOnly, None, "Complete native event objects remain available so unprojected provider fields survive the typed projection."),
        decision("OPENLINEAGE", "/", Some("/native_view/native_record"), MappingClass::PreservedRawOnly, None, "Complete native event bundle is retained, including any future top-level provider fields."),
    ];
    for (index, event) in events.iter().enumerate() {
        if let Some(facets) = event
            .get("run")
            .and_then(|run| run.get("facets"))
            .and_then(Value::as_object)
        {
            for key in facets.keys().filter(|key| key.as_str() != "e3_evidence_context") {
                decisions.push(decision(
                    "OPENLINEAGE",
                    &format!("/events/{index}/run/facets/{key}"),
                    Some("/native_view/preserved_native_events"),
                    MappingClass::UnknownOrUnsupported,
                    None,
                    "Unknown RunFacet is retained in the native event snapshot and not assigned guessed cross-provider semantics.",
                ));
            }
        }
    }
    Ok((ProviderNativeView::OpenLineage(view), decisions))
}

fn build_envelope(
    datacite: &Value,
    ogc: &Value,
    openlineage: &Value,
) -> Result<ProjectionEnvelope, String> {
    build_envelope_with_source_bytes(datacite, ogc, openlineage, [DATACITE, OGC, OPENLINEAGE])
}

fn build_envelope_with_source_bytes(
    datacite: &Value,
    ogc: &Value,
    openlineage: &Value,
    source_bytes: [&[u8]; 3],
) -> Result<ProjectionEnvelope, String> {
    let (datacite_view, mut mapping_decisions) = project_datacite(datacite)?;
    let (ogc_view, ogc_decisions) = project_ogc(ogc)?;
    let e1_context = parse(E1_CONTEXT, "E1 context")?;
    let (lineage_view, lineage_decisions) = project_openlineage(openlineage, &e1_context)?;
    mapping_decisions.extend(ogc_decisions);
    mapping_decisions.extend(lineage_decisions);
    let datacite_identity = match &datacite_view {
        ProviderNativeView::DataCite(view) => view.doi.clone(),
        _ => unreachable!("DataCite projection constructor"),
    };
    let ogc_identity = match &ogc_view {
        ProviderNativeView::OgcRecords(view) => view.record_id.clone(),
        _ => unreachable!("OGC projection constructor"),
    };
    let lineage_identity = match &lineage_view {
        ProviderNativeView::OpenLineage(view) => view.run_id.clone(),
        _ => unreachable!("OpenLineage projection constructor"),
    };
    let views = vec![datacite_view, ogc_view, lineage_view];
    let e1_context = parse(E1_CONTEXT, "E1 source context")?;
    let e1_analysis_id = required_text(
        e1_context.get("analysis").ok_or_else(|| "E1 context analysis missing".to_owned())?,
        "analysis_id",
        "E1 context analysis",
    )?;
    Ok(ProjectionEnvelope {
        schema_version: "protective-evidence-semantic-projection/0.1".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        context_source_receipt: receipt(
            "E1_CONTEXT",
            "Protective Systems E1 evidence envelope (NOT_EXECUTED context only)",
            e1_analysis_id,
            "context/e1-envelope-context.json",
            E1_CONTEXT,
        ),
        source_receipts: vec![
            receipt(
                "DATACITE",
                "DataCite REST JSON:API / Metadata Schema 4.7 selected field profile",
                &datacite_identity,
                "datacite/doi-response.json",
                source_bytes[0],
            ),
            receipt(
                "OGC_API_RECORDS",
                "OGC API - Records Part 1 Core 1.0 GeoJSON",
                &ogc_identity,
                "ogc/record-feature.json",
                source_bytes[1],
            ),
            receipt(
                "OPENLINEAGE",
                "OpenLineage RunEvent 2.0.2",
                &lineage_identity,
                "openlineage/e3-run-events.json",
                source_bytes[2],
            ),
        ],
        provider_views: views,
        mapping_decisions,
    })
}

fn mapping_class_name(class: &MappingClass) -> &'static str {
    match class {
        MappingClass::ExactNative => "EXACT_NATIVE",
        MappingClass::NormalizedEquivalent => "NORMALIZED_EQUIVALENT",
        MappingClass::Derived => "DERIVED",
        MappingClass::PreservedRawOnly => "PRESERVED_RAW_ONLY",
        MappingClass::UnknownOrUnsupported => "UNKNOWN_OR_UNSUPPORTED",
    }
}

fn build_report() -> Result<ProjectionReport, String> {
    let datacite = parse(DATACITE, "DataCite")?;
    let ogc = parse(OGC, "OGC Records")?;
    let openlineage = parse(OPENLINEAGE, "OpenLineage")?;
    let envelope = build_envelope(&datacite, &ogc, &openlineage)?;
    let encoded = serde_json::to_vec(&envelope)
        .map_err(|error| format!("semantic projection serialization failed: {error}"))?;
    let decoded: ProjectionEnvelope = serde_json::from_slice(&encoded)
        .map_err(|error| format!("semantic projection deserialization failed: {error}"))?;
    let roundtrip_equal = decoded == envelope;
    let mut class_counts = BTreeMap::new();
    for mapping in &envelope.mapping_decisions {
        *class_counts
            .entry(mapping_class_name(&mapping.classification).to_owned())
            .or_insert(0usize) += 1;
    }
    let source_bytes_retained = envelope.source_receipts.iter().all(|receipt| {
        receipt.source_byte_length > 0
            && receipt.source_sha256.len() == 64
            && receipt.retrieval_mode == "SYNTHETIC_STATIC_FIXTURE"
    }) && envelope.context_source_receipt.source_byte_length == E1_CONTEXT.len() as u64
        && envelope.context_source_receipt.source_sha256 == hash(E1_CONTEXT);
    Ok(ProjectionReport {
        schema_version: "protective-evidence-semantic-projection/0.1".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        disposition: "ABSTAIN_SYNTHETIC_METADATA_ONLY".to_owned(),
        provider_count: envelope.provider_views.len(),
        source_bytes_retained,
        projection_roundtrip_equal: roundtrip_equal,
        mapping_decision_count: envelope.mapping_decisions.len(),
        class_counts,
        envelope_sha256: hash(&encoded),
        context_source_receipt: envelope.context_source_receipt,
        source_receipts: envelope.source_receipts,
        provider_views: envelope.provider_views,
        mapping_decisions: envelope.mapping_decisions,
    })
}

fn roundtrip<T>(value: &T) -> Result<bool, String>
where
    T: Serialize + DeserializeOwned + PartialEq,
{
    let encoded =
        serde_json::to_vec(value).map_err(|error| format!("projection encode failed: {error}"))?;
    let decoded: T = serde_json::from_slice(&encoded)
        .map_err(|error| format!("projection decode failed: {error}"))?;
    Ok(decoded == *value)
}

fn main() {
    match build_report() {
        Ok(report) if report.projection_roundtrip_equal && report.source_bytes_retained => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("E4 report serialization")
            );
        }
        Ok(report) => {
            eprintln!("E4 semantic projection integrity failed");
            eprintln!("{}", serde_json::to_string_pretty(&report).expect("report"));
            process::exit(2);
        }
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parsed_fixtures() -> (Value, Value, Value) {
        (
            parse(DATACITE, "DataCite").expect("DataCite fixture"),
            parse(OGC, "OGC").expect("OGC fixture"),
            parse(OPENLINEAGE, "OpenLineage").expect("OpenLineage fixture"),
        )
    }

    #[test]
    fn all_provider_views_roundtrip_without_semantic_change() {
        let report = build_report().expect("E4 projection report");
        assert!(report.projection_roundtrip_equal);
        assert!(report.source_bytes_retained);
        assert_eq!(report.provider_count, 3);
        assert_eq!(report.disposition, "ABSTAIN_SYNTHETIC_METADATA_ONLY");
        assert_eq!(report.class_counts.get("EXACT_NATIVE"), Some(&35));
        assert_eq!(report.class_counts.get("NORMALIZED_EQUIVALENT"), Some(&1));
        assert_eq!(report.class_counts.get("DERIVED"), Some(&1));
        assert_eq!(report.class_counts.get("PRESERVED_RAW_ONLY"), Some(&10));
    }

    #[test]
    fn provider_views_remain_a_tagged_union_not_a_flat_record() {
        let report = build_report().expect("report");
        assert!(matches!(&report.provider_views[0], ProviderNativeView::DataCite(_)));
        assert!(matches!(&report.provider_views[1], ProviderNativeView::OgcRecords(_)));
        assert!(matches!(&report.provider_views[2], ProviderNativeView::OpenLineage(_)));
    }

    #[test]
    fn datacite_identity_rights_and_schema_uri_remain_distinct() {
        let report = build_report().expect("report");
        let ProviderNativeView::DataCite(view) = &report.provider_views[0] else {
            panic!("DataCite view expected")
        };
        assert_eq!(view.doi, "10.5555/synthetic-e3-protection-record-001");
        assert_eq!(view.schema_version_uri, "http://datacite.org/schema/kernel-4");
        assert_eq!(view.rights.len(), 1);
        assert_eq!(view.rights[0].rights_uri, Value::Null);
        assert_eq!(
            view.related_identifiers[0].identifier,
            "swh:1:cnt:0000000000000000000000000000000000000000"
        );
        assert!(roundtrip(view).expect("DataCite view roundtrip"));
    }

    #[test]
    fn datacite_id_and_doi_attribute_must_agree_before_projection() {
        let (mut datacite, _, _) = parsed_fixtures();
        datacite["data"]["attributes"]["doi"] = Value::String("10.5555/different".into());
        let error = project_datacite(&datacite).expect_err("identity mismatch");
        assert!(error.contains("data.id and attributes.doi must match"));
    }

    #[test]
    fn datacite_meta_extension_is_preserved_as_raw_native_json() {
        let (datacite, ogc, lineage) = parsed_fixtures();
        let (view, decisions) = project_datacite(&datacite).expect("DataCite view");
        let ProviderNativeView::DataCite(view) = view else {
            panic!("DataCite view expected")
        };
        assert_eq!(
            view.preserved_json_api_meta["e3FixtureExtension"]["preservationMode"],
            "retain-unmapped-source-fields"
        );
        assert!(decisions.iter().any(|item| {
            item.source_path == "/meta/e3FixtureExtension"
                && item.classification == MappingClass::PreservedRawOnly
        }));
        let _ = (ogc, lineage);
    }

    #[test]
    fn datacite_unrecognized_meta_extension_survives_projection_roundtrip() {
        let (mut datacite, ogc, lineage) = parsed_fixtures();
        datacite["meta"]["futureProviderField"] = Value::String("retain-me".into());
        let (view, _) = project_datacite(&datacite).expect("DataCite view");
        let encoded = serde_json::to_vec(&view).expect("encode");
        let decoded: DataCiteView = serde_json::from_slice(&encoded).expect("decode");
        assert_eq!(
            decoded.preserved_json_api_meta["futureProviderField"],
            "retain-me"
        );
        let _ = (ogc, lineage);
    }

    #[test]
    fn ogc_record_view_keeps_record_extent_and_crs_explicit() {
        let report = build_report().expect("report");
        let ProviderNativeView::OgcRecords(view) = &report.provider_views[1] else {
            panic!("OGC view expected")
        };
        assert_eq!(view.record_id, "e3-synthetic-record-001");
        assert_eq!(view.spatial_extent.crs_uri, "http://www.opengis.net/def/crs/OGC/1.3/CRS84");
        assert_eq!(view.spatial_extent.bbox, [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(view.geometry["type"], "Polygon");
        assert!(roundtrip(view).expect("OGC projection roundtrip"));
    }

    #[test]
    fn ogc_conformance_classes_remain_explicit_in_the_typed_view() {
        let report = build_report().expect("report");
        let ProviderNativeView::OgcRecords(view) = &report.provider_views[1] else {
            panic!("OGC view expected")
        };
        assert!(view.conforms_to.iter().any(|value| {
            value == "http://www.opengis.net/spec/ogcapi-records-1/1.0/conf/core"
        }));
        assert!(view.conforms_to.iter().any(|value| {
            value == "http://www.opengis.net/spec/ogcapi-records-1/1.0/conf/json"
        }));
    }

    #[test]
    fn ogc_foreign_property_extension_survives_roundtrip() {
        let (datacite, mut ogc, lineage) = parsed_fixtures();
        ogc["properties"]["e3Extension"]["futureNested"] = json!({"preserve": true});
        let (view, decisions) = project_ogc(&ogc).expect("OGC view");
        let ProviderNativeView::OgcRecords(view) = view else {
            panic!("OGC view expected")
        };
        assert!(
            view.preserved_property_extensions["futureNested"]["preserve"]
                .as_bool()
                .expect("preserve flag")
        );
        assert!(decisions.iter().any(|item| {
            item.classification == MappingClass::PreservedRawOnly
                && item.source_path == "/properties/e3Extension"
        }));
        let _ = (datacite, lineage);
    }

    #[test]
    fn openlineage_start_event_must_be_start() {
        let (datacite, ogc, mut lineage) = parsed_fixtures();
        let e1 = parse(E1_CONTEXT, "E1 context").expect("E1 context");
        lineage["events"][0]["eventType"] = Value::String("COMPLETE".into());
        let error = project_openlineage(&lineage, &e1).expect_err("invalid initial event");
        assert!(error.contains("first event must be START"));
        let _ = (datacite, ogc);
    }

    #[test]
    fn openlineage_terminal_event_must_be_after_start() {
        let (datacite, ogc, mut lineage) = parsed_fixtures();
        let e1 = parse(E1_CONTEXT, "E1 context").expect("E1 context");
        lineage["events"][1]["eventTime"] =
            Value::String("2026-11-16T09:59:59.000Z".into());
        let error = project_openlineage(&lineage, &e1).expect_err("terminal before start");
        assert!(error.contains("terminal event must follow START"));
        let _ = (datacite, ogc);
    }

    #[test]
    fn openlineage_terminal_state_is_declared_as_derived_not_source_native() {
        let report = build_report().expect("report");
        let ProviderNativeView::OpenLineage(view) = &report.provider_views[2] else {
            panic!("OpenLineage view expected")
        };
        assert_eq!(view.start_event_type, "START");
        assert_eq!(view.terminal_event_type, "COMPLETE");
        assert_eq!(view.derived_terminal_status, "COMPLETED");
        assert!(view.source_e1_not_source_execution);
        assert_eq!(view.source_e1_execution_status, "NOT_EXECUTED");
        assert_eq!(view.source_e1_relationship, "CONTEXT_ONLY");
        assert!(report.mapping_decisions.iter().any(|item| {
            item.provider == "OPENLINEAGE"
                && item.classification == MappingClass::Derived
                && item.rule_id.as_deref() == Some(TERMINAL_RULE)
        }));
        assert!(roundtrip(view).expect("OpenLineage view roundtrip"));
    }

    #[test]
    fn openlineage_terminal_event_is_preserved_separately_from_derived_status() {
        let report = build_report().expect("report");
        let ProviderNativeView::OpenLineage(view) = &report.provider_views[2] else {
            panic!("OpenLineage view expected")
        };
        assert_eq!(view.terminal_event_type, "COMPLETE");
        assert_eq!(view.derived_terminal_status, "COMPLETED");
        assert_ne!(view.terminal_event_type, view.derived_terminal_status);
        assert!(report.mapping_decisions.iter().any(|item| {
            item.source_path == "/events/1/eventType"
                && item.classification == MappingClass::ExactNative
        }));
        assert!(report.mapping_decisions.iter().any(|item| {
            item.source_path == "/events/0/eventType+/events/1/eventType"
                && item.classification == MappingClass::Derived
        }));
    }

    #[test]
    fn e4_context_receipt_binds_exact_e1_envelope_bytes() {
        let report = build_report().expect("report");
        assert_eq!(report.context_source_receipt.provider, "E1_CONTEXT");
        assert_eq!(report.context_source_receipt.source_identity, E1_ANALYSIS_ID);
        assert_eq!(report.context_source_receipt.source_byte_length, E1_CONTEXT.len() as u64);
        assert_eq!(report.context_source_receipt.source_sha256, hash(E1_CONTEXT));
        assert_eq!(
            report.context_source_receipt.native_profile,
            "Protective Systems E1 evidence envelope (NOT_EXECUTED context only)"
        );
    }

    #[test]
    fn openlineage_facet_cannot_change_the_embedded_e1_analysis_identity() {
        let (datacite, ogc, mut lineage) = parsed_fixtures();
        lineage["events"][0]["run"]["facets"]["e3_evidence_context"]
            ["sourceE1AnalysisId"] = Value::String("different-analysis".into());
        let error = project_openlineage(
            &lineage,
            &parse(E1_CONTEXT, "E1 context").expect("E1 context"),
        )
        .expect_err("E1 identity mismatch");
        assert!(error.contains("must match the exact E1 NOT_EXECUTED source context"));
        let _ = (datacite, ogc);
    }

    #[test]
    fn openlineage_event_does_not_promote_e1_source_analysis_to_a_run() {
        let report = build_report().expect("report");
        let ProviderNativeView::OpenLineage(view) = &report.provider_views[2] else {
            panic!("OpenLineage view expected")
        };
        assert_ne!(view.run_id, view.source_e1_analysis_id);
        assert_eq!(view.source_e1_execution_status, E1_STATUS);
        assert_eq!(view.source_e1_relationship, "CONTEXT_ONLY");
    }

    #[test]
    fn normalized_equivalent_mapping_is_explicit_for_bbox_numbers() {
        let report = build_report().expect("report");
        let bbox = report
            .mapping_decisions
            .iter()
            .find(|item| item.source_path == "/extent/spatial/bbox")
            .expect("bbox mapping decision");
        assert_eq!(bbox.classification, MappingClass::NormalizedEquivalent);
        assert_eq!(
            bbox.rule_id.as_deref(),
            Some("E4-OGC-NUMERIC-BBOX-001")
        );
    }

    #[test]
    fn no_mapping_decision_claims_license_authenticity() {
        let report = build_report().expect("report");
        let datacite = report.source_receipts.iter().find(|item| item.provider == "DATACITE").expect("receipt");
        assert_eq!(datacite.rights_status, "SYNTHETIC_OPEN_FIXTURE");
        assert_eq!(datacite.permitted_use, "SYNTHETIC_TEST_ONLY");
        assert!(!datacite.claim_ceiling.contains("LICENSE"));
    }

    #[test]
    fn input_json_key_order_and_whitespace_do_not_change_semantics_but_change_byte_digest() {
        let (datacite, ogc, lineage) = parsed_fixtures();
        let envelope_a = build_envelope(&datacite, &ogc, &lineage).expect("baseline");
        let compact_datacite = serde_json::to_string(&datacite).expect("compact JSON");
        let reparsed_datacite = parse(compact_datacite.as_bytes(), "compact DataCite").expect("parse");
        let envelope_b = build_envelope_with_source_bytes(
            &reparsed_datacite,
            &ogc,
            &lineage,
            [compact_datacite.as_bytes(), OGC, OPENLINEAGE],
        )
        .expect("reproject compact JSON");
        assert_eq!(envelope_a.provider_views, envelope_b.provider_views);
        assert_eq!(envelope_a.mapping_decisions, envelope_b.mapping_decisions);
        assert_ne!(
            envelope_a.source_receipts[0].source_sha256,
            envelope_b.source_receipts[0].source_sha256,
            "raw source digest remains byte-sensitive"
        );
    }

    #[test]
    fn unknown_native_values_are_not_defaulted_or_silently_discarded() {
        let (mut datacite, ogc, lineage) = parsed_fixtures();
        datacite["meta"]["providerUnknown"]["explicitUnknown"] = Value::Null;
        let (view, _) = project_datacite(&datacite).expect("DataCite projection");
        let ProviderNativeView::DataCite(view) = view else {
            panic!("DataCite projection expected")
        };
        assert_eq!(view.preserved_json_api_meta["providerUnknown"]["explicitUnknown"], Value::Null);
        let _ = (ogc, lineage);
    }

    #[test]
    fn unknown_datacite_attribute_is_preserved_and_classified() {
        let (mut datacite, ogc, lineage) = parsed_fixtures();
        datacite["data"]["attributes"]["futureProviderField"] =
            Value::String("keep-native-only".into());
        let (view, decisions) = project_datacite(&datacite).expect("DataCite projection");
        let ProviderNativeView::DataCite(view) = view else {
            panic!("DataCite view expected")
        };
        assert_eq!(
            view.preserved_unmapped_attributes["futureProviderField"],
            "keep-native-only"
        );
        assert!(decisions.iter().any(|item| {
            item.source_path == "/data/attributes/futureProviderField"
                && item.classification == MappingClass::UnknownOrUnsupported
                && item.target_path.as_deref()
                    == Some("/native_view/preserved_unmapped_attributes")
        }));
        assert!(roundtrip(&view).expect("DataCite unknown field roundtrip"));
        let _ = (ogc, lineage);
    }

    #[test]
    fn unknown_ogc_property_is_preserved_and_classified() {
        let (datacite, mut ogc, lineage) = parsed_fixtures();
        ogc["properties"]["futureCatalogProperty"] =
            json!({"keep": "native"});
        let (view, decisions) = project_ogc(&ogc).expect("OGC projection");
        let ProviderNativeView::OgcRecords(view) = view else {
            panic!("OGC view expected")
        };
        assert_eq!(
            view.preserved_unmapped_properties["futureCatalogProperty"]["keep"],
            "native"
        );
        assert!(decisions.iter().any(|item| {
            item.source_path == "/properties/futureCatalogProperty"
                && item.classification == MappingClass::UnknownOrUnsupported
        }));
        assert!(roundtrip(&view).expect("OGC unknown field roundtrip"));
        let _ = (datacite, lineage);
    }

    #[test]
    fn unknown_openlineage_facet_is_preserved_and_classified() {
        let (datacite, ogc, mut lineage) = parsed_fixtures();
        lineage["events"][0]["run"]["facets"]["futureFacet"] =
            json!({"opaque": "retain"});
        let e1 = parse(E1_CONTEXT, "E1 context").expect("E1 context");
        let (view, decisions) = project_openlineage(&lineage, &e1).expect("OpenLineage projection");
        let ProviderNativeView::OpenLineage(view) = view else {
            panic!("OpenLineage view expected")
        };
        assert_eq!(
            view.preserved_native_events[0]["run"]["facets"]["futureFacet"]["opaque"],
            "retain"
        );
        assert!(decisions.iter().any(|item| {
            item.source_path == "/events/0/run/facets/futureFacet"
                && item.classification == MappingClass::UnknownOrUnsupported
        }));
        assert!(roundtrip(&view).expect("OpenLineage unknown facet roundtrip"));
        let _ = (datacite, ogc);
    }

    #[test]
    fn each_provider_view_retains_the_complete_native_source_record() {
        let report = build_report().expect("report");
        let ProviderNativeView::DataCite(datacite) = &report.provider_views[0] else {
            panic!("DataCite view expected")
        };
        let ProviderNativeView::OgcRecords(ogc) = &report.provider_views[1] else {
            panic!("OGC view expected")
        };
        let ProviderNativeView::OpenLineage(openlineage) = &report.provider_views[2] else {
            panic!("OpenLineage view expected")
        };
        assert_eq!(datacite.native_record, parse(DATACITE, "DataCite").expect("DataCite source"));
        assert_eq!(ogc.native_record, parse(OGC, "OGC").expect("OGC source"));
        assert_eq!(
            openlineage.native_record,
            parse(OPENLINEAGE, "OpenLineage").expect("OpenLineage source")
        );
        assert!(report.mapping_decisions.iter().any(|item| {
            item.provider == "DATACITE" && item.source_path == "/" && item.target_path.as_deref() == Some("/native_view/native_record")
        }));
        assert!(report.mapping_decisions.iter().any(|item| {
            item.provider == "OGC_API_RECORDS" && item.source_path == "/"
        }));
        assert!(report.mapping_decisions.iter().any(|item| {
            item.provider == "OPENLINEAGE" && item.source_path == "/"
        }));
    }

    #[test]
    fn typed_projection_serialization_is_deterministic() {
        let report_a = build_report().expect("report A");
        let report_b = build_report().expect("report B");
        let bytes_a = serde_json::to_vec(&report_a).expect("serialize A");
        let bytes_b = serde_json::to_vec(&report_b).expect("serialize B");
        assert_eq!(bytes_a, bytes_b);
    }

    #[test]
    fn all_source_receipts_are_bound_to_the_unchanged_fixture_bytes() {
        let report = build_report().expect("report");
        let expected = [
            ("DATACITE", DATACITE),
            ("OGC_API_RECORDS", OGC),
            ("OPENLINEAGE", OPENLINEAGE),
        ];
        for (provider, bytes) in expected {
            let receipt = report
                .source_receipts
                .iter()
                .find(|item| item.provider == provider)
                .expect("source receipt");
            assert_eq!(receipt.source_byte_length, bytes.len() as u64);
            assert_eq!(receipt.source_sha256, hash(bytes));
            assert!(!receipt.source_path.is_empty());
        }
        assert!(roundtrip(&report).expect("report roundtrip"));
    }
}
