//! # Services Connector
//!
//! Services Connector is a library that provides a way to connect between services,
//! such as Proxy, Storage, and others as described in enum `ApplicationType`.
//!
//! ## Usage
//!
//! To use Services Connector, you need to create a new instance of `CommandEngine` and `ReceiverEngine`.
//! ```rust
//! let redis_string = "redis://:boul2gom@127.0.0.1:6379".to_string();
//!
//! let mut command = CommandEngine::new(redis_string.clone()).await?;
//! let mut receiver = ReceiverEngine::new(ApplicationType::Proxy, redis_string.clone()).await?;
//! ```
//! Here, two Redis connections are created: one for sending messages, and one for receiving messages.
//! To create the receiving connection, you need to specify the application type that is running.
//! It will be used to determine which messages to receive and process.
//!
//! After creating the engines, you need to subscribe to the channel, used for messages transfer.
//! ```rust
//! receiver.subscribe(CHANNEL_NAME).await?;
//! ```
//!
//! After that, you can start receiving messages.
//! ```rust
//! receiver
//!    .start(
//!       |message| {
//!          log::info!("Received message: {:?}", message);
//!      },
//!     |error| {
//!        log::error!("An error occurred: {:?}", error);
//!    },
//! )
//! .await?;
//! ```
//! The `start` method takes two closures as arguments: one for processing messages, and one for processing errors.
//! A handler system will be implemented in the future to make it easier to process messages, by registering handlers
//! for each message type.
//!
//! Sending messages is done using the `CommandEngine` instance.
//! ```rust
//! let handshake = HandshakePacket {
//!    application_name: "Proxy - 1".to_string(),
//! }
//!
//! let packet = PacketBuilder::from_packet(handshake, ApplicationType::Storage, ApplicationType::Proxy);
//! let message = packet.write()?;
//!
//! command.publish(message).await?;
//! ```
//!
//! A complete example can be found below.
//! ```rust
//! use std::env;
//!
//! use services_connector::{protocol_engine::redis_engine::{CommandEngine, ReceiverEngine}, packets::{ApplicationType, CHANNEL_NAME}};
//! use tokio::runtime::Builder;
//!
//! pub fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     env::set_var("RUST_LOG", "info");
//!     env_logger::try_init().expect("Failed to initialize logger");
//!
//!     let worker_threads = env::var("WORKER_THREADS")
//!         .unwrap_or_else(|_| "8".to_string())
//!         .parse::<usize>()
//!         .unwrap();
//!
//!     log::info!(
//!         "Starting connector with {} worker threads...",
//!         worker_threads
//!     );
//!
//!     let runtime = Builder::new_multi_thread()
//!         .thread_name("rust-api-worker")
//!         .worker_threads(worker_threads)
//!         .enable_all()
//!         .build()
//!         .expect("Failed to create Tokio runtime");
//!
//!     runtime.block_on(async_bootstrap())
//! }
//!
//! pub async fn async_bootstrap() -> Result<(), Box<dyn std::error::Error>> {
//!     log::info!("Starting async bootstrap...");
//!     let redis_string = "redis://:boul2gom@127.0.0.1:6379".to_string();
//!
//!     let mut command = CommandEngine::new(redis_string.clone()).await?;
//!
//!     let mut receiver = ReceiverEngine::new(ApplicationType::Proxy, redis_string.clone()).await?;
//!     receiver.subscribe(CHANNEL_NAME).await?;
//!
//!     receiver
//!         .start(
//!             |message| {
//!                 log::info!("Received message: {:?}", message);
//!                 
//!             },
//!             |error| {
//!                 log::error!("An error occurred: {:?}", error);
//!             },
//!         )
//!         .await?;
//!
//!     log::info!("Async bootstrap complete. Waiting for packets...");
//!
//!     let handshake = HandshakePacket {
//!         application_name: "Proxy - 1".to_string(),
//!     }
//!
//!     let packet = PacketBuilder::from_packet(handshake);
//!     let message = packet.write()?;
//!
//!     command.publish(message).await?;
//!
//!
//!     Ok(())
//! }
//! ```
pub mod packets;
pub mod protocol_engine;
