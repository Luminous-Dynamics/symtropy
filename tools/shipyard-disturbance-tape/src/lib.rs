//! Strict, dependency-free validator for shipyard disturbance tapes.
//!
//! The parser rejects duplicate JSON keys instead of accepting a lossy map
//! representation. Validation creates an observation snapshot only through an
//! explicit decision tick. The complete evaluator stream must never be mounted
//! in the policy process.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::fs;
use std::path::Path;

const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
const MAX_JSON_DEPTH: usize = 128;
const CONTRACT_VERSION: &str = "shipyard-disturbance-tape-v0";

#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

impl Json {
    fn stringify(&self) -> String {
        let mut output = String::new();
        self.write_json(&mut output);
        output
    }

    fn write_json(&self, output: &mut String) {
        match self {
            Json::Null => output.push_str("null"),
            Json::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
            Json::Number(value) => output.push_str(value),
            Json::String(value) => write_json_string(value, output),
            Json::Array(values) => {
                output.push('[');
                for (index, value) in values.iter().enumerate() {
                    if index != 0 {
                        output.push(',');
                    }
                    value.write_json(output);
                }
                output.push(']');
            }
            Json::Object(values) => {
                output.push('{');
                for (index, (key, value)) in values.iter().enumerate() {
                    if index != 0 {
                        output.push(',');
                    }
                    write_json_string(key, output);
                    output.push(':');
                    value.write_json(output);
                }
                output.push('}');
            }
        }
    }

    fn as_object(&self, path: &str) -> Result<&BTreeMap<String, Json>, String> {
        match self {
            Json::Object(values) => Ok(values),
            _ => Err(format!("{path}: expected object")),
        }
    }

    fn as_array(&self, path: &str) -> Result<&[Json], String> {
        match self {
            Json::Array(values) => Ok(values),
            _ => Err(format!("{path}: expected array")),
        }
    }

    fn as_str<'a>(&'a self, path: &str) -> Result<&'a str, String> {
        match self {
            Json::String(value) => Ok(value),
            _ => Err(format!("{path}: expected string")),
        }
    }

    fn as_u64(&self, path: &str) -> Result<u64, String> {
        match self {
            Json::Number(raw)
                if !raw.starts_with('-')
                    && !raw.contains('.')
                    && !raw.contains('e')
                    && !raw.contains('E') =>
            {
                raw.parse::<u64>()
                    .map_err(|_| format!("{path}: expected a non-negative integer in u64 range"))
            }
            _ => Err(format!(
                "{path}: expected a non-negative integer in u64 range"
            )),
        }
    }
}

fn write_json_string(value: &str, output: &mut String) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{0008}' => output.push_str("\\b"),
            '\u{000c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character <= '\u{001f}' => {
                let _ = write!(output, "\\u{:04x}", character as u32);
            }
            character => output.push(character),
        }
    }
    output.push('"');
}

fn string_json(value: impl Into<String>) -> Json {
    Json::String(value.into())
}

fn number_json(value: u64) -> Json {
    Json::Number(value.to_string())
}

fn object_json(entries: impl IntoIterator<Item = (String, Json)>) -> Json {
    Json::Object(entries.into_iter().collect())
}

