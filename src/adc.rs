use crate::callback::{CallbackSubscription, SubscribableCallback};
use crate::result::TockResult;
use crate::shared_memory::SharedMemory;
use crate::syscalls;

pub const DRIVER_NUM: usize = 0x0005;
pub const BUFFER_SIZE: usize = 128;

mod command {
    pub const COUNT: usize = 0;
    pub const START: usize = 1;
    pub const START_REPEAT: usize = 2;
    pub const START_REPEAT_BUFFER: usize = 3;
    pub const START_REPEAT_BUFFER_ALT: usize = 4;
    pub const STOP: usize = 5;
}

mod subscribe {
    pub const SUBSCRIBE_CALLBACK: usize = 0;
}

mod allow {
    pub const BUFFER: usize = 0;
    pub const BUFFER_ALT: usize = 1;
}

pub struct AdcBuffer {
    // TODO: make this generic if possible with the driver impl
    buffer: [u8; BUFFER_SIZE],
}

impl AdcBuffer {
    pub fn new() -> AdcBuffer {
        AdcBuffer {
            buffer: [0; BUFFER_SIZE],
        }
    }
}

pub struct Adc<'a> {
    count: usize,
    #[allow(dead_code)] // Used in drop
    subscription: CallbackSubscription<'a>,
}

pub fn with_callback<CB>(callback: CB) -> WithCallback<CB> {
    WithCallback { callback }
}

pub struct WithCallback<CB> {
    callback: CB,
}

impl<CB: FnMut(usize, usize)> SubscribableCallback for WithCallback<CB> {
    fn call_rust(&mut self, _: usize, channel: usize, value: usize) {
        (self.callback)(channel, value);
    }
}

impl<'a, CB> WithCallback<CB>
where
    Self: SubscribableCallback,
{
    pub fn init(&mut self) -> TockResult<Adc> {
        let adc = Adc {
            count: syscalls::command(DRIVER_NUM, command::COUNT, 0, 0)?,
            subscription: syscalls::subscribe(DRIVER_NUM, subscribe::SUBSCRIBE_CALLBACK, self)?
                .unpeek(),
        };
        Ok(adc)
    }
}

impl<'a> Adc<'a> {
    pub fn init_buffer(buffer: &'a mut AdcBuffer) -> TockResult<SharedMemory> {
        syscalls::allow(DRIVER_NUM, allow::BUFFER, &mut buffer.buffer).map_err(Into::into)
    }

    pub fn init_alt_buffer(alt_buffer: &'a mut AdcBuffer) -> TockResult<SharedMemory> {
        syscalls::allow(DRIVER_NUM, allow::BUFFER_ALT, &mut alt_buffer.buffer).map_err(Into::into)
    }

    /// Return the number of available channels
    pub fn count(&self) -> usize {
        self.count
    }

    /// Start a single sample of channel
    pub fn sample(&self, channel: usize) -> TockResult<()> {
        syscalls::command(DRIVER_NUM, command::START, channel, 0)?;
        Ok(())
    }

    /// Start continuous sampling of channel
    pub fn sample_continuous(&self, channel: usize) -> TockResult<()> {
        syscalls::command(DRIVER_NUM, command::START_REPEAT, channel, 0)?;
        Ok(())
    }

    /// Start continuous sampling to first buffer
    pub fn sample_continuous_buffered(&self, channel: usize, frequency: usize) -> TockResult<()> {
        syscalls::command(DRIVER_NUM, command::START_REPEAT_BUFFER, channel, frequency)?;
        Ok(())
    }

    /// Start continuous sampling to alternating buffer
    pub fn sample_continuous_buffered_alt(
        &self,
        channel: usize,
        frequency: usize,
    ) -> TockResult<()> {
        syscalls::command(
            DRIVER_NUM,
            command::START_REPEAT_BUFFER_ALT,
            channel,
            frequency,
        )?;
        Ok(())
    }

    /// Stop any started sampling operation
    pub fn stop(&self) -> TockResult<()> {
        syscalls::command(DRIVER_NUM, command::STOP, 0, 0)?;
        Ok(())
    }
}

impl<'a> Drop for Adc<'a> {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
