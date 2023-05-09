use serde::{Serialize, Deserialize};
use std::error::Error;
use std::process::Command;
use std::str;
use std::collections::HashMap;
use sha256::digest;
use crate::observer::RuntimeInfo;

#[derive(Debug, Serialize, Default, Deserialize)]
struct Reviews {
    reviews: Vec<ReviewItem>,
}

#[derive(Debug, Serialize, Default, Deserialize)]
struct ReviewItem {
	base_head_commit: String,
	pr_head_commit: String,
	id: String,
}

#[derive(Debug, Serialize, Default, Deserialize)]
struct StatItem {
	filepath: String,
	additions: i32,
	deletions: i32,
}
#[derive(Debug, Serialize, Default, Deserialize)]
struct BlameItem {
	hunkhash: String,
	author: String,
	timestamp: String,
	linenum: String,
	filepath: String,
}

#[derive(Debug, Serialize, Default, Deserialize)]
struct HunkPRMap {
	repo_provider: String,
	repo_owner: String,
	repo_name: String,
	prvec: Vec<BlameItem>,
}

#[derive(Debug, Serialize, Default, Deserialize)]
struct HunkMap {
	repo_provider: String,
	repo_owner: String,
	repo_name: String,
	prhunkvec: Vec<PrHunkItem>,
}

#[derive(Debug, Serialize, Default, Deserialize)]
struct PrHunkItem {
	pr_number: String,
	blamevec: Vec<BlameItem>,
}
fn generate_diff(prev_commit: &str, curr_commit: &str, smallfiles: &Vec<StatItem>, einfo: &mut RuntimeInfo) -> HashMap<String, String> {
	let mut diffmap = HashMap::<String, String>::new();
	for item in smallfiles {
		let filepath = item.filepath.as_str();
		let params = vec![
		"diff".to_string(),
		format!("{prev_commit}:{filepath}"),
		format!("{curr_commit}:{filepath}"),
		"-U0".to_string()];
		match Command::new("git").args(&params).output() {
			Ok(result) => {
				let diff = result.stdout;
				match str::from_utf8(&diff) {
					Ok(diffstr) => {
						diffmap.insert(filepath.to_string(), diffstr.to_string());
					},
					Err(e) => {einfo.record_err(e.to_string().as_str());},
				};
			}
			Err(commanderr) => {
				eprintln!("git diff command failed to start : {commanderr}");
				einfo.record_err(commanderr.to_string().as_str());
			}
		};
	}
	return diffmap;
}

fn generate_blame(commit: &str, linemap: &HashMap<String, Vec<String>>, einfo: &mut RuntimeInfo) ->  Vec<BlameItem>{
	let mut blamevec = Vec::<BlameItem>::new();
	for (path, linevec) in linemap {
		for line in linevec {
			let paramvec: Vec<&str> = vec!(
				"blame",
				commit,
				"-L",
				line.as_str(),
				"-e",
				"--date=unix",
				path.as_str());
			let linenumvec: Vec<&str> = line.split(",").collect();
			let linenum = linenumvec[0];
			match Command::new("git").args(paramvec).output() {
				Ok(resultblame) => {
					let blame = resultblame.stdout;
					match str::from_utf8(&blame) {
						Ok(blamestr) => {
							let blamelines: Vec<&str> = blamestr.lines().collect();
							let mut prev_author = "".to_string();
							let mut lnum = -1;
							for ln in blamelines {
								lnum = lnum + 1;
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
								if authorstr == prev_author {
									continue;
								}
								else {
									prev_author = authorstr.clone();
									if timestamp == "" {
										idx = idx + 1;
										while idx < wordvec.len() && wordvec[idx] == "" {
											idx = idx + 1;
										}
										if idx < wordvec.len() {
											timestamp = wordvec[idx];
										}
									}
									let hunkstr = wordvec[idx+3..].to_vec().join(" ");
									let hunkhash = digest(hunkstr);
									blamevec.push(
										BlameItem { 
											hunkhash: hunkhash,
											author: authorstr,
											timestamp: timestamp.to_string(),
											linenum: (linenum.parse::<i32>().expect("Unable to parse linenum") + lnum).to_string(),
											filepath: digest(path.as_str()),
										}
									);
								}
							}
						},
						Err(e) => {einfo.record_err(e.to_string().as_str());},
					};
				}
				Err(e) => {
					eprintln!("git blame command failed to start : {e}");
					einfo.record_err(e.to_string().as_str());
				}
			}
		}
	}
	return blamevec;
}

