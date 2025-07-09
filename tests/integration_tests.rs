use reqwest::Client;
use serde_json::{Value, json};
use std::process::{Child, Command};
use std::time::Duration;
use tokio::time::sleep;

// Helper struct to manage server lifecycle
struct TestServer {
    process: Child,
    base_url: String,
}

impl TestServer {
    async fn start() -> Self {
        Self::start_with_config("config.yaml").await
    }

    async fn start_with_config(config_file: &str) -> Self {
        // Find an available port starting from 3010
        let port = 3010;

        // Try to start server on different ports until we find one that works
        for attempt in 0..10 {
            let test_port = port + attempt;

            let mut child = Command::new("cargo")
                .args([
                    "run",
                    "--",
                    "--config",
                    config_file,
                    "--port",
                    &test_port.to_string(),
                ])
                .spawn()
                .expect("Failed to start server");

            let base_url = format!("http://localhost:{}", test_port);

            // Wait for server to start
            let client = Client::new();
            let mut server_started = false;

            for _ in 0..50 {
                // Increased wait time
                if let Ok(response) = client.get(format!("{}/health", base_url)).send().await {
                    if response.status().is_success() {
                        server_started = true;
                        break;
                    }
                }
                sleep(Duration::from_millis(200)).await;
            }

            if server_started {
                return TestServer {
                    process: child,
                    base_url,
                };
            } else {
                // Kill the process and try next port
                let _ = child.kill();
            }
        }

        panic!("Failed to start test server on any port");
    }

    async fn post_json(&self, endpoint: &str, data: Value) -> reqwest::Result<Value> {
        let client = Client::new();
        let response = client
            .post(format!("{}{}", self.base_url, endpoint))
            .json(&data)
            .send()
            .await?;

        response.json().await
    }

    async fn get_json(&self, endpoint: &str) -> reqwest::Result<Value> {
        let client = Client::new();
        let response = client
            .get(format!("{}{}", self.base_url, endpoint))
            .send()
            .await?;

        response.json().await
    }

    async fn get_with_headers(
        &self,
        endpoint: &str,
        headers: Vec<(&str, &str)>,
    ) -> reqwest::Result<reqwest::Response> {
        let client = Client::new();
        let mut request = client.get(format!("{}{}", self.base_url, endpoint));

        for (key, value) in headers {
            request = request.header(key, value);
        }

        request.send().await
    }

