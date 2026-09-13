use crate::error::{validate_text, EvolutionError};
use serde::{Deserialize, Deserializer, Serialize};

macro_rules! semantic_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, EvolutionError> {
                let value = value.into();
                validate_text(stringify!($name), &value)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(<D::Error as serde::de::Error>::custom)
            }
        }
    };
}

semantic_id!(HereditarySchemaId);
semantic_id!(LocusId);
semantic_id!(AlleleId);
semantic_id!(PopulationId);
semantic_id!(ReproductionEventId);
semantic_id!(OperatorProfileId);
semantic_id!(EvolutionExperimentId);
semantic_id!(PopulationTransitionId);
semantic_id!(PopulationProcessProfileId);
semantic_id!(PopulationStructureProfileId);
semantic_id!(DemographicEventId);
semantic_id!(ChromosomeId);
semantic_id!(ChromosomeMapId);
semantic_id!(ChromosomeRecombinationProfileId);
semantic_id!(AncestryCopyId);
semantic_id!(EvolutionIndividualId);
semantic_id!(EvolutionaryContextId);
semantic_id!(ConsequenceWindowId);
semantic_id!(ConsequenceObservationId);
semantic_id!(PhenotypeEvidenceSourceId);
semantic_id!(ExposureEvidenceSourceId);
semantic_id!(EvidenceProtocolId);
semantic_id!(SelectionComparisonDesignId);
semantic_id!(PredictorDefinitionId);
semantic_id!(ExclusionReasonId);
semantic_id!(AnalysisMethodId);
semantic_id!(ComparisonAuthorityId);
semantic_id!(CalibrationAuthorityId);
semantic_id!(CausalSelectionIdentificationId);
semantic_id!(IdentificationEvidenceAuthorityId);
semantic_id!(IdentificationQualificationAuthorityId);
semantic_id!(CausalSelectionEffectId);
semantic_id!(CompleteRandomizationReferenceId);
semantic_id!(AssignmentMaterializationAuthorityId);
semantic_id!(AssignmentMaterializationQualificationAuthorityId);
semantic_id!(AssignmentExchangeabilityAuthorityId);

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_wire_contract {
        ($type:ty, $value:literal) => {{
            let original = <$type>::new($value).unwrap();
            let encoded = serde_json::to_string(&original).unwrap();
            let restored: $type = serde_json::from_str(&encoded).unwrap();
            assert_eq!(restored, original);

            assert!(serde_json::from_str::<$type>("\"\"").is_err());
            assert!(serde_json::from_str::<$type>("\"   \"").is_err());
        }};
    }

    #[test]
    fn semantic_ids_preserve_validation_across_serde_restore() {
        assert_wire_contract!(HereditarySchemaId, "schema-v1");
        assert_wire_contract!(LocusId, "pigment");
        assert_wire_contract!(AlleleId, "dark");
        assert_wire_contract!(PopulationId, "island-a");
        assert_wire_contract!(ReproductionEventId, "birth-0001");
        assert_wire_contract!(OperatorProfileId, "reference-operators-v1");
        assert_wire_contract!(EvolutionExperimentId, "experiment-0001");
        assert_wire_contract!(PopulationTransitionId, "generation-0-to-1");
        assert_wire_contract!(PopulationProcessProfileId, "neutral-wf-v1");
        assert_wire_contract!(PopulationStructureProfileId, "archipelago-v1");
        assert_wire_contract!(DemographicEventId, "founder-event-0001");
        assert_wire_contract!(ChromosomeId, "chromosome-1");
        assert_wire_contract!(ChromosomeMapId, "reference-map-v1");
        assert_wire_contract!(ChromosomeRecombinationProfileId, "poisson-reference-v1");
        assert_wire_contract!(AncestryCopyId, "ancestry-copy-0001");
        assert_wire_contract!(EvolutionIndividualId, "individual-0001");
        assert_wire_contract!(EvolutionaryContextId, "context-0001");
        assert_wire_contract!(ConsequenceWindowId, "window-0001");
        assert_wire_contract!(ConsequenceObservationId, "consequence-0001");
        assert_wire_contract!(PhenotypeEvidenceSourceId, "phenotype-source-0001");
        assert_wire_contract!(ExposureEvidenceSourceId, "exposure-source-0001");
        assert_wire_contract!(EvidenceProtocolId, "evidence-protocol-0001");
        assert_wire_contract!(SelectionComparisonDesignId, "selection-design-0001");
        assert_wire_contract!(PredictorDefinitionId, "predictor-0001");
        assert_wire_contract!(ExclusionReasonId, "exclusion-reason-0001");
        assert_wire_contract!(AnalysisMethodId, "analysis-method-0001");
        assert_wire_contract!(ComparisonAuthorityId, "comparison-authority-0001");
        assert_wire_contract!(CalibrationAuthorityId, "calibration-authority-0001");
        assert_wire_contract!(CausalSelectionIdentificationId, "causal-identification-0001");
        assert_wire_contract!(IdentificationEvidenceAuthorityId, "identification-evidence-0001");
        assert_wire_contract!(
            IdentificationQualificationAuthorityId,
            "identification-qualification-0001"
        );
        assert_wire_contract!(CausalSelectionEffectId, "causal-effect-0001");
        assert_wire_contract!(CompleteRandomizationReferenceId, "randomization-reference-0001");
        assert_wire_contract!(
            AssignmentMaterializationAuthorityId,
            "assignment-materialization-0001"
        );
        assert_wire_contract!(
            AssignmentMaterializationQualificationAuthorityId,
            "assignment-materialization-qualification-0001"
        );
        assert_wire_contract!(
            AssignmentExchangeabilityAuthorityId,
            "assignment-exchangeability-0001"
        );
    }

    #[test]
    fn malformed_nested_schema_id_fails_before_schema_can_regain_authority() {
        let raw = r#"{
            "schema_version": 1,
            "id": "   ",
            "ploidy": 2,
            "loci": {
                "pigment": {
                    "id": "pigment",
                    "allowed_alleles": ["dark", "light"]
                }
            }
        }"#;

        assert!(serde_json::from_str::<crate::HereditarySchema>(raw).is_err());
    }

    #[test]
    fn malformed_nested_locus_or_allele_id_fails_during_restore() {
        let bad_locus = r#"{
            "schema_version": 1,
            "id": "schema-v1",
            "ploidy": 2,
            "loci": {
                " ": {
                    "id": " ",
                    "allowed_alleles": ["dark", "light"]
                }
            }
        }"#;
        let bad_allele = r#"{
            "schema_version": 1,
            "id": "schema-v1",
            "ploidy": 2,
            "loci": {
                "pigment": {
                    "id": "pigment",
                    "allowed_alleles": ["dark", " "]
                }
            }
        }"#;

        assert!(serde_json::from_str::<crate::HereditarySchema>(bad_locus).is_err());
    }
}