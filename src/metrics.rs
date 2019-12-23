use futures::future::Future;
use hyper::service::service_fn_ok;
use hyper::{Body, Request, Response, Server};
use prometheus::Encoder;
use std::net::SocketAddr;

pub fn serve(bind: &SocketAddr) {
    let server = Server::bind(bind)
        .serve(|| {
            let registry = prometheus::default_registry();
            let encoder = prometheus::TextEncoder::new();

            service_fn_ok(move |_: Request<_>| {
                let mut buffer = Vec::<u8>::new();
                encoder.encode(&registry.gather(), &mut buffer).unwrap();

                Response::new(Body::from(buffer))
            })
        })
        .map_err(|e| eprintln!("Server error: {}", e));
    hyper::rt::run(server);
}
