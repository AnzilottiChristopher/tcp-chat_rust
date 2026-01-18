use anyhow::Result;
use common::{ClientId, Errors, RoomId, Server};

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
    }
    if msg.starts_with("/join ") {
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
    } else if msg.starts_with("/leave ") {
        let room_name = msg.trim_start_matches("/leave ").to_string();
        match server.remove_client_from_room(client_id, &RoomId(room_name.clone())) {
            Ok(()) => {
                if let Some(client) = server.get_client(&client_id) {
                    let _ = client
                        .message
                        .tx
                        .send(format!("You left room '{}'", room_name))
                        .await;
                    //println!("In leave message");
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
        return Ok(true);
    } else {
        //println!("In Else statement");
        if let Some(client) = server.get_client(&client_id) {
            if let Some(room_id) = &client.current_room {
                server.send_room_message(client_id, room_id, msg).await?;
            } else {
                if let Some(client) = server.get_client(&client_id) {
                    //println!("No server joined");
                    let _ = client
                        .message
                        .tx
                        .send("You're not in any room. Use '/join 1-10' to join a room".to_string())
                        .await;
                }
            }
        }
    }

    Ok(true)
}
