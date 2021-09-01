use core::mem;
use core::fmt;

pub struct BytesWriter {
    buf: bytes::BytesMut,
}

impl BytesWriter {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            buf: bytes::BytesMut::with_capacity(10)
        }
    }

    #[inline(always)]
    ///Converts into `bytes::Bytes`
    pub fn freeze(&mut self) -> bytes::Bytes {
        mem::replace(&mut self.buf, bytes::BytesMut::new()).freeze()
    }
}

impl fmt::Write for BytesWriter {
    #[inline(always)]
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.buf.extend_from_slice(text.as_bytes());
        Ok(())
    }
}
