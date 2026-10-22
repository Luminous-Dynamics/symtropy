from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one exact anchor, found {count}")
    return text.replace(old, new, 1)


path = Path("crates/domains/symtropy-design/src/lib.rs")
text = path.read_text()

text = replace_once(
    text,
    'const DESIGN_REVISION_DIGEST_DOMAIN: &[u8] = b"symtropy.design.revision.v1\\0";\n',
    'pub const DESIGN_REVISION_SCHEMA_VERSION: u32 = 1;\n'
    'const DESIGN_REVISION_DIGEST_DOMAIN: &[u8] = b"symtropy.design.revision.v1\\0";\n',
    "digest schema constant",
)

text = replace_once(
    text,
    "pub struct DesignRevisionManifest {\n    pub design_id: DesignId,\n",
    "pub struct DesignRevisionManifest {\n    pub schema_version: u32,\n    pub design_id: DesignId,\n",
    "manifest schema field",
)

text = replace_once(
    text,
    "        let manifest = Self {\n            design_id,\n",
    "        let manifest = Self {\n            schema_version: DESIGN_REVISION_SCHEMA_VERSION,\n            design_id,\n",
    "manifest constructor schema",
)

text = replace_once(
    text,
    "    fn validate_canonical(&self) -> Result<(), DesignError> {\n        validate_stable_id(self.design_id.stable_id())?;\n",
    "    fn validate_canonical(&self) -> Result<(), DesignError> {\n"
    "        if self.schema_version != DESIGN_REVISION_SCHEMA_VERSION {\n"
    "            return Err(DesignError::UnsupportedSchemaVersion(self.schema_version));\n"
    "        }\n"
    "        validate_stable_id(self.design_id.stable_id())?;\n",
    "manifest schema validation",
)

old_digest = '''    fn content_digest(&self) -> Result<ContentDigest, DesignError> {
        self.validate_canonical()?;
        let canonical = serde_json::to_vec(self).map_err(DesignError::Serialization)?;
        let mut bytes = Vec::with_capacity(DESIGN_REVISION_DIGEST_DOMAIN.len() + canonical.len());
        bytes.extend_from_slice(DESIGN_REVISION_DIGEST_DOMAIN);
        bytes.extend_from_slice(&canonical);
        Ok(ContentDigest::sha256(&bytes))
    }
'''
new_digest = '''    fn content_digest(&self) -> Result<ContentDigest, DesignError> {
        Ok(ContentDigest::sha256(&self.canonical_preimage()?))
    }

    /// Stable digest protocol for design revision identity.
    ///
    /// This preimage is deliberately independent of Serde/JSON representation:
    /// domain tag + schema version + fixed-width integers + length-prefixed
    /// strings/collections in constructor-canonical order.
    fn canonical_preimage(&self) -> Result<Vec<u8>, DesignError> {
        self.validate_canonical()?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(DESIGN_REVISION_DIGEST_DOMAIN);
        bytes.extend_from_slice(&self.schema_version.to_le_bytes());
        encode_stable_id(&mut bytes, self.design_id.stable_id())?;
        bytes.extend_from_slice(&self.revision.to_le_bytes());

        encode_len(&mut bytes, self.parents.len())?;
        for parent in &self.parents {
            encode_stable_id(&mut bytes, parent.design_id.stable_id())?;
            bytes.extend_from_slice(&parent.revision.to_le_bytes());
            encode_digest(&mut bytes, &parent.content_digest)?;
        }

        encode_len(&mut bytes, self.artifacts.len())?;
        for artifact in &self.artifacts {
            encode_stable_id(&mut bytes, artifact.id.stable_id())?;
            encode_stable_id(&mut bytes, &artifact.role_id)?;
            encode_digest(&mut bytes, &artifact.content_digest)?;
        }

        encode_len(&mut bytes, self.semantic_refs.len())?;
        for semantic in &self.semantic_refs {
            encode_stable_id(&mut bytes, &semantic.kind_id)?;
            encode_stable_id(&mut bytes, &semantic.subject_id)?;
            bytes.extend_from_slice(&semantic.revision.to_le_bytes());
            encode_digest(&mut bytes, &semantic.content_digest)?;
        }

        Ok(bytes)
    }
'''
text = replace_once(text, old_digest, new_digest, "canonical digest implementation")

