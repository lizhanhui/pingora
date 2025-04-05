use crate::Proxy;
use pingora_core::apps::http_app::HttpServerOptions;
use pingora_core::connectors::mqtt::Connector;
use pingora_core::connectors::ConnectorOptions;
use pingora_core::modules::http::HttpModules;
use pingora_core::server::configuration::ServerConf;
use pingora_core::services::listening::Service;
use std::sync::Arc;
use tokio::sync::Notify;

/// The concrete type that holds the user defined MQTT proxy.
///
/// Users don't need to interact with this object directly.
pub struct MqttProxy<SV> {
    inner: SV, // TODO: name it better than inner
    upstream: Connector,
    shutdown: Notify,
    pub server_options: Option<HttpServerOptions>,
    pub downstream_modules: HttpModules,
    max_retries: usize,
}

impl<SV> MqttProxy<SV> {
    fn new(inner: SV, conf: Arc<ServerConf>) -> Self {
        Self {
            inner,
            upstream: Connector::new(Some(ConnectorOptions::from_server_conf(&conf))),
            shutdown: Notify::new(),
            server_options: None,
            downstream_modules: HttpModules::new(),
            max_retries: conf.max_retries,
        }
    }

    fn handle_init_modules(&mut self)
    where
        SV: Proxy,
    {
        self.inner
            .init_downstream_modules(&mut self.downstream_modules);
    }
}

/// Create a [Service] from the user implemented [Proxy].
///
/// The returned [Service] can be hosted by a [pingora_core::server::Server] directly.
pub fn mqtt_proxy_service<SV>(conf: &Arc<ServerConf>, inner: SV) -> Service<MqttProxy<SV>>
where
    SV: Proxy,
{
    mqtt_proxy_service_with_name(conf, inner, "Pingora HTTP Proxy Service")
}

/// Create a [Service] from the user implemented [Proxy].
///
/// The returned [Service] can be hosted by a [pingora_core::server::Server] directly.
pub fn mqtt_proxy_service_with_name<SV>(
    conf: &Arc<ServerConf>,
    inner: SV,
    name: &str,
) -> Service<MqttProxy<SV>>
where
    SV: Proxy,
{
    let mut proxy = MqttProxy::new(inner, conf.clone());
    proxy.handle_init_modules();
    Service::new(name.to_string(), proxy)
}
