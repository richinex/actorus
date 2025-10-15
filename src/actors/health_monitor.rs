use crate::actors::messages::*;
use crate::config::Settings;
use std::collections::HashMap;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::{timeout, Duration, Instant};

pub async fn health_monitor_actor(
    mut receiver: Receiver<RoutingMessage>,
    router_sender: Sender<RoutingMessage>,
    settings: Settings,
) {
    let mut heartbeats: HashMap<ActorType, Instant> = HashMap::new();
    let timeout_duration = Duration::from_millis(settings.system.check_interval_ms);
    let check_interval = Duration::from_millis(settings.system.heartbeat_timeout_ms);

    tracing::info!("Health Monitor actor started");

    loop {
        match timeout(timeout_duration, receiver.recv()).await {
            Ok(Some(message)) => match message {
                RoutingMessage::Heartbeat(actor_type) => {
                    heartbeats.insert(actor_type, Instant::now());
                    tracing::debug!("Heartbeat received from {:?}", actor_type);
                }
                // ✅ Handle GetState requests
                RoutingMessage::GetState(response_tx) => {
                    let snapshot = create_snapshot(&heartbeats, check_interval);
                    let _ = response_tx.send(snapshot);
                }
                RoutingMessage::Shutdown => {
                    tracing::info!("Health Monitor received shutdown signal");
                    break;
                }
                _ => {}
            },
            Ok(None) => {
                tracing::info!("Health Monitor channel closed");
                break;
            }
            Err(_) => {
                check_actor_health(&heartbeats, check_interval, &router_sender).await;
            }
        }
    }
}

// ✅ Create snapshot function
fn create_snapshot(
    heartbeats: &HashMap<ActorType, Instant>,
    check_interval: Duration,
) -> StateSnapshot {
    let now = Instant::now();
    let cutoff = now - check_interval;

    let mut active_actors = HashMap::new();
    let mut last_heartbeat = HashMap::new();

    for (actor_type, heartbeat_time) in heartbeats.iter() {
        let is_active = *heartbeat_time >= cutoff;
        active_actors.insert(*actor_type, is_active);
        last_heartbeat.insert(*actor_type, *heartbeat_time);
    }

    StateSnapshot {
        active_actors,
        last_heartbeat,
    }
}

async fn check_actor_health(
    heartbeats: &HashMap<ActorType, Instant>,
    check_interval: Duration,
    router_sender: &Sender<RoutingMessage>,
) {
    let now = Instant::now();
    let cutoff = now - check_interval;

    for (actor_type, last_heartbeat) in heartbeats.iter() {
        if *last_heartbeat < cutoff {
            let elapsed = now.duration_since(*last_heartbeat);
            tracing::warn!(
                "Actor {:?} has not sent heartbeat in {:?}. Requesting reset.",
                actor_type,
                elapsed
            );

            if let Err(e) = router_sender.send(RoutingMessage::Reset(*actor_type)).await {
                tracing::error!("Failed to send Reset message for {:?}: {}", actor_type, e);
            }
        }
    }
}
