//! Metadata-only ZIP preflight for protective model dataset archives.
//!
//! This parser never extracts or decompresses members. It validates the
//! bounded ZIP structure, path metadata and local/central header agreement.
//! A parseable central directory is not content validation: CRC and expanded
//! member bytes remain unverified.

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::process;

const STORED_HEX: &str =
    include_str!("../../../fixtures/archive-intake/safe-stored.zip.hex");
const DEFLATED_HEX: &str =
    include_str!("../../../fixtures/archive-intake/safe-deflated.zip.hex");
const POLICY_JSON: &str =
    include_str!("../../../fixtures/protective-model-archive-intake-policy-v0.1.json");
const SCHEMA: &str = "protective-model-archive-intake/0.1";
const EOCD_SIG: u32 = 0x0605_4b50;
const CENTRAL_SIG: u32 = 0x0201_4b50;
const LOCAL_SIG: u32 = 0x0403_4b50;

#[derive(Clone, Debug)]
struct Policy {
    policy_id: String,
    max_archive_bytes: usize,
    max_central_directory_bytes: usize,
    max_entries: usize,
    max_filename_bytes: usize,
    max_compressed_member_bytes: u64,
    max_uncompressed_member_bytes: u64,
    max_aggregate_uncompressed_bytes: u64,
    max_declared_compression_ratio: u64,
}

#[derive(Debug, Serialize)]
struct MemberReceipt {
    path: String,
    directory: bool,
    compression_method: u16,
    crc32_field: u32,
    compressed_bytes_declared: u64,
    uncompressed_bytes_declared: u64,
    external_attributes: u32,
    local_header_offset: u64,
    disposition: &'static str,
}

#[derive(Debug, Serialize)]
struct IntakeReport {
    schema_version: &'static str,
    policy_id: String,
    archive_byte_length: usize,
    archive_sha256: String,
    central_directory_byte_length: usize,
    member_count: usize,
    declared_uncompressed_total: u64,
    content_validation: &'static str,
    extraction_performed: bool,
    members: Vec<MemberReceipt>,
    claim_ceiling: &'static str,
}

#[derive(Clone, Debug)]
struct Entry {
    path: String,
    directory: bool,
    flags: u16,
    method: u16,
    crc32: u32,
    compressed: u64,
    uncompressed: u64,
    external_attributes: u32,
    local_offset: u64,
}

fn fail(code: &str, detail: &str) -> String {
    format!("{code}: {detail}")
}

