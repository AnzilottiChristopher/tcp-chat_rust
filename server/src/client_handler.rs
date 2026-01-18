use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::protocol::handle_client_message;
use common::{ChatClient, Server, RoomId};

pub async fn handle_client(stream: TcpStream, server: Arc<Mutex<Server>>) -> anyhow::Result<()> {
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
    let id_msg = format!("CLIENT_ID:{}\n", client_id.0);
    writer.write_all(id_msg.as_bytes()).await?;
    writer.flush().await?;

    {
        let mut server_lock = server.lock().await;
        let lobby = RoomId("0".to_string());
        if let Ok(()) = server_lock.add_client_to_room(client_id, &lobby) {
            if let Some(client) = server_lock.get_client(&client_id) {
                let _ = client
                    .message
                    .tx
                    .send("Welcome to the lobby! Use '/help' for commands".to_string())
                    .await;
            }
        }
    }

    // Writer Task: server -> client
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if writer.write_all(msg.as_bytes()).await.is_err() {
                println!("Failed to write to client");
                break;
            }
            if writer.write_all(b"\n").await.is_err() {
                println!("Failed to write to client");
                break;
            }
            if writer.flush().await.is_err() {
                println!("Failed to write to client");
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
        let mut server_lock = server.lock().await;
        let should_continue = handle_client_message(&mut server_lock, client_id, msg).await?;
        drop(server_lock);

        if !should_continue {
            break;
        }
    }
    // Cleanup on disconnect
    let mut server_lock = server.lock().await;
    server_lock.remove_client(client_id);
    println!("Client {} disconnected", client_id.0);
    Ok(())
}
