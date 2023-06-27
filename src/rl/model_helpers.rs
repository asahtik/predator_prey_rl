use bevy::prelude::Vec2;
use burn::{
    module::{Module, State},
    optim::{Adam, AdamConfig},
    tensor::{Data, Shape, Tensor},
};
use burn_autodiff::ADBackendDecorator;
use rand::rngs::ThreadRng;
use serde::{Deserialize, Serialize};

use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    entities::{Action, AgentState, AgentType},
    helpers::config_parser::{AgentConfig, RLConfig},
    rl::model::TrainModelInput,
};

use super::{
    model::{state_to_tensor, tensor_to_action, Model, ModelBackend, NormalizationData},
    Transition,
};
#[derive(Serialize, Deserialize)]
struct ModelDescription {
    lr: f32,
    eps: f32,
    counter: usize,
    layers: Vec<usize>,
    agent_type: String,
}
impl ModelDescription {
    pub fn from_agent_model(am: &AgentModel) -> Self {
        Self {
            lr: am.model.lr,
            eps: am.eps,
            counter: am.counter,
            layers: am.layers.clone(),
            agent_type: match am.agent_type {
                AgentType::Prey => "prey".to_string(),
                AgentType::Predator => "predator".to_string(),
            },
        }
    }
}

pub struct AgentModel {
    pub eps: f32,
    pub model: Model<burn_autodiff::ADBackendDecorator<ModelBackend>>,
    pub target: Model<burn_autodiff::ADBackendDecorator<ModelBackend>>,
    pub opt: Adam<burn_autodiff::ADBackendDecorator<ModelBackend>>,
    pub counter: usize,
    layers: Vec<usize>,
    agent_type: AgentType,
}
impl AgentModel {
    pub fn new(
        inputs: usize,
        outputs: usize,
        hidden_layers: &[usize],
        lr: f32,
        agent_type: AgentType,
    ) -> Self {
        let model = Model::new(inputs, outputs, hidden_layers, lr);
        Self {
            eps: 1.0,
            model: model.clone(),
            target: model,
            opt: Adam::new(&AdamConfig::new(lr as f64)),
            counter: 0,
            layers: [&[inputs], hidden_layers, &[outputs]].concat(),
            agent_type,
        }
    }
    pub fn backpropagate(
        &mut self,
        transitions: &[&Transition],
        norm: &NormalizationData,
        cfg: &RLConfig,
        rng: &mut ThreadRng,
    ) -> f32 {
        let mut loss_sum = 0f32;
        let num_batches = cfg.sample_count / cfg.batch_size;
        let to_choose = num_batches * cfg.batch_size;
        // println!("Length: {}, amount: {}", transitions.len(), to_choose);
        let selected_transition_indices =
            rand::seq::index::sample(rng, 10 * transitions.len() - 1, to_choose)
                .into_iter()
                .map(|el| el % transitions.len())
                .collect::<Vec<_>>();
        let mut selected_transitions = Vec::with_capacity(to_choose);
        for i in selected_transition_indices {
            selected_transitions.push(transitions[i].clone());
        }
        for i in 0..num_batches {
            let mut outputs: Vec<Tensor<ADBackendDecorator<ModelBackend>, 2>> =
                Vec::with_capacity(cfg.batch_size);
            let mut targets: Vec<Tensor<ADBackendDecorator<ModelBackend>, 2>> =
                Vec::with_capacity(cfg.batch_size);
            for j in 0..cfg.batch_size {
                let transition = &selected_transitions[i * cfg.batch_size + j];
                let output = self.model.forward(state_to_tensor(&transition.state, norm));
                let mut output_vec = output.to_data().value;
                outputs.push(output);
                let new_state_outputs = self
                    .target
                    .forward(state_to_tensor(&transition.next_state, norm))
                    .to_data()
                    .value;
                assert!(new_state_outputs.len() == *self.layers.last().unwrap());
                let ns_target = new_state_outputs
                    .iter()
                    .fold(f32::NEG_INFINITY, |a, b| a.max(*b));
                output_vec[transition.action.to_action_index()] =
                    transition.reward + cfg.discount * ns_target;
                let out_len = output_vec.len();
                let target = Tensor::from_floats(Data::new(output_vec, Shape::from([1, out_len])));
                targets.push(target);
            }
            let boutputs = Tensor::cat(outputs, 0);
            let btargets = Tensor::cat(targets, 0);
            loss_sum += Model::loss(boutputs.clone(), btargets.clone())
                .sum()
                .single_value();
            self.model = self.model.train_step(
                &mut self.opt,
                TrainModelInput {
                    outputs: boutputs,
                    targets: btargets,
                },
            );
        }
        self.eps -= cfg.eps_step;
        if self.eps < cfg.eps_min {
            self.eps = cfg.eps_min;
        }
        loss_sum / to_choose as f32
    }
    pub fn save(&self, path: &str) {
        let filename = {
            let start = SystemTime::now();
            let since_the_epoch = start
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
            since_the_epoch.as_millis().to_string()
        };
        let cfg = ModelDescription::from_agent_model(self);
        let toml_cfg_string = toml::to_string(&cfg).expect("Could not serialize model description");
        let at = cfg.agent_type;
        std::fs::write(format!("{path}/{at}_{filename}.toml"), toml_cfg_string)
            .expect("Could not write model description");
        self.model
            .save_model(&format!("{path}/{at}_{filename}.model"));
    }
    pub fn load(path: &str, model_name: &str, lr: f32) -> Self {
        let cfg_string = std::fs::read_to_string(format!("{path}/{model_name}.toml"))
            .expect("Could not read cfg file");
        let cfg: ModelDescription = toml::from_str(&cfg_string).expect("Could not parse cfg file");
        let mut model = Self::new(
            cfg.layers[0],
            cfg.layers[cfg.layers.len() - 1],
            &cfg.layers[1..cfg.layers.len() - 1],
            cfg.lr,
            match cfg.agent_type.as_str() {
                "prey" => AgentType::Prey,
                "predator" => AgentType::Predator,
                _ => panic!("Unknown agent type"),
            },
        );
        model.eps = cfg.eps;
        model.counter = cfg.counter;
        model.model = model
            .model
            .load(&State::load(format!("{path}/{model_name}.model").as_str()).unwrap())
            .unwrap();
        if model.model.lr != lr {
            model.model.lr = lr;
        }
        model
    }
    pub fn reset_target(&mut self) {
        self.target.state().clone_from(&self.model.state());
    }
    pub fn get_action(
        &self,
        state: &AgentState,
        config: &AgentConfig,
        world_limits: (Vec2, Vec2),
        learning: bool,
    ) -> Action {
        let output = self.model.forward(state_to_tensor(
            state,
            &NormalizationData {
                min_speed: 0.0,
                max_speed: config.run_speed,
                min_loc: world_limits.0,
                max_loc: world_limits.1,
                min_energy: 0.0,
                max_energy: 100.0,
                min_dist: 0.0,
                max_dist: config.vision_range,
            },
        ));
        tensor_to_action(&output, self.eps, learning, &mut rand::thread_rng())
    }
}