helper_anchor = '''fn validate_stable_id(id: &StableId) -> Result<(), DesignError> {
    StableId::parse(id.as_str()).map(|_| ()).map_err(|_| {
        DesignError::InvalidStableId(id.as_str().to_string())
    })
}

fn hex(bytes: &[u8]) -> String {
'''
helper_replacement = '''fn encode_digest(bytes: &mut Vec<u8>, digest: &ContentDigest) -> Result<(), DesignError> {
    digest.validate()?;
    encode_stable_id(bytes, &digest.algorithm)?;
    encode_string(bytes, &digest.value)
}

fn encode_stable_id(bytes: &mut Vec<u8>, id: &StableId) -> Result<(), DesignError> {
    validate_stable_id(id)?;
    encode_string(bytes, id.as_str())
}

fn encode_string(bytes: &mut Vec<u8>, value: &str) -> Result<(), DesignError> {
    encode_len(bytes, value.len())?;
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn encode_len(bytes: &mut Vec<u8>, len: usize) -> Result<(), DesignError> {
    let len = u64::try_from(len).map_err(|_| DesignError::LengthOverflow)?;
    bytes.extend_from_slice(&len.to_le_bytes());
    Ok(())
}

fn validate_stable_id(id: &StableId) -> Result<(), DesignError> {
    StableId::parse(id.as_str())
        .map(|_| ())
        .map_err(|_| DesignError::InvalidStableId(id.as_str().to_string()))
}

fn hex(bytes: &[u8]) -> String {
'''
text = replace_once(text, helper_anchor, helper_replacement, "canonical encoding helpers")

text = replace_once(
    text,
    "pub enum DesignError {\n    InvalidStableId(String),\n    InvalidDigestValue(String),\n",
    "pub enum DesignError {\n"
    "    InvalidStableId(String),\n"
    "    InvalidDigestValue(String),\n"
    "    UnsupportedSchemaVersion(u32),\n"
    "    LengthOverflow,\n",
    "digest protocol errors",
)

text = replace_once(
    text,
    "    NonCanonicalOrder(&'static str),\n    Serialization(serde_json::Error),\n    DigestMismatch {\n",
    "    NonCanonicalOrder(&'static str),\n    DigestMismatch {\n",
    "remove serde identity error",
)

text = replace_once(
    text,
    '''            Self::ContentRequired => write!(
                formatter,
                "design revision must contain at least one artifact or semantic reference"
            ),
''',
    '''            Self::UnsupportedSchemaVersion(version) => {
                write!(formatter, "unsupported design revision schema version {version}")
            }
            Self::LengthOverflow => write!(formatter, "canonical design length exceeds u64"),
            Self::ContentRequired => write!(
                formatter,
                "design revision must contain at least one artifact or semantic reference"
            ),
''',
    "digest protocol display",
)

text = replace_once(
    text,
    '''            Self::NonCanonicalOrder(field) => {
                write!(formatter, "design manifest field {field} is not canonically ordered")
            }
            Self::Serialization(error) => write!(formatter, "design serialization failed: {error}"),
            Self::DigestMismatch { expected, actual } => write!(
''',
    '''            Self::NonCanonicalOrder(field) => {
                write!(
                    formatter,
                    "design manifest field {field} is not canonically ordered"
                )
            }
            Self::DigestMismatch { expected, actual } => write!(
''',
    "remove serde display and format",
)

old_source = '''impl Error for DesignError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Serialization(error) => Some(error),
            _ => None,
        }
    }
}
'''
text = replace_once(text, old_source, "impl Error for DesignError {}\n", "error source cleanup")

# Add protocol regressions immediately before the existing external-digest test.
test_anchor = '''    #[test]
    fn invalid_external_digest_is_rejected() {
'''
tests = '''    #[test]
    fn canonical_digest_protocol_has_frozen_golden_vector() {
        let revision = revision_with_order(vec![artifact(
            "artifact:a",
            "design-artifact:geometry",
            "aaaa",
        )]);

        assert_eq!(revision.manifest().schema_version, DESIGN_REVISION_SCHEMA_VERSION);
        assert_eq!(revision.content_digest().algorithm, id("sha256"));
        assert_eq!(
            revision.content_digest().value,
            "3754b633817fd02c8d731793a8f5b76debd8e3c0791f01f77782c1f216ff4045"
        );
    }

    #[test]
    fn schema_version_is_digest_semantics_and_wire_tampering_fails_closed() {
        let revision = revision_with_order(vec![artifact(
            "artifact:a",
            "design-artifact:geometry",
            "aaaa",
        )]);
        let mut value = serde_json::to_value(&revision).unwrap();
        value["manifest"]["schema_version"] = 2.into();
        let decoded = serde_json::from_value::<DesignRevision>(value);
        assert!(matches!(
            decoded.unwrap_err().to_string().as_str(),
            message if message.contains("unsupported design revision schema version 2")
        ));
    }

'''
text = replace_once(text, test_anchor, tests + test_anchor, "canonical digest tests")

path.write_text(text)
