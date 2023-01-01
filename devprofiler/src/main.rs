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
use std::process;
use std::path::Path;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Serialize, Default)]
struct UserAlias {
	alias: Vec::<String>
}

fn process_repos(user_paths: Vec::<String>, einfo: &mut RuntimeInfo, writer: &mut OutputWriter) -> Vec::<String> {
	let mut valid_repo = 0;
	let mut all_aliases = HashSet::<String>::new();
	for p in user_paths {
		let ranalyzer_res = RepoAnalyzer::new(p.as_str().as_ref());
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

fn process_aliases(alias_vec: Vec::<String>, einfo: &mut RuntimeInfo, writer: &mut OutputWriter) {
	match UserInput::alias_selector(alias_vec) {
		Ok(user_aliases) => {
			let alias_obj = UserAlias{ alias: user_aliases };
			let alias_str = serde_json::to_string(&alias_obj).unwrap_or_default();
			match writer.writeln(alias_str.as_str().as_ref()) {
				Ok(_) => {},
				Err(writer_err) => {
					eprintln!("Unable to record user aliases in output file : {writer_err}");
					einfo.record_err(writer_err.to_string().as_str().as_ref());
				}
			}
		 }
		Err(error) => { 
			eprintln!("Unable to process user aliases : {:?}", error);
			einfo.record_err(error.to_string().as_str().as_ref()); 
		}
	}
}

fn main() {
	match UserInput::scan_path() {
		Ok(scan_path_str) => {
			match OutputWriter::new(){
				Ok(mut writer) => {
					let writer_mut: &mut OutputWriter = &mut writer;
					let einfo = &mut RuntimeInfo::new();
					let scan_pathbuf = Path::new(&scan_path_str).to_path_buf();
					let rscanner = RepoScanner::new(scan_pathbuf);
					let pathsvec = rscanner.scan(einfo);
					match UserInput::repo_selection(pathsvec) {
						Ok(user_paths) => {
							let alias_vec = process_repos(user_paths, einfo, writer_mut);
							process_aliases(alias_vec, einfo, writer_mut);
							let _res = einfo.write_runtime_info(writer_mut);
							match writer.finish() {
								Ok(_) => {},
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
		},
		Err(error) => {
			eprintln!("Unable to start application : {error}");
		}
	}
}