use std::rc::Rc;
use std::cell::RefCell;


pub enum Action<I> {
    StartSend(I),
    Poll,
}

struct Inner<I> {
    item: Option<I>,
    pressure: bool,
}

pub struct Sender<I> {
    item: Rc<RefCell<Inner<I>>>,
}

pub struct Receiver<I> {
    item: Rc<RefCell<Inner<I>>>,
}

pub fn new<I>() -> (Sender<I>, Receiver<I>) {
    let cell = Rc::new(RefCell::new(Inner {
        pressure: false,
        item: None,
    }));
    return (Sender { item: cell.clone() }, Receiver { item: cell.clone() });
}

impl<I> Sender<I> {
}

impl<I> Receiver<I> {
    pub fn take(&self) -> Action<I> {
        let mut cell = self.item.borrow_mut();
        match cell.item.take() {
            Some(item) => Action::StartSend(item),
            None => Action::Poll,
        }
    }
    pub fn set_backpressure(&self) {
        self.item.borrow_mut().pressure = true;
    }
    pub fn backpressure(&self, value: I) {
        let mut cell = self.item.borrow_mut();
        cell.pressure = true;
        cell.item = Some(value);
    }
    pub fn clear_backpressure(&self) {
        self.item.borrow_mut().pressure = false;
    }
}
