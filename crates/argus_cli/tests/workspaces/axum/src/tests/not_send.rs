use super::*;

async fn handler() {
    let rc = std::rc::Rc::new(());
    async {}.await;
}

fn test() {
    use_as_handler(handler);
}
