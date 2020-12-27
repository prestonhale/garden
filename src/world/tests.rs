use super::*;

#[test]
fn test_food_spawner() {
    let mut world = World{
        width: 10,
        height: 10,
        entities: vec![
            Box::new(FoodSpawner{
                last_spawned: 9,
                spawn_every_x_ticks: 10
            })
        ]
    };
    let mut randomizer = rand_pcg::Pcg32::from_seed(*b"somebody once to");
    world.update(&mut randomizer);
    assert_eq!(world.entities.len(), 2);
}
