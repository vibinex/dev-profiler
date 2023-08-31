use std::collections::HashMap;
use std::error::Error;
use std::process::Command;
use std::str;
use serde::Deserialize;
use serde::Serialize;
use sha256::digest;

use crate::bitbucket::auth::refresh_git_auth;

use super::hunk::BlameItem;
use super::review::Review;

#[derive(Debug, Serialize, Default, Deserialize)]
pub struct StatItem {
	filepath: String,
	additions: i32,
	deletions: i32,
}

pub fn commit_exists(commit: &str) -> bool {
	let output = Command::new("git")
		.arg("rev-list")
		.arg(commit)
		.output()
		.expect("failed to execute git rev-list");

	output.status.success()
}

pub async fn git_pull(review: &Review) {
	let directory = review.clone_dir();
	println!("directory = {}", &directory);
	let access_token = refresh_git_auth(review.clone_url(), review.clone_dir()).await;
    set_git_url(review.clone_url(), directory, &access_token);
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

fn set_git_url(git_url: &str, directory: &str, access_token: &str) {
    let clone_url = git_url.to_string()
        .replace("git@", format!("https://x-token-auth:{{{access_token}}}@").as_str())
        .replace("bitbucket.org:", "bitbucket.org/");
    let output = Command::new("git")
		.arg("remote").arg("set-url").arg("origin")
		.arg(clone_url)
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

pub fn get_excluded_files(review: &Review) -> Option<(Vec<StatItem>, Vec<StatItem>)> {
	let prev_commit = review.base_head_commit();
	let next_commit = review.pr_head_commit();
	let clone_dir = review.clone_dir();
	println!("prev_commit = {}, next commit = {}, clone_dir = {}", prev_commit, next_commit, clone_dir);
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
						return process_statoutput(statstr);
					},
					Err(e) => {println!("error utf: {e}");},
				};
			},
			Err(commanderr) => {
				eprintln!("git diff stat command failed to start : {commanderr}");
			}
		}
	return None;
}

fn process_statoutput(statstr: &str) -> Option<(Vec<StatItem>, Vec<StatItem>)>{
    let statvec = process_statitem(statstr);
    let mut bigfiles = Vec::<StatItem>::new();
    let mut smallfiles = Vec::<StatItem>::new();
    let line_threshold = 500;
    for item in statvec {
        // logic for exclusion
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
}

fn process_statitem(statstr: &str) -> Vec<StatItem> {
    let statlines = statstr.split("\n");
    let mut statvec = Vec::<StatItem>::new();
    for line in statlines {
        let statitems: Vec<&str> = line.split("\t").collect();
        if statitems.len() >= 3 {
            let statitem = StatItem {
                filepath: statitems[2].to_string(),
                additions: match statitems[0].to_string().parse() {
                    Ok(adds) => {adds}
                    Err(e) => {
                        eprintln!("Unable to parse additions: {:?}", e);
                        0
                    }
                },
                deletions: match statitems[0].to_string().parse() {
                    Ok(dels) => {dels}
                    Err(e) => {
                        eprintln!("Unable to parse deletions: {:?}", e);
                        0
                    }
                },
            };
            statvec.push(statitem);
        }
    }
    return statvec;
}

pub fn generate_diff(review: &Review, smallfiles: &Vec<StatItem>) -> HashMap<String, String> {
	let mut diffmap = HashMap::<String, String>::new();
	let prev_commit = review.base_head_commit();
	let curr_commit = review.pr_head_commit();
	let clone_dir = review.clone_dir();
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

pub fn process_diff(diffmap: &HashMap<String, String>) -> Result<HashMap<String, Vec<String>>,Box<dyn Error>> {
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

pub fn generate_blame(review: &Review, linemap: &HashMap<String, Vec<String>>) ->  Vec<BlameItem>{
	let mut blamevec = Vec::<BlameItem>::new();
	let commit = review.pr_head_commit();
	let clone_dir = review.clone_dir();
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
										blamevec.push(BlameItem::new(
											lineitem.author().to_string(),
											lineitem.timestamp().to_string(),
											linebreak.to_string(),
											lidx.to_string(),
											digest(path.as_str()) ));
										linebreak = lidx + 1;
									}
								}
							}
							let lastidx = linenumint + blamelines.len()-1;
							if lineauthormap.contains_key(&lastidx) {
								let lineitem = lineauthormap.get(&lastidx).expect("lastidx checked");
								blamevec.push(BlameItem::new(
									lineitem.author().to_string(),
									lineitem.timestamp().to_string(),
									linebreak.to_string(),
									lastidx.to_string(),
									digest(path.as_str())));

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

struct LineItem {
    author: String,
    timestamp: String,
}

impl LineItem {
    fn author(&self) -> &String {
        &self.author
    }

    fn timestamp(&self) -> &String {
        &self.timestamp
    }
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