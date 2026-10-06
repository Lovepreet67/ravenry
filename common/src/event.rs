use std::{collections::HashMap, str::FromStr};

use uuid::Uuid;

pub struct Event {
    from: Uuid,
    destination: Uuid,
    payload: String,
}
impl From<Event> for HashMap<String, String> {
    fn from(value: Event) -> Self {
        let mut map = HashMap::new();
        map.insert("from".into(), value.from.into());
        map.insert("destination".into(), value.destination.into());
        map.insert("payload".into(), value.payload.into());
        map
    }
}

impl From<HashMap<String, String>> for Event {
    fn from(mut value: HashMap<String, String>) -> Self {
        Self {
            from: Uuid::from_str(value.get("from").expect("Error while getting from"))
                .expect("Invalid uuid"),
            destination: Uuid::from_str(
                value
                    .get("destination")
                    .expect("Error while getting destination"),
            )
            .expect("Invalid uuid"),
            payload: value
                .remove("payload")
                .expect("Error while getting destination"),
        }
    }
}
