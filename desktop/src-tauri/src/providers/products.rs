use super::supertracker::get;
use crate::model::{sample_key, Channel};
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub struct Product {
    id: u64,
    name: String,
}
#[derive(Serialize)]
pub struct Store {
    id: u64,
    name: String,
}
pub fn fresh(day: &Value) -> bool {
    day.as_str()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .is_some_and(|d| {
            (-1..=3).contains(
                &chrono::Utc::now()
                    .date_naive()
                    .signed_duration_since(d)
                    .num_days(),
            )
        })
}
#[tauri::command]
pub async fn search_products(query: String) -> Result<Vec<Product>, String> {
    let query = query.trim().to_owned();
    if !(2..=120).contains(&query.len()) {
        return Err("Enter 2–120 characters to find a product.".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let data = get("search", &[("q", query.as_str()), ("limit", "15")])?;
        let rows = data
            .as_array()
            .ok_or("Super Tracker returned an unfamiliar search response.")?;
        Ok(rows
            .iter()
            .filter_map(|v| {
                Some(Product {
                    id: v["product_id"].as_u64()?,
                    name: [
                        v["brand"].as_str(),
                        v["name"].as_str(),
                        v["size_text"].as_str(),
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" "),
                })
            })
            .collect())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn product_stores(product: u32) -> Result<Vec<Store>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let data = get(&format!("products/{product}/stores"), &[])?;
        let rows = data
            .as_array()
            .ok_or("Super Tracker returned an unfamiliar store response.")?;
        let mut stores: Vec<_> = rows
            .iter()
            .filter_map(|v| {
                Some(Store {
                    id: v["store_id"].as_u64()?,
                    name: format!("{} · {}", v["store"].as_str()?, v["retailer"].as_str()?),
                })
            })
            .collect();
        stores.sort_by(|a, b| a.name.cmp(&b.name));
        stores.dedup_by_key(|s| s.id);
        Ok(stores)
    })
    .await
    .map_err(|e| e.to_string())?
}
pub fn price(data: &Value, store: u64) -> Option<f64> {
    let rows = if store == 0 {
        data["offers"].as_array()?
    } else {
        data.as_array()?
    };
    rows.iter()
        .filter(|v| {
            if store == 0 {
                v["available"] == true && fresh(&v["price_day"])
            } else {
                v["store_id"].as_u64() == Some(store)
            }
        })
        .filter_map(|v| v["price"].as_u64())
        .filter(|p| *p > 0)
        .min()
        .map(|p| p as f64 / 100.0)
}
pub fn sample(channel: &Channel) -> Option<(String, f64)> {
    let product = channel
        .extra
        .get("product_id")?
        .as_u64()
        .filter(|id| *id > 0)?;
    let store = channel
        .extra
        .get("store_id")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let endpoint = if store == 0 {
        format!("products/{product}")
    } else {
        format!("products/{product}/stores")
    };
    price(&get(&endpoint, &[]).ok()?, store).map(|p| (sample_key(channel), p))
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn locked_store_never_falls_back_to_cheaper_store() {
        let rows = json!([{"store_id":1,"price":420},{"store_id":2,"price":699}]);
        assert_eq!(price(&rows, 2), Some(6.99));
        assert_eq!(price(&rows, 3), None);
    }
    #[test]
    fn national_price_excludes_unavailable_stale_and_club_offers() {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let data = json!({"offers":[
            {"available":true,"price_day":today,"price":550,"promo_price":100,"promo_kind":"club"},
            {"available":false,"price_day":today,"price":100},
            {"available":true,"price_day":"2000-01-01","price":90},
            {"available":true,"price_day":today,"price":699}
        ]});
        assert_eq!(price(&data, 0), Some(5.5));
    }
}
