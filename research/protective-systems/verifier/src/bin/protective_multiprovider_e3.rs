//! E3 multi-provider evidence portability gate.
//!
//! This executable validates synthetic fixtures from DataCite REST JSON:API,
//! OGC API - Records Core 1.0 GeoJSON and OpenLineage 2.0.2. Provider-native
//! payloads are retained byte-for-byte. The common projection is deliberately
//! lossy only where fields are explicitly enumerated; extension data remains
//! in the original source record and is reported as preserved.
//!
//! No live provider request, DOI registration, external record authentication,
//! geospatial payload validation, executed E1 analysis, scientific validity or
//! physical resilience is claimed.

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::process;

const DATACITE: &[u8] = include_bytes!(
    "../../../fixtures/protective-evidence-multiprovider-e3-v0.1/datacite/doi-response.json"
);
const OGC_RECORD: &[u8] = include_bytes!(
    "../../../fixtures/protective-evidence-multiprovider-e3-v0.1/ogc/record-feature.json"
);
const OPENLINEAGE_EVENTS: &[u8] = include_bytes!(
    "../../../fixtures/protective-evidence-multiprovider-e3-v0.1/openlineage/e3-run-events.json"
);
const E1_CONTEXT: &[u8] = include_bytes!(
    "../../../fixtures/protective-evidence-multiprovider-e3-v0.1/context/e1-envelope-context.json"
);
const MANIFEST: &str = include_str!(
    "../../../fixtures/protective-evidence-multiprovider-e3-v0.1/provenance/e3-provider-fixture-manifest.json"
);
const MANIFEST_SCHEMA: &str = include_str!(
    "../../../schemas/protective-multiprovider-fixture-manifest-v0.1.schema.json"
);
const PACKAGED_MANIFEST_SCHEMA: &[u8] = include_bytes!(
    "../../../fixtures/protective-evidence-multiprovider-e3-v0.1/schemas/protective-multiprovider-fixture-manifest-v0.1.schema.json"
);
const OPENLINEAGE_SCHEMA: &str = include_str!(
    "../../../fixtures/protective-evidence-multiprovider-e3-v0.1/schemas/openlineage-2.0.2.schema.json"
);
const E1_ORIGINAL: &[u8] = include_bytes!(
    "../../../fixtures/protective-evidence-crate-v0.1/evidence/protective-model-evidence-envelope-v0.1.json"
);
const CRATE_METADATA: &str = include_str!(
    "../../../fixtures/protective-evidence-multiprovider-e3-v0.1/ro-crate-metadata.json"
);

const CLAIM_CEILING: &str = "MULTI_PROVIDER_METADATA_PORTABILITY_ONLY";
const RIGHTS_STATUS: &str = "SYNTHETIC_OPEN_FIXTURE";
const PERMITTED_USE: &str = "SYNTHETIC_TEST_ONLY";
const DATACITE_ID: &str = "10.5555/synthetic-e3-protection-record-001";
const OGC_ID: &str = "e3-synthetic-record-001";
const OPENLINEAGE_SCHEMA_ID: &str = "https://openlineage.io/spec/2-0-2/OpenLineage.json";
const OPENLINEAGE_SCHEMA_BLOB: &str = "58167f71407add7734810cda9943b39b83c89aa2";
const OGC_CORE_CONFORMANCE: &str =
    "http://www.opengis.net/spec/ogcapi-records-1/1.0/conf/core";
const OGC_JSON_CONFORMANCE: &str =
    "http://www.opengis.net/spec/ogcapi-records-1/1.0/conf/json";
const OGC_PROFILE_LINK: &str =
    "http://www.opengis.net/def/profile/OGC/0/ogc-catalog";

const ARTIFACTS: [(&str, &[u8], &str); 7] = [
    (
        "datacite/doi-response.json",
        DATACITE,
        "application/vnd.api+json",
    ),
    ("ogc/record-feature.json", OGC_RECORD, "application/geo+json"),
    (
        "openlineage/e3-run-events.json",
        OPENLINEAGE_EVENTS,
        "application/json",
    ),
    ("context/e1-envelope-context.json", E1_CONTEXT, "application/json"),
    (
        "provenance/e3-provider-fixture-manifest.json",
        MANIFEST.as_bytes(),
        "application/json",
    ),
    (
        "schemas/protective-multiprovider-fixture-manifest-v0.1.schema.json",
        PACKAGED_MANIFEST_SCHEMA,
        "application/schema+json",
    ),
    (
        "schemas/openlineage-2.0.2.schema.json",
        OPENLINEAGE_SCHEMA.as_bytes(),
        "application/schema+json",
    ),
];

#[derive(Debug, Serialize)]
struct ProviderProjection {
    provider: &'static str,
    source_identity: String,
    title_or_job: String,
    native_profile: &'static str,
    source_sha256: String,
    mapped_field_count: usize,
    preserved_extension_paths: Vec<&'static str>,
    source_bytes_retained: bool,
}

#[derive(Debug, Serialize)]
struct GateReport {
    schema_version: &'static str,
    claim_ceiling: &'static str,
    disposition: &'static str,
    source_record_count: usize,
    verified_crate_file_count: usize,
    provider_projection_count: usize,
    datacite_schema_field_profile: &'static str,
    ogc_record_profile: &'static str,
    openlineage_schema_id: &'static str,
    openlineage_events_validated: usize,
    openlineage_run_transitions: Vec<&'static str>,
    e1_context_execution_status: &'static str,
    e1_context_relationship: &'static str,
    unknown_extension_roots_preserved: usize,
    unmapped_or_extension_paths: Vec<&'static str>,
    manifest_schema_validated: bool,
    exact_e1_context_bytes_preserved: bool,
    openlineage_schema_blob_sha_pinned: bool,
    provider_projections: Vec<ProviderProjection>,
}

fn required_text<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| format!("{path}.{key} must be non-empty text"))
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn parse_json(bytes: &[u8], label: &str) -> Result<Value, String> {
    serde_json::from_slice(bytes).map_err(|error| format!("{label} JSON is invalid: {error}"))
}

