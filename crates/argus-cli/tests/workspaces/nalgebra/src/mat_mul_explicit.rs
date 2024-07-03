use nalgebra::Vector2;
use nalgebra_sparse::{CooMatrix, CsrMatrix};

fn test() {
  let coo1 = CooMatrix::<Vector2<i32>>::new(5, 5);
  let coo2 = CooMatrix::<Vector2<i32>>::new(5, 5);
  let m1 = CsrMatrix::from(&coo1);
  let m2 = CsrMatrix::from(&coo2);

  std::ops::Mul::mul(m1, m2);
}
