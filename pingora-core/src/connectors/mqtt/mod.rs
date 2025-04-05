use crate::connectors::ConnectorOptions;

pub mod v3;
pub mod v5;

pub enum Connector {
    V3(v3::Connector),
    V5(v5::Connector),
}

impl Connector {
    pub fn new(opts: Option<ConnectorOptions>) -> Self {
        Connector::V3(v3::Connector{})
    }
}