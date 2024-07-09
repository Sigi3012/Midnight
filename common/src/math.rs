use std::collections::HashMap;
use std::hash::Hash;

pub fn mode<T: Eq + Hash + Clone>(input: &Vec<T>) -> Option<T> {
    let mut map = HashMap::new();
    for i in input {
        let count = map.entry(i).or_insert(0);
        *count += 1;
    }

    map.into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(key, _)| key.clone())
}
