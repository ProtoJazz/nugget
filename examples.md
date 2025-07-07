# Enhanced Stub Server with Cross-References and Lua Scripting

## New Features

The server now supports cross-references between different object types using a powerful reference syntax, plus dynamic Lua scripting for complex logic.

## Cross-Reference Syntax

### 1. Get All Objects of a Type
```yaml
"{objects.orders}"          # Returns array of all order objects
"{objects.users}"           # Returns array of all user objects
```

### 2. Get Specific Fields from All Objects
```yaml
"{objects.orders.items}"    # Returns array of all order items
"{objects.orders.customer}" # Returns array of all customer names
"{objects.users.username}"  # Returns array of all usernames
```

### 3. Get Specific Object by ID
```yaml
"{objects.orders[uuid-123]}"     # Returns specific order by ID
"{objects.users[uuid-456]}"      # Returns specific user by ID
```

### 4. Get Specific Field from Specific Object
```yaml
"{objects.orders[uuid-123].items}"    # Returns items from specific order
"{objects.orders[uuid-123].customer}" # Returns customer from specific order
"{objects.users[uuid-456].email}"     # Returns email from specific user
```

## Usage Examples

### 1. Create Some Orders

```bash
# Create first order
curl -X POST http://localhost:3000/orders \
  -H "Content-Type: application/json" \
  -d '{
    "items": ["laptop", "mouse"],
    "customer": "John Doe",
    "total": 1200
  }'

# Response: {"id": "550e8400-e29b-41d4-a716-446655440000", ...}

# Create second order
curl -X POST http://localhost:3000/orders \
  -H "Content-Type: application/json" \
  -d '{
    "items": ["keyboard", "monitor"],
    "customer": "Jane Smith",
    "total": 800
  }'

# Response: {"id": "660e8400-e29b-41d4-a716-446655440001", ...}
```

### 2. Create Some Users

```bash
# Create first user
curl -X POST http://localhost:3000/users \
  -H "Content-Type: application/json" \
  -d '{
    "username": "johndoe",
    "email": "john@example.com",
    "role": "admin"
  }'

# Create second user
curl -X POST http://localhost:3000/users \
  -H "Content-Type: application/json" \
  -d '{
    "username": "janesmith",
    "email": "jane@example.com",
    "role": "user"
  }'
```

### 3. Get Inventory/Fulfillment Data

```bash
curl http://localhost:3000/inventory/fulfillment
```

Response:
```json
{
  "pending_orders": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "order_number": 12345,
      "items": ["laptop", "mouse"],
      "customer": "John Doe",
      "status": "pending",
      "total": 1200,
      "created_at": "2024-01-01T00:00:00Z"
    },
    {
      "id": "660e8400-e29b-41d4-a716-446655440001",
      "order_number": 67890,
      "items": ["keyboard", "monitor"],
      "customer": "Jane Smith",
      "status": "pending",
      "total": 800,
      "created_at": "2024-01-01T00:00:00Z"
    }
  ],
  "order_items": [
    ["laptop", "mouse"],
    ["keyboard", "monitor"]
  ],
  "total_items_count": 0,
  "last_updated": "2024-01-01T00:00:00Z"
}
```

### 4. Get Order Reports

```bash
curl http://localhost:3000/reports/orders
```

Response:
```json
{
  "all_orders": [...],
  "customers": ["John Doe", "Jane Smith"],
  "total_revenue": [1200, 800]
}
```

### 5. Get Items from Specific Order

```bash
curl http://localhost:3000/inventory/order/550e8400-e29b-41d4-a716-446655440000/items
```

Response:
```json
{
  "order_id": "550e8400-e29b-41d4-a716-446655440000",
  "items": ["laptop", "mouse"],
  "customer": "John Doe"
}
```

### 6. Get User Management Dashboard

```bash
curl http://localhost:3000/admin/users
```

Response:
```json
{
  "users": [
    {
      "id": "770e8400-e29b-41d4-a716-446655440002",
      "user_id": 11111,
      "username": "johndoe",
      "email": "john@example.com",
      "role": "admin",
      "created_at": "2024-01-01T00:00:00Z"
    },
    {
      "id": "880e8400-e29b-41d4-a716-446655440003",
      "user_id": 22222,
      "username": "janesmith",
      "email": "jane@example.com",
      "role": "user",
      "created_at": "2024-01-01T00:00:00Z"
    }
  ],
  "usernames": ["johndoe", "janesmith"],
  "user_count": 0
}
```

