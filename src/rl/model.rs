use bevy::prelude::Vec2;
use burn::module::Module;
use burn::module::Param;
use burn::optim::Optimizer;
use burn::tensor::Data;
use burn::tensor::Shape;
use burn::tensor::Tensor;
use burn::train::TrainOutput;
use burn::train::TrainStep;
use burn_ndarray::NdArrayBackend;
use rand::rngs::ThreadRng;
use rand::Rng;

use burn::nn;
use burn::tensor::backend::{ADBackend, Backend};

use crate::entities::Action;

use crate::entities::raycast::Detection;
use crate::entities::AgentState;

pub type ModelBackend = NdArrayBackend<f32>;

pub struct TrainModelInput<B: Backend, const D: usize> {
    pub outputs: Tensor<B, D>,
    pub targets: Tensor<B, D>,
}

#[derive(Module, Debug)]
pub struct Model<B: Backend<FloatElem = f32>> {
    pub lr: f32,
    hidden_layers: Param<Vec<nn::Linear<B>>>,
    output_layer: Param<nn::Linear<B>>,
    hidden_activation: nn::ReLU,
}
impl<B: Backend<FloatElem = f32>> Model<B> {
    pub fn new(inputs: usize, outputs: usize, hidden_layers: &[usize], lr: f32) -> Self {
        let layers = [&[inputs], hidden_layers].concat();
        let mut hidden_layers: Vec<nn::Linear<B>> = Vec::with_capacity(hidden_layers.len() + 1);
        for w in layers.windows(2) {
            let l = nn::Linear::new(&nn::LinearConfig::new(w[0], w[1]).with_bias(true));
            hidden_layers.push(l);
        }
        let l = nn::Linear::new(
            &nn::LinearConfig::new(*layers.last().unwrap(), outputs).with_bias(true),
        );
        let output_layer = l;

        Self {
            lr,
            hidden_layers: Param::from(hidden_layers),
            output_layer: Param::from(output_layer),
            hidden_activation: nn::ReLU::default(),
        }
    }

    pub fn forward(&self, mut xs: Tensor<B, 2>) -> Tensor<B, 2> {
        for layer in self.hidden_layers.iter() {
            xs = layer.forward(xs);
            xs = self.hidden_activation.forward(xs);
        }
        xs = self.output_layer.forward(xs);
        xs
    }

    pub fn loss(outputs: Tensor<B, 2>, targets: Tensor<B, 2>) -> Tensor<B, 2> {
        let batch_size = outputs.dims()[0];
        let loss = outputs.sub(targets).powf(2.0).mean_dim(1);
        debug_assert!(loss.dims()[0] == batch_size);
        loss
    }

    pub fn save_model(&self, path: &str) {
        self.state().save(path).unwrap();
    }
}
impl<B: ADBackend<FloatElem = f32>> Model<B> {
    pub fn train_step<O: Optimizer<Backend = B>>(
        &self,
        opt: &mut O,
        data: TrainModelInput<B, 2>,
    ) -> Self {
        let out = self.step(data);
        let model = self.clone();
        opt.update_module(model, out.grads)
    }
}
impl<B: ADBackend<FloatElem = f32>> TrainStep<TrainModelInput<B, 2>, Tensor<B, 2>> for Model<B> {
    fn step(&self, item: TrainModelInput<B, 2>) -> TrainOutput<Tensor<B, 2>> {
        let loss = Self::loss(item.outputs.clone(), item.targets);
        TrainOutput::new(self, loss.backward(), item.outputs)
    }
}

pub struct NormalizationData {
    pub min_speed: f32,
    pub max_speed: f32,
    pub min_loc: Vec2,
    pub max_loc: Vec2,
    pub min_energy: f32,
    pub max_energy: f32,
    pub min_dist: f32,
    pub max_dist: f32,
}

