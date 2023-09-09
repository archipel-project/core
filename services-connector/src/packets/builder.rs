use std::u8;

use super::{ApplicationType, Packet, Protocol, PACKET_ID_LENGTH, PROTOCOL_VERSION};
use bytes::{Buf, BufMut, Bytes, BytesMut};

/// The packet builder is used to create packets that can be sent over the network.
#[derive(Clone, Debug)]
pub struct PacketBuilder {
    /// The version of the protocol used.
    pub version: [u8; 3],
    /// The protocol used, also known as the packet id. See `Protocol` for more information.
    pub protocol: Protocol,

    /// The application that should receive the packet.
    pub receiver: ApplicationType,
    /// The application that sent the packet.
    pub sender: ApplicationType,

    /// The id of the packet. This is used to identify the packet when it is received.
    pub id: String,

    /// Whether or not the packet expects a response.
    pub response_expected: bool,
    /// Whether or not the packet is a response.
    pub is_response: bool,

    /// The payload of the packet.
    pub payload: BytesMut,
}

impl PacketBuilder {
    /// Creates a new packet builder with the specified protocol, receiver and sender.
    /// The id of the packet is generated automatically.
    ///
    /// # Arguments
    ///
    /// * `protocol` - The protocol used.
    /// * `receiver` - The application that should receive the packet.
    /// * `sender` - The application that sent the packet.
    ///
    /// # Example
    ///
    /// ```rust
    /// let packet = PacketBuilder::new(Protocol::Handshake, ApplicationType::Storage, ApplicationType::Proxy);
    /// ```
    pub fn new(
        protocol: Protocol,
        receiver: ApplicationType,
        sender: ApplicationType,
    ) -> PacketBuilder {
        let id = uuid::Uuid::new_v4().to_string();
        let id = id.split_at(PACKET_ID_LENGTH).0.to_string();
        let payload = BytesMut::new();

        PacketBuilder {
            version: PROTOCOL_VERSION,
            protocol,
            receiver,
            sender,
            id,
            response_expected: true,
            is_response: false,
            payload,
        }
    }

    /// Creates a new packet builder from the specified packet.
    ///
    /// # Arguments
    ///
    /// * `packet` - The packet to create the builder from.
    /// * `receiver` - The application that should receive the packet.
    /// * `sender` - The application that sent the packet.
    ///
    /// # Example
    ///
    /// ```rust
    /// let handshake = HandshakePacket {
    ///   application_name: "Proxy - 1".to_string(),
    /// }
    ///
    /// let packet = PacketBuilder::from_packet(handshake, ApplicationType::Storage, ApplicationType::Proxy);
    /// ```
    pub fn from_packet<T: Packet>(
        packet: T,
        receiver: ApplicationType,
        sender: ApplicationType,
    ) -> Result<PacketBuilder, Box<dyn std::error::Error>> {
        let mut builder = PacketBuilder::new(packet.get_id(), receiver, sender);

        builder.add_payload(&packet.as_bytes()?);

        Ok(builder)
    }

    /// Manipulates the payload of the packet, using the specified consumer.
    /// This can be used to add data to the payload, or to modify it.
    ///
    /// # Arguments
    ///
    /// * `consumer` - The consumer that will manipulate the payload.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut packet = PacketBuilder::new(Protocol::Handshake, ApplicationType::Storage, ApplicationType::Proxy);
    /// packet.with_payload(|payload| {
    ///    payload.put_u8(1);
    /// });
    /// ```
    pub fn with_payload<T: FnMut(&mut BytesMut)>(&mut self, mut consumer: T) -> &mut PacketBuilder {
        consumer(&mut self.payload);

        self
    }

    /// Adds the specified bytes to the payload of the packet.
    /// This is a convenience method for `with_payload`.
    ///
    /// # Arguments
    ///
    /// * `payload` - The payload to add to the packet.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut packet = PacketBuilder::new(Protocol::Handshake, ApplicationType::Storage, ApplicationType::Proxy);
    /// packet.add_payload(b"Hello world!");
    /// ```
    pub fn add_payload(&mut self, payload: &[u8]) -> &mut PacketBuilder {
        self.payload.put_slice(payload);

        self
    }

    /// Sets if the packet expects a response.
    ///
    /// # Arguments
    ///
    /// * `response_expected` - Whether or not the packet expects a response.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut packet = PacketBuilder::new(Protocol::Handshake, ApplicationType::Storage, ApplicationType::Proxy);
    /// packet.expect_response(true);
    /// ```
    pub fn expect_response(&mut self, response_expected: bool) -> &mut PacketBuilder {
        self.response_expected = response_expected;

        self
    }

    /// Sets if the packet is a response.
    ///
    /// # Arguments
    ///
    /// * `is_response` - Whether or not the packet is a response.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut packet = PacketBuilder::new(Protocol::Handshake, ApplicationType::Storage, ApplicationType::Proxy);
    /// packet.set_as_response(true);
    /// ```
    pub fn set_as_response(&mut self, is_response: bool) -> &mut PacketBuilder {
        self.is_response = is_response;

        self
    }

    /// Writes the packet to a byte array, which can be sent over the network.
    /// Be aware that this method consumes the packet builder.
    ///
    /// # Example
    ///
    /// ```rust
    /// let packet = PacketBuilder::new(Protocol::Handshake, ApplicationType::Storage, ApplicationType::Proxy);
    /// let bytes = packet.write();
    /// ```
    pub fn write(self) -> Bytes {
        let mut buffer = BytesMut::new();

        buffer.put(&self.version[..]);
        buffer.put_u8(self.protocol.get_id());

        buffer.put_u8(self.receiver.get_id());
        buffer.put_u8(self.sender.get_id());

        buffer.put_slice(self.id.as_bytes());

        buffer.put_u8(self.response_expected as u8);
        buffer.put_u8(self.is_response as u8);

        buffer.put_slice(&self.payload[..]);

        buffer.freeze()
    }

    /// Creates a packet builder from the specified bytes.
    /// This method is used to parse packets that are received.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The bytes to parse.
    ///
    /// # Example
    ///
    /// ```rust
    /// let bytes = b"A very long byte array, in the right format, that contains data";
    /// let packet = PacketBuilder::from_bytes(bytes);
    /// ```
    pub fn from_bytes(bytes: &[u8]) -> Result<PacketBuilder, Box<dyn std::error::Error>> {
        let mut buffer = BytesMut::from(bytes);

        let major_version = buffer.get_u8();
        let minor_version = buffer.get_u8();
        let patch_version = buffer.get_u8();
        let version = [major_version, minor_version, patch_version];

        let protocol = Protocol::from_id(buffer.get_u8());

        let receiver = ApplicationType::from_id(buffer.get_u8());
        let sender = ApplicationType::from_id(buffer.get_u8());

        let id = buffer
            .get(0..PACKET_ID_LENGTH)
            .ok_or("Failed to parse UUID from packet. Make sure the packet is valid")?;
        let id = String::from_utf8(id.to_vec())?;
        buffer.advance(PACKET_ID_LENGTH);

        let response_expected = buffer.get_u8() != 0;
        let is_response = buffer.get_u8() != 0;

        let payload = buffer
            .get(0..buffer.remaining())
            .ok_or("Failed to parse payload from packet. Make sure the packet is valid")?;

        Ok(PacketBuilder {
            version,
            protocol,
            receiver,
            sender,
            id,
            response_expected,
            is_response,
            payload: BytesMut::from(payload),
        })
    }
}
