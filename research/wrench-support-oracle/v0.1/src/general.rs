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

fn tangent_basis(normal: [f64; 3]) -> Result<([f64; 3], [f64; 3]), OracleError> {
    let reference = if normal[0].abs() <= normal[1].abs() && normal[0].abs() <= normal[2].abs() {
        [1.0, 0.0, 0.0]
    } else if normal[1].abs() <= normal[2].abs() {
        [0.0, 1.0, 0.0]
    } else {
        [0.0, 0.0, 1.0]
    };
    let tangent_u = normalize(cross(normal, reference))?;
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
            contacts: vec![contact(
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
        let result = evaluate_support(
            &model,
            &direction("fx", [1.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        )
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
