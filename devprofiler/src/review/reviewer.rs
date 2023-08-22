use serde::{Serialize, Deserialize};
use sled::IVec;
use std::error::Error;
use std::process::Command;
use std::{str, env, clone};
use std::collections::HashMap;
use sha256::digest;
use std::sync::Mutex;
use serde_json::Value;
use crate::db;
use crate::setup::bitbucket::{Repository, refresh_git_auth};
use crate::db::get_db;


#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct Review {
    base_head_commit: String,
	pr_head_commit: String,
	id: String,
	repo_name: String,
	repo_owner: String,
	provider: String,
	db_key: String,
	clone_dir: String,
	clone_url: String,
	author: String,
}

#[derive(Debug, Serialize, Default, Deserialize)]
struct StatItem {
	filepath: String,
	additions: i32,
	deletions: i32,
}
#[derive(Debug, Serialize, Default, Deserialize)]
struct BlameItem {
	author: String,
	timestamp: String,
	line_start: String,
	line_end: String,
	filepath: String,
}

#[derive(Debug, Serialize, Default, Deserialize)]
struct LineItem {
	author: String,
	timestamp: String,
}

#[derive(Debug, Serialize, Default, Deserialize)]
struct HunkPRMap {
	repo_provider: String,
	repo_owner: String,
	repo_name: String,
	prvec: Vec<BlameItem>,
}

#[derive(Debug, Serialize, Default, Deserialize)]
pub struct HunkMap {
	repo_provider: String,
	repo_owner: String,
	repo_name: String,
	prhunkvec: Vec<PrHunkItem>,
	db_key: String,
}

#[derive(Debug, Serialize, Default, Deserialize)]
struct PrHunkItem {
	pr_number: String,
	author: String,
	blamevec: Vec<BlameItem>,
  }


static GIT_PULL_MUTEX: Mutex<()> = Mutex::new(()); 


fn commit_exists(commit: &str) -> bool {
	let output = Command::new("git")
		.arg("rev-list")
		.arg(commit)
		.output()
		.expect("failed to execute git rev-list");

	output.status.success()
}

async fn git_pull(review: &Review) {
	let directory = review.clone_dir.clone();
	println!("directory = {}", &directory);
	refresh_git_auth(&review.clone_url, &review.clone_dir).await;
	let output = Command::new("git")
		.arg("pull")
		// .arg("--all") 
		// .arg(&review.clone_url)
		.current_dir(directory)
		.output()
		.expect("failed to execute git pull");
	match str::from_utf8(&output.stderr) {
		Ok(v) => println!("git pull stderr = {:?}", v),
		Err(e) => {/* error handling */ println!("{}", e)}, 
	};
	match str::from_utf8(&output.stdout) {
		Ok(v) => println!("git pull stdout = {:?}", v),
		Err(e) => {/* error handling */ println!("{}", e)}, 
	};
	println!("git pull output = {:?}, {:?}", &output.stdout, &output.stderr);
}

fn generate_diff(review: &Review, smallfiles: &Vec<StatItem>) -> HashMap<String, String> {
	let mut diffmap = HashMap::<String, String>::new();
	let prev_commit = &review.base_head_commit;
	let curr_commit = &review.pr_head_commit;
	let clone_dir = &review.clone_dir;
	for item in smallfiles {
		let filepath = item.filepath.as_str();
		let params = vec![
		"diff".to_string(),
		format!("{prev_commit}:{filepath}"),
		format!("{curr_commit}:{filepath}"),
		"-U0".to_string(),
		];
		match Command::new("git").args(&params)
		.current_dir(&clone_dir)
		.output() {
			Ok(result) => {
				let diff = result.stdout;
				match str::from_utf8(&diff) {
					Ok(diffstr) => {
						println!("diffstr = {}", &diffstr);
						diffmap.insert(filepath.to_string(), diffstr.to_string());
					},
					Err(e) => {println!("Unable to deserialize diff: {e}");},
				};
			}
			Err(commanderr) => {
				eprintln!("git diff command failed to start : {commanderr}");
			}
		};
	}
	return diffmap;
}

