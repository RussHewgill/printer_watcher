use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct FilamentProfile {
    r#type: String,
    name: String,
    from: String,
    #[serde(deserialize_with = "deserialize_bool_from_string")]
    instantiation: bool,
    inherits: Option<String>,

    #[serde(deserialize_with = "deserialize_single_element_array")]
    fan_cooling_layer_time: f32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    filament_max_volumetric_speed: f32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    filament_type: String,
    // cool_plate_temp: u32,
    // eng_plate_temp: u32,
    // hot_plate_temp: u32,
    // textured_plate_temp: u32,
    // cool_plate_temp_initial_layer: u32,
    // eng_plate_temp_initial_layer: u32,
    // hot_plate_temp_initial_layer: u32,
    // textured_plate_temp_initial_layer: u32,
    // overhang_fan_threshold: u32,
    // overhang_fan_speed: u32,
    // slow_down_for_layer_cooling: u32,
    // close_fan_the_first_x_layers: u32,
    // filament_start_gcode: String,
    // filament_end_gcode: String,
    // filament_flow_ratio: f32,
    // reduce_fan_stop_start_freq: u32,
    // fan_cooling_layer_time: f32,
    // filament_cost: f32,
    // filament_density: f32,
    // filament_retraction_speed: f32,
    // filament_deretraction_speed: f32,
    // filament_diameter: f32,
    // filament_max_volumetric_speed: f32,
    // // filament_minimal_purge_on_wipe_tower: Vec<String>,
    // filament_retraction_minimum_travel: Option<f32>,
    // filament_retract_before_wipe: Option<f32>,
    // filament_retract_when_changing_layer: Option<bool>,
    // filament_retraction_length: Option<f32>,
    // filament_z_hop: Option<f32>,
    // filament_z_hop_types: Vec<String>,
    // filament_retract_restart_extra: f32,
    // filament_settings_id: String,
    // filament_soluble: bool,
    // filament_type: String,
    // filament_vendor: String,
    // filament_wipe: Option<bool>,
    // filament_wipe_distance: Option<f32>,
    // bed_type: String,
    // nozzle_temperature_initial_layer: f32,
    // full_fan_speed_layer: u32,
    // fan_max_speed: f32,
    // fan_min_speed: f32,
    // slow_down_min_speed: f32,
    // slow_down_layer_time: f32,
    // nozzle_temperature: f32,
    // temperature_vitrification: f32,
}

// fn deserialize_single_element_array<'de, D, T>(deserializer: D) -> Result<T, D::Error>
// where
//     D: Deserializer<'de>,
//     T: Deserialize<'de>,
// {
//     let wrapper: Vec<T> = Deserialize::deserialize(deserializer)?;
//     wrapper
//         .into_iter()
//         .next()
//         .ok_or_else(|| serde::de::Error::custom("Expected a single-element array"))
// }

fn deserialize_single_element_array<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let wrapper: Vec<String> = Deserialize::deserialize(deserializer)?;
    let value_str = wrapper
        .into_iter()
        .next()
        .ok_or_else(|| serde::de::Error::custom("Expected a single-element array"))?;
    T::from_str(&value_str)
        .map_err(|e| serde::de::Error::custom(format!("Failed to parse value: {}", e)))
}

fn deserialize_bool_from_string<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match s.to_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(serde::de::Error::custom(format!(
            "Invalid boolean value: {}",
            s
        ))),
    }
}
