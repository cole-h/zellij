//! Plugins configuration metadata
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

use serde::{Deserialize, Serialize};
use url::Url;

use super::layout::{RunPlugin, RunPluginLocation};
pub use crate::data::PluginTag;

use std::collections::BTreeMap;
use std::fmt;

/// Used in the config struct for plugin metadata
#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct PluginsConfig(pub HashMap<PluginTag, PluginConfig>);

impl fmt::Debug for PluginsConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut stable_sorted = BTreeMap::new();
        for (plugin_tag, plugin_config) in self.0.iter() {
            stable_sorted.insert(plugin_tag, plugin_config);
        }
        write!(f, "{:#?}", stable_sorted)
    }
}

impl PluginsConfig {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
    pub fn from_data(data: HashMap<PluginTag, PluginConfig>) -> Self {
        PluginsConfig(data)
    }

    /// Get plugin config from run configuration specified in layout files.
    pub fn get(&self, run: impl Borrow<RunPlugin>) -> Option<PluginConfig> {
        let run = run.borrow();
        match &run.location {
            RunPluginLocation::File(path) => Some(PluginConfig {
                path: path.clone(),
                run: PluginType::Pane(None),
                _allow_exec_host_cmd: run._allow_exec_host_cmd,
                location: run.location.clone(),
            }),
            RunPluginLocation::Zellij(tag) => self.0.get(tag).cloned().map(|plugin| PluginConfig {
                _allow_exec_host_cmd: run._allow_exec_host_cmd,
                ..plugin
            }),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &PluginConfig> {
        self.0.values()
    }

    /// Merges two PluginConfig structs into one PluginConfig struct
    /// `other` overrides the PluginConfig of `self`.
    pub fn merge(&self, other: Self) -> Self {
        let mut plugin_config = self.0.clone();
        plugin_config.extend(other.0);
        Self(plugin_config)
    }
}

impl Default for PluginsConfig {
    fn default() -> Self {
        PluginsConfig(HashMap::new())
    }
}

/// Plugin metadata
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct PluginConfig {
    /// Path of the plugin, see resolve_wasm_bytes for resolution semantics
    pub path: PathBuf,
    /// Plugin type
    pub run: PluginType,
    /// Allow command execution from plugin
    pub _allow_exec_host_cmd: bool,
    /// Original location of the
    pub location: RunPluginLocation,
}

impl PluginConfig {
    /// Resolve wasm plugin bytes for the plugin path and given plugin directory. Attempts to first
    /// resolve the plugin path as an absolute path, then adds a ".wasm" extension to the path and
    /// resolves that, finally we use the plugin directory joined with the path with an appended
    /// ".wasm" extension. So if our path is "tab-bar" and the given plugin dir is
    /// "/home/bob/.zellij/plugins" the lookup chain will be this:
    ///
    /// ```bash
    ///   /tab-bar
    ///   /tab-bar.wasm
    ///   /home/bob/.zellij/plugins/tab-bar.wasm
    /// ```
    ///
    pub fn resolve_wasm_bytes(&self, plugin_dir: &Path) -> Option<Vec<u8>> {
        fs::read(&self.path)
            .or_else(|_| fs::read(&self.path.with_extension("wasm")))
            .or_else(|_| fs::read(plugin_dir.join(&self.path).with_extension("wasm")))
            .ok()
    }

    /// Sets the tab index inside of the plugin type of the run field.
    pub fn set_tab_index(&mut self, tab_index: usize) {
        match self.run {
            PluginType::Pane(..) => {
                self.run = PluginType::Pane(Some(tab_index));
            },
            PluginType::Headless => {},
        }
    }
}

/// Type of the plugin. Defaults to Pane.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PluginType {
    // TODO: A plugin with output that's cloned across every pane in a tab, or across the entire
    // application might be useful
    // Tab
    // Static
    /// Starts immediately when Zellij is started and runs without a visible pane
    Headless,
    /// Runs once per pane declared inside a layout file
    Pane(Option<usize>), // tab_index
}

impl Default for PluginType {
    fn default() -> Self {
        Self::Pane(None)
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum PluginsConfigError {
    #[error("Duplication in plugin tag names is not allowed: '{}'", String::from(.0.clone()))]
    DuplicatePlugins(PluginTag),
    #[error("Only 'file:' and 'zellij:' url schemes are supported for plugin lookup. '{0}' does not match either.")]
    InvalidUrl(Url),
    #[error("Could not find plugin at the path: '{0:?}'")]
    InvalidPluginLocation(PathBuf),
}
