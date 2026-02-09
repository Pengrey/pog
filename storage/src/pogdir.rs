use std::path::{Path, PathBuf};
use std::fs;

use crate::error::{Result, StorageError};
use crate::db::Database;

/// Default directory name inside the user's home.
const DEFAULT_DIR_NAME: &str = ".pog";

/// Manages the POGDIR – the directory where pog stores findings and its DB.
pub struct PogDir {
    root: PathBuf,
}

impl PogDir {
    /// Resolve the POGDIR.
    ///
    /// Priority:
    /// 1. `$POGDIR` environment variable
    /// 2. `$HOME/.pog`
    ///
    /// The directory (and sub-directories) are created if they don't exist.
    pub fn init() -> Result<Self> {
        let root = Self::resolve_root()?;
        fs::create_dir_all(root.join("findings"))?;
        Ok(Self { root })
    }

    /// Create a `PogDir` rooted at an explicit path (useful for tests).
    pub fn init_at(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(root.join("findings"))?;
        Ok(Self { root })
    }

    /// Open (or create) the SQLite database inside the POGDIR.
    pub fn open_db(&self) -> Result<Database> {
        Database::open(self.db_path())
    }

    /// Path to the SQLite file.
    pub fn db_path(&self) -> PathBuf {
        self.root.join("pog.db")
    }

    /// Root of the POGDIR.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Directory where individual finding folders are stored.
    pub fn findings_dir(&self) -> PathBuf {
        self.root.join("findings")
    }

    /// Return the directory for a given asset inside POGDIR.
    pub fn asset_dir(&self, asset: &str) -> PathBuf {
        self.findings_dir().join(asset)
    }

    /// Return the destination folder for a finding inside POGDIR.
    ///
    /// Layout: `findings/<asset>/<hex_id>_<slug>/`
    pub fn finding_dir(&self, asset: &str, hex_id: &str, slug: &str) -> PathBuf {
        self.asset_dir(asset).join(format!("{hex_id}_{slug}"))
    }

    /// Remove the database file and the entire `findings/` directory,
    /// then recreate the empty structure.
    pub fn clean(&self) -> Result<()> {
        let db_path = self.db_path();
        if db_path.exists() {
            fs::remove_file(&db_path)?;
        }
        let findings = self.findings_dir();
        if findings.exists() {
            fs::remove_dir_all(&findings)?;
        }
        fs::create_dir_all(&findings)?;
        Ok(())
    }

    // ------------------------------------------------------------------

    fn resolve_root() -> Result<PathBuf> {
        if let Ok(dir) = std::env::var("POGDIR") {
            return Ok(PathBuf::from(dir));
        }

        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| StorageError::NoPogDir)?;

        Ok(home.join(DEFAULT_DIR_NAME))
    }
}
