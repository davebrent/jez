extern crate jez;
use std::time::Duration;


#[test]
fn test_curves_with_triggers() {
    let dur = Duration::new(1, 0);
    let dt = Duration::new(0, 1000000);
    let res = jez::simulate(dur,
                            dt,
                            "
.version 1

.def spu 0:
  [0 127] linear 500 1 track
  [64 64 64 64] 500 2 track

.def mpu_out_note 0:
  event_value 127 event_duration 1 noteout
    ");

    assert!(res.is_ok());
}
