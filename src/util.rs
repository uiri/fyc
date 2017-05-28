use std::clone::Clone;

pub fn vec_or_empty<T: Clone>(v: Option<&Vec<T>>) -> Vec<T> {
        if let Some(a) = v {
            return (*a).clone();
        }
        Vec::new()
}
