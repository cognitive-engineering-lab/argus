use super::*;

async fn handler(foo: bool) {}

fn test() {
    use_as_handler(handler);
}
