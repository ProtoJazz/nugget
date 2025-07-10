use axum::{
    Router,
    extract::{Request, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Json},
    routing::{any, get, post},
};
use clap::Parser;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;

mod types;
mod variable_generation;
mod cross_references;
mod lua_engine;
mod interpolation;
mod request_processing;

use types::{AppState, Config};
use request_processing::{find_matching_route, process_response};

#[derive(Parser, Debug)]
#[command(name = "nugget")]
#[command(about = "A dynamic HTTP stub server with cross-references")]
struct Args {
    #[arg(short, long, default_value = "config.yaml")]
    config: String,

    #[arg(short, long, default_value = "3000")]
    port: u16,
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
        lua_state: Arc::new(RwLock::new(HashMap::new())),
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
    {
        let mut lua_state = state.lua_state.write().unwrap();
        lua_state.clear();
    }

    Json(json!({
        "status": "cleared",
        "message": "All stored state has been cleared"
    }))
}

async fn handle_request(
    State(state): State<AppState>,
    req: Request,
) -> Result<impl IntoResponse, StatusCode> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    let headers: HashMap<String, String> = req
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

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
        let response = process_response(&state, &route, &path, payload.as_ref(), &headers).await;

        // Check for Lua script status (top-level status field)
        if let Some(status_value) = response.get("status") {
            if let Some(status_code) = status_value.as_u64() {
                let status = StatusCode::from_u16(status_code as u16)
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

                let body = response.get("body").unwrap_or(&response).clone();

                return Ok((status, Json(body)).into_response());
            }
        }

        // Check for traditional template status
        if let Some(response_template) = &route.response {
            if let Some(template_status) = response_template.status {
                let status = StatusCode::from_u16(template_status)
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

                return Ok((status, Json(response)).into_response());
            }
        }

        Ok(Json(response).into_response())
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}