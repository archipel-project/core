use std::{thread, time::Duration};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, atomic};
use std::time::Instant;
use crate::networking;


pub struct App {
    should_exit: Arc<atomic::AtomicBool>,
    network_manager: networking::ServerNetworkHandler,
}

impl App {

    pub fn new() -> anyhow::Result<Self> {
        // todo: do not hardcode the config
        let socket_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5000);
        let network_manager = networking::ServerNetworkHandler::new(socket_addr)?;


        Ok(Self {
            should_exit: Arc::new(atomic::AtomicBool::new(false)),
            network_manager
        })
    }

    fn should_exit(&self) -> bool {
        !self.should_exit.load(atomic::Ordering::SeqCst)
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        self.starting()?;
        self.running()?;
        self.exiting()?;
        Ok(())
    }

    fn starting(&mut self) -> anyhow::Result<()> {
        println!("starting server");
        let atomic_ref = self.should_exit.clone();
        ctrlc::set_handler(move || {
            atomic_ref.store(true, atomic::Ordering::SeqCst);
        })?;
        Ok(())
    }

    fn running(&mut self) -> anyhow::Result<()> {
        println!("server running");
        let mut last_updated  = Instant::now();
        //main loop
        while self.should_exit() {
            let now = Instant::now();
            let delta_time = now - last_updated;
            last_updated = now;

            //delta_time should be 50ms, if it's not, we're lagging
            self.tick(delta_time)?;

            //sleep to complete the 50ms
            let time_took = now.elapsed();
            if time_took > Duration::from_millis(50) {
                println!("server is lagging");
            }
            else {
                let time_to_sleep = Duration::from_millis(50) - time_took;
                thread::sleep(time_to_sleep);
            }

        }

        Ok(())
    }

    fn exiting(&mut self) -> anyhow::Result<()> {
        println!("stopping server");
        self.network_manager.exit();
        Ok(())
    }

    pub fn tick(&mut self, delta_time: Duration) -> anyhow::Result<()> {
        self.network_manager.tick(delta_time)?;
        Ok(())
    }


}