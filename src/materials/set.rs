use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};
use std::sync::atomic::{AtomicUsize, Ordering};


#[derive(Clone)]
pub struct MaterialsSet {
    inner: Arc<RwLock<Set>>
}

pub struct MaterialsSubscriber {
    inner: Weak<RwLock<Set>>
}

struct Set {
    pub data: HashMap<String, usize>,
    pub version: usize,
}

static VERSION: AtomicUsize = AtomicUsize::new(0);

impl MaterialsSet {
    pub fn new() -> Self {
        let data = HashMap::new();
        let version = VERSION.fetch_add(1, Ordering::SeqCst);
        let set = Set { data, version };
        let inner = Arc::new(RwLock::new(set));
        Self { inner }
    }

    pub fn subscribe(&self) -> MaterialsSubscriber {
        let inner = Arc::downgrade(&self.inner);
        MaterialsSubscriber { inner }
    }

    pub fn add(&self, material: &str) {
        let mut set = self.inner.write().unwrap();
        match set.data.get_mut(material) {
            Some(rc) => *rc += 1,
            None => {
                set.data.insert(material.to_owned(), 1);
                set.version = VERSION.fetch_add(1, Ordering::SeqCst);
            },
        }
    }

    pub fn remove(&self, material: &str) {
        let mut set = self.inner.write().unwrap();
        if let Some(rc) = set.data.get_mut(material) {
            if *rc > 1 {
                *rc -= 1;
            } else {
                set.data.remove(material);
                set.version = VERSION.fetch_add(1, Ordering::SeqCst);
            }
        }
    }
}

impl MaterialsSubscriber {
    pub fn is_alive(&self) -> bool {
        self.inner.strong_count() > 0
    }

    pub fn is_subscribed(&self, set: &MaterialsSet) -> bool {
        std::ptr::addr_eq(
            Weak::as_ptr(&self.inner),
            Arc::as_ptr(&set.inner),
        )
    }

    pub fn replace(&self, current: &str, new: &str) -> bool {
        let Some(set) = self.inner.upgrade() else { return false };
        let mut set = set.write().unwrap();
        let mut update = false;
        if let Some(rc) = set.data.get_mut(current) {
            if *rc > 1 {
                *rc -= 1;
            } else {
                set.data.remove(current);
                update = true;
            }
        }
        match set.data.get_mut(new) {
            Some(rc) => *rc += 1,
            None => {
                set.data.insert(new.to_owned(), 1);
                update = true;
            },
        }
        if update {
            set.version = VERSION.fetch_add(1, Ordering::SeqCst);
        }
        true
    }
}
