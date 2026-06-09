use std::{collections::HashMap, sync::Arc};

use super::routing_key_for_subscription;
use crate::utils::Consumer;
use anyhow::Result;
use async_trait::async_trait;
use lapin::{message::Delivery, Channel, Connection};
use log::debug;
use revolt_database::{events::rabbit::*, Database};

#[derive(Clone)]
#[allow(unused)]
pub struct FRAcceptedConsumer {
    db: Database,
    authifier_db: authifier::Database,
    connection: Arc<Connection>,
    channel: Arc<Channel>,
}

#[async_trait]
impl Consumer for FRAcceptedConsumer {
    async fn create(
        db: Database,
        authifier_db: authifier::Database,
        connection: Arc<Connection>,
        channel: Arc<Channel>,
    ) -> Self {
        Self {
            db,
            authifier_db,
            connection,
            channel,
        }
    }

    fn channel(&self) -> &Arc<Channel> {
        &self.channel
    }

    /// This consumer handles delegating messages into their respective platform queues.
    async fn consume(&self, delivery: Delivery) -> Result<()> {
        let payload: FRAcceptedPayload = serde_json::from_slice(&delivery.data)?;

        debug!("Received FR accept event");

        if let Ok(sessions) = self.authifier_db.find_sessions(&payload.user).await {
            let config = revolt_config::config().await;
            for session in sessions {
                if let Some(sub) = session.subscription {
                    let mut sendable = PayloadToService {
                        notification: PayloadKind::FRAccepted(payload.clone()),
                        token: sub.auth,
                        user_id: session.user_id,
                        session_id: session.id,
                        extras: HashMap::new(),
                    };

                    let routing_key = routing_key_for_subscription(
                        sub.endpoint.as_str(),
                        sub.p256dh,
                        &mut sendable,
                        &config,
                    );

                    let payload = serde_json::to_string(&sendable)?;

                    self.publish_message(payload.as_bytes(), &config.pushd.exchange, routing_key)
                        .await?;
                }
            }
        }

        Ok(())
    }
}
