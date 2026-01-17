use std::collections::HashMap;
use tokio::sync::mpsc;

// Basic Ids
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct ClientId(u64);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoomId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoomMemberID(String);

// Each Room the server has
pub struct ChatRoom {
    pub id: RoomId,
    pub members: HashMap<ClientId, RoomMemberID>,
    pub next_member_id: u32,
}

impl ChatRoom {
    pub fn new(id: String) -> Self {
        Self {
            id: RoomId(id),
            members: HashMap::new(),
            next_member_id: 0,
        }
    }
}

// TCP Helpers
pub struct ClientSender {
    pub tx: mpsc::UnboundedSender<String>,
}

// Object of the clients. What they can do as well
pub struct Client {
    pub id: ClientId,           // The id for the client
    pub outbound: ClientSender, // The message Clients send
}


pub struct Server {
    pub rooms: HashMap<RoomId, ChatRoom>,
    pub clients: HashMap<ClientId, Client>,
    pub next_client_id: u64,
}

impl Server {
    pub fn new() -> Self {
        let rooms = (0..10)
            .map(|i| (RoomId(i.to_string()), ChatRoom::new(i.to_string())))
            .collect();
        Self {
            rooms: rooms,
            clients: HashMap::new(),
            next_client_id: 0,
        }
    }

    // This works on the asssumption the TCP connection is established
    // Furthermore it sends clients their id and adds them to the server's collection
    pub fn add_client(&mut self, outbound: ClientSender) -> ClientId {
        let client_id = ClientId(self.next_client_id);
        self.next_client_id += 1;

        let welcome_msg = format!("Your client ID is {}", client_id.0);
        let _ = outbound.tx.send(welcome_msg);

        let client = Client {
            id: client_id,
            outbound,
        };

        self.clients.insert(client_id, client);

        client_id
    }
    // The following function will remove a client from the list
    pub fn remove_client(&mut self, client_id: ClientId) {
        self.clients.remove(&client_id);

        for room in self.rooms.values_mut() {
            room.members.remove(&client_id);
        }
    }
    // Retrieve Client Info
    pub fn get_client(&self, client_id: &ClientId) -> Option<&Client> {
        self.clients.get(&client_id)
    }

    // Adding a client to a room 
    pub fn add_client_to_room(&mut self, client_id: ClientId, room_id: RoomId) -> Result<(), Errors> {
        let room = self.rooms
            .get_mut(&room_id)
            .ok_or(Errors::RoomNotFound)?;
        
        if room.members.contains_key(&client_id) {
            return Err(Errors::ClientInRoom);
        }

        let member_id = RoomMemberID(room.next_member_id.to_string());

        room.members.insert(client_id, member_id);
        room.next_member_id += 1;

        Ok(())
    }

    // Remove a client from room
    pub fn remove_client_from_room(&mut self, client_id: ClientId, room_id: RoomId) -> Result<(), Errors> {
        let room = self.rooms
            .get_mut(&room_id)
            .ok_or(Errors::RoomNotFound)?;

        if room.members.remove(&client_id).is_none() {
            return Err(Errors::ClientNotInRoom);
        }

        Ok(())
    }

    // Send the message in the room
    pub fn send_room_message(&self, from: ClientId, room_id: RoomId, message: String) -> Result<(), Errors> {
        let room = self.rooms.get(&room_id).ok_or(Errors::RoomNotFound)?;

        if !room.members.contains_key(&from) {
            return Err(Errors::ClientNotInRoom);
        }

        for clients in room.members.keys() {
            if let Some(client) = self.clients.get(clients) {
                let _ = client.outbound.tx.send(
                    format!("[{}] {}", from.0, message)
                );
            }
        }

        Ok(())
    }

    // List the rooms
    pub fn list_rooms(&self) -> Vec<RoomId> {
        self.rooms.keys().cloned().collect()
    }
}

#[derive(Debug)]
pub enum Errors {
    RoomNotFound,
    ClientInRoom,
    RoomFull,
    ClientNotInRoom,
}


#[cfg(test)]
mod tests{
    use super::*;
    use tokio::sync::mpsc;

    fn dummy_sender() -> (ClientSender, mpsc::UnboundedReceiver<String>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (ClientSender { tx }, rx)
    }

    #[test]
    fn add_client_assigns_unique_ids() {
        let mut server = Server::new();
        
        let (sender1, _) = dummy_sender();
        let (sender2, _) = dummy_sender();

        let c1 = server.add_client(sender1);
        let c2 = server.add_client(sender2);

        assert_ne!(c1, c2);
    }

}
