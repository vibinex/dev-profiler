use google_cloud_pubsub::client::{Client, ClientConfig};
use google_cloud_googleapis::pubsub::v1::PubsubMessage;
use google_cloud_pubsub::topic::TopicConfig;
use google_cloud_pubsub::subscription::SubscriptionConfig;

use std::env;
use std::collections::HashMap;
use crate::pubsub::listener::get_pubsub_client_config;


pub async fn publish_message(message: Vec<u8>, msgtype: &str) {

    let keypath = "vibi-crun-sub".to_string();//env::var("GCP_CREDENTIALS").expect("GCP_CREDENTIALS must be set");
    let topic_name = "vibi-crun-sub".to_string();//env::var("TOPIC_NAME").expect("TOPIC_NAME must be set");
    let config = get_pubsub_client_config(&keypath).await;

    let client = Client::new(config).await.unwrap();

    let topic = client.topic(&topic_name);

    let mut attributes = HashMap::new();
    attributes.insert("msgtype".to_owned(), msgtype.to_owned());

    let msg = PubsubMessage {
        data: message,
        attributes: attributes,
        ..Default::default()
    };
    let publisher = topic.new_publisher(None);
    println!("Publishing message...");
    match publisher.publish(msg).await.get().await {
        Ok(_) => println!("Message published"),
        Err(err) => { eprintln!("Message not pulished: {err}"); }
    };

}
