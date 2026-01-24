use super::Vec2;
use spacetimedb::*;

#[derive(SpacetimeType)]
pub enum MoveIntent {
    None,
    Dir(Vec2),
    Path(Vec<Vec2>),
}
