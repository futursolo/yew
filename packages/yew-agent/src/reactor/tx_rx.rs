use std::pin::Pin;

use futures::channel::mpsc;
use futures::channel::mpsc::{SendError, TrySendError};
use futures::sink::Sink;
use futures::stream::{FusedStream, Stream};
use futures::task::{Context, Poll};
use pin_project::pin_project;
use serde::{Deserialize, Serialize};

/// A receiver for reactors.
#[pin_project]
#[derive(Debug)]
pub struct ReactorReceiver<I>
where
    I: Serialize + for<'de> Deserialize<'de>,
{
    #[pin]
    rx: mpsc::UnboundedReceiver<I>,
}

impl<I> Stream for ReactorReceiver<I>
where
    I: Serialize + for<'de> Deserialize<'de>,
{
    type Item = I;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        this.rx.poll_next(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.rx.size_hint()
    }
}

impl<I> FusedStream for ReactorReceiver<I>
where
    I: Serialize + for<'de> Deserialize<'de>,
{
    fn is_terminated(&self) -> bool {
        self.rx.is_terminated()
    }
}

/// A trait to extract input type from [ReactorReceiver].
pub trait ReactorReceivable {
    /// The input message type.
    type Input: Serialize + for<'de> Deserialize<'de>;

    /// Creates a ReactorReceiver.
    fn new(rx: mpsc::UnboundedReceiver<Self::Input>) -> Self;
}

impl<I> ReactorReceivable for ReactorReceiver<I>
where
    I: Serialize + for<'de> Deserialize<'de>,
{
    type Input = I;

    fn new(rx: mpsc::UnboundedReceiver<I>) -> Self {
        Self { rx }
    }
}

/// A sender for reactors.
#[derive(Debug)]
pub struct ReactorSender<O>
where
    O: Serialize + for<'de> Deserialize<'de>,
{
    tx: mpsc::UnboundedSender<O>,
}

impl<O> Clone for ReactorSender<O>
where
    O: Serialize + for<'de> Deserialize<'de>,
{
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<O> ReactorSender<O>
where
    O: Serialize + for<'de> Deserialize<'de>,
{
    /// Send an output.
    pub fn send(&self, output: O) -> std::result::Result<(), TrySendError<O>> {
        self.tx.unbounded_send(output)
    }
}

impl<O> Sink<O> for &'_ ReactorSender<O>
where
    O: Serialize + for<'de> Deserialize<'de>,
{
    type Error = SendError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut &self.tx).poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: O) -> Result<(), Self::Error> {
        Pin::new(&mut &self.tx).start_send(item)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut &self.tx).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut &self.tx).poll_close(cx)
    }
}

/// A trait to extract output type from [ReactorSender].
pub trait ReactorSendable {
    /// The output message type.
    type Output: Serialize + for<'de> Deserialize<'de>;

    /// Creates a ReactorSender.
    fn new(tx: mpsc::UnboundedSender<Self::Output>) -> Self;
}

impl<O> ReactorSendable for ReactorSender<O>
where
    O: Serialize + for<'de> Deserialize<'de>,
{
    type Output = O;

    fn new(tx: mpsc::UnboundedSender<O>) -> Self {
        Self { tx }
    }
}
