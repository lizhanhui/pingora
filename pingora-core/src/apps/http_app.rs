// Copyright 2025 Cloudflare, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A simple HTTP application trait that maps a request to a response

use async_trait::async_trait;
use http::Response;
use log::{debug, error, trace};
use pingora_http::ResponseHeader;
use std::future::poll_fn;
use std::sync::Arc;

use crate::apps::ServerApp;
use crate::modules::http::{HttpModules, ModuleBuilder};
use crate::protocols::http::v2::server;
use crate::protocols::http::HttpTask;
use crate::protocols::http::ServerSession;
use crate::protocols::ALPN;
use crate::protocols::{Digest, Stream};
use crate::server::ShutdownWatch;

// https://datatracker.ietf.org/doc/html/rfc9113#section-3.4
const H2_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

#[non_exhaustive]
#[derive(Default)]
/// HTTP Server options that control how the server handles some transport types.
pub struct HttpServerOptions {
    /// Use HTTP/2 for plaintext.
    pub h2c: bool,
}

/// This trait defines the interface of an HTTP application.
#[async_trait]
pub trait HttpServerApp {
    /// Similar to the [`ServerApp`], this function is called whenever a new HTTP session is established.
    ///
    /// After successful processing, [`ServerSession::finish()`] can be called to return an optionally reusable
    /// connection back to the service. The caller needs to make sure that the connection is in a reusable state
    /// i.e., no error or incomplete read or write headers or bodies. Otherwise a `None` should be returned.
    async fn process_new_http(
        self: &Arc<Self>,
        session: ServerSession,
        // TODO: make this ShutdownWatch so that all task can await on this event
        shutdown: &ShutdownWatch,
    ) -> Option<Stream>;

    /// Provide options on how HTTP/2 connection should be established. This function will be called
    /// every time a new HTTP/2 **connection** needs to be established.
    ///
    /// A `None` means to use the built-in default options. See [`server::H2Options`] for more details.
    fn h2_options(&self) -> Option<server::H2Options> {
        None
    }

    /// Provide HTTP server options used to override default behavior. This function will be called
    /// every time a new connection is processed.
    ///
    /// A `None` means no server options will be applied.
    fn server_options(&self) -> Option<&HttpServerOptions> {
        None
    }

    async fn http_cleanup(&self) {}
}

#[async_trait]
impl<T> ServerApp for T
where
    T: HttpServerApp + Send + Sync + 'static,
{
    async fn process_new(
        self: &Arc<Self>,
        mut stream: Stream,
        shutdown: &ShutdownWatch,
    ) -> Option<Stream> {
        let mut h2c = self.server_options().as_ref().map_or(false, |o| o.h2c);

        // try to read h2 preface
        if h2c {
            let mut buf = [0u8; H2_PREFACE.len()];
            let peeked = stream
                .try_peek(&mut buf)
                .await
                .map_err(|e| {
                    // this error is normal when h1 reuse and close the connection
                    debug!("Read error while peeking h2c preface {e}");
                    e
                })
                .ok()?;
            // not all streams support peeking
            if peeked {
                // turn off h2c (use h1) if h2 preface doesn't exist
                h2c = buf == H2_PREFACE;
            }
        }
        if h2c || matches!(stream.selected_alpn_proto(), Some(ALPN::H2)) {
            // create a shared connection digest
            let digest = Arc::new(Digest {
                ssl_digest: stream.get_ssl_digest(),
                // TODO: log h2 handshake time
                timing_digest: stream.get_timing_digest(),
                proxy_digest: stream.get_proxy_digest(),
                socket_digest: stream.get_socket_digest(),
            });

            let h2_options = self.h2_options();
            let h2_conn = server::handshake(stream, h2_options).await;
            let mut h2_conn = match h2_conn {
                Err(e) => {
                    error!("H2 handshake error {e}");
                    return None;
                }
                Ok(c) => c,
            };

            let mut shutdown = shutdown.clone();
            loop {
                // this loop ends when the client decides to close the h2 conn
                // TODO: add a timeout?
                let h2_stream = tokio::select! {
                    _ = shutdown.changed() => {
                        h2_conn.graceful_shutdown();
                        let _ = poll_fn(|cx| h2_conn.poll_closed(cx))
                            .await.map_err(|e| error!("H2 error waiting for shutdown {e}"));
                        return None;
                    }
                    h2_stream = server::HttpSession::from_h2_conn(&mut h2_conn, digest.clone()) => h2_stream
                };
                let h2_stream = match h2_stream {
                    Err(e) => {
                        // It is common for the client to just disconnect TCP without properly
                        // closing H2. So we don't log the errors here
                        debug!("H2 error when accepting new stream {e}");
                        return None;
                    }
                    Ok(s) => s?, // None means the connection is ready to be closed
                };
                let app = self.clone();
                let shutdown = shutdown.clone();
                pingora_runtime::current_handle().spawn(async move {
                    app.process_new_http(ServerSession::new_http2(h2_stream), &shutdown)
                        .await;
                });
            }
        } else {
            // No ALPN or ALPN::H1 and h2c was not configured, fallback to HTTP/1.1
            self.process_new_http(ServerSession::new_http1(stream), shutdown)
                .await
        }
    }

    async fn cleanup(&self) {
        self.http_cleanup().await;
    }
}

