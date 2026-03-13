use std::io::{self, Write};
use tracing_subscriber::fmt::MakeWriter;

use crate::utils::secret_redactor::redact;

/// A [`Write`] adapter that line-buffers output and redacts secrets from each
/// complete line before forwarding it to the underlying writer.
///
/// # Buffering behaviour
///
/// Bytes are accumulated in an internal buffer.  Whenever a newline (`\n`) is
/// encountered the accumulated line (including the newline) is passed through
/// [`redact`] and written to the inner writer.  Incomplete lines (no trailing
/// `\n`) stay in the buffer until either the next write that completes the
/// line, a call to [`flush`], or the writer is dropped.
pub struct RedactingWriter<W: Write> {
    inner: W,
    buffer: Vec<u8>,
}

impl<W: Write> RedactingWriter<W> {
    /// Create a new `RedactingWriter` wrapping `inner`.
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            buffer: Vec::new(),
        }
    }
}

impl<W: Write> Write for RedactingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        self.buffer.extend_from_slice(buf);

        // Process every complete line (terminated by \n) from the buffer.
        loop {
            match self.buffer.iter().position(|&b| b == b'\n') {
                None => break,
                Some(pos) => {
                    // Extract the line including the newline.
                    let line_bytes = self.buffer[..=pos].to_vec();
                    self.buffer.drain(..=pos);

                    let raw = String::from_utf8_lossy(&line_bytes);
                    let redacted = redact(&raw);
                    self.inner.write_all(redacted.as_bytes())?;
                }
            }
        }

        // Return the original length so callers know all bytes were "accepted".
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // Flush any remaining partial line through redact().
        if !self.buffer.is_empty() {
            let leftover = std::mem::take(&mut self.buffer);
            let raw = String::from_utf8_lossy(&leftover);
            let redacted = redact(&raw);
            self.inner.write_all(redacted.as_bytes())?;
        }
        self.inner.flush()
    }
}

impl<W: Write> Drop for RedactingWriter<W> {
    fn drop(&mut self) {
        // Best-effort flush; errors in Drop are ignored per standard Rust practice.
        let _ = self.flush();
    }
}

/// A [`MakeWriter`] factory that wraps every produced writer in a
/// [`RedactingWriter`], enabling secret redaction when used with
/// `tracing_subscriber`.
pub struct RedactingMakeWriter<M> {
    inner: M,
}

impl<M> RedactingMakeWriter<M> {
    /// Create a new `RedactingMakeWriter` wrapping `inner`.
    pub fn new(inner: M) -> Self {
        Self { inner }
    }
}

impl<'a, M> MakeWriter<'a> for RedactingMakeWriter<M>
where
    M: for<'w> MakeWriter<'w>,
{
    type Writer = RedactingWriter<<M as MakeWriter<'a>>::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        RedactingWriter::new(self.inner.make_writer())
    }
}

#[cfg(test)]
#[path = "redacting_writer_tests.rs"]
mod tests;
