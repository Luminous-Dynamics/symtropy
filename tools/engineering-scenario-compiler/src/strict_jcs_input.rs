//! Strict raw JSON ingress for JCS consumers.
//!
//! Reject duplicate object member names and values outside the declared
//! interoperable numeric profile before canonicalization can lose evidence.

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use std::fmt;

pub const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
pub const MAX_STRICT_JCS_INPUT_BYTES: usize = 1_048_576;
pub const MAX_STRICT_JCS_NESTING_DEPTH: usize = 127;


/// I-JSON excludes Unicode noncharacters from member names and string values.
fn contains_unicode_noncharacter(value: &str) -> bool {
    value.chars().any(|character| {
        let codepoint = character as u32;
        (0xFDD0..=0xFDEF).contains(&codepoint) || codepoint & 0xFFFE == 0xFFFE
    })
}

/// Inspect raw JSON number tokens before serde_json converts oversized integer
/// lexemes to f64. The Go verifier applies the same safe-integer token profile.
fn validate_safe_integer_literals(input: &[u8]) -> Result<(), String> {
    const MAX_SAFE_INTEGER_TEXT: &[u8] = b"9007199254740991";
    let mut index = 0;

    while index < input.len() {
        match input[index] {
            b'"' => {
                // Skip strings, honoring escaped quotes and backslashes. JSON
                // syntax and escape validity are checked by serde_json below.
                index += 1;
                while index < input.len() {
                    match input[index] {
                        b'\\' => index = (index + 2).min(input.len()),
                        b'"' => {
                            index += 1;
                            break;
                        }
                        _ => index += 1,
                    }
                }
            }
            b'-' | b'0'..=b'9' => {
                let start = index;
                index += 1;
                while index < input.len()
                    && matches!(
                        input[index],
                        b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-'
                    )
                {
                    index += 1;
                }

                let token = &input[start..index];
                let is_decimal_or_exponent =
                    token.iter().any(|byte| matches!(*byte, b'.' | b'e' | b'E'));
                if !is_decimal_or_exponent {
                    let digits = if token.first() == Some(&b'-') {
                        &token[1..]
                    } else {
                        token
                    };
                    let exceeds_safe_integer = digits.len() > MAX_SAFE_INTEGER_TEXT.len()
                        || (digits.len() == MAX_SAFE_INTEGER_TEXT.len()
                            && digits > MAX_SAFE_INTEGER_TEXT);
                    if exceeds_safe_integer {
                        return Err(format!(
                            "integer literal {} is outside the declared interoperable safe-integer range",
                            String::from_utf8_lossy(token)
                        ));
                    }
                }
            }
            _ => index += 1,
        }
    }
    Ok(())
}

struct StrictJcsSeed;

impl<'de> DeserializeSeed<'de> for StrictJcsSeed {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictJcsValueVisitor)
    }
}

struct StrictJcsValueVisitor;

impl<'de> Visitor<'de> for StrictJcsValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an unambiguous I-JSON value suitable for RFC 8785")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if i128::from(value).abs() > i128::from(MAX_SAFE_INTEGER) {
            return Err(E::custom("integer is outside the interoperable safe-integer range"));
        }
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if value > MAX_SAFE_INTEGER {
            return Err(E::custom("integer is outside the interoperable safe-integer range"));
        }
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("number must be finite IEEE-754 binary64"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if contains_unicode_noncharacter(value) {
            return Err(E::custom("Unicode noncharacters are not permitted in I-JSON strings"));
        }
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if contains_unicode_noncharacter(&value) {
            return Err(E::custom("Unicode noncharacters are not permitted in I-JSON strings"));
        }
        Ok(Value::String(value))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Null)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Value::Null)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(StrictJcsSeed)? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        while let Some(key) = object.next_key::<String>()? {
            if contains_unicode_noncharacter(&key) {
                return Err(de::Error::custom(
                    "Unicode noncharacters are not permitted in I-JSON member names",
                ));
            }
            if values.contains_key(&key) {
                return Err(de::Error::custom(format!(
                    "duplicate JSON object member name {key:?}"
                )));
            }
            let value = object.next_value_seed(StrictJcsSeed)?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}

