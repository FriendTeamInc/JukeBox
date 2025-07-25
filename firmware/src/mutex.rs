use core::cell::UnsafeCell;

use rp2040_hal::sio::{Spinlock, SpinlockValid};

pub struct Mutex<const N: usize, T: ?Sized>
where
    Spinlock<N>: SpinlockValid,
{
    data: UnsafeCell<T>,
}

unsafe impl<const N: usize, T: ?Sized> Send for Mutex<N, T> where Spinlock<N>: SpinlockValid {}
unsafe impl<const N: usize, T: ?Sized> Sync for Mutex<N, T> where Spinlock<N>: SpinlockValid {}

impl<const N: usize, T> Mutex<N, T>
where
    Spinlock<N>: SpinlockValid,
{
    pub const fn new(data: T) -> Self {
        Mutex {
            data: UnsafeCell::new(data),
        }
    }

    pub fn with_lock<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let _lock = Spinlock::<N>::claim();
        cortex_m::asm::dmb();
        let r = f(unsafe { &*self.data.get() });
        cortex_m::asm::dmb();
        r
    }

    pub fn with_mut_lock<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let _lock = Spinlock::<N>::claim();
        cortex_m::asm::dmb();
        let r = f(unsafe { &mut *self.data.get() });
        cortex_m::asm::dmb();
        r
    }
}
