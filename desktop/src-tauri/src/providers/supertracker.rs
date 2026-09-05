use super::{Feed, Source};
use serde_json::Value;
use std::{io::Read, time::Duration};
const BASE: &str = "https://supertracker.nz";

pub fn catalog() -> Feed {
    let mut feed = Feed::default();
    for (id,name,unit,min,max,description) in [
        ("supertracker_index","Food-price index","index",900.0,1100.0,"Daily NZ food-price nowcast; official base 1000. Data: Super Tracker and source retailers, CC BY 4.0. Not an official statistic."),
        ("supertracker_month","Food prices · 30 days","%",-10.0,10.0,"Food-price change over 30 days. Data: Super Tracker and source retailers, CC BY 4.0. Not an official statistic."),
        ("supertracker_year","Food prices · year","%",-10.0,10.0,"Food-price change over one year. Data: Super Tracker and source retailers, CC BY 4.0. Not an official statistic."),
        ("supertracker_coverage","Basket coverage","%",0.0,100.0,"Share of the food basket observed by Super Tracker. Data: Super Tracker and source retailers, CC BY 4.0."),
    ] {
        let mut source = Source::new(id,name,"Super Tracker",unit,max,description);source.minimum=min;feed.add(source,None);
    }
    feed
}
pub fn parse(data: &Value) -> Feed {
    let mut feed = catalog();
    let fresh = data["as_of"]
        .as_str()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .is_some_and(|d| {
            let age = chrono::Utc::now()
                .date_naive()
                .signed_duration_since(d)
                .num_days();
            (-1..=3).contains(&age)
        });
    if !fresh {
        return feed;
    }
    for (id, value) in [
        ("supertracker_index", data["headline"]["value"].as_f64()),
        ("supertracker_month", data["headline"]["d30"].as_f64()),
        ("supertracker_year", data["headline"]["yoy"].as_f64()),
        (
            "supertracker_coverage",
            data["coverage"]["weight"].as_f64().map(|v| v * 100.0),
        ),
    ] {
        if let Some(v) = value.filter(|v| v.is_finite()) {
            feed.values.insert(id.into(), v);
        }
    }
    feed
}
pub fn sample() -> Feed {
    let sample = || -> Option<Feed> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(12))
            .https_only(true)
            .user_agent("ESP-Gauge/2")
            .build()
            .ok()?;
        let response = client
            .get(format!("{BASE}/api/v1/index/latest"))
            .send()
            .ok()?
            .error_for_status()
            .ok()?;
        let mut bytes = Vec::new();
        response.take(128 * 1024 + 1).read_to_end(&mut bytes).ok()?;
        if bytes.len() > 128 * 1024 {
            return None;
        }
        Some(parse(&serde_json::from_slice::<Value>(&bytes).ok()?))
    };
    sample().unwrap_or_else(catalog)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_public_data_is_unavailable() {
        let mut data = serde_json::json!({"as_of":"2000-01-01","headline":{"value":1001,"d30":-2,"yoy":3},"coverage":{"weight":0.8}});
        assert!(parse(&data).values.is_empty());
        data["as_of"] = serde_json::json!(chrono::Utc::now().format("%Y-%m-%d").to_string());
        let feed = parse(&data);
        assert_eq!(feed.values["supertracker_month"], -2.0);
        assert_eq!(feed.values["supertracker_coverage"], 80.0);
    }
}
