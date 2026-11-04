// SPDX-License-Identifier: AGPL-3.0-or-later
//! Event-level energy accounting for switched wireless-power-transfer systems.
//!
//! This validator is intentionally narrower than a circuit simulator: it checks
//! a supplied transition energy receipt and never invents a state-reset rule.

use serde_json::{json, Value};

#[derive(Debug, Clone)]
struct Mode {
    id: String,
    topology_sha256: String,
    certificate_status: String,
    stored_energy_j: f64,
    energy_uncertainty_j: f64,
    state_dimension: usize,
}

#[derive(Debug, Clone)]
struct Tank {
    pre_j: f64,
    pre_uncertainty_j: f64,
    post_j: f64,
    post_uncertainty_j: f64,
}

fn field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing required field {path}.{key}"))
}

fn text_field(value: &Value, key: &str, path: &str) -> Result<String, String> {
    let text = field(value, key, path)?
        .as_str()
        .ok_or_else(|| format!("{path}.{key} must be a string"))?;
    if text.trim().is_empty() {
        return Err(format!("{path}.{key} must not be empty"));
    }
    Ok(text.to_owned())
}

fn number_field(value: &Value, key: &str, path: &str) -> Result<f64, String> {
    let number = field(value, key, path)?
        .as_f64()
        .ok_or_else(|| format!("{path}.{key} must be a finite JSON number"))?;
    if !number.is_finite() {
        return Err(format!("{path}.{key} must be finite"));
    }
    Ok(number)
}

fn nonnegative_field(value: &Value, key: &str, path: &str) -> Result<f64, String> {
    let number = number_field(value, key, path)?;
    if number < 0.0 {
        return Err(format!("{path}.{key} must be nonnegative"));
    }
    Ok(number)
}

