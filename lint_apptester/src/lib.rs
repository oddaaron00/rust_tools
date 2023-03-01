#![feature(test)]

extern crate test;

use std::{
    collections::HashMap,
    env,
    error::Error,
    fs::{self, File},
    path::Path,
    process::Command,
    str,
};

use colored::Colorize;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, PartialEq)]
pub enum DirType {
    Features,
    Interactions,
    Pages,
    Steps,
}

pub struct Config {
    current_dir: String,
    feature: String,
}

pub struct Project {
    feature_being_tested: String,

    // ci_runner_subdir: Subdir,
    features_subdir: Subdir,
    interactions_subdir: Subdir,
    pages_subdir: Subdir,
    steps_subdir: Subdir,
}

pub struct Rule {
    name: String,
    rule: fn(&File) -> bool,
    dir_types: Vec<DirType>,
}

pub struct Rules {
    rules: Vec<Rule>,
}

pub struct Subdir {
    path: Box<Path>,
    subdir_type: DirType,
}

// TODO: Allow env vars to specify pages, interactions, etc dirs
impl Config {
    pub fn build(mut args: impl Iterator<Item = String>) -> Result<Config> {
        args.next();

        let current_dir = env::current_dir()?.to_str().unwrap().to_string();

        let feature = match args.next() {
            Some(arg) => arg,
            None => return Err("Didn't get a feature to test".into()),
        };

        let current_dir = match args.next() {
            Some(arg) => arg,
            None => current_dir,
        };

        Ok(Config {
            current_dir,
            feature,
        })
    }

    pub fn get_current_dir(&self) -> &str {
        &self.current_dir
    }

    pub fn get_feature(&self) -> &str {
        &self.feature
    }
}

impl Project {
    pub fn init(project_root: &str, feature_being_tested: &str) -> Result<Self> {
        let feature_being_tested = feature_being_tested.to_lowercase();

        let features_path_string = std::env::var("FEATURES_PATH")?;
        let features_subdir_path_string =
            format!("{project_root}{features_path_string}{feature_being_tested}");
        let features_subdir = Subdir::new(features_subdir_path_string, DirType::Features)?;

        let interactions_path_string = std::env::var("INTERACTIONS_PATH")?;
        let interactions_subdir_path_string =
            format!("{project_root}{interactions_path_string}{feature_being_tested}");
        let interactions_subdir =
            Subdir::new(interactions_subdir_path_string, DirType::Interactions)?;

        let pages_path_string = std::env::var("PAGES_PATH")?;
        let pages_subdir_path_string =
            format!("{project_root}{pages_path_string}{feature_being_tested}");
        let pages_subdir = Subdir::new(pages_subdir_path_string, DirType::Pages)?;

        let steps_path_string = std::env::var("STEPS_PATH")?;
        let steps_subdir_path_string =
            format!("{project_root}{steps_path_string}{feature_being_tested}");
        let steps_subdir = Subdir::new(steps_subdir_path_string, DirType::Steps)?;

        Ok(Self {
            feature_being_tested,
            features_subdir,
            interactions_subdir,
            pages_subdir,
            steps_subdir,
        })
    }

    pub fn get_feature_being_tested(&self) -> &str {
        &self.feature_being_tested
    }

    pub fn get_subdirs(&self) -> Vec<&Subdir> {
        vec![
            // &self.ci_runner_subdir,
            &self.features_subdir,
            &self.interactions_subdir,
            &self.pages_subdir,
            &self.steps_subdir,
        ]
    }
}

