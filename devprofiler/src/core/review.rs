use std::env;

use serde_json::Value;

use crate::{utils::{hunk::{HunkMap, PrHunkItem}, review::Review, gitops::{commit_exists, git_pull, get_excluded_files, generate_diff, process_diff, generate_blame}}, db::{hunk::{get_hunk_from_db, store_hunkmap_to_db}, repo::get_clone_url_clone_dir, review::save_review_to_db}, core::coverage::process_coverage};

pub async fn process_review(message_data: &Vec<u8>) {
	let review_opt = get_tasks(message_data);
	match review_opt {
		Some(review) => {
			let hunk = get_hunk_from_db(&review);
			match hunk {
				Some(hunkval) => {
					publish_hunkmap(&hunkval);
					return;},
				None => {}
			}
			let mut prvec = Vec::<PrHunkItem>::new();
			println!("Processing PR : {}", review.id());
			if !commit_exists(&review.base_head_commit()) || !commit_exists(&review.pr_head_commit()) {
				println!("Pulling repository {} for commit history", review.repo_name());
				git_pull(&review).await;
			}
			let fileopt = get_excluded_files(&review);
			println!("fileopt = {:?}", &fileopt);
			match fileopt {
				Some((_, smallfiles)) => {
					let diffmap = generate_diff(&review, &smallfiles);
					println!("diffmap = {:?}", &diffmap);
					let diffres = process_diff(&diffmap);
					match diffres {
						Ok(linemap) => {
							let blamevec = generate_blame(&review, &linemap);
							let hmapitem = PrHunkItem::new(
								review.id().to_string(),
								review.author().to_string(),
								blamevec,
                            );
							prvec.push(hmapitem);
						}
						Err(e) => {
							eprint!("Unable to process diff : {e}");
						}
					}
					let hunkmap = HunkMap::new(review.provider().to_string(),
						review.repo_owner().to_string(), 
						review.repo_name().to_string(), 
						prvec,
						format!("{}/hunkmap", review.db_key()),
                );
					store_hunkmap_to_db(&hunkmap, &review);
					publish_hunkmap(&hunkmap);
					process_coverage(&hunkmap).await;
				},
				None => {eprintln!("No files to review for PR {}", review.id());}
			};
		},
		None => { eprintln!("No review tasks found!" ); }
	};
}

fn get_tasks(message_data: &Vec<u8>) -> Option<Review>{
	match serde_json::from_slice::<Value>(&message_data) {
		Ok(data) => {
			let repo_provider = data["repository_provider"].to_string().trim_matches('"').to_string();
			let repo_name = data["event_payload"]["repository"]["name"].to_string().trim_matches('"').to_string();
			println!("repo NAME == {}", &repo_name);
			let workspace_name = data["event_payload"]["repository"]["workspace"]["slug"].to_string().trim_matches('"').to_string();
			let (clone_url, clone_dir) = get_clone_url_clone_dir(&repo_provider, &workspace_name, &repo_name);
			let pr_id = data["event_payload"]["pullrequest"]["id"].to_string().trim_matches('"').to_string();
			let review = Review::new(
                data["event_payload"]["pullrequest"]["destination"]["commit"]["hash"].to_string().replace("\"", ""),
				data["event_payload"]["pullrequest"]["source"]["commit"]["hash"].to_string().replace("\"", ""),
				pr_id.clone(),
                repo_name.clone(),
                workspace_name.clone(),
				repo_provider.clone(),
				format!("bitbucket/{}/{}/{}", &workspace_name, &repo_name, &pr_id),
				clone_dir,
				clone_url,
				data["event_payload"]["pullrequest"]["author"]["account_id"].to_string().replace("\"", ""),
            );
			println!("review = {:?}", &review);
			save_review_to_db(&review);
			return Some(review);
		},
		Err(e) => {eprintln!("Incoming message does not contain valid reviews: {e}");},
	};
	return None;
}

fn publish_hunkmap(hunkmap: &HunkMap) {
	let client = reqwest::Client::new();
	let hunkmap_json = serde_json::to_string(&hunkmap).expect("Unable to serialize hunkmap");
	tokio::spawn(async move {
		let url = format!("{}/api/hunks",
			env::var("SERVER_URL").expect("SERVER_URL must be set"));
		println!("url for hunkmap publishing  {}", &url);
		match client
		.post(url)
		.json(&hunkmap_json)
		.send()
		.await {
			Ok(_) => {
				println!("Hunkmap published successfully!");
			},
			Err(e) => {
				eprintln!("Failed to publish hunkmap: {}", e);
			}
		};
	});
}
