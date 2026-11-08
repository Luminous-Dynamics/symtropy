//! Dependence-safe graded evidence for foot-ground interaction.
//! The aggregator preserves each calibrated source interval and combines
//! eligible sources only by convex hull. It never multiplies likelihoods or
//! assumes independence. It does not estimate forces, apply Coulomb friction,
//! validate calibration coverage, or authorize robot actions.

use std::collections::BTreeMap;

pub const SCALE_PPM: u32 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContactChannel {
    InterfaceFields,             // D1: pressure, shear, contact geometry
    NearFootWrenchVibroAcoustic, // D2: wrench and acoustic transients
    ProprioceptiveResidual,      // D3: joint/body dynamics
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceTarget {
    ContactPresence,
    NoSlipKinematicConstraintValidity,
    TractionFeasibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReliabilityInterval {
    pub lower_ppm: u32,
    pub upper_ppm: u32,
}

impl ReliabilityInterval {
    pub fn new(lower_ppm: u32, upper_ppm: u32) -> Result<Self, EvidenceError> {
        if lower_ppm > upper_ppm || upper_ppm > SCALE_PPM {
            return Err(EvidenceError::InvalidInterval);
        }
        Ok(Self { lower_ppm, upper_ppm })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRequirement {
    pub channel: ContactChannel,
    pub calibration_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidencePolicy {
    pub foot_id: String,
    pub event_id: String,
    pub target: EvidenceTarget,
    pub constraint_digest: String,
    pub domain_digest: String,
    pub now_ns: u64,
    pub clock_uncertain: bool,
    pub maximum_age_ns: u64,
    pub maximum_skew_ns: u64,
    pub minimum_sources: usize,
    pub required_sources: BTreeMap<String, SourceRequirement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceEvidence {
    pub source_id: String,
    pub channel: ContactChannel,
    pub foot_id: String,
    pub event_id: String,
    pub target: EvidenceTarget,
    pub constraint_digest: String,
    pub domain_digest: String,
    pub calibration_digest: String,
    pub observed_at_ns: Option<u64>,
    /// Estimator-supplied bounds; this crate does not prove their coverage.
    pub reliability: ReliabilityInterval,
    pub healthy: bool,
    pub out_of_distribution: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceError {
    EmptyIdentity,
    NoRequiredSources,
    InvalidMinimumSources,
    EmptySourceId,
    EmptyCalibrationDigest,
    DuplicateSource(String),
    InvalidInterval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceDisposition {
    Qualified,
    UnknownSource,
    ChannelMismatch,
    CalibrationMismatch,
    ContextMismatch,
    Unhealthy,
    OutOfDistribution,
    TimestampMissing,
    TimestampFuture,
    Stale,
    ClockUncertain,
    CrossSourceSkew,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAssessment {
    pub source_id: String,
    pub channel: ContactChannel,
    pub disposition: SourceDisposition,
    pub reliability: ReliabilityInterval,
    pub observed_at_ns: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agreement {
    CommonIntersection,
    NoCommonIntersection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeStatus {
    Unknown,
    SingleSource,
    MultiSource,
    Disagreement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReliabilityEnvelope {
    /// Convex hull of every eligible source interval.
    pub lower_ppm: u32,
    pub upper_ppm: u32,
    pub source_count: usize,
    /// Diagnostic only; never substituted for the hull.
    pub common_intersection: Option<ReliabilityInterval>,
    pub agreement: Agreement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactEvidenceReport {
    pub status: EnvelopeStatus,
    pub foot_id: String,
    pub event_id: String,
    pub target: EvidenceTarget,
    pub constraint_digest: String,
    pub domain_digest: String,
    pub sources: Vec<SourceAssessment>,
    pub qualified_source_ids: Vec<String>,
    pub envelope: Option<ReliabilityEnvelope>,
    pub timestamp_skew_ns: Option<u64>,
}

fn nonblank(s: &str) -> bool { !s.trim().is_empty() }

fn validate_policy(p: &EvidencePolicy) -> Result<(), EvidenceError> {
    if !nonblank(&p.foot_id) || !nonblank(&p.event_id)
        || !nonblank(&p.constraint_digest) || !nonblank(&p.domain_digest) {
        return Err(EvidenceError::EmptyIdentity);
    }
    if p.required_sources.is_empty() { return Err(EvidenceError::NoRequiredSources); }
    if p.minimum_sources == 0 || p.minimum_sources > p.required_sources.len() {
        return Err(EvidenceError::InvalidMinimumSources);
    }
    for (id, req) in &p.required_sources {
        if !nonblank(id) { return Err(EvidenceError::EmptySourceId); }
        if !nonblank(&req.calibration_digest) { return Err(EvidenceError::EmptyCalibrationDigest); }
    }
    Ok(())
}

fn assess(p: &EvidencePolicy, e: &SourceEvidence) -> SourceDisposition {
    let Some(req) = p.required_sources.get(&e.source_id) else {
        return SourceDisposition::UnknownSource;
    };
    if e.channel != req.channel { return SourceDisposition::ChannelMismatch; }
    if e.calibration_digest != req.calibration_digest { return SourceDisposition::CalibrationMismatch; }
    if e.foot_id != p.foot_id || e.event_id != p.event_id || e.target != p.target
        || e.constraint_digest != p.constraint_digest || e.domain_digest != p.domain_digest {
        return SourceDisposition::ContextMismatch;
    }
    if !e.healthy { return SourceDisposition::Unhealthy; }
    if e.out_of_distribution { return SourceDisposition::OutOfDistribution; }
    if p.clock_uncertain { return SourceDisposition::ClockUncertain; }
    match e.observed_at_ns {
        None => SourceDisposition::TimestampMissing,
        Some(t) if t > p.now_ns => SourceDisposition::TimestampFuture,
        Some(t) if p.now_ns - t > p.maximum_age_ns => SourceDisposition::Stale,
        Some(_) => SourceDisposition::Qualified,
    }
}

fn unknown_report(p: &EvidencePolicy, sources: Vec<SourceAssessment>, skew: Option<u64>) -> ContactEvidenceReport {
    ContactEvidenceReport {
        status: EnvelopeStatus::Unknown,
        foot_id: p.foot_id.clone(),
        event_id: p.event_id.clone(),
        target: p.target,
        constraint_digest: p.constraint_digest.clone(),
        domain_digest: p.domain_digest.clone(),
        sources,
        qualified_source_ids: Vec::new(),
        envelope: None,
        timestamp_skew_ns: skew,
    }
}

/// Combine eligible evidence by convex hull, never by intersection or
/// likelihood multiplication. If time is uncertain, evidence is stale, or
/// source timestamps are too far apart, the aggregate is Unknown.
pub fn aggregate(p: &EvidencePolicy, input: &[SourceEvidence]) -> Result<ContactEvidenceReport, EvidenceError> {
    validate_policy(p)?;
    let mut seen = BTreeMap::<String, ()>::new();
    for e in input {
        if !nonblank(&e.source_id) { return Err(EvidenceError::EmptySourceId); }
        ReliabilityInterval::new(e.reliability.lower_ppm, e.reliability.upper_ppm)?;
        if seen.insert(e.source_id.clone(), ()).is_some() {
            return Err(EvidenceError::DuplicateSource(e.source_id.clone()));
        }
    }

    let mut sources = input.iter().map(|e| SourceAssessment {
        source_id: e.source_id.clone(),
        channel: e.channel,
        disposition: assess(p, e),
        reliability: e.reliability,
        observed_at_ns: e.observed_at_ns,
    }).collect::<Vec<_>>();
    sources.sort_by(|a,b| a.source_id.cmp(&b.source_id));

    if p.clock_uncertain { return Ok(unknown_report(p, sources, None)); }

    let mut qualified = sources.iter().filter(|s| s.disposition == SourceDisposition::Qualified)
        .cloned().collect::<Vec<_>>();
    if qualified.is_empty() { return Ok(unknown_report(p, sources, None)); }

    let min_t = qualified.iter().filter_map(|s| s.observed_at_ns).min().expect("qualified source has time");
    let max_t = qualified.iter().filter_map(|s| s.observed_at_ns).max().expect("qualified source has time");
    let skew = max_t - min_t;
    if skew > p.maximum_skew_ns {
        for s in &mut sources {
            if s.disposition == SourceDisposition::Qualified { s.disposition = SourceDisposition::CrossSourceSkew; }
        }
        return Ok(unknown_report(p, sources, Some(skew)));
    }
    if qualified.len() < p.minimum_sources { return Ok(unknown_report(p, sources, Some(skew))); }

    qualified.sort_by(|a,b| a.source_id.cmp(&b.source_id));
    let lo = qualified.iter().map(|s| s.reliability.lower_ppm).min().expect("non-empty");
    let hi = qualified.iter().map(|s| s.reliability.upper_ppm).max().expect("non-empty");
    let common_lo = qualified.iter().map(|s| s.reliability.lower_ppm).max().expect("non-empty");
    let common_hi = qualified.iter().map(|s| s.reliability.upper_ppm).min().expect("non-empty");
    let common = (common_lo <= common_hi).then_some(ReliabilityInterval { lower_ppm: common_lo, upper_ppm: common_hi });
    let agreement = if common.is_some() { Agreement::CommonIntersection } else { Agreement::NoCommonIntersection };
    let status = if common.is_none() { EnvelopeStatus::Disagreement }
        else if qualified.len() == 1 { EnvelopeStatus::SingleSource }
        else { EnvelopeStatus::MultiSource };
    let ids = qualified.iter().map(|s| s.source_id.clone()).collect();
    Ok(ContactEvidenceReport {
        status,
        foot_id: p.foot_id.clone(),
        event_id: p.event_id.clone(),
        target: p.target,
        constraint_digest: p.constraint_digest.clone(),
        domain_digest: p.domain_digest.clone(),
        sources,
        qualified_source_ids: ids,
        envelope: Some(ReliabilityEnvelope { lower_ppm: lo, upper_ppm: hi, source_count: qualified.len(), common_intersection: common, agreement }),
        timestamp_skew_ns: Some(skew),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(channel: ContactChannel, digest: &str) -> SourceRequirement {
        SourceRequirement { channel, calibration_digest: digest.to_owned() }
    }
    fn policy() -> EvidencePolicy {
        EvidencePolicy {
            foot_id: "front-left".into(), event_id: "step-42".into(),
            target: EvidenceTarget::NoSlipKinematicConstraintValidity,
            constraint_digest: "constraint-v3".into(), domain_digest: "terrain-hard-v1".into(),
            now_ns: 1_000, clock_uncertain: false, maximum_age_ns: 100,
            maximum_skew_ns: 20, minimum_sources: 1,
            required_sources: BTreeMap::from([
                ("d1".into(), req(ContactChannel::InterfaceFields,"cal-d1")),
                ("d2".into(), req(ContactChannel::NearFootWrenchVibroAcoustic,"cal-d2")),
                ("d3".into(), req(ContactChannel::ProprioceptiveResidual,"cal-d3")),
            ]),
        }
    }
    fn src(id: &str, channel: ContactChannel, cal: &str, lo: u32, hi: u32, t: Option<u64>) -> SourceEvidence {
        SourceEvidence {
            source_id:id.into(), channel, foot_id:"front-left".into(), event_id:"step-42".into(),
            target:EvidenceTarget::NoSlipKinematicConstraintValidity,
            constraint_digest:"constraint-v3".into(), domain_digest:"terrain-hard-v1".into(),
            calibration_digest:cal.into(), observed_at_ns:t,
            reliability:ReliabilityInterval::new(lo,hi).unwrap(), healthy:true, out_of_distribution:false,
        }
    }
    fn d1(lo:u32,hi:u32,t:Option<u64>)->SourceEvidence { src("d1",ContactChannel::InterfaceFields,"cal-d1",lo,hi,t) }
    fn d2(lo:u32,hi:u32,t:Option<u64>)->SourceEvidence { src("d2",ContactChannel::NearFootWrenchVibroAcoustic,"cal-d2",lo,hi,t) }
    fn d3(lo:u32,hi:u32,t:Option<u64>)->SourceEvidence { src("d3",ContactChannel::ProprioceptiveResidual,"cal-d3",lo,hi,t) }

    #[test] fn one_source_is_explicitly_single_source() {
        let r=aggregate(&policy(),&[d1(800_000,900_000,Some(995))]).unwrap();
        assert_eq!(r.status,EnvelopeStatus::SingleSource);
        assert_eq!(r.envelope.unwrap().lower_ppm,800_000);
    }
    #[test] fn d1_d2_d3_use_hull_not_intersection() {
        let r=aggregate(&policy(),&[d1(800_000,900_000,Some(995)),d2(700_000,950_000,Some(990)),d3(850_000,920_000,Some(998))]).unwrap();
        assert_eq!(r.status,EnvelopeStatus::MultiSource);
        let e=r.envelope.unwrap();
        assert_eq!((e.lower_ppm,e.upper_ppm),(700_000,950_000));
        assert_eq!(e.common_intersection,Some(ReliabilityInterval{lower_ppm:850_000,upper_ppm:900_000}));
    }
    #[test] fn disjoint_sources_flag_disagreement_and_keep_hull() {
        let r=aggregate(&policy(),&[d1(800_000,900_000,Some(995)),d2(200_000,400_000,Some(990))]).unwrap();
        assert_eq!(r.status,EnvelopeStatus::Disagreement);
        let e=r.envelope.unwrap();
        assert_eq!((e.lower_ppm,e.upper_ppm),(200_000,900_000));
        assert_eq!(e.common_intersection,None);
    }
    #[test] fn minimum_source_count_defers_without_losing_assessments() {
        let mut p=policy(); p.minimum_sources=2;
        let r=aggregate(&p,&[d1(700_000,900_000,Some(995))]).unwrap();
        assert_eq!(r.status,EnvelopeStatus::Unknown);
        assert_eq!(r.sources[0].disposition,SourceDisposition::Qualified);
    }
    #[test] fn stale_high_confidence_source_is_excluded() {
        let r=aggregate(&policy(),&[d1(990_000,1_000_000,Some(899))]).unwrap();
        assert_eq!(r.sources[0].disposition,SourceDisposition::Stale);
        assert_eq!(r.status,EnvelopeStatus::Unknown);
    }
    #[test] fn unhealthy_or_ood_source_is_excluded() {
        let mut a=d1(800_000,900_000,Some(995)); a.healthy=false;
        let mut b=d2(700_000,950_000,Some(995)); b.out_of_distribution=true;
        let r=aggregate(&policy(),&[a,b]).unwrap();
        assert!(r.sources.iter().all(|s| s.disposition!=SourceDisposition::Qualified));
    }
    #[test] fn calibration_and_context_mismatch_are_excluded() {
        let mut a=d1(800_000,900_000,Some(995)); a.calibration_digest="other".into();
        let mut b=d2(700_000,950_000,Some(995)); b.foot_id="rear-right".into();
        let r=aggregate(&policy(),&[a,b]).unwrap();
        assert!(r.sources.iter().any(|s| s.disposition==SourceDisposition::CalibrationMismatch));
        assert!(r.sources.iter().any(|s| s.disposition==SourceDisposition::ContextMismatch));
    }
    #[test] fn wrong_target_does_not_alias_contact_presence_to_no_slip() {
        let mut a=d1(900_000,990_000,Some(995)); a.target=EvidenceTarget::ContactPresence;
        let r=aggregate(&policy(),&[a]).unwrap();
        assert_eq!(r.sources[0].disposition,SourceDisposition::ContextMismatch);
    }
    #[test] fn unknown_source_and_wrong_channel_are_excluded() {
        let unknown=src("camera",ContactChannel::InterfaceFields,"cal-d1",800_000,900_000,Some(995));
        let wrong=src("d1",ContactChannel::ProprioceptiveResidual,"cal-d1",800_000,900_000,Some(995));
        let r=aggregate(&policy(),&[unknown,wrong]).unwrap();
        assert!(r.sources.iter().any(|s| s.disposition==SourceDisposition::UnknownSource));
        assert!(r.sources.iter().any(|s| s.disposition==SourceDisposition::ChannelMismatch));
    }
    #[test] fn duplicate_source_is_rejected() {
        let e=aggregate(&policy(),&[d1(700_000,900_000,Some(995)),d1(800_000,950_000,Some(996))]);
        assert_eq!(e,Err(EvidenceError::DuplicateSource("d1".into())));
    }
    #[test] fn missing_or_future_time_is_not_qualified() {
        let a=aggregate(&policy(),&[d1(800_000,900_000,None)]).unwrap();
        assert_eq!(a.sources[0].disposition,SourceDisposition::TimestampMissing);
        let b=aggregate(&policy(),&[d1(800_000,900_000,Some(1_001))]).unwrap();
        assert_eq!(b.sources[0].disposition,SourceDisposition::TimestampFuture);
    }
    #[test] fn uncertain_clock_suppresses_aggregation() {
        let mut p=policy(); p.clock_uncertain=true;
        let r=aggregate(&p,&[d1(800_000,900_000,Some(995))]).unwrap();
        assert_eq!(r.status,EnvelopeStatus::Unknown);
        assert_eq!(r.sources[0].disposition,SourceDisposition::ClockUncertain);
    }
    #[test] fn excessive_source_skew_suppresses_aggregation() {
        let r=aggregate(&policy(),&[d1(800_000,900_000,Some(970)),d2(700_000,950_000,Some(999))]).unwrap();
        assert_eq!(r.status,EnvelopeStatus::Unknown);
        assert_eq!(r.timestamp_skew_ns,Some(29));
        assert!(r.sources.iter().all(|s|s.disposition==SourceDisposition::CrossSourceSkew));
    }
    #[test] fn age_and_skew_boundaries_are_inclusive() {
        let r=aggregate(&policy(),&[d1(800_000,900_000,Some(900)),d2(700_000,950_000,Some(920))]).unwrap();
        assert_eq!(r.status,EnvelopeStatus::MultiSource);
        assert_eq!(r.timestamp_skew_ns,Some(20));
    }
    #[test] fn confidence_interval_must_be_ordered_and_bounded() {
        assert_eq!(ReliabilityInterval::new(900_000,800_000),Err(EvidenceError::InvalidInterval));
        assert_eq!(ReliabilityInterval::new(0,1_000_001),Err(EvidenceError::InvalidInterval));
    }
    #[test] fn malformed_policy_is_rejected() {
        let mut p=policy(); p.foot_id.clear();
        assert_eq!(aggregate(&p,&[]),Err(EvidenceError::EmptyIdentity));
        let mut p=policy(); p.minimum_sources=0;
        assert_eq!(aggregate(&p,&[]),Err(EvidenceError::InvalidMinimumSources));
    }
    #[test] fn order_of_input_sources_does_not_change_report() {
        let a=d1(800_000,900_000,Some(995)); let b=d2(700_000,950_000,Some(990));
        assert_eq!(aggregate(&policy(),&[a.clone(),b.clone()]).unwrap(),aggregate(&policy(),&[b,a]).unwrap());
    }
    #[test] fn gait_phase_alone_is_not_a_contact_channel() {
        let r=aggregate(&policy(),&[]).unwrap();
        assert_eq!(r.status,EnvelopeStatus::Unknown);
        assert!(r.envelope.is_none());
    }
}
