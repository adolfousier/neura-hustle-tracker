use active_win_pos_rs::get_active_window;

fn main() {
    match get_active_window() {
        Ok(window) => {
            println!("Active app: {}", window.app_name);
            println!("Window title: {}", window.title);
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}