//! In-memory catalogs for weapons, armor, shields, materials, presets.

use std::marker::PhantomData;

use crate::core::ids::Id;

#[derive(Clone, Debug)]
pub struct Catalog<Tag, T> {
    entries: Vec<T>,
    _tag: PhantomData<Tag>,
}

impl<Tag, T> Catalog<Tag, T> {
    pub fn new(entries: Vec<T>) -> Self {
        Self {
            entries,
            _tag: PhantomData,
        }
    }

    pub fn entries(&self) -> &[T] {
        &self.entries
    }

    pub fn entries_mut(&mut self) -> &mut Vec<T> {
        &mut self.entries
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

    pub fn get_mut(&mut self, id: Id<Tag>) -> Option<&mut T> {
        self.entries.get_mut(id.index())
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

    pub fn push(&mut self, entry: T) -> Id<Tag> {
        let id = Id::new(self.entries.len());
        self.entries.push(entry);
        id
    }

    pub fn replace(&mut self, id: Id<Tag>, entry: T) -> Option<T> {
        if id.index() < self.entries.len() {
            Some(std::mem::replace(&mut self.entries[id.index()], entry))
        } else {
            None
        }
    }
}
