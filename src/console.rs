use crate::callback::Identity0Consumer;
use crate::executor;
use crate::futures;
use crate::result::TockResult;
use crate::syscalls;
use core::cell::Cell;
use core::fmt::Arguments;
use core::mem;

const DRIVER_NUMBER: usize = 1;

mod command_nr {
    pub const WRITE: usize = 1;
}

mod subscribe_nr {
    pub const SET_ALARM: usize = 1;
}

mod allow_nr {
    pub const SHARE_BUFFER: usize = 1;
}

#[non_exhaustive]
pub struct ConsoleDriver;

impl ConsoleDriver {
    pub fn create_console(self) -> Console {
        Console {
            allow_buffer: [0; 64],
        }
    }
}

pub struct Console {
    allow_buffer: [u8; 64],
}

impl Console {
    pub fn write<S: AsRef<[u8]>>(&mut self, text: S) -> TockResult<()> {
        let mut not_written_yet = text.as_ref();
        while !not_written_yet.is_empty() {
            let num_bytes_to_print = self.allow_buffer.len().min(not_written_yet.len());
            self.allow_buffer[..num_bytes_to_print]
                .copy_from_slice(&not_written_yet[..num_bytes_to_print]);
            self.flush(num_bytes_to_print)?;
            not_written_yet = &not_written_yet[num_bytes_to_print..];
        }
        Ok(())
    }

    fn flush(&mut self, num_bytes_to_print: usize) -> TockResult<()> {
        let shared_memory = syscalls::allow(
            DRIVER_NUMBER,
            allow_nr::SHARE_BUFFER,
            &mut self.allow_buffer[..num_bytes_to_print],
        )?;

        let is_written = Cell::new(false);
        let mut is_written_alarm = || is_written.set(true);
        let subscription = syscalls::subscribe::<Identity0Consumer, _>(
            DRIVER_NUMBER,
            subscribe_nr::SET_ALARM,
            &mut is_written_alarm,
        )?;

        syscalls::command(DRIVER_NUMBER, command_nr::WRITE, num_bytes_to_print, 0)?;

        unsafe { executor::block_on(futures::wait_until(|| is_written.get())) };

        mem::drop(subscription);
        mem::drop(shared_memory);

        Ok(())
    }

    pub async fn write_fmt(&mut self, _args: Arguments<'_>) -> TockResult<()> {
        Ok(())
    }
}

pub fn write(output: &mut dyn Write, args: Arguments<'_>) -> Result {
    let mut formatter = Formatter {
        flags: 0,
        width: None,
        precision: None,
        buf: output,
        align: rt::v1::Alignment::Unknown,
        fill: ' ',
        args: args.args,
        curarg: args.args.iter(),
    };

    let mut idx = 0;

    match args.fmt {
        None => {
            // We can use default formatting parameters for all arguments.
            for (arg, piece) in args.args.iter().zip(args.pieces.iter()) {
                formatter.buf.write_str(*piece)?;
                (arg.formatter)(arg.value, &mut formatter)?;
                idx += 1;
            }
        }
        Some(fmt) => {
            // Every spec has a corresponding argument that is preceded by
            // a string piece.
            for (arg, piece) in fmt.iter().zip(args.pieces.iter()) {
                formatter.buf.write_str(*piece)?;
                formatter.run(arg)?;
                idx += 1;
            }
        }
    }

    // There can be only one trailing string piece left.
    if let Some(piece) = args.pieces.get(idx) {
        formatter.buf.write_str(*piece)?;
    }

    Ok(())
}
