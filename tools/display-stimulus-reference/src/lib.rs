#![deny(unsafe_code)]
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic PPM stimulus generation. A generated file proves only its own
//! bytes and code-level raster, not what a host or physical display emits.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;

pub const MANIFEST_ID: &str = "ODRH-STIM-001";
pub const SCHEMA_VERSION: &str = "0.1.0";
pub const CORPUS_ID: &str = "VIS-DISP-CORPUS-001";
pub const CORPUS_BLOB_SHA1: &str = "040cc324cc4116cfeaac249ea62300a44380024b";
const CLAIM_CEILING: &str = "The generated file bytes and code-level raster are specified. Signal negotiation, host/compositor/video-path preservation, physical panel output, photometry, colorimetry, HDR transport, and human perception are not evaluated.";
const SCHEMA_DRAFT: &str = "https://json-schema.org/draft/2020-12/schema";
const MAX_DIMENSION: u64 = 4096;
const MAX_RASTER_BYTES: u64 = 64 * 1024 * 1024;
const REPORT_SCHEMA_TEXT: &str =
    include_str!("../../../docs/visual-technology/display-stimulus-report.schema.json");

const SUPPORTED_CASES: [(&str, &str, &str); 4] = [
    (
        "VIS-DISP-GREY-RAMP-012",
        "neutral_ramp",
        "Monotonic encoded RGB code ramp only; not a physical luminance ramp or banding assessment.",
    ),
    (
        "VIS-DISP-NEARBLACK-013",
        "near_black_steps",
        "Explicit 10-bit code values only; labels do not assert nits or measured black-crush behavior.",
    ),
    (
        "VIS-DISP-LOCAL-HIGHLIGHT-014",
        "center_patch",
        "Pixel geometry and code values only; no claim about panel local dimming, peak luminance, halo, or actual average picture level.",
    ),
    (
        "VIS-DISP-COLOR-GRADIENT-028",
        "rgb_gradient",
        "Encoded RGB codes only; no ICC profile, colorimetric accuracy, display gamut, or perceptual banding is implied.",
    ),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StimulusError(pub String);

impl fmt::Display for StimulusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for StimulusError {}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedStimulus {
    pub bytes: Vec<u8>,
    pub report: Value,
}

#[derive(Debug, Clone, Copy)]
struct RasterSpec {
    width: u32,
    height: u32,
    maxval: u16,
    bit_depth: u8,
}

pub fn validate_manifest_str(manifest_text: &str, schema_text: &str) -> Result<(), StimulusError> {
    let schema: Value = serde_json::from_str(schema_text)
        .map_err(|error| StimulusError(format!("schema JSON parse error: {error}")))?;
    let manifest: Value = serde_json::from_str(manifest_text)
        .map_err(|error| StimulusError(format!("manifest JSON parse error: {error}")))?;

    if schema.get("$schema").and_then(Value::as_str) != Some(SCHEMA_DRAFT) {
        return Err(StimulusError(format!(
            "schema must declare JSON Schema Draft 2020-12 ({SCHEMA_DRAFT})"
        )));
    }

    let validator = jsonschema::draft202012::new(&schema)
        .map_err(|error| StimulusError(format!("schema compilation failed: {error}")))?;
    let violations: Vec<String> = validator
        .iter_errors(&manifest)
        .map(|error| format!("JSON Schema violation: {error}"))
        .collect();
    if !violations.is_empty() {
        return Err(StimulusError(violations.join("\n")));
    }

    validate_semantics(&manifest)
}

fn validate_semantics(manifest: &Value) -> Result<(), StimulusError> {
    if manifest.get("manifest_id").and_then(Value::as_str) != Some(MANIFEST_ID) {
        return Err(StimulusError(
            "manifest identity must remain ODRH-STIM-001".to_owned(),
        ));
    }
    if manifest.get("status").and_then(Value::as_str) != Some("SPECIFICATION_ONLY") {
        return Err(StimulusError(
            "v0.1 stimulus manifest must remain SPECIFICATION_ONLY".to_owned(),
        ));
    }
    if manifest.get("evidence_class").and_then(Value::as_str) != Some("SYNTHETIC_STIMULUS") {
        return Err(StimulusError(
            "stimulus manifest evidence class must remain SYNTHETIC_STIMULUS".to_owned(),
        ));
    }
    if manifest
        .get("physical_output_validation")
        .and_then(Value::as_str)
        != Some("NOT_PERFORMED")
    {
        return Err(StimulusError(
            "physical output validation cannot be promoted by a manifest edit".to_owned(),
        ));
    }
    if required_str(manifest, "claim_ceiling")? != CLAIM_CEILING {
        return Err(StimulusError(
            "manifest claim ceiling does not match the frozen v0.1 boundary".to_owned(),
        ));
    }
    if manifest.pointer("/encoding/format").and_then(Value::as_str) != Some("PPM_P6")
        || manifest
            .pointer("/encoding/channel_order")
            .and_then(Value::as_str)
            != Some("RGB")
        || manifest
            .pointer("/encoding/transfer_semantics")
            .and_then(Value::as_str)
            != Some("EXPLICIT_CODE_VALUES_ONLY")
    {
        return Err(StimulusError(
            "v0.1 permits only explicit-code-value PPM P6 RGB encoding".to_owned(),
        ));
    }

    let source = manifest
        .get("source_corpus")
        .ok_or_else(|| StimulusError("source_corpus is required".to_owned()))?;
    if source.get("corpus_id").and_then(Value::as_str) != Some(CORPUS_ID)
        || source.get("git_blob_sha1").and_then(Value::as_str) != Some(CORPUS_BLOB_SHA1)
        || source.get("source_status").and_then(Value::as_str) != Some("UNQUALIFIED")
    {
        return Err(StimulusError(
            "source workload corpus identity/digest/status changed; review before updating"
                .to_owned(),
        ));
    }

    let stimuli = manifest
        .get("stimuli")
        .and_then(Value::as_array)
        .ok_or_else(|| StimulusError("stimuli must be an array".to_owned()))?;
    let mut stimulus_ids = BTreeSet::new();
    let mut source_ids = BTreeSet::new();
    for stimulus in stimuli {
        let id = required_str(stimulus, "id")?;
        let source_case_id = required_str(stimulus, "source_case_id")?;
        let primitive = required_str(stimulus, "primitive")?;
        if !stimulus_ids.insert(id.to_owned()) {
            return Err(StimulusError(format!("duplicate stimulus ID: {id}")));
        }
        if !source_ids.insert(source_case_id.to_owned()) {
            return Err(StimulusError(format!(
                "duplicate source case ID: {source_case_id}"
            )));
        }
        let (expected_primitive, expected_claim_limit) = SUPPORTED_CASES
            .iter()
            .find_map(|(case, kind, claim_limit)| {
                (*case == source_case_id).then_some((*kind, *claim_limit))
            })
            .ok_or_else(|| {
                StimulusError(format!(
                    "source case is not in the frozen v0.1 renderable allowlist: {source_case_id}"
                ))
            })?;
        if primitive != expected_primitive {
            return Err(StimulusError(format!(
                "source case {source_case_id} requires primitive {expected_primitive}, not {primitive}"
            )));
        }
        if required_str(stimulus, "claim_limit")? != expected_claim_limit {
            return Err(StimulusError(format!(
                "stimulus {id} claim_limit is not the frozen v0.1 claim boundary"
            )));
        }
        if stimulus.get("status").and_then(Value::as_str) != Some("UNQUALIFIED") {
            return Err(StimulusError(format!(
                "stimulus {id} cannot self-promote beyond UNQUALIFIED"
            )));
        }
        let spec = parse_raster_spec(stimulus)?;
        validate_parameters(stimulus, spec)?;
    }
    Ok(())
}

fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, StimulusError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| StimulusError(format!("missing or empty string field: {key}")))
}

