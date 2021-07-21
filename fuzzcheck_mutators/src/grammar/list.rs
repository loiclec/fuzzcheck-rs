use std::rc::Rc;

#[derive(Debug)]
pub enum List<T>
where
    T: Clone,
{
    Empty,
    Cons(T, Rc<List<T>>),
}
impl<T> List<T>
where
    T: Clone,
{
    pub fn is_empty(&self) -> bool {
        matches!(self, List::Empty)
    }
    pub fn prepend(self: &Rc<Self>, x: T) -> Rc<List<T>> {
        Rc::new(List::Cons(x, self.clone()))
    }
    fn to_vec_recursive(self: &Rc<Self>, acc: &mut Vec<T>) {
        match self.as_ref() {
            List::Empty => {}
            List::Cons(x, rest) => {
                acc.push(x.clone());
                rest.to_vec_recursive(acc);
            }
        }
    }
    pub fn to_vec(self: &Rc<Self>) -> Vec<T> {
        let mut acc = Vec::new();
        self.to_vec_recursive(&mut acc);
        acc
    }
    pub fn from_slice(v: &[T]) -> Self {
        match v {
            [] => Self::Empty,
            [r, rs @ ..] => Self::Cons(r.clone(), Rc::new(Self::from_slice(rs))),
        }
    }
    pub fn iter(self: Rc<Self>) -> ListIter<T> {
        ListIter { list: self }
    }
}
pub struct ListIter<T>
where
    T: Clone,
{
    list: Rc<List<T>>,
}
impl<T> Iterator for ListIter<T>
where
    T: Clone,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let (next_list, result) = match self.list.as_ref() {
            List::Empty => (self.list.clone(), None),
            List::Cons(x, rest) => (rest.clone(), Some(x.clone())),
        };
        self.list = next_list;
        result
    }
}
