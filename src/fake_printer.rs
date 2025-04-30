use std::sync::Arc;

use dashmap::DashMap;

use crate::{
    config::{printer_id::PrinterId, AppConfig},
    status::{bambu_status::AmsUnit, GenericPrinterState, PrinterState},
};

#[cfg(feature = "nope")]
pub fn fake_printer(
    config: &AppConfig,
    printer_states: Arc<DashMap<PrinterId, GenericPrinterState>>,
) {
    let fake_id = PrinterId::from_id("FAKE");

    let fake_config = crate::config::printer_config::PrinterConfig::Bambu(
        fake_id.clone(),
        Arc::new(tokio::sync::RwLock::new(
            crate::config::printer_config::PrinterConfigBambu::from_id(
                "Fake".to_string(),
                "Fake".to_string(),
                "Fake".to_string(),
                "Fake".to_string(),
                fake_id.clone(),
            ),
        )),
    );

    let mut fake_printer = GenericPrinterState::default();

    fake_printer.state = PrinterState::Idle;

    let mut st = crate::status::bambu_status::PrinterStateBambu::default();

    st.printer_type = Some(crate::status::bambu_status::BambuPrinterType::H2D);

    st.ams_status = Some(768);
    st.ams = Some(AmsStatus {
        units: vec![(0, AmsUnit::default())].into_iter().collect(),
        current_tray: Some(crate::status::bambu_status::AmsCurrentSlot::Tray {
            ams_id: 63,
            tray_id: 3,
        }),
        ams_exist_bits: Some("1".to_string()),
        tray_exist_bits: Some("f".to_string()),
        tray_now: Some("255".to_string()),
        tray_pre: Some("255".to_string()),
        tray_tar: Some("255".to_string()),
        version: Some(3),
        state: Some(crate::status::bambu_status::AmsState::Idle),
        humidity: Some("5".to_string()),
    });

    fake_printer.state_bambu = Some(st);

    config.add_printer_blocking(fake_config).unwrap();
    printer_states.insert(fake_id, fake_printer);
}