struct Parser<'a> {
    input: &'a str,
    bytes: &'a [u8],
    index: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            bytes: input.as_bytes(),
            index: 0,
        }
    }

    fn error(&self, message: &str) -> String {
        format!("JSON byte {}: {message}", self.index)
    }

    fn skip_whitespace(&mut self) {
        while self.index < self.bytes.len()
            && matches!(self.bytes[self.index], b' ' | b'\n' | b'\r' | b'\t')
        {
            self.index += 1;
        }
    }

    fn parse(mut self) -> Result<Json, String> {
        let value = self.parse_value(0)?;
        self.skip_whitespace();
        if self.index != self.bytes.len() {
            return Err(self.error("trailing bytes after JSON value"));
        }
        Ok(value)
    }

    fn parse_value(&mut self, depth: usize) -> Result<Json, String> {
        if depth > MAX_JSON_DEPTH {
            return Err(self.error("maximum nesting depth exceeded"));
        }
        self.skip_whitespace();
        match self.bytes.get(self.index).copied() {
            Some(b'n') => {
                self.parse_literal("null")?;
                Ok(Json::Null)
            }
            Some(b't') => {
                self.parse_literal("true")?;
                Ok(Json::Bool(true))
            }
            Some(b'f') => {
                self.parse_literal("false")?;
                Ok(Json::Bool(false))
            }
            Some(b'"') => self.parse_string().map(Json::String),
            Some(b'[') => self.parse_array(depth + 1),
            Some(b'{') => self.parse_object(depth + 1),
            Some(b'-' | b'0'..=b'9') => self.parse_number().map(Json::Number),
            Some(_) => Err(self.error("unexpected character")),
            None => Err(self.error("unexpected end of input")),
        }
    }

    fn parse_literal(&mut self, literal: &str) -> Result<(), String> {
        let end = self.index + literal.len();
        if self.bytes.get(self.index..end) == Some(literal.as_bytes()) {
            self.index = end;
            Ok(())
        } else {
            Err(self.error("invalid literal"))
        }
    }

    fn parse_array(&mut self, depth: usize) -> Result<Json, String> {
        self.index += 1;
        self.skip_whitespace();
        let mut values = Vec::new();
        if self.bytes.get(self.index) == Some(&b']') {
            self.index += 1;
            return Ok(Json::Array(values));
        }

        loop {
            values.push(self.parse_value(depth)?);
            self.skip_whitespace();
            match self.bytes.get(self.index) {
                Some(b',') => self.index += 1,
                Some(b']') => {
                    self.index += 1;
                    return Ok(Json::Array(values));
                }
                _ => return Err(self.error("expected comma or closing bracket")),
            }
        }
    }

    fn parse_object(&mut self, depth: usize) -> Result<Json, String> {
        self.index += 1;
        self.skip_whitespace();
        let mut values = BTreeMap::new();
        if self.bytes.get(self.index) == Some(&b'}') {
            self.index += 1;
            return Ok(Json::Object(values));
        }

        loop {
            self.skip_whitespace();
            if self.bytes.get(self.index) != Some(&b'"') {
                return Err(self.error("object key must be a string"));
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.bytes.get(self.index) != Some(&b':') {
                return Err(self.error("expected colon after object key"));
            }
            self.index += 1;
            let value = self.parse_value(depth)?;
            if values.insert(key.clone(), value).is_some() {
                return Err(self.error(&format!("duplicate object key {key:?}")));
            }
            self.skip_whitespace();
            match self.bytes.get(self.index) {
                Some(b',') => self.index += 1,
                Some(b'}') => {
                    self.index += 1;
                    return Ok(Json::Object(values));
                }
                _ => return Err(self.error("expected comma or closing brace")),
            }
        }
    }

    fn parse_string(&mut self) -> Result<String, String> {
        if self.bytes.get(self.index) != Some(&b'"') {
            return Err(self.error("expected string"));
        }
        self.index += 1;
        let mut output = String::new();
        loop {
            let byte = self
                .bytes
                .get(self.index)
                .copied()
                .ok_or_else(|| self.error("unterminated string"))?;
            match byte {
                b'"' => {
                    self.index += 1;
                    return Ok(output);
                }
                b'\\' => {
                    self.index += 1;
                    let escaped = self
                        .bytes
                        .get(self.index)
                        .copied()
                        .ok_or_else(|| self.error("incomplete escape"))?;
                    self.index += 1;
                    match escaped {
                        b'"' => output.push('"'),
                        b'\\' => output.push('\\'),
                        b'/' => output.push('/'),
                        b'b' => output.push('\u{0008}'),
                        b'f' => output.push('\u{000c}'),
                        b'n' => output.push('\n'),
                        b'r' => output.push('\r'),
                        b't' => output.push('\t'),
                        b'u' => {
                            let first = self.parse_hex_code_unit()?;
                            let scalar = if (0xD800..=0xDBFF).contains(&first) {
                                if self.bytes.get(self.index..self.index + 2) != Some(&b"\\u"[..]) {
                                    return Err(self.error("high surrogate lacks low surrogate"));
                                }
                                self.index += 2;
                                let second = self.parse_hex_code_unit()?;
                                if !(0xDC00..=0xDFFF).contains(&second) {
                                    return Err(self.error("invalid low surrogate"));
                                }
                                0x10000
                                    + (((first as u32 - 0xD800) << 10) | (second as u32 - 0xDC00))
                            } else if (0xDC00..=0xDFFF).contains(&first) {
                                return Err(self.error("unexpected low surrogate"));
                            } else {
                                first as u32
                            };
                            output.push(
                                char::from_u32(scalar)
                                    .ok_or_else(|| self.error("invalid Unicode scalar"))?,
                            );
                        }
                        _ => return Err(self.error("invalid string escape")),
                    }
                }
                byte if byte < 0x20 => {
                    return Err(self.error("unescaped control character in string"));
                }
                _ => {
                    let character = self.input[self.index..]
                        .chars()
                        .next()
                        .ok_or_else(|| self.error("invalid UTF-8 boundary"))?;
                    output.push(character);
                    self.index += character.len_utf8();
                }
            }
        }
    }

    fn parse_hex_code_unit(&mut self) -> Result<u16, String> {
        let end = self.index + 4;
        let slice = self
            .bytes
            .get(self.index..end)
            .ok_or_else(|| self.error("incomplete Unicode escape"))?;
        let mut value = 0u16;
        for byte in slice {
            let digit = match byte {
                b'0'..=b'9' => (byte - b'0') as u16,
                b'a'..=b'f' => (byte - b'a' + 10) as u16,
                b'A'..=b'F' => (byte - b'A' + 10) as u16,
                _ => return Err(self.error("invalid Unicode escape")),
            };
            value = (value << 4) | digit;
        }
        self.index = end;
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<String, String> {
        let start = self.index;
        if self.bytes.get(self.index) == Some(&b'-') {
            self.index += 1;
        }

        match self.bytes.get(self.index) {
            Some(b'0') => {
                self.index += 1;
                if self
                    .bytes
                    .get(self.index)
                    .is_some_and(|byte| byte.is_ascii_digit())
                {
                    return Err(self.error("leading zero in number"));
                }
            }
            Some(b'1'..=b'9') => {
                self.index += 1;
                while self
                    .bytes
                    .get(self.index)
                    .is_some_and(|byte| byte.is_ascii_digit())
                {
                    self.index += 1;
                }
            }
            _ => return Err(self.error("invalid number integer part")),
        }

        if self.bytes.get(self.index) == Some(&b'.') {
            self.index += 1;
            let fraction_start = self.index;
            while self
                .bytes
                .get(self.index)
                .is_some_and(|byte| byte.is_ascii_digit())
            {
                self.index += 1;
            }
            if self.index == fraction_start {
                return Err(self.error("fraction requires at least one digit"));
            }
        }

        if self
            .bytes
            .get(self.index)
            .is_some_and(|byte| matches!(byte, b'e' | b'E'))
        {
            self.index += 1;
            if self
                .bytes
                .get(self.index)
                .is_some_and(|byte| matches!(byte, b'+' | b'-'))
            {
                self.index += 1;
            }
            let exponent_start = self.index;
            while self
                .bytes
                .get(self.index)
                .is_some_and(|byte| byte.is_ascii_digit())
            {
                self.index += 1;
            }
            if self.index == exponent_start {
                return Err(self.error("exponent requires at least one digit"));
            }
        }

        Ok(self.input[start..self.index].to_owned())
    }
}

fn parse_json(input: &str) -> Result<Json, String> {
    Parser::new(input).parse()
}

fn require_keys(
    object: &BTreeMap<String, Json>,
    required: &[&str],
    optional: &[&str],
    path: &str,
) -> Result<(), String> {
    for key in required {
        if !object.contains_key(*key) {
            return Err(format!("{path}: missing required field {key}"));
        }
    }
    for key in object.keys() {
        if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
            return Err(format!("{path}: unknown field {key}"));
        }
    }
    Ok(())
}

