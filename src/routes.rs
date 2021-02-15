use actix_files::NamedFile;
use actix_web::web::{self, ServiceConfig};
use actix_web::{HttpRequest, HttpResponse, Responder};
use actix_web_httpauth::extractors::basic::{BasicAuth, Config as BasicConfig};
use actix_web_httpauth::extractors::AuthenticationError;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};

use crate::switch::SwitchService;

pub fn root(cfg: &mut ServiceConfig) {
    cfg.route("/", web::get().to(index))
        .route("/scan", web::post().to(scan))
        .route("/file", web::get().to(serve_file));
}

async fn index(
    req: HttpRequest,
    auth: BasicAuth,
    switch_service: web::Data<SwitchService>,
) -> impl Responder {
    let tinfoil_headers = TinfoilRequestHeaders::from_req(&req);

    let uid: &str = &tinfoil_headers.uid.unwrap_or("[empty]");
    log::info!(
        "{ip}, {user}, UID:{uid} | Requesting index",
        ip = req.connection_info().remote_addr().unwrap_or("unknown"),
        user = auth.user_id(),
        uid = uid
    );

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

    HttpResponse::Ok().json(result)
}

async fn serve_file(
    req: HttpRequest,
    auth: BasicAuth,
    switch_service: web::Data<SwitchService>,
    query: web::Query<ServeFileQuery>,
) -> impl Responder {
    if let Some(path) = switch_service.resolve_file(&query.path) {
        log::info!(
            r#"{ip}, {user} | Requesting file "{}""#,
            &query.path,
            ip = req.connection_info().remote_addr().unwrap_or("unknown"),
            user = auth.user_id(),
        );

        let file = NamedFile::open(&path).unwrap();

        file.into_response(&req).unwrap()
    } else {
        log::warn!(
            r#"{ip}, {user} | Requesting non-existing file "{}""#,
            &query.path,
            ip = req.connection_info().remote_addr().unwrap_or("unknown"),
            user = auth.user_id(),
        );

        HttpResponse::NotFound().finish()
    }
}

async fn scan(
    req: HttpRequest,
    auth: BasicAuth,
    switch_service: web::Data<SwitchService>,
) -> impl Responder {
    if auth.user_id() != "admin" {
        let config = req
            .app_data::<BasicConfig>()
            .map(|data| data.clone())
            .unwrap_or_else(Default::default);
        let err = AuthenticationError::from(config);

        return HttpResponse::from_error(err.into());
    }

    log::info!(
        "{ip}, {user} | Requesting re-scan",
        ip = req.connection_info().remote_addr().unwrap_or("unknown"),
        user = auth.user_id()
    );

    let before = switch_service.list_files().len();
    switch_service.scan();
    let after = switch_service.list_files().len();

    HttpResponse::Ok().json(ScanReport { before, after })
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

struct TinfoilRequestHeaders<'a> {
    uid: Option<&'a str>,
    version: Option<&'a str>,
    referrer: Option<&'a str>,
}

fn header_value_lossy<'a>(req: &'a HttpRequest, name: &str) -> Option<&'a str> {
    req.headers().get(name).and_then(|val| val.to_str().ok())
}

impl<'a> TinfoilRequestHeaders<'a> {
    pub fn from_req(req: &'a HttpRequest) -> Self {
        let uid = header_value_lossy(req, "UID");
        let version = header_value_lossy(req, "Version");
        let referrer = header_value_lossy(req, "Referer");

        TinfoilRequestHeaders {
            uid,
            version,
            referrer,
        }
    }
}
