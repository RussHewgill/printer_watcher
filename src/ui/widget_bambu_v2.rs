use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use egui::{Color32, Label, Layout, Pos2, Response, RichText, Sense, Vec2};

use super::{
    app::App,
    icons::{
        icon_menu_with_size, printer_state_icon, thumbnail_bed, thumbnail_chamber, thumbnail_nozzle,
    },
    ui_types::GridLocation,
};
use crate::{
    config::printer_config::{PrinterConfigBambu, PrinterType},
    status::{
        bambu_status::{AmsCurrentSlot, AmsSlot, AmsStatus},
        GenericPrinterState,
    },
};
