use crate::apps::ServerApp;
use crate::protocols::Stream;
use crate::server::ShutdownWatch;
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait MqttServerApp {
    async fn mqtt_cleanup(&self) {}
}
