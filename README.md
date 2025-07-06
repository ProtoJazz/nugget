# Nugget

A dynamic HTTP stub server with cross-references built in Rust. Perfect for testing complex systems with interdependent data.

## Features

- **Dynamic Response Generation**: Generate UUIDs, random integers, and strings
- **Cross-Reference System**: Store and reference data between different endpoints
- **Path Parameter Extraction**: Extract and use URL parameters in responses
- **Payload Interpolation**: Merge request payloads with response templates
- **Type Preservation**: Maintain JSON types (arrays, numbers, objects)
- **State Management**: Clear server state with `/state/clear` endpoint

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