use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use sqlx::{Connection, SqliteConnection, SqlitePool};

pub struct ProfileDb {
    db: SqliteConnection,
}

impl ProfileDb {
    pub async fn new() -> Result<Self> {
        // let path = "profiles.db";
        // let path = "sqlite:profiles.db";
        let path = "profiles.db";

        let options = sqlx::sqlite::SqliteConnectOptions::new()
            // .max_connections(5)
            // .connect(path)
            .filename(path)
            .create_if_missing(true);

        debug!("connecting");
        // let conn = SqliteConnection::connect_with(&options).await?;
        let conn = SqlitePool::connect_with(options).await?;
        debug!("done");

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS filament_profiles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            from_source TEXT NOT NULL,
            instantiation BOOLEAN NOT NULL,
            inherits TEXT,
            cool_plate_temp INTEGER NOT NULL,
            eng_plate_temp INTEGER NOT NULL,
            hot_plate_temp INTEGER NOT NULL,
            textured_plate_temp INTEGER NOT NULL,
            cool_plate_temp_initial_layer INTEGER NOT NULL,
            eng_plate_temp_initial_layer INTEGER NOT NULL,
            hot_plate_temp_initial_layer INTEGER NOT NULL,
            textured_plate_temp_initial_layer INTEGER NOT NULL,
            overhang_fan_threshold TEXT NOT NULL,
            overhang_fan_speed INTEGER NOT NULL,
            slow_down_for_layer_cooling INTEGER NOT NULL,
            close_fan_the_first_x_layers INTEGER NOT NULL,
            filament_start_gcode TEXT NOT NULL,
            filament_end_gcode TEXT NOT NULL,
            filament_flow_ratio REAL NOT NULL,
            reduce_fan_stop_start_freq INTEGER NOT NULL,
            fan_cooling_layer_time REAL NOT NULL,
            filament_cost REAL NOT NULL,
            filament_density REAL NOT NULL,
            filament_diameter REAL NOT NULL,
            filament_max_volumetric_speed REAL NOT NULL,
            filament_settings_id TEXT NOT NULL,
            filament_soluble INTEGER NOT NULL,
            filament_type TEXT NOT NULL,
            filament_vendor TEXT NOT NULL,
            bed_type TEXT NOT NULL,
            nozzle_temperature_initial_layer REAL NOT NULL,
            full_fan_speed_layer INTEGER NOT NULL,
            fan_max_speed REAL NOT NULL,
            fan_min_speed REAL NOT NULL,
            slow_down_min_speed REAL NOT NULL,
            slow_down_layer_time REAL NOT NULL,
            nozzle_temperature REAL NOT NULL,
            temperature_vitrification REAL NOT NULL
        );
        "#,
        )
        .execute(&conn)
        .await?;

        unimplemented!()
    }
}