### 7. Health Check with Object Summary

```bash
curl http://localhost:3000/health
```

Response:
```json
{
  "status": "healthy",
  "timestamp": "2024-01-01T00:00:00Z",
  "objects_stored": {
    "orders": [...],
    "users": [...]
  }
}
```

## Configuration Features

### Object Storage
- Set `object_name` to define the object type (e.g., "orders", "users")
- Set `store_object: true` to enable cross-references (default: true)
- Objects are stored with their generated IDs for later reference

### Cross-Reference Patterns
- `{objects.type}` - All objects of that type
- `{objects.type.field}` - Field values from all objects
- `{objects.type[id]}` - Specific object by ID
- `{objects.type[id].field}` - Specific field from specific object

### Use Cases
- **Inventory Management**: Reference all order items across orders
- **Reporting**: Aggregate data from multiple object types
- **Analytics**: Extract specific metrics from stored objects
- **Cross-Service Data**: Share data between different API endpoints

## Advanced Example

```yaml
# Complex cross-reference example
- path: /analytics/dashboard
  method: GET
  response:
    status: 200
    body:
      summary:
        total_orders: "{objects.orders}"
        total_users: "{objects.users}"
        revenue_by_customer: "{objects.orders.total}"
        active_customers: "{objects.orders.customer}"
        admin_users: "{objects.users.role}"
      details:
        recent_orders: "{objects.orders}"
        user_breakdown: "{objects.users.username}"
```

This creates a powerful system where you can create relationships between different stub endpoints without hardcoding data!

## Lua Scripting Examples

### 1. Authentication and Authorization

```yaml
routes:
  - path: /protected-resource
    method: GET
    lua_script: |
      -- Check if user is authenticated
      local auth_header = request.headers["authorization"]
      if not auth_header then
        return {
          status = 401,
          body = { error = "Missing authorization header" }
        }
      end
      
      -- Simple token validation (in real use, validate against a service)
      local token = auth_header:match("Bearer (.+)")
      if token ~= "valid-token-123" then
        return {
          status = 403,
          body = { error = "Invalid token" }
        }
      end
      
      -- Check user role from header
      local user_role = request.headers["x-user-role"]
      if user_role ~= "admin" then
        return {
          status = 403,
          body = { error = "Insufficient permissions" }
        }
      end
      
      return {
        status = 200,
        body = {
          message = "Access granted",
          resource = "protected data here"
        }
      }
```

### 2. Rate Limiting and Circuit Breaker

```yaml
routes:
  - path: /rate-limited-api
    method: GET
    lua_script: |
      -- Get current request count for this client
      local client_ip = request.headers["x-forwarded-for"] or "unknown"
      local rate_key = "rate_limit_" .. client_ip
      local current_count = state.get(rate_key) or 0
      
      -- Check rate limit (5 requests per "window")
      if current_count >= 5 then
        return {
          status = 429,
          body = {
            error = "Rate limit exceeded",
            retry_after = 60
          }
        }
      end
      
      -- Increment counter
      state.set(rate_key, current_count + 1)
      
      -- Simulate circuit breaker - every 10th request fails
      local global_count = state.get("global_requests") or 0
      global_count = global_count + 1
      state.set("global_requests", global_count)
      
      if global_count % 10 == 0 then
        return {
          status = 503,
          body = { error = "Service temporarily unavailable" }
        }
      end
      
      return {
        status = 200,
        body = {
          message = "Request successful",
          requests_remaining = 5 - (current_count + 1),
          global_request_number = global_count
        }
      }
```

### 3. Dynamic Data Processing with Object Access

```yaml
routes:
  # First, create some users (traditional route)
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
        role: "{payload.role}"

  # Then process user data with Lua
  - path: /admin/user-stats
    method: GET
    lua_script: |
      local users = objects.users or {}
      
      -- Analyze user data
      local total_users = #users
      local admin_count = 0
      local user_count = 0
      local email_domains = {}
      
      for i, user in ipairs(users) do
        -- Count roles
        if user.role == "admin" then
          admin_count = admin_count + 1
        else
          user_count = user_count + 1
        end
        
        -- Extract email domains
        local domain = user.email:match("@(.+)")
        if domain then
          email_domains[domain] = (email_domains[domain] or 0) + 1
        end
      end
      
      return {
        status = 200,
        body = {
          total_users = total_users,
          role_breakdown = {
            admins = admin_count,
            users = user_count
          },
          email_domains = email_domains,
          users_list = users
        }
      }
```

