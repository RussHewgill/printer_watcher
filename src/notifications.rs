use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use notify_rust::Notification;

use crate::{config::printer_id::PrinterId, error_logging::error_db::ErrorDb};

pub async fn alert_print_complete(
    error_db: &ErrorDb,
    printer_id: &PrinterId,
    name: &str,
    file: &str,
) {
    error_db
        .insert(printer_id.inner(), &format!("Print complete: {}", file))
        .await
        .unwrap();

    let _ = notify_rust::Notification::new()
        .summary(&format!("Print Complete on {}", name))
        .body(&format!("{}", file))
        .appname("Printer Watcher")
        .timeout(0)
        .show();
}

pub async fn alert_printer_error(
    error_db: &ErrorDb,
    printer_id: &PrinterId,
    name: &str,
    error: &str,
) {
    error_db
        .insert(printer_id.inner(), &format!("error: {}", error))
        .await
        .unwrap();

    let _ = notify_rust::Notification::new()
        .summary(&format!("Printer Error: {}", name))
        .body(&format!("Printer error: {:?}\n\nError: {:?}", name, error))
        .appname("Printer Watcher")
        .timeout(0)
        .show();
}
