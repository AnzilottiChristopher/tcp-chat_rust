use std::{collections::HashMap, fmt::write};
use tokio::sync::mpsc;
use std::fmt;

// Basic Ids
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct ClientId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoomId(pub String);

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
pub struct ChatClient {
    pub tx: mpsc::Sender<String>,
}

// Object of the clients. What they can do as well
pub struct Client {
    pub id: ClientId,        // The id for the client
    pub message: ChatClient, // The message Clients send
}

impl Client {
    // Send a message to server
    pub async fn send(&self, msg: String) -> Result<(), Errors> {
        self.message
            .tx
            .send(msg)
            .await
            .map_err(|_| Errors::SendFailed)
    }
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
    pub async fn add_client(&mut self, outbound: ChatClient) -> Result<ClientId, Errors> {
        let client_id = ClientId(self.next_client_id);
        self.next_client_id += 1;

        let welcome_msg = format!("Your client ID is {}", client_id.0);
        outbound
            .tx
            .send(welcome_msg)
            .await
            .map_err(|_| Errors::SendFailed)?;

        let client = Client {
            id: client_id,
            message: outbound,
        };

        self.clients.insert(client_id, client);

        Ok(client_id)
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
    pub fn add_client_to_room(
        &mut self,
        client_id: ClientId,
        room_id: &RoomId,
    ) -> Result<(), Errors> {
        let room = self.rooms.get_mut(room_id).ok_or(Errors::RoomNotFound)?;

        if room.members.contains_key(&client_id) {
            return Err(Errors::ClientInRoom);
        }

        let member_id = RoomMemberID(room.next_member_id.to_string());

        room.members.insert(client_id, member_id);
        room.next_member_id += 1;

        Ok(())
    }

    // Remove a client from room
    pub fn remove_client_from_room(
        &mut self,
        client_id: ClientId,
        room_id: &RoomId,
    ) -> Result<(), Errors> {
        let room = self.rooms.get_mut(room_id).ok_or(Errors::RoomNotFound)?;

        if room.members.remove(&client_id).is_none() {
            return Err(Errors::ClientNotInRoom);
        }

        Ok(())
    }

    // Send the message in the room
    pub async fn send_room_message(
        &self,
        from: ClientId,
        room_id: &RoomId,
        message: String,
    ) -> Result<(), Errors> {
        let room = self.rooms.get(room_id).ok_or(Errors::RoomNotFound)?;

        if !room.members.contains_key(&from) {
            return Err(Errors::ClientNotInRoom);
        }

        for clients in room.members.keys() {
            if let Some(client) = self.clients.get(clients) {
                client
                    .message
                    .tx
                    .send(format!("[{}] {}", from.0, message))
                    .await
                    .map_err(|_| Errors::SendFailed)?;
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
    SendFailed,
}

impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for Errors {}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn dummy_sender() -> (ChatClient, mpsc::Receiver<String>) {
        let (tx, rx) = mpsc::channel(100);
        (ChatClient { tx }, rx)
    }

    #[tokio::test]
    async fn add_client_assigns_unique_ids() {
        let mut server = Server::new();

        let (sender1, _rx1) = dummy_sender();
        let (sender2, _rx2) = dummy_sender();

        let c1 = server.add_client(sender1).await.unwrap();
        let c2 = server.add_client(sender2).await.unwrap();

        assert_ne!(c1, c2);
    }

    #[tokio::test]
    async fn client_can_join_room() {
        let mut server = Server::new();

        let (sender, _rx) = dummy_sender();

        let client_id = server.add_client(sender).await.unwrap();
        let room_id = RoomId("0".to_string());

        assert!(server.add_client_to_room(client_id, &room_id).is_ok());
    }
}