fn field<'a>(
    object: &'a BTreeMap<String, Json>,
    key: &str,
    path: &str,
) -> Result<&'a Json, String> {
    object
        .get(key)
        .ok_or_else(|| format!("{path}: missing required field {key}"))
}

fn string_field<'a>(
    object: &'a BTreeMap<String, Json>,
    key: &str,
    path: &str,
) -> Result<&'a str, String> {
    let value = field(object, key, path)?.as_str(&format!("{path}.{key}"))?;
    if value.trim().is_empty() {
        return Err(format!("{path}.{key}: must not be empty"));
    }
    Ok(value)
}

fn tick_field(object: &BTreeMap<String, Json>, key: &str, path: &str) -> Result<u64, String> {
    field(object, key, path)?.as_u64(&format!("{path}.{key}"))
}

fn validate_payload(
    value: &Json,
    required_strings: &[&str],
    required_ticks: &[&str],
    path: &str,
) -> Result<BTreeMap<String, Json>, String> {
    let object = value.as_object(path)?;
    let required: Vec<&str> = required_strings
        .iter()
        .chain(required_ticks.iter())
        .copied()
        .collect();
    require_keys(object, &required, &[], path)?;
    for key in required_strings {
        string_field(object, key, path)?;
    }
    for key in required_ticks {
        tick_field(object, key, path)?;
    }
    Ok(object.clone())
}

