use async_trait::async_trait;
use std::collections::HashMap;
use std::i64;
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
#[async_trait]
pub trait Node {
    async fn execute(&self, state: SharedState) -> Result<(), RLLMError>;
}

// Structs
#[derive(Debug, Default)]
pub struct State(HashMap<String, Value>);

pub struct StateBuilder {
    state: SharedState,
}

pub struct FunctionNode {
    func: Box<dyn Fn(SharedState) -> Result<(), RLLMError> + Send + Sync>,
}

pub struct LLMNode {
    prompt: String,
    prompt_var_list: Vec<String>,
    endpoint: String,
    model: String,
    api_key: String,
    tools: HashMap<String, Tool>,
    tool_list: Vec<Value>,
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

pub struct Tool {
    name: String,
    tool_fn: FunctionNode,
    tool_details: Value,
}

pub struct ToolRegistry {
    tool_map: HashMap<String, Tool>,
    tools: Vec<Value>,
}

// Behaviours
impl State {
    fn check_valid_key(var_name: &str) -> bool {
        if var_name != "rllm_response" {
            true
        } else {
            false
        }
    }
    fn log(&self, var_name: &str) {
        if let Some(data) = self.0.get(var_name) {
            match data {
                Value::String(s) => println!("{}", s), // no quotes
                Value::Number(n) => println!("{}", n),
                Value::Bool(b) => println!("{}", b),
                Value::Null => println!("null"),
                _ => println!("{}", data), // arrays/objects fallback to JSON
            }
        }
    }

    pub fn get_rllm_number(&self, var_name: &str) -> Result<i64, String> {
        if let Some(data) = self.0.get(var_name) {
            if let Value::Number(n) = data {
                let num = match n.as_i64() {
                    Some(num) => Ok(num),
                    None => Err("Not a Number".to_string()),
                };
                num
            } else {
                Err("Not a Number".to_string())
            }
        } else {
            Err("No Entry Found".to_string())
        }
    }

    pub fn get_rllm_string(&self, var_name: &str) -> Result<String, String> {
        if let Some(data) = self.0.get(var_name) {
            if let Value::String(s) = data {
                Ok(s.to_string())
            } else {
                Err("Not a String".to_string())
            }
        } else {
            Err("No Entry Found".to_string())
        }
    }

    pub fn get_rllm_bool(&self, var_name: &str) -> Result<bool, String> {
        if let Some(data) = self.0.get(var_name) {
            if let Value::Bool(b) = data {
                Ok(*b)
            } else {
                Err("Not a Boolean".to_string())
            }
        } else {
            Err("No Entry Found".to_string())
        }
    }

    pub fn get_rllm_json(&self, var_name: &str) -> Result<Value, String> {
        if let Some(data) = self.0.get(var_name) {
            Ok(data.clone())
        } else {
            Err("No Entry Found".to_string())
        }
    }

    pub fn get_llm_response(&self) -> Result<String, String> {
        self.get_rllm_string("rllm_response")
    }

    pub fn get_llm_response_json(&self) -> Result<Value, String> {
        self.get_rllm_json("rllm_response")
    }

    pub fn set_rllm_number(&mut self, var_name: &str, value: i64) -> Result<(), String> {
        if Self::check_valid_key(var_name) {
            self.0
                .insert(var_name.to_string(), Value::Number(value.into()));
            Ok(())
        } else {
            Err("Restricted key".to_string())
        }
    }
    pub fn set_rllm_string(&mut self, var_name: &str, value: String) -> Result<(), String> {
        if Self::check_valid_key(var_name) {
            self.0.insert(var_name.to_string(), Value::String(value));
            Ok(())
        } else {
            Err("Restricted key".to_string())
        }
    }
    pub fn set_rllm_bool(&mut self, var_name: &str, value: bool) -> Result<(), String> {
        if Self::check_valid_key(var_name) {
            self.0.insert(var_name.to_string(), Value::Bool(value));
            Ok(())
        } else {
            Err("Restricted key".to_string())
        }
    }
    pub fn set_rllm_json(&mut self, var_name: &str, value: Value) -> Result<(), String> {
        if Self::check_valid_key(var_name) {
            self.0.insert(var_name.to_string(), value);
            Ok(())
        } else {
            Err("Restricted key".to_string())
        }
    }
    fn set_llm_response(&mut self, value: String) {
        self.0
            .insert("rllm_response".to_string(), Value::String(value));
    }
    fn set_llm_response_json(&mut self, value: Value) {
        self.0.insert("rllm_response".to_string(), value);
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

        let mut prompt = self.prompt.clone();

        match state.lock() {
            Ok(context_state) => {
                for elem in self.prompt_var_list.iter() {
                    let data = context_state.get_rllm_string(elem)?;
                    prompt = prompt.replacen("{}", data.as_str(), 1);
                }
            }
            Err(_) => println!("Couldn't aquire lock!"),
        }

        let request_body = json!({
          "model": &self.model,
          "messages": [{
              "role": "user",
              "content": prompt.as_str()
          }],
            "tools": self.tool_list
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
        if let Some(tools_call) = msg.get("tool_calls") {
            if let Some(tool_array) = tools_call.as_array() {
                for tool in tool_array {
                    match state.lock() {
                        Ok(mut context_state) => {
                            context_state.set_llm_response_json(tool.clone());
                        }
                        Err(_) => println!("Couldn't aquire lock!"),
                    }
                    if let Some(tool_name) = tool.pointer("/function/name").and_then(|v| v.as_str())
                    {
                        if let Some(tool_func) = self.tools.get(tool_name) {
                            tool_func.tool_fn.execute(Arc::clone(&state)).await?;
                        }
                    }
                }
            }
        } else {
            match state.lock() {
                Ok(mut context_state) => {
                    context_state.set_llm_response(msg["content"].to_string());
                }
                Err(_) => println!("Couldn't aquire lock!"),
            }
        }
        Ok(())
    }
}
impl LLMNode {
    pub fn new(endpoint: String, api_key: String) -> Self {
        Self {
            prompt: String::default(),
            prompt_var_list: Vec::default(),
            model: String::default(),
            api_key: api_key,
            endpoint: endpoint,
            tools: HashMap::new(),
            tool_list: Vec::new(),
        }
    }

    pub fn set_prompt(&mut self, prompt: String, var_list: Vec<String>) {
        self.prompt = prompt;
        self.prompt_var_list = var_list;
    }

    pub fn set_model(&mut self, model: String) {
        self.model = model;
    }

    pub fn set_tools(&mut self, tool_list: Vec<Value>, tools: HashMap<String, Tool>) {
        self.tools = tools;
        self.tool_list = tool_list;
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

impl Tool {
    pub fn new(tool_name: String, tool_fn: FunctionNode) -> Self {
        Self {
            name: tool_name,
            tool_fn: tool_fn,
            tool_details: Value::Null,
        }
    }

    pub fn add_tool_details(mut self, tool_details: Value) -> Self {
        self.tool_details = tool_details;
        self
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tool_map: HashMap::new(),
            tools: Vec::new(),
        }
    }

    pub fn register(&mut self, tool: Tool) {
        self.tools.push(tool.tool_details.clone());
        self.tool_map.insert(tool.name.clone(), tool);
    }

    pub fn get_tools(self) -> HashMap<String, Tool> {
        self.tool_map
    }

    pub fn get_tool_list(&self) -> Vec<Value> {
        self.tools.clone()
    }
}
