use axum::{
    Router,
    extract::{Request, State},
    http::{Method, StatusCode},
    response::Json,
    routing::{any, get, post},
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(name = "nugget")]
#[command(about = "A dynamic HTTP stub server with cross-references")]
struct Args {
    #[arg(short, long, default_value = "config.yaml")]
    config: String,

    #[arg(short, long, default_value = "3000")]
    port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    routes: Vec<Route>,
    defaults: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Route {
    path: String,
    method: String,
    response: ResponseTemplate,
    variables: Option<HashMap<String, VariableConfig>>,
    /// Name for this object type (e.g., "orders", "users")
    object_name: Option<String>,
    /// Whether to store this response for cross-references
    store_object: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResponseTemplate {
    status: Option<u16>,
    body: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VariableConfig {
    #[serde(rename = "type")]
    var_type: String,
    default: Option<Value>,
}

#[derive(Debug, Clone)]
struct StoredObject {
    id: String,
    data: Value,
}

#[derive(Debug, Clone)]
struct AppState {
    config: Config,
    storage: Arc<RwLock<HashMap<String, Value>>>,
    objects: Arc<RwLock<HashMap<String, Vec<StoredObject>>>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let config_content = fs::read_to_string(&args.config)?;
    let config: Config = if args.config.ends_with(".yaml") || args.config.ends_with(".yml") {
        serde_yaml::from_str(&config_content)?
    } else {
        serde_json::from_str(&config_content)?
    };

    let state = AppState {
        config: config.clone(),
        storage: Arc::new(RwLock::new(HashMap::new())),
        objects: Arc::new(RwLock::new(HashMap::new())),
    };

    let mut app = Router::new();

    for route in &config.routes {
        let path = &route.path;
        let method = route.method.to_uppercase();

        match method.as_str() {
            "GET" => {
                app = app.route(path, get(handle_request));
            }
            "POST" => {
                app = app.route(path, post(handle_request));
            }
            _ => {
                app = app.route(path, any(handle_request));
            }
        }
    }

    app = app.route("/state/clear", post(clear_state));

    let listener = TcpListener::bind(format!("0.0.0.0:{}", args.port)).await?;
    println!("Server running on http://0.0.0.0:{}", args.port);

    axum::serve(listener, app.with_state(state)).await?;
    Ok(())
}

async fn clear_state(State(state): State<AppState>) -> Json<Value> {
    {
        let mut objects = state.objects.write().unwrap();
        objects.clear();
    }
    {
        let mut storage = state.storage.write().unwrap();
        storage.clear();
    }

    Json(json!({
        "status": "cleared",
        "message": "All stored state has been cleared"
    }))
}

async fn handle_request(
    State(state): State<AppState>,
    req: Request,
) -> Result<Json<Value>, StatusCode> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    let payload = if method == Method::POST || method == Method::PUT || method == Method::PATCH {
        let body = axum::body::to_bytes(req.into_body(), usize::MAX)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        if !body.is_empty() {
            Some(serde_json::from_slice::<Value>(&body).map_err(|_| StatusCode::BAD_REQUEST)?)
        } else {
            None
        }
    } else {
        None
    };

    let route = find_matching_route(&state.config, method.as_ref(), &path);

    if let Some(route) = route {
        let response = process_response(&state, &route, &path, payload.as_ref()).await;
        Ok(Json(response))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

fn find_matching_route(config: &Config, method: &str, path: &str) -> Option<Route> {
    for route in &config.routes {
        if route.method.to_uppercase() == method.to_uppercase() && (route.path == path || path_matches_pattern(&route.path, path)) {
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

async fn process_response(
    state: &AppState,
    route: &Route,
    path: &str,
    payload: Option<&Value>,
) -> Value {
    let mut response_body = route.response.body.clone();

    let path_params = extract_path_parameters(&route.path, path);
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
            let storage_key = format!("{}_{}", path_parts[..path_parts.len() - 1].join("/"), id);

            if let Some(stored_response) = state.storage.read().unwrap().get(&storage_key) {
                return stored_response.clone();
            }
        }
    }

    if let Some(payload) = payload {
        response_body = interpolate_payload(&response_body, payload, &state.config.defaults);
    }

    response_body
}

fn resolve_cross_references(
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

fn generate_variable_value(var_config: &VariableConfig) -> Value {
    match var_config.var_type.as_str() {
        "uuid" => json!(Uuid::new_v4().to_string()),
        "integer" => json!(rand::random::<u32>()),
        "string" => json!(format!("generated_{}", rand::random::<u16>())),
        _ => var_config.default.clone().unwrap_or(json!("default")),
    }
}

fn replace_variables_in_value(value: &Value, variables: &HashMap<String, Value>) -> Value {
    replace_simple_placeholders(value, |placeholder| variables.get(placeholder).cloned())
}

fn interpolate_payload(
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

fn extract_path_parameters(pattern: &str, path: &str) -> HashMap<String, String> {
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

fn replace_simple_placeholders<F>(value: &Value, resolver: F) -> Value
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

fn replace_path_parameters(value: &Value, path_params: &HashMap<String, String>) -> Value {
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
                let placeholder = format!("{{path.{}}}", param_name);
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
