// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic, synthetic view-model data. Nothing here represents live offers or access claims.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Intent {
    pub id: &'static str,
    pub symbol: &'static str,
    pub label: &'static str,
    pub detail: &'static str,
}

pub const INTENTS: [Intent; 4] = [
    Intent { id: "rest", symbol: "☾", label: "Rest and recharge", detail: "A slower rhythm, room to breathe" },
    Intent { id: "together", symbol: "♡", label: "Time together", detail: "Shared memories, not a packed schedule" },
    Intent { id: "adventure", symbol: "↟", label: "Adventure", detail: "Explore, move, and try something new" },
    Intent { id: "nature", symbol: "❧", label: "Nature and discovery", detail: "Landscapes, wildlife, and learning" },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pace { Unhurried, Balanced, Full }

impl Pace {
    pub const ALL: [Pace; 3] = [Pace::Unhurried, Pace::Balanced, Pace::Full];

    pub const fn id(self) -> &'static str {
        match self { Pace::Unhurried => "unhurried", Pace::Balanced => "balanced", Pace::Full => "full" }
    }

    pub const fn label(self) -> &'static str {
        match self { Pace::Unhurried => "Unhurried", Pace::Balanced => "Balanced", Pace::Full => "Full days" }
    }

    pub const fn detail(self) -> &'static str {
        match self {
            Pace::Unhurried => "One anchor, generous breaks",
            Pace::Balanced => "A few activities and breathing room",
            Pace::Full => "More to do, with buffers kept visible",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Destination {
    pub id: &'static str,
    pub name: &'static str,
    pub region: &'static str,
    pub art_class: &'static str,
    pub symbol: &'static str,
    pub headline: &'static str,
    pub summary: &'static str,
    pub estimate: &'static str,
    pub access_label: &'static str,
    pub access_note: &'static str,
    pub resource_note: &'static str,
    pub duration: &'static str,
}

pub const DESTINATIONS: [Destination; 3] = [
    Destination {
        id: "moss", name: "Moss & Tide", region: "Coastal garden · concept destination",
        art_class: "coast", symbol: "≋", headline: "Quiet coast, soft mornings",
        summary: "A slow coastal stay with garden paths, local food, and room for unplanned time.",
        estimate: "R 8,900", access_label: "Access details incomplete",
        access_note: "Step-free routes and room transfer details are unknown in this synthetic record.",
        resource_note: "Freshwater and sensitive shoreline limits require site-specific evidence.",
        duration: "3 nights · synthetic estimate per person",
    },
    Destination {
        id: "wonder", name: "Wonderwater", region: "All-weather play · concept destination",
        art_class: "water", symbol: "≈", headline: "Make-believe, warm water, shared play",
        summary: "A story-led water and indoor-play concept with dry alternatives and quiet periods.",
        estimate: "R 10,400", access_label: "Access unknown",
        access_note: "Pool transfer, sensory conditions, and attraction-by-attraction access are not verified.",
        resource_note: "Pool health, water, and energy requirements are hard constraints, not trade-offs.",
        duration: "3 nights · synthetic estimate per person",
    },
    Destination {
        id: "highland", name: "Highland Light", region: "Mountain retreat · concept destination",
        art_class: "mountain", symbol: "⌁", headline: "Big skies, gentle trails, fireside evenings",
        summary: "A mountain base with low- and higher-effort options, local food, and weather backups.",
        estimate: "R 9,650", access_label: "Route profile unverified",
        access_note: "Gradient, surface, altitude, and accessible transport need local confirmation.",
        resource_note: "Weather, habitat, and seasonal route closures need qualified local review.",
        duration: "3 nights · synthetic estimate per person",
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Activity {
    pub id: &'static str,
    pub time: &'static str,
    pub title: &'static str,
    pub detail: &'static str,
    pub effort: &'static str,
    pub kind: &'static str,
}

pub const ACTIVITIES: [Activity; 4] = [
    Activity { id: "garden", time: "09:30", title: "Slow garden wander", detail: "A self-paced route with an easy turn-back point", effort: "Low effort · 60–90 min", kind: "Nature" },
    Activity { id: "food", time: "12:30", title: "Local lunch table", detail: "A relaxed meal with time to linger", effort: "Low effort · 75 min", kind: "Food" },
    Activity { id: "makers", time: "15:00", title: "Indoor makers’ studio", detail: "A weather-friendly creative alternative", effort: "Optional · 90 min", kind: "Creative" },
    Activity { id: "lookout", time: "16:00", title: "Coastal lookout", detail: "A short outing if conditions and access suit", effort: "Variable · site evidence needed", kind: "Outdoors" },
];

pub fn toggle_choice(values: &mut Vec<&'static str>, value: &'static str) {
    if let Some(index) = values.iter().position(|existing| *existing == value) {
        values.remove(index);
    } else {
        values.push(value);
    }
}

pub fn fit_reason(destination_id: &str, intents: &[&'static str], pace: Pace) -> &'static str {
    let seeks_rest = intents.contains(&"rest");
    let seeks_connection = intents.contains(&"together");
    let seeks_adventure = intents.contains(&"adventure");
    let seeks_nature = intents.contains(&"nature");

    match destination_id {
        "moss" if seeks_rest && seeks_nature =>
            "Your selected themes include rest and nature. Check the access unknowns and shoreline constraints before treating this as a fit.",
        "moss" if seeks_rest =>
            "The synthetic concept is designed around a slower rhythm. Its step-free routes and room details remain unverified.",
        "wonder" if seeks_connection || seeks_adventure =>
            "The synthetic concept emphasizes shared play. Attraction-specific access and water-safety information are not verified.",
        "highland" if seeks_adventure || seeks_nature =>
            "The synthetic concept offers nature and outdoor options, but route difficulty and weather suitability require local evidence.",
        "highland" if pace == Pace::Unhurried =>
            "This concept has both low- and higher-effort options, but the gentler routes must be verified for the actual site.",
        _ if pace == Pace::Unhurried =>
            "You prefer a gentler rhythm. Compare transfer effort and keep open time; the estimates and venue data here are synthetic.",
        _ => "This is a design hypothesis, not a ranked recommendation. Review price assumptions, access evidence, and site constraints before choosing.",
    }
}

pub fn day_load_label(pace: Pace, selected_activities: usize) -> &'static str {
    match (pace, selected_activities) {
        (_, 0) => "Open day · nothing scheduled",
        (Pace::Unhurried, n) if n > 2 => "Likely too packed for an unhurried day",
        (Pace::Balanced, n) if n > 3 => "Busy day · keep transfer buffers visible",
        (Pace::Full, n) if n >= 3 => "Full day · check effort and recovery time",
        (Pace::Unhurried, _) => "Gentle rhythm · keep the gaps",
        (Pace::Balanced, _) => "Balanced rhythm · optional space remains",
        (Pace::Full, _) => "Room for another option, if you want one",
    }
}

pub fn recovery_message(choice: Option<&'static str>) -> &'static str {
    match choice {
        Some("rest") => "Chosen: protect the afternoon. No replacement purchase; keep the original booking state unchanged.",
        Some("makers") => "Chosen: consider the indoor makers’ studio. Its availability is still unconfirmed; illustrative extra cost: R 230.",
        Some("defer") => "Chosen: move the outing to tomorrow in this draft only. Check tomorrow’s opening and weather before confirming.",
        _ => "Choose a recovery path. Nothing is booked or changed outside this draft.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_choice_adds_and_removes_without_duplicates() {
        let mut values = vec!["rest"];
        toggle_choice(&mut values, "nature");
        assert_eq!(values, vec!["rest", "nature"]);
        toggle_choice(&mut values, "nature");
        assert_eq!(values, vec!["rest"]);
    }

    #[test]
    fn all_options_disclose_non_verified_access() {
        assert_eq!(DESTINATIONS.len(), 3);
        assert!(DESTINATIONS.iter().all(|item| {
            !item.access_label.is_empty() && !item.access_note.is_empty()
                && (item.access_label.to_ascii_lowercase().contains("unknown")
                    || item.access_label.to_ascii_lowercase().contains("incomplete")
                    || item.access_label.to_ascii_lowercase().contains("unverified"))
        }));
    }

    #[test]
    fn no_activities_is_always_an_allowed_plan() {
        assert_eq!(day_load_label(Pace::Unhurried, 0), "Open day · nothing scheduled");
        assert_eq!(day_load_label(Pace::Full, 0), "Open day · nothing scheduled");
    }

    #[test]
    fn unhurried_pace_flags_overpacked_days() {
        assert!(day_load_label(Pace::Unhurried, 3).contains("too packed"));
    }

    #[test]
    fn closure_has_a_no_purchase_recovery_path() {
        assert!(recovery_message(Some("rest")).contains("No replacement purchase"));
    }

    #[test]
    fn recovery_does_not_claim_live_confirmation() {
        assert!(recovery_message(Some("makers")).contains("unconfirmed"));
        assert!(recovery_message(Some("defer")).contains("draft only"));
    }
}