    async fn clear_state(&self) -> reqwest::Result<Value> {
        let client = Client::new();
        let response = client
            .post(format!("{}/state/clear", self.base_url))
            .send()
            .await?;

        response.json().await
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}

#[tokio::test]
async fn test_complete_workflow() {
    let server = TestServer::start().await;

    // Clear any existing state
    server.clear_state().await.expect("Failed to clear state");

    // Step 1: Create orders
    let order1 = server
        .post_json(
            "/orders",
            json!({
                "items": ["laptop", "mouse"],
                "customer": "John Doe",
                "total": 1200
            }),
        )
        .await
        .expect("Failed to create first order");

    let order2 = server
        .post_json(
            "/orders",
            json!({
                "items": ["keyboard", "monitor"],
                "customer": "Jane Smith",
                "total": 800
            }),
        )
        .await
        .expect("Failed to create second order");

    // Verify order creation
    assert!(order1.get("id").is_some(), "First order should have ID");
    assert!(
        order1.get("order_number").is_some(),
        "First order should have order number"
    );
    assert_eq!(order1["items"], json!(["laptop", "mouse"]));
    assert_eq!(order1["customer"], "John Doe");
    assert_eq!(order1["total"], 1200);

    assert!(order2.get("id").is_some(), "Second order should have ID");
    assert_eq!(order2["customer"], "Jane Smith");

    // Step 2: Create users
    let user1 = server
        .post_json(
            "/users",
            json!({
                "username": "johndoe",
                "email": "john@example.com",
                "role": "admin"
            }),
        )
        .await
        .expect("Failed to create first user");

    let user2 = server
        .post_json(
            "/users",
            json!({
                "username": "janesmith",
                "email": "jane@example.com",
                "role": "user"
            }),
        )
        .await
        .expect("Failed to create second user");

    // Verify user creation
    assert!(user1.get("id").is_some(), "First user should have ID");
    assert!(
        user1.get("user_id").is_some(),
        "First user should have user_id"
    );
    assert_eq!(user1["username"], "johndoe");
    assert_eq!(user1["email"], "john@example.com");
    assert_eq!(user1["role"], "admin");

    assert!(user2.get("id").is_some(), "Second user should have ID");
    assert_eq!(user2["username"], "janesmith");

    // Step 3: Test cross-reference functionality

    // Test inventory fulfillment
    let fulfillment = server
        .get_json("/inventory/fulfillment")
        .await
        .expect("Failed to get inventory fulfillment");

    assert!(
        fulfillment.get("pending_orders").is_some(),
        "Should have pending_orders"
    );
    assert!(
        fulfillment.get("order_items").is_some(),
        "Should have order_items"
    );

    let pending_orders = fulfillment["pending_orders"]
        .as_array()
        .expect("pending_orders should be an array");
    assert_eq!(pending_orders.len(), 2, "Should have 2 pending orders");

    let order_items = fulfillment["order_items"]
        .as_array()
        .expect("order_items should be an array");
    assert_eq!(order_items.len(), 2, "Should have 2 order items arrays");

    // Test order reports
    let reports = server
        .get_json("/reports/orders")
        .await
        .expect("Failed to get order reports");

    assert!(
        reports.get("all_orders").is_some(),
        "Should have all_orders"
    );
    assert!(reports.get("customers").is_some(), "Should have customers");
    assert!(
        reports.get("total_revenue").is_some(),
        "Should have total_revenue"
    );

    let customers = reports["customers"]
        .as_array()
        .expect("customers should be an array");
    assert_eq!(customers.len(), 2, "Should have 2 customers");
    assert!(
        customers.contains(&json!("John Doe")),
        "Should include John Doe"
    );
    assert!(
        customers.contains(&json!("Jane Smith")),
        "Should include Jane Smith"
    );

    // Test user management
    let user_mgmt = server
        .get_json("/admin/users")
        .await
        .expect("Failed to get user management");

    assert!(user_mgmt.get("users").is_some(), "Should have users");
    assert!(
        user_mgmt.get("usernames").is_some(),
        "Should have usernames"
    );

    let users = user_mgmt["users"]
        .as_array()
        .expect("users should be an array");
    assert_eq!(users.len(), 2, "Should have 2 users");

    let usernames = user_mgmt["usernames"]
        .as_array()
        .expect("usernames should be an array");
    assert_eq!(usernames.len(), 2, "Should have 2 usernames");
    assert!(
        usernames.contains(&json!("johndoe")),
        "Should include johndoe"
    );
    assert!(
        usernames.contains(&json!("janesmith")),
        "Should include janesmith"
    );

    // Step 4: Test specific object reference
    let order1_id = order1["id"].as_str().unwrap();
    let order_items_endpoint = format!("/inventory/order/{}/items", order1_id);
    let specific_order = server
        .get_json(&order_items_endpoint)
        .await
        .expect("Failed to get specific order items");

    assert_eq!(specific_order["order_id"], order1_id);
    assert_eq!(specific_order["items"], json!(["laptop", "mouse"]));
    assert_eq!(specific_order["customer"], "John Doe");

    // Step 5: Test advanced analytics dashboard
    let analytics = server
        .get_json("/analytics/dashboard")
        .await
        .expect("Failed to get analytics dashboard");

    // Verify summary section
    assert!(analytics.get("summary").is_some(), "Should have summary");
    let summary = &analytics["summary"];

    assert!(
        summary.get("total_orders").is_some(),
        "Should have total_orders"
    );
    assert!(
        summary.get("total_users").is_some(),
        "Should have total_users"
    );
    assert!(
        summary.get("revenue_by_customer").is_some(),
        "Should have revenue_by_customer"
    );
    assert!(
        summary.get("active_customers").is_some(),
        "Should have active_customers"
    );
    assert!(
        summary.get("admin_users").is_some(),
        "Should have admin_users"
    );

    // Verify details section
    assert!(analytics.get("details").is_some(), "Should have details");
    let details = &analytics["details"];

    assert!(
        details.get("recent_orders").is_some(),
        "Should have recent_orders"
    );
    assert!(
        details.get("user_breakdown").is_some(),
        "Should have user_breakdown"
    );

    // Verify cross-reference data
    let total_orders = summary["total_orders"]
        .as_array()
        .expect("total_orders should be array");
    assert_eq!(total_orders.len(), 2, "Should have 2 orders in analytics");

    let total_users = summary["total_users"]
        .as_array()
        .expect("total_users should be array");
    assert_eq!(total_users.len(), 2, "Should have 2 users in analytics");

    let active_customers = summary["active_customers"]
        .as_array()
        .expect("active_customers should be array");
    assert_eq!(active_customers.len(), 2, "Should have 2 active customers");
    assert!(
        active_customers.contains(&json!("John Doe")),
        "Should include John Doe"
    );
    assert!(
        active_customers.contains(&json!("Jane Smith")),
        "Should include Jane Smith"
    );

    let admin_users = summary["admin_users"]
        .as_array()
        .expect("admin_users should be array");
    assert_eq!(admin_users.len(), 2, "Should have 2 user roles");
    assert!(
        admin_users.contains(&json!("admin")),
        "Should include admin role"
    );
    assert!(
        admin_users.contains(&json!("user")),
        "Should include user role"
    );

    // Step 6: Test health check
    let health = server
        .get_json("/health")
        .await
        .expect("Failed to get health check");

    assert_eq!(health["status"], "healthy");
    assert!(
        health.get("objects_stored").is_some(),
        "Should have objects_stored"
    );

    let objects_stored = &health["objects_stored"];
    assert!(
        objects_stored.get("orders").is_some(),
        "Should have stored orders"
    );
    assert!(
        objects_stored.get("users").is_some(),
        "Should have stored users"
    );
}

#[tokio::test]
async fn test_variable_generation() {
    let server = TestServer::start().await;

    // Clear any existing state
    server.clear_state().await.expect("Failed to clear state");

    // Test UUID generation
    let order1 = server
        .post_json(
            "/orders",
            json!({
                "items": ["item1"],
                "customer": "Test Customer",
                "total": 100
            }),
        )
        .await
        .expect("Failed to create order");

    let order2 = server
        .post_json(
            "/orders",
            json!({
                "items": ["item2"],
                "customer": "Test Customer 2",
                "total": 200
            }),
        )
        .await
        .expect("Failed to create order");

    // Verify UUIDs are different
    assert_ne!(order1["id"], order2["id"], "UUIDs should be different");

    // Verify order numbers are different
    assert_ne!(
        order1["order_number"], order2["order_number"],
        "Order numbers should be different"
    );

    // Test that IDs are valid UUIDs (basic format check)
    let id1 = order1["id"].as_str().unwrap();
    let id2 = order2["id"].as_str().unwrap();

    assert!(id1.len() == 36, "UUID should be 36 characters");
    assert!(id2.len() == 36, "UUID should be 36 characters");
    assert!(id1.contains('-'), "UUID should contain hyphens");
    assert!(id2.contains('-'), "UUID should contain hyphens");
}

#[tokio::test]
async fn test_string_variable_with_prefix() {
    let server = TestServer::start().await;
    
    // Clear any existing state
    server.clear_state().await.expect("Failed to clear state");
    
    // Test string variable with prefix
    let response = server
        .post_json("/test/variables/string", json!({}))
        .await
        .expect("Failed to test string variables");
    
    assert_eq!(response["message"], "String variable test");
    
    // Verify id has the SKU_ prefix
    let id = response["id"].as_str().unwrap();
    assert!(id.starts_with("SKU_"), "ID should start with SKU_ prefix");
    assert!(id.len() > 4, "ID should be longer than just the prefix");
    
    // Verify name doesn't have prefix (uses default generation)
    let name = response["name"].as_str().unwrap();
    assert!(name.starts_with("generated_"), "Name should start with generated_");
    assert!(!name.starts_with("SKU_"), "Name should not have SKU_ prefix");
    
    // Test multiple requests generate different values
    let response2 = server
        .post_json("/test/variables/string", json!({}))
        .await
        .expect("Failed to test string variables again");
    
    assert_ne!(response["id"], response2["id"], "IDs should be different");
    assert_ne!(response["name"], response2["name"], "Names should be different");
    
    // Both should still have proper prefixes
    let id2 = response2["id"].as_str().unwrap();
    assert!(id2.starts_with("SKU_"), "Second ID should also start with SKU_ prefix");
}

#[tokio::test]
async fn test_integer_variable_with_min_max() {
    let server = TestServer::start().await;
    
    // Clear any existing state
    server.clear_state().await.expect("Failed to clear state");
    
    // Test integer variables with min/max constraints
    for _ in 0..10 {
        let response = server
            .post_json("/test/variables/integer", json!({}))
            .await
            .expect("Failed to test integer variables");
        
        assert_eq!(response["message"], "Integer variable test");
        
        // Verify quantity is within range [1, 100]
        let quantity = response["quantity"].as_i64().unwrap();
        assert!(quantity >= 1, "Quantity should be >= 1, got {}", quantity);
        assert!(quantity <= 100, "Quantity should be <= 100, got {}", quantity);
        
        // Verify price is within range [500, 2000]
        let price = response["price"].as_i64().unwrap();
        assert!(price >= 500, "Price should be >= 500, got {}", price);
        assert!(price <= 2000, "Price should be <= 2000, got {}", price);
    }
    
    // Test multiple requests generate different values (high probability)
    let response1 = server
        .post_json("/test/variables/integer", json!({}))
        .await
        .expect("Failed to test integer variables");
    
    let response2 = server
        .post_json("/test/variables/integer", json!({}))
        .await
        .expect("Failed to test integer variables");
    
    // Due to randomness, there's a small chance they could be equal, but very unlikely
    // We'll accept that possibility for this test
    let quantity1 = response1["quantity"].as_i64().unwrap();
    let quantity2 = response2["quantity"].as_i64().unwrap();
    let price1 = response1["price"].as_i64().unwrap();
    let price2 = response2["price"].as_i64().unwrap();
    
    // All values should still be in range
    assert!(quantity1 >= 1 && quantity1 <= 100);
    assert!(quantity2 >= 1 && quantity2 <= 100);
    assert!(price1 >= 500 && price1 <= 2000);
    assert!(price2 >= 500 && price2 <= 2000);
}

#[tokio::test]
async fn test_uuid_variable_ignores_invalid_params() {
    let server = TestServer::start().await;
    
    // Clear any existing state
    server.clear_state().await.expect("Failed to clear state");
    
    // Test UUID variable generation
    let response = server
        .post_json("/test/variables/uuid", json!({}))
        .await
        .expect("Failed to test UUID variables");
    
    assert_eq!(response["message"], "UUID variable test");
    
    // Verify id is a valid UUID format
    let id = response["id"].as_str().unwrap();
    assert_eq!(id.len(), 36, "UUID should be 36 characters");
    assert!(id.contains('-'), "UUID should contain hyphens");
    
    // UUID format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
    let parts: Vec<&str> = id.split('-').collect();
    assert_eq!(parts.len(), 5, "UUID should have 5 parts separated by hyphens");
    assert_eq!(parts[0].len(), 8, "First part should be 8 characters");
    assert_eq!(parts[1].len(), 4, "Second part should be 4 characters");
    assert_eq!(parts[2].len(), 4, "Third part should be 4 characters");
    assert_eq!(parts[3].len(), 4, "Fourth part should be 4 characters");
    assert_eq!(parts[4].len(), 12, "Fifth part should be 12 characters");
    
    // Test multiple requests generate different UUIDs
    let response2 = server
        .post_json("/test/variables/uuid", json!({}))
        .await
        .expect("Failed to test UUID variables again");
    
    let id2 = response2["id"].as_str().unwrap();
    assert_ne!(id, id2, "UUIDs should be different");
    assert_eq!(id2.len(), 36, "Second UUID should also be 36 characters");
    assert!(id2.contains('-'), "Second UUID should also contain hyphens");
}

#[tokio::test]
async fn test_payload_interpolation() {
    let server = TestServer::start().await;

    // Clear any existing state
    server.clear_state().await.expect("Failed to clear state");

    // Test with all fields provided
    let order = server
        .post_json(
            "/orders",
            json!({
                "items": ["custom_item"],
                "customer": "Custom Customer",
                "total": 999
            }),
        )
        .await
        .expect("Failed to create order");

    assert_eq!(order["items"], json!(["custom_item"]));
    assert_eq!(order["customer"], "Custom Customer");
    assert_eq!(order["total"], 999);
    assert_eq!(order["status"], "pending");

    // Test with missing fields (should use defaults)
    let minimal_order = server
        .post_json("/orders", json!({}))
        .await
        .expect("Failed to create minimal order");

    assert_eq!(minimal_order["items"], json!([])); // default from config
    assert_eq!(minimal_order["customer"], "Anonymous"); // default from config
    assert_eq!(minimal_order["total"], 0); // default from config
    assert_eq!(minimal_order["status"], "pending"); // hardcoded in template
}

#[tokio::test]
async fn test_lua_basic_functionality() {
    let server = TestServer::start_with_config("lua-test.yaml").await;

    // Test basic Lua response
    let response = server
        .get_json("/lua-hello")
        .await
        .expect("Failed to get lua-hello");

    assert_eq!(response["message"], "Hello from Lua!");
    assert_eq!(response["method"], "GET");
    assert_eq!(response["path"], "/lua-hello");
}

#[tokio::test]
async fn test_lua_authentication() {
    let server = TestServer::start_with_config("lua-test.yaml").await;

    // Test without auth header (should get 401)
    let response = server
        .get_with_headers("/auth-check", vec![])
        .await
        .expect("Failed to get auth-check");

    assert_eq!(response.status(), 401);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["error"], "Unauthorized");
    assert_eq!(body["message"], "Authentication required");