fn u16_at(bytes: &[u8], offset: usize) -> Result<u16, String> {
    let slice = bytes
        .get(offset..offset.saturating_add(2))
        .ok_or_else(|| fail("REJECT_TRUNCATED", "u16 field outside archive bounds"))?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let slice = bytes
        .get(offset..offset.saturating_add(4))
        .ok_or_else(|| fail("REJECT_TRUNCATED", "u32 field outside archive bounds"))?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn as_usize(value: u32, label: &str) -> Result<usize, String> {
    usize::try_from(value).map_err(|_| fail("REJECT_INTEGER_OVERFLOW", label))
}

fn parse_policy() -> Result<Policy, String> {
    let value: Value =
        serde_json::from_str(POLICY_JSON).map_err(|e| fail("REJECT_POLICY_JSON", &e.to_string()))?;
    if value["schema_version"] != "protective-model-archive-intake-policy/0.1"
        || value["policy_class"] != "SYNTHETIC_CONFORMANCE_ONLY_NOT_FOR_PRODUCTION"
        || value["profile"] != "ZIP_SINGLE_DISK_NO_ZIP64_NO_ENCRYPTION_NO_SYMLINKS_V0_1"
        || value["extract_members"] != false
    {
        return Err(fail("REJECT_POLICY_PROFILE", "unsupported or unsafe policy declaration"));
    }
    let number = |key: &str| -> Result<u64, String> {
        value[key]
            .as_u64()
            .ok_or_else(|| fail("REJECT_POLICY_FIELD", key))
    };
    if value["allow_utf8_names"] != false
        || value["allow_symlinks"] != false
        || value["allow_special_file_types"] != false
        || value["allow_zip64"] != false
        || value["allow_multidisk"] != false
        || value["allow_encryption"] != false
        || value["allow_data_descriptors"] != false
    {
        return Err(fail("REJECT_POLICY_PROFILE", "v0.1 must keep extraction and risky features disabled"));
    }
    let methods = value["permitted_compression_methods"]
        .as_array()
        .ok_or_else(|| fail("REJECT_POLICY_FIELD", "permitted_compression_methods"))?;
    if methods.len() != 2
        || !methods.iter().any(|m| m.as_u64() == Some(0))
        || !methods.iter().any(|m| m.as_u64() == Some(8))
    {
        return Err(fail("REJECT_POLICY_METHODS", "v0.1 permits stored and deflate metadata only"));
    }
    Ok(Policy {
        policy_id: value["policy_id"]
            .as_str()
            .ok_or_else(|| fail("REJECT_POLICY_FIELD", "policy_id"))?
            .to_owned(),
        max_archive_bytes: usize::try_from(number("max_archive_bytes")?)
            .map_err(|_| fail("REJECT_POLICY_FIELD", "max_archive_bytes"))?,
        max_central_directory_bytes: usize::try_from(number("max_central_directory_bytes")?)
            .map_err(|_| fail("REJECT_POLICY_FIELD", "max_central_directory_bytes"))?,
        max_entries: usize::try_from(number("max_entries")?)
            .map_err(|_| fail("REJECT_POLICY_FIELD", "max_entries"))?,
        max_filename_bytes: usize::try_from(number("max_filename_bytes")?)
            .map_err(|_| fail("REJECT_POLICY_FIELD", "max_filename_bytes"))?,
        max_compressed_member_bytes: number("max_compressed_member_bytes")?,
        max_uncompressed_member_bytes: number("max_uncompressed_member_bytes")?,
        max_aggregate_uncompressed_bytes: number("max_aggregate_uncompressed_bytes")?,
        max_declared_compression_ratio: number("max_declared_compression_ratio")?,
    })
}

fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
    let compact: String = value.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    if compact.len() % 2 != 0 {
        return Err(fail("REJECT_FIXTURE_HEX", "odd number of hex characters"));
    }
    let mut out = Vec::with_capacity(compact.len() / 2);
    for pair in compact.as_bytes().chunks_exact(2) {
        let pair = std::str::from_utf8(pair)
            .map_err(|e| fail("REJECT_FIXTURE_HEX", &e.to_string()))?;
        out.push(
            u8::from_str_radix(pair, 16)
                .map_err(|e| fail("REJECT_FIXTURE_HEX", &e.to_string()))?,
        );
    }
    Ok(out)
}

fn find_eocd(bytes: &[u8]) -> Result<usize, String> {
    let minimum = 22usize;
    if bytes.len() < minimum {
        return Err(fail("REJECT_TRUNCATED_EOCD", "archive shorter than EOCD"));
    }
    let start = bytes.len().saturating_sub(minimum + 65_535);
    for offset in (start..=bytes.len() - minimum).rev() {
        if u32_at(bytes, offset).ok() == Some(EOCD_SIG) {
            let comment_length = usize::from(u16_at(bytes, offset + 20)?);
            let end = offset
                .checked_add(minimum)
                .and_then(|v| v.checked_add(comment_length))
                .ok_or_else(|| fail("REJECT_INTEGER_OVERFLOW", "EOCD comment end"))?;
            if end == bytes.len() {
                return Ok(offset);
            }
            if end < bytes.len() {
                return Err(fail("REJECT_TRAILING_BYTES", "bytes occur after EOCD comment"));
            }
            return Err(fail("REJECT_TRUNCATED_EOCD", "EOCD comment exceeds archive bounds"));
        }
    }
    Err(fail("REJECT_TRUNCATED_EOCD", "EOCD signature not found"))
}

