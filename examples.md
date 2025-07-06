# Enhanced Stub Server with Cross-References

## New Features

The server now supports cross-references between different object types using a powerful reference syntax.

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


# Create orders
POST /orders → stores as "orders" objects

# Create users  
POST /users → stores as "users" objects

# Get inventory data
GET /inventory/fulfillment → returns all order items using {objects.orders.items}

# Get specific order items
GET /inventory/order/uuid-123/items → returns items from that specific order
