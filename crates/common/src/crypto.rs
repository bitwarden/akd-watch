use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct SigningKey {
    key: ed25519_dalek::SigningKey,
    key_id: Uuid,
}