#[derive(Debug, Clone)]
struct Event {
    id: String,
    event_type: String,
    effective_tick: u64,
    observed_tick: u64,
    delivery_tick: u64,
    policy_visible: bool,
    observation: Json,
    raw: Json,
}

fn validate_event(value: &Json, index: usize) -> Result<Event, String> {
    let path = format!("events[{index}]");
    let object = value.as_object(&path)?;
    require_keys(
        object,
        &[
            "event_id",
            "event_type",
            "effective_tick",
            "observed_tick",
            "delivery_tick",
            "visibility",
            "truth",
            "observation",
        ],
        &[],
        &path,
    )?;

    let id = string_field(object, "event_id", &path)?.to_owned();
    let event_type = string_field(object, "event_type", &path)?.to_owned();
    let effective_tick = tick_field(object, "effective_tick", &path)?;
    let observed_tick = tick_field(object, "observed_tick", &path)?;
    let delivery_tick = tick_field(object, "delivery_tick", &path)?;
    if delivery_tick < observed_tick {
        return Err(format!(
            "{path}: delivery_tick {delivery_tick} precedes observed_tick {observed_tick}"
        ));
    }

    let visibility = string_field(object, "visibility", &path)?;
    let policy_visible = match visibility {
        "policy_visible" => true,
        "evaluator_only" => false,
        _ => return Err(format!("{path}.visibility: unsupported value {visibility}")),
    };
    let truth = field(object, "truth", &path)?;
    let observation = field(object, "observation", &path)?;

    let (truth_id_key, truth_strings, truth_ticks, observation_id_key, observation_status) =
        match event_type.as_str() {
            "vehicle_outage" => (
                "vehicle_id",
                vec!["vehicle_id", "cause"],
                vec!["unavailable_from_tick", "unavailable_until_tick"],
                "vehicle_id",
                "unavailable",
            ),
            "workstation_outage" => (
                "workstation_id",
                vec!["workstation_id", "cause"],
                vec!["unavailable_from_tick", "unavailable_until_tick"],
                "workstation_id",
                "unavailable",
            ),
            "yard_closure" => (
                "zone_id",
                vec!["zone_id", "cause"],
                vec!["closed_from_tick", "closed_until_tick"],
                "zone_id",
                "closed",
            ),
            "clearance_delayed" => (
                "block_id",
                vec!["block_id", "cause"],
                vec!["clearance_tick"],
                "block_id",
                "delayed",
            ),
            _ => return Err(format!("{path}.event_type: unsupported type {event_type}")),
        };

    let truth_object = validate_payload(
        truth,
        &truth_strings,
        &truth_ticks,
        &format!("{path}.truth"),
    )?;
    if event_type == "vehicle_outage" || event_type == "workstation_outage" {
        if tick_field(&truth_object, truth_ticks[0], &format!("{path}.truth"))?
            > tick_field(&truth_object, truth_ticks[1], &format!("{path}.truth"))?
        {
            return Err(format!("{path}.truth: unavailable interval is reversed"));
        }
    }
    if event_type == "yard_closure"
        && tick_field(&truth_object, truth_ticks[0], &format!("{path}.truth"))?
            > tick_field(&truth_object, truth_ticks[1], &format!("{path}.truth"))?
    {
        return Err(format!("{path}.truth: closure interval is reversed"));
    }

    let observation_value = if policy_visible {
        if matches!(observation, Json::Null) {
            return Err(format!(
                "{path}: policy_visible event requires non-null observation"
            ));
        }
        let observation_object = validate_payload(
            observation,
            &[observation_id_key, "status"],
            &["reported_at_tick"],
            &format!("{path}.observation"),
        )?;
        let truth_id = string_field(&truth_object, truth_id_key, &format!("{path}.truth"))?;
        let observation_id = string_field(
            &observation_object,
            observation_id_key,
            &format!("{path}.observation"),
        )?;
        if truth_id != observation_id {
            return Err(format!(
                "{path}: truth identity does not match observation identity"
            ));
        }
        let status = string_field(
            &observation_object,
            "status",
            &format!("{path}.observation"),
        )?;
        if status != observation_status {
            return Err(format!(
                "{path}.observation.status: expected {observation_status} for {event_type}"
            ));
        }
        observation.clone()
    } else {
        if !matches!(observation, Json::Null) {
            return Err(format!(
                "{path}: evaluator_only event must have null observation"
            ));
        }
        Json::Null
    };

    Ok(Event {
        id,
        event_type,
        effective_tick,
        observed_tick,
        delivery_tick,
        policy_visible,
        observation: observation_value,
        raw: value.clone(),
    })
}

