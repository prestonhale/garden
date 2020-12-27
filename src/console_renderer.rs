#[cfg(feature = "console-renderer")]

// Console renderer
use std::io;
use console::Term;

pub fn render(world: &World) -> io::Result<()>{
    let mut lines: Vec<String> = Vec::new();
    for y in 0..*world.get_height() {
        let mut line = String::from("");
        for x in 0..*world.get_width(){
            if x == world.player.x && y == world.player.y {
                line.push_str("░░");
            } else {
                line.push_str("██");
            }
        }
        lines.push(line);
    }
    Ok(())
}