fn process_blamelines(blamelines: &Vec<&str>, linenum: usize) -> HashMap<usize, LineItem> {
	let mut linemap = HashMap::<usize, LineItem>::new();
	for lnum  in 0..blamelines.len() {
		let ln = blamelines[lnum];
		let wordvec: Vec<&str> = ln.split(" ").collect();
		let mut author = wordvec[1];
		let mut timestamp = wordvec[2];
		let mut idx = 1;
		if author == "" {
			while idx < wordvec.len() && wordvec[idx] == "" {
				idx = idx + 1;
			}
			if idx < wordvec.len() {
				author = wordvec[idx];
			}
		}
		let authorstr = author.replace("(", "")
			.replace("<", "")
			.replace(">", "");
		if timestamp == "" {
			idx = idx + 1;
			while idx < wordvec.len() && wordvec[idx] == "" {
				idx = idx + 1;
			}
			if idx < wordvec.len() {
				timestamp = wordvec[idx];
			}
		}
		linemap.insert(
			linenum + lnum,
			LineItem { author: authorstr.to_string(), timestamp: timestamp.to_string() }
		);
	}
	return linemap;
}

fn generate_blame(review: &Review, linemap: &HashMap<String, Vec<String>>) ->  Vec<BlameItem>{
	let mut blamevec = Vec::<BlameItem>::new();
	let commit = &review.pr_head_commit;
	let clone_dir = &review.clone_dir;
	for (path, linevec) in linemap {
		for line in linevec {
			let paramvec: Vec<&str> = vec!(
				"blame",
				commit,
				"-L",
				line.as_str(),
				"-e",
				"--date=unix",
				path.as_str(),
			);
			let linenumvec: Vec<&str> = line.split(",").collect();
			let linenum = linenumvec[0];
			match Command::new("git").args(paramvec)
			.current_dir(clone_dir)
			.output() {
				Ok(resultblame) => {
					let blame = resultblame.stdout;
					match str::from_utf8(&blame) {
						Ok(blamestr) => {
							println!("blamestr = {}", blamestr);
							let blamelines: Vec<&str> = blamestr.lines().collect();
							if blamelines.len() == 0 {
								continue;
							}
							let linenumint = linenum.parse::<usize>().expect("Unable to parse linenum");
							let lineauthormap = process_blamelines(&blamelines, linenumint);
							let mut linebreak = linenumint;
							for lidx in linenumint..(linenumint + blamelines.len()-1) {
								if lineauthormap.contains_key(&lidx) && lineauthormap.contains_key(&(lidx+1)) {
									let lineitem = lineauthormap.get(&lidx).expect("lidx checked");
									if lineitem.author == 
									lineauthormap.get(&(lidx+1)).expect("lidx+1 checked").author {
										continue;
									}
									else {
										blamevec.push(BlameItem {
											author: lineitem.author.to_string(),
											timestamp: lineitem.timestamp.to_string(),
											line_start: linebreak.to_string(),
											line_end: lidx.to_string(),
											filepath: digest(path.as_str()) });
										linebreak = lidx + 1;
									}
								}
							}
							let lastidx = linenumint + blamelines.len()-1;
							if lineauthormap.contains_key(&lastidx) {
								let lineitem = lineauthormap.get(&lastidx).expect("lastidx checked");
								blamevec.push(BlameItem {
									author: lineitem.author.to_string(),
									timestamp: lineitem.timestamp.to_string(),
									line_start: linebreak.to_string(),
									line_end: lastidx.to_string(),
									filepath: digest(path.as_str()) });

							}
							
						},
						Err(e) => {println!("Unable to deserialize blame: {e}");},
					};
				}
				Err(e) => {
					eprintln!("git blame command failed to start : {e}");
				}
			}
		}
	}
	return blamevec;
}

pub(crate) fn process_reposlug(repo_slug: &str) -> (String, String) {
	let repo_owner;
	let repo_name;
	if repo_slug.contains("/") {
		let slug_parts: Vec<&str> = repo_slug.split("/").collect();
		repo_owner = slug_parts[0];
		repo_name = slug_parts[1];
	}
	else {
		repo_name = repo_slug;
		repo_owner = "";
	}
	return (repo_name.to_string(), repo_owner.to_string());
}

fn get_clone_url_clone_dir(repo_provider: &str, workspace_name: &str, repo_name: &str) -> (String, String) {
	let db = db::get_db();
	let key = format!("{}/{}/{}", repo_provider, workspace_name, repo_name);
	let repo_opt = db.get(IVec::from(key.as_bytes())).expect("Unable to get repo from db");
	let repo_ivec = repo_opt.expect("Empty value");
	let repo: Repository = serde_json::from_slice::<Repository>(&repo_ivec).unwrap();
	println!("repo = {:?}", &repo);
	let clone_dir = repo.local_dir.expect("No local dir for repo found in db").clone();
	let clone_url = repo.clone_ssh_url.clone();
	return (clone_url, clone_dir);
}

