#![feature(get_mut_unchecked)]

mod inner_ep;
mod mstream;
mod outer_ep;

pub use mstream::*;
pub use outer_ep::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct EndPointId(pub usize);
