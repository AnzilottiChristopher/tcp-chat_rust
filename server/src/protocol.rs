use common::{ClientId, Errors, RoomId, Server};
use anyhow::Result;

pub async fn handle_client_message(server: &mut Server, client_id: ClientId, msg: String) -> Result<()> {
    eprintln!("Server received from {:?}: {}", client_id, msg);
    if msg.starts_with("/join ") {
        let room_name = msg.trim_start_matches("/join ").to_string();
        server.add_client_to_room(client_id, &RoomId(room_name))?;
    } else if msg.starts_with("/leave ") {
        let room_name = msg.trim_start_matches("/leave ").to_string();
        server.remove_client_from_room(client_id, &RoomId(room_name))?;
    } else {
        // Default broadcast to room 0
        server.send_room_message(client_id, &RoomId("0".to_string()), msg).await?;
    }

    Ok(())
}