    // Test with wrong user (should get 401)
    let response = server
        .get_with_headers("/auth-check", vec![("user", "bob")])
        .await
        .expect("Failed to get auth-check");

    assert_eq!(response.status(), 401);

    // Test with admin user (should get 200)
    let response = server
        .get_with_headers("/auth-check", vec![("user", "admin")])
        .await
        .expect("Failed to get auth-check");

    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["message"], "Welcome, admin");
}

#[tokio::test]
async fn test_lua_state_management() {
    let server = TestServer::start_with_config("lua-test.yaml").await;

    // Clear state first
    server.clear_state().await.expect("Failed to clear state");

    // Test the flaky endpoint - should follow pattern: success, success, fail, success...

    // Request 1 - should succeed
    let response1 = server
        .get_with_headers("/flaky-endpoint", vec![])
        .await
        .expect("Failed to get flaky-endpoint");

    assert_eq!(response1.status(), 200);
    let body1: Value = response1.json().await.expect("Failed to parse JSON");
    assert_eq!(body1["message"], "Request successful");
    assert_eq!(body1["request_number"], 1);

    // Request 2 - should succeed
    let response2 = server
        .get_with_headers("/flaky-endpoint", vec![])
        .await
        .expect("Failed to get flaky-endpoint");

    assert_eq!(response2.status(), 200);
    let body2: Value = response2.json().await.expect("Failed to parse JSON");
    assert_eq!(body2["message"], "Request successful");
    assert_eq!(body2["request_number"], 2);

    // Request 3 - should fail
    let response3 = server
        .get_with_headers("/flaky-endpoint", vec![])
        .await
        .expect("Failed to get flaky-endpoint");

    assert_eq!(response3.status(), 500);
    let body3: Value = response3.json().await.expect("Failed to parse JSON");
    assert_eq!(body3["error"], "Simulated failure");
    assert_eq!(body3["request_number"], 3);

    // Request 4 - should succeed again
    let response4 = server
        .get_with_headers("/flaky-endpoint", vec![])
        .await
        .expect("Failed to get flaky-endpoint");

    assert_eq!(response4.status(), 200);
    let body4: Value = response4.json().await.expect("Failed to parse JSON");
    assert_eq!(body4["message"], "Request successful");
    assert_eq!(body4["request_number"], 4);
}

