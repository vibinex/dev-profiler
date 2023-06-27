use google_cloud_auth::credentials::CredentialsFile;
use google_cloud_default::WithAuthExt;
use google_cloud_googleapis::pubsub::v1::PubsubMessage;
use serde::{Serialize, Deserialize};
use serde_json;
use std::collections::HashMap;
use futures_util::StreamExt;
use serde_json::Result as JsonResult;
use google_cloud_pubsub::{
    client::{Client, ClientConfig},
    subscription::SubscriptionConfig,
};

mod reviewer;
use crate::reviewer::unfinished_tasks;
use crate::reviewer::Reviews;
mod observer;
use crate::observer::RuntimeInfo;
#[derive(Serialize, Deserialize, Debug)]
struct GitUrl {
    git_urls: HashMap<String, String>,
}

async fn send_message_to_topic(config: ClientConfig, topicname: &str, message: &str, msgtype: &str) {
    let client = Client::new(config).await.expect("Unable to create pubsub client to publish messages");
    let mut pbmsg = PubsubMessage {
        data: message.into(),
        ..Default::default()
    };
    pbmsg.attributes.insert("msg_type".to_string(), msgtype.to_string());
    let topic = client.topic(topicname);
    let publisher = topic.new_publisher(None);
    match publisher.publish(pbmsg).await.get().await {
        Ok(_) => {
            println!("Message published successfully");
        }
        Err(err) => {
            eprintln!("Failed to publish message: {:?}", err);
        }
    }
}

async fn request_git_urls(repo_owner: &str, keypath: &str) {
    let topicname = "vibi-crun";
    let mut msgmap = HashMap::<String, String>::new();
    msgmap.insert("user_token".to_string(), repo_owner.to_string());
    let message = serde_json::to_string(&msgmap).expect("Failed to serialize git get request");
    let config = get_pubsub_client_config(keypath).await;
    send_message_to_topic(config, topicname, &message, "GetGitUrl").await;
}

async fn clone_git_repo(git_urls: HashMap<String, String>) {
    for (repo_name, git_url) in git_urls {
		let mut git_url = git_url;
		if git_url.contains("git://") {
			git_url = git_url.replace("git://", "git@");
			let count = git_url.matches('/').count();
			if count > 1 {
				git_url = git_url.replacen("/", ":", 1);
			}
		}
        let directory = "/home/tapishr/testdp";
        let mut cmd = std::process::Command::new("git");
		cmd.env("GIT_SSH_COMMAND", "ssh -i /home/tapishr/.ssh/id_gitkey");
        cmd.arg("clone").arg(git_url).current_dir(directory);
        let output = cmd.output().expect("Failed to clone git repo");
        println!("Git clone output: {:?}", output);
    }
}

