pub mod printer_config;
pub mod printer_id;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use dashmap::DashMap;
use std::{
    collections::HashSet,
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};
use tokio::sync::RwLock;

use crate::{
    auth::bambu_auth::AuthDb,
    config::{printer_config::PrinterConfig, printer_id::PrinterId},
};

#[derive(Clone)]
pub struct AppConfig {
    auth_bambu: Arc<RwLock<AuthDb>>,
    logged_in: Arc<AtomicBool>,

    ids: Arc<RwLock<HashSet<PrinterId>>>,
    printers: Arc<DashMap<PrinterId, PrinterConfig>>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::empty()
    }
}

/// getters, setters
impl AppConfig {
    pub fn logged_in(&self) -> bool {
        self.logged_in.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn set_logged_in(&self, logged_in: bool) {
        self.logged_in
            .store(logged_in, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn auth(&self) -> &Arc<RwLock<AuthDb>> {
        &self.auth_bambu
    }

    pub async fn get_token_async(&self) -> Result<Option<crate::auth::bambu_auth::Token>> {
        {
            let token = self.auth_bambu.read().await.get_token_cached();
            if let Some(token) = token {
                return Ok(Some(token));
            }
        }

        self.auth_bambu.write().await.get_token()
    }
}

/// save, load
impl AppConfig {
    pub fn empty() -> Self {
        Self {
            auth_bambu: Arc::new(RwLock::new(AuthDb::empty())),
            logged_in: Arc::new(AtomicBool::new(false)),

            ids: Arc::new(RwLock::new(HashSet::new())),
            printers: Arc::new(DashMap::new()),
        }
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        unimplemented!()
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        unimplemented!()
    }
}

impl AppConfig {
    pub async fn add_printer(&self, config: PrinterConfig) -> Result<()> {
        let id = config.id();
        let mut ids = self.ids.write().await;
        if ids.contains(&id) {
            bail!("printer already exists");
        }
        ids.insert(id.clone());
        self.printers.insert(id, config);
        Ok(())
    }

    // pub fn printer_ids(&self) -> Vec<PrinterId> {
    //     self.ids.blocking_read().iter().cloned().collect()
    // }

    pub fn printer_ids(&self) -> Vec<PrinterId> {
        // self.config.printers.keys().cloned().collect()
        self.ids.blocking_read().iter().cloned().collect()
    }

    pub async fn printer_ids_async(&self) -> Vec<PrinterId> {
        self.ids.read().await.iter().cloned().collect()
    }

    pub fn printers(&self) -> Vec<PrinterConfig> {
        self.printers.iter().map(|v| v.value().clone()).collect()
    }

    pub fn get_printer(&self, serial: &PrinterId) -> Option<PrinterConfig> {
        self.printers.get(serial).map(|v| v.value().clone())
    }

    // pub fn printers(&self) -> Vec<Arc<RwLock<PrinterConfig>>> {
    //     self.printers.iter().map(|v| v.value().clone()).collect()
    // }

    // pub fn get_printer(&self, serial: &PrinterId) -> Option<Arc<RwLock<PrinterConfig>>> {
    //     self.printers.get(serial).map(|v| v.clone())
    // }
}
