#[derive(Debug)]
pub enum Message {
    Join { room: String },
    Chat { room: String, text: String },
    Quit,
}
