use inquire::{
    list_option::ListOption,
    validator::Validation,
    error::InquireResult,
    MultiSelect,
    Text,
};

pub struct UserInput {}

impl UserInput {
    pub fn scan_path() -> InquireResult<String>{
        Text::new("Enter path containing one or more git repo(s)").prompt()
    }

    pub fn repo_selection(options: Vec::<String>) -> InquireResult<Vec::<String>>{
        MultiSelect::new(
            (format!("Select relevant repo(s) out of {} repo(s)", options.len())).as_str(), 
            options)
            .with_validator(|a: &[ListOption<&String>]| {
                if a.len() < 1 {
                    return Ok(Validation::Invalid("Please select at least one repo".into()));
                }
                Ok(Validation::Valid)
            })
            .prompt()
    }

    pub fn alias_selector(options: Vec::<String>) -> InquireResult<Vec::<String>>{
        MultiSelect::new(
            (format!("Select your email alias(es) out of {} alias(es)", options.len())).as_str(),
             options)
            .with_validator(|a: &[ListOption<&String>]| {
                if a.len() < 1 {
                    return Ok(Validation::Invalid("Please select at least one alias".into()));
                }
                Ok(Validation::Valid)
            })
            .prompt()
    }
}