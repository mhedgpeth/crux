//! TODO mod docs

use crate::Command;
use serde::{Deserialize, Serialize};

// TODO revisit this
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response(pub String);

pub struct Time<Ef>
where
    Ef: Clone,
{
    effect: Ef,
}

impl<Ef> Time<Ef>
where
    Ef: Clone,
{
    pub fn new(effect: Ef) -> Self {
        Self { effect }
    }

    pub fn get<Ev, F>(&self, callback: F) -> Command<Ef, Ev>
    where
        Ev: 'static,
        F: Fn(Response) -> Ev + 'static,
    {
        Command::new(self.effect.clone(), callback)
    }
}
