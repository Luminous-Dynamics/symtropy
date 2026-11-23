use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StandardVersion {
    V2_2_0,
    V3_0_0,
    Unsupported,
}

impl StandardVersion {
    pub fn parse(value: &str) -> Self {
        match value {
            "2.2.0" => Self::V2_2_0,
            "3.0.0" => Self::V3_0_0,
            _ => Self::Unsupported,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventFamily {
    Shipment,
    Transport,
    Equipment,
    Iot,
    Reefer,
    Unsupported,
}

impl EventFamily {
    pub fn parse(value: &str) -> Self {
        match value {
            "SHIPMENT" => Self::Shipment,
            "TRANSPORT" => Self::Transport,
            "EQUIPMENT" => Self::Equipment,
            "IOT" => Self::Iot,
            "REEFER" => Self::Reefer,
            _ => Self::Unsupported,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileAccess {
    Public,
    MemberPartner,
    Unauthorized,
}

impl ProfileAccess {
    pub fn parse(value: &str) -> Self {
        match value {
            "PUBLIC" => Self::Public,
            "MEMBER_PARTNER" => Self::MemberPartner,
            _ => Self::Unauthorized,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Admission {
    SupportedV2_2,
    SupportedV3ProfileOnly,
    AccessRequired,
    ArtifactsNotPinned,
    UnsupportedVersion,
    UnsupportedEventFamily,
    UnsupportedForVersion,
}

impl Admission {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SupportedV2_2 => "SUPPORTED_V2_2_PROFILE_ONLY",
            Self::SupportedV3ProfileOnly => "SUPPORTED_V3_PROFILE_ONLY",
            Self::AccessRequired => "ACCESS_REQUIRED",
            Self::ArtifactsNotPinned => "ARTIFACTS_NOT_PINNED",
            Self::UnsupportedVersion => "UNSUPPORTED_VERSION",
            Self::UnsupportedEventFamily => "UNSUPPORTED_EVENT_FAMILY",
            Self::UnsupportedForVersion => "UNSUPPORTED_FOR_VERSION",
        }
    }
}

impl fmt::Display for Admission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ProfileEvidence<'a> {
    pub version: StandardVersion,
    pub access: ProfileAccess,
    pub artifact_sha256: Option<&'a str>,
    pub scope_ref: Option<&'a str>,
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn has_pinned_profile(profile: &ProfileEvidence<'_>) -> bool {
    profile.artifact_sha256.is_some_and(is_sha256_hex)
        && profile
            .scope_ref
            .is_some_and(|scope| !scope.trim().is_empty())
}

pub fn evaluate_profile(profile: &ProfileEvidence<'_>, family: EventFamily) -> Admission {
    if family == EventFamily::Unsupported {
        return Admission::UnsupportedEventFamily;
    }

    match profile.version {
        StandardVersion::Unsupported => Admission::UnsupportedVersion,
        StandardVersion::V2_2_0 => {
            let supported = matches!(
                family,
                EventFamily::Shipment | EventFamily::Transport | EventFamily::Equipment
            );
            if !supported {
                return Admission::UnsupportedForVersion;
            }
            if !matches!(
                profile.access,
                ProfileAccess::Public | ProfileAccess::MemberPartner
            ) {
                return Admission::AccessRequired;
            }
            if !has_pinned_profile(profile) {
                return Admission::ArtifactsNotPinned;
            }
            Admission::SupportedV2_2
        }
        StandardVersion::V3_0_0 => {
            if profile.access != ProfileAccess::MemberPartner {
                return Admission::AccessRequired;
            }
            if !matches!(
                family,
                EventFamily::Shipment
                    | EventFamily::Transport
                    | EventFamily::Equipment
                    | EventFamily::Iot
                    | EventFamily::Reefer
            ) {
                return Admission::UnsupportedForVersion;
            }
            if !has_pinned_profile(profile) {
                return Admission::ArtifactsNotPinned;
            }
            // This admits only a locally declared, authorized, pinned profile.
            // It is not a DCSA conformance result.
            Admission::SupportedV3ProfileOnly
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceClaimDisposition {
    ActualSourceAssertionOnly,
    PlannedNotPhysicalRealization,
    EstimatedNotPhysicalRealization,
    IndeterminateMissingEventTime,
    IndeterminateMissingClassifier,
    UnsupportedClassifier,
    RejectedMissingMessageId,
}

impl SourceClaimDisposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ActualSourceAssertionOnly => "ACTUAL_SOURCE_ASSERTION_ONLY",
            Self::PlannedNotPhysicalRealization => "PLANNED_NOT_PHYSICAL_REALIZATION",
            Self::EstimatedNotPhysicalRealization => "ESTIMATED_NOT_PHYSICAL_REALIZATION",
            Self::IndeterminateMissingEventTime => "INDETERMINATE_MISSING_EVENT_TIME",
            Self::IndeterminateMissingClassifier => "INDETERMINATE_MISSING_CLASSIFIER",
            Self::UnsupportedClassifier => "UNSUPPORTED_CLASSIFIER",
            Self::RejectedMissingMessageId => "REJECTED_MISSING_MESSAGE_ID",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhysicalRealization {
    NotAdjudicated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceClaimResult {
    pub disposition: SourceClaimDisposition,
    pub physical_realization: PhysicalRealization,
}

pub fn classify_source_claim(
    message_id: Option<&str>,
    event_date_time: Option<&str>,
    event_classifier_code: Option<&str>,
) -> SourceClaimResult {
    let disposition = if message_id.is_none_or(str::is_empty) {
        SourceClaimDisposition::RejectedMissingMessageId
    } else if event_classifier_code.is_none() {
        SourceClaimDisposition::IndeterminateMissingClassifier
    } else if event_date_time.is_none_or(str::is_empty) {
        SourceClaimDisposition::IndeterminateMissingEventTime
    } else {
        match event_classifier_code {
            Some("PLN") => SourceClaimDisposition::PlannedNotPhysicalRealization,
            Some("EST") => SourceClaimDisposition::EstimatedNotPhysicalRealization,
            Some("ACT") => SourceClaimDisposition::ActualSourceAssertionOnly,
            Some(_) => SourceClaimDisposition::UnsupportedClassifier,
            None => SourceClaimDisposition::IndeterminateMissingClassifier,
        }
    };

    SourceClaimResult {
        disposition,
        physical_realization: PhysicalRealization::NotAdjudicated,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventIdentityRoles<'a> {
    pub message_event_id: &'a str,
    pub subject_reference: Option<&'a str>,
}

pub fn identity_roles<'a>(
    message_event_id: &'a str,
    subject_reference: Option<&'a str>,
) -> EventIdentityRoles<'a> {
    EventIdentityRoles {
        message_event_id,
        subject_reference,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(
        version: &str,
        access: &str,
        digest: Option<&str>,
        scope: Option<&str>,
    ) -> ProfileEvidence<'_> {
        ProfileEvidence {
            version: StandardVersion::parse(version),
            access: ProfileAccess::parse(access),
            artifact_sha256: digest,
            scope_ref: scope,
        }
    }

    #[test]
    fn frozen_version_family_matrix_matches_tsv_oracle() {
        let cases = include_str!("../fixtures/version-gate-v0.1.tsv");
        let digest_v2 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let digest_v3 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

        for line in cases
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
        {
            let fields: Vec<&str> = line.split('|').collect();
            assert_eq!(fields.len(), 7, "bad fixture line: {line}");

            let digest = match fields[4] {
                "" => None,
                value if value == digest_v2 || value == digest_v3 || value == "not-a-digest" => {
                    Some(value)
                }
                _ => panic!("unexpected digest fixture value: {}", fields[4]),
            };
            let candidate = profile(
                fields[1],
                fields[3],
                digest,
                if fields[5].is_empty() {
                    None
                } else {
                    Some(fields[5])
                },
            );

            let result = evaluate_profile(&candidate, EventFamily::parse(fields[2]));
            assert_eq!(result.as_str(), fields[6], "case {}", fields[0]);
        }
    }

    #[test]
    fn actual_is_only_a_source_assertion_not_physical_realization() {
        let result =
            classify_source_claim(Some("message-1"), Some("2026-11-23T08:30:00Z"), Some("ACT"));
        assert_eq!(
            result.disposition,
            SourceClaimDisposition::ActualSourceAssertionOnly
        );
        assert_eq!(
            result.physical_realization,
            PhysicalRealization::NotAdjudicated
        );
    }

    #[test]
    fn planned_and_estimated_are_never_promoted_to_actual() {
        assert_eq!(
            classify_source_claim(Some("message-2"), Some("2026-11-24T08:30:00Z"), Some("PLN"))
                .disposition,
            SourceClaimDisposition::PlannedNotPhysicalRealization
        );
        assert_eq!(
            classify_source_claim(Some("message-3"), Some("2026-11-24T08:30:00Z"), Some("EST"))
                .disposition,
            SourceClaimDisposition::EstimatedNotPhysicalRealization
        );
    }

    #[test]
    fn message_creation_time_cannot_replace_missing_event_time() {
        // eventCreatedDateTime is deliberately not an argument to this function.
        assert_eq!(
            classify_source_claim(Some("message-4"), None, Some("ACT")).disposition,
            SourceClaimDisposition::IndeterminateMissingEventTime
        );
    }

    #[test]
    fn missing_or_unknown_source_classifier_does_not_become_actual() {
        assert_eq!(
            classify_source_claim(Some("message-5"), Some("2026-11-23T08:30:00Z"), None)
                .disposition,
            SourceClaimDisposition::IndeterminateMissingClassifier
        );
        assert_eq!(
            classify_source_claim(Some("message-6"), Some("2026-11-23T08:30:00Z"), Some("FUT"))
                .disposition,
            SourceClaimDisposition::UnsupportedClassifier
        );
    }

    #[test]
    fn missing_event_time_is_indeterminate_for_planned_and_estimated_claims() {
        assert_eq!(
            classify_source_claim(Some("message-7"), None, Some("PLN")).disposition,
            SourceClaimDisposition::IndeterminateMissingEventTime
        );
        assert_eq!(
            classify_source_claim(Some("message-8"), None, Some("EST")).disposition,
            SourceClaimDisposition::IndeterminateMissingEventTime
        );
    }

    #[test]
    fn missing_message_id_is_rejected() {
        assert_eq!(
            classify_source_claim(None, Some("2026-11-23T08:30:00Z"), Some("ACT")).disposition,
            SourceClaimDisposition::RejectedMissingMessageId
        );
    }

    #[test]
    fn event_message_identity_is_distinct_from_subject_reference() {
        let roles = identity_roles("event-message-77", Some("MSCU1234567"));
        assert_eq!(roles.message_event_id, "event-message-77");
        assert_eq!(roles.subject_reference, Some("MSCU1234567"));
        assert_ne!(roles.message_event_id, roles.subject_reference.unwrap());
    }

    #[test]
    fn malformed_digest_is_not_a_pinned_profile() {
        let candidate = profile(
            "2.2.0",
            "PUBLIC",
            Some("not-a-digest"),
            Some("DCSA-TNT-2.2.0-CONSUMER"),
        );
        assert_eq!(
            evaluate_profile(&candidate, EventFamily::Shipment),
            Admission::ArtifactsNotPinned
        );
    }

    #[test]
    fn v3_does_not_admit_under_public_access() {
        let candidate = profile(
            "3.0.0",
            "PUBLIC",
            Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            Some("DCSA-TNT-3.0.0-MEMBER"),
        );
        assert_eq!(
            evaluate_profile(&candidate, EventFamily::Iot),
            Admission::AccessRequired
        );
    }
}
