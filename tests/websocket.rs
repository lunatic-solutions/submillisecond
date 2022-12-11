use tungstenite::{client, ClientHandshake, HandshakeError, handshake::client::Response};

use lunatic::{
    abstract_process, sleep, Process, Mailbox, net::TcpStream,
    process::{ProcessRef, StartProcess},
};

use std::time::Duration;

use submillisecond::{
    websocket::{
        Message, SplitSink, SplitStream, WebSocket,
        WebSocketConnection, WebSocketUpgrade,
    },
    router, Application
};

#[lunatic::test]
fn websocket_connection_test() {
    let port = 9000;
    Process::spawn_link(port, setup_server);
    // Give enough time to for server to start
    sleep(Duration::from_millis(1000));

    let (mut socket, _response) = connect().expect("Can't connect");

    socket.write_message(Message::Text("Ping".into())).unwrap();

    let msg = socket.read_message().expect("Error reading message");
    assert_eq!(msg.into_text().unwrap(), "Pong");

    socket.close(None).unwrap();
}

struct WebSocketHandler {
    writer: SplitSink,
}

#[abstract_process]
impl WebSocketHandler {
    #[init]
    fn init(this: ProcessRef<Self>, ws_conn: WebSocketConnection) -> Self {
        let (writer, reader) = ws_conn.split();

        fn read_handler(
            (mut reader, this): (SplitStream, ProcessRef<WebSocketHandler>),
            _: Mailbox<()>,
        ) {
            loop {
                match reader.read_message() {
                    Ok(Message::Text(_)) => {
                        this.send_message("Pong".to_owned());
                    }
                    Ok(Message::Close(_)) => break,
                    Ok(_) => { /* Ignore other messages */ }
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

fn setup_server(port: u16, _: Mailbox<()>) {
    fn websocket(ws: WebSocket) -> WebSocketUpgrade {
        ws.on_upgrade((), |conn, _| {
            WebSocketHandler::start_link(conn, None);
        })
    }

    Application::new(router! {
        GET "/" => websocket
    })
    .serve(format!("0.0.0.0:{}", port))
    .unwrap();
}

fn connect() -> Result<(tungstenite::protocol::WebSocket<TcpStream>, Response), HandshakeError<ClientHandshake<TcpStream>>> {
    let tcp_stream = TcpStream::connect("127.0.0.1:9000").unwrap();

    let mut headers = [
        httparse::Header {
            name: "Connection",
            value: b"Upgrade",
        },
        httparse::Header {
            name: "Upgrade",
            value: b"websocket",
        },
        httparse::Header {
            name: "Host",
            value: b"localhost:9000",
        },
        httparse::Header {
            name: "Origin",
            value: b"http://localhost:9000",
        },
        httparse::Header {
            name: "Sec-WebSocket-Key",
            value: b"SGVsbG8sIHdvcmxkIQ==",
        },
        httparse::Header {
            name: "Sec-WebSocket-Version",
            value: b"13",
        },
    ];
    let mut req = httparse::Request::new(&mut headers);
    req.method = Some("GET");
    req.version = Some(1);
    req.path = Some("ws://localhost:9000/");

    client(req, tcp_stream)
}