fn required_u64(value: &Value, key: &str) -> Result<u64, StimulusError> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| StimulusError(format!("missing or invalid integer field: {key}")))
}

fn parse_raster_spec(stimulus: &Value) -> Result<RasterSpec, StimulusError> {
    let id = required_str(stimulus, "id")?;
    let width = required_u64(stimulus, "width")?;
    let height = required_u64(stimulus, "height")?;
    let bit_depth = required_u64(stimulus, "bit_depth")?;
    let maxval = required_u64(stimulus, "maxval")?;

    if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(StimulusError(format!(
            "stimulus {id} dimensions must be in 1..={MAX_DIMENSION}"
        )));
    }
    let expected_maxval = match bit_depth {
        8 => 255,
        10 => 1023,
        _ => {
            return Err(StimulusError(format!(
                "stimulus {id} bit_depth must be 8 or 10"
            )));
        }
    };
    if maxval != expected_maxval {
        return Err(StimulusError(format!(
            "stimulus {id} maxval {maxval} is inconsistent with bit_depth {bit_depth}"
        )));
    }
    let bytes_per_sample = if maxval < 256 { 1_u64 } else { 2_u64 };
    let raster_bytes = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(3))
        .and_then(|samples| samples.checked_mul(bytes_per_sample))
        .ok_or_else(|| StimulusError(format!("stimulus {id} raster size overflow")))?;
    if raster_bytes > MAX_RASTER_BYTES {
        return Err(StimulusError(format!(
            "stimulus {id} raster exceeds the {MAX_RASTER_BYTES}-byte v0.1 limit"
        )));
    }

    Ok(RasterSpec {
        width: width as u32,
        height: height as u32,
        maxval: maxval as u16,
        bit_depth: bit_depth as u8,
    })
}

