#!/usr/bin/env python3
"""Adapt the qualified Rust 1.96-formatted D1 source to the frozen digest transform's text anchors.

This is intentionally a textual compatibility adapter, not a semantic transform.
The authority workflow proves reversibility by applying this adapter, running the
pinned formatter, and requiring the exact original source blob before it is used
again immediately before the frozen canonical-digest transform.
"""
from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one exact anchor, found {count}")
    return text.replace(old, new, 1)


path = Path("crates/domains/symtropy-design/src/lib.rs")
text = path.read_text()

formatted_validate = '''fn validate_stable_id(id: &StableId) -> Result<(), DesignError> {
    StableId::parse(id.as_str())
        .map(|_| ())
        .map_err(|_| DesignError::InvalidStableId(id.as_str().to_string()))
}
'''
legacy_validate = '''fn validate_stable_id(id: &StableId) -> Result<(), DesignError> {
    StableId::parse(id.as_str()).map(|_| ()).map_err(|_| {
        DesignError::InvalidStableId(id.as_str().to_string())
    })
}
'''
text = replace_once(text, formatted_validate, legacy_validate, "validate_stable_id formatting")

formatted_display = '''            Self::NonCanonicalOrder(field) => {
                write!(
                    formatter,
                    "design manifest field {field} is not canonically ordered"
                )
            }
            Self::Serialization(error) => write!(formatter, "design serialization failed: {error}"),
'''
legacy_display = '''            Self::NonCanonicalOrder(field) => {
                write!(formatter, "design manifest field {field} is not canonically ordered")
            }
            Self::Serialization(error) => write!(formatter, "design serialization failed: {error}"),
'''
text = replace_once(text, formatted_display, legacy_display, "DesignError display formatting")

path.write_text(text)