fn array_contains_text(value: &Value, expected: &str) -> bool {
    value
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(expected)))
}

fn has_link(value: &Value, relation: &str, href: Option<&str>) -> bool {
    value
        .as_array()
        .is_some_and(|links| {
            links.iter().any(|link| {
                link.get("rel").and_then(Value::as_str) == Some(relation)
                    && href.is_none_or(|expected| {
                        link.get("href").and_then(Value::as_str) == Some(expected)
                    })
            })
        })
}

fn validate_datacite(value: &Value) -> Result<ProviderProjection, String> {
    let data = value
        .get("data")
        .ok_or_else(|| "DataCite JSON:API response.data is required".to_owned())?;
    if required_text(data, "type", "DataCite data")? != "dois" {
        return Err("DataCite JSON:API data.type must be dois".to_owned());
    }
    let attributes = data
        .get("attributes")
        .ok_or_else(|| "DataCite data.attributes is required".to_owned())?;
    let id = required_text(data, "id", "DataCite data")?;
    let doi = required_text(attributes, "doi", "DataCite attributes")?;
    if id != doi || doi != DATACITE_ID {
        return Err("DataCite resource id and attributes.doi must match the frozen DOI-shaped identity".to_owned());
    }
    if required_text(attributes, "schemaVersion", "DataCite attributes")?
        != "http://datacite.org/schema/kernel-4"
    {
        return Err("DataCite REST schemaVersion must retain the kernel-4 URI".to_owned());
    }
    let types = attributes
        .get("types")
        .ok_or_else(|| "DataCite attributes.types is required".to_owned())?;
    if required_text(types, "resourceTypeGeneral", "DataCite types")? != "Dataset" {
        return Err("DataCite resourceTypeGeneral must remain Dataset".to_owned());
    }
    let creators = attributes
        .get("creators")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| "DataCite creators must be a non-empty array".to_owned())?;
    let titles = attributes
        .get("titles")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| "DataCite titles must be a non-empty array".to_owned())?;
    let title = required_text(&titles[0], "title", "DataCite titles[0]")?.to_owned();
    let publisher = required_text(attributes, "publisher", "DataCite attributes")?;
    if publisher.is_empty() || creators.is_empty() {
        return Err("DataCite publisher and creators are required".to_owned());
    }
    let related = attributes
        .get("relatedIdentifiers")
        .and_then(Value::as_array)
        .ok_or_else(|| "DataCite relatedIdentifiers must be an array".to_owned())?;
    let v47_identifier = related.iter().any(|item| {
        item.get("relatedIdentifierType").and_then(Value::as_str) == Some("SWHID")
            && item.get("relationType").and_then(Value::as_str) == Some("Other")
            && item
                .get("relationTypeInformation")
                .and_then(Value::as_str)
                .is_some_and(|text| !text.trim().is_empty())
    });
    if !v47_identifier {
        return Err("DataCite 4.7 SWHID/Other/relationTypeInformation fields must be preserved".to_owned());
    }
    let rights = attributes
        .get("rightsList")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| "DataCite rightsList must be present".to_owned())?;
    required_text(&rights[0], "rights", "DataCite rightsList[0]")?;
    let extension = value
        .get("meta")
        .and_then(|meta| meta.get("e3FixtureExtension"))
        .ok_or_else(|| "DataCite JSON:API meta.e3FixtureExtension must be retained".to_owned())?;
    if required_text(extension, "preservationMode", "DataCite extension")?
        != "retain-unmapped-source-fields"
    {
        return Err("DataCite JSON:API extension preservation marker is invalid".to_owned());
    }
    Ok(ProviderProjection {
        provider: "DATACITE",
        source_identity: doi.to_owned(),
        title_or_job: title,
        native_profile: "DataCite REST JSON:API / Metadata Schema 4.7 field profile",
        source_sha256: sha256(DATACITE),
        mapped_field_count: 9,
        preserved_extension_paths: vec!["/meta/e3FixtureExtension"],
        source_bytes_retained: true,
    })
}

