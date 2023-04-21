mod reader;
use crate::reader::UserInput;
mod analyzer;
use crate::analyzer::RepoAnalyzer;
mod writer;
use crate::writer::OutputWriter;
mod observer;
use crate::observer::RuntimeInfo;
mod scanner;
use crate::scanner::RepoScanner;
use std::hash::Hash;
use std::process;
use std::path::Path;
use serde::{Serialize, Deserialize};
use std::collections::HashSet;
use std::io::Write;
use std::io;
use clap::Parser;
use std::path::PathBuf;
use std::process::Command;
use std::str;
use std::collections::HashMap;
use sha256::digest;


#[derive(Parser)]
struct Cli {
    /// Specify arg parsing mode for cli
    provider: Option<String>,
	/// path scanned for repositories
    path: Option<PathBuf>,
	//// repository name and owner
	repo_slug: Option<String>,
}

#[derive(Debug, Serialize, Default)]
struct UserAlias {
	alias: Vec::<String>
}

#[derive(Debug, Serialize, Default, Deserialize)]
struct Reviews {
    reviews: Vec<ReviewItem>,
}

#[derive(Debug, Serialize, Default, Deserialize)]
struct ReviewItem {
	prev_commit: String,
	curr_commit: String,
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

fn process_repos(user_paths: Vec::<String>, einfo: &mut RuntimeInfo, writer: &mut OutputWriter, repo_slug: Option<String>, provider: Option<String>) -> Vec::<String> {
	let mut valid_repo = 0;
	let mut all_aliases = HashSet::<String>::new();
	let num_user_path = user_paths.len();
	let mut count = 0;
	// TODO - optimize count and iterating of vector user_path, get index in for loop
	for p in user_paths {
		count += 1;
		print!("Scanning [{count}/{num_user_path}] \r");
		let _res = io::stdout().flush();
		let ranalyzer_res = RepoAnalyzer::new(p.as_str().as_ref(), &repo_slug, &provider);
		match ranalyzer_res {
			Ok(ranalyzer) => {
				valid_repo += 1;
				let anal_res = ranalyzer.analyze(writer, einfo);
				match anal_res {
					Ok(aliases) => { all_aliases.extend(aliases); },
					Err(anal_err) => {
						einfo.record_err(anal_err
							.to_string().as_str().as_ref());
					}
				}
			},
			Err(ranalyzer_err) => {
				eprintln!("Unable to parse {p} due to error : {ranalyzer_err}");
				einfo.record_err(ranalyzer_err
					.to_string().as_str().as_ref());
			}
		}
	}
	if valid_repo == 0 {
		let err_line = "Unable to parse any provided repo(s)";
		eprintln!("{err_line}");
		einfo.record_err(err_line);
		process::exit(1);
	}
	let alias_vec = all_aliases.into_iter().collect();
	alias_vec
}

fn process_aliases(alias_vec: Vec::<String>, einfo: &mut RuntimeInfo, writer: &mut OutputWriter, dockermode: bool) {
	match dockermode {
		true => {
			let alias_obj = UserAlias{ alias: alias_vec };
			let alias_str = serde_json::to_string(&alias_obj).unwrap_or_default();
			match writer.writeln(alias_str.as_str().as_ref()) {
				Ok(_) => {},
				Err(writer_err) => {
					eprintln!("Unable to record user aliases in output file : {writer_err}");
					einfo.record_err(writer_err.to_string().as_str().as_ref());
					let _res = writer.finish(); // result doesn't matter since already in error
					process::exit(1);
				}
			}
		}
		false => {
			match UserInput::alias_selector(alias_vec) {
				Ok(user_aliases) => {
					let alias_obj = UserAlias{ alias: user_aliases };
					let alias_str = serde_json::to_string(&alias_obj).unwrap_or_default();
					match writer.writeln(alias_str.as_str().as_ref()) {
						Ok(_) => {},
						Err(writer_err) => {
							eprintln!("Unable to record user aliases in output file : {writer_err}");
							einfo.record_err(writer_err.to_string().as_str().as_ref());
							let _res = writer.finish(); // result doesn't matter since already in error
							process::exit(1);
						}
					}
				}
				Err(error) => { 
					eprintln!("Unable to process user aliases : {:?}", error);
					einfo.record_err(error.to_string().as_str().as_ref());
					let _res = writer.finish(); // result doesn't matter since already in error
					process::exit(1); 
				}
			}
		}
	}	
}

fn generate_diff(prev_commit: &str, curr_commit: &str, smallfiles: &Vec<StatItem>) -> HashMap<String, String> {
	// println!("smallfiles = {:#?}", smallfiles);
	let mut diffmap = HashMap::<String, String>::new();
	for item in smallfiles {
		let filepath = item.filepath.as_str();
		let params = vec![
		"diff".to_string(),
		format!("{prev_commit}:{filepath}"),
		format!("{curr_commit}:{filepath}"),
		"-U0".to_string()];
		println!("params = {:#?}", params);
		let result = Command::new("git")
		.args(&params)
        .output()
        .expect("git diff command failed to start");
		let diff = result.stdout;
		let diffstr = match str::from_utf8(&diff) {
			Ok(v) => v,
			Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
		};
		println!("diff for item {filepath} = {:?}", diffstr);
		diffmap.insert(filepath.to_string(), diffstr.to_string());
	}
	println!("diffmap = {:#?}", diffmap);
	return diffmap;
}

fn generate_blame(commit: &str, linemap: &HashMap<String, Vec<String>>) ->  Vec<BlameItem>{
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
			let resultblame = Command::new("git")
				.args(paramvec)
				.output()
				.expect("git blame command failed to start");
			let blame = resultblame.stdout;
			let blamestr = match str::from_utf8(&blame) {
				Ok(v) => v,
				Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
			};
			let blamelines: Vec<&str> = blamestr.lines().collect();
			let wordvec: Vec<&str> = blamelines[0].split(" ").collect();
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
			let hunkstr = wordvec[idx+3..].to_vec().join(" ");
			let hunkhash = digest(hunkstr);
			blamevec.push(
				BlameItem { 
					hunkhash: hunkhash,
					author: authorstr.to_string(),
					timestamp: timestamp.to_string(), 
				}
			);
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

fn get_tasks(provider: &str, repo_slug: &str) -> Reviews{
	let api_url = "http://127.0.0.1:8080/relevance/hunk";
	let client = reqwest::blocking::Client::new();
	let mut map = HashMap::new();
	let (repo_name, repo_owner) = process_reposlug(repo_slug);
	map.insert("repo_provider", provider);
	map.insert("repo_owner", repo_owner.as_str());
	map.insert("repo_name", repo_name.as_str());
	let response = client.post(api_url)
    	.json(&map)
    	.send().expect("Get request failed")
        .json::<Reviews>().expect("Json parsing of response failed");
	return response;
}

fn store_hunkmap(hunkmap: HunkMap) {
	let api_url = "http://127.0.0.1:8080/relevance/hunk/store";
	let client = reqwest::blocking::Client::new();
	let response = client.post(api_url)
    	.json(&hunkmap)
    	.send().expect("Get /relevance/hunk/store request failed")
        .text();
	println!("{:#?}", response);
}

fn get_excluded_files(prev_commit: &str, next_commit: &str) -> (Vec<StatItem>, Vec<StatItem>) {
	// Use the command
	let resultstat = Command::new("git")
		.args(&["diff", prev_commit, next_commit, "--numstat"])
		.output()
		.expect("git diff stat command failed to start");
	let stat = resultstat.stdout;
	// parse the output
	let statstr = match str::from_utf8(&stat) {
		Ok(v) => v,
		Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
	};
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
	let line_threshold = 50;
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
	// compile the result and return
	return (bigfiles, smallfiles);
}

fn process_diff(diffmap: &HashMap<String, String>) -> HashMap<String, Vec<String>> {
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
					deletionstr.push_str(format!(",{}", delidx+1).as_str());
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
	return linemap;
}

fn unfinished_tasks(provider: &str, repo_slug: &str) {
	let reviews = get_tasks(provider, repo_slug);
	let mut prvec = Vec::<PrHunkItem>::new();
	for review in reviews.reviews {
		let (bigfiles, smallfiles) = get_excluded_files(&review.prev_commit, &review.curr_commit);
		let diffmap = generate_diff(&review.prev_commit, &review.curr_commit, &smallfiles);
		let linemap = process_diff(&diffmap);
		let blamevec = generate_blame(&review.prev_commit, &linemap);
		let hmapitem = PrHunkItem {
			pr_number: review.id,
			blamevec: blamevec,
		};
		prvec.push(hmapitem);
	}
	let (repo_name, repo_owner) = process_reposlug(repo_slug);
	let hunkmap = HunkMap { repo_provider: provider.to_string(),
		repo_owner: repo_owner, repo_name: repo_name, prhunkvec: prvec };
	store_hunkmap(hunkmap);
}

fn main() {
	let args = Cli::parse();
	let mut dockermode = false;
	match args.provider {
		Some(ref argval) => {
			if argval == "github" || argval == "bitbucket" {
				dockermode = true;
			}
		}
		None => {}
	}
	match OutputWriter::new() {
		Ok(mut writer) => {
			match dockermode {
				true => {
					unfinished_tasks(args.provider.as_ref().expect("Provider exists, checked"), args.repo_slug.as_ref().expect("No repo_slug"));
					let writer_mut: &mut OutputWriter = &mut writer;
					let einfo = &mut RuntimeInfo::new();
					let scan_pathbuf = match args.path {
						Some(scan_pathbuf) => scan_pathbuf,
						None => Path::new("/").to_path_buf()
					};
					let rscanner = RepoScanner::new(scan_pathbuf);
					let pathsvec = rscanner.scan(einfo, writer_mut, dockermode);
					let alias_vec = process_repos(pathsvec, einfo, writer_mut, args.repo_slug, args.provider);
					process_aliases(alias_vec, einfo, writer_mut, dockermode);
					let _res = einfo.write_runtime_info(writer_mut);
					match writer.finish() {
						Ok(_) => {
							println!("Extracted and uploaded metadata successfully! Proceed to https://vibinex.com/ to learn more");
						},
						Err(error) => {
							eprintln!("Unable to write to output : {error}");
						}
					}
				}
				false => {
					match UserInput::scan_path() {
						Ok(scan_path_str) => {
							let writer_mut: &mut OutputWriter = &mut writer;
							let einfo = &mut RuntimeInfo::new();
							let scan_pathbuf = Path::new(&scan_path_str).to_path_buf();
							let rscanner = RepoScanner::new(scan_pathbuf);
							let pathsvec = rscanner.scan(einfo, writer_mut, dockermode);
							match UserInput::repo_selection(pathsvec) {
								Ok(user_paths) => {
									let alias_vec = process_repos(user_paths, einfo, writer_mut, None, None);
									process_aliases(alias_vec, einfo, writer_mut, dockermode);
									let _res = einfo.write_runtime_info(writer_mut);
									match writer.finish() {
										Ok(_) => {
											println!("Extracted and uploaded metadata successfully! Proceed to https://vibinex.com/ to learn more");
										},
										Err(error) => {
											eprintln!("Unable to write to output : {error}");
										}
									}
								},
								Err(error) => {
									eprintln!("Unable to process repository selection : {error}");
								}
							} 
						},
						Err(error) => {
							eprintln!("Unable to write to present directory : {error}");
						}
					}
				}
			}
		},
		Err(error) => {
			eprintln!("Unable to start application : {error}");
		}
	}
}
// git diff a9e58c7 8433a5e -U0
// git blame a9e58c7 -L 121,+5 -e --date=unix devprofiler/src/main.rs
// git diff a9e58c7:devprofiler/src/analyzer.rs 8433a5e:devprofiler/src/analyzer.rs'
// git diff a9e58c7 8433a5e --stat
