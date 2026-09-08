// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_lifesim_core::applicability_policy_manifest::{
    ManifestBoundTransitionApplicabilityPolicy, TRANSITION_APPLICABILITY_MANIFEST_VERSION,
};
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
    InformationTransitionRegistryBuilder, InformationTransitionRegistryKey, PromotionProvenance,
    RetainedAuthorityKey, TransitionCapabilityClaim,
};
use symtropy_lifesim_core::transition_applicability::{
    TransitionApplicability, TransitionApplicabilityPolicy, TransitionApplicabilityPolicyBuilder,
    TransitionApplicabilityPolicyKey, TransitionDomainKey,
};
use symtropy_lifesim_core::transition_policy_manifest::ManifestBoundInformationTransitionRegistry;

const POLICY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(70, 3);
const GRAPH: InformationTransitionRegistryKey = InformationTransitionRegistryKey::new(80, 2);
const APPLICABILITY: TransitionApplicabilityPolicyKey =
    TransitionApplicabilityPolicyKey::new(90, 4);
const A: RepresentationKey = RepresentationKey::new(10, 1);
const B: RepresentationKey = RepresentationKey::new(11, 1);
const EDGE: InformationTransitionKey = InformationTransitionKey::new(20, 1);
const RETAINED: RetainedAuthorityKey = RetainedAuthorityKey(30, 1);
const DOMAIN: TransitionDomainKey = TransitionDomainKey::new(100, 1);

const TRANSITION_V1_HEX: &str = concat!(
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

const APPLICABILITY_PREFIX_HEX: &str = concat!(
    "53594d54524f50595f5452414e534954494f4e5f4150504c49434142494c4954595f4d414e494645535400",
    "01000000",
    "5a000000000000000000000000000000",
    "04000000",
    "6101000000000000",
);

const UNIVERSAL_SUFFIX_HEX: &str = concat!(
    "0100000000000000",
    "14000000000000000000000000000000",
    "01000000",
    "00",
);

const DOMAIN_SUFFIX_HEX: &str = concat!(
    "0100000000000000",
    "14000000000000000000000000000000",
    "01000000",
    "01",
    "64000000000000000000000000000000",
    "01000000",
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

fn graph(policy: &InformationPolicyRegistry) -> InformationTransitionRegistry {
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
                    PromotionProvenance::RetainedExact {
                        authority: RETAINED,
                    },
                )],
                [],
            )
            .unwrap(),
        )
        .unwrap();
    builder.seal(policy).unwrap()
}

fn applicability(
    graph: &InformationTransitionRegistry,
    mode: TransitionApplicability,
) -> TransitionApplicabilityPolicy {
    let mut builder = TransitionApplicabilityPolicyBuilder::new(APPLICABILITY);
    builder.register(EDGE, mode).unwrap();
    builder.seal(graph).unwrap()
}

fn exact_manifest(mode: TransitionApplicability) -> Vec<u8> {
    let policy = policy();
    let graph = graph(&policy);
    let applicability = applicability(&graph, mode);
    let bound_policy = ManifestBoundInformationPolicyRegistry::new(&policy);
    let bound_graph =
        ManifestBoundInformationTransitionRegistry::new(&graph, &bound_policy).unwrap();
    ManifestBoundTransitionApplicabilityPolicy::new(&applicability, &bound_graph)
        .unwrap()
        .authority_stamp()
        .manifest()
        .as_bytes()
        .to_vec()
}

#[test]
fn transition_applicability_manifest_v1_matches_frozen_universal_vector() {
    let manifest = exact_manifest(TransitionApplicability::Universal);
    let expected = format!(
        "{APPLICABILITY_PREFIX_HEX}{TRANSITION_V1_HEX}{UNIVERSAL_SUFFIX_HEX}"
    );

    assert_eq!(TRANSITION_APPLICABILITY_MANIFEST_VERSION, 1);
    assert_eq!(manifest.len(), 457);
    assert_eq!(encode_hex(&manifest), expected);
}

#[test]
fn registered_domain_payload_is_frozen_and_changes_exact_identity() {
    let universal = exact_manifest(TransitionApplicability::Universal);
    let gated = exact_manifest(TransitionApplicability::RegisteredDomain(DOMAIN));
    let expected = format!("{APPLICABILITY_PREFIX_HEX}{TRANSITION_V1_HEX}{DOMAIN_SUFFIX_HEX}");

    assert_eq!(gated.len(), 477);
    assert_eq!(encode_hex(&gated), expected);
    assert_ne!(universal, gated);
}
