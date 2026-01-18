// Tokio async runtime
use std::io;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

// Standard library
use std::error::Error;

// Common Crate
use common::{ChatClient, Client, ClientId, RoomId};

pub struct RuntimeClient {
    pub client: Client,
    pub name: String,
    pub rx: mpsc::Receiver<String>,
    pub server_addr: String,
}
impl RuntimeClient {
    pub fn new(id: ClientId, server_addr: String, name: String) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let chat_client = ChatClient { tx };
        let client = Client {
            id,
            message: chat_client,
            current_room: Some(RoomId("0".to_string())),
        };

        Self {
            client,
            name,
            rx,
            server_addr,
        }
    }
    pub async fn send_message(&self, msg: String) -> Result<(), common::Errors> {
        self.client.send(msg).await?;
        Ok(())
    }
    pub async fn run(mut self) -> Result<(), Box<dyn Error>> {
        let stream = TcpStream::connect(&self.server_addr).await?;
        let (reader, mut writer) = stream.into_split();

        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        reader.read_line(&mut line).await?;

        if line.starts_with("CLIENT_ID:") {
            let id_str = line["CLIENT_ID:".len()..].trim();
            let id: u64 = id_str.parse().expect("Invalid Client Id from Server");
            self.client.id = ClientId(id); // This updates to correct client id 
            println!("Assigned Client ID: {}", id);
        }

        // Writer Tasks, takes inputs from terminal to be sent to server
        let mut rx = self.rx;
        let writer_handle = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let _ = writer.write_all(msg.as_bytes()).await;
                let _ = writer.write_all(b"\n").await;
            }
        });

        // Reader task, reading from TCPSTREAM
        //let tx = self.client.message.tx.clone();
        let reader_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                println!("From Server: {}", line);
                line.clear();
            }
            println!("Connection closed by server");
        });

        // Print to terminal

        // Stdin loop
        let mut stdin = BufReader::new(tokio::io::stdin());
        let mut input = String::new();
        loop {
            input.clear();
            stdin.read_line(&mut input).await?;
            let msg = input.trim();
            if !msg.is_empty() {
                if msg.eq_ignore_ascii_case("/quit") {
                    self.client.message.tx.send(msg.to_string()).await?;
                    break;
                }
                self.client.message.tx.send(msg.to_string()).await?;
            }
        }

        let _ = tokio::time::timeout(tokio::time::Duration::from_secs(2), reader_handle).await;

        writer_handle.abort();
        println!("Disconnected from server");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut name: String = "None".to_string();
    println!("Please Enter A Username: ");
    while name == "None".to_string() {
        io::stdin()
            .read_line(&mut name)
            .expect("Failed to read line");
    }
    let client = RuntimeClient::new(ClientId(0), "127.0.0.1:8080".to_string(), name);

    client.run().await
}

fn display_to_terminal(line: String) {
    println!("{}", line);
}
