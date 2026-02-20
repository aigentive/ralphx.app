use super::*;

use futures::Stream;
use std::pin::Pin;

#[test]
fn test_which_crate_available() {
    // Verify which crate is available for CLI detection
    // Just verify import compiles, don't actually search
    use which::which;
    let _ = which("nonexistent_binary_12345");
}

#[test]
fn test_lazy_static_available() {
    // Verify lazy_static is available for global process tracking
    lazy_static::lazy_static! {
        static ref TEST_VALUE: i32 = 42;
    }
    assert_eq!(*TEST_VALUE, 42);
}

#[test]
fn test_tokio_process_feature() {
    // Verify tokio process feature is available
    use tokio::process::Command;
    let _ = Command::new("echo");
}

#[test]
fn test_pin_box_stream_return_type() {
    // Verify we can use Pin<Box<dyn Stream>> as return type
    use futures::stream;

    fn stream_fn() -> Pin<Box<dyn Stream<Item = i32> + Send>> {
        Box::pin(stream::iter(vec![1, 2, 3]))
    }

    let _ = stream_fn();
}
