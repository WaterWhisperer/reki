/// App state management.
#[expect(unused)]
pub struct App {
    /// Whether the application should quit.
    pub should_quit: bool,
}

impl App {
    #[expect(unused)]
    pub fn new() -> Self {
        Self { should_quit: false }
    }
}
