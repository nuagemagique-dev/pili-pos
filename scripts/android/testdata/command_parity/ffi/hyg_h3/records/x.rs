#[derive(uniffi::Record)]
pub struct NoSerde {
    pub id: i64,
}

#[derive(uniffi::Record, serde::Serialize, serde::Deserialize)]
pub struct NoDeny {
    pub id: i64,
}

#[derive(uniffi::Enum, serde::Serialize)]
pub enum HalfSerde {
    A,
}
