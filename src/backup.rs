//! Implementation for Restic backups.

use std::collections::HashMap;
use std::io::prelude::*;
use std::time::Instant;

use anyhow::{Result, Context, bail, anyhow};
use slog::{info, debug, error};
use tempfile::NamedTempFile;

use crate::config::Fileset;
use crate::restic::Restic;

/// Writes all patterns specified by a fileset and any filesets it inherits from to some stream, such as an include or exclude file.
fn write_fileset<W: Write>(out: &mut W, fileset: &Fileset, named_filesets: &HashMap<String, Fileset>) -> Result<()> {
    for pattern in fileset.patterns.iter() {
        writeln!(out, "{}", pattern).context("Could not write fileset")?;
    }

    for inherited in fileset.inherits.iter() {
        match named_filesets.get(inherited) {
            Some(fs) => write_fileset(out, fs, named_filesets).context("Could not write fileset")?,
            None => bail!("Fileset {} does not exist", inherited)
        }
    }

    Ok(())
}

// This uses an impl block in a separate file so it has access to all the repo info but keeps backup-specific Restic details
// nicely contained

/// Extends the Restic wrapper with backup commands.
impl <'a> Restic<'a> {
    /// Attempts to initialize the Restic repository. Note that this *does not* check if the repository has already been initialized.
    pub fn init(&self) -> Result<()> {
        let mut cmd = self.new_command();
        cmd.arg("init");
        debug!(self.logger(), "Initializing Restic repository"; "command" => ?cmd);

        let status = cmd.status()
            .with_context(|| format!("Could not run {:?}", cmd))?;
        
        if status.success() {
            debug!(self.logger(), "Restic repository initialized");
            Ok(())
        } else {
            bail!("Initializing the Restic repository failed with {}", status);
        }
    }

    /// Runs a backup. If the repository does not exist and `auto_init` is set in the profile, it will be initialized first.
    pub fn backup(&self) -> Result<()> {
        if !self.repository_exists()? {
            if self.profile().auto_init {
                self.init()?;
            } else {
                bail!("Repository not initialized");
            }
        }

        let mut include_file = NamedTempFile::new().context("Could not create temporary includes file")?;
        debug!(self.logger(), "Creating includes file"; "path" => %include_file.path().display());
        write_fileset(include_file.as_file_mut(), &self.profile().include, &self.config().filesets).context("Could not generate includes file")?;
    
        let mut exclude_file = NamedTempFile::new().context("Could not create temporary excludes file")?;
        debug!(self.logger(), "Creating excludes file"; "path" => %exclude_file.path().display());
        write_fileset(exclude_file.as_file_mut(), &self.profile().exclude, &self.config().filesets).context("Could not generate excludes file")?;

        let mut cmd = self.new_command();
        cmd
            .arg("backup")
            // Keeping these owned and using .path() instead of .into_temp_path() makes sure the files get deleted
            .arg("--files-from").arg(include_file.path())
            .arg("--exclude-file").arg(exclude_file.path());

        if self.profile().exclude_caches {
            cmd.arg("--exclude-caches");
        }

        if self.profile().one_file_system {
            cmd.arg("--one-file-system");
        }

        if self.profile().ignore_inode {
            cmd.arg("--ignore-inode");
        }

        info!(self.logger(), "Beginning backup"; "command" => ?cmd);
        let start = Instant::now();
        let status = cmd.status()
            .with_context(|| format!("Could not run {:?}", cmd))?;
        let duration = Instant::now() - start;

        if status.success() {
            info!(self.logger(), "Backup finished successfully in {:?}", duration; "command" => ?cmd);
            Ok(())
        } else {
            error!(self.logger(), "Backup failed"; "status" => %status, "command" => ?cmd);
            Err(anyhow!("Restic backup failed with {}", status))
        }
    }
}
