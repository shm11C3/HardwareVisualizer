///
/// 小数第1位へ丸める
///
pub fn round1(v: f32) -> f32 {
  (v * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_round1_positive_numbers() {
    assert_eq!(round1(1.23), 1.2);
    assert_eq!(round1(1.27), 1.3);
    assert_eq!(round1(1.25), 1.3); // 四捨五入
    assert_eq!(round1(1.24), 1.2);
    assert_eq!(round1(1.26), 1.3);
  }

  #[test]
  fn test_round1_negative_numbers() {
    assert_eq!(round1(-1.23), -1.2);
    assert_eq!(round1(-1.27), -1.3);
    assert_eq!(round1(-1.25), -1.3);
    assert_eq!(round1(-1.24), -1.2);
    assert_eq!(round1(-1.26), -1.3);
  }

  #[test]
  fn test_round1_zero_and_integers() {
    assert_eq!(round1(0.0), 0.0);
    assert_eq!(round1(1.0), 1.0);
    assert_eq!(round1(-1.0), -1.0);
    assert_eq!(round1(10.0), 10.0);
  }

  #[test]
  fn test_round1_edge_cases() {
    // 既に小数第1位の値
    assert_eq!(round1(1.1), 1.1);
    assert_eq!(round1(1.9), 1.9);

    // 小数点以下が0の場合
    assert_eq!(round1(5.05), 5.1);
    assert_eq!(round1(5.04), 5.0);

    // 非常に小さい値
    assert_eq!(round1(0.01), 0.0);
    assert_eq!(round1(0.05), 0.1);
    assert_eq!(round1(0.09), 0.1);
  }

  #[test]
  fn test_round1_large_numbers() {
    assert_eq!(round1(123.456), 123.5);
    assert_eq!(round1(999.99), 1000.0);
    assert_eq!(round1(1000.01), 1000.0);
  }

  #[test]
  fn test_round1_precision() {
    // 精度テスト：結果が正確に小数第1位まで表現されていることを確認
    let result = round1(1.234567);
    assert!((result * 10.0).fract().abs() < f32::EPSILON);
  }
}