async fn listen_messages(keypath: &str, topicname: &str, subscriptionname: &str, publishtopic: &str, einfo: &mut RuntimeInfo) {
    let config = get_pubsub_client_config(keypath).await;
    let client = Client::new(config).await.expect("Unable to create pubsub client to listen to messages");
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
    let mut stream = subscription.subscribe(None).await.expect("Unable to subscribe to messages");
    let mut repo_list: Vec<String> = Vec::new();
    while let Some(message) = stream.next().await {
        let attrmap: HashMap<String, String> = message.message.attributes.clone().into_iter().collect();
        match attrmap.get("msg_type") {
            Some(msgtype) => {
                match msgtype.as_str() {
                    "GitUrl" => {
                        // Convert the data from base64 to a string
                        let payload = String::from_utf8(message.message.data.clone()).expect("Failed to convert GitUrl msg to string");

                        // Deserialize the JSON payload into a struct
                        println!("GitUrl message: {:?}", payload);
                        let result: JsonResult<GitUrl> = serde_json::from_str(&payload);

                        let giturls = result.expect("Failed to deserialize GitUrl message").git_urls;
                        if giturls.len() > 0 {
                            clone_git_repo(giturls.clone()).await;
                            for k in giturls.keys() {
                                repo_list.push(k.clone());
                            }
                        }
                        else {
                            eprintln!("No git urls found for user");
                        }
                    }
                    "PRMessage" => {
                        let payload = String::from_utf8(message.message.data.clone()).unwrap();
                        let result: JsonResult<Reviews> = serde_json::from_str(&payload);
                        match result {
                            Ok(prmsg) => {
                                let repo_key = prmsg.repo_slug.clone();
                                println!("repo_list: {:?}", repo_list);
                                println!("repo_key: {:?}", repo_key);
                                if repo_list.contains(&repo_key) {
									// create a Command to pull in the directory with the name repo_key
									let mut cmd = std::process::Command::new("git");
									cmd.arg("pull").current_dir(format!("/home/tapishr/testdp/{}", repo_key));
                                    let hunks = unfinished_tasks(prmsg, repo_key.as_str(), einfo);
                                    let message = serde_json::to_string(&hunks).expect("Failed to serialize Hunks");
                                    send_message_to_topic(
                                        get_pubsub_client_config(keypath).await,
                                        publishtopic, &message, "HunkInfo").await;
                                }
                                else {
                                    eprintln!("Repo not found in repo list");
                                }
                            }
                            Err(err) => {
                                eprintln!("Failed to deserialize PRMessage: {:?}", err);
                            }
                        }
                    }
                    _ => {
                        eprintln!("Message type not found for message : {:?}", message.message.attributes);
                    }
                };
            },
            None => {
                eprintln!("Message type not found for message : {:?}", message.message.data);
            }
        };
        // Ack or Nack message.
        let _ = message.ack().await;
    }
}

#[tokio::main]
async fn main() {
    // let url = "https://gcscruncsql-k7jns52mtq-el.a.run.app/onprem/authenticate";

    // // Create a reqwest client
    // let client = reqwest::Client::new();

    // // Send a POST request to the authentication endpoint
    // let response = client.post(url).send().await
    //     .expect("Failed to send Vibinex authentication request");
    
    let einfo = &mut RuntimeInfo::new();

    // Check if the request was successful
    // if response.status().is_success() {
        // Get the service account JSON key from the response body
        // let service_account_key = response.text().await
        //     .expect("Failed to get service account key");
        println!("Authentication successful!");
        // Use the service account key for authentication with GCP Pub/Sub and get the client object
        let keypath = "/home/tapishr/dev-profiler/pubsub-sa.json".to_string();
        // dump_pubsub_key(service_account_key);
        let repo_owner = "rtapish".to_string();
        // env::var("REPO_OWNER").expect("Missing REPO_OWNER environment variable");
        // let git_token = env::var("GIT_TOKEN").expect("Missing GIT_TOKEN environment variable");
        // let pubsub_topic = env::var("PUBSUB_TOPIC").expect("Missing PUBSUB_TOPIC environment variable");
        // let provider = env::var("PROVIDER").expect("Missing PROVIDER environment variable");
        let kpclone = keypath.clone();
        let ownerclone = repo_owner.clone();
        tokio::spawn(async move {
            request_git_urls(&ownerclone, kpclone.as_str()).await;
        });
        listen_messages(&keypath,
            format!("{}-fromserver", repo_owner).as_str(),
            format!("{}-fromserver-sub", repo_owner).as_str(),
            "vibi-crun",
            einfo).await;

        // Keep the main thread alive
        tokio::signal::ctrl_c().await.unwrap();
    // } else {
    //     println!("Vibinex Authentication failed with status code: {}", response.status());
    // }
}

async fn get_pubsub_client_config(keypath: &str) -> ClientConfig {
    let credfile = CredentialsFile::new_from_file(keypath.to_string()).await.expect("Failed to locate credentials file");
    return ClientConfig::default()
        .with_credentials(credfile)
        .await
        .unwrap();
}

fn dump_pubsub_key(service_account_key: String) -> String {
    // Dump service account key to /tmp/service_account.json and return the path
    let keypath = "/tmp/service_account.json";
    std::fs::write(keypath, service_account_key).expect("Failed to write service account key to file");
    return String::from(keypath);
}
