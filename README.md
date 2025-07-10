# Nugget

A dynamic HTTP stub server with cross-references built in Rust. Perfect for testing complex systems with interdependent data.

## Features

- **Lua Scripting**: Write dynamic logic with access to request data, headers, body, path params, and persistent state
- **Dynamic Response Generation**: Generate UUIDs, random integers, and strings
- **Cross-Reference System**: Store and reference data between different endpoints
- **Path Parameter Extraction**: Extract and use URL parameters in responses
- **Payload Interpolation**: Merge request payloads with response templates
- **Type Preservation**: Maintain JSON types (arrays, numbers, objects)
- **State Management**: Clear server state with `/state/clear` endpoint
- **Flexible Authentication**: Header-based auth, rate limiting, conditional responses via Lua
- **Status Code Control**: Return custom HTTP status codes from Lua scripts

## Installation

### Pre-built Binaries

Download the latest release for your platform from the [Releases page](../../releases).

#### Linux/macOS
```bash
# Download and make executable
chmod +x nugget-*
sudo mv nugget-* /usr/local/bin/nugget
```

#### Windows
Download `nugget-windows-x86_64.exe` and rename to `nugget.exe`, then add to your PATH.

### From Source (requires Rust)
```bash
git clone <this-repo>
cd nugget
cargo install --path .
```

## Quick Start

1. Create a `config.yaml` file:
```yaml
routes:
  - path: /users
    method: POST
    object_name: users
    store_object: true
    variables:
      id:
        type: uuid
    response:
      status: 201
      body:
        id: "{id}"
        username: "{payload.username}"
        email: "{payload.email}"

  - path: /users
    method: GET
    response:
      status: 200
      body:
        users: "{objects.users}"
        count: 0
```

2. Run the server:
```bash
nugget -c config.yaml -p 3000
```

3. Test it:
```bash
# Create a user
curl -X POST http://localhost:3000/users \
  -H "Content-Type: application/json" \
  -d '{"username": "john", "email": "john@example.com"}'

# Get all users
curl http://localhost:3000/users
```


## Lua Scripting

Routes can use Lua scripts for dynamic behavior instead of static templates. Lua scripts have access to:

### Request Context
```lua
-- HTTP method, path, headers, body, path parameters
local method = request.method
local path = request.path
local user_header = request.headers["user"]
local request_data = request.body
local user_id = request.path_params.id
```

### Persistent State
```lua
-- Store and retrieve values across requests
local count = state.get("request_count") or 0
state.set("request_count", count + 1)
```

### Object Access
```lua
-- Access stored objects from other endpoints
local users = objects.users
local specific_user = objects.users[1]
```

### Example: Authentication
```yaml
routes:
  - path: /protected
    method: GET
    lua_script: |
      if request.headers["user"] ~= "admin" then
        return {
          status = 401,
          body = { error = "Unauthorized" }
        }
      end
      
      return {
        status = 200,
        body = { message = "Welcome, admin!" }
      }
```

### Example: Rate Limiting
```yaml
routes:
  - path: /flaky-endpoint
    method: GET
    lua_script: |
      -- Every 3rd request fails
      local count = state.get("request_count") or 0
      count = count + 1
      state.set("request_count", count)
      
      if count % 3 == 0 then
        return {
          status = 500,
          body = { error = "Simulated failure" }
        }
      end
      
      return {
        status = 200,
        body = { 
          message = "Success",
          request_number = count
        }
      }
```

### Example: Complex Data Processing
```yaml
routes:
  - path: /api/{version}/process
    method: POST
    lua_script: |
      local version = request.path_params.version
      local user_agent = request.headers["user-agent"] or "unknown"
      
      -- Validate API version
      if version ~= "v1" then
        return {
          status = 400,
          body = { error = "Unsupported API version" }
        }
      end
      
      -- Process request body
      local result = {
        processed_data = request.body,
        client_info = user_agent,
        timestamp = os.date("%Y-%m-%dT%H:%M:%SZ")
      }
      
      return {
        status = 200,
        body = result
      }
```

## Configuration

See [examples.md](examples.md) for detailed configuration examples and cross-reference patterns.

### Cross-Reference Patterns