fn validate_parameters(stimulus: &Value, spec: RasterSpec) -> Result<(), StimulusError> {
    let id = required_str(stimulus, "id")?;
    let primitive = required_str(stimulus, "primitive")?;
    let params = stimulus
        .get("parameters")
        .and_then(Value::as_object)
        .ok_or_else(|| StimulusError(format!("stimulus {id} parameters must be an object")))?;

    match primitive {
        "neutral_ramp" => {
            if !params.is_empty() {
                return Err(StimulusError(format!(
                    "stimulus {id} neutral_ramp accepts no parameters"
                )));
            }
            if spec.width < 2 {
                return Err(StimulusError(format!(
                    "stimulus {id} neutral_ramp width must be at least 2"
                )));
            }
        }
        "near_black_steps" => {
            if params.len() != 1 || !params.contains_key("levels") {
                return Err(StimulusError(format!(
                    "stimulus {id} near_black_steps requires only a levels array"
                )));
            }
            let levels = params
                .get("levels")
                .and_then(Value::as_array)
                .ok_or_else(|| StimulusError(format!("stimulus {id} levels must be an array")))?;
            if levels.len() < 2
                || levels.len() as u32 > spec.width
                || !spec.width.is_multiple_of(levels.len() as u32)
            {
                return Err(StimulusError(format!(
                    "stimulus {id} width must divide evenly into at least two level bars"
                )));
            }
            let mut previous = None;
            for level in levels {
                let value = level.as_u64().ok_or_else(|| {
                    StimulusError(format!("stimulus {id} levels must be nonnegative integers"))
                })?;
                if value > u64::from(spec.maxval) {
                    return Err(StimulusError(format!(
                        "stimulus {id} level {value} exceeds maxval {}",
                        spec.maxval
                    )));
                }
                if previous.is_some_and(|last| value <= last) {
                    return Err(StimulusError(format!(
                        "stimulus {id} levels must be strictly increasing"
                    )));
                }
                previous = Some(value);
            }
        }
        "center_patch" => {
            if params.len() != 4
                || !params.contains_key("background_code")
                || !params.contains_key("foreground_code")
                || !params.contains_key("patch_width_px")
                || !params.contains_key("patch_height_px")
            {
                return Err(StimulusError(format!(
                    "stimulus {id} center_patch requires background_code, foreground_code and integer patch dimensions"
                )));
            }
            for field in ["background_code", "foreground_code"] {
                let values = params
                    .get(field)
                    .and_then(Value::as_array)
                    .filter(|values| values.len() == 3)
                    .ok_or_else(|| {
                        StimulusError(format!(
                            "stimulus {id} {field} must have exactly three RGB code values"
                        ))
                    })?;
                for value in values {
                    let code = value.as_u64().ok_or_else(|| {
                        StimulusError(format!(
                            "stimulus {id} {field} values must be nonnegative integers"
                        ))
                    })?;
                    if code > u64::from(spec.maxval) {
                        return Err(StimulusError(format!(
                            "stimulus {id} {field} value {code} exceeds maxval {}",
                            spec.maxval
                        )));
                    }
                }
            }
            let patch_width = required_u64(params_value(stimulus)?, "patch_width_px")?;
            let patch_height = required_u64(params_value(stimulus)?, "patch_height_px")?;
            if patch_width == 0
                || patch_height == 0
                || patch_width >= u64::from(spec.width)
                || patch_height >= u64::from(spec.height)
                || (u64::from(spec.width) - patch_width) % 2 != 0
                || (u64::from(spec.height) - patch_height) % 2 != 0
            {
                return Err(StimulusError(format!(
                    "stimulus {id} patch dimensions must be positive, smaller than the raster, and symmetrically centered on integer pixels"
                )));
            }
        }
        "rgb_gradient" => {
            if params.len() != 1
                || params.get("blue_rule").and_then(Value::as_str)
                    != Some("ROUNDED_MEAN_OF_RED_GREEN")
            {
                return Err(StimulusError(format!(
                    "stimulus {id} rgb_gradient requires the declared blue_rule"
                )));
            }
            if spec.width < 2 || spec.height < 2 {
                return Err(StimulusError(format!(
                    "stimulus {id} rgb_gradient dimensions must both be at least 2"
                )));
            }
        }
        other => {
            return Err(StimulusError(format!(
                "stimulus {id} uses unsupported primitive: {other}"
            )));
        }
    }
    Ok(())
}

