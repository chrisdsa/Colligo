use clap::{Arg, ArgAction, Command};
use colligo::application::{
    assert_dependencies, generate_default_manifest, save_file, DwlMode, ExitCode, ManifestInstance,
    APP_NAME, GENERATE_MANIFEST, HTTPS, MANIFEST_INPUT, PIN, SYNC,
};
use colligo::git_version_control::GitVersionControl;
use colligo::version::APP_VERSION;
use colligo::xml_parser::XmlParser;
use simple_logger::SimpleLogger;

const DEBUG_OPTION: &str = "debug";

fn main() -> Result<(), ExitCode> {
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
    /* TODO: implement light option
    let light = Arg::new(LIGHT)
        .long(LIGHT)
        .action(ArgAction::SetTrue)
        .default_value("false")
        .help(
            "Download all projects without history. \
        This option is useful for CI and build servers. \
        All revision MUST point to a branch or a tag, commit ID are not supported.",
        );
     */

    // Pin option
    let pin = Arg::new(PIN)
        .long(PIN)
        .action(ArgAction::Set)
        .value_name("FILE")
        .help("Manifest file with pinned revisions");

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
        .arg(https)
        .arg(pin)
        .arg(debug)
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

    // Generate manifest
    if let Some(path) = matches.get_one::<String>(GENERATE_MANIFEST) {
        println!("Generate manifest file: {}", path);
        generate_default_manifest(path)?;
        return Ok(());
    }

    // Following options needs git to be installed on the system
    assert_dependencies()?;

    // All following commands require a manifest file
    // Manifest input
    let manifest_path = matches.get_one::<String>(MANIFEST_INPUT);
    let mut manifest = ManifestInstance::new(manifest_path)?;

    // Parse manifest file. Currently only support XML format.
    let xml_parser = XmlParser::new();
    println!("Parsing manifest file: {}", manifest.get_filename());
    manifest.parse(&xml_parser)?;

    // Download mode
    let dwl_mode = match matches.get_one::<bool>(HTTPS) {
        Some(true) => DwlMode::HTTPS,
        _ => DwlMode::SSH,
    };

    // Get version control system. Currently only support Git.
    let vcs = GitVersionControl::new();

    // Synchronize all projects
    if let Some(true) = matches.get_one::<bool>(SYNC) {
        println!("Synchronize all projects");
        manifest.sync(&vcs, &dwl_mode)?;
    }

    // Pin manifest
    if let Some(path) = matches.get_one::<String>(PIN) {
        println!("Pin manifest");
        let pinned = manifest.pin(&vcs, &xml_parser)?;
        save_file(path, pinned.get_file())?;
    }

    Ok(())
}
