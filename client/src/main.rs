// Tokio async runtime
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::mpsc;

// Standard library
use std::error::Error;

// Common Crate
use common::{ChatClient, Client, ClientId, RoomId};

pub struct RuntimeClient {
    pub client: Client,
    pub rx: mpsc::Receiver<String>,
    pub server_addr: String,
}
impl RuntimeClient {
    pub fn new(id: ClientId, server_addr: String) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let chat_client = ChatClient { tx };
        let client = Client {
            id,
            message: chat_client,
        };

        Self {
            client,
            rx,
            server_addr,
        }
    }
    pub async fn send_message(&self, msg: String) -> Result<(), common::Errors> {
        self.client.send(msg).await?;
        Ok(())
    }
    pub async fn run(self) -> Result<(), Box<dyn Error>> {
        let stream = TcpStream::connect(&self.server_addr).await?;
        let (reader, mut writer) = stream.into_split();

        // Writer Tasks
        let mut rx = self.rx;
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let _ = writer.write_all(msg.as_bytes()).await;
                let _ = writer.write_all(b"\n").await;
            }
        });

        // Reader task
        let tx = self.client.message.tx.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                let _ = tx.send(line.clone()).await;
                line.clear();
            }
        });

        // Stdin loop
        let mut stdin = BufReader::new(tokio::io::stdin());
        let mut input = String::new();
        loop {
            input.clear();
            stdin.read_line(&mut input).await?;
            let msg = input.trim();
            if !msg.is_empty() {
                self.client.message.tx.send(msg.to_string()).await?;
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = RuntimeClient::new(ClientId(0), "127.0.0.1:8080".to_string());

    client.run().await
}
