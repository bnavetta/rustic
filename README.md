# Rustic

Rustic is a wrapper around the [Restic](https://restic.net/) backup program. It adds support for backup profiles in a configuration file,
to keep all your backup settings in one place.

## Installation

Install with Cargo:

```sh
cargo install rustic
```

## Usage

To back up the profile `my_profile`, run:

```sh
$ rustic backup my_profile
```

To forget snapshots from `my_profile` using its configured retention policy, run:

```sh
$ rustic forget my_profile
```

You can add the `--prune` flag to `rustic forget` to automatically delete data referenced by forgotten snapshots, or seperately run `restic prune my_profile`.

To see a list of snapshots, run:

```sh
$ rustic snapshots my_profile
```

`rustic snapshots` can pass arguments through to Restic to filter which snapshots are shown:

```sh
$ rustic snapshots my_profile -- --last
```

You can also run `rustic profiles` to print out a list of all profiles and their repository locations.

## Configuration

In order to use Rustic, you need to configure at least one profile telling it what to back up and how. Rustic gets the path to the configuration file
from either the `--config` flag or the `RUSTIC_CONFIG` environment variable.

The configuration file uses TOML and has these fields:

```toml
# Path to the restic command. Defaults to `restic` if unspecified.
restic_command = "path/to/restic"

# Defines a profile named `my_profile`
[profiles.my_profile]
# Repository location. See https://restic.readthedocs.io/en/stable/030_preparing_a_new_repo.html
# for examples and supported backends
repository = "local:my-test-repository"

# This is the directory to run backups from. Included and excluded files, as well as the `password_file` and
# `credentials_file` options, are interpreted relative to this. In most cases, it will be the root directory or your
# home directory.
base_directory = "/"

# If true, running `rustic backup` will create the repository if it doesn't already exist. Note that `rustic forget`, `rustic prune`, and
# `rustic snapshots` will not create the repository, as there isn't anything for them to do with a brand-new repository. Defaults to false.
auto_init = false

# Password for the Restic repository. Unless your Rustic configuration file is well-protected, it's recommended that you use `password_file` or
# `password_command` instead.
password = "not very secret"

# A text file containing the repository password. The path is interpreted relative to `base_directory`.
password_file = "my-password.txt"

# A shell command that prints out the repository password.
password_command = "password-helper restic"

# If true, ignore cache directories marked with a `CACHEDIR.TAG` file
# See http://bford.info/cachedir/spec.html
exclude_caches = false

# If true, do not cross filesystem boundaries when backing up
one_file_system = false

# If true, ignore inode number changes when checking for modified files
ignore_inode = false

# TOML file containing a map of environment variables to pass to Restic. This is merged with the `environment` table described below.
environment_file = "my-variables.txt"

# Optional map of environment variables to pass to Restic. This is generally for backend-specific credentials like AWS or Backblaze API keys,
# but can contain any variables.
[profiles.my_profile.environment]
B2_ACCOUNT_ID = "1234"
B2_ACCOUNT_KEY = "5678"

# The retention policy controls which snapshots to keep when running `rustic forget`. All fields are optional, but `rustic forget`
# will not do anything unless at least one is set.
[profiles.my_profile.retention]
# Keep the n most recent snapshots
keep_last = 5

# Keep the most recent n hourly snapshots (that is, for the past n hours that have snapshots, keep the most recent
# snapshot from that hour)
keep_hourly = 24

# Keep the most recent n daily snapshots
keep_daily = 7

# Keep the most recent n weekly snapshots
keep_weekly = 8

# Keep the most recent n monthly snapshots
keep_monthly = 12

# Keep the most recent n yearly snapshots
keep_yearly = 10

# Keep all snapshots within some duration of the most recent snapshot. The duration should be formatted as a
# number of years, months, days, and hours, like `1y3m10d2h` for the past 1 year, 3 months, 10 days, and 2 hours.
keep_within = "1y3m10d1h"

# Keep all snapshots with any of these combinations of tags. In this case, all snapshots with the `important` tag will be kept, and
# all snapshots with both `tag1` and `tag2`.
keep_tags = [
    ["important"],
    ["tag1", "tag2"]
]

# Fileset specifying which files to back up. See `filesets` below
[profiles.my_profile.include]
patterns = [
    "/etc",
    "/var",
    "/home/*/Documents"
]

# Fileset specifying which files to exclude from the backup
[profile.my_profile.exclude]
patterns = [
    "*.o",
    "/var/log"
]

inherits = ["base_excludes"]

# Filesets specify a set of files based on glob patterns. They can inherit the patterns from other filesets defined in the
# `filesets` table. Each backup profile has a fileset specifying which files to back up and (optionally) a fileset with patterns
# to exclude from the backup.
[filesets.base_excludes]
patterns = [
    "*.zip",
    "*.tar",
    "*.tar.gz"
]
```
