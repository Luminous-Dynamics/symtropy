// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_lifesim_core::information::{
    CapabilityEvidence, EcologicalAuthorityLevel, EcologicalInformation,
    RepresentationCapabilities, RepresentationKey,
};
use symtropy_lifesim_core::information_policy_manifest::ManifestBoundInformationPolicyRegistry;
use symtropy_lifesim_core::information_registry::{
    InformationPolicyRegistry, InformationPolicyRegistryBuilder, InformationPolicyRegistryKey,
};
use symtropy_lifesim_core::information_transition::{
    InformationTransitionDefinition, InformationTransitionKey, InformationTransitionRegistry,
    InformationTransitionRegistryBuilder, InformationTransitionRegistryKey, LosslessTransformKey,
    PromotionProvenance, RetainedAuthorityKey, TransitionCapabilityClaim,
};
use symtropy_lifesim_core::transition_policy_manifest::{
    ManifestBoundInformationTransitionRegistry, INFORMATION_TRANSITION_MANIFEST_VERSION,
};

const POLICY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(70, 3);
const GRAPH: InformationTransitionRegistryKey = InformationTransitionRegistryKey::new(80, 2);
const A: RepresentationKey = RepresentationKey::new(10, 1);
const B: RepresentationKey = RepresentationKey::new(11, 1);
const EDGE: InformationTransitionKey = InformationTransitionKey::new(20, 1);
const RETAINED: RetainedAuthorityKey = RetainedAuthorityKey(30, 1);
const LOSSLESS: LosslessTransformKey = LosslessTransformKey(31, 1);

const EXPECTED_V1_HEX: &str = concat!(
    "53594d54524f50595f494e464f524d4154494f4e5f5452414e534954494f4e5f4d414e494645535400",
    "01000000",
    "50000000000000000000000000000000",
    "02000000",
    "ad00000000000000",
    "53594d54524f50595f494e464f524d4154494f4e5f504f4c4943595f4d414e494645535400",
    "01000000",
    "46000000000000000000000000000000",
    "03000000",
    "0000000000000000",
    "0200000000000000",
    "0a000000000000000000000000000000",
    "01000000",
    "01",
    "0100000000000000",
    "00",
    "0100000000000000",
    "00",
    "0b000000000000000000000000000000",
    "01000000",
    "01",
    "0200000000000000",
    "00",
    "0100000000000000",
    "00",
    "02",
    "0100000000000000",
    "00",
    "0000000000000000",
    "0100000000000000",
    "14000000000000000000000000000000",
    "01000000",
    "0a000000000000000000000000000000",
    "01000000",
    "0b000000000000000000000000000000",
    "01000000",
    "0100000000000000",
    "02",
    "00",
    "00",
    "1e000000000000000000000000000000",
    "01000000",
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

fn policy() -> InformationPolicyRegistry {
    let mut builder = InformationPolicyRegistryBuilder::new(POLICY);
    builder
        .register_representation(RepresentationCapabilities::new(
            A,
            EcologicalAuthorityLevel::Coarse,
            [(EcologicalInformation::Headcount, CapabilityEvidence::Exact)],
        ))
        .unwrap();
    builder
        .register_representation(RepresentationCapabilities::new(
            B,
            EcologicalAuthorityLevel::Coarse,
            [
                (EcologicalInformation::Headcount, CapabilityEvidence::Exact),
                (
                    EcologicalInformation::AgeDistribution,
                    CapabilityEvidence::Exact,
                ),
            ],
        ))
        .unwrap();
    builder.seal()
}

fn graph(
    policy: &InformationPolicyRegistry,
    provenance: PromotionProvenance,
) -> InformationTransitionRegistry {
    let mut builder = InformationTransitionRegistryBuilder::new(GRAPH);
    builder
        .register_transition(
            InformationTransitionDefinition::new(
                EDGE,
                A,
                B,
                [(
                    TransitionCapabilityClaim::new(
                        EcologicalInformation::AgeDistribution,
                        CapabilityEvidence::Exact,
                    ),
                    provenance,
                )],
                [],
            )
            .unwrap(),
        )
        .unwrap();
    builder.seal(policy).unwrap()
}

#[test]
fn information_transition_manifest_v1_matches_frozen_external_vector() {
    let policy = policy();
    let graph = graph(
        &policy,
        PromotionProvenance::RetainedExact {
            authority: RETAINED,
        },
    );
    let bound_policy = ManifestBoundInformationPolicyRegistry::new(&policy);
    let bound_graph =
        ManifestBoundInformationTransitionRegistry::new(&graph, &bound_policy).unwrap();
    let manifest = bound_graph.authority_stamp().manifest();

    assert_eq!(manifest.version(), INFORMATION_TRANSITION_MANIFEST_VERSION);
    assert_eq!(manifest.as_bytes().len(), 353);
    assert_eq!(encode_hex(manifest.as_bytes()), EXPECTED_V1_HEX);
}

#[test]
fn changing_only_r0_to_r1_provenance_changes_exact_graph_identity() {
    let policy = policy();
    let retained = graph(
        &policy,
        PromotionProvenance::RetainedExact {
            authority: RETAINED,
        },
    );
    let lossless = graph(
        &policy,
        PromotionProvenance::LosslessDerivation {
            transform: LOSSLESS,
        },
    );
    let bound_policy = ManifestBoundInformationPolicyRegistry::new(&policy);
    let retained =
        ManifestBoundInformationTransitionRegistry::new(&retained, &bound_policy).unwrap();
    let lossless =
        ManifestBoundInformationTransitionRegistry::new(&lossless, &bound_policy).unwrap();

    assert_ne!(retained.authority_stamp(), lossless.authority_stamp());
}
