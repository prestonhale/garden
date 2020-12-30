use std::collections::HashMap;

use rand::prelude::*;
use rand::distributions::{Distribution, Standard};

use serde::{Serialize, Deserialize};

mod pathfinding;

pub struct World{
    pub width: i32,
    pub height: i32,
    // Sync and Send are required to ensure entities are thread-safe
    entities: Vec<EntityType>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

pub enum Direction {
    Up,
    Right,
    Down,
    Left
}

impl Distribution<Direction> for Standard{
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Direction {
        match rng.gen_range(0..4) {
            0 => Direction::Up,
            1 => Direction::Right,
            2 => Direction::Down,
            _ => Direction::Left,
        }
    }
}

#[derive(Serialize)]
pub struct RenderedEntity<'a> {
    // console renderer directly accesses these fields
    pub position: &'a Position,
    pub color: &'a str,
}

type EntityType = Box<dyn Updateable + Sync + Send>;

impl World {
    pub fn new(width: i32, height: i32) -> World{
        World {
            height: height,
            width: width,
            entities: vec![],
        }
    }

    pub fn default() -> World {
        let entities: Vec<EntityType> = vec![
            Box::new(eater::Eater::new()),
            Box::new(food_spawner::FoodSpawner::new(0, 20)),
            Box::new(food::Food::new(Position { x: 20, y: 20 })),
        ];

        World{
            width: 30,
            height: 30,
            entities: entities,
        }
    }

    pub fn get_height(&self) -> &i32 { &self.height}
    pub fn get_width(&self) -> &i32 { &self.width }

    pub fn add_entity(&mut self, entity: EntityType) {
        self.entities.push(entity);
    }

    pub fn render(&self) -> Vec<RenderedEntity> { 
        let mut rendered_entities = vec![];
        for entity in self.entities.iter() {
            rendered_entities.push(RenderedEntity{
                position: entity.get_position(),
                color: entity.get_color(),
            });
        }
        rendered_entities
    }

    fn get_food_entities(&self) -> Vec<usize> {
        let mut food_entity_indices = vec![];
        for (i, entity) in self.entities.iter().enumerate() {
            if entity.get_tag() == "food" {
                food_entity_indices.push(i);
            }
        }
        food_entity_indices
    }

    fn get_new_position(&self, cur_position: &Position, direction: &Direction) -> Position {
        let mut new_position: Position;
        match direction {
            Direction::Up => {
                if cur_position.y == 0 {
                    new_position = Position{
                        x: cur_position.x, 
                        y: cur_position.y
                    }
                } else {
                    new_position = Position{
                        x: cur_position.x, 
                        y: cur_position.y - 1
                    }
                }
            },
            Direction::Right => {
                if cur_position.x == self.width - 1 {
                    new_position = Position{
                        x: cur_position.x, 
                        y: cur_position.y
                    }
                } else {
                    new_position = Position{
                        x: cur_position.x + 1, 
                        y: cur_position.y
                    }
                }
            },
            Direction::Down => {
                if cur_position.y == self.height - 1 {
                    new_position = Position{
                        x: cur_position.x, 
                        y: cur_position.y
                    }
                } else {
                    new_position = Position{
                        x: cur_position.x, 
                        y: cur_position.y + 1
                    }
                }
            },
            Direction::Left => {
                if cur_position.x == 0 {
                    new_position = Position{
                        x: cur_position.x, 
                        y: cur_position.y
                    }
                } else {
                    new_position = Position{
                        x: cur_position.x - 1, 
                        y: cur_position.y
                    }
                }
            }
        }
        
        if new_position.x >= self.width {
            new_position.x = cur_position.x;
        }
        if new_position.y >= self.height {
            new_position.y = cur_position.y;
        }
        new_position
    }

    // TODO: Generalize randomizer
    pub fn update(&mut self, randomizer: &mut rand_pcg::Pcg32) {
        let mut spawned_entities = Vec::new();
        let mut removed_entity_indices: Vec<usize> = Vec::new();
        for i in 0..self.entities.len() {
            
            for j in removed_entity_indices.iter() {
                if i == *j {
                    continue;  // Entity has already been destroyed in this update cycle
                }
            }

            let (entity, spawned_entity, removed_entity_index) = self.entities[i].update(&self, randomizer);

            // Replace entity state with new state
            self.entities[i] = entity;
            if let Some(e) = spawned_entity {
                spawned_entities.push(e);
            }
            if let Some(i) = removed_entity_index {
                removed_entity_indices.push(i)
            }
        }
        self.entities.append(&mut spawned_entities);
        for i in removed_entity_indices {
            // Removing from the "middle" could end up very expensive
            self.entities.remove(i);
        }
    }