fn validate_ogc(value: &Value) -> Result<ProviderProjection, String> {
    if required_text(value, "type", "OGC record")? != "Feature" {
        return Err("OGC GeoJSON record must be a Feature".to_owned());
    }
    let id = required_text(value, "id", "OGC record")?;
    if id != OGC_ID {
        return Err("OGC record id differs from the frozen catalog identity".to_owned());
    }
    let conforms = value
        .get("conformsTo")
        .ok_or_else(|| "OGC GeoJSON conformsTo must be top-level".to_owned())?;
    if !array_contains_text(conforms, OGC_CORE_CONFORMANCE)
        || !array_contains_text(conforms, OGC_JSON_CONFORMANCE)
    {
        return Err("OGC record must declare the Core 1.0 and GeoJSON conformance classes".to_owned());
    }
    let properties = value
        .get("properties")
        .filter(|item| item.is_object())
        .ok_or_else(|| "OGC GeoJSON properties must be an object".to_owned())?;
    let title = required_text(properties, "title", "OGC properties")?.to_owned();
    required_text(properties, "description", "OGC properties")?;
    required_text(properties, "created", "OGC properties")?;
    required_text(properties, "updated", "OGC properties")?;
    required_text(properties, "language", "OGC properties")?;
    if properties.get("keywords").and_then(Value::as_array).is_none() {
        return Err("OGC properties.keywords must remain an array".to_owned());
    }
    let extension = properties
        .get("e3Extension")
        .ok_or_else(|| "OGC properties.e3Extension must be preserved".to_owned())?;
    if required_text(extension, "value", "OGC e3Extension")? != "must-be-preserved" {
        return Err("OGC foreign extension value changed".to_owned());
    }
    let geometry = value
        .get("geometry")
        .filter(|item| item.is_object())
        .ok_or_else(|| "OGC GeoJSON Feature geometry must be present and non-null for this fixture".to_owned())?;
    if geometry.get("type").and_then(Value::as_str) != Some("Polygon") {
        return Err("OGC fixture geometry type must remain Polygon".to_owned());
    }
    let coordinates = geometry
        .get("coordinates")
        .and_then(Value::as_array)
        .and_then(|polygons| polygons.first())
        .and_then(Value::as_array)
        .filter(|ring| ring.len() >= 4)
        .ok_or_else(|| "OGC Polygon must contain a valid exterior ring".to_owned())?;
    let first = coordinates.first().ok_or_else(|| "OGC Polygon ring is empty".to_owned())?;
    let last = coordinates.last().ok_or_else(|| "OGC Polygon ring is empty".to_owned())?;
    if first != last {
        return Err("OGC Polygon exterior ring must be closed".to_owned());
    }
    for position in coordinates {
        let pair = position
            .as_array()
            .filter(|pair| pair.len() >= 2)
            .ok_or_else(|| "OGC GeoJSON positions must contain at least two coordinates".to_owned())?;
        let longitude = pair[0]
            .as_f64()
            .ok_or_else(|| "OGC longitude must be numeric".to_owned())?;
        let latitude = pair[1]
            .as_f64()
            .ok_or_else(|| "OGC latitude must be numeric".to_owned())?;
        if !(-180.0..=180.0).contains(&longitude) || !(-90.0..=90.0).contains(&latitude) {
            return Err("OGC GeoJSON coordinates are outside CRS84 longitude/latitude bounds".to_owned());
        }
    }
    let extent = value
        .get("extent")
        .ok_or_else(|| "OGC GeoJSON record extent is required by the selected JSON conformance class".to_owned())?;
    let spatial = extent
        .get("spatial")
        .ok_or_else(|| "OGC record extent.spatial is required".to_owned())?;
    let bbox = spatial
        .get("bbox")
        .and_then(Value::as_array)
        .filter(|items| items.len() == 4)
        .ok_or_else(|| "OGC spatial bbox must have four coordinates".to_owned())?;
    let bbox_values = bbox
        .iter()
        .map(|item| item.as_f64().ok_or_else(|| "OGC bbox coordinates must be numeric".to_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    if bbox_values != vec![0.0, 0.0, 1.0, 1.0] {
        return Err("OGC bbox must match the fictional unit-square geometry".to_owned());
    }
    if required_text(spatial, "crs", "OGC extent.spatial")?
        != "http://www.opengis.net/def/crs/OGC/1.3/CRS84"
    {
        return Err("OGC fixture must explicitly preserve its declared CRS84 identifier".to_owned());
    }
    if value.get("crs").is_some() || geometry.get("crs").is_some() {
        return Err("E3 OGC adapter must not invent a legacy GeoJSON CRS member".to_owned());
    }
    let links = value
        .get("links")
        .ok_or_else(|| "OGC record links must be top-level".to_owned())?;
    if !has_link(links, "self", None)
        || !has_link(links, "collection", None)
        || !has_link(links, "profile", Some(OGC_PROFILE_LINK))
    {
        return Err("OGC GeoJSON record must retain self, collection and OGC catalog profile links".to_owned());
    }
    Ok(ProviderProjection {
        provider: "OGC_API_RECORDS",
        source_identity: id.to_owned(),
        title_or_job: title,
        native_profile: "OGC API - Records Part 1 Core 1.0 GeoJSON",
        source_sha256: sha256(OGC_RECORD),
        mapped_field_count: 10,
        preserved_extension_paths: vec!["/properties/e3Extension"],
        source_bytes_retained: true,
    })
}

fn validate_openlineage(
    value: &Value,
    e1: &Value,
    schema: &Value,
) -> Result<(ProviderProjection, Vec<&'static str>), String> {
    let validator = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("pinned OpenLineage 2.0.2 schema is invalid: {error}"))?;
    let events = value
        .get("events")
        .and_then(Value::as_array)
        .filter(|items| items.len() == 2)
        .ok_or_else(|| "OpenLineage fixture must contain exactly one START and one terminal event".to_owned())?;
    for event in events {
        validator
            .validate(event)
            .map_err(|error| format!("OpenLineage 2.0.2 RunEvent validation failed: {error}"))?;
    }
    let start = &events[0];
    let complete = &events[1];
    if required_text(start, "eventType", "OpenLineage events[0]")? != "START"
        || required_text(complete, "eventType", "OpenLineage events[1]")? != "COMPLETE"
    {
        return Err("OpenLineage event sequence must be START then COMPLETE".to_owned());
    }
    let run_id = required_text(
        start.get("run").ok_or_else(|| "OpenLineage START run is required".to_owned())?,
        "runId",
        "OpenLineage START run",
    )?;
    let complete_run_id = required_text(
        complete.get("run").ok_or_else(|| "OpenLineage COMPLETE run is required".to_owned())?,
        "runId",
        "OpenLineage COMPLETE run",
    )?;
    if run_id != complete_run_id {
        return Err("OpenLineage START and COMPLETE must reference the same runId".to_owned());
    }
    let start_job = start
        .get("job")
        .ok_or_else(|| "OpenLineage START job is required".to_owned())?;
    let complete_job = complete
        .get("job")
        .ok_or_else(|| "OpenLineage COMPLETE job is required".to_owned())?;
    if start_job != complete_job {
        return Err("OpenLineage START and COMPLETE job identities must match".to_owned());
    }
    let job_namespace = required_text(start_job, "namespace", "OpenLineage job")?;
    let job_name = required_text(start_job, "name", "OpenLineage job")?;
    if job_namespace != "luminous.synthetic.e3"
        || job_name != "multiprovider-adapter-conformance"
    {
        return Err("OpenLineage fixture job must identify the E3 adapter conformance run".to_owned());
    }
    let start_time = required_text(start, "eventTime", "OpenLineage START")?;
    let complete_time = required_text(complete, "eventTime", "OpenLineage COMPLETE")?;
    if complete_time <= start_time {
        return Err("OpenLineage COMPLETE eventTime must be later than START".to_owned());
    }
    let e1_analysis = e1
        .get("analysis")
        .ok_or_else(|| "E1 context envelope analysis is required".to_owned())?;
    let e1_analysis_id = required_text(e1_analysis, "analysis_id", "E1 context analysis")?;
    let e1_status = required_text(e1_analysis, "execution_status", "E1 context analysis")?;
    if e1_status != "NOT_EXECUTED"
        || !e1_analysis.get("run_id").is_some_and(Value::is_null)
    {
        return Err("E1 source context must remain NOT_EXECUTED with no run ID".to_owned());
    }
    for event in events {
        let facet = event
            .get("run")
            .and_then(|run| run.get("facets"))
            .and_then(|facets| facets.get("e3_evidence_context"))
            .ok_or_else(|| "OpenLineage E3 evidence context facet is required".to_owned())?;
        if required_text(facet, "fixtureRunClass", "OpenLineage E3 facet")?
            != "E3_ADAPTER_CONFORMANCE_FIXTURE"
            || required_text(facet, "sourceE1AnalysisId", "OpenLineage E3 facet")?
                != e1_analysis_id
            || required_text(facet, "sourceE1ExecutionStatus", "OpenLineage E3 facet")?
                != e1_status
            || required_text(facet, "sourceBinding", "OpenLineage E3 facet")?
                != "CONTEXT_ONLY"
            || facet.get("notASourceExecution").and_then(Value::as_bool) != Some(true)
        {
            return Err("OpenLineage E3 facet must mark E1 NOT_EXECUTED as context only, not as this run".to_owned());
        }
    }
    let inputs = start
        .get("inputs")
        .and_then(Value::as_array)
        .filter(|items| items.len() == 1)
        .ok_or_else(|| "OpenLineage E3 START must contain one input dataset".to_owned())?;
    let outputs = start
        .get("outputs")
        .and_then(Value::as_array)
        .filter(|items| items.len() == 1)
        .ok_or_else(|| "OpenLineage E3 START must contain one output dataset".to_owned())?;
    if required_text(&inputs[0], "name", "OpenLineage inputs[0]")? != "datacite-doi-metadata"
        || required_text(&outputs[0], "name", "OpenLineage outputs[0]")? != "ogc-record-metadata"
    {
        return Err("OpenLineage dataset namespace/name mapping differs from E3 fixtures".to_owned());
    }
    let input_namespace = required_text(&inputs[0], "namespace", "OpenLineage inputs[0]")?;
    let output_namespace = required_text(&outputs[0], "namespace", "OpenLineage outputs[0]")?;
    if input_namespace != output_namespace
        || input_namespace != "luminous.synthetic.fixtures"
    {
        return Err("OpenLineage input/output dataset namespaces must match the E3 fixture namespace".to_owned());
    }
    let projection = ProviderProjection {
        provider: "OPENLINEAGE",
        source_identity: run_id.to_owned(),
        title_or_job: format!("{job_namespace}/{job_name}"),
        native_profile: "OpenLineage RunEvent 2.0.2",
        source_sha256: sha256(OPENLINEAGE_EVENTS),
        mapped_field_count: 11,
        preserved_extension_paths: vec![
            "/events/0/run/facets/e3_evidence_context",
            "/events/1/run/facets/e3_evidence_context",
        ],
        source_bytes_retained: true,
    };
    Ok((projection, vec!["START", "COMPLETE"]))
}

fn validate_manifest(
    manifest: &Value,
    schema: &Value,
    sources: &[(&str, &[u8], &str, &str)],
    e1_context: &[u8],
    openlineage_schema: &[u8],
) -> Result<(), String> {
    let validator = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("E3 manifest JSON Schema is invalid: {error}"))?;
    validator
        .validate(manifest)
        .map_err(|error| format!("E3 fixture manifest schema validation failed: {error}"))?;
    if required_text(manifest, "schema_version", "E3 manifest")?
        != "protective-multiprovider-fixture-manifest/0.1"
        || required_text(manifest, "evidence_class", "E3 manifest")? != "synthetic"
        || required_text(manifest, "claim_ceiling", "E3 manifest")? != CLAIM_CEILING
    {
        return Err("E3 manifest must remain synthetic and metadata-portability-only".to_owned());
    }
    let e1_context_manifest = manifest
        .get("source_e1_context")
        .ok_or_else(|| "E3 manifest source_e1_context is required".to_owned())?;
    if required_text(e1_context_manifest, "execution_status", "E3 source E1 context")?
        != "NOT_EXECUTED"
        || required_text(e1_context_manifest, "relationship", "E3 source E1 context")?
            != "CONTEXT_ONLY_NOT_A_RUN"
        || required_text(e1_context_manifest, "relative_path", "E3 source E1 context")?
            != "context/e1-envelope-context.json"
    {
        return Err("E3 manifest must preserve the E1 NOT_EXECUTED context-only boundary".to_owned());
    }
    let e1_context_length = e1_context_manifest
        .get("byte_length")
        .and_then(Value::as_u64)
        .ok_or_else(|| "E3 source E1 context byte_length is invalid".to_owned())?;
    if e1_context_length != e1_context.len() as u64
        || required_text(e1_context_manifest, "sha256", "E3 source E1 context")?
            != sha256(e1_context)
    {
        return Err("E3 source E1 envelope byte length/digest differs from the packaged context".to_owned());
    }
    let sources_manifest = manifest
        .get("sources")
        .and_then(Value::as_array)
        .filter(|items| items.len() == sources.len())
        .ok_or_else(|| "E3 manifest must contain exactly three provider source records".to_owned())?;
    for (path, bytes, provider, identity) in sources {
        let item = sources_manifest
            .iter()
            .find(|item| item.get("provider").and_then(Value::as_str) == Some(*provider))
            .ok_or_else(|| format!("E3 manifest provider entry is missing: {provider}"))?;
        let native_profile = required_text(item, "native_profile", "E3 manifest source")?;
        let profile_matches = match *provider {
            "DATACITE" => native_profile.contains("4.7"),
            "OGC_API_RECORDS" => native_profile.contains("Core 1.0 GeoJSON"),
            "OPENLINEAGE" => native_profile.contains("2.0.2"),
            _ => false,
        };
        if !profile_matches
            || required_text(item, "relative_path", "E3 manifest source")? != *path
            || required_text(item, "source_identity", "E3 manifest source")? != *identity
            || required_text(item, "retrieval_mode", "E3 manifest source")?
                != "SYNTHETIC_STATIC_FIXTURE"
            || required_text(item, "rights_status", "E3 manifest source")? != RIGHTS_STATUS
            || required_text(item, "permitted_use", "E3 manifest source")? != PERMITTED_USE
            || required_text(item, "claim_ceiling", "E3 manifest source")? != CLAIM_CEILING
        {
            return Err(format!("E3 manifest metadata differs for provider {provider}"));
        }
        if item.get("byte_length").and_then(Value::as_u64) != Some(bytes.len() as u64)
            || required_text(item, "sha256", "E3 manifest source")? != sha256(bytes)
        {
            return Err(format!("E3 manifest byte length/SHA-256 differs for provider {provider}"));
        }
    }
    let assets = manifest
        .get("validation_assets")
        .and_then(Value::as_array)
        .filter(|items| items.len() == 1)
        .ok_or_else(|| "E3 manifest must pin exactly one validation schema asset".to_owned())?;
    let asset = &assets[0];
    if required_text(asset, "provider", "E3 validation asset")? != "OPENLINEAGE"
        || required_text(asset, "relative_path", "E3 validation asset")?
            != "schemas/openlineage-2.0.2.schema.json"
        || required_text(asset, "schema_id", "E3 validation asset")? != OPENLINEAGE_SCHEMA_ID
        || required_text(asset, "source_repository", "E3 validation asset")?
            != "OpenLineage/OpenLineage"
        || required_text(asset, "source_blob_sha", "E3 validation asset")?
            != OPENLINEAGE_SCHEMA_BLOB
        || asset.get("byte_length").and_then(Value::as_u64)
            != Some(openlineage_schema.len() as u64)
        || required_text(asset, "sha256", "E3 validation asset")?
            != sha256(openlineage_schema)
    {
        return Err("E3 OpenLineage schema asset identity/digest is not pinned to the official 2.0.2 snapshot".to_owned());
    }
    Ok(())
}


