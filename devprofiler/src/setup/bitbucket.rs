use crate::db::get_db;
use std::collections::HashMap;
use rand::distributions::Alphanumeric;
use reqwest::Client;
use reqwest::Response;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tokio::fs;
use uuid::Uuid;
use serde_json::json;
use sled::IVec;
use tokio::task;
use rand::{thread_rng, Rng};
use std::env;


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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Repository {
    name: String,
    uuid: String,
    owner: String,
    is_private: bool,
    pub clone_ssh_url: String,
    project: String,
    workspace: String,
    pub local_dir: Option<String>,
    provider: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PullRequest {

}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthInfo {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: Option<String>,
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

fn bitbucket_base_url() -> String {
    env::var("BITBUCKET_BASE_URL").expect("BITBUCKET_BASE_URL must be set")
}

async fn add_pull_request_webhook_to_auth_repo_bitbucket(
    workspace_slug: &str, 
    repo_slug: &str, 
    access_token: &str
) {

    let url = format!(
        "{}/repositories/{}/{}/hooks", 
        bitbucket_base_url(), workspace_slug, repo_slug
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
    let callback_url = format!("{}/api/bitbucket/callbacks/webhook", 
        env::var("BASE_SERVER_URL").expect("WEBHOOK_CALLBACK_URL must be set"));
    let payload = json!({
        "description": "Webhook for PRs when raised and when something is pushed to the open PRs",
        "url": callback_url,
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
            if res.status().is_success() {
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
                        println!("Error in deserializing res: {:?}", err);
                    }
                };
            }
            else {
                println!("Failed to add webhook. Status code: {}, Text: {:?}",
                    res.status(), res.text().await);
            }
        },
        Err(err) => {
            println!("Error in api call: {:?}", err);
        },
    };
}

async fn get_access_token_from_bitbucket(code: &str) -> Option<AuthInfo> {
    let client = Client::new();
    let bitbucket_client_id = env::var("BITBUCKET_CLIENT_ID").unwrap();
    let bitbucket_client_secret = env::var("BITBUCKET_CLIENT_SECRET").unwrap();
    let mut params = std::collections::HashMap::new();
    let redirect_uri = format!("{}/api/bitbucket/callbacks/install",
        env::var("BASE_SERVER_URL").expect("WEBHOOK_CALLBACK_URL must be set"));
    println!("redirect_uri = {}", &redirect_uri);
    params.insert("client_id", bitbucket_client_id);
    params.insert("client_secret", bitbucket_client_secret);
    params.insert("code", code.to_owned());
    params.insert("grant_type", "authorization_code".to_owned());
    params.insert("redirect_uri", redirect_uri);

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
    let db = get_db();
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
    let db = get_db();
    let json = serde_json::to_string(&workspace).expect("Failed to serialize workspace");
    // Convert JSON string to bytes
    let bytes = json.as_bytes(); 
    // Create IVec from bytes
    let ivec = IVec::from(bytes);
    db.insert(format!("workspaces:{}", uuid), ivec).expect("Unable to save workspace in db");  
}

async fn get_bitbucket_workspaces(access_token: &str) -> Vec<Workspace> {
    let user_url = format!("{}/workspaces", bitbucket_base_url());
    let response = get_api(&user_url, access_token, None).await;
    let mut workspace_vec = Vec::new();
    for workspace_json in response {
        let val = serde_json::from_value::<Workspace>(workspace_json.clone()).expect("Unable to deserialize workspace");
        save_workspace_to_db(&val);
        workspace_vec.push(val);
    }
    return workspace_vec;
}

fn save_repo_to_db(repo: &Repository) {
    let db = get_db();
    let repo_key = format!("{}/{}/{}", repo.provider, repo.workspace, repo.name);
    println!("repo_key = {}", &repo_key);
  
    // Serialize repo struct to JSON 
    let json = serde_json::to_vec(repo).expect("Failed to serialize repo");
  
    // Insert JSON into sled DB
    db.insert(IVec::from(repo_key.as_bytes()), json).expect("Failed to upsert repo into sled DB");
}

async fn get_workspace_repos(workspace: &str, access_token: &str) -> Option<Vec<Repository>> {
    let repos_url = format!("{}/repositories/{}", bitbucket_base_url(), workspace);
    let response_json = get_api(&repos_url, access_token, None).await;
    let mut repos_data = Vec::new();
    for repo_json in response_json {
        let val = Repository{
            name: repo_json["name"].to_string().trim_matches('"').to_string(),
            uuid: repo_json["uuid"].to_string().trim_matches('"').to_string(),
            owner: repo_json["owner"]["username"].to_string().trim_matches('"').to_string(),
            is_private: repo_json["is_private"].as_bool().unwrap_or(false),
            clone_ssh_url: repo_json["links"]["clone"].as_array()
                .expect("Unable to convert clone to array").iter().filter(|clone_val| {
                clone_val["name".to_string()].as_str() == Some("ssh")
            }).collect::<Vec<&Value>>()[0]["href"].to_string().replace('\"',""),
            project: repo_json["project"]["name"].to_string().trim_matches('"').to_string(),
            workspace: repo_json["workspace"]["slug"].to_string().trim_matches('"').to_string(),
            local_dir: None,
            provider: "bitbucket".to_string(),
        };
        save_repo_to_db(&val);
        repos_data.push(val);
    }
    Some(repos_data)
}

async fn call_get_api(url: &str, access_token: &str, params: &Option<HashMap<&str, &str>> ) -> Option<Response>{
    println!("GET api url = {}", url);
    let client = reqwest::Client::new();
    let mut headers = reqwest::header::HeaderMap::new(); 
    headers.insert( reqwest::header::AUTHORIZATION, 
    format!("Bearer {}", access_token).parse().expect("Invalid auth header"), );
    headers.insert("Accept",
     "application/json".parse().expect("Invalid Accept header"));
    match params {
        Some(params) => {
            match client.get(url).headers(headers).query(params).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return Some(response);
                    }
                    else { eprintln!("Failed to call API {}, status: {}", url, response.status()); }
                },
                Err(e) => { eprintln!("Error sending GET request to {}, error: {}", url, e); },
            };
        },
        None => {
            match client.get(url).headers(headers).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return Some(response);
                    }
                    else { eprintln!("Failed to call API {}, status: {}", url, response.status()); }
                },
                Err(e) => { eprintln!("Error sending GET request to {}, error: {}", url, e); },
            };
        }
    };
    return None;
}