fn validate_extra_fields(bytes: &[u8], offset: usize, length: usize) -> Result<(), String> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| fail("REJECT_INTEGER_OVERFLOW", "extra field end"))?;
    if end > bytes.len() {
        return Err(fail("REJECT_EXTRA_BOUNDS", "extra fields outside archive bounds"));
    }
    let mut cursor = offset;
    while cursor < end {
        if end - cursor < 4 {
            return Err(fail("REJECT_EXTRA_TRUNCATED", "short extra-field header"));
        }
        let tag = u16_at(bytes, cursor)?;
        let size = usize::from(u16_at(bytes, cursor + 2)?);
        cursor += 4;
        let next = cursor
            .checked_add(size)
            .ok_or_else(|| fail("REJECT_INTEGER_OVERFLOW", "extra field size"))?;
        if next > end {
            return Err(fail("REJECT_EXTRA_TRUNCATED", "extra field payload exceeds declared area"));
        }
        if tag == 0x0001 {
            return Err(fail("REJECT_ZIP64", "ZIP64 extra field is unsupported in v0.1"));
        }
        cursor = next;
    }
    Ok(())
}

fn windows_device_component(component: &str) -> bool {
    let stem = component.split('.').next().unwrap_or("").trim_end_matches([' ', '.']);
    let upper = stem.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            upper.strip_prefix(prefix).is_some_and(|suffix| {
                suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'1'..=b'9')
            })
        })
}

fn validate_member_path(name: &[u8], maximum: usize) -> Result<(String, bool), String> {
    if name.is_empty() || name.len() > maximum {
        return Err(fail("REJECT_FILENAME_LENGTH", "member filename empty or exceeds policy"));
    }
    if name.iter().any(|b| !b.is_ascii() || *b < 0x20 || *b == 0x7f) {
        return Err(fail("REJECT_FILENAME_ENCODING", "v0.1 accepts printable ASCII names only"));
    }
    let raw = std::str::from_utf8(name)
        .map_err(|e| fail("REJECT_FILENAME_ENCODING", &e.to_string()))?;
    if raw.starts_with('/') || raw.starts_with("//") {
        return Err(fail("REJECT_PATH_ABSOLUTE", raw));
    }
    if raw.contains('\\') {
        return Err(fail("REJECT_PATH_BACKSLASH", raw));
    }
    if raw.contains(':') {
        return Err(fail("REJECT_PATH_DRIVE_OR_ADS", raw));
    }
    let directory = raw.ends_with('/');
    let trimmed = if directory { &raw[..raw.len() - 1] } else { raw };
    if trimmed.is_empty() {
        return Err(fail("REJECT_PATH_EMPTY", raw));
    }
    let components: Vec<&str> = trimmed.split('/').collect();
    if components.iter().any(|part| part.is_empty()) {
        return Err(fail("REJECT_PATH_EMPTY_COMPONENT", raw));
    }
    for component in &components {
        if *component == "." || *component == ".." {
            return Err(fail("REJECT_PATH_DOT_COMPONENT", raw));
        }
        if component.ends_with(' ') || component.ends_with('.') {
            return Err(fail("REJECT_PATH_TRAILING_DOT_OR_SPACE", raw));
        }
        if windows_device_component(component) {
            return Err(fail("REJECT_PATH_DEVICE_NAME", raw));
        }
    }
    Ok((trimmed.to_owned(), directory))
}

