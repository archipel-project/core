use rand::Rng;
use renet::transport::{ClientAuthentication, NetcodeClientTransport, NetcodeTransportError};
use renet::{DefaultChannel, RenetClient};
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

pub struct ClientNetworkHandler {
    packet_transporter: NetcodeClientTransport,
    renet_client: RenetClient,
}

impl ClientNetworkHandler {
    pub fn new(server_addr: SocketAddr) -> anyhow::Result<Self> {
        let udp_socket = std::net::UdpSocket::bind(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 0))?;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap();

        let authentication = ClientAuthentication::Unsecure {
            server_addr,
            client_id: rand::thread_rng().gen_range(0..u64::MAX),
            user_data: None,
            protocol_id: 0,
        };

        let packet_transporter =
            NetcodeClientTransport::new(current_time, authentication, udp_socket)?;
        let renet_client = RenetClient::new(Default::default());

        Ok(Self {
            packet_transporter,
            renet_client,
        })
    }

    pub fn tick(&mut self, delta_time: Duration) -> Result<(), NetcodeTransportError> {
        self.renet_client.update(delta_time);
        self.packet_transporter
            .update(delta_time, &mut self.renet_client)?;
        self.process_packet();
        self.packet_transporter
            .send_packets(&mut self.renet_client)?;
        Ok(())
    }

    pub fn process_packet(&mut self) {
        if self.renet_client.is_connected() {
            while let Some(_message) = self
                .renet_client
                .receive_message(DefaultChannel::ReliableOrdered)
            {
                //process incoming packets
            }
            self.renet_client
                .send_message(DefaultChannel::Unreliable, "test");
        }
    }

    pub fn exit(&mut self) {
        self.packet_transporter.disconnect();
    }
}
