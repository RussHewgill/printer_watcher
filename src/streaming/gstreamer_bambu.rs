use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use gst::prelude::*;
use gstreamer::{self as gst, glib::FlagsClass};
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;

pub fn test_gstreamer() -> Result<()> {
    // tutorial_main().unwrap();

    Ok(())
}
