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

use lunatic::ap::{AbstractProcess, Config, ProcessRef};
use lunatic::{abstract_process, ProcessName};
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
#[serde(bound = "")]
pub struct State<T>
where
    T: Clone + Serialize + for<'d> Deserialize<'d> + 'static,
{
    process: ProcessRef<StateProcess<T>>,
    state: T,
}

struct StateProcess<T> {
    value: T,
}

impl<T> State<T>
where
    T: Clone + Serialize + for<'de> Deserialize<'de> + 'static,
{
    /// Initializes state by spawning a process.
    pub fn init(state: T) -> Self {
        let name = StateProcessName::new::<T>();
        let process = StateProcess::start_as(&name, state.clone()).unwrap();
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
        let name = StateProcessName::new::<T>();
        let process = ProcessRef::lookup(&name)?;
        let state = process.get();
        Some(State { process, state })
    }

    /// Consumes the wrapper, returning the wrapped inner state.
    pub fn into_inner(self) -> T {
        self.state
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
    T: fmt::Debug + Clone + Serialize + for<'de> Deserialize<'de>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <T as fmt::Debug>::fmt(&self.state, f)
    }
}

struct StateProcessName {
    name: String,
}

impl StateProcessName {
    fn new<T>() -> Self {
        let name = format!("submillisecond-state-{}", std::any::type_name::<T>());
        StateProcessName { name }
    }
}

impl ProcessName for StateProcessName {
    fn process_name(&self) -> &str {
        &self.name
    }
}

#[abstract_process]
impl<T> StateProcess<T>
where
    T: Clone + Serialize + for<'de> Deserialize<'de> + 'static,
{
    #[init]
    fn init(_: Config<Self>, value: T) -> Result<Self, ()> {
        Ok(StateProcess { value })
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
