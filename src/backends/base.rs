pub trait Backend {
    /// Delete all current state
    fn drain(&mut self);
}
