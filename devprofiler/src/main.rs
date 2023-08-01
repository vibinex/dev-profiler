extern crate dotenv;
use dotenv::dotenv;
use std::env;
mod pubsub;
mod setup;
mod db;
mod review;

#[tokio::main]
async fn main() {
    dotenv().ok();
    // Get topic subscription and Listen to messages 
    let gcp_credentials = //"/home/tapishr/dev-profiler/pubsub-sa.json".to_owned();
    env::var("GCP_CREDENTIALS").expect("GCP_CREDENTIALS must be set");
    let topic_name = //"rtapish-fromserver".to_owned();
    env::var("TOPIC_NAME").expect("TOPIC_NAME must be set");
    let subscription_name = //"rtapish-fromserver-sub".to_owned();
    env::var("SUBSCRIPTION_NAME").expect("SUBSCRIPTION_NAME must be set");

    println!("env vars = {}, {}, {}", &gcp_credentials, &topic_name, &subscription_name);
    
    pubsub::listener::listen_messages(
        &gcp_credentials, 
        &topic_name,
        &subscription_name
    ).await;
}