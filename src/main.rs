use clap::{Arg, ArgAction, Command};
use colligo::application::{
    assert_dependencies, generate_default_manifest, get_projects_status, list_projects_path,
    save_file, DwlMode, ManifestInstance, APP_NAME, FORCE, GENERATE_MANIFEST, HTTPS, LIGHT, LIST,
    MANIFEST_INPUT, MANIFEST_INPUT_DEFAULT, PIN, QUIET, STATUS, SYNC,
};
use simple_logger::SimpleLogger;
use std::env;

const DEBUG_OPTION: &str = "debug";
const APP_VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"), "-", env!("GIT_SHA"));

struct UserMessage {
    quiet: bool,
}

impl UserMessage {
    fn new(quiet: bool) -> Self {
        Self {
            quiet
        }
    }

    fn message(&self, msg: String) {
        if !self.quiet {
            println!("{}", msg);
        }
    }
}

fn main() {
    // Generate manifest option
    let generate_manifest = Arg::new(GENERATE_MANIFEST)
        .long(GENERATE_MANIFEST)
        .action(ArgAction::Set)
        .value_name("FILE")
        .help("Generate a default manifest file");

    // Manifest input option
    let manifest_input = Arg::new(MANIFEST_INPUT)
        .long(MANIFEST_INPUT)
        .action(ArgAction::Set)
        .value_name("FILE")
        .help("Manifest file to use (default: manifest.xml)");

    // Sync option
    let sync = Arg::new(SYNC)
        .long(SYNC)
        .action(ArgAction::SetTrue)
        .help("Synchronize all project dependencies from manifest file");

    // HTTPS option
    let https = Arg::new(HTTPS)
        .long(HTTPS)
        .action(ArgAction::SetTrue)
        .default_value("false")
        .help("Use HTTPS instead of SSH");

    // Light option
    let light = Arg::new(LIGHT)
        .long(LIGHT)
        .action(ArgAction::SetTrue)
        .default_value("false")
        .help(
            "Download all projects without history. \
        This option is useful for CI and build servers. \
        All revision MUST point to a branch or a tag, commit ID are not supported.",
        );

    // Quiet option
    let quiet = Arg::new(QUIET)
        .long(QUIET)
        .action(ArgAction::SetTrue)
        .default_value("false")
        .help("Do not print any output, expect for errors.");

    // Force option
    let force = Arg::new(FORCE)
        .long(FORCE)
        .action(ArgAction::SetTrue)
        .default_value("false")
        .help("Discard local changes and overwrite them with the remote version.");

    // Pin option
    let pin = Arg::new(PIN)
        .long(PIN)
        .action(ArgAction::Set)
        .value_name("FILE")
        .help("Manifest file with pinned revisions");

    // List option
    let list = Arg::new(LIST)
        .long("list")
        .action(ArgAction::SetTrue)
        .help("List all projects absolute path in the manifest file");

    // Status option
    let status = Arg::new(STATUS)
        .long("status")
        .action(ArgAction::SetTrue)
        .help("Get the status of all projects in the manifest file");

    // Debug option
    let debug = Arg::new(DEBUG_OPTION)
        .long(DEBUG_OPTION)
        .action(ArgAction::SetTrue)
        .default_value("false")
        .help("Enable debug logs");

    // Application arguments
    let matches = Command::new(APP_NAME)
        .arg(generate_manifest)
        .arg(manifest_input)
        .arg(sync)
        .arg(light)
        .arg(https)
        .arg(quiet)
        .arg(force)
        .arg(pin)
        .arg(list)
        .arg(debug)
        .arg(status)
        .arg_required_else_help(true)
        .version(APP_VERSION)
        .get_matches();

    // Logger
    let log_level = match matches.get_one::<bool>(DEBUG_OPTION) {
        Some(true) => log::LevelFilter::Debug,
        _ => log::LevelFilter::Error,
    };

    SimpleLogger::new()
        .with_level(log_level)
        .init()
        .expect("Failed to initialize logger");

    let quiet = *matches.get_one::<bool>(QUIET).unwrap_or(&false);
    let user = UserMessage::new(quiet);

    let force = *matches.get_one::<bool>(FORCE).unwrap_or(&false);

    // Generate manifest
    if let Some(path) = matches.get_one::<String>(GENERATE_MANIFEST) {
        user.message(format!("Generate manifest file: {}", path));
        if let Err(error_msg) = generate_default_manifest(path) {
            eprintln!("{}", error_msg);
            std::process::exit(1);
        }
        return;
    }

    // Following options needs git to be installed on the system
    if let Err(error_msg) = assert_dependencies() {
        eprintln!("{}", error_msg);
        std::process::exit(1);
    }

    // All following commands require a manifest file
    // Manifest input
    let default_manifest = MANIFEST_INPUT_DEFAULT.to_string();
    let manifest_path = matches
        .get_one::<String>(MANIFEST_INPUT)
        .unwrap_or(&default_manifest);
    let mut manifest = match ManifestInstance::try_from(manifest_path) {
        Ok(manifest) => manifest,
        Err(_) => {
            std::process::exit(1);
        }
    };

    // Parse manifest file. Currently only support XML format.
    user.message(
        format!(
            "Parsing manifest file: {}",
            manifest.get_filename().display()
        ),
    );
    if let Err(error_msg) = manifest.parse() {
        eprintln!("{}", error_msg);
        std::process::exit(1);
    }

    // Download mode
    let dwl_mode = match matches.get_one::<bool>(HTTPS) {
        Some(true) => DwlMode::HTTPS,
        _ => DwlMode::SSH,
    };

    // Synchronize all projects
    if let Some(true) = matches.get_one::<bool>(SYNC) {
        let light = *matches.get_one::<bool>(LIGHT).unwrap_or(&false);

        user.message("Synchronize all projects".to_string());
        if let Err(error_msg) = manifest.sync(&dwl_mode, light, quiet, force) {
            eprintln!("{}", error_msg);
            std::process::exit(1);
        }
        user.message("Synchronization complete".to_string());
    }

    // Pin manifest
    if let Some(path) = matches.get_one::<String>(PIN) {
        user.message("Pin manifest".to_string());
        let pinned = match manifest.pin() {
            Ok(pinned) => pinned,
            Err(error_msg) => {
                eprintln!("{}", error_msg);
                std::process::exit(1);
            }
        };

        if let Err(error_msg) = save_file(path, pinned.get_file()) {
            eprintln!("{}", error_msg);
            std::process::exit(1);
        }
    }

    // List all projects
    if let Some(true) = matches.get_one::<bool>(LIST) {
        let workdir = env::current_dir().expect("Unable to get current directory");
        let projects = list_projects_path(&manifest, &workdir);
        for project in projects {
            println!("{}", project);
        }
    }

    // Status
    if let Some(true) = matches.get_one::<bool>(STATUS) {
        let workdir = env::current_dir().expect("Unable to get current directory");
        let all_status = get_projects_status(&manifest, &workdir);
        for status in all_status {
            println!("{}", status);
        }
    }
}
