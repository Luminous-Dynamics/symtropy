// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Offline, digest-pinned OPC UA UANodeSet admission and dependency analysis.
//!
//! This module never fetches URLs, imports a model into a server, or invokes an
//! OPC UA method. Callers supply the exact local XML bytes and a reviewed manifest.
//! DTD/DOCTYPE declarations are rejected to keep XML entity processing out of scope.

use quick_xml::{
    XmlVersion,
    events::{BytesStart, Event},
    reader::Reader,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const UANODESET_NAMESPACE: &str = "http://opcfoundation.org/UA/2011/03/UANodeSet.xsd";
const OPC_UA_CORE_NAMESPACE: &str = "http://opcfoundation.org/UA/";
const MAX_NODESET_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredModel {
    pub model_uri: String,
    pub version: String,
    pub publication_date: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSetModel {
    pub model_uri: String,
    pub version: String,
    pub model_version: Option<String>,
    pub publication_date: String,
    pub required_models: Vec<RequiredModel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedNodeSet {
    pub namespace_uris: Vec<String>,
    pub models: Vec<NodeSetModel>,
    pub checked_node_id_references: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSetAdmissionReport {
    pub admitted: bool,
    pub model_uri: String,
    pub model_version: String,
    pub model_publication_date: String,
    pub artifact_sha256: String,
    pub namespace_uri_count: usize,
    pub required_model_count: usize,
    pub checked_node_id_references: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencySkew {
    pub model_uri: String,
    pub requirements: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleReport {
    pub ready_for_composition: bool,
    pub status: &'static str,
    pub dependency_skews: Vec<DependencySkew>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalNodeId {
    pub namespace_uri: String,
    pub identifier_type: char,
    pub identifier: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceMetadataDisposition {
    pub metadata_matches: bool,
    pub subset: bool,
    pub disposition: &'static str,
    pub mismatches: Vec<String>,
}

fn attr_map(element: &BytesStart<'_>) -> Result<BTreeMap<String, String>, String> {
    let mut result = BTreeMap::new();
    for attribute in element.attributes().with_checks(true) {
        let attribute = attribute.map_err(|error| format!("invalid XML attribute: {error}"))?;
        let key = attribute.key.as_ref().to_owned();
        let value = attribute
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|error| format!("invalid XML attribute {key}: {error}"))?
            .into_owned();
        if result.insert(key.clone(), value).is_some() {
            return Err(format!("duplicate XML attribute {key:?}"));
        }
    }
    Ok(result)
}

fn required_attr<'a>(
    attributes: &'a BTreeMap<String, String>,
    name: &str,
    context: &str,
) -> Result<&'a str, String> {
    attributes
        .get(name)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{context}: missing or empty {name} attribute"))
}

fn parse_required_model(attributes: &BTreeMap<String, String>) -> Result<RequiredModel, String> {
    Ok(RequiredModel {
        model_uri: required_attr(attributes, "ModelUri", "RequiredModel")?.to_owned(),
        version: required_attr(attributes, "Version", "RequiredModel")?.to_owned(),
        publication_date: required_attr(attributes, "PublicationDate", "RequiredModel")?.to_owned(),
    })
}

fn parse_model(attributes: &BTreeMap<String, String>) -> Result<NodeSetModel, String> {
    Ok(NodeSetModel {
        model_uri: required_attr(attributes, "ModelUri", "Model")?.to_owned(),
        version: required_attr(attributes, "Version", "Model")?.to_owned(),
        model_version: attributes.get("ModelVersion").cloned(),
        publication_date: required_attr(attributes, "PublicationDate", "Model")?.to_owned(),
        required_models: Vec::new(),
    })
}

fn check_nodeset_namespace_index(
    value: &str,
    namespace_uris: &[String],
    context: &str,
) -> Result<(), String> {
    if let Some(rest) = value.strip_prefix("ns=") {
        let (index, _) = rest
            .split_once(';')
            .ok_or_else(|| format!("{context}: malformed namespace-indexed NodeId {value:?}"))?;
        let index: usize = index
            .parse()
            .map_err(|_| format!("{context}: invalid namespace index in {value:?}"))?;
        if index > namespace_uris.len() {
            return Err(format!(
                "{context}: NodeSet namespace index {index} is outside 0..={} for {value:?}",
                namespace_uris.len()
            ));
        }
    }
    Ok(())
}

fn check_browse_name_index(
    value: &str,
    namespace_uris: &[String],
    context: &str,
) -> Result<(), String> {
    if let Some((prefix, _)) = value.split_once(':')
        && let Ok(index) = prefix.parse::<usize>()
        && index > namespace_uris.len()
    {
        return Err(format!(
            "{context}: BrowseName namespace index {index} is outside 0..={} for {value:?}",
            namespace_uris.len()
        ));
    }
    Ok(())
}

fn resolve_node_id_reference<'a>(
    value: &'a str,
    aliases: &'a BTreeMap<String, String>,
    context: &str,
) -> Result<&'a str, String> {
    if value.starts_with("ns=")
        || value.starts_with("nsu=")
        || value.starts_with("i=")
        || value.starts_with("s=")
        || value.starts_with("g=")
        || value.starts_with("b=")
    {
        return Ok(value);
    }
    aliases
        .get(value)
        .map(String::as_str)
        .ok_or_else(|| format!("{context}: unknown UANodeSet Alias {value:?}"))
}

fn check_node_attributes(
    element: &BytesStart<'_>,
    namespace_uris: &[String],
    aliases: &BTreeMap<String, String>,
) -> Result<usize, String> {
    let attributes = attr_map(element)?;
    let mut checked = 0usize;
    for field in ["NodeId", "ParentNodeId", "DataType"] {
        if let Some(value) = attributes.get(field) {
            let resolved = resolve_node_id_reference(value, aliases, field)?;
            check_nodeset_namespace_index(resolved, namespace_uris, field)?;
            checked += if resolved.starts_with("ns=") { 1 } else { 0 };
        }
    }
    if let Some(value) = attributes.get("BrowseName") {
        check_browse_name_index(value, namespace_uris, "BrowseName")?;
        checked += if value.contains(':') { 1 } else { 0 };
    }
    Ok(checked)
}

/// Parse a UANodeSet XML document without resolving external entities or DTDs.
pub fn parse_nodeset_xml(xml: &[u8]) -> Result<ParsedNodeSet, String> {
    if xml.is_empty() || xml.len() > MAX_NODESET_BYTES {
        return Err(format!(
            "NodeSet byte length must be between 1 and {MAX_NODESET_BYTES}"
        ));
    }
    let source =
        std::str::from_utf8(xml).map_err(|error| format!("NodeSet XML must be UTF-8: {error}"))?;
    let mut reader = Reader::from_str(source);
    reader.config_mut().trim_text(true);

    let mut root_seen = false;
    let mut root_namespace_ok = false;
    let mut in_namespace_uris = false;
    let mut collecting_uri = false;
    let mut current_uri = String::new();
    let mut namespace_uris = Vec::new();
    let mut in_aliases = false;
    let mut collecting_alias = false;
    let mut current_alias_name = String::new();
    let mut current_alias_value = String::new();
    let mut aliases = BTreeMap::new();
    let mut models = Vec::new();
    let mut current_model: Option<NodeSetModel> = None;
    let mut collecting_reference = false;
    let mut current_reference = String::new();
    let mut checked_node_id_references = 0usize;

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                let name = element.local_name();
                match name.as_ref() {
                    "UANodeSet" => {
                        if root_seen {
                            return Err("multiple UANodeSet roots".to_owned());
                        }
                        root_seen = true;
                        let attrs = attr_map(&element)?;
                        root_namespace_ok =
                            attrs.get("xmlns").map(String::as_str) == Some(UANODESET_NAMESPACE);
                    }
                    "NamespaceUris" => in_namespace_uris = true,
                    "Aliases" => in_aliases = true,
                    "Alias" if in_aliases => {
                        if collecting_alias {
                            return Err("nested Alias elements are not permitted".to_owned());
                        }
                        let attributes = attr_map(&element)?;
                        current_alias_name =
                            required_attr(&attributes, "Alias", "Alias")?.to_owned();
                        if aliases.contains_key(&current_alias_name) {
                            return Err(format!(
                                "duplicate UANodeSet Alias {:?}",
                                current_alias_name
                            ));
                        }
                        current_alias_value.clear();
                        collecting_alias = true;
                    }
                    "Uri" if in_namespace_uris => {
                        if collecting_uri {
                            return Err("nested Uri element inside NamespaceUris".to_owned());
                        }
                        collecting_uri = true;
                        current_uri.clear();
                    }
                    "Model" => {
                        if current_model.is_some() {
                            return Err("nested Model elements are not permitted".to_owned());
                        }
                        current_model = Some(parse_model(&attr_map(&element)?)?);
                    }
                    "Reference" => {
                        if collecting_reference {
                            return Err("nested Reference elements are not permitted".to_owned());
                        }
                        let attributes = attr_map(&element)?;
                        if let Some(reference_type) = attributes.get("ReferenceType") {
                            let resolved = resolve_node_id_reference(
                                reference_type,
                                &aliases,
                                "ReferenceType",
                            )?;
                            check_nodeset_namespace_index(
                                resolved,
                                &namespace_uris,
                                "ReferenceType",
                            )?;
                            checked_node_id_references +=
                                if resolved.starts_with("ns=") { 1 } else { 0 };
                        }
                        collecting_reference = true;
                        current_reference.clear();
                    }
                    "RequiredModel" => {
                        let required = parse_required_model(&attr_map(&element)?)?;
                        let model = current_model
                            .as_mut()
                            .ok_or_else(|| "RequiredModel appears outside a Model".to_owned())?;
                        model.required_models.push(required);
                    }
                    "UAObject" | "UAObjectType" | "UAVariable" | "UAVariableType"
                    | "UADataType" | "UAReferenceType" | "UAView" | "UAMethod" => {
                        checked_node_id_references +=
                            check_node_attributes(&element, &namespace_uris, &aliases)?;
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(element)) => {
                let name = element.local_name();
                match name.as_ref() {
                    "RequiredModel" => {
                        let required = parse_required_model(&attr_map(&element)?)?;
                        let model = current_model
                            .as_mut()
                            .ok_or_else(|| "RequiredModel appears outside a Model".to_owned())?;
                        model.required_models.push(required);
                    }
                    "Model" => models.push(parse_model(&attr_map(&element)?)?),
                    "UAObject" | "UAObjectType" | "UAVariable" | "UAVariableType"
                    | "UADataType" | "UAReferenceType" | "UAView" | "UAMethod" => {
                        checked_node_id_references +=
                            check_node_attributes(&element, &namespace_uris, &aliases)?;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(text)) if collecting_uri => {
                let value = text.xml_content(XmlVersion::Implicit1_0);
                current_uri.push_str(value.as_ref());
            }
            Ok(Event::Text(text)) if collecting_reference => {
                let value = text.xml_content(XmlVersion::Implicit1_0);
                current_reference.push_str(value.as_ref());
            }
            Ok(Event::Text(text)) if collecting_alias => {
                let value = text.xml_content(XmlVersion::Implicit1_0);
                current_alias_value.push_str(value.as_ref());
            }
            Ok(Event::End(element)) => match element.local_name().as_ref() {
                "Uri" if collecting_uri => {
                    let uri = current_uri.trim().to_owned();
                    if uri.is_empty() {
                        return Err("empty namespace URI in NamespaceUris".to_owned());
                    }
                    namespace_uris.push(uri);
                    collecting_uri = false;
                }
                "NamespaceUris" => in_namespace_uris = false,
                "Aliases" => in_aliases = false,
                "Alias" if collecting_alias => {
                    let value = current_alias_value.trim().to_owned();
                    if value.is_empty() {
                        return Err(format!(
                            "UANodeSet Alias {:?} has no NodeId value",
                            current_alias_name
                        ));
                    }
                    check_nodeset_namespace_index(&value, &namespace_uris, "Alias value")?;
                    aliases.insert(current_alias_name.clone(), value);
                    collecting_alias = false;
                }
                "Reference" if collecting_reference => {
                    let target = current_reference.trim();
                    if target.is_empty() {
                        return Err("Reference element has no target NodeId".to_owned());
                    }
                    let resolved = resolve_node_id_reference(target, &aliases, "Reference target")?;
                    check_nodeset_namespace_index(resolved, &namespace_uris, "Reference target")?;
                    checked_node_id_references += if resolved.starts_with("ns=") { 1 } else { 0 };
                    collecting_reference = false;
                }
                "Model" => {
                    let model = current_model
                        .take()
                        .ok_or_else(|| "Model close without a Model start".to_owned())?;
                    models.push(model);
                }
                _ => {}
            },
            Ok(Event::DocType(_)) => {
                return Err(
                    "DOCTYPE declarations are prohibited in admitted NodeSet XML".to_owned(),
                );
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => {
                return Err(format!(
                    "malformed NodeSet XML at byte {}: {error}",
                    reader.error_position()
                ));
            }
        }
    }

    if !root_seen || !root_namespace_ok {
        return Err(format!(
            "root element must be UANodeSet in namespace {UANODESET_NAMESPACE}"
        ));
    }
    if collecting_uri || current_model.is_some() {
        return Err("unterminated NamespaceUris or Model element".to_owned());
    }
    if namespace_uris.is_empty() {
        return Err("NodeSet has no NamespaceUris".to_owned());
    }
    let unique_uris: BTreeSet<&String> = namespace_uris.iter().collect();
    if unique_uris.len() != namespace_uris.len() {
        return Err("NodeSet NamespaceUris contains duplicate URIs".to_owned());
    }
    if models.is_empty() {
        return Err("NodeSet has no Model records".to_owned());
    }
    for model in &models {
        let unique_required: BTreeSet<&String> = model
            .required_models
            .iter()
            .map(|entry| &entry.model_uri)
            .collect();
        if unique_required.len() != model.required_models.len() {
            return Err(format!(
                "Model {} contains duplicate RequiredModel URIs",
                model.model_uri
            ));
        }
    }

    Ok(ParsedNodeSet {
        namespace_uris,
        models,
        checked_node_id_references,
    })
}

/// SHA-256 over the exact artifact bytes. Implemented locally to keep the
/// admission binary's cryptographic dependency surface minimal.
pub fn sha256_hex(input: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let bit_len = (input.len() as u64).wrapping_mul(8);
    let mut data = input.to_vec();
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());

    let mut h = [
        0x6a09e667u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    let rotate = |value: u32, amount: u32| value.rotate_right(amount);

    for chunk in data.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (index, word) in chunk.chunks_exact(4).take(16).enumerate() {
            w[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for index in 16..64 {
            let x = w[index - 15];
            let y = w[index - 2];
            let s0 = rotate(x, 7) ^ rotate(x, 18) ^ (x >> 3);
            let s1 = rotate(y, 17) ^ rotate(y, 19) ^ (y >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for index in 0..64 {
            let s1 = rotate(e, 6) ^ rotate(e, 11) ^ rotate(e, 25);
            let choose = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(choose)
                .wrapping_add(K[index])
                .wrapping_add(w[index]);
            let s0 = rotate(a, 2) ^ rotate(a, 13) ^ rotate(a, 22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(majority);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    h.iter().map(|word| format!("{word:08x}")).collect()
}

/// Git's SHA-1 object identity for exact blob bytes, including the canonical
/// `blob <length>\\0` object header.
pub fn git_blob_sha1_hex(input: &[u8]) -> String {
    let header = format!("blob {}\0", input.len());
    let mut data = Vec::with_capacity(header.len() + input.len() + 72);
    data.extend_from_slice(header.as_bytes());
    data.extend_from_slice(input);
    let bit_len = (data.len() as u64).wrapping_mul(8);
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());

    let mut h = [
        0x67452301u32,
        0xefcdab89,
        0x98badcfe,
        0x10325476,
        0xc3d2e1f0,
    ];
    for chunk in data.chunks_exact(64) {
        let mut words = [0u32; 80];
        for (index, word) in chunk.chunks_exact(4).take(16).enumerate() {
            words[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for index in 16..80 {
            words[index] =
                (words[index - 3] ^ words[index - 8] ^ words[index - 14] ^ words[index - 16])
                    .rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = h;
        for (index, word) in words.iter().enumerate() {
            let (function, constant) = match index {
                0..=19 => ((b & c) | ((!b) & d), 0x5a827999),
                20..=39 => (b ^ c ^ d, 0x6ed9eba1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1bbcdc),
                _ => (b ^ c ^ d, 0xca62c1d6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(function)
                .wrapping_add(e)
                .wrapping_add(constant)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }
    h.iter().map(|word| format!("{word:08x}")).collect()
}

fn manifest_model(manifest: &Value) -> Result<&Value, String> {
    manifest
        .pointer("/nodeset_artifact/node_set_model")
        .ok_or_else(|| "manifest missing nodeset_artifact.node_set_model".to_owned())
}

fn manifest_namespace_metadata(manifest: &Value) -> Result<&Value, String> {
    manifest
        .pointer("/namespace_metadata")
        .ok_or_else(|| "manifest missing namespace_metadata".to_owned())
}

fn manifest_string<'a>(value: &'a Value, path: &str) -> Result<&'a str, String> {
    value
        .pointer(path)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("manifest missing string {path}"))
}

pub fn verify_nodeset_artifact(
    xml: &[u8],
    manifest: &Value,
) -> Result<NodeSetAdmissionReport, Vec<String>> {
    let mut issues = Vec::new();
    let artifact = manifest
        .get("nodeset_artifact")
        .ok_or_else(|| vec!["manifest missing nodeset_artifact".to_owned()])?;
    let expected_digest = artifact
        .get("content_sha256")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let actual_digest = sha256_hex(xml);
    if expected_digest != actual_digest {
        issues.push(format!(
            "content SHA-256 mismatch: expected {expected_digest}, observed {actual_digest}"
        ));
    }
    let expected_git_blob = artifact
        .get("git_blob_sha1")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let actual_git_blob = git_blob_sha1_hex(xml);
    if expected_git_blob != actual_git_blob {
        issues.push(format!(
            "Git blob SHA-1 mismatch: expected {expected_git_blob}, observed {actual_git_blob}"
        ));
    }
    let expected_len = artifact
        .get("byte_length")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if expected_len != xml.len() as u64 {
        issues.push(format!(
            "byte length mismatch: expected {expected_len}, observed {}",
            xml.len()
        ));
    }

    let parsed = parse_nodeset_xml(xml).map_err(|error| vec![error])?;
    let expected_namespace = manifest_namespace_metadata(manifest).map_err(|error| vec![error])?;
    let expected_model = manifest_model(manifest).map_err(|error| vec![error])?;

    let expected_uri =
        manifest_string(expected_namespace, "/namespace_uri").map_err(|error| vec![error])?;
    let expected_version =
        manifest_string(expected_namespace, "/namespace_version").map_err(|error| vec![error])?;
    let expected_namespace_date =
        manifest_string(expected_namespace, "/namespace_publication_date")
            .map_err(|error| vec![error])?;
    let model_uri = expected_model
        .get("model_uri")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let model_version = expected_model
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let model_publication_date = expected_model
        .get("publication_date")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let model_model_version = expected_model
        .get("model_version")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if model_uri != expected_uri {
        issues.push("manifest model URI differs from NamespaceMetadata URI".to_owned());
    }
    if model_version != expected_version {
        issues.push(
            "NodeSet Model Version differs from NamespaceMetadata NamespaceVersion".to_owned(),
        );
    }
    let target_models: Vec<&NodeSetModel> = parsed
        .models
        .iter()
        .filter(|model| model.model_uri == model_uri)
        .collect();
    if target_models.len() != 1 {
        issues.push(format!(
            "expected exactly one Model for {model_uri}, found {}",
            target_models.len()
        ));
    } else {
        let actual = target_models[0];
        if actual.version != model_version {
            issues.push(format!(
                "NodeSet Model Version mismatch: expected {model_version}, observed {}",
                actual.version
            ));
        }
        if actual.model_version.as_deref() != Some(model_model_version) {
            issues.push(format!(
                "NodeSet ModelVersion mismatch: expected {model_model_version}, observed {:?}",
                actual.model_version
            ));
        }
        if actual.publication_date.split('T').next() != Some(model_publication_date) {
            issues.push(format!(
                "NodeSet Model PublicationDate mismatch: expected {model_publication_date}, observed {}",
                actual.publication_date
            ));
        }
        let expected_required: BTreeSet<(String, String, String)> = artifact
            .get("required_models")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|value| {
                Some((
                    value.get("model_uri")?.as_str()?.to_owned(),
                    value.get("version")?.as_str()?.to_owned(),
                    value.get("publication_date")?.as_str()?.to_owned(),
                ))
            })
            .collect();
        let actual_required: BTreeSet<(String, String, String)> = actual
            .required_models
            .iter()
            .map(|model| {
                (
                    model.model_uri.clone(),
                    model.version.clone(),
                    model
                        .publication_date
                        .split('T')
                        .next()
                        .unwrap_or("")
                        .to_owned(),
                )
            })
            .collect();
        if expected_required != actual_required {
            issues.push(format!(
                "RequiredModels mismatch: expected {expected_required:?}, observed {actual_required:?}"
            ));
        }
    }

    let expected_uris: Vec<String> = artifact
        .get("namespace_uris")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    if parsed.namespace_uris != expected_uris {
        issues.push(format!(
            "NamespaceUris mismatch: expected {expected_uris:?}, observed {:?}",
            parsed.namespace_uris
        ));
    }
    if parsed
        .namespace_uris
        .iter()
        .filter(|uri| uri.as_str() == expected_uri)
        .count()
        != 1
    {
        issues.push(format!(
            "expected namespace URI {expected_uri} exactly once in NodeSet NamespaceUris"
        ));
    }

    if !issues.is_empty() {
        return Err(issues);
    }

    let _ = expected_namespace_date; // Kept distinct from NodeSet Model PublicationDate by design.
    Ok(NodeSetAdmissionReport {
        admitted: true,
        model_uri: model_uri.to_owned(),
        model_version: model_model_version.to_owned(),
        model_publication_date: model_publication_date.to_owned(),
        artifact_sha256: actual_digest,
        namespace_uri_count: parsed.namespace_uris.len(),
        required_model_count: target_models[0].required_models.len(),
        checked_node_id_references: parsed.checked_node_id_references,
    })
}

pub fn analyze_node_set_bundle(manifests: &[Value]) -> BundleReport {
    let mut requirements: BTreeMap<String, BTreeSet<(String, String)>> = BTreeMap::new();
    for manifest in manifests {
        if let Some(required) = manifest
            .pointer("/nodeset_artifact/required_models")
            .and_then(Value::as_array)
        {
            for model in required {
                if let (Some(uri), Some(version)) = (
                    model.get("model_uri").and_then(Value::as_str),
                    model.get("version").and_then(Value::as_str),
                ) {
                    requirements.entry(uri.to_owned()).or_default().insert((
                        version.to_owned(),
                        model
                            .get("publication_date")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                    ));
                }
            }
        }
    }
    let dependency_skews: Vec<DependencySkew> = requirements
        .into_iter()
        .filter_map(|(model_uri, versions)| {
            (versions.len() > 1).then(|| DependencySkew {
                model_uri,
                requirements: versions.into_iter().collect(),
            })
        })
        .collect();
    let ready = dependency_skews.is_empty();
    BundleReport {
        ready_for_composition: ready,
        status: if ready {
            "dependency_set_consistent"
        } else {
            "requires_explicit_dependency_resolution"
        },
        dependency_skews,
    }
}

pub fn canonicalize_node_id(
    value: &str,
    namespace_array: &[String],
) -> Result<CanonicalNodeId, String> {
    if namespace_array.is_empty() || namespace_array[0] != OPC_UA_CORE_NAMESPACE {
        return Err(
            "server NamespaceArray index 0 must be the OPC UA core namespace URI".to_owned(),
        );
    }
    let mut namespace_uri: Option<String> = None;
    let mut identifier: Option<(char, String)> = None;
    let mut server_index_seen = false;
    for part in value.split(';') {
        if let Some(index) = part.strip_prefix("ns=") {
            let index: usize = index
                .parse()
                .map_err(|_| format!("invalid namespace index in NodeId {value:?}"))?;
            namespace_uri = Some(namespace_array.get(index).cloned().ok_or_else(|| {
                format!("NodeId namespace index {index} is not in NamespaceArray")
            })?);
        } else if let Some(uri) = part.strip_prefix("nsu=") {
            if uri.is_empty() {
                return Err("empty namespace URI in NodeId".to_owned());
            }
            namespace_uri = Some(uri.to_owned());
        } else if let Some(index) = part.strip_prefix("svr=") {
            let _: u32 = index
                .parse()
                .map_err(|_| "invalid server index in NodeId".to_owned())?;
            server_index_seen = true;
        } else if let Some((kind, id)) = part.split_once('=') {
            if kind.len() != 1 || id.is_empty() {
                return Err(format!("invalid identifier component in NodeId {value:?}"));
            }
            let kind = kind.chars().next().expect("one-character kind");
            if !matches!(kind, 'i' | 's' | 'g' | 'b') {
                return Err(format!("unsupported NodeId identifier type {kind:?}"));
            }
            if identifier.replace((kind, id.to_owned())).is_some() {
                return Err(format!(
                    "multiple identifier components in NodeId {value:?}"
                ));
            }
        }
    }
    if server_index_seen {
        return Err(
            "server-indexed ExpandedNodeId requires an explicit server-URI resolver".to_owned(),
        );
    }
    let identifier = identifier.ok_or_else(|| format!("NodeId {value:?} has no identifier"))?;
    let namespace_uri = namespace_uri.unwrap_or_else(|| OPC_UA_CORE_NAMESPACE.to_owned());
    Ok(CanonicalNodeId {
        namespace_uri,
        identifier_type: identifier.0,
        identifier: identifier.1,
    })
}

pub fn evaluate_server_namespace_metadata(
    expected: &Value,
    observed: &Value,
) -> Result<NamespaceMetadataDisposition, String> {
    let expected_uri = manifest_string(expected, "/namespace_metadata/namespace_uri")?;
    let expected_version = manifest_string(expected, "/namespace_metadata/namespace_version")?;
    let expected_date =
        manifest_string(expected, "/namespace_metadata/namespace_publication_date")?;
    let mut mismatches = Vec::new();
    for (field, expected_value) in [
        ("namespace_uri", expected_uri),
        ("namespace_version", expected_version),
        ("namespace_publication_date", expected_date),
    ] {
        if observed.get(field).and_then(Value::as_str) != Some(expected_value) {
            mismatches.push(format!("{field} mismatch"));
        }
    }
    let subset = observed
        .get("is_namespace_subset")
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            "observed NamespaceMetadata missing boolean is_namespace_subset".to_owned()
        })?;

    if observed.get("read_only_probe").and_then(Value::as_bool) != Some(true) {
        mismatches.push("snapshot is not declared read-only".to_owned());
    }
    if observed
        .get("mutation_service_calls")
        .and_then(Value::as_u64)
        != Some(0)
    {
        mismatches.push("snapshot reports a non-zero mutation service-call count".to_owned());
    }
    if !matches!(
        observed.get("evidence_class").and_then(Value::as_str),
        Some("synthetic" | "recorded")
    ) {
        mismatches.push("snapshot evidence_class must be synthetic or recorded".to_owned());
    }
    let namespace_array = observed
        .get("namespace_array")
        .and_then(Value::as_array)
        .ok_or_else(|| "snapshot missing namespace_array".to_owned())?;
    let namespace_values: Vec<&str> = namespace_array.iter().filter_map(Value::as_str).collect();
    if namespace_values.len() != namespace_array.len()
        || namespace_values.first().copied() != Some(OPC_UA_CORE_NAMESPACE)
    {
        mismatches
            .push("NamespaceArray must contain strings and index 0 must be OPC UA core".to_owned());
    }
    let unique_namespace_values: BTreeSet<&str> = namespace_values.iter().copied().collect();
    if unique_namespace_values.len() != namespace_values.len() {
        mismatches.push("NamespaceArray contains duplicate namespace URIs".to_owned());
    }
    if namespace_values
        .iter()
        .filter(|uri| **uri == expected_uri)
        .count()
        != 1
    {
        mismatches
            .push("NamespaceArray must contain the expected namespace URI exactly once".to_owned());
    }

    let service_calls = observed
        .get("service_calls")
        .and_then(Value::as_array)
        .ok_or_else(|| "snapshot missing service_calls".to_owned())?;
    let mut expected_calls = BTreeSet::from(["Read NamespaceArray", "Read NamespaceMetadata"]);
    for call in service_calls {
        let call = call
            .as_str()
            .ok_or_else(|| "service_calls entries must be strings".to_owned())?;
        if !expected_calls.remove(call) {
            mismatches.push(format!(
                "unexpected or duplicate OPC UA service call {call:?}"
            ));
        }
    }
    if !expected_calls.is_empty() {
        mismatches.push("snapshot did not include both read-only metadata services".to_owned());
    }

    if !mismatches.is_empty() {
        return Ok(NamespaceMetadataDisposition {
            metadata_matches: false,
            subset,
            disposition: "namespace_metadata_mismatch",
            mismatches,
        });
    }
    Ok(NamespaceMetadataDisposition {
        metadata_matches: true,
        subset,
        disposition: if subset {
            "matching_declared_subset"
        } else {
            "matching_full_namespace_claim_node_coverage_unverified"
        },
        mismatches,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const MACHINERY_MANIFEST: &str =
        include_str!("../../nodesets/opcua-machinery-1.04.2.nodeset.json");
    const ROBOTICS_MANIFEST: &str = include_str!("../../nodesets/opcua-robotics-1.02.nodeset.json");

    const SYNTHETIC_XML: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<UANodeSet xmlns="http://opcfoundation.org/UA/2011/03/UANodeSet.xsd">
  <NamespaceUris>
    <Uri>http://opcfoundation.org/UA/Machinery/</Uri>
    <Uri>http://opcfoundation.org/UA/DI/</Uri>
    <Uri>http://opcfoundation.org/UA/IA/</Uri>
  </NamespaceUris>
  <Models>
    <Model ModelUri="http://opcfoundation.org/UA/Machinery/" Version="1.04.2" ModelVersion="1.4.2" PublicationDate="2026-11-02T00:00:00Z">
      <RequiredModel ModelUri="http://opcfoundation.org/UA/" Version="1.05.07" PublicationDate="2026-07-30T00:00:00Z" />
      <RequiredModel ModelUri="http://opcfoundation.org/UA/DI/" Version="1.04.0" PublicationDate="2022-11-03T00:00:00Z" />
      <RequiredModel ModelUri="http://opcfoundation.org/UA/IA/" Version="1.01.4" PublicationDate="2025-05-23T00:00:00Z" />
    </Model>
  </Models>
  <UAObject NodeId="ns=1;i=1003" BrowseName="1:MachineryEquipment" />
</UANodeSet>"#;

    #[test]
    fn git_blob_sha1_matches_git_object_known_answer() {
        assert_eq!(
            git_blob_sha1_hex(b"abc"),
            "f2ba8f84ab5c1bce84a7b441cb1959cfc7093b7f"
        );
    }

    #[test]
    fn sha256_matches_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    fn fixture_manifest() -> Value {
        let required_models = json!([
            {"model_uri":OPC_UA_CORE_NAMESPACE,"version":"1.05.07","publication_date":"2026-07-30"},
            {"model_uri":"http://opcfoundation.org/UA/DI/","version":"1.04.0","publication_date":"2022-11-03"},
            {"model_uri":"http://opcfoundation.org/UA/IA/","version":"1.01.4","publication_date":"2025-05-23"}
        ]);
        json!({
            "namespace_metadata":{
                "namespace_uri":"http://opcfoundation.org/UA/Machinery/",
                "namespace_version":"1.04.2",
                "namespace_publication_date":"2026-11-01"
            },
            "nodeset_artifact":{
                "content_sha256":sha256_hex(SYNTHETIC_XML.as_bytes()),
                "git_blob_sha1":git_blob_sha1_hex(SYNTHETIC_XML.as_bytes()),
                "byte_length":SYNTHETIC_XML.len(),
                "namespace_uris":[
                    "http://opcfoundation.org/UA/Machinery/",
                    "http://opcfoundation.org/UA/DI/",
                    "http://opcfoundation.org/UA/IA/"
                ],
                "node_set_model":{
                    "model_uri":"http://opcfoundation.org/UA/Machinery/",
                    "version":"1.04.2","model_version":"1.4.2",
                    "publication_date":"2026-11-02"
                },
                "required_models":required_models
            }
        })
    }

    #[test]
    fn exact_artifact_digest_and_separate_namespace_and_model_dates_are_admitted() {
        let report =
            verify_nodeset_artifact(SYNTHETIC_XML.as_bytes(), &fixture_manifest()).unwrap();
        assert!(report.admitted);
        assert_eq!(report.model_publication_date, "2026-11-02");
        // Normative NamespaceMetadata is 2026-11-01; NodeSet Model PublicationDate is 2026-11-02.
    }

    #[test]
    fn git_blob_hash_mismatch_is_rejected_even_when_raw_sha256_matches() {
        let mut manifest = fixture_manifest();
        manifest["nodeset_artifact"]["git_blob_sha1"] =
            json!("0000000000000000000000000000000000000000");
        let errors = verify_nodeset_artifact(SYNTHETIC_XML.as_bytes(), &manifest).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("Git blob SHA-1 mismatch"))
        );
    }

    #[test]
    fn digest_mismatch_fails_before_model_admission() {
        let mut manifest = fixture_manifest();
        manifest["nodeset_artifact"]["content_sha256"] = json!("00".repeat(32));
        assert!(
            verify_nodeset_artifact(SYNTHETIC_XML.as_bytes(), &manifest)
                .unwrap_err()
                .iter()
                .any(|error| error.contains("SHA-256 mismatch"))
        );
    }

    #[test]
    fn required_model_version_or_date_mismatch_is_rejected() {
        let mut xml = SYNTHETIC_XML.replace("Version=\"1.05.07\"", "Version=\"1.05.03\"");
        xml = xml.replace("2026-07-30T00:00:00Z", "2026-07-29T00:00:00Z");
        let parsed = parse_nodeset_xml(xml.as_bytes()).unwrap();
        assert_eq!(parsed.models[0].required_models[0].version, "1.05.03");
        let mut manifest = fixture_manifest();
        manifest["nodeset_artifact"]["content_sha256"] = json!(sha256_hex(xml.as_bytes()));
        manifest["nodeset_artifact"]["byte_length"] = json!(xml.len());
        let errors = verify_nodeset_artifact(xml.as_bytes(), &manifest).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("RequiredModels mismatch"))
        );
    }

    #[test]
    fn standard_namespace_publication_date_is_not_collapsed_into_nodeset_model_date() {
        let mut manifest = fixture_manifest();
        manifest["namespace_metadata"]["namespace_publication_date"] = json!("2026-11-01");
        manifest["nodeset_artifact"]["node_set_model"]["publication_date"] = json!("2026-11-02");
        assert!(verify_nodeset_artifact(SYNTHETIC_XML.as_bytes(), &manifest).is_ok());
    }

    #[test]
    fn parses_minimal_nodeset_and_checks_namespace_indices() {
        let parsed = parse_nodeset_xml(SYNTHETIC_XML.as_bytes()).unwrap();
        assert_eq!(parsed.namespace_uris.len(), 3);
        assert_eq!(parsed.models.len(), 1);
        assert_eq!(parsed.models[0].model_version.as_deref(), Some("1.4.2"));
        assert_eq!(parsed.checked_node_id_references, 2);
    }

    #[test]
    fn rejects_doctype_and_duplicate_namespace_uris() {
        let doctype = SYNTHETIC_XML.replace(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>",
            "<?xml version=\"1.0\" encoding=\"utf-8\"?><!DOCTYPE UANodeSet [<!ENTITY x \"boom\">]>",
        );
        assert!(
            parse_nodeset_xml(doctype.as_bytes())
                .unwrap_err()
                .contains("DOCTYPE")
        );
        let duplicate = SYNTHETIC_XML.replace(
            "<Uri>http://opcfoundation.org/UA/IA/</Uri>",
            "<Uri>http://opcfoundation.org/UA/DI/</Uri>",
        );
        assert!(
            parse_nodeset_xml(duplicate.as_bytes())
                .unwrap_err()
                .contains("duplicate")
        );
    }

    #[test]
    fn nodeset_namespace_index_zero_resolves_to_opc_ua_core_namespace() {
        let xml = SYNTHETIC_XML.replace("NodeId=\"ns=1;i=1003\"", "NodeId=\"ns=0;i=1003\"");
        assert!(parse_nodeset_xml(xml.as_bytes()).is_ok());
    }

    #[test]
    fn aliases_are_resolved_and_their_namespace_indexes_are_validated() {
        let xml = SYNTHETIC_XML.replace(
            "<Models>",
            "<Aliases><Alias Alias=\"HasComponent\">ns=2;i=47</Alias><Alias Alias=\"UInt16\">i=5</Alias></Aliases><Models>",
        ).replace(
            "<UAObject NodeId=\"ns=1;i=1003\" BrowseName=\"1:MachineryEquipment\" />",
            "<UAObject NodeId=\"ns=1;i=1003\" BrowseName=\"1:MachineryEquipment\"><References><Reference ReferenceType=\"HasComponent\">ns=2;i=85</Reference></References></UAObject>",
        );
        assert!(parse_nodeset_xml(xml.as_bytes()).is_ok());

        let bad_alias = xml.replace("ns=2;i=47", "ns=4;i=47");
        assert!(
            parse_nodeset_xml(bad_alias.as_bytes())
                .unwrap_err()
                .contains("Alias value")
        );

        let unknown_alias = xml.replace(
            "ReferenceType=\"HasComponent\"",
            "ReferenceType=\"MissingAlias\"",
        );
        assert!(
            parse_nodeset_xml(unknown_alias.as_bytes())
                .unwrap_err()
                .contains("unknown UANodeSet Alias")
        );
    }

    #[test]
    fn duplicate_alias_names_are_rejected() {
        let xml = SYNTHETIC_XML.replace(
            "<Models>",
            "<Aliases><Alias Alias=\"Same\">i=1</Alias><Alias Alias=\"Same\">i=2</Alias></Aliases><Models>",
        );
        assert!(
            parse_nodeset_xml(xml.as_bytes())
                .unwrap_err()
                .contains("duplicate UANodeSet Alias")
        );
    }

    #[test]
    fn reference_type_and_target_namespace_indexes_are_checked() {
        let xml = SYNTHETIC_XML.replace(
            "<UAObject NodeId=\"ns=1;i=1003\" BrowseName=\"1:MachineryEquipment\" />",
            "<UAObject NodeId=\"ns=1;i=1003\" BrowseName=\"1:MachineryEquipment\"><References><Reference ReferenceType=\"ns=0;i=40\">ns=3;i=85</Reference></References></UAObject>",
        );
        assert!(parse_nodeset_xml(xml.as_bytes()).is_ok());

        let invalid_target = xml.replace("ns=3;i=85", "ns=4;i=85");
        assert!(
            parse_nodeset_xml(invalid_target.as_bytes())
                .unwrap_err()
                .contains("Reference target")
        );

        let invalid_type =
            xml.replace("ReferenceType=\"ns=0;i=40\"", "ReferenceType=\"ns=4;i=40\"");
        assert!(
            parse_nodeset_xml(invalid_type.as_bytes())
                .unwrap_err()
                .contains("ReferenceType")
        );
    }

    #[test]
    fn rejects_out_of_range_nodeset_namespace_index() {
        let invalid = SYNTHETIC_XML.replace("ns=1;i=1003", "ns=4;i=1003");
        assert!(
            parse_nodeset_xml(invalid.as_bytes())
                .unwrap_err()
                .contains("outside")
        );
    }

    #[test]
    fn namespace_index_reordering_preserves_canonical_node_identity() {
        let first = vec![
            OPC_UA_CORE_NAMESPACE.to_owned(),
            "http://opcfoundation.org/UA/Machinery/".to_owned(),
            "http://opcfoundation.org/UA/DI/".to_owned(),
        ];
        let second = vec![
            OPC_UA_CORE_NAMESPACE.to_owned(),
            "http://opcfoundation.org/UA/DI/".to_owned(),
            "http://opcfoundation.org/UA/Machinery/".to_owned(),
        ];
        let a = canonicalize_node_id("ns=1;i=1003", &first).unwrap();
        let b = canonicalize_node_id("ns=2;i=1003", &second).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn rejects_unknown_namespace_index_and_expanded_server_index() {
        let namespace_array = vec![OPC_UA_CORE_NAMESPACE.to_owned()];
        assert!(canonicalize_node_id("ns=3;i=1003", &namespace_array).is_err());
        assert!(canonicalize_node_id("svr=2;ns=1;i=1003", &namespace_array).is_err());
    }

    #[test]
    fn machinery_and_robotics_have_explicit_core_model_dependency_skew() {
        let machinery: Value = serde_json::from_str(MACHINERY_MANIFEST).unwrap();
        let robotics: Value = serde_json::from_str(ROBOTICS_MANIFEST).unwrap();
        let report = analyze_node_set_bundle(&[machinery, robotics]);
        assert!(!report.ready_for_composition);
        assert_eq!(report.status, "requires_explicit_dependency_resolution");
        let core = report
            .dependency_skews
            .iter()
            .find(|entry| entry.model_uri == OPC_UA_CORE_NAMESPACE)
            .unwrap();
        assert_eq!(core.requirements.len(), 2);
    }

    #[test]
    fn server_namespace_subset_is_never_promoted_to_complete_model_support() {
        let expected: Value = json!({"namespace_metadata":{
            "namespace_uri":"http://opcfoundation.org/UA/Machinery/",
            "namespace_version":"1.04.2",
            "namespace_publication_date":"2026-11-01"
        }});
        let observed = json!({
            "evidence_class":"synthetic",
            "endpoint_id":"urn:luminous:test:opcua:machinery-server-001",
            "namespace_uri":"http://opcfoundation.org/UA/Machinery/",
            "namespace_version":"1.04.2",
            "namespace_publication_date":"2026-11-01",
            "is_namespace_subset":true,
            "namespace_array":[
                OPC_UA_CORE_NAMESPACE,
                "http://opcfoundation.org/UA/Machinery/",
                "http://opcfoundation.org/UA/DI/",
                "http://opcfoundation.org/UA/IA/"
            ],
            "captured_at":"2026-11-21T00:00:00Z",
            "read_only_probe":true,
            "service_calls":["Read NamespaceArray","Read NamespaceMetadata"],
            "mutation_service_calls":0,
            "physical_qualification":false
        });
        let result = evaluate_server_namespace_metadata(&expected, &observed).unwrap();
        assert_eq!(result.disposition, "matching_declared_subset");
    }

    #[test]
    fn rejects_namespace_array_substitution_and_any_mutation_probe() {
        let expected: Value = json!({"namespace_metadata":{
            "namespace_uri":"http://opcfoundation.org/UA/Machinery/",
            "namespace_version":"1.04.2",
            "namespace_publication_date":"2026-11-01"
        }});
        let mut observed = json!({
            "evidence_class":"synthetic",
            "namespace_uri":"http://opcfoundation.org/UA/Machinery/",
            "namespace_version":"1.04.2",
            "namespace_publication_date":"2026-11-01",
            "is_namespace_subset":false,
            "namespace_array":[
                OPC_UA_CORE_NAMESPACE,
                "http://opcfoundation.org/UA/Machinery/"
            ],
            "read_only_probe":true,
            "service_calls":["Read NamespaceArray","Read NamespaceMetadata"],
            "mutation_service_calls":0
        });
        observed["namespace_array"][1] = json!("urn:vendor:wrong-namespace");
        assert!(
            !evaluate_server_namespace_metadata(&expected, &observed)
                .unwrap()
                .metadata_matches
        );
        observed["namespace_array"][1] = json!("http://opcfoundation.org/UA/Machinery/");
        observed["mutation_service_calls"] = json!(1);
        assert!(
            !evaluate_server_namespace_metadata(&expected, &observed)
                .unwrap()
                .metadata_matches
        );
    }

    #[test]
    fn full_namespace_metadata_is_not_node_coverage_proof() {
        let expected: Value = json!({"namespace_metadata":{
            "namespace_uri":"http://opcfoundation.org/UA/Machinery/",
            "namespace_version":"1.04.2",
            "namespace_publication_date":"2026-11-01"
        }});
        let observed = json!({
            "evidence_class":"synthetic",
            "endpoint_id":"urn:luminous:test:opcua:machinery-server-full",
            "namespace_uri":"http://opcfoundation.org/UA/Machinery/",
            "namespace_version":"1.04.2",
            "namespace_publication_date":"2026-11-01",
            "is_namespace_subset":false,
            "namespace_array":[
                OPC_UA_CORE_NAMESPACE,
                "http://opcfoundation.org/UA/Machinery/"
            ],
            "captured_at":"2026-11-21T00:00:00Z",
            "read_only_probe":true,
            "service_calls":["Read NamespaceArray","Read NamespaceMetadata"],
            "mutation_service_calls":0,
            "physical_qualification":false
        });
        let result = evaluate_server_namespace_metadata(&expected, &observed).unwrap();
        assert_eq!(
            result.disposition,
            "matching_full_namespace_claim_node_coverage_unverified"
        );
    }
}
