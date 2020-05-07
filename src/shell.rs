use std::env;
use std::process::Command;

use slog::{debug, error};

use anyhow::{Context, Result, bail};

use crate::restic::Restic;

/// Extends the Restic wrapper with a command for spawning shells with Restic configuration
impl<'a> Restic<'a> {
    pub fn shell(&self) -> Result<()> {
        let shell = match shell_command() {
            Some(shell) => shell,
            None => bail!("Could not determine shell")
        };
        debug!(self.logger(), "Spawning shell `{}`", shell);

        let mut command = Command::new(&shell);

        for (k, v) in self.env().iter() {
            command.env(k, v);
        }

        command.env("RESTIC_REPOSITORY", &self.profile().repository);

        // if set, RESTIC_PASSWORD is already in shared_env
        if let Some(password_file) = &self.profile().password_file {
            command.env("RESTIC_PASSWORD_FILE", password_file);
        } else if let Some(password_command) = &self.profile().password_command {
            command.env("RESTIC_PASSWORD_COMMAND", password_command);
        }

        command.current_dir(&self.profile().base_directory);

        let status = command.status().context("Could not start shell")?;
        if !status.success() {
            error!(self.logger(), "Shell failed"; "status" => %status, "shell" => %shell);
        }

        Ok(())
    }
}

#[cfg(unix)]
fn shell_command() -> Option<String> {
    env::var("SHELL").ok()
}

#[cfg(windows)]
fn shell_command() -> Option<String> {
    env::var("COMSPEC").ok()
}