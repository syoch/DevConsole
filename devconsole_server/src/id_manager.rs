use std::ops::Add;

pub struct IDManager<T> {
    next_id: T,
}

impl<T: Add + Copy + From<u64> + Into<u64>> IDManager<T> {
    pub fn new() -> Self {
        IDManager { next_id: 1.into() }
    }

    pub fn get_new_id(&mut self) -> T {
        let id = self.next_id;
        self.next_id = (self.next_id.into() + 1).into();
        id
    }
}
