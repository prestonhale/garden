use pathfinding::directed::astar::astar;

use super::*;

const NEIGHBOR_DIRECTIONS: [(i32, i32); 4] = [
    (0, -1), // N
    (1, 0),  // E
    (0, 1),  // S
    (-1, 0), // W
];

pub fn a_star_pathfind(
    cur_pos: &Position,
    goal: &Position,
    ignored_position: &Vec<Position>,
    world: &World,
) -> (i32, Position) {
    let result = astar(
        cur_pos,
        // Create list of all position nighbors (giving cost 1 to all)
        |p| {
            let mut neighbors = Vec::new();
            'neighbor_loop: for (x_diff, y_diff) in NEIGHBOR_DIRECTIONS.iter() {
                let neighbor_x = p.x + x_diff;
                let neighbor_y = p.y + y_diff;
                if 0 <= neighbor_x
                    && neighbor_x < *world.get_width()
                    && 0 <= neighbor_y
                    && neighbor_y < *world.get_height()
                {
                    for i in 0..ignored_position.len() {
                        if ignored_position[i].x == neighbor_x && ignored_position[i].y == neighbor_y {
                            continue 'neighbor_loop;
                        }
                    }
                    neighbors.push((
                        Position {
                            x: neighbor_x,
                            y: neighbor_y,
                        },
                        1,
                    ))
                }
            }
            neighbors
        },
        // Manhattan distance heuristic
        |p| ((p.x - goal.x).abs() + (p.y - goal.y).abs()) / 3,
        // Check if (p)osition is goal
        |p| p == goal,
    );
    match result {
        Some((p, c)) => {
            // If we're somehow already standing on the object, return pretend its a square away
            // This shouldn't happen though, fix it
            if p.len() == 1 {
                return (1, p[0]);
            }
            return (c, p[1]);
        }
        None => panic!("No path to goal found"),
    }
}

#[test]
fn test_default() {
    let world = &World::new(2, 2);
    let cur_pos = &Position{x: 0, y: 0};
    let goal_pos = &Position{x: 1, y: 1};
    
    let (_, next_pos) = a_star_pathfind(cur_pos, goal_pos, &vec![], world);
    
    let expected_pos = Position{x:1, y:0};
    assert_eq!(next_pos, expected_pos);
}

#[test]
fn test_ignored_position() {
    let world = &World::new(2, 2);
    let cur_pos = &Position{x: 0, y: 0};
    let goal_pos = &Position{x: 1, y: 1};
    let ignored_position = &vec![Position{x:1, y:0}];
    
    let (_, next_pos) = a_star_pathfind(cur_pos, goal_pos, ignored_position, world);
    
    let expected_pos = Position{x:0, y:1};
    assert_eq!(next_pos, expected_pos);
}