### 4. Complex Business Logic with Request Processing

```yaml
routes:
  - path: /api/{version}/orders/{order_id}/process
    method: POST
    lua_script: |
      local version = request.path_params.version
      local order_id = request.path_params.order_id
      local processing_type = request.body and request.body.type or "standard"
      
      -- Validate API version
      if version ~= "v1" and version ~= "v2" then
        return {
          status = 400,
          body = {
            error = "Unsupported API version",
            supported_versions = {"v1", "v2"}
          }
        }
      end
      
      -- Find the order in stored objects
      local orders = objects.orders or {}
      local found_order = nil
      for i, order in ipairs(orders) do
        if order.id == order_id then
          found_order = order
          break
        end
      end
      
      if not found_order then
        return {
          status = 404,
          body = { error = "Order not found" }
        }
      end
      
      -- Process based on type and version
      local processing_fee = 0
      if version == "v2" then
        processing_fee = processing_type == "express" and 25 or 10
      else
        processing_fee = 5
      end
      
      -- Calculate new total
      local original_total = found_order.total or 0
      local new_total = original_total + processing_fee
      
      -- Store processing record
      local process_count = state.get("processed_orders") or 0
      state.set("processed_orders", process_count + 1)
      
      return {
        status = 200,
        body = {
          order_id = order_id,
          original_total = original_total,
          processing_fee = processing_fee,
          new_total = new_total,
          processing_type = processing_type,
          api_version = version,
          processed_count = process_count + 1,
          customer = found_order.customer
        }
      }
```

### 5. Secret Message Example (String Manipulation)

```yaml
routes:
  # Store secret message (traditional route)
  - path: /secret-message
    method: POST
    object_name: messages
    store_object: true
    variables:
      id:
        type: uuid
    response:
      status: 201
      body:
        id: "{id}"
        message: "{payload.message}"
        stored_at: "2024-01-01T00:00:00Z"

  # Retrieve and reverse message (Lua script)
  - path: /secret-message/{id}
    method: GET
    lua_script: |
      local message_id = request.path_params.id
      local messages = objects.messages or {}
      local found_message = nil
      
      -- Find message by ID
      for i, msg in ipairs(messages) do
        if msg.id == message_id then
          found_message = msg
          break
        end
      end
      
      if not found_message then
        return {
          status = 404,
          body = { error = "Message not found" }
        }
      end
      
      -- Reverse the message
      local original = found_message.message
      local reversed = ""
      for i = #original, 1, -1 do
        reversed = reversed .. string.sub(original, i, i)
      end
      
      return {
        status = 200,
        body = {
          id = message_id,
          original_message = original,
          reversed_message = reversed,
          retrieved_at = os.date("%Y-%m-%dT%H:%M:%SZ")
        }
      }
```

## Lua Script Capabilities

### Request Context Access
- `request.method` - HTTP method
- `request.path` - Request path
- `request.headers["name"]` - Request headers
- `request.body` - Request body (parsed JSON)
- `request.path_params.param` - URL path parameters

### Persistent State Management
- `state.get("key")` - Retrieve persistent values
- `state.set("key", value)` - Store persistent values
- State persists across all requests until server restart or `/state/clear`

### Object Access
- `objects.type` - Access stored objects from other endpoints
- Objects are the same as available in template cross-references
- Perfect for building complex relationships between endpoints

### Response Control
- Return `{ status = 200, body = {...} }` for full HTTP response control
- Status codes: 200, 201, 400, 401, 403, 404, 429, 500, 503, etc.
- Body can be any Lua table (converted to JSON)

## Testing Lua Scripts

```bash
# Test authentication
curl -H "authorization: Bearer valid-token-123" \
     -H "x-user-role: admin" \
     http://localhost:3000/protected-resource

# Test rate limiting
for i in {1..7}; do
  curl http://localhost:3000/rate-limited-api
done

# Test secret message
MESSAGE_ID=$(curl -X POST http://localhost:3000/secret-message \
  -H "Content-Type: application/json" \
  -d '{"message": "od selffaw doog tahw si rehtegot gnikcits"}' | jq -r .id)

curl http://localhost:3000/secret-message/$MESSAGE_ID
# Returns: "sticking together is what good waffles do"
```

# Create orders
POST /orders → stores as "orders" objects

# Create users  
POST /users → stores as "users" objects

# Get inventory data
GET /inventory/fulfillment → returns all order items using {objects.orders.items}

# Get specific order items
GET /inventory/order/uuid-123/items → returns items from that specific order
