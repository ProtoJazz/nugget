use crate::types::{AppState, Config, Route, LuaRequestContext, StoredObject};
use crate::cross_references::resolve_cross_references;
use crate::interpolation::{extract_path_parameters, replace_path_parameters, interpolate_payload};
use crate::lua_engine::execute_lua_script;
use crate::variable_generation::{generate_variable_value, replace_variables_in_value};
use serde_json::{Value, json};
use std::collections::HashMap;

pub fn find_matching_route(config: &Config, method: &str, path: &str) -> Option<Route> {
    for route in &config.routes {
        if route.method.to_uppercase() == method.to_uppercase()
            && (route.path == path || path_matches_pattern(&route.path, path))
        {
            return Some(route.clone());
        }
    }
    None
}

fn path_matches_pattern(pattern: &str, path: &str) -> bool {
    let pattern_parts: Vec<&str> = pattern.split('/').collect();
    let path_parts: Vec<&str> = path.split('/').collect();

    if pattern_parts.len() != path_parts.len() {
        return false;
    }

    for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
        if pattern_part.starts_with('{') && pattern_part.ends_with('}') {
            continue;
        }
        if pattern_part != path_part {
            return false;
        }
    }

    true
}

pub async fn process_response(
    state: &AppState,
    route: &Route,
    path: &str,
    payload: Option<&Value>,
    headers: &HashMap<String, String>,
) -> Value {
    let path_params = extract_path_parameters(&route.path, path);

    if let Some(lua_script) = &route.lua_script {
        let request_context = LuaRequestContext {
            method: route.method.clone(),
            path: path.to_string(),
            headers: headers.clone(),
            body: payload.cloned(),
            path_params: path_params.clone(),
        };

        match execute_lua_script(lua_script, state, &request_context).await {
            Ok(result) => return result,
            Err(_) => return json!({"error": "Failed to execute Lua script", "status": 500}),
        }
    }

    if let Some(response_template) = &route.response {
        let mut response_body = response_template.body.clone();

        response_body = replace_path_parameters(&response_body, &path_params);

        response_body = resolve_cross_references(&response_body, &state.objects);
        if route.method.to_uppercase() == "POST" {
            if let Some(variables) = &route.variables {
                let mut generated_vars = HashMap::new();

                for (var_name, var_config) in variables {
                    let value = generate_variable_value(var_config);
                    generated_vars.insert(var_name.clone(), value);
                }

                response_body = replace_variables_in_value(&response_body, &generated_vars);

                if let Some(payload) = payload {
                    response_body =
                        interpolate_payload(&response_body, payload, &state.config.defaults);
                }

                if let Some(id_value) = generated_vars.get("id") {
                    let storage_key = format!("{}_{}", route.path, id_value);
                    state
                        .storage
                        .write()
                        .unwrap()
                        .insert(storage_key, response_body.clone());

                    if let Some(object_name) = &route.object_name {
                        if route.store_object.unwrap_or(true) {
                            let stored_object = StoredObject {
                                id: id_value.as_str().unwrap_or("").to_string(),
                                data: response_body.clone(),
                            };

                            state
                                .objects
                                .write()
                                .unwrap()
                                .entry(object_name.clone())
                                .or_default()
                                .push(stored_object);
                        }
                    }
                }
            }
        }

        if route.method.to_uppercase() == "GET" && path.contains('/') {
            let path_parts: Vec<&str> = path.split('/').collect();
            if let Some(id) = path_parts.last() {
                let storage_key =
                    format!("{}_{}", path_parts[..path_parts.len() - 1].join("/"), id);

                if let Some(stored_response) = state.storage.read().unwrap().get(&storage_key) {
                    return stored_response.clone();
                }
            }
        }

        if let Some(payload) = payload {
            response_body = interpolate_payload(&response_body, payload, &state.config.defaults);
        }

        response_body
    } else {
        json!({"error": "No response template defined", "status": 500})
    }
}