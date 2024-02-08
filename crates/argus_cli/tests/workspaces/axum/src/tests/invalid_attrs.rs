use super::*;

async fn handler() {}

fn test() {
    use_as_handler(handler);
}
