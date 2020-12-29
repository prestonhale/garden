use petgraph::graphmap::{UnGraphMap};
use petgraph::algo::{astar};

use super::*;

fn graph_from_world(world: &World) -> UnGraphMap<(u8, u8), ()> {
    let mut g = UnGraphMap::new();
    for x in 0..world.width {
        for y in 0..world.height {

            let node = g.add_node((x, y));
            // Add all neighbors to cell
            for (x_diff, y_diff) in [
                (0, -1), // N
                (1, 0), // E
                (0, 1), // S
                (-1, 0) // W
            ].iter()
            {
                let neighbor_x = x as i8 + x_diff;
                let neighbor_y = y as i8 + y_diff;
                if neighbor_x < 0 
                    || neighbor_x >= world.width as i8 
                    || neighbor_y < 0 
                    || neighbor_y >= world.height as i8 { 
                        continue };
                let neighbor = g.add_node((neighbor_x as u8, neighbor_y as u8));
                g.add_edge(node, neighbor, ());
            }

        }
    }
    g
}

pub fn a_star_pathfind(cur_pos: &Position, goal: &Position, world: &World) -> (usize, Position) {
    let mut graph = graph_from_world(world);
    let a = graph.add_node((cur_pos.x, cur_pos.y));
    let f = graph.add_node((goal.x, goal.y));
    // TODO: Fix final param which should be manhattan distance heuristic
    if let Some(result) = astar(&graph, a, |finish| finish == f, |_| 1, |_| 0) {
        let (cost, path) = result;
        if cost == 0 {
            panic!("Called for pathfinding but already on goal square");
        }
        (cost, Position { x: path[1].0, y: path[1].1 })
    } else {
        panic!("Goal is unreachable!");
    }
}

mod test {
    #[allow(unused_imports)] // Not actually unused, looks like a bug
    use super::*; 

    #[test]
    fn test_graph_from_world() {
        let world = World::new(3, 3);
        let graph = graph_from_world(&world);

        assert_eq!(graph.node_count(), 9);
        assert_eq!(graph.edge_count(), 12);
    }
}