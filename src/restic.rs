//! Shared functions for interacting with Restic (mostly generating command lines)
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};
use slog::{debug, o, Logger};
use toml;

use crate::config::{Configuration, Profile};

/// Wrapper around the Restic CLI
pub struct Restic<'a> {
    logger: Logger,
    config: &'a Configuration,
    profile: &'a Profile,
    shared_args: Vec<OsString>,
    shared_env: HashMap<OsString, OsString>,
}

impl<'a> Restic<'a> {
    /// Creates a new Restic wrapper for the specified profile. This performs some precomputation and validation of Restic
    /// flags, and will return an error if that validation fails (for example, if the profile does not exist or does not set
    /// a repository password).
    pub fn for_profile(
        config: &'a Configuration,
        logger: &Logger,
        profile_name: String,
    ) -> Result<Restic<'a>> {
        let profile = match config.profiles.get(&profile_name) {
            Some(profile) => profile,
            None => bail!("Profile `{}` does not exist", profile_name),
        };
        let logger = logger.new(o!("profile" => profile_name));

        let mut shared_args = Vec::new();
        let mut shared_env = HashMap::new();
        add_password(profile, &mut shared_args, &mut shared_env)?;
        add_credentials(profile, &mut shared_env)?;
        shared_args.push("--repo".into());
        shared_args.push(profile.repository.to_string().into());

        Ok(Restic {
            config,
            profile,
            logger,
            shared_args,
            shared_env,
        })
    }

    /// Starts building a Restic command line. The returned command has all shared
    /// environment variables and flags set (such as the repository and credentials), but
    /// no operation-specific flags.
    pub fn new_command(&self) -> Command {
        let mut cmd = Command::new(&self.config.restic_command);
        cmd.current_dir(&self.profile.base_directory)
            .args(&self.shared_args)
            .envs(&self.shared_env);
        cmd
    }

    /// Overall Rustic configuration this is derived from
    pub fn config(&self) -> &Configuration {
        self.config
    }

    /// Returns the profile defining this Restic repository
    pub fn profile(&self) -> &Profile {
        self.profile
    }

    /// Returns a logger scoped to this Restic repository
    pub fn logger(&self) -> &Logger {
        &self.logger
    }

    /// Checks if the repository already exists. This uses the method suggested [in the Restic docs](https://restic.readthedocs.io/en/latest/075_scripting.html),
    /// running `restic snapshots`.
    pub fn repository_exists(&self) -> Result<bool> {
        let mut cmd = self.new_command();
        cmd.arg("snapshots")
            .arg("--compact")
            .arg("--last")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let result = cmd
            .status()
            .with_context(|| format!("Could not run {:?}", cmd))?;
        if result.success() {
            debug!(&self.logger, "`restic snapshots` succeeded, repository exists"; "command" => ?cmd);
            Ok(true)
        } else {
            debug!(&self.logger, "`restic snapshots` failed, assuming repository does not exist"; "command" => ?cmd);
            Ok(false)
        }
    }
}

/// Adds the repository password to the command template.
fn add_password(
    profile: &Profile,
    args: &mut Vec<OsString>,
    env: &mut HashMap<OsString, OsString>,
) -> Result<()> {
    if let Some(password) = &profile.password {
        if profile.password_file.is_some() {
            bail!("Cannot set both `password` and `password_file`");
        }

        if profile.password_command.is_some() {
            bail!("Cannot set both `password` and `password_command`");
        }

        env.insert("RESTIC_PASSWORD".into(), password.into());
    } else if let Some(password_file) = &profile.password_file {
        if profile.password_command.is_some() {
            bail!("Cannot set both `password_file` and `password_command`");
        }

        args.push("--password-file".into());
        args.push(password_file.into());
    } else if let Some(password_command) = &profile.password_command {
        args.push("--password-command".into());
        args.push(password_command.into());
    } else {
        bail!("Must set one of `password`, `password_file`, or `password_command`");
    }

    Ok(())
}

/// Add credential environment variables to the command.
fn add_credentials(profile: &Profile, env: &mut HashMap<OsString, OsString>) -> Result<()> {
    for (var, value) in profile.environment.iter() {
        env.insert(var.into(), value.into());
    }

    if let Some(environment_file) = &profile.environment_file {
        // .join will resolve environment_file against base_directory if it's relative, but returns
        // environment_file itself if it's absolute.
        let environment_file = &profile.base_directory.join(environment_file);
        let env_contents = fs::read_to_string(environment_file).with_context(|| {
            format!(
                "Could not read environment file {}",
                environment_file.display()
            )
        })?;
        let env_vars: HashMap<OsString, OsString> =
            toml::from_str(&env_contents).with_context(|| {
                format!(
                    "Could not parse environment file {}",
                    environment_file.display()
                )
            })?;
        for (var, value) in env_vars {
            env.insert(var, value);
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::test::TestFixture;

    #[test]
    fn test_repository_exists() {
        let fixture = TestFixture::new();
        let restic = fixture.restic();

        assert_eq!(restic.repository_exists().unwrap(), false, "Repository does not exist yet");

        restic.init().expect("Could not initialize repository");
        assert_eq!(restic.repository_exists().unwrap(), true, "Repository should now exist");
    }
}