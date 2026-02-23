use std::path::{Path, PathBuf};
use std::fs;

use crate::error::{Result, StorageError};
use crate::db::Database;

/// Default directory name inside the user's home.
const DEFAULT_DIR_NAME: &str = ".pog";

/// Name of the file that stores the default client.
const DEFAULT_CLIENT_FILE: &str = "default_client";

/// Manages the POGDIR – the directory where pog stores findings and its DB.
///
/// With multi-client support the layout is:
/// ```text
/// ~/.pog/
/// ├── clients/
/// │   ├── acme-corp/
/// │   │   ├── pog.db
/// │   │   └── findings/
/// │   └── globex/
/// │       ├── pog.db
/// │       └── findings/
/// └── default_client          ← plain-text file with the active client name
/// ```
pub struct PogDir {
    root: PathBuf,
}

impl PogDir {
    /// Legacy init – kept **only** for tests and backward compat scripts.
    /// Prefer `init_for_client` in production code.
    pub fn init() -> Result<Self> {
        let base = Self::resolve_root()?;
        let client = Self::read_default_client(&base)?;
        Self::init_for_client(&client)
    }

    /// Initialise a POGDIR scoped to a specific client.
    pub fn init_for_client(client: &str) -> Result<Self> {
        let base = Self::resolve_root()?;
        let root = base.join("clients").join(client);
        fs::create_dir_all(root.join("findings"))?;
        Ok(Self { root })
    }

    /// Create a `PogDir` rooted at an explicit path (useful for tests).
    pub fn init_at(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(root.join("findings"))?;
        Ok(Self { root })
    }

    // ------------------------------------------------------------------
    // Client management helpers
    // ------------------------------------------------------------------

    /// Resolve which client to use.
    ///
    /// Priority: explicit `--client` flag → `default_client` file → error.
    pub fn resolve_client(explicit: Option<&str>) -> Result<String> {
        if let Some(name) = explicit {
            return Ok(name.to_string());
        }
        let base = Self::resolve_root()?;
        Self::read_default_client(&base)
    }

    /// List all client names (sub-directories of `clients/`).
    pub fn list_clients() -> Result<Vec<String>> {
        let base = Self::resolve_root()?;
        let clients_dir = base.join("clients");
        if !clients_dir.exists() {
            return Ok(Vec::new());
        }
        let mut names: Vec<String> = fs::read_dir(&clients_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().to_str().map(String::from))
            .collect();
        names.sort();
        Ok(names)
    }

    /// Create a new client directory (does **not** change the default).
    pub fn create_client(name: &str) -> Result<()> {
        let base = Self::resolve_root()?;
        let dir = base.join("clients").join(name).join("findings");
        fs::create_dir_all(&dir)?;
        Ok(())
    }

    /// Delete a client and all its data.
    pub fn delete_client(name: &str) -> Result<()> {
        let base = Self::resolve_root()?;
        let dir = base.join("clients").join(name);
        if !dir.exists() {
            return Err(StorageError::ClientNotFound(name.to_string()));
        }
        fs::remove_dir_all(&dir)?;

        // If this was the default client, clear the default.
        if let Ok(current) = Self::read_default_client(&base) {
            if current == name {
                let _ = fs::remove_file(base.join(DEFAULT_CLIENT_FILE));
            }
        }
        Ok(())
    }

    /// Set (or change) the default client.
    pub fn set_default_client(name: &str) -> Result<()> {
        let base = Self::resolve_root()?;
        let dir = base.join("clients").join(name);
        if !dir.exists() {
            return Err(StorageError::ClientNotFound(name.to_string()));
        }
        fs::write(base.join(DEFAULT_CLIENT_FILE), name)?;
        Ok(())
    }

    /// Return the name of the current default client, if any.
    pub fn get_default_client() -> Result<String> {
        let base = Self::resolve_root()?;
        Self::read_default_client(&base)
    }

    // ------------------------------------------------------------------
    // Path helpers (unchanged public API)
    // ------------------------------------------------------------------

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

    /// Wipe the entire client directory and recreate the empty structure.
    pub fn clean(&self) -> Result<()> {
        if self.root.exists() {
            fs::remove_dir_all(&self.root)?;
        }
        fs::create_dir_all(self.root.join("findings"))?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Private helpers
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

    /// Read the `default_client` file inside the base POGDIR.
    fn read_default_client(base: &Path) -> Result<String> {
        let path = base.join(DEFAULT_CLIENT_FILE);
        fs::read_to_string(&path)
            .map(|s| s.trim().to_string())
            .map_err(|_| StorageError::NoClientSelected)
    }

}
