use bytes::Bytes;
use futures::StreamExt;
use redis::{
    aio::{Connection, PubSub},
    AsyncCommands, Client,
};
use tokio::task::JoinHandle;

use crate::packets::{builder::PacketBuilder, ApplicationType, CHANNEL_NAME};

/// Connects to the Redis server. This method is shared by the two types of engines.
///
/// # Arguments
///
/// * `url` - The URL of the Redis server.
///
/// # Example
///
/// ```rust
/// let (redis_client, redis_connection) = self::connect_engine(url.clone()).await?;
/// ```
async fn connect_engine(url: String) -> Result<(Client, Connection), Box<dyn std::error::Error>> {
    let redis_client = Client::open(url)?;
    let redis_connection = redis_client.get_async_connection().await?;

    Ok((redis_client, redis_connection))
}

/// The command engine is used to send commands to the server.
pub struct CommandEngine {
    pub url: String,
    pub redis_client: Client,
    pub connection: Connection,
}

/// The receiver engine is used to receive packets from the server.
pub struct ReceiverEngine {
    pub app_type: ApplicationType,
    pub url: String,
    pub redis_client: Client,

    pub broker: PubSub,
}

impl CommandEngine {
    /// Creates a new command engine.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL of the Redis server.
    ///
    /// # Example
    ///
    /// ```rust
    /// let engine = CommandEngine::new(url.clone()).await?;
    /// ```
    pub async fn new(url: String) -> Result<CommandEngine, Box<dyn std::error::Error>> {
        let (redis_client, connection) = self::connect_engine(url.clone()).await?;

        Ok(Self {
            url,
            redis_client,
            connection,
        })
    }

    /// Publishes the specified message to the server.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to publish.
    ///
    /// # Example
    ///
    /// ```rust
    /// let handshake = HandshakePacket {
    ///  application_name: "Proxy - 1".to_string(),
    /// }
    ///
    /// let packet = PacketBuilder::from_packet(handshake, ApplicationType::Storage, ApplicationType::Proxy);
    /// let message = packet.as_bytes()?;
    ///
    /// engine.publish(message).await?;
    /// ```
    pub async fn publish(&mut self, message: Bytes) -> Result<(), Box<dyn std::error::Error>> {
        self.raw_publish(&message).await
    }

    /// Publishes the specified message to the server, of any byte slice.
    /// Be careful when using this method, as it does not check if the message is a valid packet,
    /// which was created using the `PacketBuilder`.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to publish.
    ///
    /// # Example
    ///
    /// ```rust
    /// let message = b"Hello world!";
    ///
    /// engine.raw_publish(message).await?;
    /// ```
    pub async fn raw_publish(&mut self, message: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        self.connection.publish(CHANNEL_NAME, message).await?;

        Ok(())
    }
}

impl ReceiverEngine {
    /// Creates a new receiver engine.
    ///
    /// # Arguments
    ///
    /// * `app_type` - The application type of the receiver, used to filter packets.
    /// * `url` - The URL of the Redis server.
    ///
    /// # Example
    ///
    /// ```rust
    /// let engine = ReceiverEngine::new(ApplicationType::Proxy, url.clone()).await?;
    /// ```
    pub async fn new(
        app_type: ApplicationType,
        url: String,
    ) -> Result<ReceiverEngine, Box<dyn std::error::Error>> {
        let (redis_client, connection) = self::connect_engine(url.clone()).await?;
        let broker = connection.into_pubsub();

        Ok(Self {
            app_type,
            url,
            redis_client,
            broker,
        })
    }

    /// Subscribes to the specified channel.
    ///
    /// # Arguments
    ///
    /// * `channel` - The channel to subscribe to.
    ///
    /// # Example
    ///
    /// ```rust
    /// engine.subscribe("channel").await?;
    /// ```
    pub async fn subscribe(&mut self, channel: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.broker.subscribe(channel).await?;

        Ok(())
    }

    /// Starts the receiver engine, and calls the specified callback when a packet is received.
    /// This method will return a `JoinHandle` that can be used to await the task.
    /// Be careful, this method will run forever, until the task is cancelled, in another thread.
    ///
    /// # Arguments
    ///
    /// * `callback` - The callback that will be called when a packet is received.
    /// * `callback_error` - The callback that will be called when an error occurs.
    ///
    /// # Example
    ///
    /// ```rust
    /// let handle = engine.start(|packet| {
    ///    println!("Packet received: {:?}", packet);
    /// }, |error| {
    ///   println!("Error: {:?}", error);
    /// }).await?;
    /// ```
    pub async fn start<T, E>(
        mut self,
        callback: T,
        callback_error: E,
    ) -> Result<JoinHandle<()>, Box<dyn std::error::Error>>
    where
        T: Fn(PacketBuilder) + std::marker::Send + 'static,
        E: Fn(Box<dyn std::error::Error>) + std::marker::Send + 'static,
    {
        let handle = tokio::spawn(async move {
            let callback = callback;
            let callback_error = callback_error;

            let mut stream = self.broker.on_message();
            while let Some(message) = stream.next().await {
                let payload = message.get_payload_bytes();

                let packet = PacketBuilder::from_bytes(payload);
                if let Err(e) = packet {
                    callback_error(e);
                    continue;
                }

                let packet = packet.expect("Packet is not an error, this should never happen");
                if packet.receiver != self.app_type {
                    continue;
                }

                callback(packet);
            }
        });

        Ok(handle)
    }
}
