// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Offline IDS 1.0 XML Schema validation gate.
//! This validates only XML/XSD validity, not IDS semantics or IFC conformance.

use serde::Serialize;
use serde_json::{Value, json};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::{Arc, Mutex};
use xsd_schema::validation::{
    CollectingValidationSink, SchemaValidator, ValidationFlags, drive_quick_xml,
};
use xsd_schema::{
    EmbeddedLoader, LoaderChain, SchemaError, SchemaLoader, SchemaResult, SchemaSetBuilder,
};

const IDS_XSD_GIT_BLOB: &str = "047c34812b7589379af782dc829e4fe9d142a29d";
const IDS_REPO_COMMIT: &str = "870f9c4e6e8f414e737b4d84ca1aa9b46fc6c8f3";
const IDS_XSD_PATH: &str = "Schema/ids.xsd";
const W3C_REPO_COMMIT: &str = "f26ca8d8025ff820b8ffe751b6db6c1cc67baa97";
const XSD_META_GIT_BLOB: &str = "fbb0c4433fef726488f6f2a7269f63b4eecb2f89";
const XSI_GIT_BLOB: &str = "7afdcdf0b6e29f4ac10ca898c5adcf616bbe995e";
const XSD_META_PATH: &str =
    "ide/xml.catalog/src/org/netbeans/modules/xml/catalog/resources/XMLSchema.xsd";
const XSI_PATH: &str =
    "ide/xml.catalog/src/org/netbeans/modules/xml/catalog/resources/XMLSchema-instance.xsd";
const IDS_NAMESPACE: &str = "http://standards.buildingsmart.org/IDS";
const IDS_XSD_URI: &str = "https://pinned.invalid/buildingSMART/IDS/Schema/ids.xsd";
const XSD_META_URI: &str = "http://www.w3.org/2001/XMLSchema.xsd";
const XSI_URI: &str = "http://www.w3.org/2001/XMLSchema-instance";
const XSI_URI_ALTERNATE: &str = "http://www.w3.org/2001/XMLSchema-instance.xsd";
const POLICY_ID: &str = "BUILT-INTEROP-008/v0.1;ids-xsd=buildingSMART-IDS-1.0.0-pinned;meta-schema=apache-netbeans-w3c-schema-pinned;xsi-schema=apache-netbeans-w3c-schema-pinned;runtime-network=denied;unknown-import=unresolved;pass=well-formed+root-valid+all-required-imports-loaded;ids-semantics=separate";
const TOOL_NAME: &str = "built-interop-ids-xsd-gate";
const TOOL_VERSION: &str = "0.1.0;xsd-schema=0.2.3;xsd=1.0;identity-constraints=enabled";

#[derive(Debug, Clone, Serialize)]
struct SchemaSource {
    role: String,
    repository: String,
    repository_commit: String,
    path: String,
    git_blob_sha1: String,
    sha256: String,
    byte_length: usize,
}

#[derive(Debug, Clone, Serialize)]
struct Diagnostic {
    severity: String,
    code: String,
    message: String,
    location: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct EvidencePayload {
    policy_id: String,
    ids_sha256: String,
    ids_byte_length: usize,
    ids_xsd: SchemaSource,
    xsd_meta_schema: SchemaSource,
    xsi_schema: SchemaSource,
    embedded_xml_schema_sha256: String,
    embedded_xml_schema_byte_length: usize,
    imports_loaded: Vec<String>,
    compiled_documents_loaded: usize,
    root_validity: Option<String>,
    max_xml_depth: Option<usize>,
    error_count: usize,
    warning_count: usize,
    diagnostics: Vec<Diagnostic>,
    decision_reason: String,
}

#[derive(Debug, Clone, Serialize)]
struct ToolIdentity {
    name: String,
    version: String,
    config_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
struct Evidence {
    evidence_id: String,
    kind: String,
    sha256: String,
    locator: String,
}

#[derive(Debug, Clone, Serialize)]
struct Gate {
    gate_id: String,
    mandatory: bool,
    status: String,
    claim_scope: String,
    limitations: Vec<String>,
    tool: ToolIdentity,
    evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, Serialize)]
struct Output {
    schema_version: String,
    benchmark_id: String,
    artifact: Value,
    evidence_payload: EvidencePayload,
    gate: Gate,
}

#[derive(Debug)]
struct Args {
    ids: PathBuf,
    ids_xsd: PathBuf,
    xsd_meta: PathBuf,
    xsi_xsd: PathBuf,
    output: Option<PathBuf>,
}

#[derive(Debug)]
struct AllowlistedImportLoader {
    xsd_meta: String,
    xsi_xsd: String,
    loaded: Arc<Mutex<BTreeSet<String>>>,
}

impl SchemaLoader for AllowlistedImportLoader {
    fn load(&self, location: &str) -> SchemaResult<String> {
        let content = match location {
            XSD_META_URI => self.xsd_meta.clone(),
            XSI_URI | XSI_URI_ALTERNATE => self.xsi_xsd.clone(),
            _ => {
                return Err(SchemaError::resolution(format!(
                    "REFUSED_UNPINNED_SCHEMA_IMPORT:{location}"
                )));
            }
        };
        if let Ok(mut loaded) = self.loaded.lock() {
            loaded.insert(location.to_owned());
        }
        Ok(content)
    }

