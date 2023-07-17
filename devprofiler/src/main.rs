use std::env;

mod listener;
mod setup;

#[tokio::main]
async fn main() {
    // Get topic subscription and Listen to messages 
    let gcp_credentials = env::var("GCP_CREDENTIALS").expect("GCP_CREDENTIALS must be set");
    let topic_name = env::var("TOPIC_NAME").expect("TOPIC_NAME must be set");
    let subscription_name = env::var("SUBSCRIPTION_NAME").expect("SUBSCRIPTION_NAME must be set");

    println!("Listening for messages...");
    
    listener::listen_messages(
        &gcp_credentials, 
        &topic_name,
        &subscription_name
    ).await;

}