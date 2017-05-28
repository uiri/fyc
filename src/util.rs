use std::clone::Clone;

#[derive(Clone, Serialize, Deserialize)]
pub struct NameValue {
    pub name: String,
    pub value: String
}

pub fn vec_or_empty<T: Clone>(v: Option<&Vec<T>>) -> Vec<T> {
        if let Some(a) = v {
            return (*a).clone();
        }
        Vec::new()
}
