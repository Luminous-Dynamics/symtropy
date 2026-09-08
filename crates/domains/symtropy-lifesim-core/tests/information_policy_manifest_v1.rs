// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_lifesim_core::information::{
    CapabilityEvidence, EcologicalAuthorityLevel, EcologicalInformation,
    ProcessInformationProfile, ProcessInformationRequirement, ProcessKey,
    RepresentationCapabilities, RepresentationKey,
};
use symtropy_lifesim_core::information_policy_manifest::{
    InformationPolicyManifest, INFORMATION_POLICY_MANIFEST_VERSION,
};
use symtropy_lifesim_core::information_registry::{
    InformationPolicyRegistryBuilder, InformationPolicyRegistryKey,
};

const REGISTRY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(70, 3);
const PROCESS: ProcessKey = ProcessKey::new(1, 1);
const REPRESENTATION: RepresentationKey = RepresentationKey::new(10, 1);

const EXPECTED_V1_HEX: &str = concat!(
    "53594d54524f50595f494e464f524d4154494f4e5f504f4c4943595f4d414e494645535400",
    "01000000",
    "46000000000000000000000000000000",
    "03000000",
    "0100000000000000",
    "01000000000000000000000000000000",
    "01000000",
    "01",
    "0100000000000000",
    "02",
    "00",
    "0100000000000000",
    "0a000000000000000000000000000000",
    "01000000",
    "01",
    "0100000000000000",
    "02",
    "0100000000000000",
    "00",
    "0000000000000000",
);

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(*byte >> 4)]));
        output.push(char::from(HEX[usize::from(*byte & 0x0f)]));
    }
    output
}

fn minimal_registry() -> symtropy_lifesim_core::information_registry::InformationPolicyRegistry {
    let mut builder = InformationPolicyRegistryBuilder::new(REGISTRY);
    builder
        .register_process(ProcessInformationProfile::new(
            PROCESS,
            EcologicalAuthorityLevel::Coarse,
            [ProcessInformationRequirement::exact(
                EcologicalInformation::AgeDistribution,
            )],
        ))
        .unwrap();
    builder
        .register_representation(RepresentationCapabilities::new(
            REPRESENTATION,
            EcologicalAuthorityLevel::Coarse,
            [(
                EcologicalInformation::AgeDistribution,
                CapabilityEvidence::Exact,
            )],
        ))
        .unwrap();
    builder.seal()
}

#[test]
fn information_policy_manifest_v1_matches_frozen_external_vector() {
    let manifest = InformationPolicyManifest::from_registry(&minimal_registry());

    assert_eq!(manifest.version(), INFORMATION_POLICY_MANIFEST_VERSION);
    assert_eq!(manifest.as_bytes().len(), 155);
    assert_eq!(encode_hex(manifest.as_bytes()), EXPECTED_V1_HEX);
}
