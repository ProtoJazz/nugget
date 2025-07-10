use crate::types::StoredObject;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub fn resolve_cross_references(
    value: &Value,
    objects: &Arc<RwLock<HashMap<String, Vec<StoredObject>>>>,
) -> Value {
    match value {
        Value::String(s) => {
            if let Some(resolved) = resolve_reference_string(s, objects) {
                return resolved;
            }

            json!(s)
        }
        Value::Object(obj) => {
            let mut new_obj = serde_json::Map::new();
            for (k, v) in obj {
                new_obj.insert(k.clone(), resolve_cross_references(v, objects));
            }
            Value::Object(new_obj)
        }
        Value::Array(arr) => {
            let new_arr: Vec<Value> = arr
                .iter()
                .map(|v| resolve_cross_references(v, objects))
                .collect();
            Value::Array(new_arr)
        }
        _ => value.clone(),
    }
}

fn resolve_reference_string(
    s: &str,
    objects: &Arc<RwLock<HashMap<String, Vec<StoredObject>>>>,
) -> Option<Value> {
    let objects_guard = objects.read().unwrap();

    if s.starts_with("{objects.") && s.ends_with('}') {
        let object_type = &s[9..s.len() - 1];
        if let Some(objects_list) = objects_guard.get(object_type) {
            let data: Vec<Value> = objects_list.iter().map(|obj| obj.data.clone()).collect();
            return Some(json!(data));
        }
    }

    if s.starts_with("{objects.") && s.ends_with('}') && s.matches('.').count() >= 2 {
        let content = &s[9..s.len() - 1];
        let parts: Vec<&str> = content.split('.').collect();
        if parts.len() >= 2 {
            let object_type = parts[0];
            let field_path = parts[1..].join(".");

            if let Some(objects_list) = objects_guard.get(object_type) {
                let values: Vec<Value> = objects_list
                    .iter()
                    .filter_map(|obj| extract_field_value(&obj.data, &field_path))
                    .collect();
                return Some(json!(values));
            }
        }
    }

    if s.starts_with("{objects.") && s.contains('[') && s.ends_with("]}") {
        let content = &s[9..s.len() - 2];
        if let Some(bracket_pos) = content.find('[') {
            let object_type = &content[..bracket_pos];
            let id = &content[bracket_pos + 1..];

            if let Some(objects_list) = objects_guard.get(object_type) {
                if let Some(obj) = objects_list.iter().find(|o| o.id == id) {
                    return Some(obj.data.clone());
                }
            }
        }
    }

    if s.starts_with("{objects.") && s.contains('[') && s.contains("].") && s.ends_with('}') {
        let content = &s[9..s.len() - 1];
        if let Some(bracket_pos) = content.find('[') {
            if let Some(close_bracket) = content.find(']') {
                let object_type = &content[..bracket_pos];
                let id = &content[bracket_pos + 1..close_bracket];
                let field_path = &content[close_bracket + 2..];

                if let Some(objects_list) = objects_guard.get(object_type) {
                    if let Some(obj) = objects_list.iter().find(|o| o.id == id) {
                        if let Some(field_value) = extract_field_value(&obj.data, field_path) {
                            return Some(field_value);
                        }
                    }
                }
            }
        }
    }

    None
}

fn extract_field_value(data: &Value, field_path: &str) -> Option<Value> {
    let parts: Vec<&str> = field_path.split('.').collect();
    let mut current = data;

    for part in parts {
        match current {
            Value::Object(obj) => {
                if let Some(value) = obj.get(part) {
                    current = value;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }

    Some(current.clone())
}