async fn deserialize_response(response_opt: Option<Response>) -> (Vec<Value>, Option<String>) {
    let values_vec = Vec::new();
    match response_opt {
        Some(response) => {
            match response.json::<serde_json::Value>().await {
                Ok(response_json) => {
                    let mut values_vec = Vec::new();
                    if let Some(values) = response_json["values"].as_array() {
                        for value in values {
                            values_vec.push(value.to_owned()); 
                        }
                        return (values_vec, Some(response_json["next"].to_string()));
                    }
                }
                Err(e) => { eprintln!("Unable to deserialize response: {}", e); }
            };
        },
        None => { eprintln!("Response is None");}
    };
    return (values_vec, None);
    
}

async fn get_all_pages(next_url: Option<String>, access_token: &str, params: &Option<HashMap<&str, &str>>) -> Vec<Value>{
    let mut values_vec = Vec::new();
    let mut next_url = next_url;
    while next_url.is_some() {
        let url = next_url.as_ref().expect("next_url is none").trim_matches('"');
        if url != "null" {
            let response_opt = call_get_api(url, access_token, params).await;
            let (mut response_values, url_opt) = deserialize_response(response_opt).await;
            next_url = url_opt.clone();
            values_vec.append(&mut response_values);    
        } else {
            break;
        }
    }
    return values_vec;
}

async fn get_api(url: &str, access_token: &str, params: Option<HashMap<&str, &str>> ) -> Vec<Value> {
    let response_opt = call_get_api(url, access_token, &params).await;
    let (mut response_values, next_url) = deserialize_response(response_opt).await;
    if next_url.is_some() {
        let mut page_values = get_all_pages(next_url, access_token, &params).await;
        response_values.append(&mut page_values);
    }
    return response_values;
}

async fn get_webhooks_in_repo(workspace_slug: &str, repo_slug: &str, access_token: &str) -> Vec<Webhook> {
    let url = format!("{}/repositories/{}/{}/hooks", bitbucket_base_url(), workspace_slug, repo_slug);
    println!("Getting webhooks from {}", url);
    let response_json = get_api(&url, access_token, None).await;
    let mut webhooks = Vec::new();
    for webhook_json in response_json {
        let active = matches!(webhook_json["active"].to_string().trim_matches('"'), "true" | "false");
        let webhook = Webhook {
            uuid: webhook_json["uuid"].to_string(),
            active: active,
            url: webhook_json["url"].to_string().replace('"', ""),
            ping_url: webhook_json["links"]["self"]["href"].to_string().replace('"', ""),
            created_at: webhook_json["created_at"].to_string().replace('"', ""),
            events: webhook_json["events"].as_array().expect("Unable to deserialize events").into_iter()
                .map(|events| events.as_str().expect("Unable to convert event").to_string()).collect(),
        };
        webhooks.push(webhook);
    }
    return webhooks;
}

fn save_webhook_to_db(webhook: &Webhook) {
    let db = get_db();
    // Generate unique ID
    let uuid = Uuid::new_v4();
    let id = uuid.as_bytes();
    // Serialize webhook struct to JSON
    let json = serde_json::to_vec(webhook).expect("Failed to serialize webhook");
    // Insert JSON into sled DB
    db.insert(IVec::from(id), json).expect("Failed to insert webhook into sled DB");
}

