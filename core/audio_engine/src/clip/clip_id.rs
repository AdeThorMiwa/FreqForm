#[derive(Debug, PartialEq)]
pub struct ClipId(String);

impl From<uuid::Uuid> for ClipId {
    fn from(value: uuid::Uuid) -> Self {
        Self(value.to_string())
    }
}
