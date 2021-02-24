use std::convert::Infallible;
use std::sync::Arc;

use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::{Deserialize, Serialize};
use warp::{Filter, reject, Rejection, Reply};
use warp::filters::BoxedFilter;
use warp::http::Response;
use warp::hyper::Body;

use crate::switch::SwitchService;

pub fn root(switch_service: Arc<SwitchService>) -> BoxedFilter<(impl Reply, )> {
    let index = with_switch_service(switch_service.clone())
        .and(warp::path::end())
        .and(warp::get())
        .and_then(index_handler);

    let file = with_switch_service(switch_service.clone())
        .and(warp::path!("file"))
        .and(warp::get())
        .and(warp::query::query::<ServeFileQuery>())
        .and_then(file_handler);

    let scan = with_switch_service(switch_service.clone())
        .and(warp::path!("scan"))
        .and(warp::post())
        .and_then(scan_handler);

    index
        .or(file)
        .or(scan)
        .boxed()
}

fn with_switch_service(
    switch_service: Arc<SwitchService>
) -> impl Filter<Extract=(Arc<SwitchService>, ), Error=Infallible> + Clone {
    warp::any().map(move || switch_service.clone())
}

async fn index_handler(switch_service: Arc<SwitchService>) -> Result<impl Reply, Rejection> {
    let files: Vec<FileOut> = switch_service
        .list_files()
        .iter()
        .map(|file| {
            let url = {
                let path = utf8_percent_encode(&file.rel_path, NON_ALPHANUMERIC).to_string();
                let fragment = utf8_percent_encode(&file.name, NON_ALPHANUMERIC).to_string();

                format!("/file?path={}#{}", path, fragment)
            };
            let size = file.size;

            FileOut {
                url,
                size,
                name: file.name.to_owned(),
            }
        })
        .collect();
    let files_len = files.len();

    let result = IndexOut {
        files,
        directories: vec![],
        success: Some(format!("Welcome. Now serving {} files", files_len)),
        referrer: None,
    };

    Ok(warp::reply::json(&result))
}

async fn file_handler(switch_service: Arc<SwitchService>, query: ServeFileQuery) -> Result<Response<Body>, Rejection> {
    if let Some(path) = switch_service.resolve_file(&query.path) {
        Ok(super::file::file_reply(&path).await)
    } else {
        Err(reject::not_found())
    }
}

async fn scan_handler(switch_service: Arc<SwitchService>) -> Result<impl Reply, Rejection> {
    let before = switch_service.list_files().len();
    switch_service.scan();
    let after = switch_service.list_files().len();

    let result = ScanReport { before, after };

    Ok(warp::reply::json(&result))
}

#[derive(Deserialize)]
struct ServeFileQuery {
    path: String,
}

#[derive(Serialize)]
struct ScanReport {
    before: usize,
    after: usize,
}

#[derive(Serialize)]
struct FileOut {
    url: String,
    size: u64,
    name: String,
}

#[derive(Serialize)]
struct IndexOut {
    files: Vec<FileOut>,
    directories: Vec<String>,
    success: Option<String>,
    referrer: Option<String>,
}
