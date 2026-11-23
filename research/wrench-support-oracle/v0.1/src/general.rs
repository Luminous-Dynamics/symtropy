// SPDX-License-Identifier: AGPL-3.0-or-later
//! General planar/non-planar contact wrench support using polyhedral friction cones.
//!
//! Unlike the v0.1 shared-normal disk model, this module supports varying contact
//! normals, per-contact anisotropic friction, and lower/upper normal-load fractions.
//! Exactness is for the declared finite polyhedral cone at each contact and the
//! fixed unit total normal-load allocation problem.

use std::collections::BTreeSet;

use super::{OracleError, WrenchDirection, stable_norm};

pub const SCHEMA_VERSION: &str = "polyhedral-contact-wrench-cone-v0.1";

#[derive(Clone, Debug, PartialEq)]
pub struct ContactFrame {
    pub id: String,
    pub position: [f64; 3],
    pub normal: [f64; 3],
    /// Caller-declared material tangent direction, projected into the contact plane.
    pub tangent_u_hint: [f64; 3],
    /// Friction semi-axis along the material tangent-u direction.
    pub friction_u: f64,
    /// Friction semi-axis along the deterministic local tangent-v direction.
    pub friction_v: f64,
    /// Minimum fraction of the fixed total normal load assigned to this contact.
    pub min_normal_fraction: f64,
    /// Maximum fraction of the fixed total normal load assigned to this contact.
    pub max_normal_fraction: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContactWrenchModel {
    pub contacts: Vec<ContactFrame>,
    /// Number of equally spaced rays in each polyhedral friction cone.
    pub cone_sides: usize,
    pub characteristic_length: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContactLoadAllocation {
    pub contact_id: String,
    pub normal_load_fraction: f64,
    pub selected_friction_ray: usize,
    pub support_per_unit_normal_load: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContactWrenchSupport {
    pub support_value: f64,
    pub allocations: Vec<ContactLoadAllocation>,
    pub cone_sides: usize,
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn normalize(value: [f64; 3]) -> Result<[f64; 3], OracleError> {
    let norm = stable_norm(&value);
    if !norm.is_finite() || norm <= 0.0 {
        return Err(OracleError::new(
            "contact normal or tangent basis has invalid norm",
        ));
    }
    Ok([value[0] / norm, value[1] / norm, value[2] / norm])
}

fn tangent_basis(
    normal: [f64; 3],
    tangent_u_hint: [f64; 3],
) -> Result<([f64; 3], [f64; 3]), OracleError> {
    if !tangent_u_hint.iter().all(|value| value.is_finite()) {
        return Err(OracleError::new("tangent-u hint must be finite"));
    }
    let projection = dot(tangent_u_hint, normal);
    let projected_hint = [
        tangent_u_hint[0] - normal[0] * projection,
        tangent_u_hint[1] - normal[1] * projection,
        tangent_u_hint[2] - normal[2] * projection,
    ];
    let tangent_u = normalize(projected_hint)?;
    let tangent_v = normalize(cross(normal, tangent_u))?;
    Ok((tangent_u, tangent_v))
}

fn validate_model(model: &ContactWrenchModel) -> Result<Vec<[f64; 3]>, OracleError> {
    if !model.characteristic_length.is_finite() || model.characteristic_length <= 0.0 {
        return Err(OracleError::new(
            "characteristic length must be finite and positive",
        ));
    }
    if model.contacts.is_empty() {
        return Err(OracleError::new("at least one contact is required"));
    }
    if model.cone_sides < 3 || model.cone_sides > 256 {
        return Err(OracleError::new("cone_sides must be in 3..=256"));
    }

    let mut ids = BTreeSet::new();
    let mut minimum_total = 0.0_f64;
    let mut maximum_total = 0.0_f64;
    let mut bases = Vec::with_capacity(model.contacts.len());

    for contact in &model.contacts {
        if contact.id.trim().is_empty() || !ids.insert(contact.id.as_str()) {
            return Err(OracleError::new("contact IDs must be non-empty and unique"));
        }
        if !contact.position.iter().all(|value| value.is_finite()) {
            return Err(OracleError::new("contact positions must be finite"));
        }
        if !contact.friction_u.is_finite()
            || !contact.friction_v.is_finite()
            || contact.friction_u < 0.0
            || contact.friction_v < 0.0
        {
            return Err(OracleError::new(
                "friction semi-axes must be finite and non-negative",
            ));
        }
        if !contact.min_normal_fraction.is_finite()
            || !contact.max_normal_fraction.is_finite()
            || contact.min_normal_fraction < 0.0
            || contact.max_normal_fraction > 1.0
            || contact.min_normal_fraction > contact.max_normal_fraction
        {
            return Err(OracleError::new(
                "normal-load bounds must satisfy 0 <= min <= max <= 1",
            ));
        }
        minimum_total += contact.min_normal_fraction;
        maximum_total += contact.max_normal_fraction;
        let normal = normalize(contact.normal)?;
        tangent_basis(normal, contact.tangent_u_hint)?;
        bases.push(normal);
    }

    if !minimum_total.is_finite()
        || !maximum_total.is_finite()
        || minimum_total > 1.0
        || maximum_total < 1.0
    {
        return Err(OracleError::new(
            "normal-load bounds cannot allocate the fixed unit total load",
        ));
    }
    Ok(bases)
}

fn contact_ray_support(
    contact: &ContactFrame,
    normal: [f64; 3],
    tangent_u_hint: [f64; 3],
    direction: &[f64; 6],
    characteristic_length: f64,
    cone_sides: usize,
) -> Result<(f64, usize), OracleError> {
    let (tangent_u, tangent_v) = tangent_basis(normal, tangent_u_hint)?;
    let [fx, fy, fz, mx, my, mz] = *direction;
    let inverse_length = 1.0 / characteristic_length;
    let mut best_support = f64::NEG_INFINITY;
    let mut best_ray = 0_usize;
    let ray_count = if contact.friction_u == 0.0 && contact.friction_v == 0.0 {
        1
    } else {
        cone_sides
    };

    for ray in 0..ray_count {
        let angle = std::f64::consts::TAU * ray as f64 / ray_count as f64;
        let tangent_u_scale = contact.friction_u * angle.cos();
        let tangent_v_scale = contact.friction_v * angle.sin();
        let force = [
            normal[0] + tangent_u[0] * tangent_u_scale + tangent_v[0] * tangent_v_scale,
            normal[1] + tangent_u[1] * tangent_u_scale + tangent_v[1] * tangent_v_scale,
            normal[2] + tangent_u[2] * tangent_u_scale + tangent_v[2] * tangent_v_scale,
        ];
        let moment = cross(contact.position, force);
        let value = fx * force[0]
            + fy * force[1]
            + fz * force[2]
            + (mx * moment[0] + my * moment[1] + mz * moment[2]) * inverse_length;
        if !value.is_finite() {
            return Err(OracleError::new("contact-wrench ray support is non-finite"));
        }
        if value > best_support {
            best_support = value;
            best_ray = ray;
        }
    }
    Ok((best_support, best_ray))
}

/// Compute the support of the unit-total-normal-load resultant-wrench set.
///
/// Each contact contributes a polyhedral friction cone. For each contact, the
/// best cone ray is selected for the query direction. The remaining allocation
/// problem is a bounded linear program: satisfy all lower load bounds, then
/// assign the remaining load to contacts in descending support order up to their
/// upper bounds. This is exact for the declared polyhedral model.
pub fn evaluate_support(
    model: &ContactWrenchModel,
    direction: &WrenchDirection,
) -> Result<ContactWrenchSupport, OracleError> {
    let normals = validate_model(model)?;
    let direction_norm = stable_norm(&direction.components);
    if direction.id.trim().is_empty()
        || !direction.components.iter().all(|value| value.is_finite())
        || !direction_norm.is_finite()
        || direction_norm <= 0.0
    {
        return Err(OracleError::new(
            "wrench direction must have a finite non-zero norm and ID",
        ));
    }
    let unit_direction = std::array::from_fn(|index| direction.components[index] / direction_norm);

    let mut contact_scores = Vec::with_capacity(model.contacts.len());
    for (index, contact) in model.contacts.iter().enumerate() {
        let (support_per_unit_normal_load, selected_friction_ray) = contact_ray_support(
            contact,
            normals[index],
            contact.tangent_u_hint,
            &unit_direction,
            model.characteristic_length,
            model.cone_sides,
        )?;
        contact_scores.push((index, support_per_unit_normal_load, selected_friction_ray));
    }

    let mut allocations = model
        .contacts
        .iter()
        .map(|contact| contact.min_normal_fraction)
        .collect::<Vec<_>>();
    let minimum_total: f64 = allocations.iter().sum();
    let mut remaining = (1.0 - minimum_total).max(0.0);
    let mut order = (0..model.contacts.len()).collect::<Vec<_>>();
    order.sort_by(|left, right| {
        contact_scores[*right]
            .1
            .total_cmp(&contact_scores[*left].1)
            .then_with(|| model.contacts[*left].id.cmp(&model.contacts[*right].id))
    });

    for index in order {
        if remaining <= 0.0 {
            break;
        }
        let available = model.contacts[index].max_normal_fraction - allocations[index];
        let added = remaining.min(available);
        allocations[index] += added;
        remaining -= added;
    }
    if remaining > 1.0e-10 {
        return Err(OracleError::new(
            "normal-load allocation did not satisfy unit total",
        ));
    }

    let mut support_value = 0.0_f64;
    let mut report_allocations = Vec::with_capacity(model.contacts.len());
    for (index, contact) in model.contacts.iter().enumerate() {
        let (_, score, ray) = contact_scores[index];
        support_value += allocations[index] * score;
        report_allocations.push(ContactLoadAllocation {
            contact_id: contact.id.clone(),
            normal_load_fraction: allocations[index],
            selected_friction_ray: ray,
            support_per_unit_normal_load: score,
        });
    }
    if !support_value.is_finite() {
        return Err(OracleError::new(
            "aggregate contact-wrench support is non-finite",
        ));
    }
    report_allocations.sort_by(|left, right| left.contact_id.cmp(&right.contact_id));

    Ok(ContactWrenchSupport {
        support_value,
        allocations: report_allocations,
        cone_sides: model.cone_sides,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct DirectionalSetSupport {
    pub direction_id: String,
    pub full_patch_support: f64,
    pub retained_patch_support: f64,
    pub normalized_deficit: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContactSetReport {
    pub retained_contact_ids: Vec<String>,
    pub directions: Vec<DirectionalSetSupport>,
    pub maximum_normalized_deficit: f64,
    pub mean_normalized_deficit: f64,
    pub support_evaluations: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExactContactSelectorConfig {
    pub keep: usize,
    pub maximum_subsets: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GreedyContactSelectorConfig {
    pub keep: usize,
    pub maximum_evaluations: u64,
}

fn subset_count(n: usize, k: usize) -> Result<u64, OracleError> {
    let reduced_k = k.min(n - k);
    let mut value = 1_u128;
    for index in 1..=reduced_k {
        let numerator = (n - reduced_k + index) as u128;
        let product = value
            .checked_mul(numerator)
            .ok_or_else(|| OracleError::new("contact subset count exceeds u128"))?;
        value = product / index as u128;
        if value > u64::MAX as u128 {
            return Err(OracleError::new("contact subset count exceeds u64"));
        }
    }
    Ok(value as u64)
}

fn sorted_contacts_by_id(contacts: &[ContactFrame]) -> Vec<ContactFrame> {
    let mut result = contacts.to_vec();
    result.sort_by(|left, right| left.id.cmp(&right.id));
    result
}

fn subset_model(
    model: &ContactWrenchModel,
    retained_ids: &[String],
) -> Result<ContactWrenchModel, OracleError> {
    if retained_ids.is_empty() {
        return Err(OracleError::new("retained contact set must not be empty"));
    }
    let mut requested = BTreeSet::new();
    for id in retained_ids {
        if id.trim().is_empty() || !requested.insert(id.as_str()) {
            return Err(OracleError::new(
                "retained contact IDs must be non-empty and unique",
            ));
        }
    }

    let all_ids = model
        .contacts
        .iter()
        .map(|contact| contact.id.as_str())
        .collect::<BTreeSet<_>>();
    if requested.iter().any(|id| !all_ids.contains(id)) {
        return Err(OracleError::new("retained set contains an unknown contact ID"));
    }
    for contact in &model.contacts {
        if !requested.contains(contact.id.as_str()) && contact.min_normal_fraction > 0.0 {
            return Err(OracleError::new(
                "cannot remove a contact with a mandatory positive minimum normal-load fraction",
            ));
        }
    }

    let contacts = sorted_contacts_by_id(
        &model
            .contacts
            .iter()
            .filter(|contact| requested.contains(contact.id.as_str()))
            .cloned()
            .collect::<Vec<_>>(),
    );
    Ok(ContactWrenchModel {
        contacts,
        cone_sides: model.cone_sides,
        characteristic_length: model.characteristic_length,
    })
}

/// Evaluate one retained contact set over a frozen finite wrench-direction set.
/// The full model is always the reference; every reduced model uses the same
/// friction discretization and characteristic-length convention.
pub fn evaluate_retained_contact_set(
    model: &ContactWrenchModel,
    retained_contact_ids: &[String],
    directions: &[WrenchDirection],
    support_floor: f64,
) -> Result<ContactSetReport, OracleError> {
    validate_model(model)?;
    if directions.is_empty() {
        return Err(OracleError::new("at least one wrench direction is required"));
    }
    if !support_floor.is_finite() || support_floor <= 0.0 {
        return Err(OracleError::new("support floor must be finite and positive"));
    }

    let full_model = ContactWrenchModel {
        contacts: sorted_contacts_by_id(&model.contacts),
        cone_sides: model.cone_sides,
        characteristic_length: model.characteristic_length,
    };
    let reduced_model = subset_model(&full_model, retained_contact_ids)?;
    let retained_contact_ids = reduced_model
        .contacts
        .iter()
        .map(|contact| contact.id.clone())
        .collect::<Vec<_>>();

    let mut direction_ids = BTreeSet::new();
    let mut reports = Vec::with_capacity(directions.len());
    let mut maximum_normalized_deficit = 0.0_f64;
    let mut deficit_sum = 0.0_f64;
    let mut support_evaluations = 0_u64;

    let mut ordered_directions = directions.to_vec();
    ordered_directions.sort_by(|left, right| left.id.cmp(&right.id));
    for direction in &ordered_directions {
        if direction.id.trim().is_empty() || !direction_ids.insert(direction.id.as_str()) {
            return Err(OracleError::new("direction IDs must be non-empty and unique"));
        }
        let full_support = evaluate_support(&full_model, direction)?.support_value;
        let retained_support = evaluate_support(&reduced_model, direction)?.support_value;
        support_evaluations = support_evaluations
            .checked_add(2)
            .ok_or_else(|| OracleError::new("support evaluation counter overflow"))?;
        let raw_deficit = (full_support - retained_support).max(0.0);
        let normalized_deficit = raw_deficit / full_support.abs().max(support_floor);
        if !normalized_deficit.is_finite() {
            return Err(OracleError::new("normalized contact-set deficit is non-finite"));
        }
        maximum_normalized_deficit = maximum_normalized_deficit.max(normalized_deficit);
        deficit_sum += normalized_deficit;
        reports.push(DirectionalSetSupport {
            direction_id: direction.id.clone(),
            full_patch_support: full_support,
            retained_patch_support: retained_support,
            normalized_deficit,
        });
    }

    let mean_normalized_deficit = deficit_sum / reports.len() as f64;
    if !mean_normalized_deficit.is_finite() {
        return Err(OracleError::new("mean contact-set deficit is non-finite"));
    }
    Ok(ContactSetReport {
        retained_contact_ids,
        directions: reports,
        maximum_normalized_deficit,
        mean_normalized_deficit,
        support_evaluations,
    })
}

fn compare_contact_set_reports(left: &ContactSetReport, right: &ContactSetReport) -> std::cmp::Ordering {
    left.maximum_normalized_deficit
        .total_cmp(&right.maximum_normalized_deficit)
        .then_with(|| left.mean_normalized_deficit.total_cmp(&right.mean_normalized_deficit))
        .then_with(|| left.retained_contact_ids.cmp(&right.retained_contact_ids))
}

fn visit_contact_combinations<F>(
    contacts: &[ContactFrame],
    keep: usize,
    start: usize,
    current_ids: &mut Vec<String>,
    visit: &mut F,
) -> Result<(), OracleError>
where
    F: FnMut(&[String]) -> Result<(), OracleError>,
{
    if current_ids.len() == keep {
        return visit(current_ids);
    }
    let needed = keep - current_ids.len();
    let final_start = contacts.len() - needed;
    for index in start..=final_start {
        current_ids.push(contacts[index].id.clone());
        visit_contact_combinations(contacts, keep, index + 1, current_ids, visit)?;
        current_ids.pop();
    }
    Ok(())
}

/// Exact k-contact selector for the declared polyhedral cone model and finite
/// direction set. It is exponential by design and refuses work above its cap.
pub fn select_exact_contact_subset(
    model: &ContactWrenchModel,
    directions: &[WrenchDirection],
    support_floor: f64,
    config: ExactContactSelectorConfig,
) -> Result<ContactSetReport, OracleError> {
    validate_model(model)?;
    let contacts = sorted_contacts_by_id(&model.contacts);
    if config.keep == 0 || config.keep > contacts.len() {
        return Err(OracleError::new("keep must be in 1..=candidate contact count"));
    }
    let required_contacts = contacts
        .iter()
        .filter(|contact| contact.min_normal_fraction > 0.0)
        .count();
    if config.keep < required_contacts {
        return Err(OracleError::new(
            "keep is smaller than the number of contacts with mandatory positive minimum load",
        ));
    }
    if config.maximum_subsets == 0 {
        return Err(OracleError::new("maximum_subsets must be positive"));
    }
    let total = subset_count(contacts.len(), config.keep)?;
    if total > config.maximum_subsets {
        return Err(OracleError::new(format!(
            "exact contact search requires {total} subsets, exceeding configured maximum {}",
            config.maximum_subsets
        )));
    }

    let mut best: Option<ContactSetReport> = None;
    let mut current_ids = Vec::with_capacity(config.keep);
    visit_contact_combinations(
        &contacts,
        config.keep,
        0,
        &mut current_ids,
        &mut |ids| {
            let candidate =
                evaluate_retained_contact_set(model, ids, directions, support_floor)?;
            if best.as_ref().is_none_or(|incumbent| {
                compare_contact_set_reports(&candidate, incumbent).is_lt()
            }) {
                best = Some(candidate);
            }
            Ok(())
        },
    )?;
    best.ok_or_else(|| OracleError::new("exact contact selector produced no candidate"))
}

/// Deterministic greedy contact selector. This is a baseline, not a general
/// approximation guarantee; exact search measures its gap on tractable sets.
pub fn select_greedy_contact_subset(
    model: &ContactWrenchModel,
    directions: &[WrenchDirection],
    support_floor: f64,
    config: GreedyContactSelectorConfig,
) -> Result<ContactSetReport, OracleError> {
    validate_model(model)?;
    let contacts = sorted_contacts_by_id(&model.contacts);
    if config.keep == 0 || config.keep > contacts.len() {
        return Err(OracleError::new("keep must be in 1..=candidate contact count"));
    }
    if config.maximum_evaluations == 0 {
        return Err(OracleError::new("maximum_evaluations must be positive"));
    }
    let required_ids = contacts
        .iter()
        .filter(|contact| contact.min_normal_fraction > 0.0)
        .map(|contact| contact.id.clone())
        .collect::<Vec<_>>();
    if required_ids.len() > config.keep {
        return Err(OracleError::new(
            "keep is smaller than the number of contacts with mandatory positive minimum load",
        ));
    }

    let mut selected = required_ids;
    let mut evaluations = 0_u64;
    while selected.len() < config.keep {
        let mut best: Option<(ContactSetReport, Vec<String>)> = None;
        for contact in &contacts {
            if selected.iter().any(|id| id == &contact.id) {
                continue;
            }
            if evaluations >= config.maximum_evaluations {
                return Err(OracleError::new(
                    "greedy contact evaluation count exceeds configured maximum",
                ));
            }
            let mut trial = selected.clone();
            trial.push(contact.id.clone());
            let report = evaluate_retained_contact_set(model, &trial, directions, support_floor)?;
            evaluations += 1;
            if best.as_ref().is_none_or(|(incumbent, _)| {
                compare_contact_set_reports(&report, incumbent).is_lt()
            }) {
                best = Some((report, trial));
            }
        }
        let (_, ids) = best
            .ok_or_else(|| OracleError::new("greedy contact selector found no candidate"))?;
        selected = ids;
    }

    let mut result =
        evaluate_retained_contact_set(model, &selected, directions, support_floor)?;
    result.support_evaluations = result
        .support_evaluations
        .checked_add(evaluations.checked_mul(directions.len() as u64 * 2).ok_or_else(|| {
            OracleError::new("greedy support evaluation counter overflow")
        })?)
        .ok_or_else(|| OracleError::new("greedy support evaluation counter overflow"))?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contact(
        id: &str,
        position: [f64; 3],
        normal: [f64; 3],
        friction_u: f64,
        friction_v: f64,
        min_normal_fraction: f64,
        max_normal_fraction: f64,
    ) -> ContactFrame {
        let tangent_u_hint = if normal[1].abs() < 0.9 {
            [0.0, 1.0, 0.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        contact_with_tangent(
            id,
            position,
            normal,
            tangent_u_hint,
            friction_u,
            friction_v,
            min_normal_fraction,
            max_normal_fraction,
        )
    }

    fn contact_with_tangent(
        id: &str,
        position: [f64; 3],
        normal: [f64; 3],
        tangent_u_hint: [f64; 3],
        friction_u: f64,
        friction_v: f64,
        min_normal_fraction: f64,
        max_normal_fraction: f64,
    ) -> ContactFrame {
        ContactFrame {
            id: id.into(),
            position,
            normal,
            tangent_u_hint,
            friction_u,
            friction_v,
            min_normal_fraction,
            max_normal_fraction,
        }
    }

    fn direction(id: &str, components: [f64; 6]) -> WrenchDirection {
        WrenchDirection {
            id: id.into(),
            components,
        }
    }

    #[test]
    fn cardinal_friction_support_matches_axis_of_inscribed_cone() {
        let model = ContactWrenchModel {
            contacts: vec![contact("c0", [0.0; 3], [0.0, 0.0, 1.0], 0.5, 0.5, 0.0, 1.0)],
            cone_sides: 4,
            characteristic_length: 1.0,
        };
        let result = evaluate_support(&model, &direction("fx", [1.0, 0.0, 0.0, 0.0, 0.0, 0.0]))
            .expect("cardinal friction support");
        assert!((result.support_value - 0.5).abs() < 1.0e-12);
    }

    #[test]
    fn anisotropic_friction_uses_declared_tangent_axes() {
        let model = ContactWrenchModel {
            contacts: vec![contact("c0", [0.0; 3], [0.0, 0.0, 1.0], 0.2, 0.7, 0.0, 1.0)],
            cone_sides: 4,
            characteristic_length: 1.0,
        };
        let result = evaluate_support(&model, &direction("fx", [1.0, 0.0, 0.0, 0.0, 0.0, 0.0]))
            .expect("anisotropic friction");
        assert!((result.support_value - 0.7).abs() < 1.0e-12);
    }

    #[test]
    fn anisotropic_friction_follows_material_tangent_hint() {
        let model = ContactWrenchModel {
            contacts: vec![contact_with_tangent(
                "c0",
                [0.0; 3],
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 0.0],
                0.2,
                0.7,
                0.0,
                1.0,
            )],
            cone_sides: 4,
            characteristic_length: 1.0,
        };
        let result = evaluate_support(&model, &direction("fx", [1.0, 0.0, 0.0, 0.0, 0.0, 0.0]))
            .expect("friction along tangent-u");
        assert!((result.support_value - 0.2).abs() < 1.0e-12);
    }

    #[test]
    fn varying_normals_and_load_caps_use_bounded_linear_allocation() {
        let model = ContactWrenchModel {
            contacts: vec![
                contact("z-normal", [0.0; 3], [0.0, 0.0, 1.0], 0.0, 0.0, 0.0, 0.4),
                contact("x-normal", [0.0; 3], [1.0, 0.0, 0.0], 0.0, 0.0, 0.0, 1.0),
            ],
            cone_sides: 4,
            characteristic_length: 1.0,
        };
        let result = evaluate_support(&model, &direction("fz", [0.0, 0.0, 1.0, 0.0, 0.0, 0.0]))
            .expect("bounded load allocation");
        assert!((result.support_value - 0.4).abs() < 1.0e-12);
        let z_load = result
            .allocations
            .iter()
            .find(|item| item.contact_id == "z-normal")
            .unwrap();
        let x_load = result
            .allocations
            .iter()
            .find(|item| item.contact_id == "x-normal")
            .unwrap();
        assert!((z_load.normal_load_fraction - 0.4).abs() < 1.0e-12);
        assert!((x_load.normal_load_fraction - 0.6).abs() < 1.0e-12);
    }

    #[test]
    fn lower_load_bounds_are_satisfied_before_support_optimization() {
        let model = ContactWrenchModel {
            contacts: vec![
                contact("z-normal", [0.0; 3], [0.0, 0.0, 1.0], 0.0, 0.0, 0.0, 1.0),
                contact("x-normal", [0.0; 3], [1.0, 0.0, 0.0], 0.0, 0.0, 0.3, 1.0),
            ],
            cone_sides: 4,
            characteristic_length: 1.0,
        };
        let result = evaluate_support(&model, &direction("fz", [0.0, 0.0, 1.0, 0.0, 0.0, 0.0]))
            .expect("minimum load constraints");
        assert!((result.support_value - 0.7).abs() < 1.0e-12);
        let x_load = result
            .allocations
            .iter()
            .find(|item| item.contact_id == "x-normal")
            .unwrap();
        assert!((x_load.normal_load_fraction - 0.3).abs() < 1.0e-12);
    }

    #[test]
    fn friction_polyhedron_error_matches_analytic_angular_bound() {
        for cone_sides in [4_usize, 8, 16, 32, 64] {
            let half_step = std::f64::consts::PI / cone_sides as f64;
            let query = direction(
                "between-rays",
                [-half_step.sin(), half_step.cos(), 0.0, 0.0, 0.0, 0.0],
            );
            let model = ContactWrenchModel {
                contacts: vec![contact("c0", [0.0; 3], [0.0, 0.0, 1.0], 0.5, 0.5, 0.0, 1.0)],
                cone_sides,
                characteristic_length: 1.0,
            };
            let support = evaluate_support(&model, &query)
                .expect("polyhedral friction support")
                .support_value;
            let expected = 0.5 * half_step.cos();
            assert!((support - expected).abs() < 1.0e-12);
            let relative_error = 1.0 - support / 0.5;
            assert!(relative_error <= 1.0 - half_step.cos() + 1.0e-12);
        }
    }

    #[test]
    fn infeasible_normal_load_bounds_are_rejected() {
        let model = ContactWrenchModel {
            contacts: vec![contact("c0", [0.0; 3], [0.0, 0.0, 1.0], 0.0, 0.0, 0.0, 0.4)],
            cone_sides: 4,
            characteristic_length: 1.0,
        };
        assert!(
            evaluate_support(&model, &direction("fz", [0.0, 0.0, 1.0, 0.0, 0.0, 0.0])).is_err()
        );
    }

    #[test]
    fn contact_order_does_not_change_support_or_allocations() {
        let mut first = ContactWrenchModel {
            contacts: vec![
                contact("a", [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], 0.3, 0.3, 0.0, 0.6),
                contact("b", [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], 0.3, 0.3, 0.0, 1.0),
            ],
            cone_sides: 8,
            characteristic_length: 1.0,
        };
        let direction = direction("mixed", [0.2, -0.1, 0.5, 0.4, 0.2, -0.3]);
        let result_a = evaluate_support(&first, &direction).unwrap();
        first.contacts.reverse();
        let result_b = evaluate_support(&first, &direction).unwrap();
        assert!((result_a.support_value - result_b.support_value).abs() < 1.0e-12);
        assert_eq!(result_a.allocations, result_b.allocations);
    }

    #[test]
    fn tangent_hint_parallel_to_normal_is_rejected() {
        let model = ContactWrenchModel {
            contacts: vec![contact_with_tangent(
                "c0",
                [0.0; 3],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 2.0],
                0.2,
                0.7,
                0.0,
                1.0,
            )],
            cone_sides: 8,
            characteristic_length: 1.0,
        };
        assert!(
            evaluate_support(&model, &direction("fx", [1.0, 0.0, 0.0, 0.0, 0.0, 0.0])).is_err()
        );
    }

    #[test]
    fn invalid_normal_and_direction_are_rejected() {
        let model = ContactWrenchModel {
            contacts: vec![contact("c0", [0.0; 3], [0.0; 3], 0.2, 0.2, 0.0, 1.0)],
            cone_sides: 8,
            characteristic_length: 1.0,
        };
        assert!(
            evaluate_support(&model, &direction("fx", [1.0, 0.0, 0.0, 0.0, 0.0, 0.0])).is_err()
        );

        let valid = ContactWrenchModel {
            contacts: vec![contact("c0", [0.0; 3], [0.0, 0.0, 1.0], 0.2, 0.2, 0.0, 1.0)],
            cone_sides: 8,
            characteristic_length: 1.0,
        };
        assert!(evaluate_support(&valid, &direction("zero", [0.0; 6])).is_err());
    }
}
