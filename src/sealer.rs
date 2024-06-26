use env_logger::Env;
use ethercrab::{
    error::Error,
    std::{ethercat_now, tx_rx_task},
    Client, ClientConfig, PduStorage, Timeouts,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::time;
use tokio::time::{timeout, MissedTickBehavior};

const MAX_SLAVES: usize = 16;
const MAX_PDU_DATA: usize = PduStorage::element_size(1100);
const MAX_FRAMES: usize = 16;
const PDI_LEN: usize = 64;
const INTERFACE: &str = "enp1s0f0";

static PDU_STORAGE: PduStorage<MAX_FRAMES, MAX_PDU_DATA> = PduStorage::new();

pub struct Sealer {
    open_door_byte: u8,
    close_door_byte: u8,
    apply_heater_byte: u8,
    remove_heater_byte: u8,
}
impl Sealer {
    pub fn new(
        open_door_byte: u8,
        close_door_byte: u8,
        apply_heater_byte: u8,
        remove_heater_byte: u8,
    ) -> Self {
        Self {
            open_door_byte,
            close_door_byte,
            apply_heater_byte,
            remove_heater_byte,
        }
    }

    pub async fn open_door(&self, duration: Duration) -> Result<(), ()> {
        Self::loop_with_timer(self.open_door_byte, duration).await
    }

    pub async fn close_door(&self, duration: Duration) -> Result<(), ()> {
        Self::loop_with_timer(self.close_door_byte, duration).await
    }

    pub async fn apply_heater(&self, duration: Duration) -> Result<(), ()> {
        Self::loop_with_timer(self.apply_heater_byte, duration).await
    }

    pub async fn remove_heater(&self, duration: Duration) -> Result<(), ()> {
        Self::loop_with_timer(self.remove_heater_byte, duration).await
    }

    async fn loop_with_timer(command_byte: u8, duration: Duration) -> Result<(), ()> {
        let loop_command = Self::loop_command(command_byte);
        match timeout(duration, loop_command).await {
            Ok(_) => {
                eprintln!("Ethercrab loop ended abruptly");
                Err(())
            }
            Err(_) => Ok(()),
        }
    }

    async fn loop_command(command_byte: u8) {
        // helper to loop given byte command from above fn
        log::info!("Starting ethercrab...");

        let (tx, rx, pdu_loop) = PDU_STORAGE.try_split().expect("Can only split once");
        let client = Arc::new(Client::new(
            pdu_loop,
            Timeouts {
                wait_loop_delay: Duration::from_millis(2),
                mailbox_response: Duration::from_millis(1000),
                ..Default::default()
            },
            ClientConfig::default(),
        ));
        tokio::spawn(tx_rx_task(&INTERFACE, tx, rx).expect("Spawn TX/RX task"));
        let group = client
            .init_single_group::<MAX_SLAVES, PDI_LEN>(ethercat_now)
            .await
            .expect("Init");
        log::info!("Discovered {:?} slaves", group.len());
        let mut group = group.into_op(&client).await.expect("Pre-Op -> Op");
        for slave in group.iter(&client) {
            let (i, o) = slave.io_raw();
            log::info!(
                "Slave {:#06x} {} inputs: {} bytes, outputs: {} bytes",
                slave.configured_address(),
                slave.name(),
                i.len(),
                o.len(),
            );
        }
        let mut tick_interval = tokio::time::interval(Duration::from_millis(5));
        tick_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let shutdown = Arc::new(AtomicBool::new(false));
        signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&shutdown))
            .expect("Register hook");
        loop {
            if shutdown.load(Ordering::Relaxed) {
                log::info!("Shutting down...");
                break;
            }
            group.tx_rx(&client).await.expect("TX/RX");
            let mut slave = group.slave(&client, 1).unwrap();
            let (_, o) = slave.io_raw_mut();
            o[0] = command_byte;
            tick_interval.tick().await;
        }
        let group = group.into_safe_op(&client).await.expect("Op -> Safe-Op");
        log::info!("Op -> Safe-Op");
        let group = group.into_pre_op(&client).await.expect("Safe-Op -> Pre-Op");
        log::info!("Safe-Op -> Pre-Op");
        let _group = group.into_init(&client).await.expect("Pre-Op -> Init");
        log::info!("Pre-Op -> Init... Shutdown Complete!");
    }
}