fn save_review_to_db(review: &Review) {
    let db = get_db();
    let review_key = review.db_key.clone();  
    // Serialize repo struct to JSON 
    let json = serde_json::to_vec(review).expect("Failed to serialize repo");
    // Insert JSON into sled DB
    db.insert(IVec::from(review_key.as_bytes()), json).expect("Failed to upsert repo into sled DB");
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
			let review = Review {
				repo_name: repo_name.clone(),
				base_head_commit: data["event_payload"]["pullrequest"]["destination"]["commit"]["hash"].to_string().replace("\"", ""),
				pr_head_commit: data["event_payload"]["pullrequest"]["source"]["commit"]["hash"].to_string().replace("\"", ""),
				id: pr_id.clone(),
				provider: repo_provider.clone(),
				repo_owner: workspace_name.clone(),
				db_key: format!("bitbucket/{}/{}/{}", &workspace_name, &repo_name, &pr_id),
				clone_dir: clone_dir,
				clone_url: clone_url,
				author: data["event_payload"]["pullrequest"]["author"]["account_id"].to_string().replace("\"", ""),
			};
			println!("review = {:?}", &review);
			save_review_to_db(&review);
			return Some(review);
		},
		Err(e) => {eprintln!("Incoming message does not contain valid reviews: {e}");},
	};
	return None;
}

fn store_hunkmap_to_db(hunkmap: &HunkMap, review: &Review) {
    let db = db::get_db();
	let key = format!("{}/{}/{}", review.db_key, review.base_head_commit, review.pr_head_commit);
	println!("key = {}", key);
	let json = serde_json::to_vec(hunkmap).expect("Failed to serialize repo");
  
    // Insert JSON into sled DB
    db.insert(IVec::from(key.as_bytes()), json).expect("Failed to upsert repo into sled DB");
}

fn get_excluded_files(review: &Review) -> Option<(Vec<StatItem>, Vec<StatItem>)> {
	// Use the command
	let prev_commit = &review.base_head_commit;
	let next_commit = &review.pr_head_commit;
	let clone_dir = &review.clone_dir;
	println!("prev_commit = {}, next commit = {}, clone_dir = {}", &prev_commit, &next_commit, &clone_dir);
	match Command::new("git")
		.args(&["diff", prev_commit, next_commit, "--numstat"])
		.current_dir(clone_dir)
		.output() {
			Ok(resultstat) => {
				let stat = resultstat.stdout;
				// parse the output
				match str::from_utf8(&stat) {
					Ok(statstr) => {
						println!("statstr = {}", statstr);
						let statlines = statstr.split("\n");
						let mut statvec = Vec::<StatItem>::new();
						for line in statlines {
							let statitems: Vec<&str> = line.split("\t").collect();
							if statitems.len() >= 3 {
								let statitem = StatItem {
									filepath: statitems[2].to_string(),
									additions: match statitems[0].to_string().parse() {
										Ok(adds) => {adds}
										Err(e) => {0}
									},
									deletions: match statitems[0].to_string().parse() {
										Ok(dels) => {dels}
										Err(e) => {0}
									},
								};
								statvec.push(statitem);
							}
						}
						// logic for exclusion
						let mut bigfiles = Vec::<StatItem>::new();
						let mut smallfiles = Vec::<StatItem>::new();
						let line_threshold = 500;
						for item in statvec {
							if (item.additions > line_threshold) || 
							(item.deletions > line_threshold) || 
							(item.additions + item.deletions > line_threshold) {
								bigfiles.push(item);
							}
							else {
								smallfiles.push(item);
							}
						}
						return Some((bigfiles, smallfiles));
					},
					Err(e) => {println!("error utf: {e}");},
				};
			},
			Err(commanderr) => {
				eprintln!("git diff stat command failed to start : {commanderr}");
			}
		}
	
	// compile the result and return
	return None;
}

