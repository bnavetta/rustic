use std::fs;
use std::path::{PathBuf, Path};
use std::sync::Mutex;

use anyhow::{Result, Context};
use human_panic;
use slog::{Logger, Drain, o, debug, error};
use slog_term;
use structopt::StructOpt;
use toml;

mod backup;
mod config;
mod restic;

use config::Configuration;
use restic::Restic;

// TODO: prometheus
// TODO: builtin systemd-inhibit and caffeinate support?
// TODO: nice/ionice support?

#[derive(Debug, StructOpt)]
struct Args {
    /// Path to the Rustic configuration file
    #[structopt(short = "c", long = "config", env = "RUSTIC_CONFIG", parse(from_os_str))]
    config_file: PathBuf,

    #[structopt(subcommand)]
    command: Command
}

#[derive(Debug, StructOpt)]
enum Command {
    /// Run a backup
    Backup {
        /// The profile to back up
        profile: String,
    }
}

fn load_config<P: AsRef<Path>>(logger: &Logger, path: P) -> Result<Configuration> {
    let path = path.as_ref();
    debug!(logger, "Loading configuration from {}", path.display());

    let config_str = fs::read_to_string(path)
        .with_context(|| format!("Could not read configuration file {}", path.display()))?;
    
    let config = toml::from_str(&config_str)
        .with_context(|| format!("Could not parse configuration file {}", path.display()))?;
    
    // TODO: OK to log config? not if it contains credentials
    Ok(config)
}

fn run(args: Args, logger: &Logger) -> Result<()> {
    let config = load_config(logger, &args.config_file)?;

    match args.command {
        Command::Backup { profile } => {
            let restic = Restic::for_profile(&config, logger, profile)?;
            restic.backup()?;
        }
    };

    Ok(())
}

#[paw::main]
fn main(args: Args) {
    human_panic::setup_panic!();

    let decorator = slog_term::TermDecorator::new().build();
    let term_drain = slog_term::FullFormat::new(decorator).use_local_timestamp().build().fuse();
    // Despite the slog docs, we're using a Mutex for the thread-safe drain rather than slog_async. Since this is a single-threaded program, there's
    // probably more overhead adding a thread for logging than letting the main thread use a mutex uncontested. This also means we can use the logger
    // below without having to worry about flushing it before calling std::process::exit
    let drain = Mutex::new(term_drain).fuse();
    let root = Logger::root(drain, o!("rustic_version" => env!("CARGO_PKG_VERSION")));

    if let Err(err) = run(args, &root) {
        error!(root, "Fatal error: {:?}", err);
        std::process::exit(1);
    }
}
