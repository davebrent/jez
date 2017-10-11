pub use super::msgs::Event;

pub trait Filter {
    fn apply(&mut self, dur: f64, events: &[Event]) -> Vec<Event>;
}
