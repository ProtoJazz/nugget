# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Nugget is a dynamic HTTP stub server built in Rust that supports cross-references between different object types. It allows you to create mock API endpoints that can store and reference data between requests, making it ideal for testing complex systems with interdependent data.

## Architecture

The server is built using:
- **Axum** for HTTP routing and middleware
- **Tokio** for async runtime  
- **Serde** for JSON/YAML serialization
- **Clap** for command-line argument parsing

### Core Components

- **AppState** (`src/main.rs:69-73`): Global state containing configuration, storage for individual object lookups, and objects storage for cross-references by type
- **Route Configuration** (`src/main.rs:37-46`): Defines endpoints with response templates, variable generation, and object storage settings
- **Cross-Reference Engine** (`src/main.rs:245-339`): Resolves references between stored objects using patterns like `{objects.type}`, `{objects.type.field}`, and `{objects.type[id].field}`
- **Variable Generation** (`src/main.rs:361-368`): Generates dynamic values (UUIDs, integers, strings) for response templates
- **Payload Interpolation** (`src/main.rs:399-453`): Merges request payloads with response templates using placeholder syntax

## Configuration

The server loads configuration from YAML files (default: `config.yaml`) that define:
- **Routes**: HTTP endpoints with method, path, and response templates
- **Object Storage**: Which responses to store for cross-referencing (`object_name`, `store_object`)
- **Variable Generation**: Dynamic value creation for IDs and other fields
- **Defaults**: Fallback values for payload interpolation

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

### Cross-Reference System
- Store objects by type (orders, users, etc.) and reference them across endpoints
- Support for complex reference patterns: `{objects.orders[uuid].items}`
- Automatic ID-based object retrieval and field extraction

### Dynamic Response Generation
- Generate UUIDs, random integers, and strings for response fields
- Interpolate request payloads into response templates
- Apply default values for missing payload fields
- Path parameter extraction and substitution

### Flexible Configuration
- YAML-based route configuration with variable definitions
- Object storage settings for cross-endpoint data sharing
- Template-based response generation with placeholder syntax
- State management with clear endpoint for testing

## Cross-Reference Patterns

- `{objects.type}` - Complete objects of specified type
- `{objects.type.field}` - Extract specific field from all objects
- `{objects.type[id]}` - Retrieve specific object by ID
- `{objects.type[id].field}` - Extract field from specific object
- `{path.param}` - Use URL path parameters in responses