/// A validated tape plus separate policy and evaluator replay products.
///
/// The policy product is a snapshot at an explicit tick, not the full tape. The
/// evaluator product includes truth and must remain in the evaluator boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    pub episode_id: String,
    pub digest_sha256: String,
    pub event_count: usize,
    pub as_of_tick: u64,
    pub policy_view_json: String,
    pub evaluator_stream_json: String,
}

/// SHA-256 over exact source bytes, returned as lowercase hexadecimal.
pub fn sha256_hex(input: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xc24b8b70,
        0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08,
        0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3, 0x748f82ee,
        0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_length = (input.len() as u64).wrapping_mul(8);
    let mut padded = input.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_length.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut words = [0u32; 64];
        for (index, word_bytes) in chunk.chunks_exact(4).take(16).enumerate() {
            words[index] =
                u32::from_be_bytes([word_bytes[0], word_bytes[1], word_bytes[2], word_bytes[3]]);
        }
        for index in 16..64 {
            let x = words[index - 15];
            let y = words[index - 2];
            let sigma0 = x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3);
            let sigma1 = y.rotate_right(17) ^ y.rotate_right(19) ^ (y >> 10);
            words[index] = words[index - 16]
                .wrapping_add(sigma0)
                .wrapping_add(words[index - 7])
                .wrapping_add(sigma1);
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];
        for index in 0..64 {
            let sigma1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choice = (e & f) ^ ((!e) & g);
            let temporary1 = h
                .wrapping_add(sigma1)
                .wrapping_add(choice)
                .wrapping_add(K[index])
                .wrapping_add(words[index]);
            let sigma0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temporary2 = sigma0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temporary1);
            d = c;
            c = b;
            b = a;
            a = temporary1.wrapping_add(temporary2);
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    let mut output = String::with_capacity(64);
    for word in state {
        let _ = write!(output, "{word:08x}");
    }
    output
}

fn validate_digest(expected: &str, actual: &str) -> Result<(), String> {
    if expected.len() != 64 || !expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("expected SHA-256 must be exactly 64 hexadecimal characters".to_owned());
    }
    if !expected.eq_ignore_ascii_case(actual) {
        return Err(format!(
            "exact-byte SHA-256 mismatch: expected {}, computed {}",
            expected.to_ascii_lowercase(),
            actual
        ));
    }
    Ok(())
}

