use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct ResourceID
{
    uri: String,
}

impl ResourceID{
    pub fn from_string(source: &str) -> Self {
        Self{
            uri: source.to_string()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.uri.is_empty()
    }
}