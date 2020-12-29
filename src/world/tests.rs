use super::*;

#[test]
fn test_food_spawner() {
    let entities: Vec<EntityType> = vec![
        Box::new(FoodSpawner{
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

#[test]
fn test_eater_wander_goal() {
    let mut world = World::new(10, 10);
    let food = Box::new(Food {position: Position { x: 0, y: 0 }});
    world.add_entity(food);
    let eater = Eater::new();
    let goal = eater.select_goal(&world);
    assert_eq!(EaterGoal::Wander, goal);
}

#[test]
fn test_eater_food_goal() {
    let mut world = World::new(10, 10);
    let food = Box::new(Food {position: Position { x: 0, y: 0 }});
    world.add_entity(food);
    let mut eater = Eater::new();
    eater.set_desire(Desire::Hunger, 51);
    let goal = eater.select_goal(&world);
    assert_eq!(EaterGoal::GetFood(0), goal);
}