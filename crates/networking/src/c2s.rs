use std::mem;
use crate::errors::DeserializationError;
use crate::packets::{Packet, PacketId, ReadingByteBuf, WritingByteBuf};

pub struct ChatPacket {
    pub message: String,
}

impl Packet for ChatPacket {
    const ID: PacketId = 0;
    fn serialize(self) -> WritingByteBuf {
        let bytes = self.message.as_bytes();
        let len = bytes.len();
        let mut buf = Self::get_writing_byte_buff(len + mem::size_of::<usize>());
        buf.write(len);
        buf.write_bytes(bytes);
        buf
    }

    fn deserialize(mut buf: ReadingByteBuf) -> Result<Self, DeserializationError> {
        let len = buf.read::<usize>()?;
        let message_bytes = buf.read_bytes(len)?;
        let message = std::str::from_utf8(message_bytes).map_err(|_| DeserializationError::InvalidPacketContent)?;
        let message = message.to_string();
        Ok(Self {
            message
        })
    }
}