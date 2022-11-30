use serde::{Deserialize, Serialize};
use submillisecond::state::State;
use submillisecond::{router, Application};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Count(i32);

fn index(mut count: State<Count>) -> String {
    // Increment count
    count.set(Count(count.0 + 1));

    // Return current count
    format!("Count is {}", count.0)
}

fn main() -> std::io::Result<()> {
    State::init(Count(0));

    Application::new(router! {
        GET "/" => index
    })
    .serve("0.0.0.0:3000")
}
