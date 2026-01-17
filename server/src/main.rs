use tokio::net::TcpListener;
use tokio::sync::Mutex;
use std::sync::Arc;

use common::Server;

mod client_handler;
mod protocol;

use client_handler::handle_client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    let server = Arc::new(Mutex::new(Server::new()));

    println!("Server is listening on 127.0.0.1:8080");

    loop {
        let (stream, addr) = listener.accept().await?;
        println!("Client Connected: {:?}", addr);

        let server = Arc::clone(&server);

        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, server).await {
                eprintln!("Client Error: {:?}", e);
            }
        });
    }
}