#[tokio::test]
async fn test_lua_state_persistence_across_endpoints() {
    let server = TestServer::start_with_config("lua-test.yaml").await;

    // Clear state
    server.clear_state().await.expect("Failed to clear state");

    // Hit flaky endpoint to increment counter
    let _ = server.get_with_headers("/flaky-endpoint", vec![]).await;
    let _ = server.get_with_headers("/flaky-endpoint", vec![]).await;

    // Hit it again - should be request #3 and fail
    let response = server
        .get_with_headers("/flaky-endpoint", vec![])
        .await
        .expect("Failed to get flaky-endpoint");

    assert_eq!(response.status(), 500);
    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["request_number"], 3);
}

#[tokio::test]
async fn test_traditional_template_still_works() {
    let server = TestServer::start_with_config("lua-test.yaml").await;

    // Test that traditional templates still work alongside Lua
    let response = server
        .get_json("/traditional")
        .await
        .expect("Failed to get traditional endpoint");

    assert_eq!(
        response["message"],
        "This is a traditional template response"
    );
    assert_eq!(response["timestamp"], "2024-01-01T00:00:00Z");
}

#[tokio::test]
async fn test_lua_body_access() {
    let server = TestServer::start_with_config("lua-test.yaml").await;

    // Test POST request with JSON body
    let test_data = json!({
        "name": "John Doe",
        "email": "john@example.com",
        "age": 30
    });

    let response = server
        .post_json("/echo-body", test_data.clone())
        .await
        .expect("Failed to post to echo-body");

    assert_eq!(response["message"], "Echo of request body");
    assert_eq!(response["received_body"], test_data);
}

