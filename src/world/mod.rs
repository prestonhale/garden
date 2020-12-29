use serde::{Serialize, Deserialize};
use rand::prelude::*;
use std::collections::HashMap;

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

#[allow(dead_code)]
pub enum Direction {
    Up,
    Right,
    Down,
    Left
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
            Box::new(Eater::new()),
            Box::new(FoodSpawner {
                last_spawned: 0,
                spawn_every_x_ticks: 20,
            }),
            Box::new(Food {
                position: Position { x: 20, y: 20 }
            }),
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

    pub fn get_cells(&self) -> Vec<&Position> { 
        let mut positions = vec![];
        for entity in self.entities.iter() {
            positions.push(entity.get_position());
        }
        positions
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

    #[allow(dead_code)]
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
                        line.push_str("░░");
                        found_entity = true;
                    }
                }
                if !found_entity {
                    line.push_str("██");
                }
            }
            lines.push(line);
        }
        lines
    }

}

pub trait Updateable {
    fn update(&self, world: &World, rng: &mut rand_pcg::Pcg32) -> (EntityType, Option<EntityType>, Option<usize>);
    fn get_position(&self) -> &Position;
    fn get_tag(&self) -> &str { "untagged" }
}

trait Spawner {
    fn spawn(&self, world: &World) -> Option<EntityType>;
}

#[derive(Clone, Copy)]
struct FoodSpawner {
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
            let new_food = Food::new(Position{ x, y });
            (Box::new(new_spawner), Some(Box::new(new_food)), None)
        } else {
            new_spawner.last_spawned += 1;
            (Box::new(new_spawner), None, None)
        }
    }

    fn get_position(&self) -> &Position { 
        &Position{ x:0, y: 0}
    }
}

#[derive(Clone, Copy)]
struct Food {
    position: Position
}

impl Updateable for Food {
    fn update(&self, _world: &World, _rng: &mut rand_pcg::Pcg32) -> (EntityType, Option<EntityType>, Option<usize>) { 
        let new_food = self.clone();
        (Box::new(new_food), None, None)
    }
    
    fn get_position(&self) -> &Position { &self.position }
    
    fn get_tag(&self) -> &str { "food" }
}

impl Food {
    fn new(position: Position) -> Food {
        println!("Spawning food at {:?}", position);
        Food { position: position }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
enum Desire {
    Hunger
}


#[derive(Clone)]
struct Eater {
    position: Position,
    desires: HashMap<Desire, i8>,
    desire_threshold: HashMap<Desire, i8>,
}

#[derive(Debug, PartialEq)]
enum EaterGoal {
    GetFood(usize),
    Wander,
}

impl Updateable for Eater {
    fn update(&self, world: &World, _rng: &mut rand_pcg::Pcg32) -> (EntityType, Option<EntityType>, Option<usize>) {
        let mut new_eater = self.clone();
        let mut removed_entity_index = None;
        
        new_eater.increment_desire(Desire::Hunger, 1);

        let goal = self.select_goal(world);
        match goal {
            EaterGoal::Wander => {
            },
            EaterGoal::GetFood(i) => {
                let food_entity = &world.entities[i];
                let (cost, next_position) = self.pathfind(food_entity.get_position(), world);
                if cost == 1 {
                    removed_entity_index = Some(i);
                    new_eater.increment_desire(Desire::Hunger, -20);
                } else {
                    new_eater.position = next_position;
                }
            }
        }
        (Box::new(new_eater), None, removed_entity_index)
    }
    
    fn get_position(&self) -> &Position { &self.position }
}

impl Eater {
    fn new() -> Eater {
        let mut desires = HashMap::new();
        desires.insert(Desire::Hunger, 0);

        let mut desire_threshold = HashMap::new();
        desire_threshold.insert(Desire::Hunger, 0);

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

    fn increment_desire(&mut self, desire: Desire, increment: i8) {
        let mut new_desire = self.get_desire(desire) + increment;
        if new_desire < 0 { new_desire = 0 }
        self.set_desire(desire, new_desire);
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

#[cfg(test)]
mod tests;