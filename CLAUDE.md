# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Nugget is a dynamic HTTP stub server built in Rust that supports cross-references between different object types and Lua scripting for dynamic behavior. It allows you to create mock API endpoints that can store and reference data between requests, with powerful Lua scripts for authentication, rate limiting, conditional responses, and complex business logic. Perfect for testing complex systems with interdependent data.

## Architecture

The server is built using:
- **Axum** for HTTP routing and middleware
- **Tokio** for async runtime  
- **Serde** for JSON/YAML serialization
- **Clap** for command-line argument parsing
- **mlua** for Lua scripting integration

### Core Components

- **AppState** (`src/main.rs:76-81`): Global state containing configuration, storage for individual object lookups, objects storage for cross-references by type, and Lua state for persistent scripting data
- **Route Configuration** (`src/main.rs:35-45`): Defines endpoints with response templates, Lua scripts, variable generation, and object storage settings  
- **Lua Script Engine** (`src/main.rs:229-310`): Executes Lua scripts with access to request context, persistent state, and stored objects
- **Cross-Reference Engine**: Resolves references between stored objects using patterns like `{objects.type}`, `{objects.type.field}`, and `{objects.type[id].field}`
- **Variable Generation**: Generates dynamic values (UUIDs, integers, strings) for response templates
- **Payload Interpolation**: Merges request payloads with response templates using placeholder syntax

## Configuration

The server loads configuration from YAML files (default: `config.yaml`) that define:
- **Routes**: HTTP endpoints with method, path, response templates, or Lua scripts
- **Object Storage**: Which responses to store for cross-referencing (`object_name`, `store_object`)
- **Variable Generation**: Dynamic value creation for IDs and other fields
- **Defaults**: Fallback values for payload interpolation
- **Lua Scripts**: Dynamic behavior with access to request data, state, and objects

## Common Development Commands

```bash
# Build the project
cargo build

# Run in development mode
cargo run

# Run with custom config and port
cargo run -- --config custom.yaml --port 8080

# Run tests
cargo test

# Check code formatting
cargo fmt --check

# Run linter
cargo clippy
```

## Key Features

### Lua Scripting System
- Write dynamic logic with full access to request context (method, path, headers, body, path_params)
- Persistent state management with `state.get()` and `state.set()` 
- Access to stored objects from other endpoints via `objects.type`
- Custom HTTP status code control
- Authentication, rate limiting, conditional responses, and complex business logic

### Cross-Reference System  
- Store objects by type (orders, users, etc.) and reference them across endpoints
- Support for complex reference patterns: `{objects.orders[uuid].items}`
- Automatic ID-based object retrieval and field extraction
- Accessible from both templates and Lua scripts

### Dynamic Response Generation
- Generate UUIDs, random integers, and strings for response fields
- Interpolate request payloads into response templates
- Apply default values for missing payload fields
- Path parameter extraction and substitution

### Flexible Configuration
- YAML-based route configuration with variable definitions, templates, or Lua scripts
- Object storage settings for cross-endpoint data sharing
- Template-based response generation with placeholder syntax
- State management with clear endpoint for testing

## Cross-Reference Patterns

### Template System
- `{objects.type}` - Complete objects of specified type
- `{objects.type.field}` - Extract specific field from all objects
- `{objects.type[id]}` - Retrieve specific object by ID
- `{objects.type[id].field}` - Extract field from specific object
- `{path.param}` - Use URL path parameters in responses

### Lua Scripts  
- `objects.type` - Array of all objects of that type
- `objects.type[1].field` - Access specific object fields
- `request.method` - HTTP method
- `request.path` - Request path
- `request.headers["name"]` - Request headers
- `request.body` - Request body (JSON)
- `request.path_params.param` - URL path parameters
- `state.get("key")` - Get persistent state
- `state.set("key", value)` - Set persistent state