fn params_value(stimulus: &Value) -> Result<&Value, StimulusError> {
    stimulus
        .get("parameters")
        .ok_or_else(|| StimulusError("parameters object is required".to_owned()))
}

pub fn generate_stimulus(
    manifest_text: &str,
    schema_text: &str,
    stimulus_id: &str,
) -> Result<GeneratedStimulus, StimulusError> {
    validate_manifest_str(manifest_text, schema_text)?;
    let manifest: Value = serde_json::from_str(manifest_text)
        .map_err(|error| StimulusError(format!("manifest JSON parse error: {error}")))?;
    let stimuli = manifest
        .get("stimuli")
        .and_then(Value::as_array)
        .ok_or_else(|| StimulusError("stimuli must be an array".to_owned()))?;
    let stimulus = stimuli
        .iter()
        .find(|item| item.get("id").and_then(Value::as_str) == Some(stimulus_id))
        .ok_or_else(|| StimulusError(format!("unknown stimulus ID: {stimulus_id}")))?;

    let spec = parse_raster_spec(stimulus)?;
    let primitive = required_str(stimulus, "primitive")?;
    let params = params_value(stimulus)?;
    let mut raster = Vec::with_capacity(raster_capacity(spec)?);

    for y in 0..spec.height {
        for x in 0..spec.width {
            let rgb = sample_pixel(primitive, params, spec, x, y)?;
            for channel in rgb {
                append_sample(&mut raster, channel, spec.maxval)?;
            }
        }
    }

    let header = format!("P6\n{} {}\n{}\n", spec.width, spec.height, spec.maxval);
    let mut bytes = header.into_bytes();
    bytes.extend_from_slice(&raster);

    let manifest_sha256 = sha256_hex(manifest_text.as_bytes());
    let recipe_sha256 = sha256_hex(
        serde_json::to_string(stimulus)
            .map_err(|error| {
                StimulusError(format!("cannot canonicalize selected recipe: {error}"))
            })?
            .as_bytes(),
    );
    let file_sha256 = sha256_hex(&bytes);
    let bytes_per_sample = if spec.maxval < 256 { 1 } else { 2 };
    let report = json!({
        "report_schema_version": SCHEMA_VERSION,
        "manifest_id": MANIFEST_ID,
        "stimulus_id": required_str(stimulus, "id")?,
        "source_case_id": required_str(stimulus, "source_case_id")?,
        "source_corpus_id": CORPUS_ID,
        "source_corpus_git_blob_sha1": CORPUS_BLOB_SHA1,
        "manifest_sha256": manifest_sha256,
        "recipe_sha256": recipe_sha256,
        "recipe_schema_sha256": sha256_hex(schema_text.as_bytes()),
        "report_schema_sha256": sha256_hex(REPORT_SCHEMA_TEXT.as_bytes()),
        "generator": "display-stimulus-reference",
        "generator_version": env!("CARGO_PKG_VERSION"),
        "generator_source_revision": std::env::var("ODRH_STIMULUS_SOURCE_COMMIT")
            .unwrap_or_else(|_| "UNPINNED_WORKTREE".to_owned()),
        "raster": {
            "format": "PPM_P6",
            "channel_order": "RGB",
            "origin": "TOP_LEFT",
            "width": spec.width,
            "height": spec.height,
            "bit_depth": spec.bit_depth,
            "maxval": spec.maxval,
            "bytes_per_sample": bytes_per_sample,
            "sixteen_bit_sample_byte_order": if bytes_per_sample == 2 { "BIG_ENDIAN" } else { "NOT_APPLICABLE" },
            "transfer_semantics": "EXPLICIT_CODE_VALUES_ONLY",
            "color_profile": "NONE_EMBEDDED",
            "metadata_transport": "NOT_REPRESENTED"
        },
        "output_bytes": bytes.len(),
        "output_sha256": file_sha256,
        "evidence_class": "SYNTHETIC_STIMULUS",
        "status": "UNQUALIFIED",
        "physical_output_validation": "NOT_PERFORMED",
        "global_claim_ceiling": CLAIM_CEILING,
        "claim_ceiling": stimulus.get("claim_limit").and_then(Value::as_str).unwrap_or("No claim ceiling supplied.")
    });

    validate_report(&report)?;

    Ok(GeneratedStimulus { bytes, report })
}

