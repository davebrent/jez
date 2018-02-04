use std::cmp::PartialEq;
use std::f64::EPSILON;

use rand::{Rng, StdRng};

use vm::types::{Effect, Event};


#[derive(Copy, Clone, Debug)]
struct State {
    delta: f64,
    event: Event,
}

impl PartialEq for State {
    fn eq(&self, other: &State) -> bool {
        other.event.value == self.event.value &&
            (other.delta - self.delta).abs() < EPSILON
    }
}

#[derive(Clone, Debug)]
struct Node {
    parent: Option<usize>,
    children: Vec<usize>,
    state: Option<State>,
    count: usize,
}

#[derive(Clone, Debug)]
struct ProbTree {
    arena: Vec<Node>,
}

#[derive(Clone)]
pub struct MarkovFilter {
    order: usize,
    capacity: usize,
    input: Vec<State>,
    remainder: f64,
    output: Vec<State>,
    probabilities: ProbTree,
    rng: StdRng,
    ready: bool,
}

impl ProbTree {
    pub fn new() -> ProbTree {
        let mut arena = Vec::new();
        arena.push(Node {
            parent: None,
            children: vec![],
            state: None,
            count: 0,
        });
        ProbTree { arena: arena }
    }

    pub fn find(&self, from: usize, state: &State) -> Option<usize> {
        let node = &self.arena[from];

        for idx in &node.children {
            let child = &self.arena[*idx];
            let other = child.state.unwrap();
            if other == *state {
                return Some(*idx);
            }
        }

        None
    }

    pub fn append(&mut self, parent: usize, state: &State) -> usize {
        let idx = self.arena.len();

        self.arena.push(Node {
            parent: Some(parent),
            children: vec![],
            state: Some(*state),
            count: 1,
        });

        let parent = &mut self.arena[parent];
        parent.children.push(idx);
        idx
    }
}

impl MarkovFilter {
    pub fn new(order: usize, capacity: usize, rng: StdRng) -> MarkovFilter {
        MarkovFilter {
            order: order,
            capacity: capacity,
            input: vec![],
            remainder: 0.0,
            output: vec![],
            probabilities: ProbTree::new(),
            rng: rng,
            ready: false,
        }
    }

    fn feed_input(&mut self, dur: f64, events: &[Event]) {
        let mut previous = 0.0;
        let mut remainder = self.remainder;

        let mut events = events.to_vec();
        events.sort_by(|a, b| a.onset.partial_cmp(&b.onset).unwrap());

        for event in &events {
            let delta = ((event.onset + remainder) - previous).max(0.0);
            previous = event.onset;
            remainder = 0.0;

            let state = State {
                event: *event,
                delta: delta,
            };

            self.input.push(state);

            if self.input.len() > self.capacity {
                self.input.remove(0);
            }
        }

        self.remainder = match events.last() {
            Some(event) => dur - event.onset,
            None => 0.0,
        };
    }

    fn build_tree(&mut self) -> ProbTree {
        let mut buff = Vec::with_capacity(self.order);
        let mut tree = ProbTree::new();

        for state in &self.input {
            buff.push(state);
            if buff.len() <= self.order {
                continue;
            }

            self.ready = true;

            let buff1 = buff.clone();
            let buff2 = buff.clone();
            let (previous, value) = buff1.split_at(self.order);
            let (_, next) = buff2.split_at(1);
            buff = next.to_vec();

            let mut root = 0;
            for key in previous {
                root = match tree.find(root, key) {
                    Some(idx) => idx,
                    None => tree.append(root, key),
                }
            }

            tree.arena[root].count += 1;

            assert_eq!(value.len(), 1);
            let value = value[0];
            match tree.find(root, value) {
                Some(idx) => {
                    let node = &mut tree.arena[idx];
                    node.count += 1;
                }
                None => {
                    tree.append(root, value);
                }
            };
        }

        tree
    }

    fn observe(&mut self, dur: f64, events: &[Event]) {
        self.feed_input(dur, events);
        self.probabilities = self.build_tree();
    }

