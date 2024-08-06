pub mod profiles_db;

use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FilamentProfile {
    r#type: String,
    name: String,
    from: String,
    #[serde(deserialize_with = "deserialize_bool_from_string")]
    instantiation: bool,
    inherits: Option<String>,

    #[serde(deserialize_with = "deserialize_single_element_array")]
    cool_plate_temp: u32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    eng_plate_temp: u32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    hot_plate_temp: u32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    textured_plate_temp: u32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    cool_plate_temp_initial_layer: u32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    eng_plate_temp_initial_layer: u32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    hot_plate_temp_initial_layer: u32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    textured_plate_temp_initial_layer: u32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    // overhang_fan_threshold: u32,
    overhang_fan_threshold: String,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    overhang_fan_speed: u32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    slow_down_for_layer_cooling: u32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    close_fan_the_first_x_layers: u32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    filament_start_gcode: String,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    filament_end_gcode: String,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    filament_flow_ratio: f32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    reduce_fan_stop_start_freq: u32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    fan_cooling_layer_time: f32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    filament_cost: f32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    filament_density: f32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    filament_diameter: f32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    filament_max_volumetric_speed: f32,
    // #[serde(deserialize_with = "deserialize_single_element_array")]
    // filament_retraction_speed: f32,
    // #[serde(deserialize_with = "deserialize_single_element_array")]
    // filament_deretraction_speed: f32,
    // filament_minimal_purge_on_wipe_tower: Vec<String>,
    // #[serde(deserialize_with = "deserialize_option_from_string_or_nil")]
    // filament_retraction_minimum_travel: Option<f32>,
    // #[serde(deserialize_with = "deserialize_option_from_string_or_nil")]
    // filament_retract_before_wipe: Option<f32>,
    // #[serde(deserialize_with = "deserialize_option_from_string_or_nil")]
    // filament_retract_when_changing_layer: Option<bool>,
    // #[serde(deserialize_with = "deserialize_optional_single_element_array")]
    // filament_retraction_length: Option<f32>,
    // #[serde(deserialize_with = "deserialize_optional_single_element_array")]
    // filament_z_hop: Option<f32>,
    // #[serde(deserialize_with = "deserialize_single_element_array")]
    // filament_z_hop_types: String,
    // #[serde(deserialize_with = "deserialize_single_element_array")]
    // filament_retract_restart_extra: f32,
    // #[serde(deserialize_with = "deserialize_single_element_array")]
    // filament_wipe: Option<bool>,
    // #[serde(deserialize_with = "deserialize_single_element_array")]
    // filament_wipe_distance: Option<f32>,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    filament_settings_id: String,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    filament_soluble: u32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    filament_type: String,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    filament_vendor: String,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    bed_type: String,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    nozzle_temperature_initial_layer: f32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    full_fan_speed_layer: u32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    fan_max_speed: f32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    fan_min_speed: f32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    slow_down_min_speed: f32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    slow_down_layer_time: f32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    nozzle_temperature: f32,
    #[serde(deserialize_with = "deserialize_single_element_array")]
    temperature_vitrification: f32,
}

trait FromStringOrNil: Sized {
    fn from_string_or_nil(s: &str) -> Result<Self, String>;
}

impl<T: FromStr> FromStringOrNil for T {
    fn from_string_or_nil(s: &str) -> Result<Self, String> {
        s.parse().map_err(|_| format!("Failed to parse '{}'", s))
    }
}

fn deserialize_from_string_or_nil<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStringOrNil,
{
    let wrapper: Vec<String> = Deserialize::deserialize(deserializer)?;
    if wrapper.is_empty() {
        return Err(serde::de::Error::custom("Expected a non-empty array"));
    }

    T::from_string_or_nil(&wrapper[0]).map_err(serde::de::Error::custom)
}

fn deserialize_option_from_string_or_nil<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: std::fmt::Display,
{
    let wrapper: Vec<String> = Deserialize::deserialize(deserializer)?;
    if wrapper.is_empty() {
        return Ok(None);
    }

    match wrapper[0].as_str() {
        "nil" => Ok(None),
        s => T::from_str(s)
            .map(Some)
            .map_err(|e| serde::de::Error::custom(format!("Failed to parse '{}': {}", s, e))),
    }
}

// fn deserialize_from_string_or_nil<'de, D, T>(deserializer: D) -> Result<T, D::Error>
// where
//     D: Deserializer<'de>,
//     T: FromStringOrNil,
// {
//     let wrapper: Vec<String> = Deserialize::deserialize(deserializer)?;
//     if wrapper.is_empty() {
//         return Err(serde::de::Error::custom("Expected a non-empty array"));
//     }

//     T::from_string_or_nil(&wrapper[0]).map_err(serde::de::Error::custom)
// }

fn deserialize_optional_single_element_array<'de, D, T>(
    deserializer: D,
) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let wrapper: Vec<String> = Deserialize::deserialize(deserializer)?;
    if wrapper.is_empty() {
        return Ok(None);
    }

    match wrapper[0].as_str() {
        "nil" => Ok(None),
        value => T::from_str(value)
            .map(Some)
            .map_err(|e| serde::de::Error::custom(format!("Failed to parse '{}': {}", value, e))),
    }
}

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
