use std::collections::{HashMap, VecDeque};

use crate::types::{Buffer, Transform};

pub struct Database<'a> {
    pub frames: HashMap<&'a str, Transform<'a>>,
    pub trees: VecDeque<Buffer>,
}
