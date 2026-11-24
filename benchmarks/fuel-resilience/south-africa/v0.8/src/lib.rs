#![forbid(unsafe_code)]

//! SAFRB v0.8: evidence-bounded service continuity and restoration.
//!
//! The evaluator consumes service-boundary energy intervals from a named
//! upstream accounting result. It does not optimize, dispatch, control assets,
//! verify source bytes, authenticate issuers, or certify a physical facility.
//! A measured evidence record is structurally qualified only; independent
//! metrology and operator review remain necessary.

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceClass {
    Measured,
    Scenario,
    Reported,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Evidence {
    pub source_id: String,
    pub source_uri: String,
    pub evidence_class: EvidenceClass,
    /// Digest reference only. This crate does not read or hash source bytes.
    pub raw_sha256: Option<String>,
    /// Identifier of the upstream service-accounting result, e.g. v0.6.
    pub upstream_artifact_id: Option<String>,
    /// Time window represented by the underlying source measurement or fixture.
    pub measurement_start_unix_s: u64,
    pub measurement_end_unix_s: u64,
    /// When the source record was emitted or made available.
    pub recorded_at_unix_s: u64,
}

impl Evidence {
    pub fn validate_for_window(&self, start: u64, end: u64) -> Result<(), String> {
        if self.source_id.trim().is_empty() || self.source_uri.trim().is_empty() {
            return Err("evidence source_id and source_uri must not be empty".into());
        }
        if self.measurement_start_unix_s >= self.measurement_end_unix_s {
            return Err("evidence measurement window must have positive duration".into());
        }
        if self.measurement_start_unix_s > start || self.measurement_end_unix_s < end {
            return Err("evidence measurement window does not cover the service window".into());
        }
        if self.recorded_at_unix_s < self.measurement_end_unix_s {
            return Err("evidence cannot be recorded before its measurement window ends".into());
        }

        match self.evidence_class {
            EvidenceClass::Measured => {
                let digest_is_well_formed = self.raw_sha256.as_ref().is_some_and(|digest| {
                    digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
                });
                if !digest_is_well_formed {
                    return Err(
                        "measured evidence requires a 64-character SHA-256 reference".into(),
                    );
                }
                if self
                    .upstream_artifact_id
                    .as_ref()
                    .is_none_or(|artifact| artifact.trim().is_empty())
                {
                    return Err(
                        "measured evidence requires an upstream accounting artifact ID".into(),
                    );
                }
            }
            EvidenceClass::Scenario => {
                if !self.source_uri.starts_with("urn:safrb:fixture:") {
                    return Err(
                        "scenario evidence must use the explicit SAFRB fixture namespace".into(),
                    );
                }
            }
            EvidenceClass::Reported => {
                return Err("reported-only evidence cannot qualify a continuity result".into());
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceWindow {
    pub window_id: String,
    pub period_id: String,
    pub service_id: String,
    pub boundary_id: String,
    pub start_unix_s: u64,
    pub duration_seconds: u64,
    /// Minimum energy necessary to preserve the defined critical service.
    pub critical_floor_wh: u128,
    /// Desired energy for full service, at the same named boundary.
    pub target_wh: u128,
    /// Bounded net energy actually delivered at the service boundary.
    pub delivered_lower_wh: u128,
    pub delivered_upper_wh: u128,
    pub evidence: Evidence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowDisposition {
    GuaranteedAtOrAboveCriticalFloor,
    ConfirmedBelowCriticalFloor,
    UncertainAgainstCriticalFloor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CertificateDisposition {
    ContinuitySupportedWithinHorizon,
    ConfirmedCriticalServiceShortfall,
    ContinuityNotProven,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowAssessment {
    pub window_id: String,
    pub period_id: String,
    pub start_unix_s: u64,
    pub end_unix_s: u64,
    pub disposition: WindowDisposition,
    /// Lower bound on energy below the critical floor: floor - delivered upper.
    pub critical_shortfall_lower_wh: u128,
    /// Upper bound on energy below the critical floor: floor - delivered lower.
    pub critical_shortfall_upper_wh: u128,
    pub target_shortfall_lower_wh: u128,
    pub target_shortfall_upper_wh: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestorationEpisode {
    pub episode_start_unix_s: u64,
    /// First interval in a candidate sustained-recovery run, if observed.
    pub recovery_candidate_start_unix_s: Option<u64>,
    /// End of the interval that completed the required sustained-recovery time.
    pub recovery_confirmed_at_unix_s: Option<u64>,
    /// Elapsed time to the first interval of confirmed sustained recovery.
    pub restoration_latency_seconds: Option<u64>,
    /// Time spent confirming that recovery was sustained.
    pub confirmation_delay_seconds: Option<u64>,
    /// Elapsed time until the horizon ended or sustained recovery was confirmed.
    pub observed_duration_seconds: u64,
    pub confirmed_shortfall_windows: u32,
    pub uncertain_windows: u32,
    pub critical_shortfall_lower_wh: u128,
    pub critical_shortfall_upper_wh: u128,
    /// True when the observation horizon ended before sustained recovery was confirmed.
    pub right_censored: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceContinuityCertificate {
    pub evaluator_version: &'static str,
    pub service_id: String,
    pub boundary_id: String,
    pub horizon_start_unix_s: u64,
    pub horizon_end_unix_s: u64,
    pub restoration_confirmation_seconds: u64,
    pub disposition: CertificateDisposition,
    pub windows_total: u32,
    pub guaranteed_windows: u32,
    pub confirmed_shortfall_windows: u32,
    pub uncertain_windows: u32,
    pub guaranteed_service_seconds: u64,
    pub horizon_seconds: u64,
    /// Guaranteed time above the critical floor, in parts per million of the horizon.
    pub guaranteed_time_ppm: u32,
    pub critical_shortfall_lower_wh: u128,
    pub critical_shortfall_upper_wh: u128,
    pub target_shortfall_lower_wh: u128,
    pub target_shortfall_upper_wh: u128,
    pub window_assessments: Vec<WindowAssessment>,
    pub restoration_episodes: Vec<RestorationEpisode>,
    pub evidence_source_ids: Vec<String>,
    pub upstream_artifact_ids: Vec<String>,
}

#[derive(Clone, Debug)]
struct EpisodeBuilder {
    start_unix_s: u64,
    recovery_candidate_start_unix_s: Option<u64>,
    recovery_streak_seconds: u64,
    confirmed_shortfall_windows: u32,
    uncertain_windows: u32,
    critical_shortfall_lower_wh: u128,
    critical_shortfall_upper_wh: u128,
}

impl EpisodeBuilder {
    fn new(start_unix_s: u64) -> Self {
        Self {
            start_unix_s,
            recovery_candidate_start_unix_s: None,
            recovery_streak_seconds: 0,
            confirmed_shortfall_windows: 0,
            uncertain_windows: 0,
            critical_shortfall_lower_wh: 0,
            critical_shortfall_upper_wh: 0,
        }
    }

    fn add_unverified_window(&mut self, assessment: &WindowAssessment) -> Result<(), String> {
        self.recovery_candidate_start_unix_s = None;
        self.recovery_streak_seconds = 0;
        match assessment.disposition {
            WindowDisposition::ConfirmedBelowCriticalFloor => {
                self.confirmed_shortfall_windows = self
                    .confirmed_shortfall_windows
                    .checked_add(1)
                    .ok_or_else(|| "episode window count overflowed".to_string())?;
            }
            WindowDisposition::UncertainAgainstCriticalFloor => {
                self.uncertain_windows = self
                    .uncertain_windows
                    .checked_add(1)
                    .ok_or_else(|| "episode window count overflowed".to_string())?;
            }
            WindowDisposition::GuaranteedAtOrAboveCriticalFloor => {
                return Err("guaranteed window cannot be added as unverified".into());
            }
        }
        self.critical_shortfall_lower_wh = self
            .critical_shortfall_lower_wh
            .checked_add(assessment.critical_shortfall_lower_wh)
            .ok_or_else(|| "episode shortfall lower bound overflowed".to_string())?;
        self.critical_shortfall_upper_wh = self
            .critical_shortfall_upper_wh
            .checked_add(assessment.critical_shortfall_upper_wh)
            .ok_or_else(|| "episode shortfall upper bound overflowed".to_string())?;
        Ok(())
    }

    fn finish_confirmed(
        self,
        first_recovery_start: u64,
        confirmed_at: u64,
    ) -> Result<RestorationEpisode, String> {
        let latency = first_recovery_start
            .checked_sub(self.start_unix_s)
            .ok_or_else(|| "restoration timestamp precedes episode start".to_string())?;
        let confirmation_delay = confirmed_at
            .checked_sub(first_recovery_start)
            .ok_or_else(|| "restoration confirmation precedes its candidate start".to_string())?;
        let observed_duration = confirmed_at
            .checked_sub(self.start_unix_s)
            .ok_or_else(|| "restoration confirmation precedes episode start".to_string())?;
        Ok(RestorationEpisode {
            episode_start_unix_s: self.start_unix_s,
            recovery_candidate_start_unix_s: Some(first_recovery_start),
            recovery_confirmed_at_unix_s: Some(confirmed_at),
            restoration_latency_seconds: Some(latency),
            confirmation_delay_seconds: Some(confirmation_delay),
            observed_duration_seconds: observed_duration,
            confirmed_shortfall_windows: self.confirmed_shortfall_windows,
            uncertain_windows: self.uncertain_windows,
            critical_shortfall_lower_wh: self.critical_shortfall_lower_wh,
            critical_shortfall_upper_wh: self.critical_shortfall_upper_wh,
            right_censored: false,
        })
    }

    fn finish_censored(self, horizon_end: u64) -> Result<RestorationEpisode, String> {
        let duration = horizon_end
            .checked_sub(self.start_unix_s)
            .ok_or_else(|| "horizon end precedes episode start".to_string())?;
        Ok(RestorationEpisode {
            episode_start_unix_s: self.start_unix_s,
            recovery_candidate_start_unix_s: self.recovery_candidate_start_unix_s,
            recovery_confirmed_at_unix_s: None,
            restoration_latency_seconds: None,
            confirmation_delay_seconds: None,
            observed_duration_seconds: duration,
            confirmed_shortfall_windows: self.confirmed_shortfall_windows,
            uncertain_windows: self.uncertain_windows,
            critical_shortfall_lower_wh: self.critical_shortfall_lower_wh,
            critical_shortfall_upper_wh: self.critical_shortfall_upper_wh,
            right_censored: true,
        })
    }
}

fn checked_add_u128(left: u128, right: u128, label: &str) -> Result<u128, String> {
    left.checked_add(right)
        .ok_or_else(|| format!("{label} overflowed u128"))
}

fn assess_window(window: &ServiceWindow) -> Result<WindowAssessment, String> {
    if window.window_id.trim().is_empty()
        || window.period_id.trim().is_empty()
        || window.service_id.trim().is_empty()
        || window.boundary_id.trim().is_empty()
    {
        return Err("window, period, service and boundary IDs must not be empty".into());
    }
    if window.duration_seconds == 0 {
        return Err(format!(
            "window {} must have positive duration",
            window.window_id
        ));
    }
    if window.critical_floor_wh == 0 || window.target_wh == 0 {
        return Err(format!(
            "window {} must have positive service requirements",
            window.window_id
        ));
    }
    if window.target_wh < window.critical_floor_wh {
        return Err(format!(
            "window {} target is below its critical service floor",
            window.window_id
        ));
    }
    if window.delivered_lower_wh > window.delivered_upper_wh {
        return Err(format!(
            "window {} delivered lower bound exceeds upper bound",
            window.window_id
        ));
    }
    let end_unix_s = window
        .start_unix_s
        .checked_add(window.duration_seconds)
        .ok_or_else(|| format!("window {} end timestamp overflowed", window.window_id))?;
    window
        .evidence
        .validate_for_window(window.start_unix_s, end_unix_s)?;

    let disposition = if window.delivered_lower_wh >= window.critical_floor_wh {
        WindowDisposition::GuaranteedAtOrAboveCriticalFloor
    } else if window.delivered_upper_wh < window.critical_floor_wh {
        WindowDisposition::ConfirmedBelowCriticalFloor
    } else {
        WindowDisposition::UncertainAgainstCriticalFloor
    };

    Ok(WindowAssessment {
        window_id: window.window_id.clone(),
        period_id: window.period_id.clone(),
        start_unix_s: window.start_unix_s,
        end_unix_s,
        disposition,
        critical_shortfall_lower_wh: window
            .critical_floor_wh
            .saturating_sub(window.delivered_upper_wh),
        critical_shortfall_upper_wh: window
            .critical_floor_wh
            .saturating_sub(window.delivered_lower_wh),
        target_shortfall_lower_wh: window.target_wh.saturating_sub(window.delivered_upper_wh),
        target_shortfall_upper_wh: window.target_wh.saturating_sub(window.delivered_lower_wh),
    })
}

/// Evaluate one service at one named boundary over a contiguous time horizon.
///
/// A service window is guaranteed only when its delivered-energy lower bound
/// meets the critical floor. A sustained restoration is confirmed only after
/// that condition holds continuously for the configured duration. Gaps,
/// overlaps, duplicated identities, unqualified evidence and overflow fail closed.
pub fn evaluate(
    windows: &[ServiceWindow],
    restoration_confirmation_seconds: u64,
) -> Result<ServiceContinuityCertificate, String> {
    if windows.is_empty() {
        return Err("at least one service window is required".into());
    }
    if restoration_confirmation_seconds == 0 {
        return Err("restoration confirmation duration must be positive".into());
    }

    let service_id = windows[0].service_id.clone();
    let boundary_id = windows[0].boundary_id.clone();
    let mut window_ids = BTreeSet::new();
    let mut period_ids = BTreeSet::new();
    let mut source_ids = BTreeSet::new();
    let mut artifact_ids = BTreeSet::new();
    let mut assessments = Vec::with_capacity(windows.len());

    let mut expected_start = windows[0].start_unix_s;
    for window in windows {
        if window.service_id != service_id || window.boundary_id != boundary_id {
            return Err("all windows must describe the same service and boundary".into());
        }
        if !window_ids.insert(window.window_id.clone()) {
            return Err(format!("duplicate window ID: {}", window.window_id));
        }
        if !period_ids.insert(window.period_id.clone()) {
            return Err(format!("duplicate period ID: {}", window.period_id));
        }
        if window.start_unix_s != expected_start {
            return Err(format!(
                "service windows must be ordered and contiguous; expected start {}, received {}",
                expected_start, window.start_unix_s
            ));
        }
        let assessment = assess_window(window)?;
        expected_start = assessment.end_unix_s;
        source_ids.insert(window.evidence.source_id.clone());
        if let Some(artifact_id) = &window.evidence.upstream_artifact_id {
            artifact_ids.insert(artifact_id.clone());
        }
        assessments.push(assessment);
    }

    let horizon_start = assessments[0].start_unix_s;
    let horizon_end = assessments
        .last()
        .ok_or_else(|| "assessment unexpectedly empty".to_string())?
        .end_unix_s;
    let horizon_seconds = horizon_end
        .checked_sub(horizon_start)
        .ok_or_else(|| "horizon duration underflowed".to_string())?;

    let mut guaranteed_windows = 0_u32;
    let mut confirmed_shortfall_windows = 0_u32;
    let mut uncertain_windows = 0_u32;
    let mut guaranteed_service_seconds = 0_u64;
    let mut critical_shortfall_lower_wh = 0_u128;
    let mut critical_shortfall_upper_wh = 0_u128;
    let mut target_shortfall_lower_wh = 0_u128;
    let mut target_shortfall_upper_wh = 0_u128;
    let mut episodes = Vec::new();
    let mut active_episode: Option<EpisodeBuilder> = None;

    for (window, assessment) in windows.iter().zip(&assessments) {
        critical_shortfall_lower_wh = checked_add_u128(
            critical_shortfall_lower_wh,
            assessment.critical_shortfall_lower_wh,
            "critical shortfall lower bound",
        )?;
        critical_shortfall_upper_wh = checked_add_u128(
            critical_shortfall_upper_wh,
            assessment.critical_shortfall_upper_wh,
            "critical shortfall upper bound",
        )?;
        target_shortfall_lower_wh = checked_add_u128(
            target_shortfall_lower_wh,
            assessment.target_shortfall_lower_wh,
            "target shortfall lower bound",
        )?;
        target_shortfall_upper_wh = checked_add_u128(
            target_shortfall_upper_wh,
            assessment.target_shortfall_upper_wh,
            "target shortfall upper bound",
        )?;

        match assessment.disposition {
            WindowDisposition::GuaranteedAtOrAboveCriticalFloor => {
                guaranteed_windows = guaranteed_windows
                    .checked_add(1)
                    .ok_or_else(|| "guaranteed window count overflowed".to_string())?;
                guaranteed_service_seconds = guaranteed_service_seconds
                    .checked_add(window.duration_seconds)
                    .ok_or_else(|| "guaranteed service duration overflowed".to_string())?;

                if let Some(mut episode) = active_episode.take() {
                    let candidate_start = *episode
                        .recovery_candidate_start_unix_s
                        .get_or_insert(assessment.start_unix_s);
                    episode.recovery_streak_seconds = episode
                        .recovery_streak_seconds
                        .checked_add(window.duration_seconds)
                        .ok_or_else(|| "recovery streak duration overflowed".to_string())?;
                    if episode.recovery_streak_seconds >= restoration_confirmation_seconds {
                        episodes.push(
                            episode.finish_confirmed(candidate_start, assessment.end_unix_s)?,
                        );
                    } else {
                        active_episode = Some(episode);
                    }
                }
            }
            WindowDisposition::ConfirmedBelowCriticalFloor => {
                confirmed_shortfall_windows = confirmed_shortfall_windows
                    .checked_add(1)
                    .ok_or_else(|| "confirmed shortfall count overflowed".to_string())?;
                if active_episode.is_none() {
                    active_episode = Some(EpisodeBuilder::new(assessment.start_unix_s));
                }
                active_episode
                    .as_mut()
                    .ok_or_else(|| "episode initialization failed".to_string())?
                    .add_unverified_window(assessment)?;
            }
            WindowDisposition::UncertainAgainstCriticalFloor => {
                uncertain_windows = uncertain_windows
                    .checked_add(1)
                    .ok_or_else(|| "uncertain window count overflowed".to_string())?;
                if active_episode.is_none() {
                    active_episode = Some(EpisodeBuilder::new(assessment.start_unix_s));
                }
                active_episode
                    .as_mut()
                    .ok_or_else(|| "episode initialization failed".to_string())?
                    .add_unverified_window(assessment)?;
            }
        }
    }

    if let Some(episode) = active_episode {
        episodes.push(episode.finish_censored(horizon_end)?);
    }

    let disposition = if confirmed_shortfall_windows > 0 {
        CertificateDisposition::ConfirmedCriticalServiceShortfall
    } else if uncertain_windows > 0 {
        CertificateDisposition::ContinuityNotProven
    } else {
        CertificateDisposition::ContinuitySupportedWithinHorizon
    };

    let guaranteed_time_ppm = u128::from(guaranteed_service_seconds)
        .checked_mul(1_000_000)
        .ok_or_else(|| "guaranteed-time ratio overflowed".to_string())?
        .checked_div(u128::from(horizon_seconds))
        .ok_or_else(|| "horizon duration is zero".to_string())?;
    let guaranteed_time_ppm = u32::try_from(guaranteed_time_ppm)
        .map_err(|_| "guaranteed-time ratio is outside the ppm range".to_string())?;

    Ok(ServiceContinuityCertificate {
        evaluator_version: "0.8.0",
        service_id,
        boundary_id,
        horizon_start_unix_s: horizon_start,
        horizon_end_unix_s: horizon_end,
        restoration_confirmation_seconds,
        disposition,
        windows_total: u32::try_from(assessments.len())
            .map_err(|_| "window count exceeds u32".to_string())?,
        guaranteed_windows,
        confirmed_shortfall_windows,
        uncertain_windows,
        guaranteed_service_seconds,
        horizon_seconds,
        guaranteed_time_ppm,
        critical_shortfall_lower_wh,
        critical_shortfall_upper_wh,
        target_shortfall_lower_wh,
        target_shortfall_upper_wh,
        window_assessments: assessments,
        restoration_episodes: episodes,
        evidence_source_ids: source_ids.into_iter().collect(),
        upstream_artifact_ids: artifact_ids.into_iter().collect(),
    })
}

fn fixture_evidence(period_start: u64, period_end: u64) -> Evidence {
    Evidence {
        source_id: "fixture:port-backup-service".into(),
        source_uri: "urn:safrb:fixture:port-backup-service:v0.8".into(),
        evidence_class: EvidenceClass::Scenario,
        raw_sha256: None,
        upstream_artifact_id: None,
        measurement_start_unix_s: period_start,
        measurement_end_unix_s: period_end,
        recorded_at_unix_s: period_end,
    }
}

/// Four-window synthetic known-answer case; not a measurement of a port.
pub fn base_case() -> Result<ServiceContinuityCertificate, String> {
    let start = 1_800_000_000_u64;
    let readings = [(60, 60), (10, 15), (60, 65), (60, 65)];
    let mut windows = Vec::with_capacity(readings.len());
    for (index, (lower, upper)) in readings.into_iter().enumerate() {
        let window_start = start + (index as u64 * 3_600);
        let window_end = window_start + 3_600;
        windows.push(ServiceWindow {
            window_id: format!("window-{index}"),
            period_id: format!("hour-{index}"),
            service_id: "port-backup-electricity".into(),
            boundary_id: "critical-load-bus-a".into(),
            start_unix_s: window_start,
            duration_seconds: 3_600,
            critical_floor_wh: 50,
            target_wh: 80,
            delivered_lower_wh: lower,
            delivered_upper_wh: upper,
            evidence: fixture_evidence(window_start, window_end),
        });
    }
    evaluate(&windows, 7_200)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(index: u64, lower: u128, upper: u128) -> ServiceWindow {
        let start = 1_800_000_000 + index * 3_600;
        let end = start + 3_600;
        ServiceWindow {
            window_id: format!("w-{index}"),
            period_id: format!("p-{index}"),
            service_id: "hospital-backup".into(),
            boundary_id: "hospital-bus".into(),
            start_unix_s: start,
            duration_seconds: 3_600,
            critical_floor_wh: 50,
            target_wh: 80,
            delivered_lower_wh: lower,
            delivered_upper_wh: upper,
            evidence: fixture_evidence(start, end),
        }
    }

    #[test]
    fn known_answer_tracks_service_loss_and_sustained_restoration() {
        let certificate = base_case().expect("fixture must evaluate");
        assert_eq!(
            certificate.disposition,
            CertificateDisposition::ConfirmedCriticalServiceShortfall
        );
        assert_eq!(certificate.windows_total, 4);
        assert_eq!(certificate.guaranteed_windows, 3);
        assert_eq!(certificate.confirmed_shortfall_windows, 1);
        assert_eq!(certificate.uncertain_windows, 0);
        assert_eq!(certificate.critical_shortfall_lower_wh, 35);
        assert_eq!(certificate.critical_shortfall_upper_wh, 40);
        let episode = &certificate.restoration_episodes[0];
        assert_eq!(episode.restoration_latency_seconds, Some(3_600));
        assert_eq!(episode.confirmation_delay_seconds, Some(7_200));
        assert_eq!(episode.observed_duration_seconds, 10_800);
        assert!(!episode.right_censored);
    }

    #[test]
    fn all_windows_above_floor_support_continuity() {
        let windows = vec![window(0, 50, 55), window(1, 60, 65)];
        let result = evaluate(&windows, 7_200).expect("valid windows");
        assert_eq!(
            result.disposition,
            CertificateDisposition::ContinuitySupportedWithinHorizon
        );
        assert_eq!(result.guaranteed_time_ppm, 1_000_000);
        assert!(result.restoration_episodes.is_empty());
    }

    #[test]
    fn interval_crossing_floor_is_uncertain_not_guaranteed() {
        let windows = vec![window(0, 45, 55)];
        let result = evaluate(&windows, 3_600).expect("valid interval");
        assert_eq!(
            result.disposition,
            CertificateDisposition::ContinuityNotProven
        );
        assert_eq!(result.uncertain_windows, 1);
        assert_eq!(result.critical_shortfall_lower_wh, 0);
        assert_eq!(result.critical_shortfall_upper_wh, 5);
    }

    #[test]
    fn confirmed_shortfall_has_nonzero_lower_bound() {
        let windows = vec![window(0, 10, 20)];
        let result = evaluate(&windows, 3_600).expect("valid interval");
        assert_eq!(result.critical_shortfall_lower_wh, 30);
        assert_eq!(result.critical_shortfall_upper_wh, 40);
    }

    #[test]
    fn unresolved_episode_is_right_censored_at_horizon_end() {
        let windows = vec![window(0, 0, 5), window(1, 0, 5)];
        let result = evaluate(&windows, 7_200).expect("valid windows");
        let episode = &result.restoration_episodes[0];
        assert!(episode.right_censored);
        assert_eq!(episode.observed_duration_seconds, 7_200);
        assert_eq!(episode.recovery_confirmed_at_unix_s, None);
    }

    #[test]
    fn recovery_candidate_requires_full_sustained_duration() {
        let windows = vec![window(0, 0, 5), window(1, 60, 65)];
        let result = evaluate(&windows, 7_200).expect("valid windows");
        let episode = &result.restoration_episodes[0];
        assert!(episode.right_censored);
        assert_eq!(episode.recovery_candidate_start_unix_s, Some(1_800_003_600));
    }

    #[test]
    fn a_new_shortfall_resets_the_recovery_streak() {
        let windows = vec![
            window(0, 0, 5),
            window(1, 60, 65),
            window(2, 0, 5),
            window(3, 60, 65),
            window(4, 60, 65),
        ];
        let result = evaluate(&windows, 7_200).expect("valid windows");
        assert_eq!(result.restoration_episodes.len(), 1);
        let episode = &result.restoration_episodes[0];
        assert_eq!(episode.confirmed_shortfall_windows, 2);
        assert_eq!(episode.recovery_candidate_start_unix_s, Some(1_800_010_800));
        assert!(!episode.right_censored);
    }

    #[test]
    fn gapped_windows_fail_closed() {
        let mut second = window(1, 60, 65);
        second.start_unix_s += 60;
        second.evidence.measurement_start_unix_s += 60;
        second.evidence.measurement_end_unix_s += 60;
        second.evidence.recorded_at_unix_s += 60;
        assert!(evaluate(&[window(0, 60, 65), second], 3_600).is_err());
    }

    #[test]
    fn duplicate_window_identity_is_rejected() {
        let first = window(0, 60, 65);
        let mut second = window(1, 60, 65);
        second.window_id = first.window_id.clone();
        assert!(evaluate(&[first, second], 3_600).is_err());
    }

    #[test]
    fn duplicate_period_identity_is_rejected() {
        let first = window(0, 60, 65);
        let mut second = window(1, 60, 65);
        second.period_id = first.period_id.clone();
        assert!(evaluate(&[first, second], 3_600).is_err());
    }

    #[test]
    fn out_of_order_windows_are_rejected() {
        assert!(evaluate(&[window(1, 60, 65), window(0, 60, 65)], 3_600).is_err());
    }

    #[test]
    fn mixed_service_or_boundary_is_rejected() {
        let first = window(0, 60, 65);
        let mut second = window(1, 60, 65);
        second.service_id = "different-service".into();
        assert!(evaluate(&[first, second], 3_600).is_err());
    }

    #[test]
    fn reported_only_evidence_cannot_qualify() {
        let mut sample = window(0, 60, 65);
        sample.evidence.evidence_class = EvidenceClass::Reported;
        assert!(evaluate(&[sample], 3_600).is_err());
    }

    #[test]
    fn measured_evidence_requires_digest_and_upstream_artifact() {
        let mut sample = window(0, 60, 65);
        sample.evidence.evidence_class = EvidenceClass::Measured;
        sample.evidence.raw_sha256 = Some("a".repeat(64));
        sample.evidence.upstream_artifact_id = Some("safrb-v0.6:result-01".into());
        assert!(evaluate(std::slice::from_ref(&sample), 3_600).is_ok());

        sample.evidence.upstream_artifact_id = None;
        assert!(evaluate(&[sample], 3_600).is_err());
    }

    #[test]
    fn measured_evidence_requires_well_formed_digest() {
        let mut sample = window(0, 60, 65);
        sample.evidence.evidence_class = EvidenceClass::Measured;
        sample.evidence.raw_sha256 = Some("not-a-digest".into());
        sample.evidence.upstream_artifact_id = Some("safrb-v0.6:result-01".into());
        assert!(evaluate(&[sample], 3_600).is_err());
    }

    #[test]
    fn evidence_must_cover_the_whole_service_window() {
        let mut sample = window(0, 60, 65);
        sample.evidence.measurement_end_unix_s -= 1;
        assert!(evaluate(&[sample], 3_600).is_err());
    }

    #[test]
    fn lower_delivery_bound_cannot_exceed_upper_bound() {
        let mut sample = window(0, 60, 65);
        sample.delivered_lower_wh = 66;
        assert!(evaluate(&[sample], 3_600).is_err());
    }

    #[test]
    fn target_cannot_be_below_critical_floor() {
        let mut sample = window(0, 60, 65);
        sample.target_wh = 49;
        assert!(evaluate(&[sample], 3_600).is_err());
    }

    #[test]
    fn timestamp_overflow_is_rejected() {
        let mut sample = window(0, 60, 65);
        sample.start_unix_s = u64::MAX - 10;
        sample.evidence.measurement_start_unix_s = sample.start_unix_s;
        sample.evidence.measurement_end_unix_s = u64::MAX;
        sample.evidence.recorded_at_unix_s = u64::MAX;
        assert!(evaluate(&[sample], 3_600).is_err());
    }

    #[test]
    fn restoration_confirmation_must_be_positive() {
        assert!(evaluate(&[window(0, 60, 65)], 0).is_err());
    }
}

#[cfg(test)]
mod temporal_assurance_tests;
