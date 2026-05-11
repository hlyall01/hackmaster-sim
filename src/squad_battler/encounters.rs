//! Route nodes and enemy squad generation.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SquadNodeKind {
    Fight,
    Recruit,
    Event,
    Elite,
    Boss,
    Rest,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SquadRouteNode {
    pub id: usize,
    pub floor: u32,
    pub lane: u32,
    pub kind: SquadNodeKind,
    pub completed: bool,
}

pub fn placeholder_route() -> Vec<SquadRouteNode> {
    vec![
        SquadRouteNode {
            id: 0,
            floor: 0,
            lane: 0,
            kind: SquadNodeKind::Fight,
            completed: false,
        },
        SquadRouteNode {
            id: 1,
            floor: 0,
            lane: 1,
            kind: SquadNodeKind::Recruit,
            completed: false,
        },
        SquadRouteNode {
            id: 2,
            floor: 0,
            lane: 2,
            kind: SquadNodeKind::Fight,
            completed: false,
        },
    ]
}
