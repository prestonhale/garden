use petgraph::graphmap::{UnGraphMap};
use petgraph::algo::{astar};
use petgraph::visit::GraphBase;

use super::*;

type GardenGraph = UnGraphMap<(i32, i32), ()>;

fn graph_from_world(world: &World) -> GardenGraph {
    let mut g = UnGraphMap::new();
    for x in 0..*world.get_width() {
        for y in 0..*world.get_height() {

            let node = g.add_node((x, y));
            // Add all neighbors to cell
            for (x_diff, y_diff) in [
                (0, -1), // N
                (1, 0), // E
                (0, 1), // S
                (-1, 0) // W
            ].iter()
            {
                let neighbor_x = x + x_diff;
                let neighbor_y = y + y_diff;
                if neighbor_x < 0 
                    || neighbor_x >= world.width
                    || neighbor_y < 0 
                    || neighbor_y >= world.height { 
                        continue };
                let neighbor = g.add_node((neighbor_x, neighbor_y));
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
    let edge_cost_one = |_| { 1 };
    let manhattan_distance_heuristic = |cur_node: <GardenGraph as GraphBase>::NodeId| {
        (a.0 - cur_node.0) + (a.1 - cur_node.1)
    };
    if let Some(result) = astar(&graph, a, |finish| finish == f, edge_cost_one, manhattan_distance_heuristic) {
        let (cost, path) = result;
        if cost == 0 {
            panic!("Called for pathfinding but already on goal square");
        }
        (cost as usize, Position { x: path[1].0, y: path[1].1 })
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