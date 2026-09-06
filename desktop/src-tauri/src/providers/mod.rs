mod agents;
mod local_data;
pub mod products;
mod quotas;
mod rpc;
pub mod supertracker;

use crate::metrics::Samples;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Serialize)]
pub struct Choice {
    pub id: String,
    pub name: String,
}
#[derive(Clone, Serialize)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub group: String,
    pub unit: String,
    pub scale: f64,
    pub minimum: f64,
    pub description: String,
    pub options: Vec<Choice>,
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
            options: Vec::new(),
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
    wants_public: Arc<Mutex<Vec<crate::model::Channel>>>,
}
impl Providers {
    pub fn start() -> Self {
        let local = Arc::new(Mutex::new(Feed::default()));
        let quotas = Arc::new(Mutex::new(Feed::default()));
        let public = Arc::new(Mutex::new(supertracker::catalog()));
        let wants_public = Arc::new(Mutex::new(Vec::<crate::model::Channel>::new()));
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
            let mut cache = std::collections::BTreeMap::<
                String,
                (std::time::Instant, crate::metrics::Samples),
            >::new();
            loop {
                let channels = wanted.lock().unwrap().clone();
                let mut next = supertracker::catalog();
                let mut active = std::collections::BTreeSet::new();
                for channel in &channels {
                    let key = if channel.source == "supertracker_product" {
                        crate::model::sample_key(channel)
                    } else {
                        "index".into()
                    };
                    if !active.insert(key.clone()) {
                        continue;
                    }
                    if cache
                        .get(&key)
                        .is_none_or(|(at, _)| at.elapsed() >= Duration::from_secs(300))
                    {
                        let values = if key == "index" {
                            supertracker::sample().values
                        } else {
                            products::sample(channel).into_iter().collect()
                        };
                        cache.insert(key.clone(), (std::time::Instant::now(), values));
                    }
                    if let Some((_, values)) = cache.get(&key) {
                        next.values.extend(values.clone());
                    }
                }
                cache.retain(|key, _| active.contains(key));
                *shared.lock().unwrap() = next;
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
    pub fn snapshot(&self, config: Option<&crate::model::Config>) -> Feed {
        *self.wants_public.lock().unwrap() = config
            .into_iter()
            .flat_map(|c| &c.channels)
            .filter(|c| c.enabled && c.source.starts_with("supertracker_"))
            .cloned()
            .collect();
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
