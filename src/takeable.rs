pub(crate) struct Takeable<T>(Option<T>);

impl<T> Takeable<T> {
    pub fn new(value: T) -> Self {
        Self(Some(value))
    }

    pub fn transition<F: FnOnce(T) -> T>(&mut self, f: F) {
        self.0 = Some(f(self.0.take().unwrap_or_else(|| unreachable!())));
    }

    pub fn get(self) -> T {
        self.0.unwrap_or_else(|| unreachable!())
    }
}
