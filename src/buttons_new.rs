use crate::callback::CallbackSubscription;
use crate::callback::SubscribableCallback;
use crate::futures;
use crate::result::TockResult;
use crate::syscalls;
use core::cell::Cell;
use core::marker::PhantomData;

const DRIVER_NUMBER: usize = 0x00003;

mod command_nr {
    pub const COUNT: usize = 0;
    pub const ENABLE_INTERRUPT: usize = 1;
    pub const DISABLE_INTERRUPT: usize = 2;
    pub const READ: usize = 3;
}

mod subscribe_nr {
    pub const SUBSCRIBE_CALLBACK: usize = 0;
}

pub struct ButtonsDriverFactory;

impl ButtonsDriverFactory {
    /// Creates a context before the [ButtonsDriver] can be initialized.
    /// Note that the context cannot be created within the driver intitialization routine itself.
    /// This restriction is a consequence of `libtock-rs` being heap agnostic resulting in a more complex ownership situation.
    pub fn create_context(&mut self) -> ButtonsDriverContext {
        ButtonsDriverContext {
            callback: ButtonsDriverCallback {
                recorded_state: Cell::new(RecordedState::Released(0)),
                record_id: Cell::new(RecordId::A),
            },
            lifetime: Default::default(),
        }
    }
}

pub struct ButtonsDriverContext<'a> {
    callback: ButtonsDriverCallback,
    lifetime: PhantomData<&'a ()>,
}

impl ButtonsDriverContext<'_> {
    pub fn init_driver(&mut self) -> TockResult<ButtonsDriver> {
        let num_buttons = syscalls::command(DRIVER_NUMBER, command_nr::COUNT, 0, 0)?;

        let subscription = syscalls::subscribe(
            DRIVER_NUMBER,
            subscribe_nr::SUBSCRIBE_CALLBACK,
            &self.callback,
        )?;

        Ok(ButtonsDriver {
            num_buttons,
            callback: &self.callback,
            subscription,
        })
    }
}

struct ButtonsDriverCallback {
    recorded_state: Cell<RecordedState>,
    record_id: Cell<RecordId>,
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum RecordedState {
    Pressed(usize),
    Released(usize),
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum RecordId {
    A,
    B,
}

impl RecordId {
    fn next(self) -> Self {
        match self {
            RecordId::A => RecordId::B,
            RecordId::B => RecordId::A,
        }
    }
}

impl ButtonsDriverCallback {
    async fn wait_for_event(&self, expected_state: RecordedState) {
        while self.wait_for_any_event().await != expected_state {}
    }

    async fn wait_for_any_event(&self) -> RecordedState {
        let last_seen_record_id = self.record_id.get();
        futures::wait_for_value(|| {
            if self.record_id.get() == last_seen_record_id {
                None
            } else {
                Some(self.recorded_state.get())
            }
        })
        .await
    }
}

impl SubscribableCallback for ButtonsDriverCallback {
    fn call_rust(&self, button_num: usize, button_state: usize, _: usize) {
        match button_state {
            0 => self.recorded_state.set(RecordedState::Released(button_num)),
            1 => self.recorded_state.set(RecordedState::Pressed(button_num)),
            _ => return,
        }
        self.record_id.set(self.record_id.get().next());
    }
}

pub struct ButtonsDriver<'a> {
    num_buttons: usize,
    callback: &'a ButtonsDriverCallback,
    #[allow(unused)] // Used in drop
    subscription: CallbackSubscription<'a>,
}

impl<'a> ButtonsDriver<'a> {
    pub fn buttons(&mut self) -> Buttons {
        Buttons {
            num_buttons: self.num_buttons,
            callback: self.callback,
            curr_button: 0,
        }
    }

    pub async fn wait_for_event(&self) -> (ButtonState, usize) {
        match self.callback.wait_for_any_event().await {
            RecordedState::Pressed(button_num) => (ButtonState::Pressed, button_num),
            RecordedState::Released(button_num) => (ButtonState::Released, button_num),
        }
    }
}

pub struct Buttons<'a> {
    num_buttons: usize,
    callback: &'a ButtonsDriverCallback,
    curr_button: usize,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ButtonState {
    Pressed,
    Released,
}

impl From<usize> for ButtonState {
    fn from(state: usize) -> ButtonState {
        match state {
            0 => ButtonState::Released,
            1 => ButtonState::Pressed,
            _ => unreachable!(), // TODO
        }
    }
}

impl<'a> Iterator for Buttons<'a> {
    type Item = Button<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr_button < self.num_buttons {
            let item = Button {
                button_num: self.curr_button,
                callback: self.callback,
            };
            self.curr_button += 1;
            Some(item)
        } else {
            None
        }
    }
}

pub struct Button<'a> {
    button_num: usize,
    callback: &'a ButtonsDriverCallback,
}

impl<'a> Button<'a> {
    pub fn read(&self) -> TockResult<ButtonState> {
        syscalls::command(DRIVER_NUMBER, command_nr::READ, self.button_num, 0)
            .map(ButtonState::from)
            .map_err(Into::into)
    }

    pub fn enable_interrupt(&self) -> TockResult<()> {
        syscalls::command(
            DRIVER_NUMBER,
            command_nr::ENABLE_INTERRUPT,
            self.button_num,
            0,
        )?;
        Ok(())
    }

    pub fn disable_interrupt(&self) -> TockResult<()> {
        syscalls::command(
            DRIVER_NUMBER,
            command_nr::DISABLE_INTERRUPT,
            self.button_num,
            0,
        )?;
        Ok(())
    }

    pub async fn wait_for_pressed_event(&self) {
        self.callback
            .wait_for_event(RecordedState::Pressed(self.button_num))
            .await
    }

    pub async fn wait_for_released_event(&self) {
        self.callback
            .wait_for_event(RecordedState::Released(self.button_num))
            .await
    }
}
