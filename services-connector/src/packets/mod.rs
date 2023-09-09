//! This module contains the packets used to communicate between the server and the services.
pub mod builder;
pub mod model;

use derive_more::Display;
use enum_assoc::Assoc;
use serde::{Deserialize, Serialize};

/// The version of the protocol used, in the format `[Major].[Minor].[Patch]`.
pub const PROTOCOL_VERSION: [u8; 3] = [0x01, 0x00, 0x00]; // 1.0.0
/// The name of the channel used to send and receive packets.
pub const CHANNEL_NAME: &str = "service-connector";
/// The length of the packet id.
pub const PACKET_ID_LENGTH: usize = 8;

/// The protocol used, also known as the packet id.
#[derive(Assoc, Clone, Debug, Display)]
#[func(pub const fn get_id(&self) -> u8)]
pub enum Protocol {
    /// The handshake packet is sent by the service to the server to establish a connection.
    #[display(fmt = "Handshake")]
    #[assoc(get_id = 0x00)]
    Handshake,
    /// The ping packet is sent by the server to the service to check if the connection is still alive.
    #[display(fmt = "Ping")]
    #[assoc(get_id = 0x01)]
    Ping,
    /// The alive packet is sent by the service to the server to indicate that the service is still alive.
    #[display(fmt = "Alive")]
    #[assoc(get_id = 0x02)]
    Alive,
    /// The register packet is sent by the service to the server to register the new instance.
    #[display(fmt = "Register")]
    #[assoc(get_id = 0x03)]
    Register,

    /// The unknown packet is used when the packet id is unknown.
    #[display(fmt = "Unknown")]
    #[assoc(get_id = 0xFF)]
    Unknown,
}

/// The application that sent the packet.
#[derive(Assoc, Clone, Debug, Display, PartialEq, Eq)]
#[func(pub const fn get_id(&self) -> u8)]
pub enum ApplicationType {
    /// The auth application that is used to authenticate users.
    #[display(fmt = "Auth")]
    #[assoc(get_id = 0x00)]
    Auth,
    /// The proxy application that is used to proxy packets between the client and the services.
    #[display(fmt = "Proxy")]
    #[assoc(get_id = 0x01)]
    Proxy,
    /// The storage application that is used to store data.
    #[display(fmt = "Storage")]
    #[assoc(get_id = 0x02)]
    Storage,
    /// The manager application that is used to manage the services.
    #[display(fmt = "Manager")]
    #[assoc(get_id = 0x03)]
    Manager,

    /// The debug client that is used to debug the services.
    #[display(fmt = "Client")]
    #[assoc(get_id = 0x98)]
    Client,
    /// The bot client that is used to control the services.
    #[display(fmt = "Bot")]
    #[assoc(get_id = 0x99)]
    Bot,

    /// The unknown application is used when the application id is unknown.
    #[display(fmt = "Unknown")]
    #[assoc(get_id = 0xFF)]
    Unknown,
}

/// The packet trait is used to serialize and deserialize packets.
///
/// # Example
///
/// ```rust
/// pub struct PingPacket {
///    pub timestamp: u64,
/// }
///
/// impl Packet for PingPacket {
///    fn get_id(&self) -> Protocol {
///       Protocol::Ping
///   }
/// }
/// ```
pub trait Packet: Serialize + for<'a> Deserialize<'a> {
    /// Returns the protocol used.
    fn get_id(&self) -> Protocol;

    /// Returns the packet serialized as bytes.
    fn as_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let serialized = serde_json::to_string(self)?;
        let bytes = serialized.as_bytes().to_owned();

        Ok(bytes)
    }
}

impl Protocol {
    /// Returns the protocol from the specified id.
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the protocol.
    ///
    /// # Example
    ///
    /// ```rust
    /// let protocol = Protocol::from_id(0x00);
    /// ```
    pub fn from_id(id: u8) -> Protocol {
        match id {
            0x00 => Protocol::Handshake,
            0x01 => Protocol::Ping,
            0x02 => Protocol::Alive,
            0x03 => Protocol::Register,

            _ => Protocol::Unknown,
        }
    }
}

impl ApplicationType {
    /// Returns the application type from the specified id.
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the application type.
    ///
    /// # Example
    ///
    /// ```rust
    /// let application_type = ApplicationType::from_id(0x00);
    /// ```
    pub fn from_id(id: u8) -> ApplicationType {
        match id {
            0x00 => ApplicationType::Auth,
            0x01 => ApplicationType::Proxy,
            0x02 => ApplicationType::Storage,
            0x03 => ApplicationType::Manager,

            0x98 => ApplicationType::Client,
            0x99 => ApplicationType::Bot,

            _ => ApplicationType::Unknown,
        }
    }
}