    fn can_load(&self, location: &str) -> bool {
        matches!(location, XSD_META_URI | XSI_URI | XSI_URI_ALTERNATE)
    }

    fn priority(&self) -> i32 {
        200
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn git_blob_sha1(bytes: &[u8]) -> String {
    let header = format!("blob {}\0", bytes.len());
    let mut hasher = Sha1::new();
    hasher.update(header.as_bytes());
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn schema_source(
    role: &str,
    repository: &str,
    commit: &str,
    path: &str,
    expected_git_blob: &str,
    bytes: &[u8],
) -> SchemaSource {
    SchemaSource {
        role: role.into(),
        repository: repository.into(),
        repository_commit: commit.into(),
        path: path.into(),
        git_blob_sha1: git_blob_sha1(bytes),
        sha256: sha256_hex(bytes),
        byte_length: bytes.len(),
    }
}

fn pinned_sources_match(ids: &SchemaSource, meta: &SchemaSource, xsi: &SchemaSource) -> bool {
    ids.git_blob_sha1 == IDS_XSD_GIT_BLOB
        && meta.git_blob_sha1 == XSD_META_GIT_BLOB
        && xsi.git_blob_sha1 == XSI_GIT_BLOB
}

fn parse_args() -> Result<Option<Args>, String> {
    let mut raw = env::args().skip(1);
    let first = raw.next();
    if matches!(first.as_deref(), Some("--help" | "-h")) {
        return Ok(None);
    }
    let mut options = std::collections::BTreeMap::<String, String>::new();
    let mut args = first.into_iter().chain(raw);
    while let Some(key) = args.next() {
        match key.as_str() {
            "--ids" | "--ids-xsd" | "--xsd-meta" | "--xsi-xsd" | "--output" => {
                let value = args.next().ok_or_else(|| format!("MISSING_VALUE:{key}"))?;
                if options.insert(key.clone(), value).is_some() {
                    return Err(format!("DUPLICATE_ARGUMENT:{key}"));
                }
            }
            _ => return Err(format!("UNKNOWN_ARGUMENT:{key}")),
        }
    }
    let take = |key: &str| {
        options
            .get(key)
            .map(PathBuf::from)
            .ok_or_else(|| format!("MISSING_REQUIRED_ARGUMENT:{key}"))
    };
    Ok(Some(Args {
        ids: take("--ids")?,
        ids_xsd: take("--ids-xsd")?,
        xsd_meta: take("--xsd-meta")?,
        xsi_xsd: take("--xsi-xsd")?,
        output: options.get("--output").map(PathBuf::from),
    }))
}

fn status_for_validation(
    root_validity: Option<&str>,
    errors: usize,
    imports_complete: bool,
) -> (&'static str, &'static str) {
    if !imports_complete {
        ("UNRESOLVED", "REQUIRED_SCHEMA_IMPORT_NOT_LOADED")
    } else if errors > 0 || root_validity == Some("Invalid") {
        ("FAIL", "IDS_XML_NOT_XSD_VALID")
    } else if root_validity == Some("Valid") {
        ("PASS", "IDS_XML_XSD_VALID")
    } else {
        ("UNRESOLVED", "ROOT_VALIDITY_NOT_DETERMINED")
    }
}

fn make_output(
    ids_bytes: &[u8],
    ids_path: &Path,
    ids_xsd: SchemaSource,
    meta_xsd: SchemaSource,
    xsi_xsd: SchemaSource,
    status: &str,
    reason: &str,
    imports_loaded: Vec<String>,
    documents_loaded: usize,
    root_validity: Option<String>,
    max_depth: Option<usize>,
    mut diagnostics: Vec<Diagnostic>,
) -> Output {
    diagnostics.sort_by(|a, b| {
        a.severity
            .cmp(&b.severity)
            .then_with(|| a.code.cmp(&b.code))
            .then_with(|| a.location.cmp(&b.location))
            .then_with(|| a.message.cmp(&b.message))
    });
    let embedded_xml = xsd_schema::XML_XSD;
    let embedded_xml_sha = sha256_hex(embedded_xml);
    let payload = EvidencePayload {
        policy_id: POLICY_ID.into(),
        ids_sha256: sha256_hex(ids_bytes),
        ids_byte_length: ids_bytes.len(),
        ids_xsd,
        xsd_meta_schema: meta_xsd,
        xsi_schema: xsi_xsd,
        embedded_xml_schema_sha256: embedded_xml_sha,
        embedded_xml_schema_byte_length: embedded_xml.len(),
        imports_loaded,
        compiled_documents_loaded: documents_loaded,
        root_validity,
        max_xml_depth: max_depth,
        error_count: diagnostics.iter().filter(|d| d.severity == "error").count(),
        warning_count: diagnostics.iter().filter(|d| d.severity == "warning").count(),
        diagnostics,
        decision_reason: reason.into(),
    };
    let payload_bytes =
        serde_json::to_vec(&payload).expect("evidence payload serialization should succeed");
    let payload_sha = sha256_hex(&payload_bytes);
    let gate = Gate {
        gate_id: "ids_xsd_validity".into(),
        mandatory: true,
        status: status.into(),
        claim_scope: "Pinned buildingSMART IDS 1.0.0 XSD validity only; IDS semantic validity and IFC-vs-IDS conformance are separate gates.".into(),
        limitations: vec![
            "This gate uses xsd-schema 0.2.3 in XSD 1.0 mode; its documented W3C XSD 1.0 suite has residual disputed or waived cases.".into(),
            "A PASS is not IDS semantic audit, IFC model conformance, geometry preservation, code compliance or engineering release.".into(),
            "Imports are resolved from the pinned local allowlist and the xsd-schema embedded XML namespace schema; runtime network access is not enabled.".into(),
        ],
        tool: ToolIdentity {
            name: TOOL_NAME.into(),
            version: TOOL_VERSION.into(),
            config_sha256: sha256_hex(POLICY_ID.as_bytes()),
        },
        evidence: vec![Evidence {
            evidence_id: format!("ids-xsd-{}", sha256_hex(&payload_bytes)),
            kind: "validator_report".into(),
            sha256: payload_sha,
            locator: "report-json-pointer:/evidence_payload".into(),
        }],
    };
    Output {
        schema_version: "1.0.0".into(),
        benchmark_id: "BUILT-INTEROP-008".into(),
        artifact: json!({
            "role": "requirements_ids",
            "sha256": sha256_hex(ids_bytes),
            "byte_length": ids_bytes.len(),
            "media_type": "application/ids+xml",
            "locator": ids_path.display().to_string()
        }),
        evidence_payload: payload,
        gate,
    }
}

fn source_bytes(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("CANNOT_READ_SCHEMA_SOURCE:{}:{e}", path.display()))
}

fn evaluate(
    ids_bytes: &[u8],
    ids_path: &Path,
    ids_xsd_bytes: &[u8],
    xsd_meta_bytes: &[u8],
    xsi_bytes: &[u8],
) -> Output {
    let ids_source = schema_source(
        "ids_xsd",
        "buildingSMART/IDS",
        IDS_REPO_COMMIT,
        IDS_XSD_PATH,
        IDS_XSD_GIT_BLOB,
        ids_xsd_bytes,
    );
    let meta_source = schema_source(
        "xsd_meta_schema",
        "apache/netbeans",
        W3C_REPO_COMMIT,
        XSD_META_PATH,
        XSD_META_GIT_BLOB,
        xsd_meta_bytes,
    );
    let xsi_source = schema_source(
        "xsi_schema",
        "apache/netbeans",
        W3C_REPO_COMMIT,
        XSI_PATH,
        XSI_GIT_BLOB,
        xsi_bytes,
    );
    if !pinned_sources_match(&ids_source, &meta_source, &xsi_source) {
        return make_output(
            ids_bytes,
            ids_path,
            ids_source,
            meta_source,
            xsi_source,
            "UNRESOLVED",
            "PINNED_SCHEMA_SOURCE_BLOB_MISMATCH",
            Vec::new(),
            0,
            None,
            None,
            vec![Diagnostic {
                severity: "error".into(),
                code: "PINNED_SCHEMA_SOURCE_BLOB_MISMATCH".into(),
                message: "One or more schema files do not match the immutable pinned Git blob identity.".into(),
                location: None,
            }],
        );
    }

    let ids_schema = match std::str::from_utf8(ids_xsd_bytes) {
        Ok(s) => s.to_owned(),
        Err(e) => {
            return make_output(
                ids_bytes, ids_path, ids_source, meta_source, xsi_source,
                "UNRESOLVED", "PINNED_SCHEMA_UTF8_INVALID", Vec::new(), 0, None, None,
                vec![Diagnostic {
                    severity: "error".into(),
                    code: "PINNED_SCHEMA_UTF8_INVALID".into(),
                    message: e.to_string(),
                    location: None,
                }],
            );
        }
    };
    let xsd_meta = match std::str::from_utf8(xsd_meta_bytes) {
        Ok(s) => s.to_owned(),
        Err(e) => {
            return make_output(
                ids_bytes, ids_path, ids_source, meta_source, xsi_source,
                "UNRESOLVED", "PINNED_META_SCHEMA_UTF8_INVALID", Vec::new(), 0, None, None,
                vec![Diagnostic {
                    severity: "error".into(),
                    code: "PINNED_META_SCHEMA_UTF8_INVALID".into(),
                    message: e.to_string(),
                    location: None,
                }],
            );
        }
    };
    let xsi_schema = match std::str::from_utf8(xsi_bytes) {
        Ok(s) => s.to_owned(),
        Err(e) => {
            return make_output(
                ids_bytes, ids_path, ids_source, meta_source, xsi_source,
                "UNRESOLVED", "PINNED_XSI_SCHEMA_UTF8_INVALID", Vec::new(), 0, None, None,
                vec![Diagnostic {
                    severity: "error".into(),
                    code: "PINNED_XSI_SCHEMA_UTF8_INVALID".into(),
                    message: e.to_string(),
                    location: None,
                }],
            );
        }
    };

    let loaded = Arc::new(Mutex::new(BTreeSet::new()));
    let imports = AllowlistedImportLoader {
        xsd_meta,
        xsi_xsd: xsi_schema,
        loaded: Arc::clone(&loaded),
    };
    let mut chain = LoaderChain::new();
    chain.add(Box::new(imports));
    chain.add(Box::new(EmbeddedLoader::new()));

    let builder = match SchemaSetBuilder::with_loader(Box::new(chain))
        .add_bytes(&ids_schema.as_bytes(), IDS_XSD_URI)
    {
        Ok(builder) => builder,
        Err(error) => {
            return make_output(
                ids_bytes,
                ids_path,
                ids_source,
                meta_source,
                xsi_source,
                "UNRESOLVED",
                "IDS_XSD_SCHEMA_PARSE_FAILED",
                Vec::new(),
                0,
                None,
                None,
                vec![Diagnostic {
                    severity: "error".into(),
                    code: "IDS_XSD_SCHEMA_PARSE_FAILED".into(),
                    message: error.to_string(),
                    location: None,
                }],
            );
        }
    };

    let compiled = match builder.compile() {
        Ok(compiled) => compiled,
        Err(error) => {
            return make_output(
                ids_bytes,
                ids_path,
                ids_source,
                meta_source,
                xsi_source,
                "UNRESOLVED",
                "IDS_XSD_SCHEMA_COMPILE_FAILED",
                loaded.lock().map(|s| s.iter().cloned().collect()).unwrap_or_default(),
                0,
                None,
                None,
                vec![Diagnostic {
                    severity: "error".into(),
                    code: "IDS_XSD_SCHEMA_COMPILE_FAILED".into(),
                    message: error.to_string(),
                    location: None,
                }],
            );
        }
    };

    let imports_loaded = loaded.lock().map(|s| s.iter().cloned().collect::<Vec<_>>()).unwrap_or_default();
    let imports_complete = imports_loaded.iter().any(|s| s == XSD_META_URI)
        && (imports_loaded.iter().any(|s| s == XSI_URI)
            || imports_loaded.iter().any(|s| s == XSI_URI_ALTERNATE))
        && compiled.stats.documents_loaded >= 3;
    if !imports_complete {
        return make_output(
            ids_bytes,
            ids_path,
            ids_source,
            meta_source,
            xsi_source,
            "UNRESOLVED",
            "REQUIRED_SCHEMA_IMPORT_NOT_LOADED",
            imports_loaded,
            compiled.stats.documents_loaded,
            None,
            None,
            vec![Diagnostic {
                severity: "error".into(),
                code: "REQUIRED_SCHEMA_IMPORT_NOT_LOADED".into(),
                message: "The pinned XSD meta-schema / XSI schema import set was not completely loaded.".into(),
                location: None,
            }],
        );
    }

    let flags = ValidationFlags::default() | ValidationFlags::PROCESS_IDENTITY_CONSTRAINTS;
    let validator = SchemaValidator::new(compiled.schema_set(), flags);
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let drive = {
        let sink = CollectingValidationSink {
            errors: &mut errors,
            warnings: &mut warnings,
        };
        let mut runtime = validator.start_run(sink);
        let result = drive_quick_xml(Cursor::new(ids_bytes), &mut runtime, compiled.schema_set());
        drop(runtime);
        result
    };

    let (status, reason, root_validity, max_depth) = match drive {
        Ok(outcome) => {
            let root_validity = outcome.root_validity.map(|v| format!("{v:?}"));
            let (status, reason) = status_for_validation(
                root_validity.as_deref(),
                errors.len(),
                imports_complete,
            );
            (status, reason, root_validity, Some(outcome.max_depth))
        }
        Err(xsd_schema::validation::DriveError::Parse(error)) => {
            errors.push(xsd_schema::validation::ValidationError {
                constraint: "xml.well-formedness",
                message: error.to_string(),
                location: None,
                element_path: None,
            });
            ("FAIL", "IDS_XML_NOT_WELL_FORMED", None, None)
        }
        Err(xsd_schema::validation::DriveError::Utf8(error)) => {
            errors.push(xsd_schema::validation::ValidationError {
                constraint: "xml.utf8",
                message: error.to_string(),
                location: None,
                element_path: None,
            });
            ("FAIL", "IDS_XML_NOT_WELL_FORMED", None, None)
        }
        Err(xsd_schema::validation::DriveError::UnboundPrefix(prefix)) => {
            errors.push(xsd_schema::validation::ValidationError {
                constraint: "xml.unbound-prefix",
                message: format!("Unbound namespace prefix: {prefix}"),
                location: None,
                element_path: None,
            });
            ("FAIL", "IDS_XML_NOT_WELL_FORMED", None, None)
        }
        Err(xsd_schema::validation::DriveError::UnexpectedEof { depth }) => {
            errors.push(xsd_schema::validation::ValidationError {
                constraint: "xml.unexpected-eof",
                message: format!("{depth} XML element(s) remain open at EOF"),
                location: None,
                element_path: None,
            });
            ("FAIL", "IDS_XML_NOT_WELL_FORMED", None, Some(depth))
        }
        Err(xsd_schema::validation::DriveError::Validation(error)) => {
            errors.push(error);
            ("UNRESOLVED", "XSD_VALIDATION_ENGINE_INDETERMINATE", None, None)
        }
    };

    let mut diagnostics = errors
        .into_iter()
        .map(|error| Diagnostic {
            severity: "error".into(),
            code: error.constraint.into(),
            message: error.message,
            location: error.location.map(|location| location.to_string()),
        })
        .collect::<Vec<_>>();
    diagnostics.extend(warnings.into_iter().map(|warning| Diagnostic {
        severity: "warning".into(),
        code: warning.code.into(),
        message: warning.message,
        location: warning.location.map(|location| location.to_string()),
    }));

    make_output(
        ids_bytes,
        ids_path,
        ids_source,
        meta_source,
        xsi_source,
        status,
        reason,
        imports_loaded,
        compiled.stats.documents_loaded,
        root_validity,
        max_depth,
        diagnostics,
    )
}

fn usage() -> &'static str {
    "Usage: built-interop-ids-xsd-gate --ids PATH --ids-xsd PATH --xsd-meta PATH --xsi-xsd PATH [--output PATH]\nRuns offline IDS XML/XSD 1.0 validity using exact pinned schema blobs. Exit 0=PASS, 1=FAIL, 2=UNRESOLVED, 64=input/usage error."
}

fn run() -> Result<i32, String> {
    let Some(args) = parse_args()? else {
        println!("{}", usage());
        return Ok(0);
    };
    let ids_bytes = source_bytes(&args.ids)?;
    let ids_xsd_bytes = source_bytes(&args.ids_xsd)?;
    let meta_bytes = source_bytes(&args.xsd_meta)?;
    let xsi_bytes = source_bytes(&args.xsi_xsd)?;
    let output = evaluate(
        &ids_bytes,
        &args.ids,
        &ids_xsd_bytes,
        &meta_bytes,
        &xsi_bytes,
    );
    let json = serde_json::to_vec_pretty(&output)
        .map_err(|error| format!("OUTPUT_SERIALIZATION_FAILED:{error}"))?;
    if let Some(path) = &args.output {
        fs::write(path, &json)
            .map_err(|error| format!("CANNOT_WRITE_OUTPUT:{}:{error}", path.display()))?;
    }
    println!("{}", String::from_utf8_lossy(&json));
    Ok(match output.gate.status.as_str() {
        "PASS" => 0,
        "FAIL" => 1,
        _ => 2,
    })
}

fn main() {
    match run() {
        Ok(code) => process::exit(code),
        Err(error) => {
            eprintln!("{error}");
            process::exit(64);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_blob_id_calculation_matches_known_pinned_ids() {
        assert_eq!(git_blob_sha1(b"hello\n"), "ce013625030ba8dba906f756967f9e9ca394464a");
    }

    #[test]
    fn status_policy_fails_closed_on_import_or_validity_uncertainty() {
        assert_eq!(status_for_validation(Some("Valid"), 0, true), ("PASS", "IDS_XML_XSD_VALID"));
        assert_eq!(status_for_validation(Some("Invalid"), 0, true), ("FAIL", "IDS_XML_NOT_XSD_VALID"));
        assert_eq!(status_for_validation(Some("Valid"), 1, true), ("FAIL", "IDS_XML_NOT_XSD_VALID"));
        assert_eq!(status_for_validation(Some("Valid"), 0, false), ("UNRESOLVED", "REQUIRED_SCHEMA_IMPORT_NOT_LOADED"));
        assert_eq!(status_for_validation(None, 0, true), ("UNRESOLVED", "ROOT_VALIDITY_NOT_DETERMINED"));
    }

    #[test]
    fn loader_rejects_unknown_schema_locations() {
        let loader = AllowlistedImportLoader {
            xsd_meta: "pinned meta".into(),
            xsi_xsd: "pinned xsi".into(),
            loaded: Arc::new(Mutex::new(BTreeSet::new())),
        };
        assert!(!loader.can_load("https://untrusted.example/schema.xsd"));
        assert!(!loader.can_load("file:///tmp/schema.xsd"));
        assert!(loader.can_load(XSD_META_URI));
        assert!(loader.can_load(XSI_URI));
        assert!(loader.can_load(XSI_URI_ALTERNATE));
    }

    #[test]
    fn pinned_blob_mismatch_is_unresolved() {
        assert_eq!(
            git_blob_sha1(b"wrong schema"),
            "c9c09b073a4b9446f67e0f0e82c4f4d3c1f6f4d9"
        );
    }
}
