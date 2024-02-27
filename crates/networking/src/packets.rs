use crate::errors::DeserializationError;
use bytemuck::{bytes_of, from_bytes, pod_read_unaligned, Pod};
use log::error;
use std::collections::HashMap;
use std::mem;

pub type PacketId = u8;
pub type ByteBuf = Box<[u8]>;
pub trait Packet: Sized {
    const ID: PacketId;

    fn get_writing_byte_buff(capacity: usize) -> WritingByteBuf {
        let mut data = Vec::with_capacity(capacity + mem::size_of::<PacketId>());
        data.extend_from_slice(bytes_of(&Self::ID)); //for u16 compatibility
        WritingByteBuf { data }
    }

    fn serialize(self) -> WritingByteBuf;

    fn deserialize(data: ReadingByteBuf) -> Result<Self, DeserializationError>;
}

trait PacketHandler {
    fn handle_packet(&self, data: ReadingByteBuf);
}

struct PacketHandlerImpl<PacketType, CallBack>
where
    PacketType: Packet,
    CallBack: Fn(PacketType) -> (),
{
    callback: CallBack,
    phantom: std::marker::PhantomData<PacketType>,
}

impl<PacketType, CallBack> PacketHandler for PacketHandlerImpl<PacketType, CallBack>
where
    PacketType: Packet,
    CallBack: Fn(PacketType) -> (),
{
    fn handle_packet(&self, data: ReadingByteBuf) {
        let packet = PacketType::deserialize(data).unwrap();
        (self.callback)(packet);
    }
}

pub struct Dispatcher {
    handlers: HashMap<PacketId, Box<dyn PacketHandler>>,
}

impl Dispatcher {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register_handler<PacketType, CallBack>(&mut self, callback: CallBack)
    where
        PacketType: Packet + 'static,
        CallBack: Fn(PacketType) -> () + 'static,
    {
        assert!(self.handlers.get(&PacketType::ID).is_none());
        let handler = PacketHandlerImpl {
            callback,
            phantom: std::marker::PhantomData,
        };
        self.handlers.insert(PacketType::ID, Box::new(handler));
    }

    pub fn dispatch_packet(&self, data: ByteBuf) {
        let data = ReadingByteBuf::new(data);
        let id = data.get_packet_id();
        let handler = self.handlers.get(&id);
        if let Some(handler) = handler {
            handler.handle_packet(data);
        } else {
            error!("unknown packet received {}", id);
        }
    }
}

pub struct WritingByteBuf {
    data: Vec<u8>,
}

impl WritingByteBuf {
    pub fn write<T>(&mut self, value: T)
    where
        T: Pod,
    {
        self.data.extend_from_slice(bytes_of(&value));
    }

    pub fn write_bytes(&mut self, value: &[u8]) {
        self.data.extend_from_slice(value);
    }
}

impl From<WritingByteBuf> for ByteBuf {
    fn from(buf: WritingByteBuf) -> Self {
        buf.data.into_boxed_slice()
    }
}

pub struct ReadingByteBuf {
    data: Box<[u8]>,
    offset: usize,
}

impl ReadingByteBuf {
    fn new(data: Box<[u8]>) -> Self {
        Self {
            data,
            offset: mem::size_of::<PacketId>(),
        }
    }

    fn get_packet_id(&self) -> PacketId {
        let id = &self.data[0..mem::size_of::<PacketId>()];
        *from_bytes::<PacketId>(id)
    }

    pub fn read<T>(&mut self) -> Result<T, DeserializationError>
    where
        T: Pod,
    {
        let type_size = mem::size_of::<T>();
        if self.offset + type_size > self.data.len() {
            return Err(DeserializationError::NotEnoughBytes);
        }

        let slice = &self.data[self.offset..self.offset + type_size];
        assert_eq!(slice.len(), type_size);
        self.offset += type_size;
        Ok(pod_read_unaligned(slice))
    }

    pub fn read_bytes(&mut self, size: usize) -> Result<&[u8], DeserializationError> {
        if self.offset + size > self.data.len() {
            return Err(DeserializationError::NotEnoughBytes);
        }
        let slice = &self.data[self.offset..self.offset + size];
        self.offset += size;
        Ok(slice)
    }
}
