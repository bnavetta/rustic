use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use slog::{o, Drain, Logger};
use slog_term::{FullFormat, TermDecorator};
use tempfile::TempDir;

use crate::config::*;
use crate::restic::Restic;

pub const TEST_REPOSITORY_PASSWORD: &str = "test-password";

/// Helper for testing with Restic
pub struct TestFixture {
    root: TempDir,
    content_root: PathBuf,
    repository_path: PathBuf,
    config: Configuration,
    logger: Logger,
}

impl TestFixture {
    pub fn new() -> TestFixture {
        let root = TempDir::new().expect("Could not create temporary directory");
        let content_root = root.path().join("content");
        let repository_path = root.path().join("repository");
        fs::create_dir(&content_root).expect("Could not create content directory");

        let profile = Profile {
            repository: format!("local:{}", repository_path.display()),
            auto_init: false,
            base_directory: content_root.clone(),
            password: Some(TEST_REPOSITORY_PASSWORD.to_string()),
            password_file: None,
            password_command: None,
            environment: HashMap::new(),
            environment_file: None,
            include: Fileset::default(),
            exclude: Fileset::default(),
            exclude_caches: false,
            one_file_system: false,
            ignore_inode: false,
            retention: RetentionPolicy::default(),
        };

        let config = Configuration {
            restic_command: "restic".to_string(),
            profiles: {
                let mut profiles = HashMap::new();
                profiles.insert("test".into(), profile);
                profiles
            },
            cache_directory: None,
            filesets: HashMap::new(),
        };

        let decorator = TermDecorator::new().build();
        let drain = FullFormat::new(decorator).build();
        let drain = Mutex::new(drain).fuse();
        let logger = Logger::root(drain, o!("test_root" => root.path().display().to_string()));

        TestFixture {
            root,
            content_root,
            repository_path,
            config,
            logger,
        }
    }

    /// Directory containing content files for backing up
    pub fn content_root(&self) -> &Path {
        &self.content_root
    }

    /// Restic repository path
    pub fn repository_path(&self) -> &Path {
        &self.repository_path
    }

    pub fn profile(&self) -> &Profile {
        &self.config.profiles["test"]
    }

    pub fn profile_mut(&mut self) -> &mut Profile {
        self.config.profiles.get_mut("test").unwrap()
    }

    pub fn restic(&self) -> Restic {
        Restic::for_profile(&self.config, &self.logger, "test".to_string()).unwrap()
    }
}
