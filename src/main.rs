mod sealer;
use crate::sealer::{Sealer, SealerCommand};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), ()> {
    env_logger::init();

    let sealer = Sealer::new(0x02, 0x01, 0x08, 0x04);

    let (tx, rx) = mpsc::channel(10);
    let sealer_handler = tokio::spawn(async move { sealer.actor(rx).await });

    tx.send(SealerCommand::CloseDoor(5.3))
        .await
        .expect("Open door failed");
    tx.send(SealerCommand::ApplyHeater(7f64))
        .await
        .expect("Apply heater failed");
    tx.send(SealerCommand::RemoveHeater(7f64))
        .await
        .expect("Remove heater failed");
    tx.send(SealerCommand::OpenDoor(5.3))
        .await
        .expect("Close door failed");
    let _ = { tx };
    sealer_handler.await.expect("Sealer Actor failed");
    Ok(())
}
