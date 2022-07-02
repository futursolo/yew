use serde::{Deserialize, Serialize};

/// The Bridge Input.
#[derive(Serialize, Deserialize)]
pub(crate) enum ReactorInput<I>
where
    I: 'static,
{
    /// Starts the bridge.
    Start,
    /// An input message.
    Input(I),
}

/// The Bridge Output.
#[derive(Debug, Serialize, Deserialize)]
pub enum ReactorOutput<O>
where
    O: 'static,
{
    /// An output message has been received.
    Output(O),
    /// Reactor for current bridge has exited.
    Finish,
}