fn validate_crate_metadata(
    crate_text: &str,
    artifacts: &[(&str, &[u8], &str)],
) -> Result<(), String> {
    let crate_root = parse_json(crate_text.as_bytes(), "E3 RO-Crate")?;
    let contexts = crate_root
        .get("@context")
        .and_then(Value::as_array)
        .ok_or_else(|| "E3 RO-Crate @context must be an array".to_owned())?;
    if contexts.first().and_then(Value::as_str) != Some("https://w3id.org/ro/crate/1.3/context") {
        return Err("E3 RO-Crate must pin the RO-Crate 1.3 context".to_owned());
    }
    let graph = crate_root
        .get("@graph")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| "E3 RO-Crate @graph must be non-empty".to_owned())?;
    let mut entities = std::collections::BTreeMap::new();
    for entity in graph {
        let id = required_text(entity, "@id", "E3 RO-Crate graph entity")?.to_owned();
        if entities.insert(id.clone(), entity).is_some() {
            return Err(format!("E3 RO-Crate contains duplicate @id: {id}"));
        }
    }
    let descriptor = entities
        .get("ro-crate-metadata.json")
        .ok_or_else(|| "E3 RO-Crate descriptor is missing".to_owned())?;
    if descriptor.get("@type").and_then(Value::as_str) != Some("CreativeWork")
        || descriptor
            .get("conformsTo")
            .and_then(|item| item.get("@id"))
            .and_then(Value::as_str)
            != Some("https://w3id.org/ro/crate/1.3")
        || descriptor
            .get("about")
            .and_then(|item| item.get("@id"))
            .and_then(Value::as_str)
            != Some("./")
    {
        return Err("E3 RO-Crate descriptor must identify RO-Crate 1.3 and root ./".to_owned());
    }
    let root = entities
        .get("./")
        .ok_or_else(|| "E3 RO-Crate root Dataset is missing".to_owned())?;
    if root.get("@type").and_then(Value::as_str) != Some("Dataset")
        || required_text(root, "claimCeiling", "E3 RO-Crate root")? != CLAIM_CEILING
        || required_text(root, "rightsStatus", "E3 RO-Crate root")? != RIGHTS_STATUS
        || required_text(root, "permittedUse", "E3 RO-Crate root")? != PERMITTED_USE
        || root.get("license").is_some()
    {
        return Err("E3 RO-Crate root must preserve the synthetic rights/claim ceiling and omit unverified license".to_owned());
    }
    let expected: BTreeSet<String> =
        artifacts.iter().map(|(path, _, _)| (*path).to_owned()).collect();
    let parts = root
        .get("hasPart")
        .and_then(Value::as_array)
        .ok_or_else(|| "E3 RO-Crate root.hasPart must be an array".to_owned())?;
    let part_ids: BTreeSet<String> = parts
        .iter()
        .map(|item| {
            item.get("@id")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| "E3 RO-Crate root.hasPart entry requires @id".to_owned())
        })
        .collect::<Result<_, _>>()?;
    if part_ids != expected || parts.len() != expected.len() {
        return Err("E3 RO-Crate root.hasPart does not exactly match the seven-file inventory".to_owned());
    }
    let file_ids: BTreeSet<String> = entities
        .iter()
        .filter(|(_, item)| item.get("@type").and_then(Value::as_str) == Some("File"))
        .map(|(id, _)| id.clone())
        .collect();
    if file_ids != expected {
        return Err("E3 RO-Crate File entity inventory does not match its root Dataset".to_owned());
    }
    for (path, bytes, media_type) in artifacts {
        let file = entities
            .get(*path)
            .ok_or_else(|| format!("E3 RO-Crate File entity is missing: {path}"))?;
        if file.get("@type").and_then(Value::as_str) != Some("File")
            || required_text(file, "encodingFormat", path)? != *media_type
            || required_text(file, "contentSize", path)? != bytes.len().to_string()
            || required_text(file, "sha256", path)? != sha256(bytes)
            || required_text(file, "rightsStatus", path)? != RIGHTS_STATUS
            || required_text(file, "permittedUse", path)? != PERMITTED_USE
            || required_text(file, "claimCeiling", path)? != CLAIM_CEILING
        {
            return Err(format!("E3 RO-Crate file metadata/digest mismatch: {path}"));
        }
    }
    Ok(())
}

