mod reader;
use crate::reader::UserInput;
mod analyzer;
use crate::analyzer::RepoAnalyzer;
mod writer;
use crate::writer::OutputWriter;
mod observer;
use crate::observer::ErrorInfo;
mod scanner;
use crate::scanner::RepoScanner;
use std::path::Path;


fn main() {
	match UserInput::scan_path() {
		Ok(scan_path_str) => {
			let writer_result = OutputWriter::new();
			if writer_result.is_ok() {
				let writer = &mut writer_result.expect("Checked, is ok");
				let einfo = &mut ErrorInfo::new();
				let scan_pathbuf = Path::new(&scan_path_str).to_path_buf();
				let rscanner = RepoScanner::new(scan_pathbuf);
				let pathvec = rscanner.scan(einfo);
				let mut valid_path = 0;
				for p in pathvec {
					let ranalyzer_res = RepoAnalyzer::new(p);
					match ranalyzer_res {
						Ok(ranalyzer) => {
							valid_path += 1;
							let anal_res = ranalyzer.analyze(writer, einfo);
							match anal_res {
								Ok(aliases) => println!("aliases = {:?}", aliases),
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
				let _res = einfo.write_err(writer);
				let _res2 = writer.finish();
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