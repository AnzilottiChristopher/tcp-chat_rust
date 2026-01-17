use tokio::net::TcpStream;
use tokio::io::{BufReader, AsyncBufReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

use std::sync::Arc;
use tokio::sync::Mutex;

use common::{Server, ChatClient};
use crate::protocol::handle_client_message;

pub async fn handle_client(stream: TcpStream, server: Arc<Mutex<Server>>,) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Creating channel for sending messages to this client 
    let (tx, mut rx) = mpsc::channel::<String>(100);
    let chat_client = ChatClient { tx };

    // Register client and get client_id
    let client_id = {
        let mut server_lock = server.lock().await;
        server_lock.add_client(chat_client).await?
    };

    // Writer Task: server -> client 
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if writer.write_all(msg.as_bytes()).await.is_err() {
                break;
            }
        }
    });

    // Reader loop: client -> server 
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).await?;
        if bytes == 0 {
            break;
        }

        let msg = line.trim().to_string();

        // Lock server to handle message 
        println!("Received raw message from {}: {:?}", client_id.0, msg);
        let mut server_lock = server.lock().await;
        handle_client_message(&mut server_lock, client_id, msg).await?;

    }
        // Cleanup on disconnect 
        let mut server_lock = server.lock().await;
        server_lock.remove_client(client_id);

        Ok(())
}
