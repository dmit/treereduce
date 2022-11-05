#[derive(Clone, Debug)]
pub struct Versioned<T> {
    value: T,
    version: usize,
}

impl<T> Versioned<T> {
    pub fn extract(self) -> T {
        self.value
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    fn _mutate<F: FnOnce(T) -> T>(mut self, f: F) {
        self.value = f(self.value);
        self.inc();
    }

    pub fn mutate_clone<F: FnOnce(T) -> T>(&self, f: F) -> Self
    where
        T: Clone,
    {
        let mut r = (*self).clone();
        r.value = f(r.value);
        let ret = r.inc();
        debug_assert!(self.old_version(&ret));
        ret
    }

    pub fn new(value: T) -> Self {
        Versioned { value, version: 0 }
    }

    pub fn inc(self) -> Self {
        Versioned {
            value: self.value,
            version: self.version + 1,
        }
    }

    fn _modify(&self, value: T) -> Self {
        Versioned {
            value,
            version: self.version + 1,
        }
    }

    pub fn old_version(&self, other: &Versioned<T>) -> bool {
        self.version + 1 == other.version
    }

    fn _same_version(&self, other: &Versioned<T>) -> bool {
        self.version == other.version
    }
}
