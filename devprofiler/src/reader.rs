use dialoguer::{ MultiSelect, Input };

pub struct UserInput {
}

impl UserInput {
    pub fn scan_path() -> Result<String, std::io::Error>{
        let input : String = Input::new()
        .with_prompt("Enter path containing git repo(s)")
        .interact()?;
        Ok(input)
    }

    pub fn user_error(err_str: &str) {
        println!("{}", err_str);  
    }

    pub fn repo_selection(options: &Vec::<String>) {
        Self::fuzzy_multi_select(options);
    }

    pub fn alias_selector(options: &Vec::<String>) {
        Self::fuzzy_multi_select(options);
    }

    fn fuzzy_multi_select(options: &Vec::<String>) {
        let input: String = Input::new()
            .with_prompt("Enter search term ")
            .interact()
            .unwrap();
        Self::show_options_to_select_on_cli(options, &input);
    }
    fn show_options_to_select_on_cli(options: &Vec::<String>, input: &str) {
        loop {
            let filtered_options: Vec<&String> = options
                .into_iter()
                .filter(|option| option.contains(input))
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
}