trait ToString {
  fn to_string(&self) -> String;
}

impl ToString for i32 {
  fn to_string(&self) -> String {
    todo!()
  }
}

impl<S, T> ToString for (S, T)
where
  S: ToString,
  T: ToString,
{
  fn to_string(&self) -> String {
    format!("({}, {})", self.0.to_string(), self.1.to_string())
  }
}

fn print_items<T: ToString>(items: &[T]) {
  for item in items {
    println!("{}", item.to_string());
  }
}

pub fn foo() {
  print_items(&[("a", "b")]);
}
