pub mod ack;
pub mod dm_call;
pub mod fr_accepted;
pub mod fr_received;
pub mod generic;
pub mod mass_mention;
pub mod message;

use revolt_database::events::rabbit::PayloadToService;

const APN_DESKTOP_ENDPOINT: &str = "apn_desktop";
const APN_TOPIC_EXTRA: &str = "apn_topic";

pub fn is_apn_endpoint(endpoint: &str) -> bool {
    matches!(endpoint, "apn" | APN_DESKTOP_ENDPOINT)
}

pub fn set_apn_topic_extra(
    endpoint: &str,
    sendable: &mut PayloadToService,
    config: &revolt_config::Settings,
) {
    if endpoint == APN_DESKTOP_ENDPOINT {
        sendable.extras.insert(
            APN_TOPIC_EXTRA.to_string(),
            config.pushd.apn.desktop_topic.clone(),
        );
    }
}

pub fn routing_key_for_subscription<'a>(
    endpoint: &str,
    p256dh: String,
    sendable: &mut PayloadToService,
    config: &'a revolt_config::Settings,
) -> &'a str {
    match endpoint {
        "apn" | APN_DESKTOP_ENDPOINT => {
            set_apn_topic_extra(endpoint, sendable, config);
            &config.pushd.apn.queue
        }
        "fcm" => &config.pushd.fcm.queue,
        endpoint => {
            sendable.extras.insert("p256dh".to_string(), p256dh);
            sendable
                .extras
                .insert("endpoint".to_string(), endpoint.to_string());

            &config.pushd.vapid.queue
        }
    }
}
