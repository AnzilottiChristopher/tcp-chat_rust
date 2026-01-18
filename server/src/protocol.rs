use anyhow::Result;
use common::{ClientId, RoomId, Server};

pub async fn handle_client_message(
    server: &mut Server,
    client_id: ClientId,
    msg: String,
) -> Result<bool> {
    eprintln!("Server received from {:?}: {}", client_id, msg);
    if msg.starts_with("/quit") {
        if let Some(client) = server.get_client(&client_id) {
            let _ = client.message.tx.send("Goodbye!".to_string()).await;
        }
        return Ok(false);
    } else if msg.starts_with("/join ") {
        let room_name = msg.trim_start_matches("/join ").to_string();

        // Check if client is in a room
        let old_room = server
            .get_client(&client_id)
            .and_then(|c| c.current_room.clone());
        if let Some(old_room_id) = old_room {
            let _ = server.remove_client_from_room(client_id, &old_room_id);
        }

        match server.add_client_to_room(client_id, &RoomId(room_name.clone())) {
            Ok(()) => {
                if let Some(client) = server.get_client(&client_id) {
                    let _ = client
                        .message
                        .tx
                        .send(format!("You joined room '{}'", room_name))
                        .await;
                    //println!("IN HERE");
                }
            }
            Err(e) => {
                if let Some(client) = server.get_client(&client_id) {
                    let _ = client
                        .message
                        .tx
                        .send(format!("Failed to join room: {:?}", e))
                        .await;
                }
            }
        }
        return Ok(true);
    } else if msg == "/leave" {
        let old_room = server
            .get_client(&client_id)
            .and_then(|c| c.current_room.clone());

        if let Some(room_id) = old_room {
            let room_name = room_id.0.clone();

            match server.remove_client_from_room(client_id, &room_id) {
                Ok(()) => {
                    if let Some(client) = server.get_client(&client_id) {
                        let _ = client
                            .message
                            .tx
                            .send(format!("You left room {}", room_name))
                            .await;
                    }

                    // Auto-join lobby (room "0")
                    let lobby = RoomId("0".to_string());
                    match server.add_client_to_room(client_id, &lobby) {
                        Ok(()) => {
                            if let Some(client) = server.get_client(&client_id) {
                                let _ = client
                                    .message
                                    .tx
                                    .send("You joined the lobby".to_string())
                                    .await;
                            }
                        }
                        Err(_) => {}
                    }
                }
                Err(e) => {
                    if let Some(client) = server.get_client(&client_id) {
                        let _ = client
                            .message
                            .tx
                            .send(format!("Failed to leave room: {:?}", e))
                            .await;
                    }
                }
            }
        } else {
            if let Some(client) = server.get_client(&client_id) {
                let _ = client
                    .message
                    .tx
                    .send("You're not in any room".to_string())
                    .await;
            }
        }
        return Ok(true);
    } else if msg == "/help" {
        if let Some(client) = server.get_client(&client_id) {
            let _ = client.message.tx.send("Commands: /join 1-10 (Joins a room), /leave (Leaves the current room), /quit (quits the program), /help (Lists the commands)".to_string()).await;
        }
    } else {
        let current_room = server
            .get_client(&client_id)
            .and_then(|c| c.current_room.clone());

        match current_room {
            Some(room_id) if room_id.0 == "0" => {
                println!("In lobby = 0");
                if let Some(client) = server.get_client(&client_id) {
                    let _ = client
                        .message
                        .tx
                        .send("You are in the lobby. Use '/help' for commands or '/join 1-10' to join a room".to_string());
                }
            }
            Some(room_id) => {
                server.send_room_message(client_id, &room_id, msg).await?;
            }
            None => {
                if let Some(client) = server.get_client(&client_id) {
                    let _ = client
                        .message
                        .tx
                        .send(
                            "You're not in any room. Use '/help' for a list of commands"
                                .to_string(),
                        )
                        .await;
                }
            }
        }
    }

    Ok(true)
}
