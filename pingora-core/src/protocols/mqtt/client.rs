use crate::protocols::l4::socket::SocketAddr;
use crate::protocols::Digest;
use bytes::Bytes;
use pingora_error::Result;
use std::time::Duration;

pub struct MqttSession {
    pub(crate) read_timeout: Option<Duration>,
}

impl MqttSession {
    pub async fn write_request(&mut self, _data: Bytes, _end: bool) -> Result<()> {
        todo!()
    }

    pub(crate) fn finish_request(&self) -> Result<()> {
        todo!()
    }

    pub async fn read_response(&mut self) -> Result<Option<Bytes>> {
        todo!()
    }

    pub fn shutdown(&self) {
        todo!()
    }

    pub fn digest(&self) -> Option<&Digest> {
        todo!()
    }

    pub(crate) fn digest_mut(&self) -> Option<&mut Digest> {
        todo!()
    }

    pub(crate) fn server_addr(&self) -> Option<&SocketAddr> {
        todo!()
    }

    pub(crate) fn client_addr(&self) -> Option<&SocketAddr> {
        todo!()
    }
}
