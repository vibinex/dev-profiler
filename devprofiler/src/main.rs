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
use std::path::Path;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Serialize, Default)]
struct UserAlias {
	alias: Vec::<String>
}

fn main() {
	match UserInput::scan_path() {
		Ok(scan_path_str) => {
			let writer_result = OutputWriter::new();
			if writer_result.is_ok() {
				let writer = &mut writer_result.expect("Checked, is ok");
				let einfo = &mut RuntimeInfo::new();
				let scan_pathbuf = Path::new(&scan_path_str).to_path_buf();
				let rscanner = RepoScanner::new(scan_pathbuf);
				let pathsvec = rscanner.scan(einfo);
				match UserInput::repo_selection(pathsvec) {
					Ok(user_paths) => {
						let mut valid_path = 0;
						let mut all_aliases = HashSet::<String>::new();
						for p in user_paths {
							let ranalyzer_res = RepoAnalyzer::new(p);
							match ranalyzer_res {
								Ok(ranalyzer) => {
									valid_path += 1;
									let anal_res = ranalyzer.analyze(writer, einfo);
									match anal_res {
										Ok(aliases) => { all_aliases.extend(aliases); },
										Err(anal_err) => {
											einfo.push(anal_err
												.to_string().as_str().as_ref());
										}
									}
								},
								Err(ranalyzer_err) => {
									einfo.push(ranalyzer_err.to_string().as_str().as_ref());
								}
							}
						}
						if valid_path == 0 {
							let err_line = "Unable to parse a single repo";
							// TODO - display on ui
							einfo.push(err_line);
						}
						let alias_vec = all_aliases.into_iter().collect();
						match UserInput::alias_selector(alias_vec) {
							Ok(user_aliases) => {
								let alias_obj = UserAlias{ alias: user_aliases };
								let alias_str = serde_json::to_string(&alias_obj).unwrap_or_default();
								match writer.writeln(alias_str.as_str().as_ref()) {
									Ok(_) => {},
									Err(writer_err) => {
										einfo.push(writer_err.to_string().as_str().as_ref());
									}
								}
							 }
							Err(error) => { 
								eprintln!("Unable to process user aliases : {:?}", error);
								einfo.push(error.to_string().as_str().as_ref()); 
							}
						}
						let _res = einfo.write_runtime_info(writer);
						let _res2 = writer.finish();
					},
					Err(error) => {
						eprintln!("Unable to process repository selection : {:?}", error);
					}
				} 
			}
			else {
				let err = writer_result.err().expect("Checked, is err");
				eprintln!("Unable to write to present directory : {err}");
			}
		},
		Err(error) => {
			eprintln!("Unable to start application : {:?}", error);
		}
	}
}