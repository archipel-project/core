use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, SystemTime};
use renet::{DefaultChannel, RenetServer, ServerEvent};
use renet::transport::{NetcodeServerTransport, NetcodeTransportError, ServerAuthentication, ServerConfig};

pub struct ServerNetworkHandler {
    packet_transporter: NetcodeServerTransport,
    renet_server: RenetServer,
}

impl ServerNetworkHandler {
    pub fn new(server_address : SocketAddr) -> anyhow::Result<Self> {
        let udp_socket = UdpSocket::bind(server_address)?;
        let server_config = ServerConfig{
            current_time: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap(),
            max_clients: 64,
            protocol_id: 0,
            public_addresses: vec![server_address],
            authentication: ServerAuthentication::Unsecure,
        };
        let packet_transporter = NetcodeServerTransport::new(server_config, udp_socket)?;

        let renet_server = RenetServer::new(Default::default());

        Ok(Self {
            packet_transporter,
            renet_server,
        })
    }

    pub fn tick(&mut self, delta_time: Duration) -> Result<(), NetcodeTransportError> {

        self.packet_transporter.update(delta_time, &mut self.renet_server)?;
        self.renet_server.update(delta_time);
        self.process_events();
        self.process_packets();
        self.packet_transporter.send_packets(&mut self.renet_server);
        Ok(())
    }

    pub fn process_events(&mut self) {
        while let Some(event) = self.renet_server.get_event() {
            match event {
                ServerEvent::ClientConnected{ client_id } => println!("Client {client_id} connected"),
                ServerEvent::ClientDisconnected{ client_id, reason } => println!("Client {client_id} disconnected: {reason}"),
            }
        }
    }

    pub fn process_packets(&mut self) {

        for client_id in self.renet_server.clients_id() {
            while let Some(packet) = self.renet_server.receive_message(client_id, DefaultChannel::Unreliable) {
                let str = String::from_utf8_lossy(packet.as_ref());
                println!("received packet: {}", str);
            }
        }
    }

    pub fn exit(&mut self) {
        self.packet_transporter.disconnect_all(&mut self.renet_server);
    }
}