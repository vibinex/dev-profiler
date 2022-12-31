use clap::Parser;
use dialoguer::{ MultiSelect, Input };
mod analyzer;
use crate::analyzer::RepoAnalyzer;
mod writer;
use crate::writer::OutputWriter;
mod observer;
use crate::observer::ErrorInfo;
mod scanner;
use crate::scanner::RepoScanner;

#[derive(Parser)]
struct Cli {
    /// path
    path: std::path::PathBuf,
}
fn show_options_to_select_on_cli(options: &[&str], input: &str) {
	loop {
		let filtered_options: Vec<&str> = options
			.iter()
			.filter(|option| option.contains(input))
			.cloned()
			.collect();

		if filtered_options.is_empty() {
			println!("No options available");
			return;
		}

		let selection: Vec<usize> = MultiSelect::new()
			.items(&filtered_options)
			.interact()
			.unwrap();

		for option in selection {
			println!("Option {} selected", option + 1);
		}
		let input = Input::new()
			.with_prompt("Do you want to select more options? (y/n) ")
			.default("n".to_string())
			.interact();
		if input.is_ok(){
			break;
		}
	}
}

fn main() {
    let args = Cli::parse();
	let options = ["Option 1", "Option 2", "Option 3"]; //hardcoded the values right now. We will be using emails and repo options instead.
	let input: String = Input::new()
		.with_prompt("Enter search term ")
		.interact()
		.unwrap();
		
	show_options_to_select_on_cli(&options, &input);
    let writer_result = OutputWriter::new();
    if writer_result.is_ok() {
        let writer = &mut writer_result.expect("Checked, is ok");
        let einfo = &mut ErrorInfo::new();
        let rscanner = RepoScanner::new(args.path);
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
}