#[cfg_attr(target_arch = "riscv32", path = "platform_riscv32.rs")]
#[cfg_attr(target_arch = "arm", path = "platform_arm.rs")]
#[cfg_attr(
    not(any(target_arch = "arm", target_arch = "riscv32")),
    path = "platform_mock.rs"
)]
mod platform;

use crate::callback::CallbackSubscription;
use crate::callback::SubscribableCallback;
use crate::result::AllowError;
use crate::result::CommandError;
use crate::result::SubscribeError;
use crate::shared_memory::SharedMemory;

pub mod raw {
    use super::platform;

    pub use platform::*;

    /// # Safety
    ///
    /// Yielding in the main function should be safe. Nevertheless, yielding manually is not required as this is already achieved by the `async` runtime.
    ///
    /// When yielding in callbacks, two problems can arise:
    /// - The guarantees of `FnMut` are violated. In this case, make sure your callback has `Fn` behavior.
    /// - Callbacks can get executed in a nested manner and overflow the stack quickly.
    #[export_name = "libtock::syscalls::raw::yieldk"]
    pub unsafe fn yieldk() {
        platform::yieldk()
    }
}

pub fn subscribe<CB: SubscribableCallback>(
    driver_number: usize,
    subscribe_number: usize,
    callback: &mut CB,
) -> Result<CallbackSubscription, SubscribeError> {
    extern "C" fn c_callback<CB: SubscribableCallback>(
        arg1: usize,
        arg2: usize,
        arg3: usize,
        data: usize,
    ) {
        let callback = unsafe { &mut *(data as *mut CB) };
        callback.call_rust(arg1, arg2, arg3);
    }

    subscribe_fn(
        driver_number,
        subscribe_number,
        c_callback::<CB>,
        callback as *mut CB as usize,
    )
    .map(|_| CallbackSubscription::new(driver_number, subscribe_number))
}

pub fn subscribe_fn(
    driver_number: usize,
    subscribe_number: usize,
    callback: extern "C" fn(usize, usize, usize, usize),
    userdata: usize,
) -> Result<(), SubscribeError> {
    let return_code = unsafe {
        raw::subscribe(
            driver_number,
            subscribe_number,
            callback as *const _,
            userdata,
        )
    };

    if return_code == 0 {
        Ok(())
    } else {
        Err(SubscribeError {
            driver_number,
            subscribe_number,
            return_code,
        })
    }
}

pub fn command(
    driver_number: usize,
    command_number: usize,
    arg1: usize,
    arg2: usize,
) -> Result<usize, CommandError> {
    let return_code = unsafe { raw::command(driver_number, command_number, arg1, arg2) };
    if return_code >= 0 {
        Ok(return_code as usize)
    } else {
        Err(CommandError {
            driver_number,
            command_number,
            arg1,
            arg2,
            return_code,
        })
    }
}

/// [command1_insecure()] is a variant of [command()] that only sets the first
/// argument in the system call interface. It has the benefit of generating
/// simpler assembly than [command()], but it leaves the second argument's register
/// as-is which leaks it to the kernel driver being called. Prefer to use
/// [command()] instead of [command1_insecure()], unless the benefit of generating
/// simpler assembly outweighs the drawbacks of potentially leaking arbitrary
/// information to the driver you are calling.
///
/// At the moment, the only suitable use case for [command1_insecure()] is the low
/// level debug interface.
pub fn command1_insecure(
    driver_number: usize,
    command_number: usize,
    arg: usize,
) -> Result<usize, CommandError> {
    let return_code = unsafe { raw::command1(driver_number, command_number, arg) };
    if return_code >= 0 {
        Ok(return_code as usize)
    } else {
        Err(CommandError {
            driver_number,
            command_number,
            arg1: arg,
            arg2: 0,
            return_code,
        })
    }
}

pub fn allow(
    driver_number: usize,
    allow_number: usize,
    buffer_to_share: &mut [u8],
) -> Result<SharedMemory, AllowError> {
    let len = buffer_to_share.len();
    let return_code = unsafe {
        raw::allow(
            driver_number,
            allow_number,
            buffer_to_share.as_mut_ptr(),
            len,
        )
    };
    if return_code == 0 {
        Ok(SharedMemory::new(
            driver_number,
            allow_number,
            buffer_to_share,
        ))
    } else {
        Err(AllowError {
            driver_number,
            allow_number,
            return_code,
        })
    }
}
