use lunatic::ap::{Config, ProcessRef};
use lunatic::{abstract_process, AbstractProcess, Mailbox, Process};
use serde::{Deserialize, Serialize};
use submillisecond::websocket::{
    Message, SplitSink, SplitStream, WebSocket, WebSocketConnection, WebSocketUpgrade,
};
use submillisecond::{router, Application};

#[derive(Serialize, Deserialize)]
struct WebSocketHandler {
    writer: SplitSink,
}

#[abstract_process]
impl WebSocketHandler {
    #[init]
    fn init(this: Config<Self>, ws_conn: WebSocketConnection) -> Result<Self, ()> {
        let (writer, reader) = ws_conn.split();

        fn read_handler(
            (mut reader, this): (SplitStream, ProcessRef<WebSocketHandler>),
            _: Mailbox<()>,
        ) {
            loop {
                match reader.read_message() {
                    Ok(Message::Text(msg)) => {
                        print!("{msg}");
                        this.send_message("Pong".to_owned());
                    }
                    Ok(Message::Close(_)) => break,
                    Ok(_) => { /* Ignore other messages */ }
                    Err(err) => eprintln!("Read Message Error: {err:?}"),
                }
            }
        }

        Process::spawn_link((reader, this.self_ref()), read_handler);

        Ok(WebSocketHandler { writer })
    }

    #[handle_message]
    fn send_message(&mut self, message: String) {
        self.writer
            .write_message(Message::text(message))
            .unwrap_or_default();
    }
}

fn main() -> std::io::Result<()> {
    fn websocket(ws: WebSocket) -> WebSocketUpgrade {
        ws.on_upgrade((), |conn, _| {
            WebSocketHandler::link().start(conn).unwrap();
        })
    }

    Application::new(router! {
        GET "/" => websocket
    })
    .serve("0.0.0.0:3000")
}
