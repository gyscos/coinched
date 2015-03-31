pub mod cards;
pub mod bid;
pub mod game;

#[derive(PartialEq,Copy)]
pub struct PlayerPos(usize);
impl PlayerPos {
    pub fn next(self) -> PlayerPos {
        if self.0 == 3 {
            PlayerPos(self.0+1)
        } else {
            PlayerPos(0)
        }
    }
}

