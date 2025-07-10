use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub routes: Vec<Route>,
    pub defaults: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub path: String,
    pub method: String,
    pub response: Option<ResponseTemplate>,
    pub variables: Option<HashMap<String, VariableConfig>>,
    pub lua_script: Option<String>,
    /// Name for this object type (e.g., "orders", "users")
    pub object_name: Option<String>,
    /// Whether to store this response for cross-references
    pub store_object: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseTemplate {
    pub status: Option<u16>,
    pub body: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableConfig {
    #[serde(rename = "type")]
    pub var_type: String,
    pub default: Option<Value>,
    // String type parameters
    pub prefix: Option<String>,
    // Integer type parameters
    pub min: Option<i64>,
    pub max: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredObject {
    pub id: String,
    pub data: Value,
}

#[derive(Debug, Clone)]
pub struct LuaRequestContext {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Value>,
    pub path_params: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: Config,
    pub storage: Arc<RwLock<HashMap<String, Value>>>,
    pub objects: Arc<RwLock<HashMap<String, Vec<StoredObject>>>>,
    pub lua_state: Arc<RwLock<HashMap<String, Value>>>,
}
