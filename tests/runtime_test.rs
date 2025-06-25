// Tests for runtime module to improve coverage
use pcode::runtime::Runtime;
use std::time::Duration;

#[tokio::test]
async fn test_runtime_creation() {
    let runtime = Runtime::new();
    assert!(runtime.is_ok());
    
    let runtime = runtime.unwrap();
    
    // Test spawning multiple tasks
    let mut handles = vec![];
    for i in 0..5 {
        let handle = runtime.spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            i * 2
        });
        handles.push(handle);
    }
    
    // Wait for all tasks
    for (i, handle) in handles.into_iter().enumerate() {
        let result = handle.await.unwrap();
        assert_eq!(result, i * 2);
    }
}

#[tokio::test]
async fn test_runtime_spawn_blocking_with_result() {
    let runtime = Runtime::new().unwrap();
    
    let handle = runtime.spawn_blocking(|| {
        // Simulate CPU-intensive work
        let mut sum = 0;
        for i in 0..1000 {
            sum += i;
        }
        sum
    });
    
    let result = handle.await.unwrap();
    assert_eq!(result, 499500); // Sum of 0..999
}

#[tokio::test]
async fn test_runtime_concurrent_operations() {
    let runtime = Runtime::new().unwrap();
    
    // Spawn multiple async tasks
    let async_task = runtime.spawn(async {
        tokio::time::sleep(Duration::from_millis(50)).await;
        "async done"
    });
    
    // Spawn blocking task
    let blocking_task = runtime.spawn_blocking(|| {
        std::thread::sleep(Duration::from_millis(30));
        "blocking done"
    });
    
    // Both should complete successfully
    let async_result = async_task.await.unwrap();
    let blocking_result = blocking_task.await.unwrap();
    
    assert_eq!(async_result, "async done");
    assert_eq!(blocking_result, "blocking done");
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