fn validate_bundle(
    datacite_text: &str,
    ogc_text: &str,
    openlineage_text: &str,
    e1_context_text: &str,
    manifest_text: &str,
    crate_text: &str,
) -> Result<GateReport, String> {
    if E1_CONTEXT != E1_ORIGINAL || e1_context_text.as_bytes() != E1_ORIGINAL {
        return Err("E3 packaged E1 context envelope is not byte-identical to the E1 source fixture".to_owned());
    }
    if E1_CONTEXT != e1_context_text.as_bytes() {
        return Err("E3 packaged E1 context differs from original bytes".to_owned());
    }
    if PACKAGED_MANIFEST_SCHEMA != MANIFEST_SCHEMA.as_bytes() {
        return Err("E3 packaged manifest schema differs from the verifier schema".to_owned());
    }
    let datacite = parse_json(datacite_text.as_bytes(), "DataCite")?;
    let ogc = parse_json(ogc_text.as_bytes(), "OGC API - Records")?;
    let openlineage = parse_json(openlineage_text.as_bytes(), "OpenLineage event bundle")?;
    let e1 = parse_json(e1_context_text.as_bytes(), "E1 context envelope")?;
    let manifest = parse_json(manifest_text.as_bytes(), "E3 manifest")?;
    let manifest_schema = parse_json(MANIFEST_SCHEMA.as_bytes(), "E3 manifest schema")?;
    let openlineage_schema = parse_json(OPENLINEAGE_SCHEMA.as_bytes(), "OpenLineage 2.0.2 schema")?;
    if required_text(&e1, "evidence_class", "E1 context")? != "synthetic" {
        return Err("E3 E1 context envelope must remain synthetic".to_owned());
    }
    let sources = [
        (
            "datacite/doi-response.json",
            DATACITE,
            "DATACITE",
            DATACITE_ID,
        ),
        ("ogc/record-feature.json", OGC_RECORD, "OGC_API_RECORDS", OGC_ID),
        (
            "openlineage/e3-run-events.json",
            OPENLINEAGE_EVENTS,
            "OPENLINEAGE",
            "0e70677e-e4b5-4a1f-b908-7e22ef00ed31",
        ),
    ];
    validate_manifest(
        &manifest,
        &manifest_schema,
        &sources,
        E1_CONTEXT,
        OPENLINEAGE_SCHEMA.as_bytes(),
    )?;
    let datacite_projection = validate_datacite(&datacite)?;
    let ogc_projection = validate_ogc(&ogc)?;
    let (openlineage_projection, transitions) =
        validate_openlineage(&openlineage, &e1, &openlineage_schema)?;
    validate_crate_metadata(CRATE_METADATA, &ARTIFACTS)?;

    Ok(GateReport {
        schema_version: "protective-multiprovider-e3/0.1",
        claim_ceiling: CLAIM_CEILING,
        disposition: "ABSTAIN_SYNTHETIC_METADATA_ONLY",
        source_record_count: 3,
        verified_crate_file_count: ARTIFACTS.len(),
        provider_projection_count: 3,
        datacite_schema_field_profile: "DataCite REST JSON:API with 4.7 field-level controls; no XSD registration validation",
        ogc_record_profile: "OGC API - Records Part 1 Core 1.0 GeoJSON record-profile checks; no live API or geospatial payload validation",
        openlineage_schema_id: OPENLINEAGE_SCHEMA_ID,
        openlineage_events_validated: 2,
        openlineage_run_transitions: transitions,
        e1_context_execution_status: "NOT_EXECUTED",
        e1_context_relationship: "CONTEXT_ONLY_NOT_A_RUN",
        unknown_extension_roots_preserved: 3,
        unmapped_or_extension_paths: vec![
            "/meta/e3FixtureExtension",
            "/properties/e3Extension",
            "/run/facets/e3_evidence_context",
        ],
        manifest_schema_validated: true,
        exact_e1_context_bytes_preserved: true,
        openlineage_schema_blob_sha_pinned: true,
        provider_projections: vec![
            datacite_projection,
            ogc_projection,
            openlineage_projection,
        ],
    })
}

