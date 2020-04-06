#![no_std]

use libtock::result::TockResult;
use libtock::sensors::Sensor;
use libtock::timer::Duration;

#[libtock::main]
async fn main() -> TockResult<()> {
    let mut drivers = libtock::retrieve_drivers()?;

    let mut timer_driver = drivers.timer.create_timer_driver();
    let timer_driver = timer_driver.activate()?;
    let mut console = drivers.console.create_console();

    loop {
        writeln!(
            console,
            "Humidity:    {}\n",
            drivers.humidity_sensor.read()?
        )
        .await?;
        writeln!(
            console,
            "Temperature: {}\n",
            drivers.temperature_sensor.read()?
        )
        .await?;
        writeln!(
            console,
            "Light:       {}\n",
            drivers.ambient_light_sensor.read()?
        )
        .await?;
        writeln!(
            console,
            "Accel:       {}\n",
            drivers.ninedof.read_acceleration()?
        )
        .await?;
        timer_driver.sleep(Duration::from_ms(500)).await?;
    }
}