pub fn state_to_tensor<B: Backend>(state: &AgentState, norm: &NormalizationData) -> Tensor<B, 2> {
    let AgentState {
        location,
        direction,
        speed,
        energy,
        environment,
        sight,
        hearing,
    } = state;

    let size =
        2 + 2 + 1 + 1 + 5 + sight.len() * (1 + (5 + 3) + 1 + 5) + hearing.len() * (1 + (5 + 3) + 1);
    let mut data = vec![0.0; size];

    data[0] = (location.x - norm.min_loc.x) / (norm.max_loc.x - norm.min_loc.x);
    data[1] = (location.y - norm.min_loc.y) / (norm.max_loc.y - norm.min_loc.y);
    data[2] = direction.cos();
    data[3] = direction.sin();
    data[4] = (speed - norm.min_speed) / (norm.max_speed - norm.min_speed);
    data[5] = (energy - norm.min_energy) / (norm.max_energy - norm.min_energy);
    data[6 + environment.get_index()] = 1.0;
    for (i, det) in sight.iter().enumerate() {
        let offset = 11 + i * (1 + (5 + 3) + 1 + 1);
        data[offset] = (det.distance - norm.min_dist) / (norm.max_dist - norm.min_dist);
        data[offset + 1 + det.detection.get_index()] = 1.0;
        if let Detection::PreyAlive(en, dir) = det.detection {
            data[offset + 1 + 5] = (en - norm.min_energy) / (norm.max_energy - norm.min_energy);
            data[offset + 1 + 6] = dir.cos();
            data[offset + 1 + 7] = dir.sin();
        } else if let Detection::Predator(en, dir) = det.detection {
            data[offset + 1 + 5] = (en - norm.min_energy) / (norm.max_energy - norm.min_energy);
            data[offset + 1 + 6] = dir.cos();
            data[offset + 1 + 7] = dir.sin();
        } else if let Detection::PreyDead(en) = det.detection {
            data[offset + 1 + 5] = (en - norm.min_energy) / (norm.max_energy - norm.min_energy);
        }
        if det.food {
            data[offset + 1 + 8] = 1.0;
        }
        data[offset + 1 + 9 + det.env.get_index()] = 1.0;
    }
    for (i, det) in hearing.iter().enumerate() {
        let offset = 11 + i * (1 + (5 + 3) + 1);
        data[offset] = (det.distance - norm.min_dist) / (norm.max_dist - norm.min_dist);
        data[offset + 1 + det.detection.get_index()] = 1.0;
        if let Detection::PreyAlive(en, dir) = det.detection {
            data[offset + 1 + 5] = (en - norm.min_energy) / (norm.max_energy - norm.min_energy);
            data[offset + 1 + 6] = dir.cos();
            data[offset + 1 + 7] = dir.sin();
        } else if let Detection::Predator(en, dir) = det.detection {
            data[offset + 1 + 5] = (en - norm.min_energy) / (norm.max_energy - norm.min_energy);
            data[offset + 1 + 6] = dir.cos();
            data[offset + 1 + 7] = dir.sin();
        } else if let Detection::PreyDead(en) = det.detection {
            data[offset + 1 + 5] = (en - norm.min_energy) / (norm.max_energy - norm.min_energy);
        }
        if det.food {
            data[offset + 1 + 8] = 1.0;
        }
    }
    Tensor::from_floats(Data::new(data, Shape::from([1, size])))
}

pub fn tensor_to_action<B: Backend<FloatElem = f32>>(
    tensor: &Tensor<B, 2>,
    explore_prob: f32,
    learning: bool,
    rng: &mut ThreadRng,
) -> Action {
    let size = 2 + 1 + 2 + 1 + 2 + 1 + 1 + 1;
    let data: Vec<f32> = tensor.to_data().value;
    assert!(data.len() == size);
    if learning && rng.gen::<f32>() < explore_prob {
        let action_idx = rng.gen_range(0..size);
        Action::from_action_index(action_idx)
    } else {
        let mut max_idx = 0;
        let mut max_val = data[0];
        for (i, val) in data.iter().enumerate() {
            if *val > max_val {
                max_idx = i;
                max_val = *val;
            }
        }
        // TODO: remove
        if rng.gen::<f32>() < explore_prob {
            let action_idx = rng.gen_range(0..8);
            Action::from_action_index(action_idx)
        } else {
            Action::from_action_index(max_idx)
        }
    }
}
