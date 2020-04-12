use std::ffi::OsString;
use std::fs;
use std::io::{self, prelude::*};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};
use human_panic;
use slog::{debug, error, o, Drain, Level, LevelFilter, Logger};
use slog_term;
use structopt::StructOpt;
use tabwriter;
use toml;

mod backup;
mod config;
mod forget;
mod restic;
mod snapshots;

use config::Configuration;
use restic::Restic;

// TODO: prometheus
// TODO: builtin systemd-inhibit and caffeinate support?
// TODO: nice/ionice support?

#[derive(Debug, StructOpt)]
struct Args {
    /// Path to the Rustic configuration file
    #[structopt(
        short = "c",
        long = "config",
        env = "RUSTIC_CONFIG",
        parse(from_os_str)
    )]
    config_file: PathBuf,

    /// Adjust the verbosity of log output. By default, only print errors and warnings. Pass `-v` for informational messages or
    /// `-vv` for debug messages.
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: u8,

    #[structopt(subcommand)]
    command: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    /// Run a backup
    Backup {
        /// The profile to back up
        profile: String,
    },

    /// Forget snapshots according to the configured retention policy
    Forget {
        /// The profile to forget from
        profile: String,

        /// Automatically prune any forgotten snapshots
        #[structopt(short = "p", long = "prune")]
        prune: bool,
    },

    /// Prune unreferenced data in the repository
    Prune {
        /// Profile to prune from
        profile: String,
    },

    /// List snapshots in a repository
    Snapshots {
        /// Profile to list
        profile: String,

        /// Additional arguments to pass to `restic snapshots`
        #[structopt(parse(from_os_str))]
        extra_args: Vec<OsString>,
    },

    /// List all profiles
    Profiles,
}

fn load_config<P: AsRef<Path>>(logger: &Logger, path: P) -> Result<Configuration> {
    let path = path.as_ref();
    debug!(logger, "Loading configuration from {}", path.display());

    let config_str = fs::read_to_string(path)
        .with_context(|| format!("Could not read configuration file {}", path.display()))?;

    let config = toml::from_str(&config_str)
        .with_context(|| format!("Could not parse configuration file {}", path.display()))?;

    Ok(config)
}

fn list_profiles(config: &Configuration) -> Result<()> {
    let mut tw = tabwriter::TabWriter::new(io::stdout());
    writeln!(tw, "Profile\tRepository")?;
    writeln!(tw, "-------\t----------")?;
    for (name, profile) in config.profiles.iter() {
        writeln!(tw, "{}\t{}", name, profile.repository)?;
    }
    tw.flush()?;

    Ok(())
}

fn run(args: Args, logger: &Logger) -> Result<()> {
    let config = load_config(logger, &args.config_file)?;

    // TODO: pass verbosity flag along to restic
    match args.command {
        Command::Backup { profile } => {
            let restic = Restic::for_profile(&config, logger, profile)?;
            restic.backup()?;
        }
        Command::Forget { profile, prune } => {
            let restic = Restic::for_profile(&config, logger, profile)?;
            restic.forget(prune)?;
        }
        Command::Prune { profile } => {
            let restic = Restic::for_profile(&config, logger, profile)?;
            restic.prune()?;
        }
        Command::Snapshots {
            profile,
            extra_args,
        } => {
            let restic = Restic::for_profile(&config, logger, profile)?;
            restic.dump_snapshots(&extra_args)?;
        }
        Command::Profiles => {
            list_profiles(&config)?;
        }
    };

    Ok(())
}

#[paw::main]
fn main(args: Args) {
    human_panic::setup_panic!();

    // The clap-verbosity-flag crate adds a `-q` flag for setting the log level below the default. However, for a backup program, it seems
    // risky to allow hiding errors/warnings
    let slog_level = match args.verbose {
        0 => Level::Warning,
        1 => Level::Info,
        2 => Level::Debug,
        _ => Level::Trace,
    };

    let decorator = slog_term::TermDecorator::new().build();
    let term_drain = slog_term::FullFormat::new(decorator)
        .use_local_timestamp()
        .build()
        .fuse();
    // Despite the slog docs, we're using a Mutex for the thread-safe drain rather than slog_async. Since this is a single-threaded program, there's
    // probably more overhead adding a thread for logging than letting the main thread use a mutex uncontested. This also means we can use the logger
    // below without having to worry about flushing it before calling std::process::exit
    let drain = Mutex::new(term_drain);
    let filtered = LevelFilter::new(drain, slog_level).fuse();
    let root = Logger::root(filtered, o!("rustic_version" => env!("CARGO_PKG_VERSION")));

    if let Err(err) = run(args, &root) {
        error!(root, "Fatal error: {:?}", err);
        std::process::exit(1);
    }
}