fn run() -> Result<GateReport, String> {
    validate_bundle(
        std::str::from_utf8(DATACITE).map_err(|error| format!("DataCite is not UTF-8: {error}"))?,
        std::str::from_utf8(OGC_RECORD).map_err(|error| format!("OGC fixture is not UTF-8: {error}"))?,
        std::str::from_utf8(OPENLINEAGE_EVENTS)
            .map_err(|error| format!("OpenLineage fixture is not UTF-8: {error}"))?,
        std::str::from_utf8(E1_CONTEXT).map_err(|error| format!("E1 context is not UTF-8: {error}"))?,
        MANIFEST,
        CRATE_METADATA,
    )
}

fn main() {
    match run() {
        Ok(report) => println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("E3 report serialization")
        ),
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

    fn datacite_value() -> Value {
        serde_json::from_slice(DATACITE).expect("DataCite fixture")
    }

    fn ogc_value() -> Value {
        serde_json::from_slice(OGC_RECORD).expect("OGC fixture")
    }

    fn openlineage_value() -> Value {
        serde_json::from_slice(OPENLINEAGE_EVENTS).expect("OpenLineage fixture")
    }

    fn e1_value() -> Value {
        serde_json::from_slice(E1_CONTEXT).expect("E1 context envelope")
    }

    fn openlineage_schema_value() -> Value {
        serde_json::from_str(OPENLINEAGE_SCHEMA).expect("OpenLineage schema")
    }

    #[test]
    fn valid_multi_provider_package_is_accepted_and_abstains() {
        let report = run().expect("valid E3 synthetic package");
        assert_eq!(report.source_record_count, 3);
        assert_eq!(report.verified_crate_file_count, 7);
        assert_eq!(report.provider_projection_count, 3);
        assert_eq!(report.openlineage_events_validated, 2);
        assert_eq!(report.openlineage_run_transitions, vec!["START", "COMPLETE"]);
        assert_eq!(report.e1_context_execution_status, "NOT_EXECUTED");
        assert_eq!(report.disposition, "ABSTAIN_SYNTHETIC_METADATA_ONLY");
        assert_eq!(report.unknown_extension_roots_preserved, 3);
    }

    #[test]
    fn datacite_resource_identity_mismatch_is_rejected() {
        let mut value = datacite_value();
        value["data"]["id"] = Value::String("10.5555/other".into());
        let error = validate_datacite(&value).expect_err("mismatched DOI identity");
        assert!(error.contains("resource id and attributes.doi"));
    }

    #[test]
    fn datacite_47_swhid_other_relationship_fields_are_required() {
        let mut value = datacite_value();
        value["data"]["attributes"]["relatedIdentifiers"][0]
            .as_object_mut()
            .expect("related identifier")
            .remove("relationTypeInformation");
        let error = validate_datacite(&value).expect_err("missing 4.7 extension");
        assert!(error.contains("SWHID/Other/relationTypeInformation"));
    }

    #[test]
    fn datacite_api_extension_is_retained_outside_the_core_metadata_mapping() {
        let value = datacite_value();
        let projection = validate_datacite(&value).expect("valid DataCite profile");
        assert!(projection
            .preserved_extension_paths
            .contains(&"/meta/e3FixtureExtension"));
        assert!(value["meta"]["e3FixtureExtension"]["preservationMode"].is_string());
    }

    #[test]
    fn datacite_unknown_extension_loss_is_rejected() {
        let mut value = datacite_value();
        value["meta"].as_object_mut().expect("meta").remove("e3FixtureExtension");
        let error = validate_datacite(&value).expect_err("dropped extension");
        assert!(error.contains("meta.e3FixtureExtension"));
    }

    #[test]
    fn ogc_core_geojson_feature_requires_top_level_conformance() {
        let mut value = ogc_value();
        value["properties"]["conformsTo"] = value["conformsTo"].clone();
        value.as_object_mut().expect("feature").remove("conformsTo");
        let error = validate_ogc(&value).expect_err("nested conformance");
        assert!(error.contains("conformsTo must be top-level"));
    }

    #[test]
    fn ogc_core_and_json_conformance_classes_are_both_required() {
        let mut value = ogc_value();
        value["conformsTo"] = json!([OGC_CORE_CONFORMANCE]);
        let error = validate_ogc(&value).expect_err("missing JSON conformance");
        assert!(error.contains("Core 1.0 and GeoJSON"));
    }

    #[test]
    fn ogc_record_requires_self_collection_and_profile_links() {
        let mut value = ogc_value();
        value["links"] = json!([{"rel":"self","href":"https://example.invalid/record"}]);
        let error = validate_ogc(&value).expect_err("missing links");
        assert!(error.contains("self, collection and OGC catalog profile"));
    }

    #[test]
    fn ogc_polygon_ring_must_be_closed() {
        let mut value = ogc_value();
        value["geometry"]["coordinates"][0][4] = json!([0.5, 0.5]);
        let error = validate_ogc(&value).expect_err("unclosed ring");
        assert!(error.contains("ring must be closed"));
    }

    #[test]
    fn ogc_out_of_range_coordinate_is_rejected() {
        let mut value = ogc_value();
        value["geometry"]["coordinates"][0][1] = json!([500.0, 0.0]);
        let error = validate_ogc(&value).expect_err("invalid longitude");
        assert!(error.contains("CRS84 longitude/latitude bounds"));
    }

    #[test]
    fn ogc_extent_bbox_must_match_geometry() {
        let mut value = ogc_value();
        value["extent"]["spatial"]["bbox"] = json!([0.0, 0.0, 2.0, 2.0]);
        let error = validate_ogc(&value).expect_err("bbox/geometry mismatch");
        assert!(error.contains("bbox must match"));
    }

    #[test]
    fn ogc_extent_crs_must_remain_explicit() {
        let mut value = ogc_value();
        value["extent"]["spatial"].as_object_mut().expect("spatial").remove("crs");
        let error = validate_ogc(&value).expect_err("CRS missing");
        assert!(error.contains("extent.spatial.crs"));
    }

    #[test]
    fn ogc_legacy_crs_member_is_not_invented() {
        let mut value = ogc_value();
        value["crs"] = json!({"type":"name","properties":{"name":"EPSG:3857"}});
        let error = validate_ogc(&value).expect_err("invented CRS");
        assert!(error.contains("must not invent a legacy GeoJSON CRS"));
    }

    #[test]
    fn ogc_unknown_extension_is_required_and_preserved() {
        let mut value = ogc_value();
        value["properties"].as_object_mut().expect("properties").remove("e3Extension");
        let error = validate_ogc(&value).expect_err("extension dropped");
        assert!(error.contains("e3Extension"));
    }

    #[test]
    fn openlineage_events_validate_against_the_pinned_official_schema() {
        let e1 = e1_value();
        let (projection, transitions) = validate_openlineage(
            &openlineage_value(),
            &e1,
            &openlineage_schema_value(),
        )
        .expect("OpenLineage official schema and E3 semantic checks");
        assert_eq!(projection.provider, "OPENLINEAGE");
        assert_eq!(transitions, vec!["START", "COMPLETE"]);
    }

    #[test]
    fn openlineage_invalid_run_uuid_is_rejected_by_official_schema() {
        let mut value = openlineage_value();
        value["events"][0]["run"]["runId"] = Value::String("not-a-uuid".into());
        let error = validate_openlineage(&value, &e1_value(), &openlineage_schema_value())
            .expect_err("invalid run UUID");
        assert!(error.contains("OpenLineage 2.0.2 RunEvent validation failed"));
    }

    #[test]
    fn openlineage_start_and_complete_must_share_a_run_id() {
        let mut value = openlineage_value();
        value["events"][1]["run"]["runId"] =
            Value::String("1d4d6b46-8b16-44a8-b090-174ec7e0ff44".into());
        let error = validate_openlineage(&value, &e1_value(), &openlineage_schema_value())
            .expect_err("run identity changed");
        assert!(error.contains("same runId"));
    }

    #[test]
    fn openlineage_e1_not_executed_facet_cannot_be_promoted_to_real_run() {
        let mut value = openlineage_value();
        value["events"][0]["run"]["facets"]["e3_evidence_context"]
            ["notASourceExecution"] = Value::Bool(false);
        let error = validate_openlineage(&value, &e1_value(), &openlineage_schema_value())
            .expect_err("source run fabrication");
        assert!(error.contains("context only, not as this run"));
    }

    #[test]
    fn openlineage_terminal_event_must_follow_start() {
        let mut value = openlineage_value();
        value["events"][1]["eventTime"] = Value::String("2026-11-16T10:00:00.000Z".into());
        let error = validate_openlineage(&value, &e1_value(), &openlineage_schema_value())
            .expect_err("invalid transition");
        assert!(error.contains("COMPLETE eventTime must be later than START"));
    }

    #[test]
    fn e1_context_must_remain_not_executed() {
        let mut e1 = e1_value();
        e1["analysis"]["execution_status"] = Value::String("COMPLETED".into());
        let error = validate_openlineage(
            &openlineage_value(),
            &e1,
            &openlineage_schema_value(),
        )
        .expect_err("E1 execution status promotion");
        assert!(error.contains("E1 source context must remain NOT_EXECUTED"));
    }

    #[test]
    fn openlineage_custom_facet_must_be_preserved() {
        let mut value = openlineage_value();
        value["events"][0]["run"]["facets"]
            .as_object_mut()
            .expect("facets")
            .remove("e3_evidence_context");
        let error = validate_openlineage(&value, &e1_value(), &openlineage_schema_value())
            .expect_err("facet lost");
        assert!(error.contains("E3 evidence context facet is required"));
    }

    #[test]
    fn e3_manifest_schema_rejects_unknown_fields() {
        let mut manifest: Value = serde_json::from_str(MANIFEST).expect("manifest");
        manifest["unrecognized"] = Value::Bool(true);
        let schema = serde_json::from_str(MANIFEST_SCHEMA).expect("manifest schema");
        let error = validate_manifest(
            &manifest,
            &schema,
            &[
                ("datacite/doi-response.json", DATACITE, "DATACITE", DATACITE_ID),
                ("ogc/record-feature.json", OGC_RECORD, "OGC_API_RECORDS", OGC_ID),
                (
                    "openlineage/e3-run-events.json",
                    OPENLINEAGE_EVENTS,
                    "OPENLINEAGE",
                    "0e70677e-e4b5-4a1f-b908-7e22ef00ed31",
                ),
            ],
            E1_CONTEXT,
            OPENLINEAGE_SCHEMA.as_bytes(),
        )
        .expect_err("unknown manifest property");
        assert!(error.contains("manifest schema validation failed"));
    }

    #[test]
    fn e3_manifest_cannot_escalate_rights() {
        let mut manifest: Value = serde_json::from_str(MANIFEST).expect("manifest");
        manifest["sources"][0]["rights_status"] = Value::String("LICENSED".into());
        let schema = serde_json::from_str(MANIFEST_SCHEMA).expect("manifest schema");
        let error = validate_manifest(
            &manifest,
            &schema,
            &[
                ("datacite/doi-response.json", DATACITE, "DATACITE", DATACITE_ID),
                ("ogc/record-feature.json", OGC_RECORD, "OGC_API_RECORDS", OGC_ID),
                (
                    "openlineage/e3-run-events.json",
                    OPENLINEAGE_EVENTS,
                    "OPENLINEAGE",
                    "0e70677e-e4b5-4a1f-b908-7e22ef00ed31",
                ),
            ],
            E1_CONTEXT,
            OPENLINEAGE_SCHEMA.as_bytes(),
        )
        .expect_err("rights escalation");
        assert!(error.contains("manifest schema validation failed"));
    }

    #[test]
    fn e3_manifest_digest_swap_is_rejected() {
        let mut manifest: Value = serde_json::from_str(MANIFEST).expect("manifest");
        manifest["sources"][0]["sha256"] = Value::String("00".repeat(32));
        let schema = serde_json::from_str(MANIFEST_SCHEMA).expect("manifest schema");
        let error = validate_manifest(
            &manifest,
            &schema,
            &[
                ("datacite/doi-response.json", DATACITE, "DATACITE", DATACITE_ID),
                ("ogc/record-feature.json", OGC_RECORD, "OGC_API_RECORDS", OGC_ID),
                (
                    "openlineage/e3-run-events.json",
                    OPENLINEAGE_EVENTS,
                    "OPENLINEAGE",
                    "0e70677e-e4b5-4a1f-b908-7e22ef00ed31",
                ),
            ],
            E1_CONTEXT,
            OPENLINEAGE_SCHEMA.as_bytes(),
        )
        .expect_err("source digest swap");
        assert!(error.contains("byte length/SHA-256 differs"));
    }

    #[test]
    fn e3_package_preserves_original_provider_source_bytes() {
        let report = run().expect("valid E3 package");
        assert!(report.provider_projections.iter().all(|source| source.source_bytes_retained));
        assert_eq!(report.unmapped_or_extension_paths.len(), 3);
    }

    #[test]
    fn openlineage_schema_asset_is_pinned_to_the_official_blob() {
        let manifest: Value = serde_json::from_str(MANIFEST).expect("manifest");
        let asset = &manifest["validation_assets"][0];
        assert_eq!(asset["source_repository"], "OpenLineage/OpenLineage");
        assert_eq!(asset["source_blob_sha"], OPENLINEAGE_SCHEMA_BLOB);
        assert_eq!(asset["schema_id"], OPENLINEAGE_SCHEMA_ID);
    }
}
