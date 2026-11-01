//! Strict raw JSON ingress for JCS consumers.
//!
//! Reject duplicate object member names and values outside the declared
//! interoperable numeric profile before canonicalization can lose evidence.

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use std::fmt;

pub const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

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
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
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

fn validate_plain_integer_lexemes(input: &[u8]) -> Result<(), String> {
    let mut index = 0usize;
    let mut in_string = false;
    while index < input.len() {
        let byte = input[index];
        if in_string {
            match byte {
                b'\\' => {
                    index = (index + 2).min(input.len());
                }
                b'"' => {
                    in_string = false;
                    index += 1;
                }
                _ => index += 1,
            }
            continue;
        }
        if byte == b'"' {
            in_string = true;
            index += 1;
            continue;
        }
        if byte == b'-' || byte.is_ascii_digit() {
            let start = index;
            index += 1;
            while index < input.len()
                && (input[index].is_ascii_digit()
                    || matches!(input[index], b'.' | b'e' | b'E' | b'+' | b'-'))
            {
                index += 1;
            }
            let token = &input[start..index];
            if !token.iter().any(|value| matches!(value, b'.' | b'e' | b'E')) {
                let lexeme = std::str::from_utf8(token)
                    .map_err(|_| "JSON integer token is not UTF-8".to_owned())?;
                let integer = lexeme.parse::<i128>().map_err(|_| {
                    format!("plain JSON integer {lexeme:?} exceeds the supported lexical range")
                })?;
                let safe_limit = i128::from(MAX_SAFE_INTEGER);
                if integer > safe_limit || integer < -safe_limit {
                    return Err(format!(
                        "plain JSON integer {lexeme:?} is outside the interoperable safe-integer range"
                    ));
                }
            }
            continue;
        }
        index += 1;
    }
    Ok(())
}

/// Parse exactly one raw JSON value. Duplicate member names are detected after
/// escape decoding, at every nesting depth, before a map can overwrite them.
pub fn parse_strict_jcs_input(input: &[u8]) -> Result<Value, String> {
    validate_plain_integer_lexemes(input)?;
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
        assert!(parse_strict_jcs_input(b"{} {}").is_err());
        assert!(parse_strict_jcs_input(b"9007199254740992").is_err());
        assert!(parse_strict_jcs_input(b"-9007199254740992").is_err());
        assert!(parse_strict_jcs_input(b"18446744073709551616").is_err());
        assert!(parse_strict_jcs_input(b"-9223372036854775809").is_err());
        assert!(parse_strict_jcs_input(b"-170141183460469231731687303715884105728").is_err());
        assert!(parse_strict_jcs_input(b"999999999999999999999999999999999999999999").is_err());
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
          "safe_integer":9007199254740991
        }"#.as_bytes();
        let value = parse_strict_jcs_input(input).unwrap();
        assert_eq!(value["composed"], "é");
        assert_ne!(value["composed"], value["decomposed"]);
        assert!(value["symbol"].as_str().is_some());
        assert!(canonicalize_strict_jcs_input(input).is_ok());
    }

    #[test]
    fn rust_jcs_matches_adversarial_golden_through_strict_raw_ingress() {
        let input = include_bytes!("../interop/golden/adversarial-input.json");
        let expected = include_bytes!("../interop/golden/adversarial-output.jcs");
        let actual = canonicalize_strict_jcs_input(input).unwrap();
        assert_eq!(actual, std::str::from_utf8(expected).unwrap().trim_end().as_bytes());
    }
}