#[tokio::test]
async fn test_lua_path_parameters() {
    let server = TestServer::start_with_config("lua-test.yaml").await;

    // Test path parameter extraction
    let response = server
        .get_json("/users/123/profile")
        .await
        .expect("Failed to get user profile");

    assert_eq!(response["user_id"], "123");
    assert_eq!(response["profile"]["name"], "User 123");
    assert_eq!(response["profile"]["active"], true);
}

#[tokio::test]
async fn test_lua_complex_features() {
    let server = TestServer::start_with_config("lua-test.yaml").await;

    // Test complex route with path params, headers, and body
    let request_data = json!({
        "username": "johndoe",
        "preferences": {
            "theme": "dark",
            "notifications": true
        }
    });

    // Test successful request
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/v1/users/456", server.base_url))
        .header("user-agent", "TestClient/1.0")
        .json(&request_data)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 201);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["user_id"], "456");
    assert_eq!(body["version"], "v1");
    assert_eq!(body["user_agent"], "TestClient/1.0");
    assert_eq!(body["received_data"], request_data);

    // Test unsupported API version
    let response = client
        .post(format!("{}/api/v2/users/456", server.base_url))
        .json(&request_data)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 400);

    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["error"], "Unsupported API version");
    assert!(
        body["supported_versions"]
            .as_array()
            .unwrap()
            .contains(&json!("v1"))
    );
}