async fn get_prs(workspace_slug: &str, repo_name: &str, access_token: &str, state: &str) -> Vec<PullRequest> {

    let url = format!("https://api.bitbucket.org/2.0/repositories/{workspace_slug}/{repo_name}/pullrequests");
    let mut params = std::collections::HashMap::new();
    params.insert("state", state);

    let response = get_api(&url, access_token, Some(params)).await;
    let mut pull_reqs = Vec::new();
    for pr_json in response {
        let pr = serde_json::from_value::<PullRequest>(pr_json);
        match pr {
            Ok(pullrequest) => {
                pull_reqs.push(pullrequest);
            },
            Err(e) => {eprintln!("Error parsing pull request json: {}", e);}
        };
    }
    for pr in &pull_reqs {
        save_pr_to_db(pr);
    }
    return pull_reqs;
}

fn save_pr_to_db(pr: &PullRequest) {
    // let db = get_db();
    // Generate unique ID
    // let uuid = Uuid::new_v4();
    // let id = uuid.as_bytes();
    // Serialize pull request struct to JSON 
    // let json = serde_json::to_vec(pr).expect("Unable to serialize PR");
    // Insert JSON into sled DB
    // db.insert(IVec::from(id), json).expect("Unable to insert pr info in db");
}

async fn clone_git_repo(repo: &mut Repository, access_token: &str) {
    let git_url = &repo.clone_ssh_url;
    let clone_url = git_url.to_string()
        .replace("git@", format!("https://x-token-auth:{{{access_token}}}@").as_str())
        .replace("bitbucket.org:", "bitbucket.org/");
    let random_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();
    let mut directory = format!("/tmp/{}/{}/{}", &repo.provider, &repo.workspace, random_string);
    // Check if directory exists
    if fs::metadata(&directory).await.is_ok() {
        fs::remove_dir_all(&directory).await.expect("Unable to remove pre-existing directory components");
    }
    fs::create_dir_all(&directory).await.expect("Unable to create directory");
    println!("directory exists? {}", fs::metadata(&directory).await.is_ok());
    let mut cmd = std::process::Command::new("git");
    cmd.arg("clone").arg(clone_url).current_dir(&directory);
    let output = cmd.output().expect("Failed to clone git repo");
    println!("Git clone output: {:?}", output);
    directory = format!("{}/{}", &directory, &repo.name);
    repo.local_dir = Some(directory);
    save_repo_to_db(repo);
}


pub async fn handle_install_bitbucket(installation_code: &str) {
    // get access token from installation code by calling relevant repo provider's api
    // out of github, bitbucket, gitlab

    let authinfo = get_access_token_from_bitbucket(installation_code).await.expect("Unable to get access token");
    println!("AuthInfo: {:?}", authinfo);
    // let auth_info = { "access_token": access_token, "expires_at": expires_at_formatted, "refresh_token": auth_info["refresh_token"] }; db.insert("auth_info", serde_json::to_string(&auth_info).unwrap());
    let access_token = authinfo.access_token.clone();
    let user_workspaces = get_bitbucket_workspaces(&access_token).await;
    let webhook_callback_url = format!("{}/api/bitbucket/callbacks/webhook", 
        env::var("BASE_SERVER_URL").expect("WEBHOOK_CALLBACK_URL must be set"));
    for workspace in user_workspaces {
        let workspace_slug = workspace.slug.to_string();
        println!("=========<{:?}>=======", workspace_slug);
    
        let repos = get_workspace_repos(&workspace.uuid, 
            &access_token).await;
        for repo in repos.expect("repos is None") {
            let token_copy = access_token.clone();
            let mut repo_copy = repo.clone();
            clone_git_repo(&mut repo_copy, &token_copy).await;
            let repo_name = repo.name;
            println!("Repo url git = {:?}", &repo.clone_ssh_url);
            println!("Repo name = {:?}", repo_name);
            let webhooks_data = get_webhooks_in_repo(
                &workspace_slug, &repo_name, &access_token).await;
            match webhooks_data.is_empty() {
                true => { 
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
                false => {
                    let matching_webhook = webhooks_data.into_iter()
                        .find(|w| w.url == webhook_callback_url);
                    if matching_webhook.is_some() {
                        let webhook = matching_webhook.expect("no matching webhook");
                        println!("Webhook already exists: {:?}", &webhook);
                        save_webhook_to_db(&webhook);
                    } else {
                        println!("Adding new webhook...");
                        let repo_name_async = repo_name.clone();
                        let workspace_slug_async = workspace_slug.clone();
                        let access_token_async = access_token.clone();
                        task::spawn(async move {
                            add_pull_request_webhook_to_auth_repo_bitbucket(
                                &workspace_slug_async, 
                                &repo_name_async, 
                                &access_token_async).await;
                        });
                    }
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
    
}