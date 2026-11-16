#![forbid(unsafe_code)]
//! Dependency-free reference kernels for GeoBio Foundry.
//!
//! This crate deliberately covers evidence-aware material/energy accounting.
//! It is not a bioprocess model, geomechanics solver, pressure-vessel solver,
//! life-support controller, or certification tool.
//!
//! Units are fixed by field names: elemental and material mass is kg, and
//! energy is J. Unknown composition is not treated as zero. Correlation groups
//! are an explicit first-order approximation: uncertainties within one named
//! group are treated as perfectly correlated; different groups and ungrouped
//! measurements are treated as independent.

pub mod pressure;
pub mod subterra;

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceState {
    Assumed,
    Modelled,
    Measured,
    IndependentlyReproduced,
    QualifiedForDeclaredEnvelope,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OriginClass {
    LocalMeasured,
    LocalInferred,
    Imported,
    Recycled,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Measurement {
    pub value: f64,
    /// One-standard-deviation uncertainty. None means the uncertainty is
    /// unavailable; it does not mean zero uncertainty.
    pub standard_uncertainty: Option<f64>,
    /// Measurements sharing a non-empty group are treated as perfectly
    /// correlated in kernel v0.2. This is not a general covariance model.
    pub correlation_group: Option<String>,
}

impl Measurement {
    pub fn exact(value: f64) -> Self {
        Self {
            value,
            standard_uncertainty: Some(0.0),
            correlation_group: None,
        }
    }

    pub fn measured(
        value: f64,
        standard_uncertainty: f64,
        correlation_group: Option<String>,
    ) -> Self {
        Self {
            value,
            standard_uncertainty: Some(standard_uncertainty),
            correlation_group,
        }
    }

    pub fn unquantified_uncertainty(value: f64) -> Self {
        Self {
            value,
            standard_uncertainty: None,
            correlation_group: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ElementMass {
    /// Chemical symbol or stable ontology identifier, e.g. "Fe".
    pub element: String,
    /// Elemental mass, not compound mass.
    pub mass_kg: Measurement,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaterialFlow {
    pub flow_id: String,
    pub material_id: String,
    pub origin: OriginClass,
    pub mass_kg: Measurement,
    /// True only when all constituents are represented in the element list.
    /// If false, an unlisted element is unknown, not zero.
    pub composition_complete: bool,
    pub elements: Vec<ElementMass>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Provenance {
    pub producer: String,
    pub method_id: String,
    pub observed_at: String,
    /// SHA-256 digest of frozen source inputs.
    pub input_sha256: String,
    /// Optional SHA-256 digest of model/configuration.
    pub model_sha256: Option<String>,
    /// Required for independently reproduced and qualified claims.
    pub independent_witness: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnergyLedger {
    /// Gross energy consumed. Do not net recovered energy against this value.
    pub gross_consumed_j: Measurement,
    /// Recovered energy reported separately, if measured.
    pub recovered_j: Option<Measurement>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResourceProcess {
    pub process_id: String,
    /// Explicit functional unit, e.g. "1 kg on-spec oxygen at outlet".
    pub functional_unit: String,
    pub primary_product_flow_id: String,
    pub evidence_state: EvidenceState,
    pub inputs: Vec<MaterialFlow>,
    pub products: Vec<MaterialFlow>,
    pub waste_streams: Vec<MaterialFlow>,
    pub energy: EnergyLedger,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BalanceStatus {
    WithinTolerance,
    OutsideTolerance,
    Unquantified,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ElementBalance {
    pub element: String,
    pub input_kg: f64,
    pub output_kg: f64,
    /// Known input minus known output. When status is Unquantified this is a
    /// partial residual, not a complete inventory residual.
    pub known_residual_kg: f64,
    pub combined_standard_uncertainty_kg: Option<f64>,
    pub status: BalanceStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ElementalBalanceReport {
    pub element_balances: Vec<ElementBalance>,
    pub inventory_complete: bool,
    pub overall_status: BalanceStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OriginMassIntensity {
    pub local_measured_kg_per_kg_product: f64,
    pub local_measured_standard_uncertainty: Option<f64>,
    pub local_inferred_kg_per_kg_product: f64,
    pub local_inferred_standard_uncertainty: Option<f64>,
    pub imported_kg_per_kg_product: f64,
    pub imported_standard_uncertainty: Option<f64>,
    pub recycled_kg_per_kg_product: f64,
    pub recycled_standard_uncertainty: Option<f64>,
    pub unknown_origin_kg_per_kg_product: f64,
    pub unknown_origin_standard_uncertainty: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResourceIntensityReport {
    pub primary_product_mass_kg: f64,
    pub gross_energy_j_per_kg_product: f64,
    pub gross_energy_standard_uncertainty: Option<f64>,
    /// Kept separate from gross energy; not netted.
    pub recovered_energy_j_per_kg_product: Option<f64>,
    pub recovered_energy_standard_uncertainty: Option<f64>,
    pub input_mass: OriginMassIntensity,
    pub all_flow_compositions_complete: bool,
    /// True only when every returned ratio has propagated standard uncertainty.
    pub uncertainty_propagated: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MorphometryError {
    InvalidProfile(Vec<String>),
    NumericalOverflow(String),
}

impl std::fmt::Display for MorphometryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidProfile(errors) => {
                write!(f, "invalid subterranean profile: {}", errors.join("; "))
            }
            Self::NumericalOverflow(context) => {
                write!(f, "numerical overflow while calculating {context}")
            }
        }
    }
}

impl std::error::Error for MorphometryError {}

#[derive(Clone, Debug, PartialEq)]
pub enum KernelError {
    InvalidTolerance,
    InvalidRelativeTolerance,
    InvalidZScore,
    InvalidProcess(Vec<String>),
    MissingPrimaryProduct,
    ZeroPrimaryProductMass,
    NumericalOverflow(String),
}

impl std::fmt::Display for KernelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTolerance => write!(f, "absolute tolerance must be finite and >= 0"),
            Self::InvalidRelativeTolerance => {
                write!(f, "relative tolerance must be finite and >= 0")
            }
            Self::InvalidZScore => write!(f, "z-score must be finite and >= 0"),
            Self::InvalidProcess(errors) => {
                write!(f, "invalid process: {}", errors.join("; "))
            }
            Self::MissingPrimaryProduct => {
                write!(f, "primary product flow is not present in products")
            }
            Self::ZeroPrimaryProductMass => {
                write!(f, "primary product mass must be greater than zero")
            }
            Self::NumericalOverflow(context) => {
                write!(f, "numerical overflow while calculating {context}")
            }
        }
    }
}

impl std::error::Error for KernelError {}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_measurement(
    measurement: &Measurement,
    field: &str,
    errors: &mut Vec<String>,
) {
    if !measurement.value.is_finite() || measurement.value < 0.0 {
        errors.push(format!("{field}.value must be finite and >= 0"));
    }
    if let Some(uncertainty) = measurement.standard_uncertainty {
        if !uncertainty.is_finite() || uncertainty < 0.0 {
            errors.push(format!(
                "{field}.standard_uncertainty must be finite and >= 0"
            ));
        }
    }
    if let Some(group) = &measurement.correlation_group {
        if !nonempty(group) {
            errors.push(format!("{field}.correlation_group must not be empty"));
        }
    }
}

fn validate_flow(
    flow: &MaterialFlow,
    group_name: &str,
    index: usize,
    composition_tolerance_kg: f64,
    errors: &mut Vec<String>,
) {
    let prefix = format!("{group_name}[{index}]");

    if !nonempty(&flow.flow_id) {
        errors.push(format!("{prefix}.flow_id must not be empty"));
    }
    if !nonempty(&flow.material_id) {
        errors.push(format!("{prefix}.material_id must not be empty"));
    }

    validate_measurement(&flow.mass_kg, &format!("{prefix}.mass_kg"), errors);

    if flow.elements.is_empty() {
        errors.push(format!("{prefix}.elements must not be empty"));
    }

    let mut seen = BTreeSet::new();
    let mut element_mass_sum = 0.0_f64;

    for (element_index, element) in flow.elements.iter().enumerate() {
        let element_prefix = format!("{prefix}.elements[{element_index}]");

        if !nonempty(&element.element) {
            errors.push(format!("{element_prefix}.element must not be empty"));
        }
        if !seen.insert(element.element.as_str()) {
            errors.push(format!(
                "{prefix} contains duplicate element {}",
                element.element
            ));
        }

        validate_measurement(
            &element.mass_kg,
            &format!("{element_prefix}.mass_kg"),
            errors,
        );

        element_mass_sum += element.mass_kg.value;
        if !element_mass_sum.is_finite() {
            errors.push(format!("{prefix} elemental mass sum overflowed"));
            break;
        }
    }

    if flow.mass_kg.value.is_finite()
        && flow.mass_kg.value >= 0.0
        && element_mass_sum.is_finite()
    {
        let unassigned_mass = flow.mass_kg.value - element_mass_sum;
        if flow.composition_complete {
            if unassigned_mass.abs() > composition_tolerance_kg {
                errors.push(format!(
                    "{prefix} is marked composition_complete but elemental mass differs from total mass by {unassigned_mass} kg"
                ));
            }
        } else if unassigned_mass < -composition_tolerance_kg {
            errors.push(format!(
                "{prefix} quantified elemental mass exceeds total material mass"
            ));
        }
    }
}

/// Validate the process record. For complete compositions, the sum of listed
/// elemental masses must equal total flow mass within the supplied tolerance.
/// For incomplete compositions, known elemental mass may be less than total
/// mass, but never greater.
pub fn validate_process(
    process: &ResourceProcess,
    composition_tolerance_kg: f64,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if !composition_tolerance_kg.is_finite() || composition_tolerance_kg < 0.0 {
        errors.push("composition_tolerance_kg must be finite and >= 0".to_string());
    }

    if !nonempty(&process.process_id) {
        errors.push("process_id must not be empty".to_string());
    }
    if !nonempty(&process.functional_unit) {
        errors.push("functional_unit must not be empty".to_string());
    }
    if !nonempty(&process.primary_product_flow_id) {
        errors.push("primary_product_flow_id must not be empty".to_string());
    }
    if process.inputs.is_empty() {
        errors.push("at least one input flow is required".to_string());
    }
    if process.products.is_empty() {
        errors.push("at least one product flow is required".to_string());
    }

    validate_measurement(
        &process.energy.gross_consumed_j,
        "energy.gross_consumed_j",
        &mut errors,
    );
    if let Some(recovered) = &process.energy.recovered_j {
        validate_measurement(recovered, "energy.recovered_j", &mut errors);
    }

    if !nonempty(&process.provenance.producer) {
        errors.push("provenance.producer must not be empty".to_string());
    }
    if !nonempty(&process.provenance.method_id) {
        errors.push("provenance.method_id must not be empty".to_string());
    }
    if !nonempty(&process.provenance.observed_at) {
        errors.push("provenance.observed_at must not be empty".to_string());
    }
    if !valid_sha256(&process.provenance.input_sha256) {
        errors.push("provenance.input_sha256 must be a 64-character SHA-256 hex digest".to_string());
    }
    if let Some(digest) = &process.provenance.model_sha256 {
        if !valid_sha256(digest) {
            errors.push("provenance.model_sha256 must be a 64-character SHA-256 hex digest".to_string());
        }
    }
    if matches!(
        process.evidence_state,
        EvidenceState::IndependentlyReproduced | EvidenceState::QualifiedForDeclaredEnvelope
    ) && !process
        .provenance
        .independent_witness
        .as_deref()
        .is_some_and(nonempty)
    {
        errors.push(
            "independent_witness is required for independently reproduced or qualified claims"
                .to_string(),
        );
    }

    let mut all_flow_ids = BTreeSet::new();
    for (group_name, flows) in [
        ("inputs", &process.inputs),
        ("products", &process.products),
        ("waste_streams", &process.waste_streams),
    ] {
        for (index, flow) in flows.iter().enumerate() {
            validate_flow(
                flow,
                group_name,
                index,
                composition_tolerance_kg,
                &mut errors,
            );
            if !all_flow_ids.insert(flow.flow_id.as_str()) {
                errors.push(format!("duplicate flow_id {}", flow.flow_id));
            }
        }
    }

    if !process
        .products
        .iter()
        .any(|flow| flow.flow_id == process.primary_product_flow_id)
    {
        errors.push("primary_product_flow_id does not identify a product flow".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn add_finite(
    current: f64,
    increment: f64,
    context: &str,
) -> Result<f64, KernelError> {
    let next = current + increment;
    if next.is_finite() {
        Ok(next)
    } else {
        Err(KernelError::NumericalOverflow(context.to_string()))
    }
}

/// Combine standard uncertainties for a linear residual. Unnamed measurements
/// are independent. Measurements with the same group are treated as fully
/// correlated: signed standard uncertainties are summed within each group,
/// and group variances are then combined as independent groups.
fn combined_uncertainty(
    terms: &[(f64, &Measurement)],
    context: &str,
) -> Result<Option<f64>, KernelError> {
    let mut independent_variance = 0.0_f64;
    let mut grouped_amplitudes = BTreeMap::<String, f64>::new();
    let mut complete = true;

    for (coefficient, measurement) in terms {
        let Some(standard_uncertainty) = measurement.standard_uncertainty else {
            complete = false;
            continue;
        };

        let signed_contribution = coefficient * standard_uncertainty;
        if let Some(group) = &measurement.correlation_group {
            let previous = grouped_amplitudes.get(group).copied().unwrap_or(0.0);
            let combined = add_finite(previous, signed_contribution, context)?;
            grouped_amplitudes.insert(group.clone(), combined);
        } else {
            let variance = signed_contribution * signed_contribution;
            independent_variance =
                add_finite(independent_variance, variance, context)?;
        }
    }

    if !complete {
        return Ok(None);
    }

    let mut total_variance = independent_variance;
    for amplitude in grouped_amplitudes.values() {
        total_variance = add_finite(
            total_variance,
            amplitude * amplitude,
            context,
        )?;
    }

    if !total_variance.is_finite() || total_variance < 0.0 {
        return Err(KernelError::NumericalOverflow(context.to_string()));
    }

    let result = total_variance.sqrt();
    if result.is_finite() {
        Ok(Some(result))
    } else {
        Err(KernelError::NumericalOverflow(context.to_string()))
    }
}

fn find_element<'a>(
    flow: &'a MaterialFlow,
    element: &str,
) -> Option<&'a Measurement> {
    flow.elements
        .iter()
        .find(|entry| entry.element == element)
        .map(|entry| &entry.mass_kg)
}

/// First-order uncertainty for A/B, where A is a sum of the supplied
/// numerator measurements and B is one measured denominator. Shared
/// correlation groups are propagated by the same fully-correlated-group
/// approximation as elemental_balance. This is not a general covariance
/// estimator.
fn ratio_standard_uncertainty(
    numerator_measurements: &[&Measurement],
    numerator_value: f64,
    denominator: &Measurement,
    denominator_value: f64,
    context: &str,
) -> Result<Option<f64>, KernelError> {
    if numerator_measurements.is_empty() {
        return if numerator_value == 0.0 {
            Ok(Some(0.0))
        } else {
            Err(KernelError::InvalidProcess(vec![
                "non-zero numerator has no source measurements".to_string(),
            ]))
        };
    }

    let numerator_coefficient = 1.0 / denominator_value;
    let denominator_coefficient =
        -numerator_value / (denominator_value * denominator_value);
    if !numerator_coefficient.is_finite() || !denominator_coefficient.is_finite() {
        return Err(KernelError::NumericalOverflow(context.to_string()));
    }

    let mut terms = Vec::with_capacity(numerator_measurements.len() + 1);
    for measurement in numerator_measurements {
        terms.push((numerator_coefficient, *measurement));
    }
    terms.push((denominator_coefficient, denominator));
    combined_uncertainty(&terms, context)
}

/// Calculate element-by-element material residuals. An unlisted element in a
/// composition-incomplete flow makes that element unquantified. For complete
/// flows, an absent element is known to be zero. Unknown composition is never
/// silently interpreted as zero.
pub fn elemental_balance(
    process: &ResourceProcess,
    absolute_tolerance_kg: f64,
    relative_tolerance: f64,
    z_score: f64,
) -> Result<ElementalBalanceReport, KernelError> {
    if !absolute_tolerance_kg.is_finite() || absolute_tolerance_kg < 0.0 {
        return Err(KernelError::InvalidTolerance);
    }
    if !relative_tolerance.is_finite() || relative_tolerance < 0.0 {
        return Err(KernelError::InvalidRelativeTolerance);
    }
    if !z_score.is_finite() || z_score < 0.0 {
        return Err(KernelError::InvalidZScore);
    }

    validate_process(process, absolute_tolerance_kg)
        .map_err(KernelError::InvalidProcess)?;

    let all_flows = process
        .inputs
        .iter()
        .chain(process.products.iter())
        .chain(process.waste_streams.iter());

    let inventory_complete = all_flows.clone().all(|flow| flow.composition_complete);

    let mut element_names = BTreeSet::<String>::new();
    for flow in all_flows {
        for entry in &flow.elements {
            element_names.insert(entry.element.clone());
        }
    }

    let mut balances = Vec::with_capacity(element_names.len());

    for element in element_names {
        let mut input_kg = 0.0_f64;
        let mut output_kg = 0.0_f64;
        let mut composition_known = true;
        let mut terms = Vec::<(f64, &Measurement)>::new();

        for flow in &process.inputs {
            match find_element(flow, &element) {
                Some(measurement) => {
                    input_kg = add_finite(
                        input_kg,
                        measurement.value,
                        &format!("input elemental mass for {element}"),
                    )?;
                    terms.push((1.0, measurement));
                }
                None if flow.composition_complete => {}
                None => composition_known = false,
            }
        }

        for flow in process.products.iter().chain(process.waste_streams.iter()) {
            match find_element(flow, &element) {
                Some(measurement) => {
                    output_kg = add_finite(
                        output_kg,
                        measurement.value,
                        &format!("output elemental mass for {element}"),
                    )?;
                    terms.push((-1.0, measurement));
                }
                None if flow.composition_complete => {}
                None => composition_known = false,
            }
        }

        let known_residual_kg = input_kg - output_kg;
        if !known_residual_kg.is_finite() {
            return Err(KernelError::NumericalOverflow(format!(
                "elemental residual for {element}"
            )));
        }

        let uncertainty = if composition_known {
            combined_uncertainty(&terms, &format!("uncertainty for {element}"))?
        } else {
            None
        };

        let status = match uncertainty {
            Some(sigma) => {
                let scale = input_kg.max(output_kg);
                let threshold = absolute_tolerance_kg
                    + relative_tolerance * scale
                    + z_score * sigma;
                if !threshold.is_finite() {
                    return Err(KernelError::NumericalOverflow(format!(
                        "balance threshold for {element}"
                    )));
                }
                if known_residual_kg.abs() <= threshold {
                    BalanceStatus::WithinTolerance
                } else {
                    BalanceStatus::OutsideTolerance
                }
            }
            None => BalanceStatus::Unquantified,
        };

        balances.push(ElementBalance {
            element,
            input_kg,
            output_kg,
            known_residual_kg,
            combined_standard_uncertainty_kg: uncertainty,
            status,
        });
    }

    let overall_status = if balances
        .iter()
        .any(|entry| entry.status == BalanceStatus::OutsideTolerance)
    {
        BalanceStatus::OutsideTolerance
    } else if !inventory_complete
        || balances
            .iter()
            .any(|entry| entry.status == BalanceStatus::Unquantified)
    {
        BalanceStatus::Unquantified
    } else {
        BalanceStatus::WithinTolerance
    };

    Ok(ElementalBalanceReport {
        element_balances: balances,
        inventory_complete,
        overall_status,
    })
}

/// Calculate nominal process-resource intensities per kilogram of the declared
/// primary product. Gross consumed and recovered energy remain separate. The
/// output is a point estimate only: ratio uncertainty is not propagated in
/// kernel v0.2, and inventory completeness is explicitly reported.
pub fn resource_intensity(
    process: &ResourceProcess,
    composition_tolerance_kg: f64,
) -> Result<ResourceIntensityReport, KernelError> {
    validate_process(process, composition_tolerance_kg)
        .map_err(KernelError::InvalidProcess)?;

    let primary = process
        .products
        .iter()
        .find(|flow| flow.flow_id == process.primary_product_flow_id)
        .ok_or(KernelError::MissingPrimaryProduct)?;

    let product_mass_kg = primary.mass_kg.value;
    if product_mass_kg <= 0.0 {
        return Err(KernelError::ZeroPrimaryProductMass);
    }

    let mut origin_mass = BTreeMap::<OriginClass, f64>::new();
    let mut origin_measurements = BTreeMap::<OriginClass, Vec<&Measurement>>::new();
    for flow in &process.inputs {
        let previous = origin_mass.get(&flow.origin).copied().unwrap_or(0.0);
        let combined = add_finite(
            previous,
            flow.mass_kg.value,
            "input mass by origin",
        )?;
        origin_mass.insert(flow.origin, combined);
        origin_measurements
            .entry(flow.origin)
            .or_default()
            .push(&flow.mass_kg);
    }

    let per_product_kg = |origin: OriginClass| {
        origin_mass.get(&origin).copied().unwrap_or(0.0) / product_mass_kg
    };
    let ratio_uncertainty = |origin: OriginClass| -> Result<Option<f64>, KernelError> {
        let numerator = origin_mass.get(&origin).copied().unwrap_or(0.0);
        let measurements = origin_measurements
            .get(&origin)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        ratio_standard_uncertainty(
            measurements,
            numerator,
            &primary.mass_kg,
            product_mass_kg,
            "feedstock-origin intensity uncertainty",
        )
    };

    let origin_intensity = OriginMassIntensity {
        local_measured_kg_per_kg_product: per_product_kg(OriginClass::LocalMeasured),
        local_measured_standard_uncertainty: ratio_uncertainty(OriginClass::LocalMeasured)?,
        local_inferred_kg_per_kg_product: per_product_kg(OriginClass::LocalInferred),
        local_inferred_standard_uncertainty: ratio_uncertainty(OriginClass::LocalInferred)?,
        imported_kg_per_kg_product: per_product_kg(OriginClass::Imported),
        imported_standard_uncertainty: ratio_uncertainty(OriginClass::Imported)?,
        recycled_kg_per_kg_product: per_product_kg(OriginClass::Recycled),
        recycled_standard_uncertainty: ratio_uncertainty(OriginClass::Recycled)?,
        unknown_origin_kg_per_kg_product: per_product_kg(OriginClass::Unknown),
        unknown_origin_standard_uncertainty: ratio_uncertainty(OriginClass::Unknown)?,
    };

    let gross_energy_measurements = [&process.energy.gross_consumed_j];
    let gross_energy_j_per_kg_product =
        process.energy.gross_consumed_j.value / product_mass_kg;
    let gross_energy_standard_uncertainty = ratio_standard_uncertainty(
        &gross_energy_measurements,
        process.energy.gross_consumed_j.value,
        &primary.mass_kg,
        product_mass_kg,
        "gross-energy intensity uncertainty",
    )?;

    let (recovered_energy_j_per_kg_product, recovered_energy_standard_uncertainty) =
        match &process.energy.recovered_j {
            Some(recovered) => {
                let measurements = [recovered];
                let value = recovered.value / product_mass_kg;
                let uncertainty = ratio_standard_uncertainty(
                    &measurements,
                    recovered.value,
                    &primary.mass_kg,
                    product_mass_kg,
                    "recovered-energy intensity uncertainty",
                )?;
                (Some(value), uncertainty)
            }
            None => (None, None),
        };

    let intensity_values = [
        origin_intensity.local_measured_kg_per_kg_product,
        origin_intensity.local_inferred_kg_per_kg_product,
        origin_intensity.imported_kg_per_kg_product,
        origin_intensity.recycled_kg_per_kg_product,
        origin_intensity.unknown_origin_kg_per_kg_product,
        gross_energy_j_per_kg_product,
    ];
    if intensity_values.iter().any(|value| !value.is_finite())
        || recovered_energy_j_per_kg_product.is_some_and(|value| !value.is_finite())
    {
        return Err(KernelError::NumericalOverflow(
            "resource intensity ratio".to_string(),
        ));
    }

    let all_flow_compositions_complete = process
        .inputs
        .iter()
        .chain(process.products.iter())
        .chain(process.waste_streams.iter())
        .all(|flow| flow.composition_complete);

    let uncertainty_propagated = [
        origin_intensity.local_measured_standard_uncertainty,
        origin_intensity.local_inferred_standard_uncertainty,
        origin_intensity.imported_standard_uncertainty,
        origin_intensity.recycled_standard_uncertainty,
        origin_intensity.unknown_origin_standard_uncertainty,
        gross_energy_standard_uncertainty,
    ]
    .iter()
    .all(|uncertainty| uncertainty.is_some())
        && (process.energy.recovered_j.is_none()
            || recovered_energy_standard_uncertainty.is_some());

    Ok(ResourceIntensityReport {
        primary_product_mass_kg: product_mass_kg,
        gross_energy_j_per_kg_product,
        gross_energy_standard_uncertainty,
        recovered_energy_j_per_kg_product,
        recovered_energy_standard_uncertainty,
        input_mass: origin_intensity,
        all_flow_compositions_complete,
        uncertainty_propagated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measurement(value: f64, sigma: f64) -> Measurement {
        Measurement::measured(value, sigma, None)
    }

    fn element(symbol: &str, value: f64, sigma: f64) -> ElementMass {
        ElementMass {
            element: symbol.to_string(),
            mass_kg: measurement(value, sigma),
        }
    }

    fn flow(
        flow_id: &str,
        material_id: &str,
        origin: OriginClass,
        total_mass_kg: f64,
        elements: Vec<ElementMass>,
    ) -> MaterialFlow {
        MaterialFlow {
            flow_id: flow_id.to_string(),
            material_id: material_id.to_string(),
            origin,
            mass_kg: measurement(total_mass_kg, 0.0001),
            composition_complete: true,
            elements,
        }
    }

    fn water_electrolysis_fixture() -> ResourceProcess {
        ResourceProcess {
            process_id: "water-electrolysis-reference".to_string(),
            functional_unit: "1 kg water feed processed".to_string(),
            primary_product_flow_id: "oxygen-product".to_string(),
            evidence_state: EvidenceState::Modelled,
            inputs: vec![flow(
                "water-feed",
                "water",
                OriginClass::LocalMeasured,
                1.0,
                vec![
                    element("H", 0.111898, 0.0001),
                    element("O", 0.888102, 0.0001),
                ],
            )],
            products: vec![
                flow(
                    "hydrogen-product",
                    "hydrogen",
                    OriginClass::LocalMeasured,
                    0.111898,
                    vec![element("H", 0.111898, 0.0001)],
                ),
                flow(
                    "oxygen-product",
                    "oxygen",
                    OriginClass::LocalMeasured,
                    0.888102,
                    vec![element("O", 0.888102, 0.0001)],
                ),
            ],
            waste_streams: vec![],
            energy: EnergyLedger {
                gross_consumed_j: measurement(1_000.0, 1.0),
                recovered_j: Some(measurement(50.0, 1.0)),
            },
            provenance: Provenance {
                producer: "synthetic-fixture".to_string(),
                method_id: "stoichiometric-water-electrolysis-v0".to_string(),
                observed_at: "2026-11-16T00:00:00Z".to_string(),
                input_sha256: "a".repeat(64),
                model_sha256: Some("b".repeat(64)),
                independent_witness: None,
            },
        }
    }

    #[test]
    fn stoichiometric_fixture_balances_each_element() {
        let result = elemental_balance(&water_electrolysis_fixture(), 1e-9, 0.0, 3.0).unwrap();
        assert_eq!(result.element_balances.len(), 2);
        assert_eq!(result.overall_status, BalanceStatus::WithinTolerance);
        assert!(result
            .element_balances
            .iter()
            .all(|entry| entry.status == BalanceStatus::WithinTolerance));
        assert!(result
            .element_balances
            .iter()
            .all(|entry| entry.known_residual_kg.abs() < 1e-12));
    }

    #[test]
    fn missing_product_mass_is_detected() {
        let mut process = water_electrolysis_fixture();
        process.products[1].mass_kg.value = 0.80;
        process.products[1].elements[0].mass_kg.value = 0.80;

        let result = elemental_balance(&process, 1e-6, 0.0, 3.0).unwrap();
        let oxygen = result
            .element_balances
            .iter()
            .find(|entry| entry.element == "O")
            .unwrap();
        assert_eq!(oxygen.status, BalanceStatus::OutsideTolerance);
        assert!(oxygen.known_residual_kg > 0.08);
    }

    #[test]
    fn incomplete_inventory_cannot_pass_as_a_complete_process_balance() {
        let mut process = water_electrolysis_fixture();
        process.products[0].composition_complete = false;

        let result = elemental_balance(&process, 1e-6, 0.0, 3.0).unwrap();
        assert!(!result.inventory_complete);
        assert_eq!(result.overall_status, BalanceStatus::Unquantified);

        let hydrogen = result
            .element_balances
            .iter()
            .find(|entry| entry.element == "H")
            .unwrap();
        assert_eq!(hydrogen.status, BalanceStatus::WithinTolerance);

        let oxygen = result
            .element_balances
            .iter()
            .find(|entry| entry.element == "O")
            .unwrap();
        assert_eq!(oxygen.status, BalanceStatus::Unquantified);
    }

    #[test]
    fn missing_measurement_uncertainty_prevents_element_balance_qualification() {
        let mut process = water_electrolysis_fixture();
        process.inputs[0].elements[0].mass_kg.standard_uncertainty = None;

        let result = elemental_balance(&process, 1e-6, 0.0, 3.0).unwrap();
        let hydrogen = result
            .element_balances
            .iter()
            .find(|entry| entry.element == "H")
            .unwrap();
        assert_eq!(hydrogen.status, BalanceStatus::Unquantified);

        let oxygen = result
            .element_balances
            .iter()
            .find(|entry| entry.element == "O")
            .unwrap();
        assert_eq!(oxygen.status, BalanceStatus::WithinTolerance);
    }

    #[test]
    fn fully_correlated_measurement_errors_are_not_treated_as_independent() {
        let mut process = water_electrolysis_fixture();
        let correlated_input = Measurement::measured(
            1.0,
            0.1,
            Some("shared-assay-bias".to_string()),
        );
        let correlated_output = Measurement::measured(
            0.9,
            0.1,
            Some("shared-assay-bias".to_string()),
        );

        process.inputs = vec![MaterialFlow {
            flow_id: "iron-feed".to_string(),
            material_id: "iron-bearing-rock".to_string(),
            origin: OriginClass::LocalMeasured,
            mass_kg: correlated_input.clone(),
            composition_complete: true,
            elements: vec![ElementMass {
                element: "Fe".to_string(),
                mass_kg: correlated_input,
            }],
        }];
        process.products = vec![MaterialFlow {
            flow_id: "iron-product".to_string(),
            material_id: "iron".to_string(),
            origin: OriginClass::LocalMeasured,
            mass_kg: correlated_output.clone(),
            composition_complete: true,
            elements: vec![ElementMass {
                element: "Fe".to_string(),
                mass_kg: correlated_output,
            }],
        }];
        process.primary_product_flow_id = "iron-product".to_string();

        let result = elemental_balance(&process, 0.0, 0.0, 0.0).unwrap();
        let iron = &result.element_balances[0];
        assert_eq!(iron.element, "Fe");
        assert_eq!(iron.combined_standard_uncertainty_kg, Some(0.0));
        assert_eq!(iron.status, BalanceStatus::OutsideTolerance);
    }

    #[test]
    fn negative_mass_is_rejected() {
        let mut process = water_electrolysis_fixture();
        process.inputs[0].mass_kg.value = -0.1;
        assert!(validate_process(&process, 1e-6).is_err());
    }

    #[test]
    fn non_finite_element_mass_is_rejected() {
        let mut process = water_electrolysis_fixture();
        process.inputs[0].elements[0].mass_kg.value = f64::NAN;
        assert!(validate_process(&process, 1e-6).is_err());
    }

    #[test]
    fn aggregate_mass_overflow_is_rejected() {
        let mut process = water_electrolysis_fixture();

        let large_flow = flow(
            "large-iron-feed-a",
            "iron",
            OriginClass::Imported,
            1.0e308,
            vec![element("Fe", 1.0e308, 1.0)],
        );
        let mut second_large_flow = large_flow.clone();
        second_large_flow.flow_id = "large-iron-feed-b".to_string();

        process.inputs = vec![large_flow, second_large_flow];
        process.products = vec![flow(
            "iron-product",
            "iron",
            OriginClass::LocalMeasured,
            1.0,
            vec![element("Fe", 1.0, 0.1)],
        )];
        process.primary_product_flow_id = "iron-product".to_string();

        let result = elemental_balance(&process, 1e-6, 0.0, 3.0);
        assert!(matches!(result, Err(KernelError::NumericalOverflow(_))));
    }

    #[test]
    fn qualified_claim_requires_an_independent_witness() {
        let mut process = water_electrolysis_fixture();
        process.evidence_state = EvidenceState::QualifiedForDeclaredEnvelope;
        assert!(validate_process(&process, 1e-6).is_err());

        process.provenance.independent_witness =
            Some("independent-laboratory-record-001".to_string());
        assert!(validate_process(&process, 1e-6).is_ok());
    }

    #[test]
    fn malformed_input_digest_is_rejected() {
        let mut process = water_electrolysis_fixture();
        process.provenance.input_sha256 = "not-a-digest".to_string();
        assert!(validate_process(&process, 1e-6).is_err());
    }

    #[test]
    fn resource_intensity_propagates_independent_ratio_uncertainty() {
        let mut process = water_electrolysis_fixture();
        process.inputs[0].mass_kg = Measurement::measured(2.0, 0.2, None);
        process.inputs[0].elements = vec![
            element("H", 0.223796, 0.0001),
            element("O", 1.776204, 0.0001),
        ];
        process.inputs[0].composition_complete = false;
        process.products[1].mass_kg = Measurement::measured(1.0, 0.1, None);
        process.products[1].elements = vec![element("O", 1.0, 0.0001)];
        process.products[1].composition_complete = false;

        let report = resource_intensity(&process, 1e-6).unwrap();
        let expected = (0.2_f64.powi(2) + (2.0_f64 * 0.1).powi(2)).sqrt();
        assert!((report.input_mass.local_measured_standard_uncertainty.unwrap() - expected).abs() < 1e-12);
        assert!(report.gross_energy_standard_uncertainty.is_some());
        assert!(report.uncertainty_propagated);
    }

    #[test]
    fn shared_scale_error_can_cancel_in_a_ratio() {
        let mut process = water_electrolysis_fixture();
        process.inputs = vec![MaterialFlow {
            flow_id: "iron-feed".to_string(),
            material_id: "iron-bearing-rock".to_string(),
            origin: OriginClass::LocalMeasured,
            mass_kg: Measurement::measured(2.0, 0.2, Some("shared-scale".to_string())),
            composition_complete: true,
            elements: vec![element("Fe", 2.0, 0.0)],
        }];
        process.products = vec![MaterialFlow {
            flow_id: "iron-product".to_string(),
            material_id: "iron".to_string(),
            origin: OriginClass::LocalMeasured,
            mass_kg: Measurement::measured(1.0, 0.1, Some("shared-scale".to_string())),
            composition_complete: true,
            elements: vec![element("Fe", 1.0, 0.0)],
        }];
        process.primary_product_flow_id = "iron-product".to_string();

        let report = resource_intensity(&process, 1e-9).unwrap();
        assert!((report.input_mass.local_measured_kg_per_kg_product - 2.0).abs() < 1e-12);
        assert!(report.input_mass.local_measured_standard_uncertainty.unwrap().abs() < 1e-12);
    }

    #[test]
    fn missing_ratio_uncertainty_is_explicit() {
        let mut process = water_electrolysis_fixture();
        process.energy.gross_consumed_j.standard_uncertainty = None;
        let report = resource_intensity(&process, 1e-6).unwrap();
        assert!(report.gross_energy_standard_uncertainty.is_none());
        assert!(!report.uncertainty_propagated);
    }

    #[test]
    fn complete_composition_must_reconcile_to_total_flow_mass() {
        let mut process = water_electrolysis_fixture();
        process.inputs[0].elements[0].mass_kg.value = 0.20;
        assert!(validate_process(&process, 1e-6).is_err());
    }

    #[test]
    fn resource_intensity_ratio_overflow_is_rejected() {
        let mut process = water_electrolysis_fixture();
        process.products[1].mass_kg.value = 1.0e-308;
        process.products[1].elements[0].mass_kg.value = 1.0e-308;

        let result = resource_intensity(&process, 1e-6);
        assert!(matches!(result, Err(KernelError::NumericalOverflow(_))));
    }

    #[test]
    fn nominal_resource_intensity_keeps_recovered_energy_separate() {
        let report = resource_intensity(&water_electrolysis_fixture(), 1e-6).unwrap();
        assert!((report.primary_product_mass_kg - 0.888102).abs() < 1e-12);
        assert!((report.gross_energy_j_per_kg_product - (1_000.0 / 0.888102)).abs() < 1e-9);
        assert!((report.recovered_energy_j_per_kg_product.unwrap()
            - (50.0 / 0.888102))
            .abs()
            < 1e-9);
        assert!(report.gross_energy_standard_uncertainty.is_some());
        assert!(report.recovered_energy_standard_uncertainty.is_some());
        assert!(report.all_flow_compositions_complete);
        assert!(report.uncertainty_propagated);
    }
}
