use serde::{Deserialize, Serialize};

use crate::packets::{Packet, Protocol};

/// The handshake packet is sent by the client to the server to establish a connection.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HandshakePacket {
    /// The name of the application that is running.
    pub application_name: String,
}

impl Packet for HandshakePacket {
    fn get_id(&self) -> Protocol {
        Protocol::Handshake
    }
}
