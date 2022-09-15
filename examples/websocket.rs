use submillisecond::websocket::{Message, WebSocket, WebSocketUpgrade};
use submillisecond::{router, Application};

fn websocket(ws: WebSocket) -> WebSocketUpgrade {
    ws.on_upgrade((), |mut conn, ()| {
        conn.write_message(Message::text("Hello from submillisecond!"))
            .unwrap();
    })
}

fn main() -> std::io::Result<()> {
    Application::new(router! {
        GET "/" => websocket
    })
    .serve("0.0.0.0:3000")
}
