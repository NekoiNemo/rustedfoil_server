use std::{env, io};

use actix_web::{App, dev::ServiceRequest, HttpServer, middleware, Result as WebResult, web};
use actix_web_httpauth::extractors::AuthenticationError;
use actix_web_httpauth::extractors::basic::{BasicAuth, Config as BasicConfig};
use actix_web_httpauth::middleware::HttpAuthentication;
use listenfd::ListenFd;

use auth::AuthService;
use crate::switch::SwitchService;

mod auth;
mod routes;

pub async fn start(switch_service: SwitchService) -> io::Result<()> {
    let auth_service = web::Data::new(AuthService::from_env());

    let switch_service = web::Data::new(switch_service);

    let mut server = HttpServer::new(move || {
        let auth = HttpAuthentication::basic(auth_validator);

        App::new()
            .app_data(auth_service.clone())
            .app_data(switch_service.clone())
            .wrap(middleware::Logger::new(
                "%t | %r | from %a response %s took %Dms",
            ))
            .wrap(middleware::Compress::default())
            .wrap(auth)
            .configure(routes::root)
    });

    let mut listenfd = ListenFd::from_env();
    server = if let Some(tcp_listener) = listenfd.take_tcp_listener(0).unwrap() {
        server
            .listen(tcp_listener)
            .unwrap_or_else(|err| panic!("Failed to start server using TCP listener\n{}", err))
    } else {
        let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = env::var("PORT").expect("PORT env not set");
        let addr = format!("{}:{}", host, port);

        server
            .bind(&addr)
            .unwrap_or_else(|err| panic!("Failed to start server @ {}\n{}", addr, err))
    };

    server.run().await
}

async fn auth_validator(req: ServiceRequest, credentials: BasicAuth) -> WebResult<ServiceRequest> {
    let config = req
        .app_data::<BasicConfig>()
        .map(|data| data.clone())
        .unwrap_or_else(Default::default);

    let auth_service = req.app_data::<web::Data<AuthService>>().unwrap();

    if auth_service.check_credentials(
        credentials.user_id(),
        credentials.password().unwrap().trim(),
    ) {
        Ok(req)
    } else {
        log::info!(r#"Failed login attempt from "{}""#, &credentials.user_id());

        Err(AuthenticationError::from(config).into())
    }
}
