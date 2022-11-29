//! Application state stored in a long running process.
//!
//! # Example
//!
//! ```
//! State::init(0);
//!
//! #[derive(Clone, Copy, Debug, Serialize, Deserialize)]
//! struct Count(i32);
//!
//! fn index(mut count: State<Count>) -> String {
//!    // Increment count
//!    count.set(Count(count.0 + 1));
//!
//!    // Return current count
//!    format!("Count is {}", count.0)
//! }
//! ```

use std::{fmt, ops};

use lunatic::abstract_process;
use lunatic::process::{ProcessRef, StartProcess};
use serde::{Deserialize, Serialize};

/// State stored in a process.
///
/// State should be initialized before use with [`State::init`], and can be
/// updated with [`State::set`].
///
/// State implements [`FromRequest`](crate::extract::FromRequest), allowing it
/// to be used as an extractor in handlers. If the state is not initialized, an
/// internal server error will be returned to the response.
#[derive(Clone, Serialize, Deserialize)]
pub struct State<T> {
    process: ProcessRef<StateProcess<T>>,
    state: T,
}

struct StateProcess<T> {
    value: T,
}

impl<T> State<T>
where
    T: Clone + Serialize + for<'de> Deserialize<'de>,
{
    /// Initializes state by spawning a process.
    pub fn init(state: T) -> Self {
        let name = format!("submillisecond-state-{}", std::any::type_name::<T>());
        let process = StateProcess::start(state.clone(), Some(&name));
        State { process, state }
    }

    /// Updates the value of state.
    pub fn set(&mut self, value: T) {
        self.state = value.clone();
        self.process.set(value);
    }

    /// Loads the current state.
    ///
    /// If the state has not initialized, `None` is returned.
    pub fn load() -> Option<Self> {
        let name = format!("submillisecond-state-{}", std::any::type_name::<T>());
        let process = ProcessRef::lookup(&name)?;
        let state = process.get();
        Some(State { process, state })
    }
}

impl<T> ops::Deref for State<T>
where
    T: Clone + Serialize + for<'de> Deserialize<'de>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<T> fmt::Debug for State<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <T as fmt::Debug>::fmt(&self.state, f)
    }
}

#[abstract_process]
impl<T> StateProcess<T>
where
    T: Clone + Serialize + for<'de> Deserialize<'de>,
{
    #[init]
    fn init(_: ProcessRef<Self>, value: T) -> Self {
        StateProcess { value }
    }

    #[handle_request]
    fn get(&self) -> T {
        self.value.clone()
    }

    #[handle_message]
    fn set(&mut self, value: T) {
        self.value = value;
    }
}