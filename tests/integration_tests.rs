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
        // Find an available port starting from 3010
        let port = 3010;

        // Try to start server on different ports until we find one that works
        for attempt in 0..10 {
            let test_port = port + attempt;

            let mut child = Command::new("cargo")
                .args(&[
                    "run",
                    "--",
                    "--config",
                    "config.yaml",
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
                if let Ok(response) = client.get(&format!("{}/health", base_url)).send().await {
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
            .post(&format!("{}{}", self.base_url, endpoint))
            .json(&data)
            .send()
            .await?;

        response.json().await
    }

    async fn get_json(&self, endpoint: &str) -> reqwest::Result<Value> {
        let client = Client::new();
        let response = client
            .get(&format!("{}{}", self.base_url, endpoint))
            .send()
            .await?;

        response.json().await
    }

    async fn clear_state(&self) -> reqwest::Result<Value> {
        let client = Client::new();
        let response = client
            .post(&format!("{}/state/clear", self.base_url))
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
    
    assert!(summary.get("total_orders").is_some(), "Should have total_orders");
    assert!(summary.get("total_users").is_some(), "Should have total_users");
    assert!(summary.get("revenue_by_customer").is_some(), "Should have revenue_by_customer");
    assert!(summary.get("active_customers").is_some(), "Should have active_customers");
    assert!(summary.get("admin_users").is_some(), "Should have admin_users");

    // Verify details section
    assert!(analytics.get("details").is_some(), "Should have details");
    let details = &analytics["details"];
    
    assert!(details.get("recent_orders").is_some(), "Should have recent_orders");
    assert!(details.get("user_breakdown").is_some(), "Should have user_breakdown");

    // Verify cross-reference data
    let total_orders = summary["total_orders"].as_array().expect("total_orders should be array");
    assert_eq!(total_orders.len(), 2, "Should have 2 orders in analytics");

    let total_users = summary["total_users"].as_array().expect("total_users should be array");
    assert_eq!(total_users.len(), 2, "Should have 2 users in analytics");

    let active_customers = summary["active_customers"].as_array().expect("active_customers should be array");
    assert_eq!(active_customers.len(), 2, "Should have 2 active customers");
    assert!(active_customers.contains(&json!("John Doe")), "Should include John Doe");
    assert!(active_customers.contains(&json!("Jane Smith")), "Should include Jane Smith");

    let admin_users = summary["admin_users"].as_array().expect("admin_users should be array");
    assert_eq!(admin_users.len(), 2, "Should have 2 user roles");
    assert!(admin_users.contains(&json!("admin")), "Should include admin role");
    assert!(admin_users.contains(&json!("user")), "Should include user role");

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
