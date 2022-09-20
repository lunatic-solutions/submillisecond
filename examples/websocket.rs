use lunatic::{
    abstract_process,
    process::{ProcessRef, StartProcess},
    Mailbox, Process,
};
use submillisecond::websocket::{
    Message, SplitSink, SplitStream, WebSocket, WebSocketConnection, WebSocketUpgrade,
};
use submillisecond::{router, Application};

struct WebSocketHandler {
    writer: SplitStream,
}

#[abstract_process]
impl WebSocketHandler {
    #[init]
    fn init(this: ProcessRef<Self>, (ws_conn,): (WebSocketConnection,)) -> Self {
        let (reader, writer) = ws_conn.split();

        fn read_handler(
            (mut reader, this): (SplitSink, ProcessRef<WebSocketHandler>),
            _: Mailbox<()>,
        ) {
            loop {
                match reader.read_message() {
                    Ok(Message::Text(msg)) => {
                        print!("{msg}");
                        this.send_message("Pong".to_owned());
                    }
                    Ok(Message::Close(_)) => break,
                    Ok(_) => {}
                    Err(err) => eprintln!("Read Message Error: {err:?}"),
                }
            }
        }

        Process::spawn_link((reader, this), read_handler);

        WebSocketHandler { writer }
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
        ws.on_upgrade(|conn| {
            lunatic_log::info!("Establish WebSocket Connection...");
            WebSocketHandler::start_link((conn,), None);
        })
    }

    Application::new(router! {
        GET "/" => websocket
    })
    .serve("0.0.0.0:3000")
}
