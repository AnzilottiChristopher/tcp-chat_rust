// Tokio async runtime
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, AsyncBufReadExt};
use tokio::sync::mpsc;
use tokio::select;

// Standard library
use std::error::Error;

// Common Crate
use common::{ChatClient, ClientId, RoomId, Client};

pub struct RuntimeClient {
    pub client: Client,
    pub rx: mpsc::Receiver<String>,
    pub server_addr: String,
}   
impl RuntimeClient {
    pub fn new(id: ClientId, server_addr: String) -> Self {
        let(tx, rx) = mpsc::channel(100);
        let chat_client = ChatClient { tx };
        let client = Client { id, message: chat_client };

        Self { client, rx, server_addr }
    }
    pub async fn send_message(&self, msg: String) -> Result<(), common::Errors> {
        self.client.send(msg).await
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    let (tx, mut rx): (mpsc::Sender<String>, mpsc::Receiver<String>) = mpsc::channel(100);

    let client = Client {
        id: ClientId (0),
        message: ChatClient { tx },
    };

    let mut client = RuntimeClient {
        client: client,
        rx: rx, // RX will read from the channels buffer, TX sends to the buffer to be read
        server_addr: "127.0.0.1:8080".to_string()
    };


    // Connection with "server" starts here
    let stream = TcpStream::connect(client.server_addr).await?;

    // Main will end when reaches this and will shut down other threads
    Ok(())
}

