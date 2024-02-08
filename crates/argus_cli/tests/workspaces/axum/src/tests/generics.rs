use super::*;

async fn handler<T>() {}

fn test() {
    use_as_handler(handler);
}