fn validate_report(report: &Value) -> Result<(), StimulusError> {
    let schema: Value = serde_json::from_str(REPORT_SCHEMA_TEXT)
        .map_err(|error| StimulusError(format!("report schema JSON parse error: {error}")))?;
    if schema.get("$schema").and_then(Value::as_str) != Some(SCHEMA_DRAFT) {
        return Err(StimulusError(
            "generated report schema must declare JSON Schema Draft 2020-12".to_owned(),
        ));
    }
    let validator = jsonschema::draft202012::new(&schema)
        .map_err(|error| StimulusError(format!("report schema compilation failed: {error}")))?;
    let errors: Vec<String> = validator
        .iter_errors(report)
        .map(|error| format!("generated report schema violation: {error}"))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(StimulusError(errors.join("\\n")))
    }
}

fn raster_capacity(spec: RasterSpec) -> Result<usize, StimulusError> {
    let sample_bytes = if spec.maxval < 256 { 1_usize } else { 2_usize };
    usize::try_from(spec.width)
        .ok()
        .and_then(|width| width.checked_mul(spec.height as usize))
        .and_then(|pixels| pixels.checked_mul(3))
        .and_then(|samples| samples.checked_mul(sample_bytes))
        .ok_or_else(|| StimulusError("raster allocation size overflow".to_owned()))
}

