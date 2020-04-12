use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Configuration {
    /// Backup profiles
    pub profiles: HashMap<String, Profile>,

    /// Named filesets to reuse in profiles
    #[serde(default)]
    pub filesets: HashMap<String, Fileset>,

    /// Location of the `restic` binary. Defaults to `restic`
    #[serde(default = "default_restic_command")]
    pub restic_command: String,
}

fn default_restic_command() -> String {
    "restic".into()
}

#[derive(Deserialize)]
pub struct Profile {
    /// Repository URL
    pub repository: String,

    /// If true, automatically initialize the repository if it does not exist.
    #[serde(default)]
    pub auto_init: bool,

    /// Directory to run backups from. Included and excluded files, and the password and environment files (if specified), will be
    /// resolved relative to this directory.
    pub base_directory: PathBuf,

    /// Repository password. Prefer `password_file` or `password_command` instead, unless there are strict permissions on the
    /// configuration file.
    #[serde(default)]
    pub password: Option<String>,

    /// File containing the repository password. Exactly one of `password`, `password_file`, or `password_command` must be specified.
    #[serde(default)]
    pub password_file: Option<String>,

    /// Command to run to get the repository password. Exactly one of `password`, `password_file`, or `password_command` must be specified.
    #[serde(default)]
    pub password_command: Option<String>,

    /// Environment variables to pass to Restic. Can be used to set repository backend credentials (ex. Backblaze B2 API keys). These will be
    /// merged with variables in `environment_file`, if both are given.
    #[serde(default)]
    pub environment: HashMap<String, String>,

    /// TOML file containing a table of environment variables to pass to Restic. Can be used to set repository backend credentials. Variables in
    /// this file will be merged with `environment`, if both are given.
    #[serde(default)]
    pub environment_file: Option<PathBuf>,

    /// Files to back up
    pub include: Fileset,

    /// Files to exclude from the backup
    #[serde(default)]
    pub exclude: Fileset,

    /// Whether or not to exclude cache directories marked with a `CACHEDIR.TAG` file. See the
    /// [Cache Directory Tagging Standard](http://bford.info/cachedir/spec.html) for more.
    #[serde(default)]
    pub exclude_caches: bool,

    /// Do not cross filesystem boundaries when backing up
    #[serde(default)]
    pub one_file_system: bool,

    /// Ignore inode number changes when checking for modified files
    #[serde(default)]
    pub ignore_inode: bool,

    /// Policy for how long to keep backup snapshots
    #[serde(default)]
    pub retention: RetentionPolicy,
}

#[derive(Deserialize, Default)]
/// Specification of a set of files (to include or exclude)
pub struct Fileset {
    /// Names of other filesets to inherit from. Patterns from inherited filesets (including from filesets they inherit from)
    /// are added to the patterns in this one.
    #[serde(default)]
    pub inherits: Vec<String>,

    /// File patterns to include.
    ///
    /// See Restic's documentation on [including](https://restic.readthedocs.io/en/latest/040_backup.html#including-files) and
    /// [excluding](https://restic.readthedocs.io/en/latest/040_backup.html#excluding-files) files for details on how these are interpreted.
    #[serde(default)]
    pub patterns: Vec<String>,
}

/// Describes how to keep/forget snapshots.
///
/// See the [Restic documentation](https://restic.readthedocs.io/en/latest/060_forget.html#removing-snapshots-according-to-a-policy).
#[derive(Deserialize, Default)]
#[serde(default)]
pub struct RetentionPolicy {
    /// Keep the `n` most recent snapshots
    pub keep_last: Option<usize>,

    /// Keep the most recent snapshot for the last `n` hours that have one
    pub keep_hourly: Option<usize>,

    /// Keep the most recent snapshot for the last `n` days that have one
    pub keep_daily: Option<usize>,

    /// Keep the most recent snapshot for the last `n` weeks that have one
    pub keep_weekly: Option<usize>,

    /// Keep the most recent snapshot for the last `n` months that have one
    pub keep_monthly: Option<usize>,

    /// Keep the most recent snapshot for the last `n` years that have one
    pub keep_yearly: Option<usize>,

    /// Keep all snapshots made within some duration of the most recent snapshot. The duration should be formatted as a number of years, months, days,
    /// and hours, like `1y3m10d2h` for the past 1 year, 3 months, 10 days, and 2 hours.
    pub keep_within: Option<String>,

    /// Keep all snapshots with any of these tag lists. For example, if this is set to `[["tag1", "tag2"], ["tag3"]]`, Restic will keep snapshots
    /// that either have both `tag1` and `tag2` or have `tag3`.
    pub keep_tags: Vec<Vec<String>>, // TODO: restrict to tags + host
}

impl RetentionPolicy {
    /// Returns `true` if this policy is empty (i.e. it doesn't specify any snapshots to keep)
    pub fn is_empty(&self) -> bool {
        self.keep_last.is_none()
            && self.keep_hourly.is_none()
            && self.keep_daily.is_none()
            && self.keep_weekly.is_none()
            && self.keep_monthly.is_none()
            && self.keep_yearly.is_none()
            && self.keep_within.is_none()
            && self.keep_tags.is_empty()
    }
}
