use crate::error::{validate_text, EvolutionError};
use serde::{Deserialize, Serialize};

macro_rules! semantic_id {
    ($name:ident) => {
        #[derive(
            Debug,
            Clone,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Serialize,
            Deserialize,
        )]
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
    };
}

semantic_id!(HereditarySchemaId);
semantic_id!(LocusId);
semantic_id!(AlleleId);
semantic_id!(PopulationId);
semantic_id!(ReproductionEventId);
semantic_id!(OperatorProfileId);