/// Validate exact source bytes, optionally verify their declared SHA-256, and
/// construct a policy observation snapshot for a single decision tick.
pub fn validate_bytes(
    bytes: &[u8],
    expected_sha256: Option<&str>,
    as_of_tick: u64,
) -> Result<ValidationReport, String> {
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(format!("input exceeds {MAX_INPUT_BYTES} byte safety limit"));
    }
    let digest_sha256 = sha256_hex(bytes);
    if let Some(expected) = expected_sha256 {
        validate_digest(expected, &digest_sha256)?;
    }
    let input =
        std::str::from_utf8(bytes).map_err(|error| format!("input is not UTF-8: {error}"))?;
    let root = parse_json(input)?;
    let root_object = root.as_object("root")?;
    require_keys(
        root_object,
        &[
            "contract_version",
            "episode_id",
            "tick_duration_ms",
            "events",
        ],
        &["notes"],
        "root",
    )?;

    let version = string_field(root_object, "contract_version", "root")?;
    if version != CONTRACT_VERSION {
        return Err(format!(
            "root.contract_version: unsupported version {version}"
        ));
    }
    let episode_id = string_field(root_object, "episode_id", "root")?.to_owned();
    let tick_duration_ms = tick_field(root_object, "tick_duration_ms", "root")?;
    if tick_duration_ms == 0 {
        return Err("root.tick_duration_ms must be greater than zero".to_owned());
    }
    if let Some(notes) = root_object.get("notes") {
        for (index, note) in notes.as_array("root.notes")?.iter().enumerate() {
            note.as_str(&format!("root.notes[{index}]"))?;
        }
    }

    let input_events = field(root_object, "events", "root")?.as_array("root.events")?;
    if input_events.is_empty() {
        return Err("root.events must contain at least one event".to_owned());
    }
    let mut ids = BTreeSet::new();
    let mut events = Vec::with_capacity(input_events.len());
    for (index, value) in input_events.iter().enumerate() {
        let event = validate_event(value, index)?;
        if !ids.insert(event.id.clone()) {
            return Err(format!("events: duplicate event_id {}", event.id));
        }
        events.push(event);
    }

    let mut policy_events: Vec<&Event> = events
        .iter()
        .filter(|event| event.policy_visible && event.delivery_tick <= as_of_tick)
        .collect();
    policy_events.sort_by(|left, right| {
        left.delivery_tick
            .cmp(&right.delivery_tick)
            .then_with(|| left.id.cmp(&right.id))
    });
    let policy_observations = policy_events
        .into_iter()
        .map(|event| {
            object_json([
                ("event_id".to_owned(), string_json(event.id.clone())),
                (
                    "event_type".to_owned(),
                    string_json(event.event_type.clone()),
                ),
                ("observed_tick".to_owned(), number_json(event.observed_tick)),
                ("delivery_tick".to_owned(), number_json(event.delivery_tick)),
                ("observation".to_owned(), event.observation.clone()),
            ])
        })
        .collect::<Vec<_>>();
    let policy_view = object_json([
        ("contract_version".to_owned(), string_json(CONTRACT_VERSION)),
        ("episode_id".to_owned(), string_json(episode_id.clone())),
        ("as_of_tick".to_owned(), number_json(as_of_tick)),
        ("observations".to_owned(), Json::Array(policy_observations)),
    ]);

    let mut evaluator_events = events.iter().collect::<Vec<_>>();
    evaluator_events.sort_by(|left, right| {
        left.effective_tick
            .cmp(&right.effective_tick)
            .then_with(|| left.id.cmp(&right.id))
    });
    let evaluator_events = evaluator_events
        .into_iter()
        .map(|event| event.raw.clone())
        .collect::<Vec<_>>();
    let evaluator_stream = object_json([
        ("contract_version".to_owned(), string_json(CONTRACT_VERSION)),
        ("episode_id".to_owned(), string_json(episode_id.clone())),
        ("tick_duration_ms".to_owned(), number_json(tick_duration_ms)),
        (
            "source_sha256".to_owned(),
            string_json(digest_sha256.clone()),
        ),
        ("events".to_owned(), Json::Array(evaluator_events)),
    ]);

    Ok(ValidationReport {
        episode_id,
        digest_sha256,
        event_count: events.len(),
        as_of_tick,
        policy_view_json: policy_view.stringify(),
        evaluator_stream_json: evaluator_stream.stringify(),
    })
}

