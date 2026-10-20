use serde_json::{Value, json};
use std::collections::BTreeMap;

pub type Check<T> = Result<T, String>;

pub fn tags<'a>(xml: &'a str, element: &str) -> Check<Vec<&'a str>> {
    let needle = format!("<{element} ");
    let mut cursor = 0usize;
    let mut out = Vec::new();
    while let Some(offset) = xml.get(cursor..).and_then(|tail| tail.find(&needle)) {
        let start = cursor + offset;
        let end = xml.get(start..).and_then(|tail| tail.find('>'))
            .map(|n| start + n)
            .ok_or_else(|| format!("unterminated {element} tag"))?;
        out.push(&xml[start..=end]);
        cursor = end + 1;
    }
    Ok(out)
}

pub fn attrs(tag: &str) -> Check<BTreeMap<String, String>> {
    let inner = tag
        .strip_prefix('<')
        .and_then(|s| s.strip_suffix('>'))
        .ok_or_else(|| "malformed XML tag".to_owned())?
        .trim_end_matches('/')
        .trim();
    let mut cursor = inner.find(char::is_whitespace).unwrap_or(inner.len());
    let bytes = inner.as_bytes();
    let mut out = BTreeMap::new();
    while cursor < bytes.len() {
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor == bytes.len() {
            break;
        }
        let start = cursor;
        while cursor < bytes.len() && !bytes[cursor].is_ascii_whitespace() && bytes[cursor] != b'='
        {
            cursor += 1;
        }
        if cursor == start {
            return Err("malformed attribute name".into());
        }
        let name = inner[start..cursor].to_owned();
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b'=' {
            return Err(format!("attribute {name} lacks equals sign"));
        }
        cursor += 1;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b'"' {
            return Err(format!("attribute {name} is not double-quoted"));
        }
        cursor += 1;
        let value_start = cursor;
        while cursor < bytes.len() && bytes[cursor] != b'"' {
            cursor += 1;
        }
        if cursor == bytes.len() {
            return Err(format!("attribute {name} is unterminated"));
        }
        let value = inner[value_start..cursor].to_owned();
        cursor += 1;
        if out.insert(name.clone(), value).is_some() {
            return Err(format!("duplicate attribute {name}"));
        }
    }
    Ok(out)
}

pub fn number(attrs: &BTreeMap<String, String>, key: &str) -> Check<Option<f64>> {
    match attrs.get(key) {
        None => Ok(None),
        Some(value) => {
            let parsed = value
                .parse::<f64>()
                .map_err(|_| format!("attribute {key} is not numeric"))?;
            if !parsed.is_finite() {
                return Err(format!("attribute {key} is not finite"));
            }
            Ok(Some(parsed))
        }
    }
}

pub fn count(attrs: &BTreeMap<String, String>, key: &str) -> Check<u64> {
    attrs
        .get(key)
        .ok_or_else(|| format!("summary lacks {key}"))?
        .parse::<u64>()
        .map_err(|_| format!("summary {key} is not a non-negative integer"))
}

pub fn distribution(values: &[f64]) -> Value {
    fn percentile(values: &[f64], p: f64) -> Option<f64> {
        if values.is_empty() {
            return None;
        }
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let rank = ((p * sorted.len() as f64).ceil() as usize)
            .saturating_sub(1)
            .min(sorted.len() - 1);
        Some(sorted[rank])
    }
    let mean = (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64);
    json!({
        "sample_count": values.len(),
        "mean": mean,
        "p50": percentile(values, 0.5),
        "p90": percentile(values, 0.9),
        "p95": percentile(values, 0.95)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_self_closing_tags_and_nearest_rank_percentiles() {
        let parsed = attrs(r#"<tripinfo id="veh0" arrival="-1.00"/>"#).unwrap();
        assert_eq!(parsed["id"], "veh0");
        assert_eq!(number(&parsed, "arrival").unwrap(), Some(-1.0));
        assert_eq!(distribution(&[20.0, 12.0])["p50"], 12.0);
        assert_eq!(distribution(&[20.0, 12.0])["p95"], 20.0);
        assert_eq!(distribution(&[])["p95"], Value::Null);
    }

    #[test]
    fn does_not_confuse_tripinfos_root_with_tripinfo_element() {
        assert_eq!(
            tags(r#"<tripinfos><tripinfo id="v0"/></tripinfos>"#, "tripinfo")
                .unwrap()
                .len(),
            1
        );
    }
}