fn sample_pixel(
    primitive: &str,
    params: &Value,
    spec: RasterSpec,
    x: u32,
    y: u32,
) -> Result<[u16; 3], StimulusError> {
    match primitive {
        "neutral_ramp" => {
            let code = rounded_scale(x, spec.width - 1, spec.maxval)?;
            Ok([code, code, code])
        }
        "near_black_steps" => {
            let levels = params
                .get("levels")
                .and_then(Value::as_array)
                .ok_or_else(|| StimulusError("levels array is required".to_owned()))?;
            let bar_width = spec.width / levels.len() as u32;
            let index = (x / bar_width) as usize;
            let code = levels[index]
                .as_u64()
                .ok_or_else(|| StimulusError("level must be a nonnegative integer".to_owned()))?
                as u16;
            Ok([code, code, code])
        }
        "center_patch" => {
            let patch_width = required_u64(params, "patch_width_px")? as u32;
            let patch_height = required_u64(params, "patch_height_px")? as u32;
            let left = (spec.width - patch_width) / 2;
            let top = (spec.height - patch_height) / 2;
            let inside = x >= left && x < left + patch_width && y >= top && y < top + patch_height;
            let color_key = if inside {
                "foreground_code"
            } else {
                "background_code"
            };
            let values = params
                .get(color_key)
                .and_then(Value::as_array)
                .ok_or_else(|| StimulusError(format!("{color_key} must be an RGB array")))?;
            Ok([
                read_code(&values[0])?,
                read_code(&values[1])?,
                read_code(&values[2])?,
            ])
        }
        "rgb_gradient" => {
            let red = rounded_scale(x, spec.width - 1, spec.maxval)?;
            let green = rounded_scale(y, spec.height - 1, spec.maxval)?;
            let blue = (u32::from(red) + u32::from(green)).div_ceil(2) as u16;
            Ok([red, green, blue])
        }
        other => Err(StimulusError(format!(
            "unsupported raster primitive: {other}"
        ))),
    }
}

fn read_code(value: &Value) -> Result<u16, StimulusError> {
    value
        .as_u64()
        .and_then(|code| u16::try_from(code).ok())
        .ok_or_else(|| StimulusError("RGB code must be a nonnegative 16-bit integer".to_owned()))
}

fn rounded_scale(position: u32, denominator: u32, maxval: u16) -> Result<u16, StimulusError> {
    if denominator == 0 {
        return Err(StimulusError(
            "raster dimension must allow a nonzero scale denominator".to_owned(),
        ));
    }
    let numerator = u64::from(position)
        .checked_mul(u64::from(maxval))
        .and_then(|value| value.checked_add(u64::from(denominator) / 2))
        .ok_or_else(|| StimulusError("integer scale calculation overflow".to_owned()))?;
    u16::try_from(numerator / u64::from(denominator))
        .map_err(|_| StimulusError("scaled channel value exceeds u16".to_owned()))
}

