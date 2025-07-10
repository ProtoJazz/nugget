use crate::types::VariableConfig;
use serde_json::{Value, json};
use std::collections::HashMap;
use uuid::Uuid;

pub fn validate_variable_parameters(var_config: &VariableConfig) {
    let var_type = var_config.var_type.as_str();

    match var_type {
        "uuid" => {
            if var_config.prefix.is_some() {
                println!(
                    "Warning: UUID type doesn't support 'prefix' parameter. Ignoring this parameter."
                );
            }
            if var_config.min.is_some() {
                println!(
                    "Warning: UUID type doesn't support 'min' parameter. Ignoring this parameter."
                );
            }
            if var_config.max.is_some() {
                println!(
                    "Warning: UUID type doesn't support 'max' parameter. Ignoring this parameter."
                );
            }
        }
        "integer" => {
            if var_config.prefix.is_some() {
                println!(
                    "Warning: Integer type doesn't support 'prefix' parameter. Ignoring this parameter."
                );
            }
        }
        "string" => {
            if var_config.min.is_some() {
                println!(
                    "Warning: String type doesn't support 'min' parameter. Ignoring this parameter."
                );
            }
            if var_config.max.is_some() {
                println!(
                    "Warning: String type doesn't support 'max' parameter. Ignoring this parameter."
                );
            }
        }
        _ => {
            // Unknown type, warn about any parameters
            if var_config.prefix.is_some() || var_config.min.is_some() || var_config.max.is_some() {
                println!("Warning: Unknown variable type '{var_type}'. Parameters may not be supported.");
            }
        }
    }
}

pub fn generate_variable_value(var_config: &VariableConfig) -> Value {
    validate_variable_parameters(var_config);

    match var_config.var_type.as_str() {
        "uuid" => {
            json!(Uuid::new_v4().to_string())
        }
        "integer" => {
            let min = var_config.min.unwrap_or(0);
            let max = var_config.max.unwrap_or(i64::MAX);

            if min > max {
                println!("Warning: min value ({min}) is greater than max value ({max}). Using default range.");
                json!(rand::random::<u32>())
            } else {
                let range = (max - min) as u64;
                if range == 0 {
                    json!(min)
                } else {
                    let random_val = (rand::random::<u64>() % range) as i64 + min;
                    json!(random_val)
                }
            }
        }
        "string" => {
            let base_string = format!("generated_{}", rand::random::<u16>());
            if let Some(prefix) = &var_config.prefix {
                json!(format!("{}{}", prefix, base_string))
            } else {
                json!(base_string)
            }
        }
        _ => var_config.default.clone().unwrap_or(json!("default")),
    }
}

pub fn replace_variables_in_value(value: &Value, variables: &HashMap<String, Value>) -> Value {
    crate::interpolation::replace_simple_placeholders(value, |placeholder| {
        variables.get(placeholder).cloned()
    })
}
