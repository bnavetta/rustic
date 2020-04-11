//! List backup snapshots

use std::ffi::OsString;

use anyhow::{Result, Context};
use slog::debug;

use crate::restic::Restic;

// Could parse and print JSON instead of passing through to Restic

/// Extends the Restic wrapper with snapshot commands
impl <'a> Restic<'a> {
    /// List snapshots to stdout. This is a simple wrapper around the `restic snapshots` command.
    /// Extra args are added directly to the command line.
    pub fn dump_snapshots(&self, extra_args: &[OsString]) -> Result<()> {
        let mut cmd = self.new_command();
        cmd.arg("snapshots");
        cmd.args(extra_args);

        debug!(self.logger(), "Listing snapshots"; "command" => ?cmd);
        cmd.status()
            .with_context(|| format!("Could not run {:?}", cmd))?;

        Ok(())
    }
}