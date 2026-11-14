from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one exact anchor, found {count}")
    return text.replace(old, new, 1)


path = Path("crates/domains/symtropy-fabrication/src/capability.rs")
text = path.read_text()

text = replace_once(
    text,
    "/// Exact canonical F5 requirement semantics captured by executable authority.\n",
    "pub const CAPABILITY_NEED_SNAPSHOT_SCHEMA_VERSION: u32 = 1;\n\n"
    "/// Exact canonical F5 requirement semantics captured by executable authority.\n",
    "snapshot schema constant",
)

text = replace_once(
    text,
    "pub struct CapabilityNeedSnapshot {\n    id: CapabilityNeedId,\n",
    "pub struct CapabilityNeedSnapshot {\n    schema_version: u32,\n    id: CapabilityNeedId,\n",
    "snapshot schema field",
)

text = replace_once(
    text,
    "struct CapabilityNeedSnapshotWire {\n    id: CapabilityNeedId,\n",
    "struct CapabilityNeedSnapshotWire {\n    schema_version: u32,\n    id: CapabilityNeedId,\n",
    "snapshot wire schema field",
)

text = replace_once(
    text,
    "        let value = Self {\n            id: wire.id,\n",
    "        let value = Self {\n            schema_version: wire.schema_version,\n            id: wire.id,\n",
    "snapshot wire schema restore",
)

text = replace_once(
    text,
    "        Self {\n            id: need.id,\n",
    "        Self {\n            schema_version: CAPABILITY_NEED_SNAPSHOT_SCHEMA_VERSION,\n            id: need.id,\n",
    "snapshot constructor schema",
)

text = replace_once(
    text,
    "    pub fn id(&self) -> &CapabilityNeedId {\n",
    "    pub const fn schema_version(&self) -> u32 {\n"
    "        self.schema_version\n"
    "    }\n\n"
    "    pub fn id(&self) -> &CapabilityNeedId {\n",
    "snapshot schema accessor",
)

text = replace_once(
    text,
    "    pub fn validate_canonical(&self) -> Result<(), CapabilityError> {\n        for axis in &self.axes {\n",
    "    pub fn validate_canonical(&self) -> Result<(), CapabilityError> {\n"
    "        if self.schema_version != CAPABILITY_NEED_SNAPSHOT_SCHEMA_VERSION {\n"
    "            return Err(CapabilityError::UnsupportedNeedSnapshotSchemaVersion(\n"
    "                self.schema_version,\n"
    "            ));\n"
    "        }\n"
    "        for axis in &self.axes {\n",
    "snapshot schema validation",
)

text = replace_once(
    text,
    "    InvalidAxisRange {\n",
    "    UnsupportedNeedSnapshotSchemaVersion(u32),\n"
    "    InvalidAxisRange {\n",
    "schema error variant",
)

text = replace_once(
    text,
    "            Self::InvalidAxisRange {\n",
    "            Self::UnsupportedNeedSnapshotSchemaVersion(version) => write!(\n"
    "                formatter,\n"
    "                \"unsupported exact capability-need snapshot schema version {version}\"\n"
    "            ),\n"
    "            Self::InvalidAxisRange {\n",
    "schema error display",
)

test_anchor = """    #[test]\n    fn exact_need_snapshot_rejects_noncanonical_wire_order() {\n"""
tests = r'''    #[test]
    fn exact_need_snapshot_binds_schema_version_and_rejects_unknown_wire_version() {
        let snapshot = welding_need().snapshot().unwrap();
        assert_eq!(
            snapshot.schema_version(),
            CAPABILITY_NEED_SNAPSHOT_SCHEMA_VERSION
        );

        let mut value = serde_json::to_value(&snapshot).unwrap();
        value["schema_version"] = 2.into();
        let restored = serde_json::from_value::<CapabilityNeedSnapshot>(value);
        assert!(matches!(
            restored.unwrap_err().to_string().as_str(),
            message if message.contains("unsupported exact capability-need snapshot schema version 2")
        ));
    }

'''
text = replace_once(text, test_anchor, tests + test_anchor, "schema regression")

path.write_text(text)
