use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use reqwest::{
    Client,
    header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue},
};
use serde_json::{Value, json};

// Type Alias
pub type SharedState = Arc<Mutex<State>>;
pub type RLLMError = Box<dyn std::error::Error + Send + Sync>;

// Traits
pub trait Log {
    fn log_llm_response(&self);
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

pub struct Graph<'a> {
    nodes: &'a HashMap<String, Box<dyn Node>>,
    start_edges: Vec<String>,
    adjacent_edge_map: HashMap<String, Vec<String>>,
}

pub struct GraphBuilder {
    nodes: HashMap<String, Box<dyn Node>>,
    edges: Vec<(String, String)>,
}

// Behaviours
impl Log for State {
    fn log_llm_response(&self) {
        if let Some(llm_response) = self.0.get("llm_entry") {
            println!("{}", llm_response);
        }
    }
}

impl State {
    pub fn set_user_entry(&mut self, value: String) {
        self.0.insert("user_entry".to_string(), value);
    }
    pub fn set_llm_entry(&mut self, value: String) {
        self.0.insert("llm_entry".to_string(), value);
    }
}

impl Log for StateBuilder {
    fn log_llm_response(&self) {
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
        let client = Client::new();
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&self.api_key)?);
        let request_body = json!({
          "model": &self.model,
          "messages": [{
              "role": "user",
              "content": &self.prompt
          }]
        });
        let res = client
            .post(&self.endpoint)
            .headers(headers)
            .body(request_body.to_string())
            .send()
            .await?
            .text()
            .await?;

        let body: Value = serde_json::from_str(&res)?;
        let msg = &body["choices"][0]["message"];
        match state.lock() {
            Ok(mut context_state) => {
                context_state.set_llm_entry(msg["content"].to_string());
                // context_state.log();
            }
            Err(_) => println!("Couldn't aquire lock!"),
        }
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

impl Graph<'_> {
    pub async fn run(&self) -> Result<(), RLLMError> {
        let mut visited_nodes: HashMap<String, bool> = HashMap::new();
        let shared_state = StateBuilder::new();
        for edge in &self.start_edges {
            if let Some(node) = self.nodes.get(edge) {
                if let Some(_) = visited_nodes.get(edge) {
                } else {
                    visited_nodes.insert(edge.clone(), true);
                    node.execute(shared_state.state()).await?;
                }
            }

            if let Some(end_edges) = self.adjacent_edge_map.get(edge) {
                for end_edge in end_edges {
                    if let Some(node) = self.nodes.get(end_edge) {
                        if let Some(_) = visited_nodes.get(end_edge) {
                        } else {
                            visited_nodes.insert(end_edge.clone(), true);
                            node.execute(shared_state.state()).await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl GraphBuilder {
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

    fn build_adjacent_edge(&self) -> (Vec<String>, HashMap<String, Vec<String>>) {
        let mut adjacent_edge_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut start_edges: Vec<String> = Vec::new();
        for edge in &self.edges {
            start_edges.push(edge.0.clone());
            adjacent_edge_map
                .entry(edge.0.clone())
                .or_insert_with(Vec::new)
                .push(edge.1.clone());
        }
        (start_edges, adjacent_edge_map)
    }

    pub fn build(&self) -> Graph {
        let (start_edges, adjacent_edge_map) = self.build_adjacent_edge();
        Graph {
            nodes: &self.nodes,
            start_edges: start_edges,
            adjacent_edge_map: adjacent_edge_map,
        }
    }
}

/*
use dotenv::dotenv;
use rllm::{FunctionNode, GraphBuilder, LLMNode, Log, RLLMError, SharedState};
use std::env;

#[tokio::main]
async fn main() -> Result<(), RLLMError> {
    dotenv().ok();
    let endpoint = env::var("END_POINT")?;
    let api_key = "Bearer ".to_string() + &env::var("API_KEY")?;

    // Creating LLMNode
    let mut llm_node = LLMNode::new(endpoint, api_key);
    llm_node.set_model("llama-3.3-70b-versatile".to_string());
    llm_node.set_prompt("What's the capital of Hungery?".to_string());

    // Creating FunctionNode
    let log_fn = FunctionNode::new(Box::new(|state: SharedState| -> Result<(), RLLMError> {
        match state.lock() {
            Ok(context_state) => {
                context_state.log_llm_response();
            }
            Err(_) => println!("Couldn't aquire lock!"),
        }

        Ok(())
    }));

    // Building Graph
    let mut g_build = GraphBuilder::new();
    g_build.add_node("A".to_string(), Box::new(llm_node));
    g_build.add_node("B".to_string(), Box::new(log_fn));
    g_build.add_edge(("A".to_string(), "B".to_string()));
    let graph = g_build.build();

    // Running the graph
    graph.run().await?;

    Ok(())
}
* */
