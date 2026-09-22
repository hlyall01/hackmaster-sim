//! In-memory catalogs for weapons, armor, shields, materials, presets.

use std::marker::PhantomData;
use std::sync::Arc;

use crate::core::ids::Id;

#[derive(Clone, Debug)]
pub struct Catalog<Tag, T> {
    entries: Arc<Vec<T>>,
    _tag: PhantomData<Tag>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshots_share_storage_until_any_mutation() {
        let original = Catalog::<(), String>::new(vec!["first".into(), "second".into()]);
        for mutation in 0..4 {
            let mut copy = original.clone();
            assert!(Arc::ptr_eq(&original.entries, &copy.entries));
            match mutation {
                0 => copy.entries_mut()[0] = "changed".into(),
                1 => *copy.get_mut(Id::new(0)).unwrap() = "changed".into(),
                2 => {
                    copy.push("third".into());
                }
                _ => {
                    copy.replace(Id::new(0), "changed".into());
                }
            }
            assert!(!Arc::ptr_eq(&original.entries, &copy.entries));
            assert_eq!(original.entries(), &["first", "second"]);
            assert_ne!(copy.entries(), original.entries());
        }
    }
}

impl<Tag, T> Catalog<Tag, T> {
    pub fn new(entries: Vec<T>) -> Self {
        Self {
            entries: Arc::new(entries),
            _tag: PhantomData,
        }
    }

    pub fn entries(&self) -> &[T] {
        &self.entries
    }

    pub fn entries_mut(&mut self) -> &mut Vec<T>
    where
        T: Clone,
    {
        Arc::make_mut(&mut self.entries)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, id: Id<Tag>) -> Option<&T> {
        self.entries.get(id.index())
    }

    pub fn get_mut(&mut self, id: Id<Tag>) -> Option<&mut T>
    where
        T: Clone,
    {
        if id.index() >= self.entries.len() {
            return None;
        }
        Arc::make_mut(&mut self.entries).get_mut(id.index())
    }

    pub fn id_from_index(&self, index: usize) -> Option<Id<Tag>> {
        if index < self.entries.len() {
            Some(Id::new(index))
        } else {
            None
        }
    }

    pub fn index_of(&self, id: Id<Tag>) -> usize {
        id.index()
    }

    pub fn first_id(&self) -> Option<Id<Tag>> {
        self.id_from_index(0)
    }

    pub fn push(&mut self, entry: T) -> Id<Tag>
    where
        T: Clone,
    {
        let id = Id::new(self.entries.len());
        Arc::make_mut(&mut self.entries).push(entry);
        id
    }

    pub fn replace(&mut self, id: Id<Tag>, entry: T) -> Option<T>
    where
        T: Clone,
    {
        if id.index() < self.entries.len() {
            Some(std::mem::replace(
                &mut Arc::make_mut(&mut self.entries)[id.index()],
                entry,
            ))
        } else {
            None
        }
    }
}
