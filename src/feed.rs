use std::{collections::HashMap, fs::File, io::ErrorKind, time::Duration};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::config::GlobalConfig;


#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum FeedType {
    Gtfs,
    GtfsRt,
    Gbfs,
}

#[derive(Deserialize, Clone, Debug)]
pub struct GtfsFeed {
    pub id: String,
    pub data_type: FeedType,
    pub latest_dataset: LatestDataset,
}


#[derive(Deserialize, Clone, Debug)]
pub struct LatestDataset {
    pub id: String,
    pub hosted_url: String,
    pub hash: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Feeds {
    pub all_feeds: IndexMap<String, GtfsFeed>,
}


#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct CacheControlFile(pub IndexMap<String, String>);

pub fn query_metadata(config: &mut GlobalConfig) -> std::io::Result<Feeds> {
    let mut feeds = Feeds {all_feeds: IndexMap::new()};

    let access_token = config.get_global_access_token()?
        .map(|v| format!("Bearer {v}"));

    for (name, agency) in &config.config.agencies.0 {
        let id = &agency.id;
        let base_url = &config.config.api.base_url;

        let url = format!("https://{base_url}/v1/gtfs_feeds/{id}");

        let mut req = ureq::get(url)
            .header("Accept", "application/json");

        if let Some(key) = &access_token {
            req = req.header("Authorization", key);
        }

        let mut body = req.call().map_err(|e| e.into_io())?;

        let feed = body.body_mut().read_json().map_err(|e| e.into_io())?;

        feeds.all_feeds.insert(name.clone(), feed);

        std::thread::sleep(Duration::new(0, 500_000_000));
    }
    
    Ok(feeds)
}

fn read_cache() -> std::io::Result<Option<CacheControlFile>> {
    let file = std::fs::read_to_string("dataset-cache/.cache.toml")
        .map(Some)
        .or_else(|e| match e.kind() {
            ErrorKind::NotFound | ErrorKind::PermissionDenied | ErrorKind::IsADirectory => Ok(None),
            _ => Err(e)
        })?;

    file
        .map(|v| toml::from_str(&v))
        .transpose()
        .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))
}

pub fn download_feeds(feeds: &Feeds) -> std::io::Result<()> {
    std::fs::create_dir_all("dataset-cache")?;
    let mut cache = read_cache()?.unwrap_or_default();


    for (agency, feed) in &feeds.all_feeds {
        let fname = format!("dataset-cache/{agency}.zip");
        if std::fs::metadata(&fname).is_err() || cache.0.get(agency) != Some(&feed.latest_dataset.hash) {
            cache.0.insert(agency.clone(), feed.latest_dataset.hash.clone());
            let mut req = ureq::get(&feed.latest_dataset.hosted_url).call()
                .map_err(|e| e.into_io())?;

            let mut reader = req.body_mut().as_reader();

            

            let mut file = File::create(fname)?;

            std::io::copy(&mut reader, &mut file)?;
        }
    }

    let cache = toml::to_string(&cache).unwrap();

    std::fs::write("dataset-cache/.cache.toml", &cache)?;

    Ok(())
}