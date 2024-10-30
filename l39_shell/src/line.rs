use std::{cell::OnceCell, ops::Deref, string::Drain};

pub struct Line<'a> {
    inner: Drain<'a>,
    out_ptr: OnceCell<*const str>,
}

impl<'a> Deref for Line<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        unsafe { &**self.out_ptr.get_or_init(|| self.inner.as_str().trim()) }
    }
}

impl<'a> Line<'a> {
    pub fn drain(s: &'a mut String) -> Self {
        let inner = s.drain(..);
        let out_ptr = OnceCell::new();

        Self { inner, out_ptr }
    }
}
