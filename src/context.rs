use crate::position_map::PositionMap;

pub struct Context<'a> {
    position_map: &'a PositionMap,
}

impl<'a> Context<'a> {
    pub fn new(position_map: &'a PositionMap) -> Self {
        Self { position_map }
    }

    pub fn position_map(&self) -> &'a PositionMap {
        &self.position_map
    }
}
