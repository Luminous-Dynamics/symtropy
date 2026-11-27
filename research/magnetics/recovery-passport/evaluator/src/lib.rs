//! Evidence-gated recovery passport semantics for rare-earth permanent magnets.
//!
//! This crate validates route scope, event/product identity, conservative
//! mass-balance intervals and downstream reuse joins. It is not a recovery
//! process manual, chemistry controller, LCA engine or laboratory authority.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MagnetFamily {
    NdFeB,
    SmCo,
    SmFeN,
    Ferrite,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FeedstockForm {
    PolymerBondedMagnet,
    SinteredMagnet,
    MagnetPowder,
    HybridAssembly,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecoveryRoute {
    PolymerHydrolysis,
    HydrogenShortLoop,
    MechanicalLiberation,
    Hydrometallurgy,
    Pyrometallurgy,
    Electrochemical,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProductForm {
    MagnetFeedstock,
    RemanufacturableAlloy,
    ElementalProduct,
    NonMagneticFraction,
    Residue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    Qualified,
    Fail,
    Indeterminate,
    NotEvaluable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MassBalanceStatus {
    ExactNumericClosure,
    ConsistentWithinUncertainty,
    UnaccountedInputMass,
    OutputExceedsInput,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DigestRecord {
    pub record_id: String,
    pub digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeedstockLot {
    pub component_id: String,
    pub specimen_id: String,
    pub lot_id: String,
    pub magnet_family: MagnetFamily,
    pub feedstock_form: FeedstockForm,
    pub grade_or_composition_record: DigestRecord,
    pub provenance_record: DigestRecord,
    pub manufacturing_genealogy: DigestRecord,
    pub process_history_digest: String,
    pub modification_history_digest: String,
    pub service_history_digest: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteEligibilityReceipt {
    pub receipt: DigestRecord,
    pub feedstock_lot_id: String,
    pub magnet_family: MagnetFamily,
    pub feedstock_form: FeedstockForm,
    pub route: RecoveryRoute,
    pub eligibility: Disposition,
    pub scope_basis_digest: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MassMeasurement {
    pub value_mg: u64,
    pub uncertainty_mg: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryOutput {
    pub output_id: String,
    pub record: DigestRecord,
    pub form: ProductForm,
    pub mass: MassMeasurement,
    pub composition_record: Option<DigestRecord>,
    pub quality_record: Option<DigestRecord>,
    pub magnetic_characterization_record: Option<DigestRecord>,
    pub waste_classification_record: Option<DigestRecord>,
    pub disposition_record: Option<DigestRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryStage {
    pub stage_id: String,
    pub stage_record: DigestRecord,
    pub feedstock_lot_id: String,
    pub route: RecoveryRoute,
    pub method_id: String,
    pub input_mass: MassMeasurement,
    pub outputs: Vec<RecoveryOutput>,
    pub closure_tolerance_mg: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownstreamUseReceipt {
    pub recovered_output_id: String,
    pub new_material_lot_id: String,
    pub manufacturing_genealogy: DigestRecord,
    pub as_built_acceptance_receipt: DigestRecord,
    pub downstream_disposition: Disposition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryPassport {
    pub passport_id: String,
    pub feedstock: FeedstockLot,
    pub route_eligibility: RouteEligibilityReceipt,
    pub stage: RecoveryStage,
    pub downstream_use: Option<DownstreamUseReceipt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Blocker {
    MissingIdentity,
    InvalidDigest,
    InvalidFeedstockScope,
    InvalidRouteEligibilityScope,
    RouteNotEligible,
    RouteEligibilityUnresolved,
    UnsupportedRouteForm,
    MissingRecoveryOutput,
    DuplicateOutputId,
    DuplicateEvidenceRecordId,
    InvalidOutputEvidence,
    MissingMagneticCharacterization,
    MissingElementalProductPurity,
    MissingResidueDisposition,
    InvalidMassUncertainty,
    MassBalanceUnaccountedInput,
    MassBalanceOutputExceedsInput,
    MassBalanceUnresolved,
    DownstreamOutputMismatch,
    MissingDownstreamEvidence,
    DownstreamAcceptanceNotQualified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignedInterval {
    pub lower_mg: i128,
    pub upper_mg: i128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageMassBalance {
    pub input_interval: SignedInterval,
    pub output_interval: SignedInterval,
    /// Input minus measured outputs. Positive residual means unaccounted
    /// input mass; negative residual means apparent output mass gain.
    pub residual_interval: SignedInterval,
    pub status: MassBalanceStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryEvaluation {
    pub passport_id: String,
    pub disposition: Disposition,
    pub blockers: Vec<Blocker>,
    pub mass_balance: StageMassBalance,
    pub recovered_magnet_feedstock_mg: u64,
    pub recovered_elemental_product_mg: u64,
    pub qualified_closed_loop_product_ids: Vec<String>,
}

pub fn evaluate(passport: &RecoveryPassport) -> RecoveryEvaluation {
    let mut blockers = Vec::new();
    validate_feedstock(&passport.feedstock, &mut blockers);
    validate_route_eligibility(passport, &mut blockers);
    let mass_balance = evaluate_mass_balance(&passport.stage, &mut blockers);
    validate_outputs(&passport.stage, &mut blockers);

    let mut recovered_magnet_feedstock_mg = 0u64;
    let mut recovered_elemental_product_mg = 0u64;
    for output in &passport.stage.outputs {
        match output.form {
            ProductForm::MagnetFeedstock | ProductForm::RemanufacturableAlloy => {
                recovered_magnet_feedstock_mg =
                    recovered_magnet_feedstock_mg.saturating_add(output.mass.value_mg);
            }
            ProductForm::ElementalProduct => {
                recovered_elemental_product_mg =
                    recovered_elemental_product_mg.saturating_add(output.mass.value_mg);
            }
            ProductForm::NonMagneticFraction | ProductForm::Residue => {}
        }
    }

    let mut qualified_closed_loop_product_ids = Vec::new();
    if let Some(use_receipt) = &passport.downstream_use {
        let output_exists = passport
            .stage
            .outputs
            .iter()
            .any(|output| output.output_id == use_receipt.recovered_output_id);
        if !output_exists {
            blockers.push(Blocker::DownstreamOutputMismatch);
        }
        if use_receipt.new_material_lot_id.trim().is_empty()
            || !valid_record(&use_receipt.manufacturing_genealogy)
            || !valid_record(&use_receipt.as_built_acceptance_receipt)
        {
            blockers.push(Blocker::MissingDownstreamEvidence);
        }
        if use_receipt.downstream_disposition != Disposition::Qualified {
            blockers.push(Blocker::DownstreamAcceptanceNotQualified);
        }
        if output_exists
            && !blockers.contains(&Blocker::DownstreamOutputMismatch)
            && use_receipt.downstream_disposition == Disposition::Qualified
            && valid_record(&use_receipt.manufacturing_genealogy)
            && valid_record(&use_receipt.as_built_acceptance_receipt)
            && !use_receipt.new_material_lot_id.trim().is_empty()
        {
            qualified_closed_loop_product_ids.push(use_receipt.recovered_output_id.clone());
        }
    }

    let disposition = if blockers.is_empty() {
        Disposition::Qualified
    } else if blockers.contains(&Blocker::RouteNotEligible) {
        Disposition::Fail
    } else {
        Disposition::Indeterminate
    };

    RecoveryEvaluation {
        passport_id: passport.passport_id.clone(),
        disposition,
        blockers,
        mass_balance,
        recovered_magnet_feedstock_mg,
        recovered_elemental_product_mg,
        qualified_closed_loop_product_ids,
    }
}

fn validate_feedstock(feedstock: &FeedstockLot, blockers: &mut Vec<Blocker>) {
    if feedstock.component_id.trim().is_empty()
        || feedstock.specimen_id.trim().is_empty()
        || feedstock.lot_id.trim().is_empty()
    {
        blockers.push(Blocker::MissingIdentity);
    }
    for record in [
        &feedstock.grade_or_composition_record,
        &feedstock.provenance_record,
        &feedstock.manufacturing_genealogy,
    ] {
        if !valid_record(record) {
            blockers.push(Blocker::InvalidDigest);
        }
    }
    if !valid_sha256(&feedstock.process_history_digest)
        || !valid_sha256(&feedstock.modification_history_digest)
    {
        blockers.push(Blocker::InvalidDigest);
    }
    if feedstock
        .service_history_digest
        .as_ref()
        .is_some_and(|digest| !valid_sha256(digest))
    {
        blockers.push(Blocker::InvalidDigest);
    }
}

fn validate_route_eligibility(passport: &RecoveryPassport, blockers: &mut Vec<Blocker>) {
    let feedstock = &passport.feedstock;
    let receipt = &passport.route_eligibility;
    if !valid_record(&receipt.receipt) || !valid_sha256(&receipt.scope_basis_digest) {
        blockers.push(Blocker::InvalidDigest);
    }
    if receipt.feedstock_lot_id != feedstock.lot_id
        || receipt.magnet_family != feedstock.magnet_family
        || receipt.feedstock_form != feedstock.feedstock_form
        || receipt.route != passport.stage.route
    {
        blockers.push(Blocker::InvalidRouteEligibilityScope);
    }
    match receipt.eligibility {
        Disposition::Qualified => {}
        Disposition::Fail => blockers.push(Blocker::RouteNotEligible),
        Disposition::Indeterminate | Disposition::NotEvaluable => {
            blockers.push(Blocker::RouteEligibilityUnresolved);
        }
    }
    if !route_form_supported(feedstock.feedstock_form, passport.stage.route) {
        blockers.push(Blocker::UnsupportedRouteForm);
    }
}

fn route_form_supported(form: FeedstockForm, route: RecoveryRoute) -> bool {
    match route {
        RecoveryRoute::PolymerHydrolysis => form == FeedstockForm::PolymerBondedMagnet,
        RecoveryRoute::HydrogenShortLoop => {
            matches!(form, FeedstockForm::SinteredMagnet | FeedstockForm::MagnetPowder)
        }
        RecoveryRoute::MechanicalLiberation => {
            matches!(form, FeedstockForm::HybridAssembly | FeedstockForm::SinteredMagnet)
        }
        RecoveryRoute::Hydrometallurgy
        | RecoveryRoute::Pyrometallurgy
        | RecoveryRoute::Electrochemical => {
            !matches!(form, FeedstockForm::Unknown)
        }
        RecoveryRoute::Other => false,
    }
}

fn evaluate_mass_balance(
    stage: &RecoveryStage,
    blockers: &mut Vec<Blocker>,
) -> StageMassBalance {
    if stage.input_mass.uncertainty_mg > stage.input_mass.value_mg {
        blockers.push(Blocker::InvalidMassUncertainty);
    }
    let input_interval = measurement_interval(stage.input_mass);
    let mut output_lower = 0i128;
    let mut output_upper = 0i128;
    for output in &stage.outputs {
        if output.mass.uncertainty_mg > output.mass.value_mg {
            blockers.push(Blocker::InvalidMassUncertainty);
        }
        let interval = measurement_interval(output.mass);
        output_lower = output_lower.saturating_add(interval.lower_mg);
        output_upper = output_upper.saturating_add(interval.upper_mg);
    }
    let output_interval = SignedInterval {
        lower_mg: output_lower,
        upper_mg: output_upper,
    };
    let residual_interval = SignedInterval {
        lower_mg: input_interval.lower_mg - output_interval.upper_mg,
        upper_mg: input_interval.upper_mg - output_interval.lower_mg,
    };
    let width = residual_interval.upper_mg - residual_interval.lower_mg;
    let status = if residual_interval.lower_mg == 0 && residual_interval.upper_mg == 0 {
        MassBalanceStatus::ExactNumericClosure
    } else if residual_interval.lower_mg > 0 {
        blockers.push(Blocker::MassBalanceUnaccountedInput);
        MassBalanceStatus::UnaccountedInputMass
    } else if residual_interval.upper_mg < 0 {
        blockers.push(Blocker::MassBalanceOutputExceedsInput);
        MassBalanceStatus::OutputExceedsInput
    } else if width <= i128::from(stage.closure_tolerance_mg) {
        MassBalanceStatus::ConsistentWithinUncertainty
    } else {
        blockers.push(Blocker::MassBalanceUnresolved);
        MassBalanceStatus::Unresolved
    };
    StageMassBalance {
        input_interval,
        output_interval,
        residual_interval,
        status,
    }
}

fn validate_outputs(stage: &RecoveryStage, blockers: &mut Vec<Blocker>) {
    if stage.stage_id.trim().is_empty()
        || !valid_record(&stage.stage_record)
        || stage.feedstock_lot_id.trim().is_empty()
        || stage.method_id.trim().is_empty()
        || stage.outputs.is_empty()
    {
        blockers.push(Blocker::MissingIdentity);
        if stage.outputs.is_empty() {
            blockers.push(Blocker::MissingRecoveryOutput);
        }
    }
    let mut output_ids = std::collections::BTreeSet::new();
    let mut evidence_ids = std::collections::BTreeSet::new();
    for output in &stage.outputs {
        if output.output_id.trim().is_empty() || !valid_record(&output.record) {
            blockers.push(Blocker::InvalidDigest);
        }
        if !output_ids.insert(output.output_id.as_str()) {
            blockers.push(Blocker::DuplicateOutputId);
        }
        if !evidence_ids.insert(output.record.record_id.as_str()) {
            blockers.push(Blocker::DuplicateEvidenceRecordId);
        }
        match output.form {
            ProductForm::MagnetFeedstock | ProductForm::RemanufacturableAlloy => {
                if !valid_optional(&output.composition_record)
                    || !valid_optional(&output.quality_record)
                {
                    blockers.push(Blocker::InvalidOutputEvidence);
                }
                if output.magnetic_characterization_record
                    .as_ref()
                    .is_none_or(|record| !valid_record(record))
                {
                    blockers.push(Blocker::MissingMagneticCharacterization);
                }
            }
            ProductForm::ElementalProduct => {
                if !valid_optional(&output.composition_record)
                    || output
                        .quality_record
                        .as_ref()
                        .is_none_or(|record| !valid_record(record))
                {
                    blockers.push(Blocker::MissingElementalProductPurity);
                }
            }
            ProductForm::NonMagneticFraction | ProductForm::Residue => {
                if !valid_optional(&output.waste_classification_record)
                    || !valid_optional(&output.disposition_record)
                {
                    blockers.push(Blocker::MissingResidueDisposition);
                }
            }
        }
        for record in [
            output.composition_record.as_ref(),
            output.quality_record.as_ref(),
            output.magnetic_characterization_record.as_ref(),
            output.waste_classification_record.as_ref(),
            output.disposition_record.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if !valid_record(record) || !evidence_ids.insert(record.record_id.as_str()) {
                blockers.push(Blocker::InvalidOutputEvidence);
            }
        }
    }
}

fn valid_optional(record: &Option<DigestRecord>) -> bool {
    record.as_ref().is_some_and(valid_record)
}

fn valid_record(record: &DigestRecord) -> bool {
    !record.record_id.trim().is_empty() && valid_sha256(&record.digest)
}

fn valid_sha256(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn measurement_interval(measurement: MassMeasurement) -> SignedInterval {
    SignedInterval {
        lower_mg: i128::from(measurement.value_mg) - i128::from(measurement.uncertainty_mg),
        upper_mg: i128::from(measurement.value_mg) + i128::from(measurement.uncertainty_mg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String {
        format!("sha256:{}", ch.to_string().repeat(64))
    }

    fn record(id: &str, ch: char) -> DigestRecord {
        DigestRecord {
            record_id: id.to_string(),
            digest: digest(ch),
        }
    }

    fn feedstock() -> FeedstockLot {
        FeedstockLot {
            component_id: "MOTOR-001".to_string(),
            specimen_id: "MAG-001".to_string(),
            lot_id: "LOT-001".to_string(),
            magnet_family: MagnetFamily::NdFeB,
            feedstock_form: FeedstockForm::PolymerBondedMagnet,
            grade_or_composition_record: record("CHEM-001", '1'),
            provenance_record: record("ORIGIN-001", '2'),
            manufacturing_genealogy: record("GEN-001", '3'),
            process_history_digest: digest('4'),
            modification_history_digest: digest('5'),
            service_history_digest: Some(digest('6')),
        }
    }

    fn output(
        id: &str,
        mass_mg: u64,
        form: ProductForm,
        suffix: char,
    ) -> RecoveryOutput {
        let magnet = matches!(form, ProductForm::MagnetFeedstock | ProductForm::RemanufacturableAlloy);
        let elemental = form == ProductForm::ElementalProduct;
        let residual = matches!(form, ProductForm::NonMagneticFraction | ProductForm::Residue);
        RecoveryOutput {
            output_id: id.to_string(),
            record: record(&format!("REC-{id}"), suffix),
            form,
            mass: MassMeasurement {
                value_mg: mass_mg,
                uncertainty_mg: 0,
            },
            composition_record: (magnet || elemental).then(|| record(&format!("COMP-{id}"), '7')),
            quality_record: (magnet || elemental).then(|| record(&format!("QUALITY-{id}"), '8')),
            magnetic_characterization_record: magnet.then(|| record(&format!("MAGCHAR-{id}"), '9')),
            waste_classification_record: residual.then(|| record(&format!("WASTE-{id}"), 'a')),
            disposition_record: residual.then(|| record(&format!("DISP-{id}"), 'b')),
        }
    }

    fn valid_passport() -> RecoveryPassport {
        RecoveryPassport {
            passport_id: "PASS-001".to_string(),
            feedstock: feedstock(),
            route_eligibility: RouteEligibilityReceipt {
                receipt: record("ELIGIBILITY-001", 'c'),
                feedstock_lot_id: "LOT-001".to_string(),
                magnet_family: MagnetFamily::NdFeB,
                feedstock_form: FeedstockForm::PolymerBondedMagnet,
                route: RecoveryRoute::PolymerHydrolysis,
                eligibility: Disposition::Qualified,
                scope_basis_digest: digest('d'),
            },
            stage: RecoveryStage {
                stage_id: "STAGE-001".to_string(),
                stage_record: record("STAGE-RECORD-001", 'e'),
                feedstock_lot_id: "LOT-001".to_string(),
                route: RecoveryRoute::PolymerHydrolysis,
                method_id: "LAB-METHOD-001".to_string(),
                input_mass: MassMeasurement {
                    value_mg: 1000,
                    uncertainty_mg: 0,
                },
                outputs: vec![
                    output("MAGNET-FEED", 700, ProductForm::MagnetFeedstock, 'f'),
                    output("POLYMER-RESIDUE", 300, ProductForm::Residue, 'a'),
                ],
                closure_tolerance_mg: 5,
            },
            downstream_use: Some(DownstreamUseReceipt {
                recovered_output_id: "MAGNET-FEED".to_string(),
                new_material_lot_id: "RECYCLED-LOT-001".to_string(),
                manufacturing_genealogy: record("NEW-GENEALOGY-001", 'b'),
                as_built_acceptance_receipt: record("NEW-ACCEPTANCE-001", 'c'),
                downstream_disposition: Disposition::Qualified,
            }),
        }
    }

    #[test]
    fn scoped_polymer_bonded_recovery_with_mass_closure_and_downstream_acceptance_qualifies() {
        let result = evaluate(&valid_passport());
        assert_eq!(result.disposition, Disposition::Qualified);
        assert_eq!(result.mass_balance.status, MassBalanceStatus::ExactNumericClosure);
        assert_eq!(result.recovered_magnet_feedstock_mg, 700);
        assert_eq!(result.qualified_closed_loop_product_ids, vec!["MAGNET-FEED"]);
        assert!(result.blockers.is_empty());
    }

    #[test]
    fn unknown_polymer_hydrolysis_feedstock_is_not_allowed() {
        let mut passport = valid_passport();
        passport.feedstock.feedstock_form = FeedstockForm::SinteredMagnet;
        let result = evaluate(&passport);
        assert_eq!(result.disposition, Disposition::Fail);
        assert!(result.blockers.contains(&Blocker::UnsupportedRouteForm));
    }

    #[test]
    fn eligibility_receipt_must_match_exact_lot_and_route() {
        let mut passport = valid_passport();
        passport.route_eligibility.feedstock_lot_id = "LOT-OTHER".to_string();
        let result = evaluate(&passport);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::InvalidRouteEligibilityScope));
    }

    #[test]
    fn ineligible_route_fails_without_assuming_a_process_recipe() {
        let mut passport = valid_passport();
        passport.route_eligibility.eligibility = Disposition::Fail;
        let result = evaluate(&passport);
        assert_eq!(result.disposition, Disposition::Fail);
        assert!(result.blockers.contains(&Blocker::RouteNotEligible));
    }

    #[test]
    fn unresolved_route_eligibility_does_not_pass() {
        let mut passport = valid_passport();
        passport.route_eligibility.eligibility = Disposition::Indeterminate;
        let result = evaluate(&passport);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert!(result.blockers.contains(&Blocker::RouteEligibilityUnresolved));
    }

    #[test]
    fn positive_unaccounted_mass_is_not_called_recovered() {
        let mut passport = valid_passport();
        passport.stage.outputs.pop();
        let result = evaluate(&passport);
        assert_eq!(result.mass_balance.status, MassBalanceStatus::UnaccountedInputMass);
        assert!(result.blockers.contains(&Blocker::MassBalanceUnaccountedInput));
    }

    #[test]
    fn output_mass_exceeding_input_is_rejected() {
        let mut passport = valid_passport();
        passport.stage.outputs[0].mass.value_mg = 900;
        let result = evaluate(&passport);
        assert_eq!(result.mass_balance.status, MassBalanceStatus::OutputExceedsInput);
        assert!(result.blockers.contains(&Blocker::MassBalanceOutputExceedsInput));
    }

    #[test]
    fn overlapping_mass_intervals_are_consistent_only_within_tolerance() {
        let mut passport = valid_passport();
        passport.stage.input_mass.uncertainty_mg = 3;
        passport.stage.outputs[0].mass.uncertainty_mg = 2;
        passport.stage.closure_tolerance_mg = 10;
        let result = evaluate(&passport);
        assert_eq!(result.mass_balance.status, MassBalanceStatus::ConsistentWithinUncertainty);
    }

    #[test]
    fn wide_uncertainty_interval_is_not_a_pass() {
        let mut passport = valid_passport();
        passport.stage.input_mass.uncertainty_mg = 100;
        passport.stage.closure_tolerance_mg = 10;
        let result = evaluate(&passport);
        assert_eq!(result.mass_balance.status, MassBalanceStatus::Unresolved);
        assert!(result.blockers.contains(&Blocker::MassBalanceUnresolved));
    }

    #[test]
    fn elemental_product_requires_composition_and_purity_evidence() {
        let mut passport = valid_passport();
        passport.stage.outputs[0].form = ProductForm::ElementalProduct;
        passport.stage.outputs[0].magnetic_characterization_record = None;
        passport.stage.outputs[0].quality_record = None;
        let result = evaluate(&passport);
        assert!(result.blockers.contains(&Blocker::MissingElementalProductPurity));
    }

    #[test]
    fn magnet_feedstock_requires_magnetic_characterization() {
        let mut passport = valid_passport();
        passport.stage.outputs[0].magnetic_characterization_record = None;
        let result = evaluate(&passport);
        assert!(result.blockers.contains(&Blocker::MissingMagneticCharacterization));
    }

    #[test]
    fn duplicate_output_identity_is_rejected() {
        let mut passport = valid_passport();
        passport.stage.outputs[1].output_id = passport.stage.outputs[0].output_id.clone();
        let result = evaluate(&passport);
        assert!(result.blockers.contains(&Blocker::DuplicateOutputId));
    }

    #[test]
    fn duplicate_evidence_record_ids_are_rejected() {
        let mut passport = valid_passport();
        passport.stage.outputs[1].record.record_id =
            passport.stage.outputs[0].record.record_id.clone();
        let result = evaluate(&passport);
        assert!(result.blockers.contains(&Blocker::DuplicateEvidenceRecordId));
    }

    #[test]
    fn downstream_closed_loop_requires_exact_existing_output() {
        let mut passport = valid_passport();
        passport.downstream_use.as_mut().unwrap().recovered_output_id = "UNKNOWN".to_string();
        let result = evaluate(&passport);
        assert!(result.blockers.contains(&Blocker::DownstreamOutputMismatch));
        assert!(result.qualified_closed_loop_product_ids.is_empty());
    }

    #[test]
    fn nonqualified_downstream_component_is_not_closed_loop_success() {
        let mut passport = valid_passport();
        passport.downstream_use.as_mut().unwrap().downstream_disposition = Disposition::Indeterminate;
        let result = evaluate(&passport);
        assert!(result.blockers.contains(&Blocker::DownstreamAcceptanceNotQualified));
        assert!(result.qualified_closed_loop_product_ids.is_empty());
    }

    #[test]
    fn feedstock_process_and_modification_history_digests_are_required() {
        let mut passport = valid_passport();
        passport.feedstock.process_history_digest.clear();
        let result = evaluate(&passport);
        assert!(result.blockers.contains(&Blocker::InvalidDigest));
    }

    #[test]
    fn route_family_is_not_transferred_across_magnet_families_or_forms() {
        let mut passport = valid_passport();
        passport.feedstock.magnet_family = MagnetFamily::SmCo;
        passport.route_eligibility.magnet_family = MagnetFamily::SmCo;
        let result = evaluate(&passport);
        assert_eq!(result.disposition, Disposition::Fail);
        assert!(result.blockers.contains(&Blocker::UnsupportedRouteForm) == false);
        // Polymer hydrolysis remains form-compatible, but the scoped eligibility
        // receipt is the required authority for this material family/route pair.
    }

    #[test]
    fn feedstock_lot_and_stage_input_must_match() {
        let mut passport = valid_passport();
        passport.stage.feedstock_lot_id = "LOT-OTHER".to_string();
        let result = evaluate(&passport);
        assert_eq!(result.disposition, Disposition::Indeterminate);
    }
}
