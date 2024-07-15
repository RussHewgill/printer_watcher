use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use notify_rust::Notification;

pub fn alert_print_complete(name: &str, file: &str) {
    let _ = notify_rust::Notification::new()
        .summary(&format!("Print Complete on {}", name))
        .body(&format!("{}", file))
        .appname("Printer Watcher")
        .timeout(0)
        .show();
}

pub fn alert_printer_error(name: &str, error: &str) {
    let _ = notify_rust::Notification::new()
        .summary(&format!("Printer Error: {}", name))
        .body(&format!("Printer error: {:?}\n\nError: {:?}", name, error))
        .appname("Printer Watcher")
        .timeout(0)
        .show();
}