impl Rule {
    pub fn new(name: &str, rule: fn(&File) -> bool, dir_types: Vec<DirType>) -> Self {
        Self {
            name: String::from(name),
            rule,
            dir_types,
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_rule(&self) -> &fn(&File) -> bool {
        &self.rule
    }

    pub fn get_dir_types(&self) -> &Vec<DirType> {
        &self.dir_types
    }
}

impl Rules {
    pub fn init() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    pub fn get_rules(&self) -> &Vec<Rule> {
        &self.rules
    }
}

impl Subdir {
    pub fn new(subdir_path_string: String, subdir_type: DirType) -> Result<Self> {
        let path = Path::new(&subdir_path_string);
        let path = if path.exists() {
            path.into()
        } else {
            return Err(format!("Could not locate {subdir_path_string}").into());
        };

        Ok(Self { path, subdir_type })
    }

    pub fn get_path(&self) -> &Path {
        &self.path
    }

    pub fn get_subdir_type(&self) -> &DirType {
        &self.subdir_type
    }
}

pub fn get_project_root(current_dir: &str) -> Result<String> {
    let command_output = match Command::new("git")
        .current_dir(current_dir)
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
    {
        Ok(output) => output,
        Err(_) => return Err("Could not run command 'git rev-parse --show-toplevel'".into()),
    };

    let stderr = str::from_utf8(&command_output.stderr)?;
    if !stderr.is_empty() {
        return Err(stderr.into());
    }

    let stdout = str::from_utf8(&command_output.stdout)?.trim_end();
    if stdout.is_empty() {
        return Err("'git rev-parse --show-toplevel' output is empty".into());
    }
    let repo_name = std::env::var("REPOSITORY_NAME")?;
    if !cfg!(debug_assertions) && !stdout.ends_with(&repo_name) {
        return Err("Not in the correct repository".into());
    }

    let dev_project_root = std::env::var("DEV_PROJECT_ROOT")?;
    let project_root = if cfg!(debug_assertions) {
        &dev_project_root
    } else {
        stdout
    };

    Ok(project_root.to_owned())
}

pub fn print_results(rules: Vec<&Rule>, rule_status_map: HashMap<&str, bool>) {
    for &rule in &rules {
        println!(
            "  - {}: {}",
            rule.get_name(),
            if *rule_status_map.get(rule.get_name()).unwrap() {
                "PASS".green()
            } else {
                "FAIL".red()
            }
        );
    }
}

pub fn process_subdir(subdir: &Subdir, rules: &Rules) -> Result<()> {
    let rules: Vec<&Rule> = rules
        .get_rules()
        .iter()
        .filter(|&rule| rule.get_dir_types().contains(subdir.get_subdir_type()))
        .collect();
    println!(
        "{:?} ({}):",
        subdir.get_subdir_type(),
        subdir.get_path().to_str().unwrap()
    );
    if rules.is_empty() {
        println!("  # No rules for this directory");
        return Ok(());
    }

    // TODO: Map rule to Vec<u16> (line numbers with problems)
    let mut rule_status_map: HashMap<&str, bool> = HashMap::new();
    for &rule in &rules {
        rule_status_map.insert(rule.get_name(), true);
    }

    let dir = fs::read_dir(subdir.get_path()).unwrap();
    for entry in dir {
        let entry = entry?;
        if !["feature", "java", "js"].contains(
            &entry
                .path()
                .extension()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default(),
        ) {
            continue;
        }

        for &rule in &rules {
            let file = File::open(entry.path())?; // Inefficient: Pass buffer.by_ref() to closure - figure out
            if rule.get_dir_types().contains(subdir.get_subdir_type()) && !(rule.get_rule())(&file)
            {
                rule_status_map.insert(rule.get_name(), false);
            }
        }
    }

    print_results(rules, rule_status_map);
    Ok(())
}

pub mod rules {
    use std::{
        fs::File,
        io::{BufRead, BufReader},
    };

    use crate::{DirType, Rule, Rules};

    pub fn get_rules() -> Rules {
        let mut rules = Rules::init();
        rules.add_rule(get_log_instead_of_sout());
        rules.add_rule(get_no_assert_calls());
        rules.add_rule(get_no_locator_calls());
        rules.add_rule(get_platform_locator_methods());

        rules
    }

    fn get_log_instead_of_sout() -> Rule {
        Rule::new(
            "Log instead of sout",
            |file: &File| {
                let buffered_reader = BufReader::new(file);
                buffered_reader
                    .lines()
                    .map(|line| {
                        let line = line.unwrap();
                        return line.trim().to_owned();
                    })
                    .skip_while(|line| !line.starts_with("public class"))
                    .filter(|line| !line.starts_with("//"))
                    .all(|line| !line.starts_with("System.out.print"))
            },
            vec![DirType::Interactions, DirType::Pages, DirType::Steps],
        )
    }

    fn get_no_assert_calls() -> Rule {
        Rule::new(
            "No assert calls",
            |file: &File| {
                let buffered_reader = BufReader::new(file);
                buffered_reader
                    .lines()
                    .map(|line| {
                        let line = line.unwrap();
                        return line.trim().to_owned();
                    })
                    .skip_while(|line| !line.starts_with("public class"))
                    .filter(|line| !line.starts_with("//"))
                    .all(|line| !line.contains("assert"))
            },
            vec![DirType::Steps],
        )
    }

    fn get_no_locator_calls() -> Rule {
        Rule::new(
            "No locator calls",
            |file: &File| {
                let locator_class_path = match std::env::var("LOCATOR_CLASS_PATH") {
                    Ok(path) => path,
                    Err(_) => {
                        eprintln!("Could not find variable LOCATOR_CLASS_PATH");
                        return false; // Will always fail
                    }
                };

                let buffered_reader = BufReader::new(file);
                buffered_reader
                    .lines()
                    .map(|line| {
                        let line = line.unwrap();
                        return line.trim().to_owned();
                    })
                    .take_while(|line| !line.starts_with("public class"))
                    .filter(|line| !line.starts_with("//"))
                    .all(|line| !line.starts_with(&locator_class_path))
            },
            vec![DirType::Steps, DirType::Interactions],
        )
    }

    fn get_platform_locator_methods() -> Rule {
        Rule::new(
            "Use platform Locator methods",
            |file: &File| {
                let buffered_reader = BufReader::new(file);
                buffered_reader
                    .lines()
                    .map(|line| {
                        let line = line.unwrap();
                        return line.trim().to_owned();
                    })
                    .skip_while(|line| !line.starts_with("public class"))
                    .filter(|line| !line.starts_with("//"))
                    .filter(|line| line.contains("Locator."))
                    .all(|line| line.contains("Platform") || line.contains("Children"))
            },
            vec![DirType::Pages],
        )
    }

    #[cfg(test)]
    mod tests {
        use test::{black_box, Bencher};

        use super::{
            get_log_instead_of_sout, get_no_assert_calls, get_no_locator_calls,
            get_platform_locator_methods, get_rules,
        };
        use crate::{get_project_root, process_subdir, Config, Project, Rules};
        use dotenv::dotenv;

        fn get_path() -> String {
            dotenv().ok();
            std::env::var("DEV_PROJECT_ROOT").unwrap()
        }

        #[bench]
        fn bench_all_rules(b: &mut Bencher) {
            let config =
                Config::build(["".to_owned(), "Files".to_owned(), get_path()].into_iter()).unwrap();
            let project_root = get_project_root(&config.current_dir).unwrap();
            let project = Project::init(&project_root, "Files").unwrap();
            let rules = get_rules();

            b.iter(black_box(|| {
                for subdir in project.get_subdirs() {
                    process_subdir(subdir, &rules).unwrap();
                }
            }))
        }

        #[bench]
        fn bench_rule_log_instead_of_sout(b: &mut Bencher) {
            let config =
                Config::build(["".to_owned(), "Files".to_owned(), get_path()].into_iter()).unwrap();
            let project_root = get_project_root(&config.current_dir).unwrap();
            let project = Project::init(&project_root, "Files").unwrap();
            let mut rules = Rules::init();
            rules.add_rule(get_log_instead_of_sout());

            b.iter(black_box(|| {
                for subdir in project.get_subdirs() {
                    process_subdir(subdir, &rules).unwrap();
                }
            }))
        }

        #[bench]
        fn bench_rule_no_assert_calls(b: &mut Bencher) {
            let config =
                Config::build(["".to_owned(), "Files".to_owned(), get_path()].into_iter()).unwrap();
            let project_root = get_project_root(&config.current_dir).unwrap();
            let project = Project::init(&project_root, "Files").unwrap();
            let mut rules = Rules::init();
            rules.add_rule(get_no_assert_calls());

            b.iter(black_box(|| {
                for subdir in project.get_subdirs() {
                    process_subdir(subdir, &rules).unwrap();
                }
            }))
        }

        #[bench]
        fn bench_rule_no_locator_calls(b: &mut Bencher) {
            let config =
                Config::build(["".to_owned(), "Files".to_owned(), get_path()].into_iter()).unwrap();
            let project_root = get_project_root(&config.current_dir).unwrap();
            let project = Project::init(&project_root, "Files").unwrap();
            let mut rules = Rules::init();
            rules.add_rule(get_no_locator_calls());

            b.iter(black_box(|| {
                for subdir in project.get_subdirs() {
                    process_subdir(subdir, &rules).unwrap();
                }
            }))
        }

        #[bench]
        fn bench_rule_platform_locator_methods(b: &mut Bencher) {
            let config =
                Config::build(["".to_owned(), "Files".to_owned(), get_path()].into_iter()).unwrap();
            let project_root = get_project_root(&config.current_dir).unwrap();
            let project = Project::init(&project_root, "Files").unwrap();
            let mut rules = Rules::init();
            rules.add_rule(get_platform_locator_methods());

            b.iter(black_box(|| {
                for subdir in project.get_subdirs() {
                    process_subdir(subdir, &rules).unwrap();
                }
            }))
        }
    }
}
