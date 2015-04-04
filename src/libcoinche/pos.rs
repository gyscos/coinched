#[derive(PartialEq,Clone,Copy)]
pub struct PlayerPos(pub usize);

pub struct PlayerIterator {
    current: PlayerPos,
    remaining: usize,
}

impl Iterator for PlayerIterator {
    type Item = PlayerPos;

    fn next(&mut self) -> Option<PlayerPos> {
        if self.remaining == 0 {
            return None;
        }

        let r = self.current;
        self.current = self.current.next();
        self.remaining -= 1;
        Some(r)
    }
}

impl PlayerPos {
    pub fn next(self) -> PlayerPos {
        if self.0 == 3 {
            PlayerPos(0)
        } else {
            PlayerPos(self.0+1)
        }
    }

    pub fn next_n(self, n: usize) -> PlayerPos {
        if n == 0 {
            self
        } else {
            self.next().next_n(n-1)
        }
    }

    pub fn prev(self) -> PlayerPos {
        if self.0 == 0 {
            PlayerPos(3)
        } else {
            PlayerPos(self.0 - 1)
        }
    }

    pub fn until_n(self, n: usize) -> PlayerIterator {
        PlayerIterator {
            current:self,
            remaining: n,
        }
    }

    pub fn distance_until(self, other: PlayerPos) -> usize {
        (3 + other.0 - self.0) % 4 + 1
    }

    // Iterate on every player between self included and other excluded.
    pub fn until(self, other: PlayerPos) -> PlayerIterator {
        let d = self.distance_until(other);
        self.until_n(d)
    }
}

#[test]
fn test_pos() {
    let mut count = [0; 4];
    for i in 0..4 {
        for pos in PlayerPos(i).until(PlayerPos(0)) {
            count[pos.0] += 1;
        }
        for pos in PlayerPos(0).until(PlayerPos(i)) {
            count[pos.0] += 1;
        }
    }

    for c in count.iter() {
        assert!(*c == 5);
    }

    for i in 0..4 {
        assert!(PlayerPos(i).next() == PlayerPos((i+1)%4));
        assert!(PlayerPos(i) == PlayerPos((i+1)%4).prev());
        assert!(PlayerPos(i).next().prev() == PlayerPos(i));
    }
}
