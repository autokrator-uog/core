use std::collections::VecDeque;

pub trait VecDequeExt<T>
    where T: Eq
{
    fn remove_item(&mut self, elem: &T) -> Option<T>;
}

impl<T> VecDequeExt<T> for VecDeque<T>
    where T: Eq
{
    fn remove_item(&mut self, elem: &T) -> Option<T> {
        let index = self.iter().position(|ref e| *e == elem);

        if let Some(index) = index {
            self.remove(index)
        } else {
            None
        }
    }
}
