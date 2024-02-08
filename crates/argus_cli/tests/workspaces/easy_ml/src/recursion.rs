use easy_ml::matrices::Matrix;

fn test() {
  let matrix = Matrix::from(vec![vec![1, 2], vec![3, 4]]);
  let determinant = easy_ml::linear_algebra::determinant(&matrix);
}
