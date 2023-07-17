use std::collections::HashMap;
use reqwest::Client;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;
use serde_json::json;
use sled::IVec;
use tokio::task;


#[derive(Debug, Serialize, Deserialize)]
struct Workspace {
    name: String,
    uuid: String,
    slug: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Webhook {
    uuid: String,
    active: bool,
    created_at: String,
    events: Vec<String>,
    ping_url: String,
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Repository {
    name: String,
    uuid: String,
    owner: String,
    is_private: bool,
    clone_ssh_url: String,
    project: String,
    workspace: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PullRequest {

}

#[derive(Debug, Deserialize, Serialize)]
struct AuthInfo {
    access_token: String,
    refresh_token: String,
    expires_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WebhookResponse {
    uuid: String,
    active: bool,
    url: String,
    created_at: String,
    events: Vec<String>,
    links: HashMap<String, HashMap<String,String>>,
}

const BITBUCKET_API_BASE_URL: &str = "https://api.bitbucket.org/2.0";

async fn add_pull_request_webhook_to_auth_repo_bitbucket(
    workspace_slug: &str, 
    repo_slug: &str, 
    access_token: &str
) {

    let url = format!(
        "{}/repositories/{}/{}/hooks", 
        BITBUCKET_API_BASE_URL, workspace_slug, repo_slug
    );

    let mut headers_map = HeaderMap::new();
    let auth_header = format!("Bearer {}", access_token);
    let headervalres = HeaderValue::from_str(&auth_header);
    match headervalres {
        Ok(headerval) => {
            headers_map.insert("Authorization", headerval);
        },
        Err(e) => panic!("Could not parse header value: {}", e),
    };
    headers_map.insert("Accept", HeaderValue::from_static("application/vnd.github+json"));

    let payload = json!({
        "description": "Webhook for PRs when raised and when something is pushed to the open PRs",
        "url": "https://gcscruncsql-k7jns52mtq-el.a.run.app/handle_bitbucket_pr_webhook",
        "active": true,
        "events": ["pullrequest:created", "pullrequest:updated"] 
    });

    let response = reqwest::Client::new()
        .post(&url)
        .headers(headers_map)
        .json(&payload)
        .send()
        .await;

    match response {
        Ok(res) => {
            if !res.status().is_success() {
                // return None;
            }
            match res.json::<WebhookResponse>().await {
                Ok(webhook) => {
                    let webhook_data = Webhook {
                        uuid: webhook.uuid,
                        active: webhook.active,
                        url: webhook.url,
                        created_at: webhook.created_at,
                        events: webhook.events,
                        ping_url: webhook.links["self"]["href"].clone(),
                    };
                    save_webhook_to_db(&webhook_data);      
                },
                Err(err) => {

                }
            };
        },
        Err(_) => todo!(),
    };
}

async fn get_access_token_from_bitbucket(code: &str) -> Option<AuthInfo> {
    let client = Client::new();
    let bitbucket_client_id = "raFykYJRvEBHPttQAm".to_string();
    // std::env::var("BITBUCKET_CLIENT_ID").unwrap();
    let bitbucket_client_secret = "cZBfwZqzvgW9kemcrwyMy3szwnCX3pba".to_string();
    // std::env::var("BITBUCKET_CLIENT_SECRET").unwrap();
    let mut params = std::collections::HashMap::new();
    params.insert("client_id", bitbucket_client_id);
    params.insert("client_secret", bitbucket_client_secret);
    params.insert("code", code.to_owned());
    params.insert("grant_type", "authorization_code".to_owned());
    params.insert("redirect_uri", 
    "https://ea89-171-76-82-143.ngrok-free.app/api/bitbucket/callbacks/install"
    // "https://gcscruncsql-k7jns52mtq-el.a.run.app/authorise_bitbucket_consumer"
    .to_owned());

    let response = client
        .post("https://bitbucket.org/site/oauth2/access_token")
        .form(&params)
        .send()
        .await;
    match response {
        Ok(res) => {
            if !res.status().is_success() {
                println!(
                    "Failed to exchange code for access token. Status code: {}, Response content: {}",
                    res.status(),
                    res.text().await.expect("No text in response")
                );
                return None;
            }
        
            match res.json::<AuthInfo>().await { Ok(response_json) => {
                save_auth_info_to_db(&response_json);
                return Some(response_json);
            }, Err(e) => {
                println!("error : {:?}", e);
                return None;} };
        },
        Err(e) => {
            println!("error : {:?}", e);
            return None},
    }
}

fn save_auth_info_to_db(auth_info: &AuthInfo) {
    let db = sled::open("/tmp/db").expect("Failed to open sled database");
    let json = serde_json::to_string(&auth_info).expect("Failed to serialize auth info");
    // Convert JSON string to bytes
    let bytes = json.as_bytes(); 

    // Create IVec from bytes
    let ivec = IVec::from(bytes);

    // Insert into sled DB
    db.insert("bitbucket_auth_info", ivec).expect("Failed to insert auth info in sled database");
}

fn save_workspace_to_db(workspace: &Workspace) {
    let uuid = workspace.uuid.clone();
    let db = sled::open("/tmp/db").expect("Failed to open sled database");
    let json = serde_json::to_string(&workspace).expect("Failed to serialize workspace");
    // Convert JSON string to bytes
    let bytes = json.as_bytes(); 

    // Create IVec from bytes
    let ivec = IVec::from(bytes);
    db.insert(format!("workspaces:{}", uuid), ivec).expect("Unable to save workspace in db");  
}

async fn get_bitbucket_workspaces(access_token: &str) -> Option<Vec<Workspace>> {
    let user_url = format!("{}/workspaces", BITBUCKET_API_BASE_URL);
    let headers = {
        let mut headers = HeaderMap::new();
        headers.insert("Accept", "application/json".parse().unwrap());
        headers.insert("Authorization", format!("Bearer {}", access_token).parse().unwrap());
        headers
      };
    
    let response = reqwest::Client::new()
        .get(user_url)
        .headers(headers)
        .send()
        .await.expect("Failed to get workspaces");

    if !response.status().is_success() {
    panic!("Failed to retrieve current user's workspaces. Status code: {}, Response content: {}", 
        response.status(), 
        response.text().await.expect("Failed to parse response text")) 
    };
    let response_json = response.json::<serde_json::Value>().await.expect("Failed to parse response as JSON");
    match response_json["values"].as_array() {
        Some(values) => {let workspace_vec = values
          .iter()
          .map(|workspace_json| {
            let val = serde_json::from_value::<Workspace>(workspace_json.clone()).expect("Unable to deserialize workspace");
            save_workspace_to_db(&val);
            val
          })
          .collect();
          return Some(workspace_vec);  
        },
        None => {return None},
      };
    
}

fn save_repo_to_db(repo: &Repository) {
    let db = sled::open("/tmp/db").expect("Failed to open sled database");
    // Generate unique ID
    let uuid = Uuid::new_v4();
    let id = uuid.as_bytes();
  
    // Serialize repo struct to JSON 
    let json = serde_json::to_vec(repo).expect("Failed to serialize repo");
  
    // Insert JSON into sled DB
    db.insert(IVec::from(id), json).expect("Failed to insert repo into sled DB");
}

async fn get_workspace_repos(workspace: &str, access_token: &str) -> Option<Vec<Repository>> {
    let repos_url = format!("{}/repositories/{}", BITBUCKET_API_BASE_URL, workspace);
    let client = reqwest::Client::new();
    let mut headers = reqwest::header::HeaderMap::new(); 
    headers.insert( reqwest::header::AUTHORIZATION, 
        format!("Bearer {}", access_token).parse().unwrap(), );
    let mut response = client.get(&repos_url).headers(headers.clone()).send().await.unwrap();
    let mut repos_data = Vec::new();
    if !response.status().is_success() {
        panic!( "Failed to retrieve current user's repositories for the workspace {}. Status code: {}, Response content: {}",
        workspace, response.status(), response.text().await.unwrap() ); 
    }
    let mut response_json = response.json::<serde_json::Value>().await.unwrap();
    repos_data.append(&mut match response_json["values"].as_array() {
        Some(values) => values  
                .iter()
                .map(|repo_json| {
                let val = Repository{
                    name: repo_json["name"].to_string(),
                    uuid: repo_json["uuid"].to_string(),
                    owner: repo_json["owner"]["username"].to_string(),
                    is_private: repo_json["is_private"].as_bool().unwrap_or(false),
                    clone_ssh_url: repo_json["links"]["clone"].as_array()
                        .expect("Unable to convert clone to array").iter().filter(|clone_val| {
                        clone_val["name".to_string()].as_str() == Some("ssh")
                    }).collect::<Vec<&Value>>()[0]["href"].to_string(),
                    project: repo_json["project"]["name"].to_string(),
                    workspace: repo_json["workspace"]["slug"].to_string(),
                };
                val
                })
                .collect(),
            None => Vec::new(),  
    });
    while response_json.get("next").is_some() {
        response = client.get(response_json["next"].as_str().unwrap()).headers(headers.clone()).send().await.unwrap();
        if !response.status().is_success() {
            panic!( "Failed to retrieve current user's repositories for the workspace {}. Status code: {}, Response content: {}",
        workspace, response.status(), response.text().await.unwrap() );} 
        response_json = response.json::<serde_json::Value>().await.unwrap(); 
        repos_data.append(&mut match response_json["values"].as_array() {
            Some(values) => values  
                .iter()
                .map(|repo_json| {
                let val = Repository{
                    name: repo_json["name"].to_string(),
                    uuid: repo_json["uuid"].to_string(),
                    owner: repo_json["owner"]["username"].to_string(),
                    is_private: repo_json["is_private"].as_bool().unwrap_or(false),
                    clone_ssh_url: repo_json["links"]["clone"].as_array()
                        .expect("Unable to convert clone to array").iter().filter(|clone_val| {
                        clone_val["name".to_string()].as_str() == Some("ssh")
                    }).collect::<Vec<&Value>>()[0]["href"].to_string(),
                    project: repo_json["project"]["name"].to_string(),
                    workspace: repo_json["workspace"]["slug"].to_string(),
                };
                val
                })
                .collect(),
            None => Vec::new(),  
        }); 
    }
    if repos_data.is_empty() { 
        return None;
    }

    for repo in &repos_data {
        save_repo_to_db(&repo);
    }
    Some(repos_data)
}

async fn get_webhooks_in_repo(workspace_slug: &str, repo_slug: &str, access_token: &str) -> Option<Vec<Webhook>> {
    let url = format!("{}/repositories/{}/{}/hooks", BITBUCKET_API_BASE_URL, workspace_slug, repo_slug);
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Authorization", format!("Bearer {}", access_token).parse().expect("Invalid access token"));
    headers.insert("Accept", "application/json".parse().expect("Invalid Accept header"));

    let response = reqwest::Client::new()
        .get(&url)
        .headers(headers) 
        .send()
        .await.expect("Failed to get webhooks");

    if !response.status().is_success() {
        return None
    }
    let response_json: Value = response.json().await.expect("Failed to parse JSON");
    match response_json["values"].as_array() {
        Some(values) => {
            return Some(values
          .iter()
          .map(|webhook_json| {
            let webhook = Webhook {
                uuid: webhook_json["uuid"].to_string(),
                active: webhook_json["active"].as_bool()
                .unwrap_or(false),
                url: webhook_json["url"].to_string(),
                ping_url: webhook_json["links"]["self"]["href"].to_string(),
                created_at: webhook_json["created_at"].to_string(),
                events: webhook_json["events"].as_array().expect("Unable to deserialize events").into_iter()
                    .map(|events| events.as_str().expect("Unable to convert event").to_string()).collect(),
            };
            webhook
          }).collect());
        },
        None => { return None},
      };
}

fn save_webhook_to_db(webhook: &Webhook) {
    let db = sled::open("/tmp/db").expect("Failed to open sled database");
    // Generate unique ID
    let uuid = Uuid::new_v4();
    let id = uuid.as_bytes();
  
    // Serialize webhook struct to JSON
    let json = serde_json::to_vec(webhook).expect("Failed to serialize webhook");
  
    // Insert JSON into sled DB
    db.insert(IVec::from(id), json).expect("Failed to insert webhook into sled DB");
}

async fn get_prs(workspace_slug: &str, repo_name: &str, access_token: &str, state: &str) -> Option<Vec<PullRequest>> {

    let url = format!("https://api.bitbucket.org/2.0/repositories/{workspace_slug}/{repo_name}/pullrequests");

    let client = reqwest::Client::new();

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        format!("Bearer {}", access_token).parse().unwrap()
    );

    let mut params = std::collections::HashMap::new();
    params.insert("state", state);

    let response = client.get(&url)
        .headers(headers)
        .query(&params)
        .send()
        .await
        .unwrap();

    let pull_requests: Vec<PullRequest> = match response.json::<serde_json::Value>().await {
        Ok(json) => {
        json["values"].as_array()
            .map(|prs| {
            prs.iter()
                .map(|pr| serde_json::from_value(pr.clone()).unwrap())
                .collect()
            })
            .unwrap_or_default()
        },
        Err(_) => Vec::new()
    };

    if pull_requests.is_empty() {
        None 
    } else {
        //Save to db
        for pr in &pull_requests {
            save_pr_to_db(pr);
        }
        Some(pull_requests)
    }

}

fn save_pr_to_db(pr: &PullRequest) {
    let db = sled::open("/tmp/db").expect("Failed to open sled database");

    // Generate unique ID
    let uuid = Uuid::new_v4();
    let id = uuid.as_bytes();
  
    // Serialize pull request struct to JSON 
    let json = serde_json::to_vec(pr).expect("Unable to serialize PR");
  
    // Insert JSON into sled DB
    db.insert(IVec::from(id), json).expect("Unable to insert pr info in db");
  
  }

pub async fn handle_install_bitbucket(installation_code: &str) {
    // get access token from installation code by calling relevant repo provider's api
    // out of github, bitbucket, gitlab

    let authinfo = get_access_token_from_bitbucket(installation_code).await.expect("Unable to get access token");
    // let auth_info = { "access_token": access_token, "expires_at": expires_at_formatted, "refresh_token": auth_info["refresh_token"] }; db.insert("auth_info", serde_json::to_string(&auth_info).unwrap());
    let access_token = authinfo.access_token.clone();
    let user_workspaces = get_bitbucket_workspaces(&access_token).await; 
    if user_workspaces.is_some() {
        let mut workspace_slugs = Vec::new(); 
        let webhook_callback_url = "https://gcscruncsql-k7jns52mtq-el.a.run.app/handle_bitbucket_pr_webhook";
    
        for workspace in user_workspaces.expect("user_workspaces is None") {
            let workspace_name = workspace.name;
            let workspace_slug = workspace.slug.to_string();
            workspace_slugs.push(workspace.slug);
        
            let repos = get_workspace_repos(&workspace.uuid, 
                &access_token).await;
            if repos.is_some() {
                for repo in repos.expect("repos is None") {
                    let repo_name = repo.name;
                    let webhooks_data = get_webhooks_in_repo(
                        &workspace_slug, &repo_name, &access_token).await;
                    if webhooks_data.is_some() {
                        match webhooks_data {
                            Some(hooks) => { let matching_webhook = hooks.into_iter()
                               .find(|w| w.ping_url == webhook_callback_url);
                                if matching_webhook.is_none() {
                                    let webhook = matching_webhook.expect("no matching webhook");
                                    save_webhook_to_db(&webhook);
                                }
                            },
                            None => {
                                let repo_name_async = repo_name.clone();
                                let workspace_slug_async = workspace_slug.clone();
                                let access_token_async = access_token.clone();
                                task::spawn(async move {
                                    add_pull_request_webhook_to_auth_repo_bitbucket(
                                        &workspace_slug_async, 
                                        &repo_name_async, 
                                        &access_token_async).await;
                                });
                            },
                        };
                        let repo_name_async = repo_name.clone();
                        let workspace_slug_async = workspace_slug.clone();
                        let access_token_async = access_token.clone();
                        task::spawn(async move {
                            get_prs(&workspace_slug_async,
                                &repo_name_async,
                                &access_token_async,
                                "OPEN").await;
                        });
                        
                    }
                }
            } else {
                println!("No repos found for workspace: {}", workspace_name);
            }
        } 
    } else { 
        println!("Unable to get the workspaces for the current user");
    }
}