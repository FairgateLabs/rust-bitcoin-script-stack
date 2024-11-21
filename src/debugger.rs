use bitcoin::{hashes::Hash, TapLeafHash, Transaction};
use bitcoin_scriptexec::{Exec, ExecCtx, Options, Stack, TxTemplate};

pub use bitcoin_script::script;
pub use bitcoin_script::builder::StructuredScript as Script;

use crate::stack::{StackData, StackTracker, StackVariable};


pub struct StepResult {
    pub error: bool,
    pub error_msg: String,
    pub success: bool,
    pub last_opcode: String,
    pub stack: Vec<String>,
    pub altstack: Vec<String>,
}

impl StepResult {
    pub fn new(error:bool, error_msg:String, success:bool, last_opcode:String, stack:Vec<String>, altstack:Vec<String>) -> Self {
        StepResult { error, error_msg, success, last_opcode, stack, altstack }
    }
}
pub fn debug_script(script: bitcoin::ScriptBuf) -> (Exec, String) {
    let mut exec = Exec::new(
        ExecCtx::Tapscript,
        Options::default(),
        TxTemplate {
            tx: Transaction {
                version: bitcoin::transaction::Version::TWO,
                lock_time: bitcoin::locktime::absolute::LockTime::ZERO,
                input: vec![],
                output: vec![],
            },
            prevouts: vec![],
            input_idx: 0,
            taproot_annex_scriptleaf: Some((TapLeafHash::all_zeros(), None)),
        },
        script,
        vec![],
    )
    .expect("error creating exec");

    let mut last_opcode = String::new();
    loop {
        if !exec.remaining_script().is_empty() {
            let last_opcode_new = exec.remaining_script()[0..1].to_asm_string(); 
            if !last_opcode_new.is_empty() {
                last_opcode = last_opcode_new;
            }
        }
        if exec.exec_next().is_err() {
            break;
        }
    }
    (exec, last_opcode)

}


pub fn convert_stack(stack: &Stack) -> Vec<String> {
    let converted = (0..stack.len()).map(|f| stack.get(f))
        .map(|v| { if v.is_empty() { vec![0] } else { v.clone()} }).collect::<Vec<Vec<u8>>>();

    let hex_strings: Vec<String> = converted.into_iter().map(|sub_vec| {
        sub_vec.iter()
               .map(|byte| format!("{:x}", byte)) // Convert each byte to a hex string
               .collect::<Vec<String>>()            // Collect all hex strings into a vector
               .join("")                            // Join all elements of the vector into a single string
    }).collect();

    hex_strings

}


pub fn print_execute_step(stack: &StackTracker, step_number: usize) {
    let ex = execute_step(stack, step_number);
    if ex.error {
        println!("Error: {:?}", ex.error_msg);
    }
    if ex.success {
        println!("Success!");
    }
    println!("Last opcode: {:?}", ex.last_opcode);

    println!("======= STACK: ======");
    for s in ex.stack.iter() {
        println!("{}", s);
    }
    println!("==== ALT-STACK: ====");
    for s in ex.altstack.iter() {
        println!("{}", s);
    }
}

pub fn execute_step(stack: &StackTracker, step_number: usize) -> StepResult {

    let script = script! {
        for s in stack.script.iter().take(step_number+1) {
            { s.clone() }
        }
    };

    let height = stack.history[step_number];
    let step_data = stack.data.new_from_redo_height(height as usize);

    let (result, last) = debug_script(script.compile());

    let with_error = result.result().as_ref().unwrap().error.is_some();
    let error = format!("{:?}", result.result().as_ref().unwrap().error);
    let success = step_number == stack.script.len() - 1 && result.result().as_ref().unwrap().success;

    let converted = convert_stack(result.stack());
    let stack = show_stacks(&step_data, &step_data.stack, converted, false);

    let converted = convert_stack(result.altstack());
    let altstack = show_stacks(&step_data, &step_data.altstack, converted, true);

    StepResult::new(with_error, error, success, last, stack, altstack)

}

pub fn show_stacks(data: &StackData, stack: &[StackVariable], mut real: Vec<String>, reverse: bool) -> Vec<String> {
    let iter : Box<dyn Iterator<Item=&StackVariable>> = if reverse {
        Box::new(stack.iter().rev())
    } else {
        Box::new(stack.iter())
    };
    if reverse {
        real.reverse();
    }

    let mut ret = Vec::new();
    for var in iter {
        let data_item = format!("id: {:<width$} | size: {:<width$} | name: {:<width_name$} | ", var.id(), var.size(), data.names.get(&var.id()).unwrap_or(&"unknown".to_string()), width=7, width_name=20 );
        let mut real_sub = String::new();
        if !real.is_empty() && real.len() >= var.size() as usize {
            real_sub = real.iter().take(var.size() as usize).cloned().collect();
            real.drain(0..var.size() as usize);
        }
        ret.push(format!("{} {}", data_item, real_sub).to_string());
    }
    ret
} 

pub fn show_stack(data: &StackData, real: Vec<String> ) {
    println!("======= STACK: ======");
    for s in show_stacks(data, &data.stack, real, false) {
        println!("{}", s);
    }
}

pub fn show_altstack(data: &StackData, real: Vec<String> ) {
    println!("==== ALT-STACK: ====");
    for s in show_stacks(data, &data.altstack, real, true) {
        println!("{}", s);
    }
}