fn dimension_field(value: &Value, key: &str, path: &str) -> Result<usize, String> {
    let raw = field(value, key, path)?
        .as_u64()
        .ok_or_else(|| format!("{path}.{key} must be a positive integer"))?;
    let dimension = usize::try_from(raw)
        .map_err(|_| format!("{path}.{key} does not fit this platform"))?;
    if dimension == 0 || dimension > 256 {
        return Err(format!("{path}.{key} must be in 1..=256"));
    }
    Ok(dimension)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn mode(value: &Value, path: &str) -> Result<Mode, String> {
    let id = text_field(value, "mode_id", path)?;
    let topology_sha256 = text_field(value, "topology_sha256", path)?;
    if !valid_sha256(&topology_sha256) {
        return Err(format!("{path}.topology_sha256 must be 64 lowercase hex characters"));
    }

    Ok(Mode {
        id,
        topology_sha256,
        certificate_status: text_field(value, "certificate_status", path)?,
        stored_energy_j: nonnegative_field(value, "stored_energy_j", path)?,
        energy_uncertainty_j: nonnegative_field(value, "energy_uncertainty_j", path)?,
        state_dimension: dimension_field(value, "state_dimension", path)?,
    })
}

fn tank(value: Option<&Value>) -> Result<Option<Tank>, String> {
    let Some(value) = value else {
        return Ok(None);
    };

    Ok(Some(Tank {
        pre_j: nonnegative_field(value, "pre_energy_j", "energy_tank")?,
        pre_uncertainty_j: nonnegative_field(
            value,
            "pre_energy_uncertainty_j",
            "energy_tank",
        )?,
        post_j: nonnegative_field(value, "post_energy_j", "energy_tank")?,
        post_uncertainty_j: nonnegative_field(
            value,
            "post_energy_uncertainty_j",
            "energy_tank",
        )?,
    }))
}

/// Evaluate one transition receipt.
///
/// All uncertainty fields are deterministic absolute error bounds in joules.
/// They are added conservatively; they are not standard deviations or
/// confidence intervals. The source/model that produced each energy value
/// remains responsible for its calibration and provenance.
pub fn evaluate(record: &Value) -> Value {
    let event_id = record
        .get("event_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_owned();

    match evaluate_checked(record, &event_id) {
        Ok(report) => report,
        Err(message) => json!({
            "event_id": event_id,
            "disposition": "INVALID_INPUT",
            "message": message
        }),
    }
}

fn evaluate_checked(record: &Value, event_id: &str) -> Result<Value, String> {
    let schema_version = text_field(record, "schema_version", "record")?;
    if schema_version != "1.0.0" {
        return Err(format!("unsupported schema_version {schema_version}"));
    }

    let pre = mode(field(record, "pre_mode", "record")?, "pre_mode")?;
    let post = mode(field(record, "post_mode", "record")?, "post_mode")?;
    let transition = field(record, "switch_event", "record")?;

    let mapping_kind = text_field(transition, "mapping_kind", "switch_event")?;
    let mapping_sha256 = text_field(transition, "mapping_sha256", "switch_event")?;
    if !valid_sha256(&mapping_sha256) {
        return Err("switch_event.mapping_sha256 must be 64 lowercase hex characters".into());
    }

    let source_dimension = dimension_field(
        transition,
        "source_state_dimension",
        "switch_event",
    )?;
    let target_dimension = dimension_field(
        transition,
        "target_state_dimension",
        "switch_event",
    )?;
    if source_dimension != pre.state_dimension || target_dimension != post.state_dimension {
        return Err("state mapping dimensions do not match the pre/post mode declarations".into());
    }
    let allowed_mappings = [
        "IDENTITY",
        "CHARGE_FLUX_CONSISTENT",
        "ENERGY_MINIMIZING",
        "DECLARED_PHYSICAL_RESET",
        "ENERGY_TANK",
    ];
    if !allowed_mappings.contains(&mapping_kind.as_str()) {
        return Err(format!("unsupported mapping_kind {mapping_kind}"));
    }
    if mapping_kind == "IDENTITY" && pre.state_dimension != post.state_dimension {
        return Err("IDENTITY mapping cannot change state dimension".into());
    }

    let switch_work_j = number_field(transition, "switch_work_net_j", "switch_event")?;
    let switch_work_u = nonnegative_field(
        transition,
        "switch_work_uncertainty_j",
        "switch_event",
    )?;
    let port_net_j = number_field(transition, "port_net_energy_j", "switch_event")?;
    let port_net_u = nonnegative_field(
        transition,
        "port_net_energy_uncertainty_j",
        "switch_event",
    )?;
    let dissipation_j = nonnegative_field(
        transition,
        "event_dissipation_j",
        "switch_event",
    )?;
    let dissipation_u = nonnegative_field(
        transition,
        "event_dissipation_uncertainty_j",
        "switch_event",
    )?;
    let absolute_tolerance_j =
        nonnegative_field(transition, "absolute_tolerance_j", "switch_event")?;
    let relative_tolerance =
        nonnegative_field(transition, "relative_tolerance", "switch_event")?;
    if relative_tolerance > 1.0 {
        return Err("switch_event.relative_tolerance must not exceed 1.0".into());
    }

    let energy_tank = tank(record.get("energy_tank"))?;
    if energy_tank.is_some() && mapping_kind != "ENERGY_TANK" {
        return Err("energy_tank is present but mapping_kind is not ENERGY_TANK".into());
    }
    if mapping_kind == "ENERGY_TANK" && energy_tank.is_none() {
        return Err("ENERGY_TANK mapping requires a complete energy_tank record".into());
    }

    let tank_pre = energy_tank.as_ref().map_or(0.0, |item| item.pre_j);
    let tank_post = energy_tank.as_ref().map_or(0.0, |item| item.post_j);
    let tank_pre_u = energy_tank
        .as_ref()
        .map_or(0.0, |item| item.pre_uncertainty_j);
    let tank_post_u = energy_tank
        .as_ref()
        .map_or(0.0, |item| item.post_uncertainty_j);

    let pre_total_j = pre.stored_energy_j + tank_pre;
    let post_total_j = post.stored_energy_j + tank_post;
    let delta_storage_j = post_total_j - pre_total_j;
    let expected_delta_j = port_net_j + switch_work_j - dissipation_j;
    let residual_j = delta_storage_j - expected_delta_j;

    let residual_uncertainty_j = pre.energy_uncertainty_j
        + post.energy_uncertainty_j
        + tank_pre_u
        + tank_post_u
        + port_net_u
        + switch_work_u
        + dissipation_u;

    let scale_j = pre_total_j
        .abs()
        .max(post_total_j.abs())
        .max(expected_delta_j.abs())
        .max(1.0e-12);
    let tolerance_j = absolute_tolerance_j + relative_tolerance * scale_j;
    let residual_lower_j = residual_j - residual_uncertainty_j;
    let residual_upper_j = residual_j + residual_uncertainty_j;

    // Independent non-generation inequality: stored energy may not increase
    // beyond declared port input and switching work, even if a loss estimate
    // is questionable.
    let generation_margin_j = delta_storage_j - (port_net_j + switch_work_j);
    let generation_uncertainty_j = pre.energy_uncertainty_j
        + post.energy_uncertainty_j
        + tank_pre_u
        + tank_post_u
        + port_net_u
        + switch_work_u;
    let generation_lower_j = generation_margin_j - generation_uncertainty_j;
    let generation_upper_j = generation_margin_j + generation_uncertainty_j;

    for (name, value) in [
        ("pre_total_j", pre_total_j),
        ("post_total_j", post_total_j),
        ("delta_storage_j", delta_storage_j),
        ("expected_delta_j", expected_delta_j),
        ("residual_j", residual_j),
        ("residual_uncertainty_j", residual_uncertainty_j),
        ("tolerance_j", tolerance_j),
        ("generation_lower_j", generation_lower_j),
        ("generation_upper_j", generation_upper_j),
    ] {
        if !value.is_finite() {
            return Err(format!("derived value {name} is non-finite"));
        }
    }

    let dispositions = if generation_lower_j > tolerance_j {
        (
            "ENERGY_GENERATING_REJECTED",
            "conservative lower bound proves stored-energy growth beyond declared port and switch work",
        )
    } else if pre.certificate_status != "PASSIVE_CERTIFIED"
        || post.certificate_status != "PASSIVE_CERTIFIED"
    {
        (
            "MODE_UNQUALIFIED",
            "both adjacent modes require upstream PASSIVE_CERTIFIED evidence before transition qualification",
        )
    } else if residual_lower_j >= -tolerance_j && residual_upper_j <= tolerance_j {
        (
            "PASSIVE_TRANSITION_CERTIFIED",
            "event energy balance closes within the conservative uncertainty and tolerance envelope",
        )
    } else {
        (
            "ENERGY_BALANCE_UNRESOLVED",
            "event energy balance does not close tightly enough to certify this transition",
        )
    };

    Ok(json!({
        "event_id": event_id,
        "disposition": dispositions.0,
        "message": dispositions.1,
        "pre_mode_id": pre.id,
        "post_mode_id": post.id,
        "pre_topology_sha256": pre.topology_sha256,
        "post_topology_sha256": post.topology_sha256,
        "mapping_kind": mapping_kind,
        "mapping_sha256": mapping_sha256,
        "pre_total_stored_energy_j": pre_total_j,
        "post_total_stored_energy_j": post_total_j,
        "delta_total_stored_energy_j": delta_storage_j,
        "port_net_energy_j": port_net_j,
        "switch_work_net_j": switch_work_j,
        "event_dissipation_j": dissipation_j,
        "expected_storage_change_j": expected_delta_j,
        "energy_residual_j": residual_j,
        "energy_residual_uncertainty_j": residual_uncertainty_j,
        "energy_residual_interval_j": [residual_lower_j, residual_upper_j],
        "non_generation_margin_j": generation_margin_j,
        "non_generation_interval_j": [generation_lower_j, generation_upper_j],
        "decision_tolerance_j": tolerance_j
    }))
}

#[cfg(test)]
mod tests {
    use super::evaluate;
    use serde_json::{json, Value};

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const DIGEST_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    fn event(
        event_id: &str,
        pre_energy_j: f64,
        post_energy_j: f64,
        switch_work_j: f64,
        dissipation_j: f64,
    ) -> Value {
        json!({
            "schema_version": "1.0.0",
            "event_id": event_id,
            "event_time_ns": 1000,
            "pre_mode": {
                "mode_id": "LCC_S",
                "topology_sha256": DIGEST_A,
                "certificate_status": "PASSIVE_CERTIFIED",
                "stored_energy_j": pre_energy_j,
                "energy_uncertainty_j": 0.00001,
                "state_dimension": 2
            },
            "post_mode": {
                "mode_id": "LCC_S_S",
                "topology_sha256": DIGEST_B,
                "certificate_status": "PASSIVE_CERTIFIED",
                "stored_energy_j": post_energy_j,
                "energy_uncertainty_j": 0.00001,
                "state_dimension": 2
            },
            "switch_event": {
                "mapping_kind": "CHARGE_FLUX_CONSISTENT",
                "mapping_sha256": DIGEST_C,
                "source_state_dimension": 2,
                "target_state_dimension": 2,
                "switch_work_net_j": switch_work_j,
                "switch_work_uncertainty_j": 0.00001,
                "port_net_energy_j": 0.0,
                "port_net_energy_uncertainty_j": 0.00001,
                "event_dissipation_j": dissipation_j,
                "event_dissipation_uncertainty_j": 0.00001,
                "absolute_tolerance_j": 0.0001,
                "relative_tolerance": 0.000001
            }
        })
    }

    fn disposition(record: &Value) -> String {
        evaluate(record)["disposition"]
            .as_str()
            .expect("disposition must be a string")
            .to_owned()
    }

    #[test]
    fn certifies_energy_conserving_transition() {
        assert_eq!(
            disposition(&event("conserved", 2.0, 2.0, 0.0, 0.0)),
            "PASSIVE_TRANSITION_CERTIFIED"
        );
    }

    #[test]
    fn certifies_explicitly_dissipative_transition() {
        assert_eq!(
            disposition(&event("dissipative", 2.0, 1.9, 0.0, 0.1)),
            "PASSIVE_TRANSITION_CERTIFIED"
        );
    }

    #[test]
    fn requires_explicit_work_for_storage_increase() {
        assert_eq!(
            disposition(&event("switch-work", 1.0, 1.2, 0.2, 0.0)),
            "PASSIVE_TRANSITION_CERTIFIED"
        );
    }

    #[test]
    fn rejects_energy_creating_state_reset_even_when_both_modes_are_passive() {
        assert_eq!(
            disposition(&event("hidden-injection", 1.0, 1.2, 0.0, 0.0)),
            "ENERGY_GENERATING_REJECTED"
        );
    }

    #[test]
    fn energy_tank_can_fund_a_dimension_changing_transition() {
        let mut record = event("tank-funded", 1.0, 1.4, 0.0, 0.0);
        record["pre_mode"]["state_dimension"] = json!(2);
        record["post_mode"]["state_dimension"] = json!(3);
        record["switch_event"]["mapping_kind"] = json!("ENERGY_TANK");
        record["switch_event"]["target_state_dimension"] = json!(3);
        record["energy_tank"] = json!({
            "pre_energy_j": 0.5,
            "pre_energy_uncertainty_j": 0.00001,
            "post_energy_j": 0.1,
            "post_energy_uncertainty_j": 0.00001
        });
        assert_eq!(
            disposition(&record),
            "PASSIVE_TRANSITION_CERTIFIED"
        );
    }

    #[test]
    fn rejects_identity_mapping_when_state_dimension_changes() {
        let mut record = event("bad-identity", 1.0, 1.0, 0.0, 0.0);
        record["post_mode"]["state_dimension"] = json!(3);
        record["switch_event"]["target_state_dimension"] = json!(3);
        record["switch_event"]["mapping_kind"] = json!("IDENTITY");
        assert_eq!(disposition(&record), "INVALID_INPUT");
    }

    #[test]
    fn abstains_when_uncertainty_overlaps_energy_creation_boundary() {
        let mut record = event("uncertain", 1.0, 1.1, 0.0, 0.0);
        record["pre_mode"]["energy_uncertainty_j"] = json!(0.06);
        record["post_mode"]["energy_uncertainty_j"] = json!(0.06);
        assert_eq!(disposition(&record), "ENERGY_BALANCE_UNRESOLVED");
    }

    #[test]
    fn refuses_unqualified_adjacent_mode() {
        let mut record = event("unqualified-mode", 1.0, 1.0, 0.0, 0.0);
        record["pre_mode"]["certificate_status"] = json!("UNRESOLVED");
        assert_eq!(disposition(&record), "MODE_UNQUALIFIED");
    }

    #[test]
    fn rejects_negative_dissipation() {
        assert_eq!(
            disposition(&event("negative-loss", 1.0, 1.0, 0.0, -0.1)),
            "INVALID_INPUT"
        );
    }

    #[test]
    fn requires_complete_energy_tank_receipt() {
        let mut record = event("missing-tank", 1.0, 1.0, 0.0, 0.0);
        record["switch_event"]["mapping_kind"] = json!("ENERGY_TANK");
        assert_eq!(disposition(&record), "INVALID_INPUT");
    }

    #[test]
    fn refuses_incomplete_topology_digest() {
        let mut record = event("bad-digest", 1.0, 1.0, 0.0, 0.0);
        record["post_mode"]["topology_sha256"] = json!("short");
        assert_eq!(disposition(&record), "INVALID_INPUT");
    }

    #[test]
    fn deterministic_scale_sweep_preserves_relative_decision() {
        for scale in [1.0e-9, 1.0, 1.0e9, 1.0e15] {
            let record = event(
                &format!("scale-{scale:e}"),
                scale,
                scale * 1.1,
                scale * 0.1,
                0.0,
            );
            assert_eq!(
                disposition(&record),
                "PASSIVE_TRANSITION_CERTIFIED",
                "scale {scale:e}"
            );
        }
    }
}
