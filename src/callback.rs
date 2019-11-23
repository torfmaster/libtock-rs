use crate::syscalls;
use core::marker::PhantomData;
use core::ptr;

pub trait SubscribableCallback {
    fn call_rust(&mut self, arg1: usize, arg2: usize, arg3: usize);
}

impl<F: FnMut(usize, usize, usize)> SubscribableCallback for F {
    fn call_rust(&mut self, arg1: usize, arg2: usize, arg3: usize) {
        self(arg1, arg2, arg3)
    }
}

#[must_use = "Subscriptions risk being dropped too early. Drop them manually."]
pub struct CallbackSubscription<'a> {
    driver_number: usize,
    subscribe_number: usize,
    _lifetime: PhantomData<&'a ()>,
}

#[must_use = "Subscriptions risk being dropped too early. Drop them manually."]
pub struct PeekableCallbackSubscription<'a, CB> {
    subscription: CallbackSubscription<'a>,
    callback: &'a mut CB,
}

impl<'a, CB> PeekableCallbackSubscription<'a, CB> {
    pub fn new(driver_number: usize, subscribe_number: usize, callback: &'a mut CB) -> Self {
        PeekableCallbackSubscription {
            subscription: CallbackSubscription {
                driver_number,
                subscribe_number,
                _lifetime: Default::default(),
            },
            callback,
        }
    }

    pub fn peek<T, F: FnOnce(&mut CB) -> T>(&mut self, peek_fn: F) -> T {
        peek_fn(self.callback)
    }

    pub fn unpeek(self) -> CallbackSubscription<'a> {
        self.subscription
    }
}

impl<'a> Drop for CallbackSubscription<'a> {
    fn drop(&mut self) {
        unsafe {
            syscalls::raw::subscribe(self.driver_number, self.subscribe_number, ptr::null(), 0);
        };
    }
}
