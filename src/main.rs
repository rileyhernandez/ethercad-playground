mod sealer;
use crate::sealer::Sealer;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), ()> {
    let sealer = Sealer::new(0x02, 0x01, 0x08, 0x04);

    sealer.open_door(Duration::from_secs_f64(5.3f64)).await?;
    // sealer.close_door(Duration::from_secs_f64(5.3f64)).await?;
    // sealer.apply_heater(Duration::from_secs_f64(7f64)).await?;
    // sealer.remove_heater(Duration::from_secs_f64(7f64)).await?;

    Ok(())
}
