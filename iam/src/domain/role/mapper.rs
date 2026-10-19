use crate::domain::role::{dto, model};

impl From<model::Role> for dto::Role {
    fn from(value: model::Role) -> Self {
        Self {
            id: value.id,
            title: value.title,
            description: value.description,
            slug: value.slug,
            created_at: value.created_at,
        }
    }
}
