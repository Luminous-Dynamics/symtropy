#!/usr/bin/env python3
from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one exact anchor, found {count}")
    return text.replace(old, new, 1)


path = Path("crates/domains/symtropy-analysis-admission/src/lib.rs")
text = path.read_text()

text = replace_once(
    text,
    "pub const VERIFICATION_REGISTRY_SCHEMA_VERSION: u32 = 1;\npub const ADMISSION_RECEIPT_SCHEMA_VERSION: u32 = 1;",
    "pub const VERIFICATION_REGISTRY_SCHEMA_VERSION: u32 = 1;\n"
    "pub const VERIFICATION_RECORD_SCHEMA_VERSION: u32 = 1;\n"
    "pub const ADMISSION_RECEIPT_SCHEMA_VERSION: u32 = 1;",
    "record schema version",
)
text = replace_once(
    text,
    'const REGISTRY_DIGEST_DOMAIN: &[u8] = b"symtropy.analysis.verification-registry.v1\\0";\n',
    'const REGISTRY_DIGEST_DOMAIN: &[u8] = b"symtropy.analysis.verification-registry.v1\\0";\n'
    'const RECORD_DIGEST_DOMAIN: &[u8] = b"symtropy.analysis.verification-record.v1\\0";\n',
    "record digest domain",
)

old_impl_tail = '''        self.verification_receipt\n            .validate()\n            .map_err(AdmissionError::Analysis)\n    }\n}'''
new_impl_tail = '''        self.verification_receipt\n            .validate()\n            .map_err(AdmissionError::Analysis)\n    }\n\n    /// Return the standalone content digest of this exact verification record.\n    ///\n    /// The record body bytes are encoded by the same function used inside the\n    /// verification-registry preimage; only this standalone digest prepends its\n    /// own record domain and schema version.\n    pub fn content_digest(&self) -> Result<ContentDigest, AdmissionError> {\n        self.validate()?;\n        let mut bytes = Vec::new();\n        bytes.extend_from_slice(RECORD_DIGEST_DOMAIN);\n        bytes.extend_from_slice(&VERIFICATION_RECORD_SCHEMA_VERSION.to_le_bytes());\n        encode_verification_record(&mut bytes, self)?;\n        sha256_digest(&bytes)\n    }\n\n    /// Return the exact standalone reference used by external authentication.\n    pub fn exact_ref(&self) -> Result<VerificationRecordRef, AdmissionError> {\n        Ok(VerificationRecordRef {\n            id: self.id.clone(),\n            content_digest: self.content_digest()?,\n        })\n    }\n}\n\n/// Content-bound reference to one exact verification record.\n#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]\npub struct VerificationRecordRef {\n    pub id: VerificationRecordId,\n    pub content_digest: ContentDigest,\n}'''
text = replace_once(text, old_impl_tail, new_impl_tail, "record identity methods")

old_registry_loop = '''        for record in &self.records {\n            encode_stable_id(&mut bytes, record.id.stable_id());\n            encode_stable_id(&mut bytes, &record.facet_id);\n            encode_digest(&mut bytes, &record.qualification_cut_digest)?;\n            encode_evidence_ref(&mut bytes, &record.subject_evidence)?;\n            encode_exact_ref(&mut bytes, &record.assessment_attestation)?;\n            encode_exact_ref(&mut bytes, &record.issuer_profile)?;\n            encode_exact_ref(&mut bytes, &record.verifier_profile)?;\n            encode_exact_ref(&mut bytes, &record.verification_receipt)?;\n            bytes.push(match record.status {\n                VerificationStatus::Verified => 0,\n                VerificationStatus::Revoked => 1,\n                VerificationStatus::Superseded => 2,\n            });\n        }'''
new_registry_loop = '''        for record in &self.records {\n            encode_verification_record(&mut bytes, record)?;\n        }'''
text = replace_once(text, old_registry_loop, new_registry_loop, "shared registry record encoding")

helper_anchor = '''fn compare_record_key(\n    left: &FacetVerificationInput,\n    right: &FacetVerificationInput,\n) -> std::cmp::Ordering {'''
helper = '''fn encode_verification_record(\n    bytes: &mut Vec<u8>,\n    record: &FacetVerificationInput,\n) -> Result<(), AdmissionError> {\n    record.validate()?;\n    encode_stable_id(bytes, record.id.stable_id());\n    encode_stable_id(bytes, &record.facet_id);\n    encode_digest(bytes, &record.qualification_cut_digest)?;\n    encode_evidence_ref(bytes, &record.subject_evidence)?;\n    encode_exact_ref(bytes, &record.assessment_attestation)?;\n    encode_exact_ref(bytes, &record.issuer_profile)?;\n    encode_exact_ref(bytes, &record.verifier_profile)?;\n    encode_exact_ref(bytes, &record.verification_receipt)?;\n    bytes.push(match record.status {\n        VerificationStatus::Verified => 0,\n        VerificationStatus::Revoked => 1,\n        VerificationStatus::Superseded => 2,\n    });\n    Ok(())\n}\n\n'''
if helper_anchor not in text:
    raise SystemExit("record encoder insertion anchor missing")