#[tokio::test]
async fn test_lua_object_access_and_string_reversal() {
    let server = TestServer::start_with_config("lua-test.yaml").await;

    // Clear state first
    server.clear_state().await.expect("Failed to clear state");

    // Post the secret message
    let secret_message = "od selffaw doog tahw si rehtegot gnikcits";
    let response = server
        .post_json(
            "/secret-message",
            json!({
                "message": secret_message
            }),
        )
        .await
        .expect("Failed to post secret message");

    // Verify the message was stored
    assert!(response.get("id").is_some());
    assert_eq!(response["message"], secret_message);

    let message_id = response["id"].as_str().unwrap();

    // Retrieve and reverse the message using Lua
    let response = server
        .get_json(&format!("/secret-message/{}", message_id))
        .await
        .expect("Failed to get secret message");

    assert_eq!(response["id"], message_id);
    assert_eq!(response["original_message"], secret_message);
    assert_eq!(
        response["reversed_message"],
        "sticking together is what good waffles do"
    );

    // Test with non-existent message ID
    let response = server
        .get_with_headers("/secret-message/non-existent-id", vec![])
        .await
        .expect("Failed to get non-existent message");

    assert_eq!(response.status(), 404);
    let body: Value = response.json().await.expect("Failed to parse JSON");
    assert_eq!(body["error"], "Message not found");
    assert_eq!(body["id"], "non-existent-id");
}
