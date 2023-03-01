#![feature(test)]
#![feature(string_remove_matches)]

use dotenv::dotenv;
use std::{env, process};

use lint_apptester::{get_project_root, process_subdir, rules::get_rules, Config, Project, Result};

fn main() {
    dotenv().ok();

    let config = Config::build(env::args()).unwrap_or_else(|err| {
        eprintln!("Problem with arguments: {err}");
        process::exit(1);
    });

    let project_root = get_project_root(config.get_current_dir()).unwrap_or_else(|err| {
        eprintln!("Problem getting project root: {err}");
        process::exit(1);
    });
    let project = Project::init(&project_root, config.get_feature()).unwrap_or_else(|err| {
        eprintln!("Problem initialising: {err}");
        process::exit(1);
    });

    if let Err(err) = run(project) {
        eprintln!("Application error: {err}");
        process::exit(1);
    }
}

fn run(project: Project) -> Result<()> {
    let rules = get_rules();

    for subdir in project.get_subdirs() {
        process_subdir(subdir, &rules)?;
    }

    Ok(())
}
