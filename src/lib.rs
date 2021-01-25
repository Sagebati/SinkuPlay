use std::sync::{Arc, Mutex, RwLock};
use std::rc::Rc;

pub mod server;
pub mod client;

pub trait Ignore {
    #[inline(always)]
    fn ignore(&self) -> () {
        ()
    }
}

impl<T> Ignore for T {}

#[inline(always)]
pub fn ignore() -> () {
    ()
}


pub trait Builder where Self: Sized {
    #[inline(always)]
    fn arc(self) -> Arc<Self>{
        Arc::new(self)
    }

    #[inline(always)]
    fn boxed(self) -> Box<Self> {
        Box::new(self)
    }

    #[inline(always)]
    fn rc(self) -> Rc<Self> {
        Rc::new(self)
    }

    #[inline(always)]
    fn some(self) -> Option<Self> {
        Some(self)
    }

    #[inline(always)]
    fn ok<Err>(self) -> Result<Self, Err> {
        Ok(self)
    }

    #[inline(always)]
    fn err<T>(self) -> Result<T, Self> {
        Err(self)
    }

    #[inline]
    fn mutex(self) -> Mutex<Self> {
        Mutex::new(self)
    }

    #[inline]
    fn rw_lock(self) -> RwLock<Self> {
        RwLock::new(self)
    }
}

impl<T> Builder for T {}

