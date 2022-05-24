use std::borrow::Cow;

use futures::channel::mpsc::{self, UnboundedSender};
use futures::stream::{Stream, StreamExt};

use crate::html::{BaseComponent, Scope};
use crate::platform::{run_pinned, spawn_local};

// Same as std::io::BufWriter and futures::io::BufWriter.
const DEFAULT_BUF_SIZE: usize = 8 * 1024;

/// A [`futures::io::BufWriter`], but operates over string and yields into a Stream.
pub(crate) struct BufWriter {
    buf: String,
    tx: UnboundedSender<String>,
    capacity: usize,
}

impl BufWriter {
    pub fn with_capacity(capacity: usize) -> (Self, impl Stream<Item = String>) {
        let (tx, rx) = mpsc::unbounded::<String>();

        let this = Self {
            buf: String::with_capacity(capacity),
            tx,
            capacity,
        };

        (this, rx)
    }

    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Writes a string into the buffer, optionally drains the buffer.
    pub fn write(&mut self, s: Cow<'_, str>) {
        if s.len() > self.capacity {
            // if the next part is more than buffer size, we drain the buffer and the next
            // part.
            if !self.buf.is_empty() {
                let mut buf = String::with_capacity(self.capacity);
                std::mem::swap(&mut buf, &mut self.buf);
                let _ = self.tx.unbounded_send(buf);
            }

            let _ = self.tx.unbounded_send(s.into_owned());
        } else if self.buf.capacity() >= s.len() {
            // There is enough capacity, we push it on to the buffer.
            self.buf.push_str(&s);
        } else {
            // The next part is not going to fit into the buffer, we send
            // the current buffer and make a new buffer.
            let mut buf = String::with_capacity(self.capacity);
            buf.push_str(&s);

            std::mem::swap(&mut buf, &mut self.buf);
            let _ = self.tx.unbounded_send(buf);
        }
    }
}

impl Drop for BufWriter {
    fn drop(&mut self) {
        if !self.buf.is_empty() {
            let mut buf = "".to_string();
            std::mem::swap(&mut buf, &mut self.buf);
            let _ = self.tx.unbounded_send(buf);
        }
    }
}

/// A Yew Server-side Renderer that renders on the current thread.
#[cfg_attr(documenting, doc(cfg(feature = "ssr")))]
#[derive(Debug)]
pub struct LocalServerRenderer<COMP>
where
    COMP: BaseComponent,
{
    props: COMP::Properties,
    hydratable: bool,
    capacity: usize,
}

impl<COMP> Default for LocalServerRenderer<COMP>
where
    COMP: BaseComponent,
    COMP::Properties: Default,
{
    fn default() -> Self {
        Self::with_props(COMP::Properties::default())
    }
}

impl<COMP> LocalServerRenderer<COMP>
where
    COMP: BaseComponent,
    COMP::Properties: Default,
{
    /// Creates a [LocalServerRenderer] with default properties.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<COMP> LocalServerRenderer<COMP>
where
    COMP: BaseComponent,
{
    /// Creates a [LocalServerRenderer] with custom properties.
    pub fn with_props(props: COMP::Properties) -> Self {
        Self {
            props,
            hydratable: true,
            capacity: DEFAULT_BUF_SIZE,
        }
    }

    /// Sets the capacity of renderer buffer.
    ///
    /// Default: `8192`
    pub fn capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity;

        self
    }

    /// Sets whether an the rendered result is hydratable.
    ///
    /// Defaults to `true`.
    ///
    /// When this is sets to `true`, the rendered artifact will include additional information
    /// to assist with the hydration process.
    pub fn hydratable(mut self, val: bool) -> Self {
        self.hydratable = val;

        self
    }

    /// Renders Yew Application.
    pub async fn render(self) -> String {
        let mut s = String::new();

        self.render_to_string(&mut s).await;

        s
    }

    /// Renders Yew Application to a String.
    pub async fn render_to_string(self, w: &mut String) {
        let mut s = self.render_stream().await;

        while let Some(m) = s.next().await {
            w.push_str(&m);
        }
    }

    /// Renders Yew Applications into a string Stream
    // Whilst not required to be async here, this function is async to keep the same function
    // signature as the ServerRenderer.
    pub async fn render_stream(self) -> impl Stream<Item = String> {
        let (mut w, rx) = BufWriter::with_capacity(self.capacity);

        let scope = Scope::<COMP>::new(None);
        spawn_local(async move {
            scope
                .render_into_stream(&mut w, self.props.into(), self.hydratable)
                .await;
        });

        rx
    }
}

/// A Yew Server-side Renderer.
///
/// For runtimes with multi-threading support,
/// Yew manages a default worker pool with the number of worker thread equal to the CPU running
/// cores. You may override the spawning logic with
#[cfg_attr(documenting, doc(cfg(feature = "ssr")))]
#[derive(Debug)]
pub struct ServerRenderer<COMP>
where
    COMP: BaseComponent,
    COMP::Properties: Send,
{
    props: COMP::Properties,
    hydratable: bool,
    capacity: usize,
}

impl<COMP> Default for ServerRenderer<COMP>
where
    COMP: BaseComponent,
    COMP::Properties: Default + Send,
{
    fn default() -> Self {
        Self::with_props(COMP::Properties::default())
    }
}

impl<COMP> ServerRenderer<COMP>
where
    COMP: BaseComponent,
    COMP::Properties: Default + Send,
{
    /// Creates a [ServerRenderer] with default properties.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<COMP> ServerRenderer<COMP>
where
    COMP: BaseComponent,
    COMP::Properties: Send,
{
    /// Creates a [ServerRenderer] with custom properties.
    pub fn with_props(props: COMP::Properties) -> Self {
        Self {
            props,
            hydratable: true,
            capacity: DEFAULT_BUF_SIZE,
        }
    }

    /// Sets the capacity of renderer buffer.
    ///
    /// Default: `8192`
    pub fn capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity;

        self
    }

    /// Sets whether an the rendered result is hydratable.
    ///
    /// Defaults to `true`.
    ///
    /// When this is sets to `true`, the rendered artifact will include additional information
    /// to assist with the hydration process.
    pub fn hydratable(mut self, val: bool) -> Self {
        self.hydratable = val;

        self
    }

    /// Renders Yew Application.
    pub async fn render(self) -> String {
        let mut s = String::new();

        self.render_to_string(&mut s).await;

        s
    }

    /// Renders Yew Application to a String.
    pub async fn render_to_string(self, w: &mut String) {
        let mut s = self.render_stream().await;

        while let Some(m) = s.next().await {
            w.push_str(&m);
        }
    }

    /// Renders Yew Applications into a string Stream.
    pub async fn render_stream(self) -> impl Stream<Item = String> {
        run_pinned(move || async move {
            let Self {
                props,
                hydratable,
                capacity,
            } = self;

            LocalServerRenderer::<COMP>::with_props(props)
                .hydratable(hydratable)
                .capacity(capacity)
                .render_stream()
                .await
        })
        .await
    }
}
