use safrb_service_continuity::{CertificateDisposition, base_case};

fn main() -> Result<(), String> {
    let certificate = base_case()?;
    println!("metric,value,unit");
    println!("service_id,{},label", certificate.service_id);
    println!("boundary_id,{},label", certificate.boundary_id);
    println!("horizon_seconds,{},s", certificate.horizon_seconds);
    println!("windows_total,{},count", certificate.windows_total);
    println!(
        "guaranteed_windows,{},count",
        certificate.guaranteed_windows
    );
    println!(
        "confirmed_shortfall_windows,{},count",
        certificate.confirmed_shortfall_windows
    );
    println!("uncertain_windows,{},count", certificate.uncertain_windows);
    println!(
        "guaranteed_time_ppm,{},ppm",
        certificate.guaranteed_time_ppm
    );
    println!(
        "critical_shortfall_lower,{},Wh",
        certificate.critical_shortfall_lower_wh
    );
    println!(
        "critical_shortfall_upper,{},Wh",
        certificate.critical_shortfall_upper_wh
    );
    println!(
        "target_shortfall_lower,{},Wh",
        certificate.target_shortfall_lower_wh
    );
    println!(
        "target_shortfall_upper,{},Wh",
        certificate.target_shortfall_upper_wh
    );
    println!("disposition,{:?},label", certificate.disposition);
    if certificate.disposition == CertificateDisposition::ConfirmedCriticalServiceShortfall {
        println!("# Synthetic fixture demonstrates a confirmed critical-service shortfall.");
    }
    for (index, episode) in certificate.restoration_episodes.iter().enumerate() {
        println!(
            "episode_{index}_start,{},unix_s",
            episode.episode_start_unix_s
        );
        println!(
            "episode_{index}_recovery_candidate_start,{},unix_s",
            episode
                .recovery_candidate_start_unix_s
                .map_or_else(|| "none".into(), |value| value.to_string())
        );
        println!(
            "episode_{index}_restoration_latency,{},s",
            episode
                .restoration_latency_seconds
                .map_or_else(|| "unconfirmed".into(), |value| value.to_string())
        );
        println!(
            "episode_{index}_confirmed_at,{},unix_s",
            episode
                .recovery_confirmed_at_unix_s
                .map_or_else(|| "unconfirmed".into(), |value| value.to_string())
        );
        println!(
            "episode_{index}_confirmation_delay,{},s",
            episode
                .confirmation_delay_seconds
                .map_or_else(|| "unconfirmed".into(), |value| value.to_string())
        );
        println!(
            "episode_{index}_observed_duration,{},s",
            episode.observed_duration_seconds
        );
        println!(
            "episode_{index}_confirmed_shortfall_windows,{},count",
            episode.confirmed_shortfall_windows
        );
        println!(
            "episode_{index}_uncertain_windows,{},count",
            episode.uncertain_windows
        );
        println!(
            "episode_{index}_critical_shortfall_lower,{},Wh",
            episode.critical_shortfall_lower_wh
        );
        println!(
            "episode_{index}_critical_shortfall_upper,{},Wh",
            episode.critical_shortfall_upper_wh
        );
        println!(
            "episode_{index}_right_censored,{},bool",
            episode.right_censored
        );
    }
    println!("# Synthetic fixture only; not a physical port or facility measurement.");
    Ok(())
}
