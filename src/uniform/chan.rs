use std::rc::Rc;
use std::cell::RefCell;
use std::net::SocketAddr;

use futures::task::{self, Task};
use uniform::Active;


pub enum Action<I> {
    StartSend(I),
    Poll,
}

pub(in uniform) struct Inner<I> {
    addr: SocketAddr,
    request: Option<I>,
    active: Rc<RefCell<Active<I>>>,
    task: Option<Task>,
    pub(in uniform) queued: bool,
    // TODO(tailhook) verify that close flag is okay
    pub(in uniform) closed: bool,
}

pub struct Controller<I> {
    pub(in uniform) inner: Rc<RefCell<Inner<I>>>,
}

pub struct Helper<I> {
    pub(in uniform) inner: Rc<RefCell<Inner<I>>>,
}

impl<I> Helper<I> {
    pub fn new(addr: SocketAddr, active: Rc<RefCell<Active<I>>>)
        -> Helper<I>
    {
        let inner = Rc::new(RefCell::new(Inner {
            addr, active,
            task: None,
            queued: false,
            closed: false,
            request: None,
        }));
        return Helper { inner }
    }
    pub fn controller(&self) -> Controller<I> {
        Controller {
            inner: self.inner.clone(),
        }
    }
    pub fn take(&self) -> Action<I> {
        let mut cell = self.inner.borrow_mut();
        match cell.request.take() {
            Some(item) => Action::StartSend(item),
            None => Action::Poll,
        }
    }
    pub fn backpressure(&self, value: I) {
        let mut cell = self.inner.borrow_mut();
        cell.request = Some(value);
        assert!(!cell.queued);
    }
    pub fn requeue(&self) {
        let active = {
            let mut cell = self.inner.borrow_mut();
            cell.task = Some(task::current());
            if cell.queued {
                return;
            }
            cell.active.clone()
        };
        active.borrow_mut().add(self.controller());
    }
    pub fn closed(&self) {
        self.inner.borrow_mut().closed = true;
    }
    pub fn addr(&self) -> SocketAddr {
        self.inner.borrow().addr
    }
}

impl<I> Controller<I> {
    pub fn is_closed(&self) -> bool {
        self.inner.borrow().closed
    }
    pub fn request_back(&self) -> Option<I> {
        let mut cell = self.inner.borrow_mut();
        let res = cell.request.take();
        res
    }
    pub fn request(&self, item: I) {
        let mut inner = self.inner.borrow_mut();
        assert!(inner.request.is_none());
        inner.request = Some(item);
        inner.task.as_ref().map(|x| x.notify());
    }
}

// TODO(tailhook) check for closed or queued flag in Drop but only when
// drop is implemented in LazyUniform. And that will only be reasonably
// possible if `FuturesUnordered::iter_mut()` is implemented
//
// impl<I> Drop for Helper<I> {
//     fn drop(&mut self) {
//         assert!(self.inner.borrow().closed);
//     }
// }
