//! Exact finite-rank spatial field reference with mesh-independent coordinates.
//!
//! The stochastic atoms are stable basis coefficients. Mesh vertices are query
//! locations and never become random-number identities.

use std::collections::{BTreeMap, BTreeSet};

use crate::StableId;

const DOMAIN: &[u8] = b"symtropy.road-spatial-dependence.v0.1\0";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct RationalCoordinate {
    pub numerator: u32,
    pub denominator: u32,
}

impl RationalCoordinate {
    pub fn new(numerator: u32, denominator: u32) -> Result<Self, SpatialError> {
        if denominator == 0 || numerator > denominator {
            return Err(SpatialError::InvalidCoordinate);
        }
        let divisor = gcd(u128::from(numerator), u128::from(denominator));
        let divisor = u32::try_from(divisor).map_err(|_| SpatialError::ArithmeticOverflow)?;
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SpatialCoupling {
    SharedInvariant {
        equivalence_group: StableId,
        justification: StableId,
    },
    BranchSpecific {
        branch: StableId,
    },
    NotIdentified {
        evidence_ref: StableId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct BasisCoefficientSpec {
    pub id: StableId,
    pub source_id: StableId,
    pub source_profile_digest: [u8; 32],
    pub units: StableId,
    pub coupling: SpatialCoupling,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct BasisCoefficientState {
    pub left_value: i64,
    pub right_value: i64,
    pub weight: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpatialFieldManifest {
    pub field_id: StableId,
    pub version: StableId,
    pub mechanism_id: StableId,
    pub coordinate_frame: StableId,
    pub domain_id: StableId,
    pub left_basis: BasisCoefficientSpec,
    pub right_basis: BasisCoefficientSpec,
    pub basis_version: StableId,
    pub evidence_ref: StableId,
    pub states: Vec<BasisCoefficientState>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SpatialQueryContext {
    /// Discretization identity is provenance only; it does not select a draw.
    pub mesh_id: StableId,
    pub refinement_level: StableId,
    /// Optional caller ordinal; never used as a stochastic identity.
    pub vertex_ordinal: Option<u64>,
    pub coordinate_transform_id: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactRational {
    pub numerator: i128,
    pub denominator: u128,
}

impl ExactRational {
    fn new(numerator: i128, denominator: u128) -> Result<Self, SpatialError> {
        if denominator == 0 {
            return Err(SpatialError::ArithmeticOverflow);
        }
        if numerator == 0 {
            return Ok(Self {
                numerator: 0,
                denominator: 1,
            });
        }
        let divisor = gcd(numerator.unsigned_abs(), denominator);
        let signed_divisor =
            i128::try_from(divisor).map_err(|_| SpatialError::ArithmeticOverflow)?;
        Ok(Self {
            numerator: numerator / signed_divisor,
            denominator: denominator / divisor,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpatialError {
    InvalidCoordinate,
    EmptyStateTable,
    InvalidBasisIdentity,
    DuplicateJointState,
    ZeroStateWeight,
    WeightOverflow,
    InvalidCoefficientValue,
    CoefficientsNotIndependent,
    NotIdentified,
    WrongBranchScope,
    IncompatibleFieldEvaluations,
    UnknownQuery,
    ArithmeticOverflow,
    ReceiptMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpatialFieldReceipt {
    pub schema_id: &'static str,
    pub manifest_digest: [u8; 32],
    pub branch_id: StableId,
    pub coordinate: RationalCoordinate,
    pub query_context: SpatialQueryContext,
    pub left_basis_id: StableId,
    pub right_basis_id: StableId,
    pub left_coefficient: i64,
    pub right_coefficient: i64,
    pub left_basis_weight: u32,
    pub right_basis_weight: u32,
    /// Exact field value is field_value_numerator / coordinate.denominator.
    pub field_value_numerator: i128,
    pub field_value_denominator: u32,
    pub state_weight: u64,
    pub receipt_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpatialFieldEvaluation {
    pub manifest_digest: [u8; 32],
    pub branch_id: StableId,
    pub coordinate: RationalCoordinate,
    pub total_weight: u64,
    pub receipts: Vec<SpatialFieldReceipt>,
}

impl SpatialFieldManifest {
    pub fn validate(&self) -> Result<u64, SpatialError> {
        if self.left_basis.id == self.right_basis.id
            || self.coordinate_frame.as_str().is_empty()
            || self.domain_id.as_str().is_empty()
        {
            return Err(SpatialError::InvalidBasisIdentity);
        }
        if self.states.is_empty() {
            return Err(SpatialError::EmptyStateTable);
        }

        let mut unique = BTreeSet::new();
        let mut left_marginal = BTreeMap::<i64, u64>::new();
        let mut right_marginal = BTreeMap::<i64, u64>::new();
        let mut total = 0u64;

        for state in &self.states {
            if !matches!(state.left_value, -1 | 1) || !matches!(state.right_value, -1 | 1) {
                return Err(SpatialError::InvalidCoefficientValue);
            }
            if state.weight == 0 {
                return Err(SpatialError::ZeroStateWeight);
            }
            if !unique.insert((state.left_value, state.right_value)) {
                return Err(SpatialError::DuplicateJointState);
            }
            total = total
                .checked_add(state.weight)
                .ok_or(SpatialError::WeightOverflow)?;
            let left = left_marginal.entry(state.left_value).or_default();
            *left = left
                .checked_add(state.weight)
                .ok_or(SpatialError::WeightOverflow)?;
            let right = right_marginal.entry(state.right_value).or_default();
            *right = right
                .checked_add(state.weight)
                .ok_or(SpatialError::WeightOverflow)?;
        }

        for state in &self.states {
            let left_mass = u128::from(left_marginal[&state.left_value]);
            let right_mass = u128::from(right_marginal[&state.right_value]);
            let observed = u128::from(state.weight)
                .checked_mul(u128::from(total))
                .ok_or(SpatialError::ArithmeticOverflow)?;
            let independent = left_mass
                .checked_mul(right_mass)
                .ok_or(SpatialError::ArithmeticOverflow)?;
            if observed != independent {
                return Err(SpatialError::CoefficientsNotIndependent);
            }
        }
        Ok(total)
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, SpatialError> {
        self.validate()?;
        let mut out = Vec::new();
        out.extend_from_slice(DOMAIN);
        push_id(&mut out, &self.field_id);
        push_id(&mut out, &self.version);
        push_id(&mut out, &self.mechanism_id);
        push_id(&mut out, &self.coordinate_frame);
        push_id(&mut out, &self.domain_id);
        push_id(&mut out, &self.basis_version);
        push_id(&mut out, &self.evidence_ref);
        push_basis_spec(&mut out, &self.left_basis);
        push_basis_spec(&mut out, &self.right_basis);

        let mut states = self.states.iter().collect::<Vec<_>>();
        states.sort_by_key(|state| (state.left_value, state.right_value));
        push_len(&mut out, states.len());
        for state in states {
            out.extend_from_slice(&state.left_value.to_le_bytes());
            out.extend_from_slice(&state.right_value.to_le_bytes());
            out.extend_from_slice(&state.weight.to_le_bytes());
        }
        Ok(out)
    }

    pub fn digest(&self) -> Result<[u8; 32], SpatialError> {
        Ok(*blake3::hash(&self.canonical_bytes()?).as_bytes())
    }

    pub fn evaluate(
        &self,
        coordinate: RationalCoordinate,
        branch_id: StableId,
        query_context: SpatialQueryContext,
    ) -> Result<SpatialFieldEvaluation, SpatialError> {
        let total_weight = self.validate()?;
        validate_coupling(&self.left_basis.coupling, &branch_id)?;
        validate_coupling(&self.right_basis.coupling, &branch_id)?;
        let manifest_digest = self.digest()?;
        let left_weight = coordinate.denominator - coordinate.numerator;
        let right_weight = coordinate.numerator;

        let mut states = self.states.iter().collect::<Vec<_>>();
        states.sort_by_key(|state| (state.left_value, state.right_value));
        let mut receipts = Vec::with_capacity(states.len());

        for state in states {
            let value_left = i128::from(state.left_value)
                .checked_mul(i128::from(left_weight))
                .ok_or(SpatialError::ArithmeticOverflow)?;
            let value_right = i128::from(state.right_value)
                .checked_mul(i128::from(right_weight))
                .ok_or(SpatialError::ArithmeticOverflow)?;
            let field_value_numerator = value_left
                .checked_add(value_right)
                .ok_or(SpatialError::ArithmeticOverflow)?;

            let mut receipt = SpatialFieldReceipt {
                schema_id: "symtropy.road-spatial-dependence.receipt.v0.1",
                manifest_digest,
                branch_id: branch_id.clone(),
                coordinate: coordinate.clone(),
                query_context: query_context.clone(),
                left_basis_id: self.left_basis.id.clone(),
                right_basis_id: self.right_basis.id.clone(),
                left_coefficient: state.left_value,
                right_coefficient: state.right_value,
                left_basis_weight: left_weight,
                right_basis_weight: right_weight,
                field_value_numerator,
                field_value_denominator: coordinate.denominator,
                state_weight: state.weight,
                receipt_digest: [0; 32],
            };
            receipt.receipt_digest = *blake3::hash(&receipt_bytes(&receipt)).as_bytes();
            receipts.push(receipt);
        }

        Ok(SpatialFieldEvaluation {
            manifest_digest,
            branch_id,
            coordinate,
            total_weight,
            receipts,
        })
    }
}

impl SpatialFieldEvaluation {
    pub fn mean(&self) -> Result<ExactRational, SpatialError> {
        let mut weighted_sum = 0i128;
        for receipt in &self.receipts {
            weighted_sum = weighted_sum
                .checked_add(
                    receipt
                        .field_value_numerator
                        .checked_mul(i128::from(receipt.state_weight))
                        .ok_or(SpatialError::ArithmeticOverflow)?,
                )
                .ok_or(SpatialError::ArithmeticOverflow)?;
        }
        let denominator = u128::from(self.total_weight)
            .checked_mul(u128::from(self.coordinate.denominator))
            .ok_or(SpatialError::ArithmeticOverflow)?;
        ExactRational::new(weighted_sum, denominator)
    }

    pub fn variance(&self) -> Result<ExactRational, SpatialError> {
        self.covariance_with(self)
    }

    pub fn covariance_with(
        &self,
        other: &SpatialFieldEvaluation,
    ) -> Result<ExactRational, SpatialError> {
        if self.manifest_digest != other.manifest_digest
            || self.total_weight != other.total_weight
            || self.receipts.len() != other.receipts.len()
        {
            return Err(SpatialError::IncompatibleFieldEvaluations);
        }

        let mut sum_left = 0i128;
        let mut sum_right = 0i128;
        let mut sum_product = 0i128;

        for (left, right) in self.receipts.iter().zip(&other.receipts) {
            if left.left_coefficient != right.left_coefficient
                || left.right_coefficient != right.right_coefficient
                || left.state_weight != right.state_weight
            {
                return Err(SpatialError::IncompatibleFieldEvaluations);
            }
            let weight = i128::from(left.state_weight);
            sum_left = sum_left
                .checked_add(
                    left.field_value_numerator
                        .checked_mul(weight)
                        .ok_or(SpatialError::ArithmeticOverflow)?,
                )
                .ok_or(SpatialError::ArithmeticOverflow)?;
            sum_right = sum_right
                .checked_add(
                    right
                        .field_value_numerator
                        .checked_mul(weight)
                        .ok_or(SpatialError::ArithmeticOverflow)?,
                )
                .ok_or(SpatialError::ArithmeticOverflow)?;
            sum_product = sum_product
                .checked_add(
                    left.field_value_numerator
                        .checked_mul(right.field_value_numerator)
                        .and_then(|value| value.checked_mul(weight))
                        .ok_or(SpatialError::ArithmeticOverflow)?,
                )
                .ok_or(SpatialError::ArithmeticOverflow)?;
        }

        let total = i128::from(self.total_weight);
        let numerator = sum_product
            .checked_mul(total)
            .and_then(|value| {
                sum_left
                    .checked_mul(sum_right)
                    .and_then(|means| value.checked_sub(means))
            })
            .ok_or(SpatialError::ArithmeticOverflow)?;
        let denominator = u128::from(self.total_weight)
            .checked_mul(u128::from(self.total_weight))
            .and_then(|value| value.checked_mul(u128::from(self.coordinate.denominator)))
            .and_then(|value| value.checked_mul(u128::from(other.coordinate.denominator)))
            .ok_or(SpatialError::ArithmeticOverflow)?;
        ExactRational::new(numerator, denominator)
    }

    pub fn marginal_counts(&self) -> Result<BTreeMap<i128, u64>, SpatialError> {
        let mut counts = BTreeMap::new();
        for receipt in &self.receipts {
            let entry = counts.entry(receipt.field_value_numerator).or_insert(0u64);
            *entry = entry
                .checked_add(receipt.state_weight)
                .ok_or(SpatialError::WeightOverflow)?;
        }
        Ok(counts)
    }
}

impl SpatialFieldReceipt {
    pub fn integrity_check(&self) -> bool {
        *blake3::hash(&receipt_bytes(self)).as_bytes() == self.receipt_digest
    }

    pub fn verify_against(&self, manifest: &SpatialFieldManifest) -> Result<(), SpatialError> {
        let evaluation = manifest.evaluate(
            self.coordinate.clone(),
            self.branch_id.clone(),
            self.query_context.clone(),
        )?;
        if evaluation
            .receipts
            .iter()
            .any(|candidate| candidate == self)
        {
            Ok(())
        } else {
            Err(SpatialError::ReceiptMismatch)
        }
    }
}

fn validate_coupling(coupling: &SpatialCoupling, branch: &StableId) -> Result<(), SpatialError> {
    match coupling {
        SpatialCoupling::SharedInvariant { .. } => Ok(()),
        SpatialCoupling::BranchSpecific { branch: scope } if scope == branch => Ok(()),
        SpatialCoupling::BranchSpecific { .. } => Err(SpatialError::WrongBranchScope),
        SpatialCoupling::NotIdentified { .. } => Err(SpatialError::NotIdentified),
    }
}

fn push_basis_spec(out: &mut Vec<u8>, spec: &BasisCoefficientSpec) {
    push_id(out, &spec.id);
    push_id(out, &spec.source_id);
    out.extend_from_slice(&spec.source_profile_digest);
    push_id(out, &spec.units);
    match &spec.coupling {
        SpatialCoupling::SharedInvariant {
            equivalence_group,
            justification,
        } => {
            out.push(1);
            push_id(out, equivalence_group);
            push_id(out, justification);
        }
        SpatialCoupling::BranchSpecific { branch } => {
            out.push(2);
            push_id(out, branch);
        }
        SpatialCoupling::NotIdentified { evidence_ref } => {
            out.push(3);
            push_id(out, evidence_ref);
        }
    }
}

fn receipt_bytes(receipt: &SpatialFieldReceipt) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"symtropy.road-spatial-dependence.receipt.v0.1\0");
    push_bytes(&mut out, receipt.schema_id.as_bytes()).expect("static schema ID length fits u64");
    out.extend_from_slice(&receipt.manifest_digest);
    push_id(&mut out, &receipt.branch_id);
    out.extend_from_slice(&receipt.coordinate.numerator.to_le_bytes());
    out.extend_from_slice(&receipt.coordinate.denominator.to_le_bytes());
    push_id(&mut out, &receipt.query_context.mesh_id);
    push_id(&mut out, &receipt.query_context.refinement_level);
    match receipt.query_context.vertex_ordinal {
        Some(value) => {
            out.push(1);
            out.extend_from_slice(&value.to_le_bytes());
        }
        None => out.push(0),
    }
    push_id(&mut out, &receipt.query_context.coordinate_transform_id);
    push_id(&mut out, &receipt.left_basis_id);
    push_id(&mut out, &receipt.right_basis_id);
    out.extend_from_slice(&receipt.left_coefficient.to_le_bytes());
    out.extend_from_slice(&receipt.right_coefficient.to_le_bytes());
    out.extend_from_slice(&receipt.left_basis_weight.to_le_bytes());
    out.extend_from_slice(&receipt.right_basis_weight.to_le_bytes());
    out.extend_from_slice(&receipt.field_value_numerator.to_le_bytes());
    out.extend_from_slice(&receipt.field_value_denominator.to_le_bytes());
    out.extend_from_slice(&receipt.state_weight.to_le_bytes());
    out
}

fn push_id(out: &mut Vec<u8>, id: &StableId) {
    let bytes = id.as_str().as_bytes();
    let length = u32::try_from(bytes.len()).expect("StableId length is bounded");
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), SpatialError> {
    let length = u64::try_from(bytes.len()).map_err(|_| SpatialError::ArithmeticOverflow)?;
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(bytes);
    Ok(())
}

fn push_len(out: &mut Vec<u8>, len: usize) {
    let len = u64::try_from(len).expect("fixture length must fit u64");
    out.extend_from_slice(&len.to_le_bytes());
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::new(value).expect("fixture ID")
    }

    fn manifest() -> SpatialFieldManifest {
        SpatialFieldManifest {
            field_id: id("road-weather-cross-section"),
            version: id("v0.1"),
            mechanism_id: id("linear-lagrange-1d-v1"),
            coordinate_frame: id("road-local-x"),
            domain_id: id("normalized-unit-interval"),
            left_basis: basis_spec("basis-left"),
            right_basis: basis_spec("basis-right"),
            basis_version: id("linear-lagrange-basis-v1"),
            evidence_ref: id("fixture:exact-finite-basis-law"),
            states: vec![
                BasisCoefficientState {
                    left_value: -1,
                    right_value: -1,
                    weight: 1,
                },
                BasisCoefficientState {
                    left_value: -1,
                    right_value: 1,
                    weight: 1,
                },
                BasisCoefficientState {
                    left_value: 1,
                    right_value: -1,
                    weight: 1,
                },
                BasisCoefficientState {
                    left_value: 1,
                    right_value: 1,
                    weight: 1,
                },
            ],
        }
    }

    fn shared_coupling() -> SpatialCoupling {
        SpatialCoupling::SharedInvariant {
            equivalence_group: id("weather-cross-section"),
            justification: id("scm:shared-field-coefficients"),
        }
    }

    fn basis_spec(name: &str) -> BasisCoefficientSpec {
        BasisCoefficientSpec {
            id: id(name),
            source_id: id("weather-source-01"),
            source_profile_digest: *blake3::hash(b"weather-source-profile-v1").as_bytes(),
            units: id("normalized-distance"),
            coupling: shared_coupling(),
        }
    }

    fn context(mesh: &str, level: &str, ordinal: u64) -> SpatialQueryContext {
        SpatialQueryContext {
            mesh_id: id(mesh),
            refinement_level: id(level),
            vertex_ordinal: Some(ordinal),
            coordinate_transform_id: id("identity-coordinate-transform-v1"),
        }
    }

    fn evaluate(
        manifest: &SpatialFieldManifest,
        numerator: u32,
        denominator: u32,
        mesh: &str,
        level: &str,
        ordinal: u64,
    ) -> SpatialFieldEvaluation {
        manifest
            .evaluate(
                RationalCoordinate::new(numerator, denominator).unwrap(),
                id("branch-a"),
                context(mesh, level, ordinal),
            )
            .unwrap()
    }

    #[test]
    fn shared_linear_basis_has_exact_variance_five_eighths_and_covariance_three_eighths() {
        let manifest = manifest();
        let left = evaluate(&manifest, 1, 4, "mesh-coarse", "L0", 1);
        let right = evaluate(&manifest, 3, 4, "mesh-coarse", "L0", 3);

        assert_eq!(
            left.variance().unwrap(),
            ExactRational {
                numerator: 5,
                denominator: 8
            }
        );
        assert_eq!(
            right.variance().unwrap(),
            ExactRational {
                numerator: 5,
                denominator: 8
            }
        );
        assert_eq!(
            left.covariance_with(&right).unwrap(),
            ExactRational {
                numerator: 3,
                denominator: 8
            }
        );
    }

    #[test]
    fn independent_per_site_control_has_same_marginals_but_zero_covariance() {
        let manifest = manifest();
        let left = evaluate(&manifest, 1, 4, "mesh-coarse", "L0", 1);
        let right = evaluate(&manifest, 3, 4, "mesh-coarse", "L0", 3);
        assert_eq!(
            left.marginal_counts().unwrap(),
            right.marginal_counts().unwrap()
        );

        // The independent-per-site joint law is the Cartesian product of
        // the same pointwise marginal, producing a 16-state joint table.
        // Compute its exact cross-covariance using the common integer
        // numerator (denominator 4) at each site.
        let marginal = left.marginal_counts().unwrap();
        let mut total_joint = 0i128;
        let mut sum_left = 0i128;
        let mut sum_right = 0i128;
        let mut sum_product = 0i128;
        for (left_value, left_weight) in &marginal {
            for (right_value, right_weight) in &marginal {
                let weight = i128::from(*left_weight) * i128::from(*right_weight);
                total_joint += weight;
                sum_left += left_value * weight;
                sum_right += right_value * weight;
                sum_product += left_value * right_value * weight;
            }
        }
        let numerator = sum_product * total_joint - sum_left * sum_right;
        let denominator = u128::try_from(total_joint * total_joint).unwrap() * 16;
        assert_eq!(
            ExactRational::new(numerator, denominator).unwrap(),
            ExactRational {
                numerator: 0,
                denominator: 1
            }
        );
    }

    #[test]
    fn same_physical_coordinate_is_invariant_across_meshes_and_vertex_orders() {
        let manifest = manifest();
        let coarse = evaluate(&manifest, 1, 4, "mesh-coarse", "L0", 1);
        let refined = evaluate(&manifest, 2, 8, "mesh-refined", "L1", 7);
        assert_eq!(coarse.manifest_digest, refined.manifest_digest);
        assert_eq!(coarse.coordinate, refined.coordinate);
        for (left, right) in coarse.receipts.iter().zip(&refined.receipts) {
            assert_eq!(left.left_coefficient, right.left_coefficient);
            assert_eq!(left.right_coefficient, right.right_coefficient);
            assert_eq!(left.field_value_numerator, right.field_value_numerator);
            assert_eq!(left.field_value_denominator, right.field_value_denominator);
        }
    }

    #[test]
    fn rational_coordinate_normalization_is_canonical() {
        assert_eq!(
            RationalCoordinate::new(2, 8).unwrap(),
            RationalCoordinate::new(1, 4).unwrap()
        );
    }

    #[test]
    fn endpoint_basis_values_are_exact() {
        let manifest = manifest();
        let left = evaluate(&manifest, 0, 1, "mesh", "L0", 0);
        let right = evaluate(&manifest, 1, 1, "mesh", "L0", 1);
        for (receipt_left, receipt_right) in left.receipts.iter().zip(&right.receipts) {
            assert_eq!(
                receipt_left.field_value_numerator,
                receipt_left.left_coefficient.into()
            );
            assert_eq!(
                receipt_right.field_value_numerator,
                receipt_right.right_coefficient.into()
            );
        }
    }

    #[test]
    fn declaration_order_does_not_change_field_manifest_or_receipts() {
        let first = manifest();
        let mut second = first.clone();
        second.states.reverse();
        let a = evaluate(&first, 1, 4, "mesh", "L0", 2);
        let b = evaluate(&second, 1, 4, "mesh", "L0", 2);
        assert_eq!(a.manifest_digest, b.manifest_digest);
        assert_eq!(a.receipts, b.receipts);
    }

    #[test]
    fn profile_identity_and_basis_identity_are_bound() {
        let first = manifest();
        let mut changed = first.clone();
        changed.version = id("v0.2");
        assert_ne!(first.digest().unwrap(), changed.digest().unwrap());

        let mut renamed = first.clone();
        renamed.left_basis.id = id("basis-west");
        assert_ne!(first.digest().unwrap(), renamed.digest().unwrap());
    }

    #[test]
    fn unknown_or_wrong_branch_coupling_is_rejected() {
        let mut unknown_manifest = manifest();
        unknown_manifest.left_basis.coupling = SpatialCoupling::NotIdentified {
            evidence_ref: id("evidence:unknown"),
        };
        assert_eq!(
            unknown_manifest.evaluate(
                RationalCoordinate::new(1, 4).unwrap(),
                id("branch-a"),
                context("mesh", "L0", 0),
            ),
            Err(SpatialError::NotIdentified)
        );

        let mut branch_manifest = manifest();
        branch_manifest.left_basis.coupling = SpatialCoupling::BranchSpecific {
            branch: id("branch-b"),
        };
        assert_eq!(
            branch_manifest.evaluate(
                RationalCoordinate::new(1, 4).unwrap(),
                id("branch-a"),
                context("mesh", "L0", 0),
            ),
            Err(SpatialError::WrongBranchScope)
        );
    }

    #[test]
    fn source_profile_units_and_coordinate_frame_are_bound_to_manifest() {
        let first = manifest();
        let mut changed_source = first.clone();
        changed_source.left_basis.source_profile_digest[0] ^= 1;
        assert_ne!(first.digest().unwrap(), changed_source.digest().unwrap());

        let mut changed_units = first.clone();
        changed_units.left_basis.units = id("meters");
        assert_ne!(first.digest().unwrap(), changed_units.digest().unwrap());

        let mut changed_frame = first.clone();
        changed_frame.coordinate_frame = id("projected-road-frame");
        assert_ne!(first.digest().unwrap(), changed_frame.digest().unwrap());
    }

    #[test]
    fn mesh_context_tampering_fails_manifest_replay() {
        let manifest = manifest();
        let evaluation = evaluate(&manifest, 1, 4, "mesh-coarse", "L0", 1);
        let mut receipt = evaluation.receipts[0].clone();
        receipt.query_context.vertex_ordinal = Some(99);
        assert!(!receipt.integrity_check());
        assert_eq!(
            receipt.verify_against(&manifest),
            Err(SpatialError::ReceiptMismatch)
        );
    }

    #[test]
    fn invalid_coordinates_duplicate_states_and_zero_weights_reject() {
        assert_eq!(
            RationalCoordinate::new(1, 0),
            Err(SpatialError::InvalidCoordinate)
        );
        assert_eq!(
            RationalCoordinate::new(2, 1),
            Err(SpatialError::InvalidCoordinate)
        );

        let mut duplicate = manifest();
        duplicate.states.push(duplicate.states[0].clone());
        assert_eq!(duplicate.validate(), Err(SpatialError::DuplicateJointState));

        let mut zero = manifest();
        zero.states[0].weight = 0;
        assert_eq!(zero.validate(), Err(SpatialError::ZeroStateWeight));
    }

    #[test]
    fn receipt_replay_is_bound_to_mesh_context_and_coordinate() {
        let manifest = manifest();
        let evaluation = evaluate(&manifest, 1, 4, "mesh-coarse", "L0", 1);
        let receipt = evaluation.receipts[0].clone();
        assert!(receipt.integrity_check());
        assert_eq!(receipt.verify_against(&manifest), Ok(()));

        let mut tampered = receipt;
        tampered.query_context.vertex_ordinal = Some(99);
        assert!(!tampered.integrity_check());
    }
}
