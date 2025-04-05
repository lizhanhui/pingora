use crate::protocols::l4::socket::SocketAddr;
use crate::protocols::Digest;
use bytes::Bytes;
use pingora_error::Result;
use pingora_http::{RequestHeader, ResponseHeader};

pub struct MqttSession {}

impl MqttSession {
    pub(crate) fn req_header(&self) -> &RequestHeader {
        todo!()
    }

    pub(crate) fn req_header_mut(&self) -> &mut RequestHeader {
        todo!()
    }

    pub(crate) async fn read_body_bytes(&self) -> Result<Option<Bytes>> {
        todo!()
    }

    pub(crate) fn write_response_header(
        &self,
        _response_header: Box<ResponseHeader>,
        _end: bool,
    ) -> Result<()> {
        todo!()
    }

    pub(crate) fn write_response_header_ref(
        &self,
        _response_header: &ResponseHeader,
        _end: bool,
    ) -> Result<()> {
        todo!()
    }

    pub(crate) async fn write_body(&self, _data: Bytes, _end: bool) -> Result<()> {
        todo!()
    }

    pub(crate) fn request_summary(&self) -> String {
        todo!()
    }

    pub(crate) fn shutdown(&self) {
        todo!()
    }

    pub(crate) fn get_retry_buffer(&self) -> Option<Bytes> {
        todo!()
    }

    pub(crate) async fn read_body_or_idle(&self, _no_body_expected: bool) -> Result<Option<Bytes>> {
        todo!()
    }

    pub(crate) fn body_bytes_sent(&self) -> usize {
        todo!()
    }

    pub(crate) fn digest_mut(&self) -> Option<&mut Digest> {
        todo!()
    }

    pub(crate) fn body_bytes_read(&self) -> usize {
        todo!()
    }

    pub(crate) fn digest(&self) -> Option<&Digest> {
        todo!()
    }

    pub(crate) fn client_addr(&self) -> Option<&SocketAddr> {
        todo!()
    }

    pub(crate) fn server_addr(&self) -> Option<&SocketAddr> {
        todo!()
    }
}
