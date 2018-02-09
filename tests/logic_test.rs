extern crate jez;

use jez::{InterpState, Value};


fn eval(rev: usize, prog: &'static str) -> (Value, InterpState) {
    jez::eval(rev, "main", prog).unwrap()
}

#[test]
fn test_sieves_basic() {
    let (tos, mut state) = eval(
        0,
        "
.version 0

.def main 0:
  0 10 range = @seq

  @seq 3 2 sieve
  @seq intersection

  0 swap 10 swap onsets
    ",
    );

    let (start, end) = tos.as_range().unwrap();
    let actual = state.heap_slice_mut(start, end).unwrap().to_vec();

    let expected = vec![0, 0, 1, 0, 0, 1, 0, 0, 1, 0]
        .iter()
        .map(|n| Value::Number(f64::from(*n)))
        .collect::<Vec<_>>();

    assert_eq!(actual, expected);
}

#[test]
fn test_sieves_xor() {
    let (tos, mut state) = eval(
        0,
        "
.version 0

.def main 0:
  0 32 range = @seq

  @seq 7 3 sieve = @seq1
  @seq 7 5 sieve = @seq2
  @seq 3 0 sieve = @seq3

  @seq1 @seq2 union @seq3 symmetric_difference
    ",
    );

    let (start, end) = tos.as_range().unwrap();
    let actual = state.heap_slice_mut(start, end).unwrap().to_vec();

    let expected = vec![0, 5, 6, 9, 10, 15, 17, 18, 19, 21, 26, 27, 30, 31]
        .iter()
        .map(|n| Value::Number(f64::from(*n)))
        .collect::<Vec<_>>();

    assert_eq!(actual, expected);
}

#[test]
fn test_onsets() {
    let (tos, mut state) = eval(
        0,
        "
.version 0

.def main 0:
  5 10 [0 1 2 7 8 10] onsets
    ",
    );

    let (start, end) = tos.as_range().unwrap();
    let actual = state.heap_slice_mut(start, end).unwrap().to_vec();

    let expected = vec![0, 0, 1, 1, 0]
        .iter()
        .map(|n| Value::Number(f64::from(*n)))
        .collect::<Vec<_>>();

    assert_eq!(actual, expected);
}

#[test]
fn test_rotate() {
    let (tos, mut state) = eval(
        0,
        "
.version 0

.def main 0:
  [ 1 2 3 4 ] 5 rotate
    ",
    );

    let (start, end) = tos.as_range().unwrap();
    let actual = state.heap_slice_mut(start, end).unwrap().to_vec();

    let expected = vec![4, 1, 2, 3]
        .iter()
        .map(|n| Value::Number(f64::from(*n)))
        .collect::<Vec<_>>();

    assert_eq!(actual, expected);
}
