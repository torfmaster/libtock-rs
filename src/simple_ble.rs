use crate::ble_composer::BlePayload;
use crate::callback::CallbackSubscription;
use crate::callback::SubscribableCallback;
use crate::result::TockResult;
use crate::shared_memory::SharedMemory;
use crate::syscalls;

const DRIVER_NUMBER: usize = 0x30000;
pub const MAX_PAYLOAD_SIZE: usize = 9;
pub const BUFFER_SIZE_ADVERTISE: usize = 39;
pub const BUFFER_SIZE_SCAN: usize = 39;

mod ble_commands {
    pub const START_ADVERTISING: usize = 0;
    pub const ALLOW_ADVERTISMENT_BUFFER: usize = 0;
    pub const BLE_PASSIVE_SCAN_SUB: usize = 0;
    pub const ALLOW_SCAN_BUFFER: usize = 1;
    pub const PASSIVE_SCAN: usize = 5;
}

mod gap_flags {
    pub const BLE_DISCOVERABLE: usize = 0x02;
}

pub mod gap_data {
    pub const COMPLETE_LIST_16BIT_SERVICE_IDS: usize = 0x03;
    pub const COMPLETE_LOCAL_NAME: usize = 0x09;
    pub const SET_FLAGS: usize = 1;
    pub const SERVICE_DATA: usize = 0x16;
}

pub struct BleAdvertisingDriver;

impl BleAdvertisingDriver {
    pub fn create_advertising_buffer() -> [u8; BUFFER_SIZE_ADVERTISE] {
        [0; BUFFER_SIZE_ADVERTISE]
    }
    pub fn initialize<'a>(
        interval: usize,
        service_payload: &BlePayload,
        advertising_buffer: &'a mut [u8; BUFFER_SIZE_ADVERTISE],
    ) -> TockResult<SharedMemory<'a>> {
        let mut shared_memory = syscalls::allow(
            DRIVER_NUMBER,
            ble_commands::ALLOW_ADVERTISMENT_BUFFER,
            advertising_buffer,
        )?;
        shared_memory.write_bytes(service_payload);
        Self::start_advertising(gap_flags::BLE_DISCOVERABLE, interval)?;
        Ok(shared_memory)
    }

    fn start_advertising(pdu_type: usize, interval: usize) -> TockResult<()> {
        syscalls::command(
            DRIVER_NUMBER,
            ble_commands::START_ADVERTISING,
            pdu_type,
            interval,
        )?;
        Ok(())
    }
}

pub struct BleCallback<CB> {
    callback: CB,
}

impl<CB> BleCallback<CB> {
    pub fn new(callback: CB) -> Self {
        BleCallback { callback }
    }
}

impl<CB: FnMut(usize, usize)> SubscribableCallback for BleCallback<CB> {
    fn call_rust(&mut self, arg1: usize, arg2: usize, _: usize) {
        (self.callback)(arg1, arg2);
    }
}

pub struct BleDriver;

impl BleDriver {
    pub fn create_scan_buffer() -> [u8; BUFFER_SIZE_SCAN] {
        [0; BUFFER_SIZE_SCAN]
    }

    pub fn share_memory(scan_buffer: &mut [u8; BUFFER_SIZE_SCAN]) -> TockResult<SharedMemory> {
        let shared_memory =
            syscalls::allow(DRIVER_NUMBER, ble_commands::ALLOW_SCAN_BUFFER, scan_buffer)?;
        Ok(shared_memory)
    }

    pub fn start<CB>(callback: &mut BleCallback<CB>) -> TockResult<CallbackSubscription>
    where
        BleCallback<CB>: SubscribableCallback,
    {
        let subscription =
            syscalls::subscribe(DRIVER_NUMBER, ble_commands::BLE_PASSIVE_SCAN_SUB, callback)?;
        syscalls::command(DRIVER_NUMBER, ble_commands::PASSIVE_SCAN, 1, 0)?;
        Ok(subscription)
    }
}