fn process_reposlug(repo_slug: &str) -> (String, String) {
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

fn get_tasks(provider: &str, repo_slug: &str, einfo: &mut RuntimeInfo) -> Option<Reviews>{
	let api_url = "https://gcscruncsql-k7jns52mtq-el.a.run.app/relevance/hunk";
	let client = reqwest::blocking::Client::new();
	let mut map = HashMap::new();
	let (repo_name, repo_owner) = process_reposlug(repo_slug);
	map.insert("repo_provider", provider);
	map.insert("repo_owner", repo_owner.as_str());
	map.insert("repo_name", repo_name.as_str());
	let mut response:Option<Reviews> = None;
	match client.post(api_url).json(&map).send(){
		Ok(resobj) => { match resobj.json::<Reviews>() {
			Ok(revobj) => {
				response = Some(revobj);
			},
			Err(parsererr) => {
				einfo.record_err(parsererr.to_string().as_str());
				eprintln!("Unable to parse tasks : {parsererr}");
			}
		}},
		Err(reqerr) => {
			einfo.record_err(reqerr.to_string().as_str());
			eprintln!("Unable to get tasks : {reqerr}");
		}
	};
	// .expect("Get request failed")
    //     .json::<Reviews>().expect("Json parsing of response failed");
	return response;
}

fn store_hunkmap(hunkmap: HunkMap, einfo: &mut RuntimeInfo) {
	let api_url = "https://gcscruncsql-k7jns52mtq-el.a.run.app/relevance/hunk/store";
	let client = reqwest::blocking::Client::new();
	match client.post(api_url).json(&hunkmap).send(){
		Ok(response) => {
			match response.text() {
				Ok(restext) => {
					einfo.record_err(&restext);
					println!("Hunk relevance task complete");
				},
				Err(reserr) => {einfo.record_err(reserr.to_string().as_str());}
			}
		}
		Err(reqerr) => {einfo.record_err(reqerr.to_string().as_str());}

	}
}

fn get_excluded_files(prev_commit: &str, next_commit: &str, einfo: &mut RuntimeInfo) -> Option<(Vec<StatItem>, Vec<StatItem>)> {
	// Use the command
	match Command::new("git")
		.args(&["diff", prev_commit, next_commit, "--numstat"])
		.output() {
			Ok(resultstat) => {
				let stat = resultstat.stdout;
				// parse the output
				match str::from_utf8(&stat) {
					Ok(statstr) => {
						let statlines = statstr.split("\n");
						let mut statvec = Vec::<StatItem>::new();
						for line in statlines {
							let statitems: Vec<&str> = line.split("\t").collect();
							if statitems.len() >= 3 {
								let statitem = StatItem {
									filepath: statitems[2].to_string(),
									additions: statitems[0].to_string().parse().expect("Failed to parse git diff stat additions"),
									deletions: statitems[1].to_string().parse().expect("Failed to parse git diff stat deletions"),
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
					Err(e) => {einfo.record_err(e.to_string().as_str());},
				};
			},
			Err(commanderr) => {
				eprintln!("git diff stat command failed to start : {commanderr}");
				einfo.record_err(commanderr.to_string().as_str());
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

pub(crate) fn unfinished_tasks(provider: &str, repo_slug: &str, einfo: &mut RuntimeInfo) {
	let reviews = get_tasks(provider, repo_slug, einfo);
	if reviews.is_some() {
		let mut prvec = Vec::<PrHunkItem>::new();
		for review in reviews.expect("Validated reviews").reviews {
			let fileopt = get_excluded_files(&review.base_head_commit, &review.pr_head_commit, einfo);
			if fileopt.is_some() {
				let (bigfiles, smallfiles) = fileopt.expect("Validated fileopt");
				let diffmap = generate_diff(&review.base_head_commit, &review.pr_head_commit, &smallfiles, einfo);
				let diffres = process_diff(&diffmap);
				match diffres {
					Ok(linemap) => {
						let blamevec = generate_blame(&review.base_head_commit, &linemap, einfo);
						let hmapitem = PrHunkItem {
							pr_number: review.id,
							blamevec: blamevec,
						};
						prvec.push(hmapitem);
					}
					Err(e) => {
						eprint!("Unable to process diff : {e}");
						einfo.record_err(e.to_string().as_str());
					}
				}
			}
		}
		let (repo_name, repo_owner) = process_reposlug(repo_slug);
		let hunkmap = HunkMap { repo_provider: provider.to_string(),
			repo_owner: repo_owner, repo_name: repo_name, prhunkvec: prvec };
		store_hunkmap(hunkmap, einfo);
	}
}