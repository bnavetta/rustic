//! Forgetting and pruning snapshots

use std::process::Command;
use std::time::Instant;

use anyhow::{Result, Context, anyhow};
use itertools::join;
use slog::{warn, info, error};

use crate::config::RetentionPolicy;
use crate::restic::Restic;

impl <'a> Restic<'a> {
    /// Forgets snapshots according to the configured retention policy.
    pub fn forget(&self, prune: bool) -> Result<()> {
        let policy = &self.profile().retention;
        if policy.is_empty() {
            warn!(self.logger(), "Retention policy is empty, not forgetting any snapshots");
            return Ok(())
        }

        // TODO: check if repository exists and soft-fail or init?

        let mut cmd = self.new_command();
        cmd.arg("forget");
        add_policy(policy, &mut cmd);

        if prune {
            cmd.arg("--prune");
        }

        info!(self.logger(), "Forgetting snapshots"; "prune" => prune, "command" => ?cmd);
        let start = Instant::now();
        let status = cmd.status()
            .with_context(|| format!("Could not run {:?}", cmd))?;
        let duration = Instant::now() - start;

        if status.success() {
            info!(self.logger(), "Forgot snapshots in {:?}", duration; "command" => ?cmd);
            Ok(())
        } else {
            error!(self.logger(), "Forgetting snapshots failed"; "status" => %status, "command" => ?cmd);
            Err(anyhow!("Restic forget failed with {}", status))
        }
    }

    /// Prunes any unreferenced data in the repository (ex. from forgotten snapshots)
    pub fn prune(&self) -> Result<()> {
        // TODO: check if repository exists and soft-fail or init?

        let mut cmd = self.new_command();
        cmd.arg("prune");

        info!(self.logger(), "Pruning repository"; "command" => ?cmd);
        let start = Instant::now();
        let status = cmd.status()
            .with_context(|| format!("Could not run {:?}", cmd))?;
        let duration = Instant::now() - start;

        if status.success() {
            info!(self.logger(), "Pruned repository in {:?}", duration; "command" => ?cmd);
            Ok(())
        } else {
            error!(self.logger(), "Pruning repository failed"; "status" => %status, "command" => ?cmd);
            Err(anyhow!("Restic prune failed with {}", status))
        }
    }
}

fn add_policy(policy: &RetentionPolicy, cmd: &mut Command) {
    if let Some(keep_last) = policy.keep_last {
        cmd.arg("--keep-last").arg(keep_last.to_string());
    }

    if let Some(keep_hourly) = policy.keep_hourly {
        cmd.arg("--keep-hourly").arg(keep_hourly.to_string());
    }

    if let Some(keep_daily) = policy.keep_daily {
        cmd.arg("--keep-daily").arg(keep_daily.to_string());
    }

    if let Some(keep_weekly) = policy.keep_weekly {
        cmd.arg("--keep-weekly").arg(keep_weekly.to_string());
    }

    if let Some(keep_monthly) = policy.keep_monthly {
        cmd.arg("--keep-monthly").arg(keep_monthly.to_string());
    }

    if let Some(keep_yearly) = policy.keep_yearly {
        cmd.arg("--keep-yearly").arg(keep_yearly.to_string());
    }

    if let Some(keep_within) = &policy.keep_within {
        cmd.arg("--keep-within").arg(keep_within);
    }

    for taglist in policy.keep_tags.iter() {
        cmd.arg("--keep-tag").arg(join(taglist, ","));
    }
}