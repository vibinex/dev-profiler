use std::collections::HashMap;

use futures_util::StreamExt;
use google_cloud_auth::credentials::CredentialsFile;
use google_cloud_default::WithAuthExt;
use google_cloud_pubsub::{
    client::{Client, ClientConfig},
    subscription::{SubscriptionConfig, Subscription},
};
use serde::Deserialize;
use serde_json::Value;
use tokio::task;
use std::collections::VecDeque;
use sha256::digest;
use crate::setup::bitbucket::handle_install_bitbucket;
use crate::review::reviewer::process_review;

#[derive(Debug, Deserialize)]
struct InstallCallback {
    repository_provider: String,
    installation_code: String,
}


pub async fn get_pubsub_client_config(keypath: &str) -> ClientConfig {
    let credfile = CredentialsFile::new_from_file(keypath.to_string()).await.expect("Failed to locate credentials file");
    return ClientConfig::default()
        .with_credentials(credfile)
        .await
        .unwrap();
}


async fn setup_subscription(keypath: &str, topicname: &str, subscriptionname: &str) -> Subscription{
    let config = get_pubsub_client_config(keypath).await;
    let client = Client::new(config).await
        .expect("Unable to create pubsub client to listen to messages");
    let topic = client.topic(topicname);
    let subconfig = SubscriptionConfig {
        enable_message_ordering: true,
        ..Default::default()
    };
    let subscription = client.subscription(subscriptionname);
    if !subscription.exists(None).await.expect("Unable to get subscription information") {
        subscription.create(
            topic.fully_qualified_name(), subconfig, None)
            .await.expect("Unable to create subscription for listening to messages");
    }
    subscription
}
pub async fn listen_messages(keypath: &str, topicname: &str, subscriptionname: &str) {
    let queue_cap = 100;
    let mut message_hashes = VecDeque::with_capacity(queue_cap);
    let subscription = setup_subscription(keypath, topicname, subscriptionname).await;
    let mut stream = subscription.subscribe(None).await
        .expect("Unable to subscribe to messages");
    while let Some(message) = stream.next().await {
        println!("Listening for messages...");
        let attrmap: HashMap<String, String> = message.message.attributes.clone().into_iter().collect();
        let message_hash = digest(&*message.message.data);
        if !message_hashes.contains(&message_hash) {
            message_hashes.push_back(message_hash);
            if message_hashes.len() > queue_cap {
                while message_hashes.len() > queue_cap {
                    message_hashes.pop_front();
                }
            }
            match attrmap.get("msgtype") {
                Some(msgtype) => {
                    match msgtype.as_str() {
                        "install_callback" => {
                            println!("Processing install callback message");
                            let data_bytes: Vec<u8> = message.message.data.clone();
                            match serde_json::from_slice::<InstallCallback>(&data_bytes) {
                                Ok(data) => {
                                    let code_async = data.installation_code.clone();
                                    task::spawn(async move {
                                        handle_install_bitbucket(&code_async).await;
                                        println!("Processed install callback message");
                                    });
                                },
                                Err(e) => eprintln!("Error deserializing install callback: {e}"),
                            };
                        },
                        "webhook_callback" => {
                            let data_bytes: Vec<u8> = message.message.data.clone();
                            task::spawn(async move {
                                process_review(&data_bytes).await;
                            });
                        }
                        _ => {
                            eprintln!("Message type not found for message : {:?}", message.message.attributes);
                        }
                    };
                },
                None => {
                    eprintln!("Message type not found for message : {:?}", message.message.attributes);
                }
            };
        }
        // Ack or Nack message.
        let _ = message.ack().await;
    }
}
