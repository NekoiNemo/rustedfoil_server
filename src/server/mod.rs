use std::convert::Infallible;
use std::env;
use std::net::IpAddr;
use std::sync::Arc;

use hyper::server::Server;
use listenfd::ListenFd;
use warp::hyper;

use crate::switch::SwitchService;

pub mod file;
mod routes;

pub async fn start(switch_service: SwitchService) {
    let routes = routes::root(Arc::new(switch_service));

    let warp_service = warp::service(routes);

    let hyper_service = hyper::service::make_service_fn(|_: _| {
        let svc = warp_service.clone();
        async move { Ok::<_, Infallible>(svc) }
    });

    let mut listenfd = ListenFd::from_env();
    let server = if let Some(tcp_listener) = listenfd.take_tcp_listener(0).unwrap() {
        Server::from_tcp(tcp_listener).unwrap_or_else(|err| {
            panic!("Failed to start server_actix using TCP listener\n{}", err)
        })
    } else {
        let host: IpAddr = env::var("HOST")
            .unwrap_or_else(|_| "127.0.0.1".to_string())
            .parse()
            .expect("HOST env is not a valid ip addr");
        let port: u16 = env::var("PORT")
            .unwrap_or_else(|_| "9000".to_string())
            .parse()
            .expect("PORT env is not a number");

        Server::bind(&(host, port).into())
    };

    server
        .serve(hyper_service)
        .await
        .expect("Failed to start server")
}
