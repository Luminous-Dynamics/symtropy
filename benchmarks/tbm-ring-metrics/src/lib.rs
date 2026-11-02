//! Unit-explicit reference metrics for synthetic TBM ring/inspection records.
//!
//! This is arithmetic over supplied measurements, not a TBM physics model,
//! predictor, optimizer, controller or work authorization system.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExcavationSegment {
    pub segment_id: String,
    pub axial_advance_millimetres: u64,
    pub cutterhead_revolutions: u64,
    /// Integrated mechanical energy at the declared excavation boundary.
    pub mechanical_excavation_energy_watt_hours: Option<u64>,
    pub excavated_volume_litres: u64,
    /// Integrated energy from a separately declared whole-site meter boundary.
    pub whole_site_electrical_energy_watt_hours: Option<u64>,
    pub accepted_excavation_volume_litres: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricError {
    InvalidIdentity,
    ZeroRevolutions,
    ZeroExcavatedVolume,
    ZeroAcceptedVolume,
    AcceptedVolumeExceedsExcavated,
    ZeroInspectionIntervalVolume,
    InspectionIdentityInvalid,
    RadiusIncreased,
    ArithmeticOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExcavationMetrics {
    pub penetration_millimetres_per_revolution: f64,
    pub mechanical_specific_excavation_energy_kwh_per_m3: Option<f64>,
    pub whole_site_electrical_energy_kwh_per_accepted_m3: Option<f64>,
    pub accepted_volume_fraction: f64,
}

#[must_use]
pub fn derive_excavation_metrics(
    segment: &ExcavationSegment,
) -> Result<ExcavationMetrics, MetricError> {
    if segment.segment_id.trim().is_empty() {
        return Err(MetricError::InvalidIdentity);
    }
    if segment.cutterhead_revolutions == 0 {
        return Err(MetricError::ZeroRevolutions);
    }
    if segment.excavated_volume_litres == 0 {
        return Err(MetricError::ZeroExcavatedVolume);
    }
    if segment.accepted_excavation_volume_litres == 0 {
        return Err(MetricError::ZeroAcceptedVolume);
    }
    if segment.accepted_excavation_volume_litres > segment.excavated_volume_litres {
        return Err(MetricError::AcceptedVolumeExceedsExcavated);
    }

    // Wh/L is numerically identical to kWh/m³ because 1 L = 0.001 m³.
    let mechanical_specific_energy = segment
        .mechanical_excavation_energy_watt_hours
        .map(|energy_wh| energy_wh as f64 / segment.excavated_volume_litres as f64);
    let site_electrical_energy = segment
        .whole_site_electrical_energy_watt_hours
        .map(|energy_wh| energy_wh as f64 / segment.accepted_excavation_volume_litres as f64);

    Ok(ExcavationMetrics {
        penetration_millimetres_per_revolution: segment.axial_advance_millimetres as f64
            / segment.cutterhead_revolutions as f64,
        mechanical_specific_excavation_energy_kwh_per_m3: mechanical_specific_energy,
        whole_site_electrical_energy_kwh_per_accepted_m3: site_electrical_energy,
        accepted_volume_fraction: segment.accepted_excavation_volume_litres as f64
            / segment.excavated_volume_litres as f64,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CutterWearInterval {
    pub cutter_id: String,
    pub start_inspection_id: String,
    pub end_inspection_id: String,
    pub start_radial_radius_micrometres: u64,
    pub end_radial_radius_micrometres: u64,
    /// Declared expanded uncertainty for the radius-loss difference, in µm.
    pub expanded_loss_uncertainty_micrometres: u64,
    /// Excavated volume between the two inspections, in litres.
    pub interval_excavated_volume_litres: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CutterWearRate {
    /// Central estimate, numerically mm/m³.
    pub wear_rate_mm_per_m3: f64,
    /// Bounded interval induced by the declared expanded measurement uncertainty.
    pub lower_bound_mm_per_m3: f64,
    pub upper_bound_mm_per_m3: f64,
    pub radial_loss_micrometres: u64,
}

#[must_use]
pub fn derive_cutter_wear_rate(
    interval: &CutterWearInterval,
) -> Result<CutterWearRate, MetricError> {
    if interval.cutter_id.trim().is_empty() {
        return Err(MetricError::InvalidIdentity);
    }
    if interval.start_inspection_id.trim().is_empty()
        || interval.end_inspection_id.trim().is_empty()
        || interval.start_inspection_id == interval.end_inspection_id
    {
        return Err(MetricError::InspectionIdentityInvalid);
    }
    if interval.interval_excavated_volume_litres == 0 {
        return Err(MetricError::ZeroInspectionIntervalVolume);
    }
    if interval.end_radial_radius_micrometres > interval.start_radial_radius_micrometres {
        return Err(MetricError::RadiusIncreased);
    }

    let loss = interval.start_radial_radius_micrometres
        - interval.end_radial_radius_micrometres;
    let lower_loss = loss.saturating_sub(interval.expanded_loss_uncertainty_micrometres);
    let upper_loss = loss
        .checked_add(interval.expanded_loss_uncertainty_micrometres)
        .ok_or(MetricError::ArithmeticOverflow)?;
    let volume_litres = interval.interval_excavated_volume_litres as f64;

    // µm/L is numerically identical to mm/m³ (both scale by 1,000).
    Ok(CutterWearRate {
        wear_rate_mm_per_m3: loss as f64 / volume_litres,
        lower_bound_mm_per_m3: lower_loss as f64 / volume_litres,
        upper_bound_mm_per_m3: upper_loss as f64 / volume_litres,
        radial_loss_micrometres: loss,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment() -> ExcavationSegment {
        ExcavationSegment {
            segment_id: "drive-a-ring-100".into(),
            axial_advance_millimetres: 1_100,
            cutterhead_revolutions: 100,
            mechanical_excavation_energy_watt_hours: Some(3_000),
            excavated_volume_litres: 500,
            whole_site_electrical_energy_watt_hours: Some(5_000),
            accepted_excavation_volume_litres: 400,
        }
    }

    fn wear() -> CutterWearInterval {
        CutterWearInterval {
            cutter_id: "edge-cutter-12".into(),
            start_inspection_id: "inspection-a".into(),
            end_inspection_id: "inspection-b".into(),
            start_radial_radius_micrometres: 18_000,
            end_radial_radius_micrometres: 17_900,
            expanded_loss_uncertainty_micrometres: 10,
            interval_excavated_volume_litres: 100,
        }
    }

    #[test]
    fn hand_calculates_penetration_and_separate_energy_boundaries() {
        let metrics = derive_excavation_metrics(&segment()).unwrap();
        assert!((metrics.penetration_millimetres_per_revolution - 11.0).abs() < 1e-12);
        assert!((metrics.mechanical_specific_excavation_energy_kwh_per_m3.unwrap() - 6.0).abs() < 1e-12);
        assert!((metrics.whole_site_electrical_energy_kwh_per_accepted_m3.unwrap() - 12.5).abs() < 1e-12);
        assert!((metrics.accepted_volume_fraction - 0.8).abs() < 1e-12);
    }

    #[test]
    fn installed_or_mechanical_energy_is_not_substituted_for_site_electricity() {
        let mut s = segment();
        s.whole_site_electrical_energy_watt_hours = None;
        let m = derive_excavation_metrics(&s).unwrap();
        assert_eq!(m.mechanical_specific_excavation_energy_kwh_per_m3, Some(6.0));
        assert_eq!(m.whole_site_electrical_energy_kwh_per_accepted_m3, None);
    }

    #[test]
    fn accepted_volume_cannot_exceed_gross_excavated_volume() {
        let mut s = segment();
        s.accepted_excavation_volume_litres = 501;
        assert_eq!(
            derive_excavation_metrics(&s),
            Err(MetricError::AcceptedVolumeExceedsExcavated)
        );
    }

    #[test]
    fn zero_revolutions_are_rejected() {
        let mut s = segment();
        s.cutterhead_revolutions = 0;
        assert_eq!(derive_excavation_metrics(&s), Err(MetricError::ZeroRevolutions));
    }

    #[test]
    fn zero_excavated_volume_is_rejected() {
        let mut s = segment();
        s.excavated_volume_litres = 0;
        assert_eq!(derive_excavation_metrics(&s), Err(MetricError::ZeroExcavatedVolume));
    }

    #[test]
    fn zero_accepted_volume_is_rejected_for_accepted_energy_intensity() {
        let mut s = segment();
        s.accepted_excavation_volume_litres = 0;
        assert_eq!(derive_excavation_metrics(&s), Err(MetricError::ZeroAcceptedVolume));
    }

    #[test]
    fn blank_segment_id_is_rejected() {
        let mut s = segment();
        s.segment_id = " ".into();
        assert_eq!(derive_excavation_metrics(&s), Err(MetricError::InvalidIdentity));
    }

    #[test]
    fn cutter_wear_rate_matches_dimensional_hand_calculation() {
        let result = derive_cutter_wear_rate(&wear()).unwrap();
        assert_eq!(result.radial_loss_micrometres, 100);
        assert!((result.wear_rate_mm_per_m3 - 1.0).abs() < 1e-12);
        assert!((result.lower_bound_mm_per_m3 - 0.9).abs() < 1e-12);
        assert!((result.upper_bound_mm_per_m3 - 1.1).abs() < 1e-12);
    }

    #[test]
    fn uncertainty_wider_than_wear_has_zero_lower_bound() {
        let mut w = wear();
        w.expanded_loss_uncertainty_micrometres = 200;
        let result = derive_cutter_wear_rate(&w).unwrap();
        assert_eq!(result.lower_bound_mm_per_m3, 0.0);
        assert_eq!(result.upper_bound_mm_per_m3, 3.0);
    }

    #[test]
    fn same_inspection_cannot_define_an_interval() {
        let mut w = wear();
        w.end_inspection_id = w.start_inspection_id.clone();
        assert_eq!(derive_cutter_wear_rate(&w), Err(MetricError::InspectionIdentityInvalid));
    }

    #[test]
    fn zero_inspection_volume_is_rejected() {
        let mut w = wear();
        w.interval_excavated_volume_litres = 0;
        assert_eq!(
            derive_cutter_wear_rate(&w),
            Err(MetricError::ZeroInspectionIntervalVolume)
        );
    }

    #[test]
    fn increased_radius_requires_review_of_identity_or_measurement() {
        let mut w = wear();
        w.end_radial_radius_micrometres = 18_010;
        assert_eq!(derive_cutter_wear_rate(&w), Err(MetricError::RadiusIncreased));
    }

    #[test]
    fn upper_uncertainty_overflow_is_rejected() {
        let mut w = wear();
        w.start_radial_radius_micrometres = u64::MAX;
        w.end_radial_radius_micrometres = 0;
        w.expanded_loss_uncertainty_micrometres = 1;
        assert_eq!(derive_cutter_wear_rate(&w), Err(MetricError::ArithmeticOverflow));
    }
}