fn process_diff(diffmap: &HashMap<String, String>) -> Result<HashMap<String, Vec<String>>,Box<dyn Error>> {
	let mut linemap: HashMap<String, Vec<String>> = HashMap::new();
	for (filepath, diff) in diffmap {
		let mut limiterpos = Vec::new();
		let delimitter = "@@";
		for (idx, _) in diff.match_indices(delimitter) {
			limiterpos.push(idx);
		}
		let mut idx: usize = 0;
		let len = limiterpos.len();
		while (idx + 1) < len {
			let line = diff.get(
				(limiterpos[idx]+delimitter.len())..limiterpos[idx+1]
			).expect("Unable to format diff line");
			let sublines: Vec<&str> = line.split(" ").collect();
			if line.contains("\n") || sublines.len() != 4 {
				idx += 1;
				continue;
			}
			let mut deletionstr = sublines[1].to_owned();
			// let additionstr = sublines[1];
			if deletionstr.contains("-") {
				deletionstr = deletionstr.replace("-", "");
				if deletionstr.contains(",") {
					let delsplit: Vec<&str> = deletionstr.split(",").collect();
					let delidx = delsplit[0].parse::<i32>().unwrap();
					let deldiff = delsplit[1].parse::<i32>().unwrap();
					deletionstr = format!("{delidx},{}", delidx+deldiff);
				}
				else {
					let delidx = deletionstr.parse::<i32>().unwrap();
					deletionstr.push_str(format!(",{}", delidx).as_str());
				}
			}
			else {
				idx += 1;
				continue;
			}
			if linemap.contains_key(filepath) {
				linemap.get_mut(filepath).unwrap().push(deletionstr);
			}
			else {
				linemap.insert(filepath.to_string(), vec!(deletionstr));
			}
			idx += 1;
		}
	}
	return Ok(linemap);
}

fn publish_hunkmap(hunkmap: &HunkMap) {
	let client = reqwest::Client::new();
	let hunkmap_json = serde_json::to_string(&hunkmap).expect("Unable to serialize hunkmap");
	tokio::spawn(async move {
		let url = format!("{}/api/hunks",
			env::var("BASE_SERVER_URL").expect("BASE_SERVER_URL must be set"));
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

fn get_hunk_from_db(review: &Review) -> Option<HunkMap> {
	let db = db::get_db();
	let key = format!("{}/{}/{}", review.db_key, 
		review.base_head_commit, review.pr_head_commit);
	let hunkmap_val = db.get(&key);
	match hunkmap_val {
		Ok(hunkmap_val) => {
			match hunkmap_val {
				Some(hunkmap_json) => {
					match serde_json::from_slice(&hunkmap_json) {
						Ok(hunkmap) => {
							return Some(hunkmap);},
						Err(e) => {eprintln!("Error deserializing hunkmap: {}", e);},
					};
				}, None => {eprintln!("No hunkmap stored in db for key: {}", &key)}
			};
		}, Err(e) => {eprintln!("Error getting hunkmap from db, key: {}, err: {e}", &key);}
	};
	return None;
}

pub(crate) async fn process_review(message_data: &Vec<u8>) -> Option<HunkMap> {
	let review_opt = get_tasks(message_data);
	match review_opt {
		Some(review) => {
			let hunk = get_hunk_from_db(&review);
			match hunk {
				Some(hunkval) => {
					publish_hunkmap(&hunkval);
					return Some(hunkval);},
				None => {}
			}
			let mut prvec = Vec::<PrHunkItem>::new();
			println!("Processing PR : {}", review.id);
			if !commit_exists(&review.base_head_commit) || !commit_exists(&review.pr_head_commit) {
				println!("Pulling repository {} for commit history", &review.repo_name);
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
							let hmapitem = PrHunkItem {
								pr_number: review.id.clone(),
								author: review.author.clone(),
								blamevec: blamevec,
							};
							prvec.push(hmapitem);
						}
						Err(e) => {
							eprint!("Unable to process diff : {e}");
						}
					}
					let hunkmap = HunkMap { repo_provider: review.provider.clone(),
						repo_owner: review.repo_owner.clone(), 
						repo_name: review.repo_name.clone(), 
						prhunkvec: prvec,
						db_key: format!("{}/hunkmap", &review.db_key),
					 };
					store_hunkmap_to_db(&hunkmap, &review);
					publish_hunkmap(&hunkmap);
					return Some(hunkmap)
				},
				None => {eprintln!("No files to review for PR {}", &review.id);}
			};
		},
		None => { eprintln!("No review tasks found!" ); }
	};
	None		
}