fn validate_archive(bytes: &[u8], policy: &Policy) -> Result<IntakeReport, String> {
    if bytes.is_empty() || bytes.len() > policy.max_archive_bytes {
        return Err(fail("REJECT_ARCHIVE_SIZE", "empty archive or compressed byte limit exceeded"));
    }
    let eocd = find_eocd(bytes)?;
    let disk = u16_at(bytes, eocd + 4)?;
    let cd_disk = u16_at(bytes, eocd + 6)?;
    let entries_on_disk = u16_at(bytes, eocd + 8)?;
    let total_entries = u16_at(bytes, eocd + 10)?;
    let cd_size_raw = u32_at(bytes, eocd + 12)?;
    let cd_offset_raw = u32_at(bytes, eocd + 16)?;
    if disk != 0 || cd_disk != 0 || entries_on_disk != total_entries {
        return Err(fail("REJECT_MULTIDISK", "split or multi-disk ZIP is unsupported"));
    }
    if total_entries == u16::MAX || cd_size_raw == u32::MAX || cd_offset_raw == u32::MAX {
        return Err(fail("REJECT_ZIP64", "ZIP64 sentinel is unsupported"));
    }
    let count = usize::from(total_entries);
    if count > policy.max_entries {
        return Err(fail("REJECT_ENTRY_COUNT", "member count exceeds policy"));
    }
    let cd_size = as_usize(cd_size_raw, "central directory size")?;
    let cd_offset = as_usize(cd_offset_raw, "central directory offset")?;
    if cd_size > policy.max_central_directory_bytes {
        return Err(fail("REJECT_CENTRAL_DIRECTORY_SIZE", "central directory exceeds policy"));
    }
    let cd_end = cd_offset
        .checked_add(cd_size)
        .ok_or_else(|| fail("REJECT_INTEGER_OVERFLOW", "central directory end"))?;
    if cd_end != eocd {
        return Err(fail("REJECT_CENTRAL_DIRECTORY_BOUNDS", "central directory must end exactly at EOCD"));
    }
    if cd_end > bytes.len() {
        return Err(fail("REJECT_CENTRAL_DIRECTORY_BOUNDS", "central directory exceeds archive bounds"));
    }

    let mut cursor = cd_offset;
    let mut entries = Vec::with_capacity(count);
    let mut seen_exact = BTreeSet::new();
    let mut seen_folded = BTreeSet::new();
    let mut declared_total = 0u64;

    for _ in 0..count {
        if u32_at(bytes, cursor)? != CENTRAL_SIG {
            return Err(fail("REJECT_CENTRAL_DIRECTORY_SIGNATURE", "invalid central file header"));
        }
        let made_by_os = bytes
            .get(cursor + 5)
            .copied()
            .ok_or_else(|| fail("REJECT_TRUNCATED", "central header platform byte"))?;
        if made_by_os != 0 && made_by_os != 3 {
            return Err(fail("REJECT_PLATFORM", "only DOS and Unix creator metadata are supported"));
        }
        let flags = u16_at(bytes, cursor + 8)?;
        let method = u16_at(bytes, cursor + 10)?;
        let crc32 = u32_at(bytes, cursor + 16)?;
        let compressed_raw = u32_at(bytes, cursor + 20)?;
        let uncompressed_raw = u32_at(bytes, cursor + 24)?;
        let name_length = usize::from(u16_at(bytes, cursor + 28)?);
        let extra_length = usize::from(u16_at(bytes, cursor + 30)?);
        let comment_length = usize::from(u16_at(bytes, cursor + 32)?);
        let start_disk = u16_at(bytes, cursor + 34)?;
        let external_attributes = u32_at(bytes, cursor + 38)?;
        let local_offset_raw = u32_at(bytes, cursor + 42)?;
        if flags & 0x0001 != 0 || flags & 0x0040 != 0 || flags & 0x2000 != 0 {
            return Err(fail("REJECT_ENCRYPTED", "encrypted ZIP entry flags are unsupported"));
        }
        if flags & 0x0008 != 0 {
            return Err(fail("REJECT_DATA_DESCRIPTOR", "data-descriptor entries are unsupported"));
        }
        if flags & !0x0800 != 0 {
            return Err(fail("REJECT_GENERAL_PURPOSE_FLAGS", "unsupported general-purpose ZIP flags"));
        }
        if ![0, 8].contains(&method) {
            return Err(fail("REJECT_COMPRESSION_METHOD", "compression method not permitted by v0.1"));
        }
        if start_disk != 0 {
            return Err(fail("REJECT_MULTIDISK", "member starts on a different disk"));
        }
        if compressed_raw == u32::MAX
            || uncompressed_raw == u32::MAX
            || local_offset_raw == u32::MAX
        {
            return Err(fail("REJECT_ZIP64", "ZIP64 size or offset sentinel"));
        }
        let compressed = u64::from(compressed_raw);
        let uncompressed = u64::from(uncompressed_raw);
        if compressed > policy.max_compressed_member_bytes {
            return Err(fail("REJECT_MEMBER_COMPRESSED_SIZE", "compressed member exceeds policy"));
        }
        if uncompressed > policy.max_uncompressed_member_bytes {
            return Err(fail("REJECT_MEMBER_UNCOMPRESSED_SIZE", "declared uncompressed member exceeds policy"));
        }
        declared_total = declared_total
            .checked_add(uncompressed)
            .ok_or_else(|| fail("REJECT_INTEGER_OVERFLOW", "aggregate uncompressed bytes"))?;
        if declared_total > policy.max_aggregate_uncompressed_bytes {
            return Err(fail("REJECT_AGGREGATE_UNCOMPRESSED_SIZE", "aggregate declared uncompressed bytes exceed policy"));
        }
        if uncompressed > 0 {
            if compressed == 0 {
                return Err(fail("REJECT_COMPRESSION_RATIO", "non-empty member declares zero compressed bytes"));
            }
            let ratio_limit = compressed
                .checked_mul(policy.max_declared_compression_ratio)
                .ok_or_else(|| fail("REJECT_INTEGER_OVERFLOW", "compression ratio limit"))?;
            if uncompressed > ratio_limit {
                return Err(fail("REJECT_COMPRESSION_RATIO", "declared compression ratio exceeds policy"));
            }
        }
        if method == 0 && compressed != uncompressed {
            return Err(fail("REJECT_STORED_SIZE_MISMATCH", "stored member compressed and uncompressed sizes differ"));
        }
        let name_start = cursor
            .checked_add(46)
            .ok_or_else(|| fail("REJECT_INTEGER_OVERFLOW", "central name offset"))?;
        let name_end = name_start
            .checked_add(name_length)
            .ok_or_else(|| fail("REJECT_INTEGER_OVERFLOW", "central name end"))?;
        let extra_end = name_end
            .checked_add(extra_length)
            .ok_or_else(|| fail("REJECT_INTEGER_OVERFLOW", "central extra end"))?;
        let record_end = extra_end
            .checked_add(comment_length)
            .ok_or_else(|| fail("REJECT_INTEGER_OVERFLOW", "central record end"))?;
        if record_end > cd_end {
            return Err(fail("REJECT_CENTRAL_DIRECTORY_BOUNDS", "central record exceeds declared directory"));
        }
        let name = &bytes[name_start..name_end];
        let (path, directory_name) = validate_member_path(name, policy.max_filename_bytes)?;
        validate_extra_fields(bytes, name_end, extra_length)?;

        let unix_mode = if made_by_os == 3 {
            external_attributes >> 16
        } else {
            0
        };
        let file_type = unix_mode & 0xf000;
        if file_type == 0xa000 {
            return Err(fail("REJECT_SYMLINK", "symbolic-link member is forbidden"));
        }
        if file_type != 0 && file_type != 0x8000 && file_type != 0x4000 {
            return Err(fail("REJECT_SPECIAL_FILE", "non-regular special file entry"));
        }
        let directory_attr = (external_attributes & 0x10) != 0 || file_type == 0x4000;
        if directory_name != directory_attr {
            return Err(fail("REJECT_DIRECTORY_TYPE_MISMATCH", "trailing slash conflicts with file attributes"));
        }
        if !seen_exact.insert(path.clone()) {
            return Err(fail("REJECT_DUPLICATE_PATH", "duplicate member path"));
        }
        let folded = path.to_ascii_lowercase();
        if !seen_folded.insert(folded) {
            return Err(fail("REJECT_CASE_COLLISION", "case-insensitive path collision"));
        }

        let local_offset = as_usize(local_offset_raw, "local header offset")?;
        if u32_at(bytes, local_offset)? != LOCAL_SIG {
            return Err(fail("REJECT_LOCAL_HEADER", "missing or invalid local file header"));
        }
        let local_flags = u16_at(bytes, local_offset + 6)?;
        let local_method = u16_at(bytes, local_offset + 8)?;
        let local_crc = u32_at(bytes, local_offset + 14)?;
        let local_compressed = u32_at(bytes, local_offset + 18)?;
        let local_uncompressed = u32_at(bytes, local_offset + 22)?;
        let local_name_length = usize::from(u16_at(bytes, local_offset + 26)?);
        let local_extra_length = usize::from(u16_at(bytes, local_offset + 28)?);
        let local_name_start = local_offset
            .checked_add(30)
            .ok_or_else(|| fail("REJECT_INTEGER_OVERFLOW", "local name offset"))?;
        let local_name_end = local_name_start
            .checked_add(local_name_length)
            .ok_or_else(|| fail("REJECT_INTEGER_OVERFLOW", "local name end"))?;
        let local_extra_end = local_name_end
            .checked_add(local_extra_length)
            .ok_or_else(|| fail("REJECT_INTEGER_OVERFLOW", "local extra end"))?;
        if local_extra_end > cd_offset || local_name_end > cd_offset {
            return Err(fail("REJECT_LOCAL_HEADER_BOUNDS", "local header overlaps central directory"));
        }
        validate_extra_fields(bytes, local_name_end, local_extra_length)?;
        if local_flags != flags
            || local_method != method
            || local_crc != crc32
            || u64::from(local_compressed) != compressed
            || u64::from(local_uncompressed) != uncompressed
            || &bytes[local_name_start..local_name_end] != name
        {
            return Err(fail("REJECT_LOCAL_CENTRAL_MISMATCH", "local and central header fields differ"));
        }
        let data_end = local_extra_end
            .checked_add(as_usize(compressed_raw, "compressed size")?)
            .ok_or_else(|| fail("REJECT_INTEGER_OVERFLOW", "member data end"))?;
        if data_end > cd_offset {
            return Err(fail("REJECT_MEMBER_DATA_BOUNDS", "compressed data overlaps central directory"));
        }

        entries.push(Entry {
            path,
            directory: directory_name,
            flags,
            method,
            crc32,
            compressed,
            uncompressed,
            external_attributes,
            local_offset: u64::from(local_offset_raw),
        });
        cursor = record_end;
    }
    if cursor != cd_end {
        return Err(fail("REJECT_CENTRAL_DIRECTORY_SIZE", "entry records do not consume declared directory exactly"));
    }

    let mut paths: BTreeMap<String, bool> = BTreeMap::new();
    for entry in &entries {
        paths.insert(entry.path.clone(), entry.directory);
    }
    for entry in &entries {
        let components: Vec<&str> = entry.path.split('/').collect();
        for index in 1..components.len() {
            let parent = components[..index].join("/");
            if paths.get(&parent).is_some_and(|is_dir| !is_dir) {
                return Err(fail("REJECT_FILE_DIRECTORY_CONFLICT", "a file is an ancestor of another member"));
            }
        }
        if !entry.directory
            && paths
                .keys()
                .any(|candidate| candidate.starts_with(&(entry.path.clone() + "/")))
        {
            return Err(fail("REJECT_FILE_DIRECTORY_CONFLICT", "a file path is also an ancestor directory"));
        }
    }

    let mut ranges = Vec::with_capacity(entries.len());
    for entry in &entries {
        let local_offset = as_usize(entry.local_offset as u32, "local header offset")?;
        let name_length = usize::from(u16_at(bytes, local_offset + 26)?);
        let extra_length = usize::from(u16_at(bytes, local_offset + 28)?);
        let start = local_offset;
        let end = local_offset + 30 + name_length + extra_length
            + usize::try_from(entry.compressed)
                .map_err(|_| fail("REJECT_INTEGER_OVERFLOW", "member compressed length"))?;
        ranges.push((start, end));
    }
    ranges.sort_unstable();
    for pair in ranges.windows(2) {
        if pair[0].1 > pair[1].0 {
            return Err(fail("REJECT_LOCAL_RECORD_OVERLAP", "local member records overlap"));
        }
    }

    let members = entries
        .iter()
        .map(|entry| MemberReceipt {
            path: entry.path.clone(),
            directory: entry.directory,
            compression_method: entry.method,
            crc32_field: entry.crc32,
            compressed_bytes_declared: entry.compressed,
            uncompressed_bytes_declared: entry.uncompressed,
            external_attributes: entry.external_attributes,
            local_header_offset: entry.local_offset,
            disposition: "ACCEPT_METADATA_ONLY",
        })
        .collect();
    Ok(IntakeReport {
        schema_version: SCHEMA,
        policy_id: policy.policy_id.clone(),
        archive_byte_length: bytes.len(),
        archive_sha256: format!("{:x}", Sha256::digest(bytes)),
        central_directory_byte_length: cd_size,
        member_count: entries.len(),
        declared_uncompressed_total: declared_total,
        content_validation: "METADATA_ONLY_CONTENT_NOT_VERIFIED",
        extraction_performed: false,
        members,
        claim_ceiling: "ZIP_METADATA_AND_ORIGINAL_ARCHIVE_BYTE_IDENTITY_ONLY; NO_DECOMPRESSION_CRC_OR_MODEL_VALIDATION",
    })
}

