mod agents;
mod local_data;
mod quotas;
mod rpc;
mod supertracker;

use crate::metrics::Samples;
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

#[derive(Clone, Serialize)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub group: String,
    pub unit: String,
    pub scale: f64,
    pub minimum: f64,
    pub description: String,
}
impl Source {
    pub fn new(
        id: &str,
        name: &str,
        group: &str,
        unit: &str,
        scale: f64,
        description: &str,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            group: group.into(),
            unit: unit.into(),
            scale,
            minimum: 0.0,
            description: description.into(),
        }
    }
}
#[derive(Clone, Default)]
pub struct Feed {
    pub sources: Vec<Source>,
    pub values: Samples,
}
impl Feed {
    pub fn add(&mut self, source: Source, value: Option<f64>) {
        if let Some(value) = value.filter(|n| n.is_finite()) {
            self.values.insert(source.id.clone(), value);
        }
        self.sources.push(source);
    }
    fn merge(&mut self, other: &Self) {
        self.sources.extend(other.sources.iter().cloned());
        self.values.extend(other.values.clone());
    }
}
pub struct Providers {
    local: Arc<Mutex<Feed>>,
    quotas: Arc<Mutex<Feed>>,
    public: Arc<Mutex<Feed>>,
    wants_public: Arc<AtomicBool>,
}
impl Providers {
    pub fn start() -> Self {
        let local = Arc::new(Mutex::new(Feed::default()));
        let quotas = Arc::new(Mutex::new(Feed::default()));
        let public = Arc::new(Mutex::new(supertracker::catalog()));
        let wants_public = Arc::new(AtomicBool::new(false));
        let shared = local.clone();
        std::thread::spawn(move || {
            let mut agents = agents::Agents::new();
            loop {
                *shared.lock().unwrap() = agents.sample();
                std::thread::sleep(Duration::from_secs(2));
            }
        });
        let shared = quotas.clone();
        std::thread::spawn(move || loop {
            let mut next = quotas::sample();
            let mut previous = shared.lock().unwrap();
            for source in &previous.sources {
                if !next.sources.iter().any(|s| s.id == source.id) {
                    next.sources.push(source.clone());
                }
            }
            *previous = next;
            drop(previous);
            std::thread::sleep(Duration::from_secs(60));
        });
        let shared = public.clone();
        let wanted = wants_public.clone();
        std::thread::spawn(move || {
            let mut last = std::time::Instant::now() - Duration::from_secs(600);
            loop {
                if wanted.load(Ordering::Relaxed) && last.elapsed() >= Duration::from_secs(300) {
                    *shared.lock().unwrap() = supertracker::sample();
                    last = std::time::Instant::now();
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        });
        Self {
            local,
            quotas,
            public,
            wants_public,
        }
    }
    pub fn snapshot(&self, public: bool) -> Feed {
        self.wants_public.store(public, Ordering::Relaxed);
        let mut feed = self.local.lock().unwrap().clone();
        feed.merge(&self.quotas.lock().unwrap());
        feed.merge(&self.public.lock().unwrap());
        feed
    }
}

pub fn diagnose() -> Feed {
    let mut feed = agents::Agents::new().sample();
    feed.merge(&quotas::sample());
    feed.merge(&supertracker::catalog());
    feed
}
