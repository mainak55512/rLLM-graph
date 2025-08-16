use async_trait::async_trait;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Type Alias
pub type SharedState = Arc<Mutex<State>>;
pub type RLLMError = Box<dyn std::error::Error + Send + Sync>;

// Traits
pub trait Log {
    fn log(&self);
}

#[async_trait]
pub trait Node {
    async fn execute(&self, state: SharedState) -> Result<(), RLLMError>;
}

// Structs
#[derive(Debug, Default)]
pub struct State(HashMap<String, String>);

pub struct StateBuilder {
    state: SharedState,
}

pub struct FunctionNode {
    func: Box<dyn Fn(SharedState) -> Result<(), RLLMError> + Send + Sync>,
}

pub struct LLMNode {
    prompt: String,
    endpoint: String,
    model: String,
    api_key: String,
}

pub struct Graph {
    nodes: HashMap<String, Box<dyn Node>>,
    edges: Vec<(String, String)>,
}

// Behaviours
impl Log for State {
    fn log(&self) {
        println!("{:?}", &self);
    }
}

impl State {
    pub fn insert(&mut self, key: String, value: String) {
        self.0.insert(key, value);
    }
}

impl Log for StateBuilder {
    fn log(&self) {
        match &self.state.lock() {
            Ok(current) => {
                println!("{:?}", current)
            }
            Err(e) => {
                println!("{:?}", e);
            }
        }
    }
}

impl StateBuilder {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(State::default())),
        }
    }
    pub fn state(&self) -> SharedState {
        Arc::clone(&self.state)
    }
}

//Node Structs
#[async_trait]
impl Node for FunctionNode {
    async fn execute(&self, state: SharedState) -> Result<(), RLLMError> {
        let _ = (self.func)(state)?;
        Ok(())
    }
}

impl FunctionNode {
    pub fn new(func: Box<dyn Fn(SharedState) -> Result<(), RLLMError> + Send + Sync>) -> Self {
        Self { func: func }
    }
}

#[async_trait]
impl Node for LLMNode {
    async fn execute(&self, state: SharedState) -> Result<(), RLLMError> {
        Ok(())
    }
}
impl LLMNode {
    pub fn new(endpoint: String, api_key: String) -> Self {
        Self {
            prompt: String::default(),
            model: String::default(),
            api_key: api_key,
            endpoint: endpoint,
        }
    }

    pub fn set_prompt(&mut self, prompt: String) {
        self.prompt = prompt;
    }

    pub fn set_model(&mut self, model: String) {
        self.model = model;
    }
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node_name: String, node: Box<dyn Node>) {
        self.nodes.insert(node_name, node);
    }

    pub fn add_edge(&mut self, edge: (String, String)) {
        self.edges.push(edge);
    }

    pub fn run(&self) {
        for edge in &self.edges {}
    }
}

/*
use rllm::{FunctionNode, Log, Node, RLLMError, SharedState, StateBuilder};

#[tokio::main]
async fn main() -> Result<(), RLLMError> {
    let shared_state = StateBuilder::new();
    let new_func = FunctionNode::new(Box::new(|state: SharedState| -> Result<(), RLLMError> {
        match state.lock() {
            Ok(mut context_state) => {
                context_state.insert("ABC".to_string(), "UVW".to_string());
                context_state.log();
            }
            Err(_) => println!("Couldn't aquire lock!"),
        }

        Ok(())
    }));
    new_func.execute(shared_state.state()).await?;
    let new_func_1 = FunctionNode::new(Box::new(|state: SharedState| -> Result<(), RLLMError> {
        match state.lock() {
            Ok(mut context_state) => {
                context_state.insert("DEF".to_string(), "XYZ".to_string());
                context_state.log();
            }
            Err(_) => println!("Couldn't acquire lock!"),
        }

        Ok(())
    }));
    new_func_1.execute(shared_state.state()).await?;
    shared_state.log();
    Ok(())
}
* */