fn run() -> Result<IntakeReport, String> {
    let policy = parse_policy()?;
    let archive = decode_hex(STORED_HEX)?;
    validate_archive(&archive, &policy)
}

fn main() {
    match run() {
        Ok(report) => println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("report serialization")
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

    fn policy() -> Policy {
        parse_policy().expect("frozen synthetic policy")
    }

    fn stored() -> Vec<u8> {
        decode_hex(STORED_HEX).expect("stored ZIP fixture")
    }

    fn deflated() -> Vec<u8> {
        decode_hex(DEFLATED_HEX).expect("deflated ZIP fixture")
    }

    fn find_signature(bytes: &[u8], signature: u32) -> usize {
        bytes
            .windows(4)
            .position(|window| u32::from_le_bytes([window[0], window[1], window[2], window[3]]) == signature)
            .expect("signature in fixture")
    }

    #[test]
    fn stored_archive_is_metadata_accepted_without_extraction() {
        let report = validate_archive(&stored(), &policy()).expect("valid stored archive");
        assert_eq!(report.member_count, 1);
        assert_eq!(report.members[0].path, "safe/readme.txt");
        assert_eq!(report.members[0].compression_method, 0);
        assert!(!report.extraction_performed);
        assert_eq!(report.content_validation, "METADATA_ONLY_CONTENT_NOT_VERIFIED");
    }

    #[test]
    fn deflated_archive_is_metadata_accepted_without_decompression() {
        let report = validate_archive(&deflated(), &policy()).expect("valid deflate archive");
        assert_eq!(report.member_count, 1);
        assert_eq!(report.members[0].compression_method, 8);
        assert_eq!(report.declared_uncompressed_total, 23);
        assert!(!report.extraction_performed);
    }

    #[test]
    fn parent_traversal_member_is_rejected() {
        assert!(validate_member_path(b"../e/readme.txt", 128)
            .expect_err("parent component")
            .starts_with("REJECT_PATH_DOT_COMPONENT"));
    }

    #[test]
    fn absolute_path_member_is_rejected() {
        assert!(validate_member_path(b"/safe/readme.txt", 128)
            .expect_err("absolute path")
            .starts_with("REJECT_PATH_ABSOLUTE"));
    }

    #[test]
    fn backslash_drive_and_reserved_device_paths_are_rejected() {
        assert!(validate_member_path(b"safe\\readme.txt", 128).is_err());
        assert!(validate_member_path(b"C:/readme.txt", 128).is_err());
        assert!(validate_member_path(b"safe/CON.txt", 128).is_err());
    }

    #[test]
    fn case_fold_collision_is_rejected() {
        let mut seen = BTreeSet::new();
        let first = validate_member_path(b"safe/A.txt", 128).expect("first path");
        let second = validate_member_path(b"safe/a.txt", 128).expect("second path");
        assert!(seen.insert(first.0.to_ascii_lowercase()));
        assert!(!seen.insert(second.0.to_ascii_lowercase()));
    }

    #[test]
    fn encrypted_entry_is_rejected() {
        let mut bytes = stored();
        let local = find_signature(&bytes, LOCAL_SIG);
        bytes[local + 6] |= 0x01;
        let central = find_signature(&bytes, CENTRAL_SIG);
        bytes[central + 8] |= 0x01;
        assert!(validate_archive(&bytes, &policy())
            .expect_err("encryption flag")
            .starts_with("REJECT_ENCRYPTED"));
    }

    #[test]
    fn trailing_bytes_after_eocd_are_rejected() {
        let mut bytes = stored();
        bytes.push(0);
        assert!(validate_archive(&bytes, &policy())
            .expect_err("trailing bytes")
            .starts_with("REJECT_TRAILING_BYTES"));
    }

    #[test]
    fn policy_archive_byte_limit_is_enforced() {
        let mut policy = policy();
        policy.max_archive_bytes = 10;
        assert!(validate_archive(&stored(), &policy)
            .expect_err("archive byte policy")
            .starts_with("REJECT_ARCHIVE_SIZE"));
    }

    #[test]
    fn truncated_eocd_is_rejected() {
        let mut bytes = stored();
        bytes.truncate(bytes.len() - 4);
        assert!(validate_archive(&bytes, &policy())
            .expect_err("truncated archive")
            .starts_with("REJECT_TRUNCATED_EOCD"));
    }
}
