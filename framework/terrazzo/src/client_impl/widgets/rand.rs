//! Represents a flying cookie in the game

use std::num::NonZeroU32;
use std::sync::Mutex;

use terrazzo_client::prelude::OrElseLog as _;

pub struct Rand {
    next: Mutex<u32>,
}

pub fn rand(from: i32, to: i32) -> i32 {
    static RAND: Rand = Rand::new(if let Some(next) = NonZeroU32::new(13) {
        next
    } else {
        unreachable!()
    });
    RAND.next(from, to)
}

impl Rand {
    pub const fn new(next: NonZeroU32) -> Self {
        Self {
            next: Mutex::new(next.get()),
        }
    }

    pub fn next(&self, from: i32, to: i32) -> i32 {
        let mut rand = self.next.lock().or_throw("Next rand");
        let rand = &mut *rand;
        *rand ^= *rand << 13;
        *rand ^= *rand >> 17;
        *rand ^= *rand << 5;
        let d = (to - from) as u32;
        return (*rand % d) as i32 + from;
    }
}

#[cfg(test)]
#[test]
fn test_rand() {
    let rand = Rand::new(13.try_into().unwrap());
    assert_eq!(
        &[4, 6, 3, 2, 2, 2, 4, 1, 4, 1],
        (0..10)
            .map(|_| rand.next(1, 7))
            .collect::<Vec<_>>()
            .as_slice()
    );
    assert_eq!(
        &[831, 742, 13, 256, 846, 724, 654, 936, 320, 763],
        (0..10)
            .map(|_| rand.next(0, 1000))
            .collect::<Vec<_>>()
            .as_slice()
    );
}