    pub fn render_to_string(&self) -> Vec<String>{
        let mut lines: Vec<String> = Vec::new();
        for y in 0..*self.get_height() {
            let mut line = String::from("");
            for x in 0..*self.get_width(){
                let mut found_entity = false;
                for entity in self.entities.iter() {
                    if x == entity.get_position().x && y == entity.get_position().y {
                        line.push_str("🍓");
                        found_entity = true;
                    }
                }
                if !found_entity {
                    line.push_str("  ");
                }
            }
            lines.push(line);
        }
        lines
    }

}

pub const RED: &str = "#ff0000";
pub const BROWN: &str = "#996600";
pub const BLACK: &str = "#000000";
pub const GREEN: &str = "#009933";

pub trait Updateable {
    fn update(&self, world: &World, rng: &mut rand_pcg::Pcg32) -> (EntityType, Option<EntityType>, Option<usize>);
    fn get_position(&self) -> &Position;
    fn get_tag(&self) -> &str { "untagged" }
    fn get_color(&self) -> &str;
}

trait Spawner {
    fn spawn(&self, world: &World) -> Option<EntityType>;
}

mod food_spawner {
    use super::*;

    #[derive(Clone, Copy)]
    pub struct FoodSpawner {
        last_spawned: i32,
        spawn_every_x_ticks: i32,
    }

    impl Updateable for FoodSpawner {
        fn update(&self, world: &World, rng: &mut rand_pcg::Pcg32) -> (EntityType, Option<EntityType>, Option<usize>) {
            let mut new_spawner = self.clone();
            if self.last_spawned + 1 >= self.spawn_every_x_ticks {
                let x = rng.gen_range(0..world.width);
                let y = rng.gen_range(0..world.height);
                new_spawner.last_spawned = 0;
                let new_food = food::Food::new(Position{ x, y });
                (Box::new(new_spawner), Some(Box::new(new_food)), None)
            } else {
                new_spawner.last_spawned += 1;
                (Box::new(new_spawner), None, None)
            }
        }

        fn get_color(&self) -> &str {GREEN} // Hack to make appear invisible

        fn get_position(&self) -> &Position { 
            &Position{ x:0, y: 0}
        }
    }

    impl FoodSpawner {
        pub fn new(last_spawned: i32, spawn_every_x_ticks: i32) -> FoodSpawner {
            FoodSpawner {
                last_spawned,
                spawn_every_x_ticks
            }
        }
    }

    #[test]
    fn test_food_spawner() {
        let entities: Vec<EntityType> = vec![
            Box::new(food_spawner::FoodSpawner{
                last_spawned: 9,
                spawn_every_x_ticks: 10
            })
        ];
        let mut world = World::new(
            10,
            10,
        );
        world.entities = entities;
        let mut randomizer = rand_pcg::Pcg32::from_seed(*b"somebody once to");
        world.update(&mut randomizer);
        assert_eq!(world.entities.len(), 2);
    }
}

mod food {
    use super::*;
    #[derive(Clone, Copy)]
    pub struct Food {
        position: Position
    }

    impl Updateable for Food {
        fn update(&self, _world: &World, _rng: &mut rand_pcg::Pcg32) -> (EntityType, Option<EntityType>, Option<usize>) { 
            let new_food = self.clone();
            (Box::new(new_food), None, None)
        }
        
        fn get_position(&self) -> &Position { &self.position }

        fn get_color(&self) -> &str {RED}
        
        fn get_tag(&self) -> &str { "food" }
    }

