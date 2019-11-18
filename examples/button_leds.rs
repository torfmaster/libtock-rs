#![no_std]

use futures::future;
use libtock::buttons_new::ObservableButton;
use libtock::result::TockResult;

#[libtock::main]
async fn main() -> TockResult<()> {
    let mut drivers = libtock::retrieve_drivers()?;

    let buttons_driver = drivers.buttons.init_driver()?;
    let leds_driver = drivers.leds.init_driver()?;

    future::try_join3(
        future::try_join3(
            toggle_led_on_button_press(&button_0, 0),
            toggle_led_on_button_press(&button_0, 2),
            toggle_led_on_button_press(&button_1, 1),
        ),
        future::try_join3(
            toggle_led_on_button_press(&button_1, 3),
            toggle_led_on_button_press(&button_2, 0),
            toggle_led_on_button_press(&button_2, 1),
        ),
        //future::try_join(
        toggle_led_on_button_press(&button_3, 2),
        //toggle_led_on_button_press(&button_3, 3),
        //),
    )
    .await?;
    Ok(())
}

async fn toggle_led_on_button_press(
    button: &ObservableButton<'_>,
    led_num: usize,
) -> TockResult<()> {
    loop {
        button.wait_for_pressed_event().await;
        led::get(led_num).unwrap().toggle()?;
    }
}
