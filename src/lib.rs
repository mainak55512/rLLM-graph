use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Traits
trait Log {
    fn log(&self);
}

trait Node {
    async fn execute(&self, state: Arc<Mutex<State>>) -> Result<(), Box<dyn std::error::Error>>;
}

// Structs
#[derive(Debug, Default)]
pub struct State(HashMap<String, String>);

pub struct SharableState {
    state: Arc<Mutex<State>>,
}

pub struct FunctionNode {
    func: Box<dyn Fn(Arc<Mutex<State>>) -> Result<(), Box<dyn std::error::Error>>>,
}

pub struct LLMNode {}

// Behaviours
impl Log for State {
    fn log(&self) {
        println!("{:?}", &self);
    }
}

impl Log for SharableState {
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

impl SharableState {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(State::default())),
        }
    }
    fn state(&self) -> Arc<Mutex<State>> {
        Arc::clone(&self.state)
    }
}

//Node Structs
impl Node for FunctionNode {
    async fn execute(&self, state: Arc<Mutex<State>>) -> Result<(), Box<dyn std::error::Error>> {
        let _ = (self.func)(state)?;
        Ok(())
    }
}

impl FunctionNode {
    fn new(func: Box<dyn Fn(Arc<Mutex<State>>) -> Result<(), Box<dyn std::error::Error>>>) -> Self {
        Self { func: func }
    }
}

/*
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let shared_state = SharableState::new();
    let new_func = FunctionNode::new(Box::new(
        |state| -> Result<(), Box<dyn std::error::Error>> {
            match state.lock() {
                Ok(mut context_state) => {
                    context_state.0.insert("ABC".to_string(), "UVW".to_string());
                    context_state.log();
                }
                Err(_) => println!("Couldn't aquire lock!"),
            }

            Ok(())
        },
    ));
    new_func.execute(shared_state.state()).await?;
    shared_state.log();
    Ok(())
}
* */
