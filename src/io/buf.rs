#[derive(Default, Debug)]
pub struct OctetBuffer {
    pub(crate) buffer: Vec<u8>,
    pub(crate) write_position: usize,
    pub(crate) read_position: usize,
}

impl OctetBuffer {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            ..Default::default()
        }
    }

    pub fn from_bytes(buffer: Vec<u8>) -> Self {
        Self {
            buffer,
            ..Default::default()
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.write_position = 0;
        self.read_position = 0;
    }

    pub fn reset_read_position(&mut self) {
        self.read_position = 0;
    }

    pub fn content(&self) -> &[u8] {
        &self.buffer
    }

    pub fn byte_len(&self) -> usize {
        self.buffer.len()
    }

    /// Changes the write-position to the given position for the closure call.
    /// Restores the original write-position after the call.
    ///
    /// # Panics
    /// Positions beyond the current buffer length will result in panics.
    #[inline]
    pub fn with_write_position_at<T, F: Fn(&mut Self) -> T>(&mut self, position: usize, f: F) -> T {
        debug_assert!(position <= self.buffer.len() * 8);
        let before = core::mem::replace(&mut self.write_position, position);
        let result = f(self);
        self.write_position = before;
        result
    }

    /// Changes the read-position to the given position for the closure call.
    /// Restores the original read-position after the call.
    ///
    /// # Panics
    /// Positions beyond the current write-position will result in panics.
    #[inline]
    pub fn with_read_position_at<T, F: Fn(&mut Self) -> T>(&mut self, position: usize, f: F) -> T {
        debug_assert!(position < self.write_position);
        let before = core::mem::replace(&mut self.read_position, position);
        let result = f(self);
        self.read_position = before;
        result
    }

    /// Sets the `write_position` to `read_position + max_read_len` for the call of the given
    /// closure
    pub fn with_max_read<T, F: Fn(&mut Self) -> T>(&mut self, max_read_len: usize, f: F) -> T {
        let before =
            core::mem::replace(&mut self.write_position, self.read_position + max_read_len);
        let result = f(self);
        self.write_position = before;
        result
    }

    pub fn ensure_can_write_additional_bytese(&mut self, byte_len: usize) {
        if self.write_position + byte_len >= self.buffer.len() {
            let required_len = self.write_position + byte_len;
            let extend_by_len = required_len - self.buffer.len();
            self.buffer
                .extend(core::iter::repeat(0u8).take(extend_by_len))
        }
    }
}

impl Into<Vec<u8>> for OctetBuffer {
    fn into(self) -> Vec<u8> {
        self.buffer
    }
}

impl From<Vec<u8>> for OctetBuffer {
    fn from(buffer: Vec<u8>) -> Self {
        Self::from_bytes(buffer)
    }
}
