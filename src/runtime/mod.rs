use std::sync::Arc;
use tokio::runtime::{Builder, Runtime as TokioRuntime};
use tracing::info;

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Failed to create runtime: {0}")]
    Creation(String),

    #[error("Task execution error: {0}")]
    Execution(String),
}

pub struct Runtime {
    inner: Arc<TokioRuntime>,
}

impl Runtime {
    pub fn new() -> Result<Self, RuntimeError> {
        let runtime = Builder::new_multi_thread()
            .worker_threads(2)
            .max_blocking_threads(2)
            .thread_stack_size(2 * 1024 * 1024)
            .enable_all()
            .thread_name("pcode-worker")
            .build()
            .map_err(|e| RuntimeError::Creation(e.to_string()))?;

        info!("Runtime initialized with 2 worker threads");

        Ok(Self {
            inner: Arc::new(runtime),
        })
    }

    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: std::future::Future,
    {
        self.inner.block_on(future)
    }

    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.inner.spawn(future)
    }

    pub fn spawn_blocking<F, R>(&self, f: F) -> tokio::task::JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        self.inner.spawn_blocking(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let runtime = Runtime::new();
        assert!(runtime.is_ok());
    }

    #[test]
    fn test_runtime_spawn() {
        let runtime = Runtime::new().unwrap();
        let handle = runtime.spawn(async { 42 });
        let result = runtime.block_on(handle);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_runtime_spawn_blocking() {
        let runtime = Runtime::new().unwrap();
        let handle = runtime.spawn_blocking(|| 84);
        let result = runtime.block_on(handle);
        assert_eq!(result.unwrap(), 84);
    }

    #[test]
    fn test_runtime_error_display() {
        let err = RuntimeError::Creation("test error".to_string());
        assert_eq!(err.to_string(), "Failed to create runtime: test error");

        let err = RuntimeError::Execution("exec error".to_string());
        assert_eq!(err.to_string(), "Task execution error: exec error");
    }
}
