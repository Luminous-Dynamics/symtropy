// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Network configuration — signaling, STUN/TURN, transport settings.
//!
//! URLs are configurable so game studios deploying Symtropy can use their
//! own infrastructure. No public signaling default is provided: the relay expects
//! Symtropy's JSON protocol, which is not the Holochain SBD wire protocol.

use serde::{Deserialize, Serialize};

/// Network configuration for Symtropy multiplayer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// URL of a server implementing Symtropy's JSON signaling protocol.
    /// Set SYMTROPY_SIGNAL_URL; no compatible public server is configured by default.
    pub signal_url: String,

    /// STUN URL reserved for a future ICE/WebRTC transport.
    /// The current WebSocket JSON relay does not consult this setting.
    pub stun_url: String,

    /// Optional TURN URL reserved for a future ICE/WebRTC transport.
    /// The current WebSocket JSON relay does not use TURN; `None` does not
    /// imply direct P2P or guarantee any fallback transport.
    pub turn_url: Option<String>,

    /// TURN credentials reserved for a future ICE/WebRTC transport.
    /// The current WebSocket JSON relay does not consume these fields.
    pub turn_username: Option<String>,
    pub turn_credential: Option<String>,

    /// Maximum peers to connect to simultaneously.
    pub max_peers: usize,

    /// Physics state send rate (Hz). Higher = smoother but more bandwidth.
    pub send_rate_hz: u32,

    /// Authority transfer timeout (ms). If a peer doesn't respond
    /// within this time, their bodies become unclaimed.
    pub authority_timeout_ms: u64,

    /// Room/lobby identifier for matchmaking.
    pub room_id: Option<String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            // Do not default to Holochain SBD: it uses an authenticated binary
            // protocol, not Symtropy's JSON join/Welcome/relay_data protocol.
            // An empty value makes an unconfigured deployment fail explicitly.
            signal_url: std::env::var("SYMTROPY_SIGNAL_URL").unwrap_or_default(),
            stun_url: std::env::var("SYMTROPY_STUN_URL")
                .unwrap_or_else(|_| "stun:stun.l.google.com:19302".to_string()),
            turn_url: std::env::var("SYMTROPY_TURN_URL").ok(),
            turn_username: std::env::var("SYMTROPY_TURN_USER").ok(),
            turn_credential: std::env::var("SYMTROPY_TURN_CRED").ok(),
            max_peers: 8,
            send_rate_hz: 20,
            authority_timeout_ms: 5000,
            room_id: None,
        }
    }
}

impl NetworkConfig {
    /// Create a config for local testing (no real signaling).
    pub fn local_test() -> Self {
        Self {
            signal_url: "ws://localhost:9001".to_string(),
            stun_url: "stun:stun.l.google.com:19302".to_string(),
            turn_url: None,
            turn_username: None,
            turn_credential: None,
            max_peers: 4,
            send_rate_hz: 20,
            authority_timeout_ms: 3000,
            room_id: Some("test-room".to_string()),
        }
    }

    /// Create a config with an explicitly supplied Symtropy signal endpoint.
    /// The STUN value is reserved for a future ICE/WebRTC transport.
    pub fn self_hosted(signal: &str, stun: &str) -> Self {
        Self {
            signal_url: signal.to_string(),
            stun_url: stun.to_string(),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NetworkConfig;

    #[test]
    fn default_signaling_endpoint_is_explicitly_configured() {
        // Defaults must not silently select an unrelated public signaling protocol.
        // Preserve explicit runtime configuration while leaving the unconfigured
        // default empty and fail-closed.
        let expected = std::env::var("SYMTROPY_SIGNAL_URL").unwrap_or_default();
        assert_eq!(NetworkConfig::default().signal_url, expected);
    }
}
