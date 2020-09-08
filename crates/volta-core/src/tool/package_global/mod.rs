use std::fmt::{self, Display};
use std::path::PathBuf;

use super::Tool;
use crate::error::{Context, ErrorKind, Fallible};
use crate::fs::{create_staging_dir, remove_dir_if_exists, rename};
use crate::layout::volta_home;
use crate::session::Session;
use crate::style::tool_version;
use crate::sync::VoltaLock;
use crate::version::VersionSpec;
use fs_utils::ensure_containing_dir_exists;
use tempfile::TempDir;

mod install;

/// The Tool implementation for installing 3rd-party global packages
pub struct Package {
    name: String,
    version: VersionSpec,
    staging: TempDir,
}

impl Package {
    pub fn new(name: String, version: VersionSpec) -> Fallible<Self> {
        let staging = create_staging_dir()?;
        Ok(Package {
            name,
            version,
            staging,
        })
    }

    pub fn persist_install(self) -> Fallible<()> {
        let home = volta_home()?;
        let package_dir = new_package_image_dir(home, &self.name);

        remove_dir_if_exists(&package_dir)?;

        // Handle scoped packages (@vue/cli), which have an extra directory for the scope
        ensure_containing_dir_exists(&package_dir).with_context(|| {
            ErrorKind::ContainingDirError {
                path: package_dir.to_owned(),
            }
        })?;

        rename(self.staging.path(), &package_dir).with_context(|| {
            ErrorKind::SetupToolImageError {
                tool: self.name,
                version: self.version.to_string(),
                dir: package_dir,
            }
        })?;

        Ok(())
    }
}

impl Tool for Package {
    fn fetch(self: Box<Self>, _session: &mut Session) -> Fallible<()> {
        Err(ErrorKind::CannotFetchPackage {
            package: self.to_string(),
        }
        .into())
    }

    fn install(self: Box<Self>, session: &mut Session) -> Fallible<()> {
        let _lock = VoltaLock::acquire()?;
        self.global_install(session)?;

        // TODO: Parse package.json for version / bins
        // TODO: Write package config, bin configs, and shims

        self.persist_install()
    }

    fn pin(self: Box<Self>, _session: &mut Session) -> Fallible<()> {
        Err(ErrorKind::CannotPinPackage { package: self.name }.into())
    }
}

impl Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.version {
            VersionSpec::None => f.write_str(&self.name),
            _ => f.write_str(&tool_version(&self.name, &self.version)),
        }
    }
}

fn new_package_image_dir(home: &volta_layout::v2::VoltaHome, package_name: &str) -> PathBuf {
    // TODO: An updated layout (and associated migration) will be added in a follow-up PR
    // at which point this function can be removed
    home.package_image_root_dir().join(package_name)
}