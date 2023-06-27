use bevy::prelude::Resource;

pub mod model;
pub mod model_helpers;

#[derive(Clone, Debug)]
pub struct Transition {
    pub state: super::entities::AgentState,
    pub action: super::entities::Action,
    pub reward: f32,
    pub next_state: super::entities::AgentState,
}

#[derive(Default, Debug)]
pub struct ReplayBuffer {
    pub buffer: Vec<Option<Transition>>,
    pub capacity: usize,
    size: usize,
    i: usize,
}
impl ReplayBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![None; capacity],
            capacity,
            size: 0,
            i: 0,
        }
    }
    pub fn add(&mut self, t: Transition) {
        self.buffer[self.i] = Some(t);
        if self.size < self.capacity {
            self.size += 1;
        }
        self.i = (self.i + 1) % self.capacity;
    }
    pub fn get(&self) -> Vec<&Transition> {
        self.buffer
            .iter()
            .filter_map(|x| x.as_ref())
            .collect::<Vec<_>>()
    }
}

#[derive(Resource, Default, Debug)]
pub struct ReplayBufferPrey {
    pub buffer: ReplayBuffer,
}
impl ReplayBufferPrey {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: ReplayBuffer::new(capacity),
        }
    }
}

#[derive(Resource, Default, Debug)]
pub struct ReplayBufferPredator {
    pub buffer: ReplayBuffer,
}
impl ReplayBufferPredator {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: ReplayBuffer::new(capacity),
        }
    }
}

#[derive(Resource)]
pub struct ModelPrey {
    pub model: model_helpers::AgentModel,
}

#[derive(Resource)]
pub struct ModelPredator {
    pub model: model_helpers::AgentModel,
}
