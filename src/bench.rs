use std::time::Instant;

pub struct Monitor {
    start_point: Instant,
    points: Vec<(String, Instant)>,
}

impl Monitor {
    pub fn new() -> Self {
        Self {
            start_point: Instant::now(),
            points: Vec::new(),
        }
    }

    pub fn print(&self) {
        let mut pos = &self.start_point;
        for (a, b) in &self.points {
            println!("{} in {:?}", a, b.duration_since(*pos));
            pos = b;
        }
    }

    pub fn add_fn<T>(&mut self, name: &str, fx: impl Fn() -> T) -> T {
        let k = fx();
        self.add(name);
        k
    }

    pub fn add(&mut self, name: &str) {
        self.points.push((name.to_owned(), Instant::now()));
    }
}