text = text.replace(helper_anchor, helper + helper_anchor, 1)

# Add proof tests immediately before the test module's closing brace.
marker = "#[cfg(test)]\nmod tests {"
head, tests = text.split(marker, 1)
body, closing = tests.rsplit("\n}", 1)
proofs = r'''

    #[test]
    fn standalone_verification_record_digest_is_frozen_and_content_bound() {
        let request = request();
        let evidence = evidence(&request, "solver-a", true);
        let profile = profile();
        let cut = cut(&request, &evidence, &profile, FacetDisposition::Established);
        let policy = admission_policy(&profile);
        let base = verification_records(
            &request,
            &evidence,
            &profile,
            &cut,
            &policy,
            VerificationStatus::Verified,
        )
        .remove(0);
        let base_digest = base.content_digest().unwrap();
        println!("A21_RECORD_GOLDEN={}", base_digest.value);

        let mut variants = Vec::new();
        let mut changed = base.clone();
        changed.id = VerificationRecordId::new(id("verification:changed"));
        variants.push(changed);

        let mut changed = base.clone();
        changed.facet_id = id("facet:changed");
        variants.push(changed);

        let mut changed = base.clone();
        changed.qualification_cut_digest = digest("different-cut");
        variants.push(changed);

        let mut changed = base.clone();
        let different_evidence = evidence(&request, "solver-b", true);
        changed.subject_evidence = different_evidence
            .rebind(&request)
            .unwrap()
            .exact_ref()
            .unwrap();
        variants.push(changed);

        let mut changed = base.clone();
        changed.assessment_attestation = exact(
            "authority:model-qualification",
            "attestation:changed",
            "changed-assessment",
        );
        variants.push(changed);

        let mut changed = base.clone();
        changed.issuer_profile = exact(
            "authority:model-qualification",
            "issuer-profile:changed",
            "changed-issuer",
        );
        variants.push(changed);

        let mut changed = base.clone();
        changed.verifier_profile = exact(
            "authority:xenia",
            "verifier-profile:changed",
            "changed-verifier",
        );
        variants.push(changed);

        let mut changed = base.clone();
        changed.verification_receipt = exact(
            "authority:xenia",
            "receipt:changed",
            "changed-receipt",
        );
        variants.push(changed);

        let mut changed = base.clone();
        changed.status = VerificationStatus::Revoked;
        variants.push(changed);

        for variant in variants {
            assert_ne!(variant.content_digest().unwrap(), base_digest);
        }
        assert_eq!(base.exact_ref().unwrap().content_digest, base_digest);
    }

    #[test]
    fn registry_embeds_the_exact_standalone_record_body_encoding() {
        let request = request();
        let evidence = evidence(&request, "solver-a", true);
        let profile = profile();
        let cut = cut(&request, &evidence, &profile, FacetDisposition::Established);
        let policy = admission_policy(&profile);
        let record = verification_records(
            &request,
            &evidence,
            &profile,
            &cut,
            &policy,
            VerificationStatus::Verified,
        )
        .remove(0);

        let mut shared = Vec::new();
        encode_verification_record(&mut shared, &record).unwrap();

        let mut legacy = Vec::new();
        encode_stable_id(&mut legacy, record.id.stable_id());
        encode_stable_id(&mut legacy, &record.facet_id);
        encode_digest(&mut legacy, &record.qualification_cut_digest).unwrap();
        encode_evidence_ref(&mut legacy, &record.subject_evidence).unwrap();
        encode_exact_ref(&mut legacy, &record.assessment_attestation).unwrap();
        encode_exact_ref(&mut legacy, &record.issuer_profile).unwrap();
        encode_exact_ref(&mut legacy, &record.verifier_profile).unwrap();
        encode_exact_ref(&mut legacy, &record.verification_receipt).unwrap();
        legacy.push(match record.status {
            VerificationStatus::Verified => 0,
            VerificationStatus::Revoked => 1,
            VerificationStatus::Superseded => 2,
        });

        assert_eq!(shared, legacy);
    }
'''
text = head + marker + body + proofs + "\n}" + closing
path.write_text(text)
