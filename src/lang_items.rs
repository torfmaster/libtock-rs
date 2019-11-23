//! Lang item required to make the normal `main` work in applications
//!
//! This is how the `start` lang item works:
//! When `rustc` compiles a binary crate, it creates a `main` function that looks
//! like this:
//!
//! ```
//! #[export_name = "main"]
//! pub extern "C" fn rustc_main(argc: isize, argv: *const *const u8) -> isize {
//!     start(main, argc, argv)
//! }
//! ```
//!
//! Where `start` is this function and `main` is the binary crate's `main`
//! function.
//!
//! The final piece is that the entry point of our program, _start, has to call
//! `rustc_main`. That's covered by the `_start` function in the root of this
//! crate.

use crate::led;
use crate::result::TockError;
use crate::timer;
use crate::timer::Duration;
use core::alloc::Layout;
use core::executor;
use core::future::Future;
use core::panic::PanicInfo;

#[lang = "start"]
extern "C" fn start<T>(main: fn() -> T, _argc: isize, _argv: *const *const u8) -> i32
where
    T: Termination,
{
    main().report()
}

#[lang = "termination"]
pub trait Termination {
    fn report(self) -> i32;
}

impl<T> Termination for T
where
    T: Future<Output = Result<(), TockError>>,
{
    fn report(self) -> i32 {
        let _ = unsafe { executor::block_on(self) };
        // TODO: Print result
        0
    }
}

#[panic_handler]
unsafe fn panic_handler(_info: &PanicInfo) -> ! {
    // Signal a panic using the LowLevelDebug capsule (if available).
    super::debug::low_level_status_code(1);

    // Flash all LEDs (if available).
    executor::block_on(async {
        loop {
            for led in led::all() {
                let _ = led.on();
            }
            let _ = timer::sleep(Duration::from_ms(100)).await;
            for led in led::all() {
                let _ = led.off();
            }
            let _ = timer::sleep(Duration::from_ms(100)).await;
        }
    });
    // Never type is not supported for T in Future
    unreachable!()
}

#[alloc_error_handler]
unsafe fn cycle_leds(_: Layout) -> ! {
    executor::block_on(async {
        loop {
            for led in led::all() {
                let _ = led.on();
                let _ = timer::sleep(Duration::from_ms(100)).await;
                let _ = led.off();
            }
        }
    });
    // Never type is not supported for T in Future
    unreachable!()
}
