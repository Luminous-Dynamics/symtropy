// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

const APP_WEBSOCKET_GAP: &str = "authenticated Holochain AppWebSocket transport is not implemented; a raw WebSocket connection is not a zome-call receipt";

/// Relay configuration.
///
/// `conductor_url` is retained for the future authenticated transport adapter.
/// This scaffold deliberately does not open a raw WebSocket and report it as a
/// usable Holochain connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayConfig {
    pub conductor_url: String,
    pub mode: RelayMode,
    pub auto_connect: bool,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            conductor_url: "ws://localhost:4444".to_string(),
            mode: RelayMode::Shadow,
            auto_connect: false,
        }
    }
}

/// Relay operating intent.
///
/// Neither mode can make an unimplemented transport authoritative. In
/// particular, `Authoritative` must not cause the caller to treat an action as
/// committed without a verified result from the configured Holochain zome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelayMode {
    Shadow,
    Authoritative,
}

/// Game action to forward to the conductor.
///
/// This is a domain-level proposal only. A real adapter must map each variant
/// to a versioned Mycelix zome input and must not serialize this enum directly
/// as though it were the Holochain App API wire protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameAction {
    CastVote {
        proposal_id: String,
        voter_id: String,
        phi_score: f64,
        approve: bool,
    },
    TendExchange {
        provider_id: String,
        receiver_id: String,
        amount: i64,
    },
}

/// Result from the relay.
///
/// `success` means an operation was actually dispatched and its completion
/// was verified. It must never mean only that a TCP/WebSocket handshake worked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayResult {
    pub success: bool,
    pub response: Option<String>,
}

/// Connection state. `Unsupported` is distinct from a network failure: the
/// current implementation intentionally fails closed because it does not yet
/// perform AppWebSocket authentication, zome-call signing, MessagePack framing,
/// or application-response verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Unsupported(String),
    Failed(String),
}

/// Holochain relay boundary.
///
/// This crate is a fail-closed integration scaffold, **not** a production
/// Holochain client. Use the supported Holochain client protocol and an
/// authorized signer before enabling action dispatch.
pub struct HolochainRelay {
    pub config: RelayConfig,
    state: Arc<Mutex<ConnectionState>>,
}

impl Default for HolochainRelay {
    fn default() -> Self {
        Self {
            config: RelayConfig::default(),
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
        }
    }
}

impl HolochainRelay {
    pub fn new(config: RelayConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
        }
    }

    /// Return a read-only snapshot of the relay state.
    pub async fn connection_state(&self) -> ConnectionState {
        self.state.lock().await.clone()
    }

    /// Establish an authenticated Holochain AppWebSocket session.
    ///
    /// This deliberately refuses to treat a plain WebSocket handshake as
    /// conductor authentication. A future implementation must use an
    /// AppAuthenticationToken, an authorized zome-call signer, the Holochain
    /// MessagePack request/response protocol, and exact app/role routing.
    pub async fn connect(&self) -> Result<(), String> {
        {
            let mut state = self.state.lock().await;
            *state = ConnectionState::Connecting;
            *state = ConnectionState::Unsupported(APP_WEBSOCKET_GAP.to_string());
        }
        Err(APP_WEBSOCKET_GAP.to_string())
    }

    /// Forward a game action to a Mycelix zome.
    ///
    /// No action is dispatched until an authenticated AppWebSocket adapter is
    /// implemented. Both Shadow and Authoritative modes refuse success here so
    /// downstream systems cannot mistake a placeholder response for a durable
    /// Holochain commit.
    pub async fn send_action(&self, _action: &GameAction) -> RelayResult {
        let message = format!("action not dispatched: {APP_WEBSOCKET_GAP}");
        let mut state = self.state.lock().await;
        *state = ConnectionState::Unsupported(message.clone());
        RelayResult {
            success: false,
            response: Some(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_refuses_to_equate_raw_websocket_with_holochain_authentication() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        runtime.block_on(async {
            let relay = HolochainRelay::default();
            let error = relay
                .connect()
                .await
                .expect_err("unimplemented AppWebSocket must fail closed");
            assert!(error.contains("AppWebSocket"));
            assert_eq!(
                relay.connection_state().await,
                ConnectionState::Unsupported(error)
            );
        });
    }

    #[test]
    fn no_action_reports_success_without_a_verified_zome_dispatch() {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        runtime.block_on(async {
            for mode in [RelayMode::Shadow, RelayMode::Authoritative] {
                let relay = HolochainRelay::new(RelayConfig {
                    mode,
                    ..RelayConfig::default()
                });
                let result = relay
                    .send_action(&GameAction::CastVote {
                        proposal_id: "proposal-test".to_string(),
                        voter_id: "agent-test".to_string(),
                        phi_score: 0.75,
                        approve: true,
                    })
                    .await;
                assert!(!result.success, "mode {mode:?} must not fake delivery");
                assert!(
                    result
                        .response
                        .as_deref()
                        .is_some_and(|v| v.contains("not dispatched")),
                    "failure must explain the missing dispatch"
                );
                assert!(matches!(
                    relay.connection_state().await,
                    ConnectionState::Unsupported(_)
                ));
            }
        });
    }
}
