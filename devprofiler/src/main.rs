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
mod reviewer;
use crate::reviewer::unfinished_tasks;
use std::process;
use std::path::Path;
use serde::{Serialize};
use std::collections::HashSet;
use std::io::Write;
use std::io;
use clap::Parser;
use std::path::PathBuf;


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
					let einfo = &mut RuntimeInfo::new();
					unfinished_tasks(args.provider.as_ref().expect("Provider exists, checked"), args.repo_slug.as_ref().expect("No repo_slug"), einfo);
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