    fn start(&mut self) -> Option<Vec<State>> {
        if self.probabilities.arena.len() == 1 {
            return None;
        }

        let mut output = Vec::with_capacity(self.order);
        let mut node = &self.probabilities.arena[0];

        while output.len() != self.order {
            if node.children.is_empty() {
                return None;
            }

            let idx = self.rng.gen_range(0, node.children.len());
            node = &self.probabilities.arena[node.children[idx]];
            output.push(node.state.unwrap());
        }

        Some(output)
    }

    fn step(&mut self) -> Option<State> {
        let mut trys = 0;
        let mut clear = false;

        'outer: loop {
            trys += 1;
            if trys > 100 {
                return None;
            }

            if clear {
                self.output.clear();
                clear = false;
            }

            if self.output.is_empty() {
                match self.start() {
                    Some(states) => self.output = states,
                    None => continue 'outer,
                };
            }

            let mut root = 0;
            for key in &self.output {
                root = match self.probabilities.find(root, key) {
                    Some(idx) => idx,
                    None => {
                        clear = true;
                        continue 'outer;
                    }
                }
            }

            let node = &self.probabilities.arena[root];
            let mut weight = self.rng.gen_range(0, node.count as i64);

            for child in &node.children {
                let child = &self.probabilities.arena[*child];
                weight -= child.count as i64;
                if weight > 0 {
                    continue;
                }

                let state = child.state.unwrap();
                self.output.push(state);

                if self.output.len() > self.order {
                    self.output.remove(0);
                }

                return Some(state);
            }

            self.output.clear();
        }
    }

    fn generate(&mut self, dur: f64) -> Vec<Event> {
        let mut output = Vec::new();
        let mut t = 0.0;

        while t < dur {
            let state = match self.step() {
                Some(state) => state,
                None => return vec![],
            };

            let mut event = state.event;
            event.onset = t;
            output.push(event);
            t += state.delta;
        }

        output
    }
}

impl Effect for MarkovFilter {
    fn apply(&mut self, dur: f64, events: &[Event]) -> Vec<Event> {
        self.observe(dur, events);

        if self.ready {
            self.generate(dur)
        } else {
            events.to_vec()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vm::types::{Destination, EventValue};

    use rand::SeedableRng;

    fn event(onset: f64, dur: f64, val: f64) -> Event {
        Event {
            dest: Destination::Midi(0, 0),
            onset: onset,
            dur: dur,
            value: EventValue::Trigger(val),
        }
    }

    fn random() -> StdRng {
        let seed: &[_] = &[8, 8, 8, 8];
        SeedableRng::from_seed(seed)
    }

    #[test]
    fn test_start_key() {
        let mut f = MarkovFilter::new(2, 16, random());

        let events = vec![
            event(0.0, 100.0, 1.0),
            event(100.0, 100.0, 2.0),
            event(200.0, 100.0, 3.0),
            event(300.0, 100.0, 4.0),
        ];

        f.observe(1000.0, &events);

        assert_eq!(
            f.start(),
            Some(vec![
                State {
                    delta: 0.0,
                    event: event(0.0, 100.0, 1.0),
                },
                State {
                    delta: 100.0,
                    event: event(100.0, 100.0, 2.0),
                },
            ])
        );
    }

    #[test]
    fn test_continuous_stream() {
        let mut f = MarkovFilter::new(1, 8, random());
        let events = vec![
            event(0.0, 100.0, 1.0),
            event(100.0, 100.0, 2.0),
            event(200.0, 100.0, 3.0),
            event(300.0, 100.0, 4.0),
        ];

        let result = f.apply(400.0, &events);

        assert_eq!(f.ready, true);
        assert_eq!(
            result,
            vec![
                event(0.0, 100.0, 3.0),
                event(100.0, 100.0, 4.0),
                event(200.0, 100.0, 4.0),
                event(300.0, 100.0, 2.0),
            ]
        );
    }
}
