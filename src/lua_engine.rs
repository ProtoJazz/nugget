use crate::types::{AppState, LuaRequestContext};
use mlua::{Lua, LuaSerdeExt, Value as LuaValue};
use serde_json::Value;
use std::collections::HashMap;

pub async fn execute_lua_script(
    script: &str,
    state: &AppState,
    request_context: &LuaRequestContext,
) -> Result<Value, String> {
    let lua = Lua::new();

    let request_table = lua.create_table().map_err(|e| e.to_string())?;
    request_table
        .set("method", request_context.method.clone())
        .map_err(|e| e.to_string())?;
    request_table
        .set("path", request_context.path.clone())
        .map_err(|e| e.to_string())?;

    let headers_table = lua.create_table().map_err(|e| e.to_string())?;
    for (key, value) in &request_context.headers {
        headers_table
            .set(key.clone(), value.clone())
            .map_err(|e| e.to_string())?;
    }

    let state_arc = state.lua_state.clone();
    let state_get = lua
        .create_function(move |lua, key: String| {
            let state_guard = state_arc.read().unwrap();
            match state_guard.get(&key) {
                Some(value) => lua.to_value(value),
                None => Ok(LuaValue::Nil),
            }
        })
        .map_err(|e| e.to_string())?;

    let state_arc2 = state.lua_state.clone();
    let state_set = lua
        .create_function(move |lua, (key, value): (String, LuaValue)| {
            let mut state_guard = state_arc2.write().unwrap();
            let json_value: Value = lua.from_value(value).unwrap_or(Value::Null);
            state_guard.insert(key, json_value);
            Ok(())
        })
        .map_err(|e| e.to_string())?;

    let state_table = lua.create_table().map_err(|e| e.to_string())?;
    state_table
        .set("get", state_get)
        .map_err(|e| e.to_string())?;
    state_table
        .set("set", state_set)
        .map_err(|e| e.to_string())?;

    lua.globals()
        .set("state", state_table)
        .map_err(|e| e.to_string())?;

    let objects_guard = state.objects.read().unwrap();
    let mut lua_objects: HashMap<String, Vec<Value>> = HashMap::new();

    for (object_type, stored_objects) in objects_guard.iter() {
        let data_objects: Vec<Value> = stored_objects.iter().map(|obj| obj.data.clone()).collect();
        lua_objects.insert(object_type.clone(), data_objects);
    }

    let objects_value = lua.to_value(&lua_objects).map_err(|e| e.to_string())?;
    lua.globals()
        .set("objects", objects_value)
        .map_err(|e| e.to_string())?;

    request_table
        .set("headers", headers_table)
        .map_err(|e| e.to_string())?;

    if let Some(body) = &request_context.body {
        let body_value = lua.to_value(body).map_err(|e| e.to_string())?;
        request_table
            .set("body", body_value)
            .map_err(|e| e.to_string())?;
    }

    let path_params_table = lua.create_table().map_err(|e| e.to_string())?;
    for (key, value) in &request_context.path_params {
        path_params_table
            .set(key.clone(), value.clone())
            .map_err(|e| e.to_string())?;
    }
    request_table
        .set("path_params", path_params_table)
        .map_err(|e| e.to_string())?;

    lua.globals()
        .set("request", request_table)
        .map_err(|e| e.to_string())?;

    let result: LuaValue = lua.load(script).eval().map_err(|e| e.to_string())?;

    let json_result: Value = lua
        .from_value(result)
        .map_err(|e| format!("Failed to convert Lua result to JSON: {e}"))?;

    Ok(json_result)
}