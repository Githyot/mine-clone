//! The dedicated server runs without a window or a graphics card (Lote 9): no Vulkan, no
//! display, so it runs on any machine, even one with no screen. The game keeps its window,
//! renderer and model drawer behind `Gpu`, which reads like the thing itself; the server
//! never draws, and the few places the game logic touches them check `get_mut` first.

use std::ops::{Deref, DerefMut};

pub(crate) struct Gpu<T>(Option<T>);

impl<T> Gpu<T> {
    pub fn new(v: Option<T>) -> Self {
        Gpu(v)
    }

    pub fn get(&self) -> Option<&T> {
        self.0.as_ref()
    }

    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.0.as_mut()
    }
}

impl<T> Deref for Gpu<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0.as_ref().expect("the dedicated server has no window or graphics")
    }
}

impl<T> DerefMut for Gpu<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.0.as_mut().expect("the dedicated server has no window or graphics")
    }
}
