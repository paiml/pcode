// Tests for runtime module to improve coverage
use pcode::runtime::Runtime;
use std::time::Duration;

#[test]
fn test_runtime_creation_sync() {
    let runtime = Runtime::new();
    assert!(runtime.is_ok());
}

#[test]
fn test_runtime_spawn_sync() {
    let runtime = Runtime::new().unwrap();

    let handle = runtime.spawn(async {
        tokio::time::sleep(Duration::from_millis(10)).await;
        42
    });

    let result = runtime.block_on(handle);
    assert_eq!(result.unwrap(), 42);
}

#[test]
fn test_runtime_spawn_blocking_sync() {
    let runtime = Runtime::new().unwrap();

    let handle = runtime.spawn_blocking(|| {
        // Simulate CPU-intensive work
        let mut sum = 0;
        for i in 0..1000 {
            sum += i;
        }
        sum
    });

    let result = runtime.block_on(handle);
    assert_eq!(result.unwrap(), 499500); // Sum of 0..999
}

#[test]
fn test_runtime_block_on() {
    let runtime = Runtime::new().unwrap();

    // Test block_on method
    let result = runtime.block_on(async {
        tokio::time::sleep(Duration::from_millis(10)).await;
        "completed"
    });

    assert_eq!(result, "completed");
}