/// Validate first, then write the policy snapshot and the separate evaluator stream.
pub fn validate_file(
    input_path: &Path,
    expected_sha256: &str,
    as_of_tick: u64,
    policy_output_path: &Path,
    evaluator_output_path: &Path,
) -> Result<ValidationReport, String> {
    let bytes = fs::read(input_path)
        .map_err(|error| format!("cannot read {}: {error}", input_path.display()))?;
    let report = validate_bytes(&bytes, Some(expected_sha256), as_of_tick)?;
    fs::write(policy_output_path, &report.policy_view_json).map_err(|error| {
        format!(
            "cannot write policy snapshot {}: {error}",
            policy_output_path.display()
        )
    })?;
    fs::write(evaluator_output_path, &report.evaluator_stream_json).map_err(|error| {
        format!(
            "cannot write evaluator stream {}: {error}",
            evaluator_output_path.display()
        )
    })?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        include_str!("../../shipyard-evaluation-lab/fixtures/synthetic-disturbance-tape-v0.json");

    fn valid_report(as_of_tick: u64) -> ValidationReport {
        let bytes = FIXTURE.as_bytes();
        validate_bytes(bytes, Some(&sha256_hex(bytes)), as_of_tick)
            .expect("synthetic fixture should validate")
    }

    #[test]
    fn sha256_matches_standard_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn fixture_validates_against_exact_byte_digest() {
        let bytes = FIXTURE.as_bytes();
        let report = validate_bytes(bytes, Some(&sha256_hex(bytes)), 5)
            .expect("synthetic fixture should validate");
        assert_eq!(report.episode_id, "synthetic-yard-001");
        assert_eq!(report.event_count, 3);
        assert_eq!(report.digest_sha256.len(), 64);
    }

    #[test]
    fn policy_snapshot_only_contains_delivered_visible_observations() {
        let at_five = valid_report(5);
        assert!(at_five.policy_view_json.contains("evt-vehicle-001"));
        assert!(!at_five.policy_view_json.contains("evt-yard-closure-001"));
        assert!(
            !at_five
                .policy_view_json
                .contains("evt-clearance-hidden-001")
        );
        assert!(!at_five.policy_view_json.contains("truth"));
        assert!(
            !at_five
                .policy_view_json
                .contains("synthetic_inspection_delay")
        );

        let at_twelve = valid_report(12);
        assert!(at_twelve.policy_view_json.contains("evt-yard-closure-001"));
        assert!(
            !at_twelve
                .policy_view_json
                .contains("evt-clearance-hidden-001")
        );
        assert!(
            !at_twelve
                .policy_view_json
                .contains("synthetic_inspection_delay")
        );
    }

    #[test]
    fn evaluator_stream_contains_all_events_in_effective_time_order() {
        let report = valid_report(5);
        let first = report
            .evaluator_stream_json
            .find("evt-vehicle-001")
            .unwrap();
        let second = report
            .evaluator_stream_json
            .find("evt-clearance-hidden-001")
            .unwrap();
        let third = report
            .evaluator_stream_json
            .find("evt-yard-closure-001")
            .unwrap();
        assert!(first < second && second < third);
        assert!(
            report
                .evaluator_stream_json
                .contains("synthetic_inspection_delay")
        );
    }

    #[test]
    fn duplicate_object_keys_are_rejected() {
        let error = parse_json(r#"{"x":1,"x":2}"#).expect_err("duplicate key");
        assert!(error.contains("duplicate object key"));
    }

    #[test]
    fn duplicate_event_ids_are_rejected() {
        let changed = FIXTURE.replace("evt-clearance-hidden-001", "evt-vehicle-001");
        let error = validate_bytes(changed.as_bytes(), None, 5).expect_err("duplicate ID");
        assert!(error.contains("duplicate event_id"));
    }

    #[test]
    fn delivery_before_observation_is_rejected() {
        let changed = FIXTURE.replace(
            "\"observed_tick\": 4,\n      \"delivery_tick\": 5",
            "\"observed_tick\": 4,\n      \"delivery_tick\": 3",
        );
        let error = validate_bytes(changed.as_bytes(), None, 5).expect_err("delivery violation");
        assert!(error.contains("precedes observed_tick"));
    }

    #[test]
    fn unknown_fields_and_event_types_are_rejected() {
        let unknown_field =
            FIXTURE.replace("\"notes\": [", "\"unexpected\": true,\n  \"notes\": [");
        assert!(
            validate_bytes(unknown_field.as_bytes(), None, 5)
                .expect_err("unknown root field")
                .contains("unknown field unexpected")
        );

        let unknown_type = FIXTURE.replace("vehicle_outage", "teleporting_vehicle");
        assert!(
            validate_bytes(unknown_type.as_bytes(), None, 5)
                .expect_err("unknown type")
                .contains("unsupported type")
        );
    }

    #[test]
    fn visibility_payload_mismatch_is_rejected() {
        let changed = FIXTURE.replace(
            "\"observation\": null",
            "\"observation\": {\"block_id\":\"blk-03\",\"status\":\"delayed\",\"reported_at_tick\":7}",
        );
        assert!(
            validate_bytes(changed.as_bytes(), None, 7)
                .expect_err("hidden payload")
                .contains("must have null observation")
        );
    }

    #[test]
    fn malformed_ticks_and_unknown_versions_are_rejected() {
        let negative = FIXTURE.replace("\"effective_tick\": 4", "\"effective_tick\": -1");
        assert!(
            validate_bytes(negative.as_bytes(), None, 5)
                .expect_err("negative tick")
                .contains("non-negative integer")
        );

        let fractional = FIXTURE.replace("\"effective_tick\": 4", "\"effective_tick\": 4.0");
        assert!(
            validate_bytes(fractional.as_bytes(), None, 5)
                .expect_err("fractional tick")
                .contains("non-negative integer")
        );

        let version = FIXTURE.replace(CONTRACT_VERSION, "shipyard-disturbance-tape-v99");
        assert!(
            validate_bytes(version.as_bytes(), None, 5)
                .expect_err("version")
                .contains("unsupported version")
        );
    }

    #[test]
    fn digest_mismatch_is_rejected() {
        let error =
            validate_bytes(FIXTURE.as_bytes(), Some(&"0".repeat(64)), 5).expect_err("wrong digest");
        assert!(error.contains("SHA-256 mismatch"));
    }

    #[test]
    fn hidden_future_mutation_cannot_change_earlier_policy_snapshot() {
        let baseline = valid_report(5);
        let changed = FIXTURE.replace("zone-west", "zone-east");
        let mutated = validate_bytes(changed.as_bytes(), None, 5).expect("future mutation valid");
        assert_eq!(baseline.policy_view_json, mutated.policy_view_json);
        assert_ne!(baseline.digest_sha256, mutated.digest_sha256);
    }

    #[test]
    fn input_event_order_is_semantically_irrelevant() {
        let baseline = valid_report(12);
        let mut root = parse_json(FIXTURE).expect("fixture JSON");
        let object = match &mut root {
            Json::Object(object) => object,
            _ => panic!("root object"),
        };
        match object.get_mut("events").expect("events") {
            Json::Array(events) => events.reverse(),
            _ => panic!("events array"),
        }
        let reordered = root.stringify();
        let reordered_report =
            validate_bytes(reordered.as_bytes(), None, 12).expect("reordered tape valid");
        assert_eq!(baseline.policy_view_json, reordered_report.policy_view_json);
        let baseline_evaluator =
            parse_json(&baseline.evaluator_stream_json).expect("baseline evaluator");
        let reordered_evaluator =
            parse_json(&reordered_report.evaluator_stream_json).expect("reordered evaluator");
        let baseline_events = baseline_evaluator
            .as_object("baseline evaluator")
            .expect("evaluator object")
            .get("events")
            .expect("baseline events");
        let reordered_events = reordered_evaluator
            .as_object("reordered evaluator")
            .expect("evaluator object")
            .get("events")
            .expect("reordered events");
        assert_eq!(baseline_events, reordered_events);
        assert_ne!(baseline.digest_sha256, reordered_report.digest_sha256);
    }

    #[test]
    fn changed_visible_payload_changes_policy_information() {
        let baseline = valid_report(5);
        let changed = FIXTURE.replace("\"vehicle_id\": \"veh-02\"", "\"vehicle_id\": \"veh-03\"");
        let mutated = validate_bytes(changed.as_bytes(), None, 5).expect("valid mutation");
        assert_ne!(baseline.policy_view_json, mutated.policy_view_json);
    }

    #[test]
    fn duplicate_keys_nested_in_event_payload_are_rejected() {
        let changed = FIXTURE.replace(
            "\"status\": \"unavailable\",",
            "\"status\": \"unavailable\", \"status\": \"available\",",
        );
        assert!(
            validate_bytes(changed.as_bytes(), None, 5)
                .expect_err("duplicate nested key")
                .contains("duplicate object key")
        );
    }

    #[test]
    fn unicode_escapes_and_surrogate_pairs_round_trip() {
        let parsed =
            parse_json(r#"{"rocket":"\ud83d\ude80","line":"a\nb"}"#).expect("valid Unicode JSON");
        let serialized = parsed.stringify();
        assert!(serialized.contains("🚀"));
        assert_eq!(parse_json(&serialized).expect("serialized JSON"), parsed);
    }

    #[test]
    fn oversized_ticks_and_malformed_digests_are_rejected() {
        let oversized = FIXTURE.replace(
            "\"effective_tick\": 4",
            "\"effective_tick\": 18446744073709551616",
        );
        assert!(
            validate_bytes(oversized.as_bytes(), None, 5)
                .expect_err("overflowing tick")
                .contains("non-negative integer")
        );

        assert!(
            validate_bytes(FIXTURE.as_bytes(), Some(&"z".repeat(64)), 5)
                .expect_err("non-hex digest")
                .contains("64 hexadecimal")
        );
    }
}