/// This trait defines how to map a request to a response
#[async_trait]
pub trait ServeHttp {
    /// Define the mapping from a request to a response.
    /// Note that the request header is already read, but the implementation needs to read the
    /// request body if any.
    ///
    /// # Limitation
    /// In this API, the entire response has to be generated before the end of this call.
    /// So it is not suitable for streaming response or interactive communications.
    /// Users need to implement their own [`super::HttpServerApp`] for those use cases.
    async fn response(&self, http_session: &mut ServerSession) -> Response<Vec<u8>>;
}

// TODO: remove this in favor of HttpServer?
#[async_trait]
impl<SV> HttpServerApp for SV
where
    SV: ServeHttp + Send + Sync,
{
    async fn process_new_http(
        self: &Arc<Self>,
        mut http: ServerSession,
        shutdown: &ShutdownWatch,
    ) -> Option<Stream> {
        match http.read_request().await {
            Ok(res) => match res {
                false => {
                    debug!("Failed to read request header");
                    return None;
                }
                true => {
                    debug!("Successfully get a new request");
                }
            },
            Err(e) => {
                error!("HTTP server fails to read from downstream: {e}");
                return None;
            }
        }
        trace!("{:?}", http.req_header());
        if *shutdown.borrow() {
            http.set_keepalive(None);
        } else {
            http.set_keepalive(Some(60));
        }
        let new_response = self.response(&mut http).await;
        let (parts, body) = new_response.into_parts();
        let resp_header: ResponseHeader = parts.into();
        match http.write_response_header(Box::new(resp_header)).await {
            Ok(()) => {
                debug!("HTTP response header done.");
            }
            Err(e) => {
                error!(
                    "HTTP server fails to write to downstream: {e}, {}",
                    http.request_summary()
                );
            }
        }
        if !body.is_empty() {
            // TODO: check if chunked encoding is needed
            match http.write_response_body(body.into(), true).await {
                Ok(_) => debug!("HTTP response written."),
                Err(e) => error!(
                    "HTTP server fails to write to downstream: {e}, {}",
                    http.request_summary()
                ),
            }
        }
        match http.finish().await {
            Ok(c) => c,
            Err(e) => {
                error!("HTTP server fails to finish the request: {e}");
                None
            }
        }
    }
}

/// A helper struct for HTTP server with http modules embedded
pub struct HttpServer<SV> {
    app: SV,
    modules: HttpModules,
}

impl<SV> HttpServer<SV> {
    /// Create a new [HttpServer] with the given app which implements [ServeHttp]
    pub fn new_app(app: SV) -> Self {
        HttpServer {
            app,
            modules: HttpModules::new(),
        }
    }

    /// Add [ModuleBuilder] to this [HttpServer]
    pub fn add_module(&mut self, module: ModuleBuilder) {
        self.modules.add_module(module)
    }
}

#[async_trait]
impl<SV> HttpServerApp for HttpServer<SV>
where
    SV: ServeHttp + Send + Sync,
{
    async fn process_new_http(
        self: &Arc<Self>,
        mut http: ServerSession,
        shutdown: &ShutdownWatch,
    ) -> Option<Stream> {
        match http.read_request().await {
            Ok(res) => match res {
                false => {
                    debug!("Failed to read request header");
                    return None;
                }
                true => {
                    debug!("Successfully get a new request");
                }
            },
            Err(e) => {
                error!("HTTP server fails to read from downstream: {e}");
                return None;
            }
        }
        trace!("{:?}", http.req_header());
        if *shutdown.borrow() {
            http.set_keepalive(None);
        } else {
            http.set_keepalive(Some(60));
        }
        let mut module_ctx = self.modules.build_ctx();
        let req = http.req_header_mut();
        module_ctx.request_header_filter(req).await.ok()?;
        let new_response = self.app.response(&mut http).await;
        let (parts, body) = new_response.into_parts();
        let mut resp_header: ResponseHeader = parts.into();
        module_ctx
            .response_header_filter(&mut resp_header, body.is_empty())
            .await
            .ok()?;

        let task = HttpTask::Header(Box::new(resp_header), body.is_empty());
        trace!("{task:?}");

        match http.response_duplex_vec(vec![task]).await {
            Ok(_) => {
                debug!("HTTP response header done.");
            }
            Err(e) => {
                error!(
                    "HTTP server fails to write to downstream: {e}, {}",
                    http.request_summary()
                );
            }
        }

        let mut body = Some(body.into());
        module_ctx.response_body_filter(&mut body, true).ok()?;

        let task = HttpTask::Body(body, true);

        trace!("{task:?}");

        // TODO: check if chunked encoding is needed
        match http.response_duplex_vec(vec![task]).await {
            Ok(_) => debug!("HTTP response written."),
            Err(e) => error!(
                "HTTP server fails to write to downstream: {e}, {}",
                http.request_summary()
            ),
        }
        match http.finish().await {
            Ok(c) => c,
            Err(e) => {
                error!("HTTP server fails to finish the request: {e}");
                None
            }
        }
    }
}
