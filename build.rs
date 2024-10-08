// extern crate embed_resource;
extern crate winresource;

// use std::path::Path;

// use dotenvy::dotenv;
// use std::env;
use anyhow::Result;
use vergen::EmitBuilder;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=build.rs");

    EmitBuilder::builder()
        // .all_build()
        // .all_cargo()
        .all_git()
        .git_sha(true)
        // .all_rustc()
        // .all_sysinfo()
        .emit()?;

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winresource::WindowsResource::new();
        res
            // .set_icon("icon.ico");
            .set_icon_with_id("icon3.ico", "1")
            .compile()
            .unwrap();
    }

    // panic!()
    Ok(())
}
