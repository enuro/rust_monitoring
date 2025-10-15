pub enum CurrentScreen {
    Main,
    CPUScreen,
    GPUScreen,
    MemoryScreen,
    NetworkScreen,
    TaskListScreen,
}

pub struct App {
    pub current_screen: CurrentScreen,
}

impl App {
    pub fn new() -> App {
        App {
            current_screen: CurrentScreen::Main,
        }
    }
}