fn append_sample(output: &mut Vec<u8>, sample: u16, maxval: u16) -> Result<(), StimulusError> {
    if sample > maxval {
        return Err(StimulusError(format!(
            "generated sample {sample} exceeds declared maxval {maxval}"
        )));
    }
    if maxval < 256 {
        output.push(sample as u8);
    } else {
        output.extend_from_slice(&sample.to_be_bytes());
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest.iter().copied() {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{generate_stimulus, sha256_hex, validate_manifest_str};
    use serde_json::Value;

    const MANIFEST_TEXT: &str =
        include_str!("../../../docs/visual-technology/display-stimulus-recipes-v0.1.json");
    const SCHEMA_TEXT: &str =
        include_str!("../../../docs/visual-technology/display-stimulus-recipes.schema.json");

    fn parse_manifest() -> Value {
        serde_json::from_str(MANIFEST_TEXT).expect("frozen stimulus manifest must parse")
    }

    fn encoded(value: &Value) -> String {
        serde_json::to_string(value).expect("mutated stimulus manifest must serialize")
    }

    fn generate(id: &str) -> super::GeneratedStimulus {
        generate_stimulus(MANIFEST_TEXT, SCHEMA_TEXT, id)
            .expect("frozen known-answer stimulus should generate")
    }

    #[test]
    fn recipe_manifest_validates_but_remains_unqualified() {
        validate_manifest_str(MANIFEST_TEXT, SCHEMA_TEXT)
            .expect("frozen v0.1 stimulus manifest should validate");
        let manifest = parse_manifest();
        assert_eq!(
            manifest
                .get("physical_output_validation")
                .and_then(Value::as_str),
            Some("NOT_PERFORMED")
        );
        assert_eq!(
            manifest.get("evidence_class").and_then(Value::as_str),
            Some("SYNTHETIC_STIMULUS")
        );
    }

    #[test]
    fn generated_report_is_schema_valid_and_cannot_promote_physical_output() {
        let generated = generate("ODRH-STIM-GREY-RAMP-001");
        super::validate_report(&generated.report)
            .expect("generated report must satisfy its frozen schema");

        let mut mutated = generated.report;
        mutated["physical_output_validation"] = Value::String("VALIDATED".to_owned());
        assert!(
            super::validate_report(&mutated)
                .expect_err("report cannot self-promote physical validation")
                .0
                .contains("generated report schema violation")
        );
    }

    #[test]
    fn sha256_matches_independent_standard_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn neutral_ramp_has_exact_ppm_header_and_known_rgb_samples() {
        let mut manifest = parse_manifest();
        let stimulus = manifest["stimuli"]
            .as_array_mut()
            .expect("stimuli array")
            .iter_mut()
            .find(|item| item["id"] == "ODRH-STIM-GREY-RAMP-001")
            .expect("neutral ramp recipe");
        stimulus["width"] = Value::from(3);
        stimulus["height"] = Value::from(1);
        let generated =
            generate_stimulus(&encoded(&manifest), SCHEMA_TEXT, "ODRH-STIM-GREY-RAMP-001")
                .expect("3 x 1 neutral ramp");
        let mut expected = b"P6\n3 1\n255\n".to_vec();
        expected.extend_from_slice(&[0, 0, 0, 128, 128, 128, 255, 255, 255]);
        assert_eq!(generated.bytes, expected);
        assert_eq!(
            generated.report["physical_output_validation"],
            "NOT_PERFORMED"
        );
    }

    #[test]
    fn ten_bit_samples_use_two_byte_big_endian_encoding() {
        let generated = generate("ODRH-STIM-NEARBLACK-001");
        assert!(generated.bytes.starts_with(b"P6\n12 4\n1023\n"));
        let header_len = b"P6\n12 4\n1023\n".len();
        let raster = &generated.bytes[header_len..];
        assert_eq!(&raster[0..6], &[0, 0, 0, 0, 0, 0]);
        assert_eq!(&raster[12..18], &[0, 1, 0, 1, 0, 1]);
        assert_eq!(generated.report["raster"]["bytes_per_sample"], 2);
        assert_eq!(
            generated.report["raster"]["sixteen_bit_sample_byte_order"],
            "BIG_ENDIAN"
        );
    }

    #[test]
    fn center_patch_has_exact_declared_pixel_area_and_code_values() {
        let generated = generate("ODRH-STIM-LOCAL-HIGHLIGHT-001");
        let header_len = b"P6\n16 12\n1023\n".len();
        let pixels = &generated.bytes[header_len..];
        let sample = |x: usize, y: usize, channel: usize| -> u16 {
            let offset = ((y * 16 + x) * 3 + channel) * 2;
            u16::from_be_bytes([pixels[offset], pixels[offset + 1]])
        };
        assert_eq!(sample(0, 0, 0), 0);
        assert_eq!(sample(6, 4, 0), 1023);
        assert_eq!(sample(9, 7, 2), 1023);
        assert_eq!(sample(5, 4, 0), 0);
        assert_eq!(sample(10, 4, 0), 0);
        let white_pixels = (0..12)
            .flat_map(|y| (0..16).map(move |x| (x, y)))
            .filter(|(x, y)| sample(*x, *y, 0) == 1023)
            .count();
        assert_eq!(white_pixels, 16);
    }

    #[test]
    fn gradient_known_corners_follow_code_value_contract() {
        let generated = generate("ODRH-STIM-COLOR-GRADIENT-001");
        let header_len = b"P6\n16 16\n255\n".len();
        let pixels = &generated.bytes[header_len..];
        let rgb = |x: usize, y: usize| {
            let offset = (y * 16 + x) * 3;
            [pixels[offset], pixels[offset + 1], pixels[offset + 2]]
        };
        assert_eq!(rgb(0, 0), [0, 0, 0]);
        assert_eq!(rgb(15, 0), [255, 0, 128]);
        assert_eq!(rgb(0, 15), [0, 255, 128]);
        assert_eq!(rgb(15, 15), [255, 255, 255]);
    }

    #[test]
    fn identical_manifest_and_case_produce_identical_bytes_and_report_digest() {
        let a = generate("ODRH-STIM-GREY-RAMP-001");
        let b = generate("ODRH-STIM-GREY-RAMP-001");
        assert_eq!(a.bytes, b.bytes);
        assert_eq!(a.report["output_sha256"], b.report["output_sha256"]);
        assert_eq!(a.report["manifest_sha256"], b.report["manifest_sha256"]);
    }

    #[test]
    fn unsupported_source_case_cannot_enter_frozen_v01_renderer() {
        let mut manifest = parse_manifest();
        manifest["stimuli"][0]["source_case_id"] = Value::String("VIS-DISP-UNKNOWN-999".to_owned());
        assert!(
            validate_manifest_str(&encoded(&manifest), SCHEMA_TEXT)
                .expect_err("unknown source case must fail")
                .0
                .contains("allowlist")
        );
    }

    #[test]
    fn manifest_cannot_self_promote_physical_output_validation() {
        let mut manifest = parse_manifest();
        manifest["physical_output_validation"] = Value::String("VALIDATED".to_owned());
        assert!(validate_manifest_str(&encoded(&manifest), SCHEMA_TEXT).is_err());
    }

    #[test]
    fn claim_limit_cannot_be_weakened_into_a_physical_result_claim() {
        let mut manifest = parse_manifest();
        manifest["stimuli"][1]["claim_limit"] =
            Value::String("This proves physical luminance in nits".to_owned());
        assert!(
            validate_manifest_str(&encoded(&manifest), SCHEMA_TEXT)
                .expect_err("overclaiming claim_limit must fail closed")
                .0
                .contains("claim_limit is not the frozen")
        );
    }

    #[test]
    fn recipe_cannot_self_promote_stimulus_status() {
        let mut manifest = parse_manifest();
        manifest["stimuli"][1]["status"] = Value::String("QUALIFIED".to_owned());
        assert!(
            validate_manifest_str(&encoded(&manifest), SCHEMA_TEXT)
                .expect_err("stimulus status cannot be promoted by editing the recipe")
                .0
                .contains("UNQUALIFIED")
        );
    }

    #[test]
    fn invalid_levels_and_nondivisible_bars_fail_closed() {
        let mut manifest = parse_manifest();
        manifest["stimuli"][1]["parameters"]["levels"] = serde_json::json!([0, 1, 1024]);
        assert!(
            validate_manifest_str(&encoded(&manifest), SCHEMA_TEXT)
                .expect_err("levels beyond declared maxval must fail")
                .0
                .contains("exceeds maxval")
        );

        let mut manifest = parse_manifest();
        manifest["stimuli"][1]["width"] = Value::from(11);
        assert!(
            validate_manifest_str(&encoded(&manifest), SCHEMA_TEXT)
                .expect_err("uneven bars must fail")
                .0
                .contains("divide evenly")
        );
    }

    #[test]
    fn duplicate_stimulus_identity_is_rejected() {
        let mut manifest = parse_manifest();
        let first_id = manifest["stimuli"][0]["id"].clone();
        manifest["stimuli"][1]["id"] = first_id;
        assert!(
            validate_manifest_str(&encoded(&manifest), SCHEMA_TEXT)
                .expect_err("duplicate stimulus IDs must fail")
                .0
                .contains("duplicate stimulus ID")
        );
    }
}