/// Enforce the same byte and container-depth profile as the Go verifier.
fn validate_json_nesting_depth(input: &[u8]) -> Result<(), String> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for byte in input {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match *byte {
                b'\\' => escaped = true,
                b'"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match *byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > MAX_STRICT_JCS_NESTING_DEPTH {
                    return Err(format!(
                        "JSON nesting depth exceeds {}",
                        MAX_STRICT_JCS_NESTING_DEPTH
                    ));
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

/// Parse exactly one raw JSON value. Duplicate member names are detected after
/// escape decoding, at every nesting depth, before a map can overwrite them.
pub fn parse_strict_jcs_input(input: &[u8]) -> Result<Value, String> {
    if input.len() > MAX_STRICT_JCS_INPUT_BYTES {
        return Err(format!(
            "input exceeds {}-byte JCS profile limit",
            MAX_STRICT_JCS_INPUT_BYTES
        ));
    }
    validate_json_nesting_depth(input)?;
    // Preserve integer-vs-float lexical identity before serde_json's visitor
    // can receive an oversized integer as a finite f64 fallback.
    validate_safe_integer_literals(input)?;
    let mut deserializer = serde_json::Deserializer::from_slice(input);
    let value = StrictJcsSeed
        .deserialize(&mut deserializer)
        .map_err(|error| format!("strict JCS JSON input rejected: {error}"))?;
    deserializer
        .end()
        .map_err(|error| format!("strict JCS JSON input has trailing content: {error}"))?;
    Ok(value)
}

/// Strictly parse raw input then emit the profile's RFC 8785 canonical bytes.
pub fn canonicalize_strict_jcs_input(input: &[u8]) -> Result<Vec<u8>, String> {
    let value = parse_strict_jcs_input(input)?;
    serde_jcs::to_vec(&value).map_err(|error| format!("JCS canonicalization failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_duplicate_member_names_at_all_depths() {
        for input in [
            br#"{"a":1,"a":2}"#.as_slice(),
            br#"{"outer":{"a":1,"a":2}}"#.as_slice(),
            br#"{"items":[{"a":1,"a":2}]}"#.as_slice(),
            br#"{"a":1,"\u0061":2}"#.as_slice(),
            r#"{"é":1,"\u00e9":2}"#.as_bytes(),
        ] {
            assert!(parse_strict_jcs_input(input).is_err());
        }
    }

    #[test]
    fn rejects_invalid_utf8_unpaired_surrogates_trailing_and_unsafe_integers() {
        assert!(parse_strict_jcs_input(&[0xff, b'{', b'}']).is_err());
        assert!(parse_strict_jcs_input(br#"{"bad":"\uD800"}"#).is_err());
        assert!(parse_strict_jcs_input(br#"{"bad":"\uDC00"}"#).is_err());
        assert!(parse_strict_jcs_input(br#"{"bad":"\uFDD0"}"#).is_err());
        assert!(parse_strict_jcs_input(br#"{"\uFFFF":1}"#).is_err());
        assert!(parse_strict_jcs_input(br#"{"bad":"\uD83F\uDFFE"}"#).is_err());
        let literal_noncharacter = format!(r#"{{"bad":"{}"}}"#, '\u{FDD0}');
        assert!(parse_strict_jcs_input(literal_noncharacter.as_bytes()).is_err());
        assert!(parse_strict_jcs_input(b"{} {}").is_err());
        assert!(parse_strict_jcs_input(b"9007199254740992").is_err());
        assert!(parse_strict_jcs_input(b"-9007199254740992").is_err());
        assert!(parse_strict_jcs_input(b"18446744073709551617").is_err());
        assert!(parse_strict_jcs_input(b"-9223372036854775809").is_err());
        assert!(parse_strict_jcs_input(b"1e9999").is_err());
    }

    #[test]
    fn accepts_valid_unicode_and_binary64_boundaries() {
        let input = r#"{
          "composed":"é",
          "decomposed":"e\u0301",
          "symbol":"\uD834\uDD1E",
          "negative_zero":-0,
          "subnormal":5e-324,
          "maximum":1.7976931348623157e308,
          "safe_integer":9007199254740991,
          "numeric_text":"18446744073709551617"
        }"#;
        let value = parse_strict_jcs_input(input.as_bytes()).unwrap();
        assert_eq!(value["composed"], "é");
        assert_eq!(value["numeric_text"], "18446744073709551617");
        assert_ne!(value["composed"], value["decomposed"]);
        assert!(value["symbol"].as_str().is_some());
        assert!(canonicalize_strict_jcs_input(input).is_ok());
    }

    #[test]
    fn enforces_input_byte_and_nesting_depth_limits() {
        let oversized = vec![b' '; MAX_STRICT_JCS_INPUT_BYTES + 1];
        assert!(parse_strict_jcs_input(&oversized).is_err());

        let mut at_size_limit = Vec::with_capacity(MAX_STRICT_JCS_INPUT_BYTES);
        at_size_limit.push(b'0');
        at_size_limit.extend(std::iter::repeat(b' ').take(MAX_STRICT_JCS_INPUT_BYTES - 1));
        assert!(parse_strict_jcs_input(&at_size_limit).is_ok());

        let depth = MAX_STRICT_JCS_NESTING_DEPTH + 1;
        let too_deep = format!("{}0{}", "[".repeat(depth), "]".repeat(depth));
        assert!(parse_strict_jcs_input(too_deep.as_bytes()).is_err());

        let at_limit = format!(
            "{}0{}",
            "[".repeat(MAX_STRICT_JCS_NESTING_DEPTH),
            "]".repeat(MAX_STRICT_JCS_NESTING_DEPTH)
        );
        assert!(parse_strict_jcs_input(at_limit.as_bytes()).is_ok());
    }

    #[test]
    fn rust_jcs_matches_adversarial_golden_through_strict_raw_ingress() {
        let input = include_bytes!("../interop/golden/adversarial-input.json");
        let expected = include_bytes!("../interop/golden/adversarial-output.jcs");
        let actual = canonicalize_strict_jcs_input(input).unwrap();
        assert_eq!(actual, std::str::from_utf8(expected).unwrap().trim_end().as_bytes());
    }
}
