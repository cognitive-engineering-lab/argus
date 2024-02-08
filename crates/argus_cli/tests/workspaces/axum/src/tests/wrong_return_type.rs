use super::*;

async fn handler() -> bool {
    false
}

fn test() {
    use_as_handler(handler);
}
