use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PrinterId(Arc<String>);

impl PrinterId {
    pub fn from_id(id: &str) -> Self {
        Self(Arc::new(id.to_string()))
    }

    pub fn generate() -> Self {
        Self(Arc::new(nanoid::nanoid!()))
    }

    pub fn inner(&self) -> &Arc<String> {
        &self.0
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl From<Arc<String>> for PrinterId {
    fn from(arc: Arc<String>) -> Self {
        PrinterId(arc)
    }
}

impl From<String> for PrinterId {
    fn from(s: String) -> Self {
        PrinterId(Arc::new(s))
    }
}

impl Serialize for PrinterId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PrinterId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(PrinterId(Arc::new(s)))
    }
}