- `{objects.type}` - All objects of that type
- `{objects.type.field}` - Field values from all objects  
- `{objects.type[id]}` - Specific object by ID
- `{objects.type[id].field}` - Specific field from specific object

### Variable Generation

```yaml
variables:
  id:
    type: uuid        # Generates UUID
  order_number:
    type: integer     # Generates random integer
  token:
    type: string      # Generates random string
```

## Variable Generation Parameters

Variables support different types with customizable parameters:

### String Variables

Generate random strings with optional prefixes:

```yaml
variables:
  sku:
    type: string
    prefix: "SKU_"    # Optional: adds prefix to generated string
  name:
    type: string       # Without prefix: generates "generated_12345"
```

**String Parameters:**
- `prefix` (optional): String to prepend to the generated value
- `min` and `max` are ignored for string types (with warning)

**Examples:**
- `prefix: "ORDER_"` → `"ORDER_generated_54321"`
- No prefix → `"generated_12345"`

### Integer Variables

Generate random integers within specified ranges:

```yaml
variables:
  quantity:
    type: integer
    min: 1           # Optional: minimum value (default: 0)
    max: 100         # Optional: maximum value (default: i64::MAX)
  price:
    type: integer
    min: 500
    max: 2000
  id:
    type: integer    # Without constraints: generates any u32
```

**Integer Parameters:**
- `min` (optional): Minimum value (inclusive)
- `max` (optional): Maximum value (inclusive)  
- `prefix` is ignored for integer types (with warning)

**Examples:**
- `min: 1, max: 100` → Random number between 1 and 100
- `min: 1000, max: 9999` → 4-digit random number
- No constraints → Any random 32-bit unsigned integer

### UUID Variables

Generate RFC 4122 compliant UUIDs:

```yaml
variables:
  id:
    type: uuid        # Generates standard UUID
  user_id:
    type: uuid
    prefix: "ignored" # All parameters ignored for UUID (with warnings)
```

**UUID Parameters:**
- All parameters (`prefix`, `min`, `max`) are ignored with console warnings
- Always generates standard UUID format: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`

**Examples:**
- Always generates: `"550e8400-e29b-41d4-a716-446655440000"` (example format)

### Default Values

All variable types support default fallback values:

```yaml
variables:
  priority:
    type: integer
    min: 1
    max: 5
    default: 3       # Used if generation fails
  status:
    type: string
    prefix: "STATUS_"
    default: "PENDING"
```

### Parameter Validation

The system validates parameters and shows un-helpful warnings:

```yaml
variables:
  invalid_example:
    type: uuid
    prefix: "UUID_"   # Warning: UUID type doesn't support 'prefix' parameter
    min: 1           # Warning: UUID type doesn't support 'min' parameter
```

**Console Output:**
```
Warning: UUID type doesn't support 'prefix' parameter. Ignoring this parameter.
Warning: UUID type doesn't support 'min' parameter. Ignoring this parameter.
```

### Complete Example

```yaml
routes:
  - path: /products
    method: POST
    variables:
      id:
        type: uuid
      sku:
        type: string
        prefix: "PROD_"
      price:
        type: integer
        min: 100
        max: 50000
      stock:
        type: integer
        min: 0
        max: 1000
        default: 0
    response:
      status: 201
      body:
        id: "{id}"              # "550e8400-e29b-41d4-a716-446655440000"
        sku: "{sku}"            # "PROD_generated_12345"
        price: "{price}"        # 15750 (between 100-50000)
        stock: "{stock}"        # 42 (between 0-1000)
        name: "{payload.name}"
        category: "{payload.category}"
```

### Path Parameters

Use `{id}` in paths and reference with `{path.id}` in responses:

```yaml
- path: /users/{id}
  method: GET
  response:
    body:
      user_id: "{path.id}"
      profile: "{objects.users[{path.id}]}"
```

## State Management

Clear all stored data:
```bash
curl -X POST http://localhost:3000/state/clear
```

## Development

```bash
# Run tests
cargo test

# Run with custom config
cargo run -- -c config.yaml -p 3000

# Format code
cargo fmt

# Lint
cargo clippy
```

## License

MIT License - see LICENSE file for details.