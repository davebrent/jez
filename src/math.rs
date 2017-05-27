use std::time::Duration;


pub type Point = [f64; 2];
pub type Curve = [f64; 8];


/// Create a cubic bezier curve from two points
pub fn path_to_curve(p0: &Point, p1: &Point) -> Curve {
    let xt = (p1[0] - p0[0]) * (1.0 / 3.0);
    let yt = (p1[1] - p0[1]) * (1.0 / 3.0);
    [p0[0],
     p0[1],
     p0[0] + xt,
     p0[1] + yt,
     p0[0] + (xt * 2.0),
     p0[1] + (yt * 2.0),
     p1[0],
     p1[1]]
}

/// Compute point 't' on a cubic bezier curve
pub fn point_on_curve(t: f64, curve: &Curve) -> Point {
    let t = t.min(1.0).max(0.0);
    let p0x = curve[0];
    let p0y = curve[1];
    let p1x = curve[2];
    let p1y = curve[3];
    let p2x = curve[4];
    let p2y = curve[5];
    let p3x = curve[6];
    let p3y = curve[7];
    let t2 = t * t;
    let t3 = t2 * t;
    let ct = 1.0 - t;
    let ct2 = ct * ct;
    let ct3 = ct2 * ct;
    let x = ct3 * p0x + 3.0 * ct2 * t * p1x + 3.0 * ct * t2 * p2x + t3 * p3x;
    let y = ct3 * p0y + 3.0 * ct2 * t * p1y + 3.0 * ct * t2 * p2y + t3 * p3y;
    [x, y]
}

pub fn millis_to_dur(millis: f64) -> Duration {
    let secs = (millis / 1000.0).floor();
    let nanos = (millis - (secs * 1000.0)) * 1000000.0;
    Duration::new(secs as u64, nanos as u32)
}

pub fn dur_to_millis(dur: &Duration) -> f64 {
    let secs = dur.as_secs() as f64 * 1000.0;
    let nanos = dur.subsec_nanos() as f64 / 1000000.0;
    secs + nanos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_on_curve() {
        let curve = path_to_curve(&[0.0, 0.0], &[1.0, 128.0]);
        assert_eq!(point_on_curve(0.0, &curve), [0.0, 0.0]);
        assert_eq!(point_on_curve(1.0, &curve), [1.0, 128.0]);
        assert_eq!(point_on_curve(1.5, &curve), [1.0, 128.0]);
        assert_eq!(point_on_curve(-1.5, &curve), [0.0, 0.0]);
        assert_eq!(point_on_curve(0.5, &curve), [0.5, 64.0]);
    }

    #[test]
    fn test_time_fns() {
        let dur = millis_to_dur(2500.0);
        assert_eq!(dur, Duration::new(2, 500000000));
        assert_eq!(dur_to_millis(&dur), 2500.0);
    }
}
