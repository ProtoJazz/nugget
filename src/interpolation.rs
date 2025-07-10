use serde_json::{Value, json};
use std::collections::HashMap;

pub fn interpolate_payload(
    template: &Value,
    payload: &Value,
    defaults: &Option<HashMap<String, Value>>,
) -> Value {
    replace_simple_placeholders(template, |placeholder| {
        if let Some(field_name) = placeholder.strip_prefix("payload.") {
            if let Some(payload_obj) = payload.as_object() {
                if let Some(value) = payload_obj.get(field_name) {
                    return Some(value.clone());
                }
            }

            if let Some(defaults) = defaults {
                if let Some(default_value) = defaults.get(field_name) {
                    return Some(default_value.clone());
                }
            }
        }

        None
    })
}

pub fn extract_path_parameters(pattern: &str, path: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    let pattern_parts: Vec<&str> = pattern.split('/').collect();
    let path_parts: Vec<&str> = path.split('/').collect();

    if pattern_parts.len() != path_parts.len() {
        return params;
    }

    for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
        if pattern_part.starts_with('{') && pattern_part.ends_with('}') {
            let param_name = &pattern_part[1..pattern_part.len() - 1];
            params.insert(param_name.to_string(), path_part.to_string());
        }
    }

    params
}

pub fn replace_simple_placeholders<F>(value: &Value, resolver: F) -> Value
where
    F: Fn(&str) -> Option<Value> + Copy,
{
    match value {
        Value::String(s) => {
            if s.starts_with('{') && s.ends_with('}') {
                let placeholder_content = &s[1..s.len() - 1];
                if let Some(replacement) = resolver(placeholder_content) {
                    return replacement;
                }
            }

            let mut result = s.clone();
            let mut start = 0;

            while let Some(open_pos) = result[start..].find('{') {
                let open_pos = start + open_pos;
                if let Some(close_pos) = result[open_pos..].find('}') {
                    let close_pos = open_pos + close_pos;
                    let placeholder_content = &result[open_pos + 1..close_pos];

                    if let Some(replacement) = resolver(placeholder_content) {
                        let placeholder = &result[open_pos..=close_pos];
                        let replacement_str = match replacement {
                            Value::String(s) => s,
                            _ => replacement.to_string(),
                        };
                        result = result.replace(placeholder, &replacement_str);
                        start = open_pos + replacement_str.len();
                    } else {
                        start = close_pos + 1;
                    }
                } else {
                    break;
                }
            }

            json!(result)
        }
        Value::Object(obj) => {
            let new_obj = obj
                .iter()
                .map(|(k, v)| (k.clone(), replace_simple_placeholders(v, resolver)))
                .collect();
            Value::Object(new_obj)
        }
        Value::Array(arr) => {
            let new_arr = arr
                .iter()
                .map(|v| replace_simple_placeholders(v, resolver))
                .collect();
            Value::Array(new_arr)
        }
        _ => value.clone(),
    }
}

pub fn replace_path_parameters(value: &Value, path_params: &HashMap<String, String>) -> Value {
    let preprocessed = preprocess_path_parameters(value, path_params);

    replace_simple_placeholders(&preprocessed, |placeholder| {
        if let Some(param_name) = placeholder.strip_prefix("path.") {
            path_params.get(param_name).map(|v| json!(v))
        } else {
            None
        }
    })
}

fn preprocess_path_parameters(value: &Value, path_params: &HashMap<String, String>) -> Value {
    match value {
        Value::String(s) => {
            let mut result = s.clone();
            for (param_name, param_value) in path_params {
                let placeholder = format!("{{path.{param_name}}}");
                result = result.replace(&placeholder, param_value);
            }
            json!(result)
        }
        Value::Object(obj) => {
            let new_obj = obj
                .iter()
                .map(|(k, v)| (k.clone(), preprocess_path_parameters(v, path_params)))
                .collect();
            Value::Object(new_obj)
        }
        Value::Array(arr) => {
            let new_arr = arr
                .iter()
                .map(|v| preprocess_path_parameters(v, path_params))
                .collect();
            Value::Array(new_arr)
        }
        _ => value.clone(),
    }
}