    impl Food {
        pub fn new(position: Position) -> Food {
            println!("Spawning food at {:?}", position);
            Food { position: position }
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
enum Desire {
    Hunger
}


// Basic entity concerned only with eating
mod eater {
    use super::*;

    #[derive(Clone)]
    pub struct Eater {
        position: Position,
        desires: HashMap<Desire, i8>,
        desire_threshold: HashMap<Desire, i8>,
    }

    #[derive(Debug, PartialEq)]
    enum EaterGoal {
        GetFood(usize), // Approach or consume food entity
        Wander, // Move randomly
    }

    impl Updateable for Eater {
        fn update(&self, world: &World, rand_gen: &mut rand_pcg::Pcg32) -> (EntityType, Option<EntityType>, Option<usize>) {
            let mut new_eater = self.clone();
            let mut removed_entity_index = None;
            
            let new_desire = new_eater.increment_desire(Desire::Hunger, 1);
            if new_desire > 99 {
                // TODO: Starve
                // Normally we'd starve here, temporarily clamp to 0-99
                new_eater.increment_desire(Desire::Hunger, -1);
            }

            let goal = self.select_goal(world);
            match goal {
                EaterGoal::Wander => {
                    let direction: Direction = rand_gen.sample(Standard);
                    let next_position = world.get_new_position(&self.position, &direction);
                    new_eater.position = next_position;
                },
                EaterGoal::GetFood(i) => {
                    let food_entity = &world.entities[i];
                    let (cost, next_position) = self.pathfind(food_entity.get_position(), world);
                    if cost == 1 { // Eater is adjacent to food
                        removed_entity_index = Some(i);
                        new_eater.increment_desire(Desire::Hunger, -20);
                    } else {
                        new_eater.position = next_position;
                    }
                }
            }
            (Box::new(new_eater), None, removed_entity_index)
        }
        
        fn get_color(&self) -> &str {BROWN}
        fn get_position(&self) -> &Position { &self.position }
    }

    impl Eater {
        pub fn new() -> Eater {
            let mut desires = HashMap::new();
            desires.insert(Desire::Hunger, 0);

            let mut desire_threshold = HashMap::new();
            desire_threshold.insert(Desire::Hunger, 20);

            Eater {
                position: Position{ x:0, y:0 },
                desires: desires,
                desire_threshold: desire_threshold,
            }
        }

        fn set_desire(&mut self, desire: Desire, level: i8) {
            self.desires.insert(desire, level);
        }

        fn get_desire(&self, desire: Desire) -> i8 {
            match self.desires.get(&desire) {
                Some(i) => *i,
                None => panic!("Asked for desire that isn't on entity")
            }
        }

        // 0-99 acceptable; 100 is a death state
        fn increment_desire(&mut self, desire: Desire, increment: i8) -> i8 {
            let mut new_desire = self.get_desire(desire) + increment;
            if new_desire < 0 { new_desire = 0 }
            self.set_desire(desire, new_desire);
            new_desire
        }

        fn get_desire_threshold(&self, desire: Desire) -> i8 {
            match self.desire_threshold.get(&desire) {
                Some(i) => *i,
                None => panic!("Asked for desire threshold that isn't on entity")
            }
        }

        fn select_goal(&self, world: &World) -> EaterGoal {
            let goal: EaterGoal;
            let entities = self.get_line_of_sight_entities(world);
            if self.get_desire(Desire::Hunger) > self.get_desire_threshold(Desire::Hunger) 
                && entities.len() > 0
            {
                goal = EaterGoal::GetFood(entities[0])
            } else {
                goal = EaterGoal::Wander;
            }
            goal
        }

        fn get_line_of_sight_entities<'a>(&self, world: &'a World) -> Vec<usize>{
            // Omniscient
            world.get_food_entities()
        }

        fn pathfind(&self, goal: &Position, world: &World) -> (usize, Position) {
        pathfinding::a_star_pathfind(&self.position, goal, world)
        }
    }

    #[test]
    fn test_eater_wander_goal() {
        let mut world = World::new(10, 10);
        let food = Box::new(food::Food::new(Position { x: 0, y: 0 }));
        world.add_entity(food);
        let eater = Eater::new();
        let goal = eater.select_goal(&world);
        assert_eq!(EaterGoal::Wander, goal);
    }

    // TODO: Private method, remove when fails
    #[test]
    fn test_eater_food_goal() {
        let mut world = World::new(10, 10);
        let food = Box::new(food::Food::new(Position { x: 0, y: 0 }));
        world.add_entity(food);
        let mut eater = Eater::new();
        eater.set_desire(Desire::Hunger, 51);
        let goal = eater.select_goal(&world);
        assert_eq!(eater::EaterGoal::GetFood(0), goal);
    }
}

#[cfg(test)]
mod tests;