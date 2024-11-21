use bitcoin::ScriptBuf;
use bitcoin::{script::Instruction, Opcode};
pub use bitcoin_script::script;
pub use bitcoin_script::builder::StructuredScript as Script;
use bitcoin::opcodes::all::*;

fn to_vec(script: &ScriptBuf) -> Vec<Instruction<'_>> {
    let mut instructions = Vec::new();
    for x in script.instructions_minimal() {
        match x {
            Ok(x) => {
                instructions.push(x);
            }
            Err(e) => {
                println!("{:?}", e);
            }
        }
    }
    instructions
}

fn from_vec(instructions: Vec<Instruction<'_>>) -> ScriptBuf {
    let mut new_script = Script::new("").compile();
    for x in instructions {
        new_script.push_instruction(x);
    }
    new_script
}

fn get_digit(instruction: &Instruction) -> Option<u8> {
    match instruction {
        Instruction::Op(op) => {
            let v = op.to_u8();
            match v {
                0x0 => return Some(0), //op_0
                0x51..=0x60 => return Some(v-0x50),
                _ => return None
            }
        }, 
        Instruction::PushBytes(x) => { // zero bytes is op_0
            if x.as_bytes().len() == 0 {
                return Some(0);
            }
        }
    }
    None
}

fn get_opcode(instruction: &Instruction) -> Option<Opcode> {
    match instruction {
        Instruction::Op(op) => Some(op.clone()),
        _ => None
    }
}

fn count_ahead(instructions: &Vec<Instruction>, i: usize) -> usize {
    let mut j = i + 1;
    let mut count = 0;
    while j < instructions.len() {
        let next_instruction = &instructions[j];
        if instructions[i] == *next_instruction {
            count += 1;
        } else {
            break;
        }
        j += 1;
    }
    count
}

fn replace(instructions: &mut Vec<Instruction>, mut i: usize, count: usize) -> usize {
    let ops = match count {
        3 => vec![1, 2],
        4 => vec![1, 1, 2],
        5 => vec![1, 2, 2],
        6 => vec![1, 2, 3],
        7 => vec![1, 2, 2, 2],
        8 => vec![1, 2, 2, 3], 
        9 => vec![1, 2, 3, 3],
        10 => vec![1, 2, 2, 2, 3],
        11 => vec![1, 2, 2, 3, 3],
        12 => vec![1, 2, 3, 3, 3],
        13 => vec![1, 2, 2, 2, 3, 3],
        14 => vec![1, 2, 2, 3, 3, 3],
        15 => vec![1, 2, 3, 3, 3, 3],
        _ => return 0
    };

    let new_size = ops.len();
    let drain = count - new_size;

    for op in ops {
        instructions[i+1] = match op {
            1 => Instruction::Op(OP_DUP),
            2 => Instruction::Op(OP_2DUP),
            3 => Instruction::Op(OP_3DUP),
            _ => panic!("unexpected op")
        };
        i += 1;
    }
    instructions.drain(i+1 .. i+1+drain);
    new_size
}

pub fn opcode_transformation( opcode: &Opcode, previous_opcode: Option<Opcode>, previous_digit: Option<u8>) -> Option<Option<Opcode>> {
    match (opcode, previous_opcode, previous_digit) {
        (&OP_FROMALTSTACK, Some(OP_TOALTSTACK), None) => Some(None),
        (&OP_PICK, None, Some(0)) => Some(Some(OP_DUP)),
        (&OP_PICK, None, Some(1)) => Some(Some(OP_OVER)),
        (&OP_ROLL, None, Some(0)) => Some(None),
        (&OP_ROLL, None, Some(1)) => Some(Some(OP_SWAP)),
        (&OP_ROLL, None, Some(2)) => Some(Some(OP_ROT)),
        _ => None
    }
} 

pub fn optimize(script: ScriptBuf) -> ScriptBuf {

    let mut instructions = to_vec(&script);
    let mut i = 0;
    while i < instructions.len() {

        if i > 0 {
            if let Some(opcode) = get_opcode(&instructions[i]) {
                if let Some(transformation)  = opcode_transformation(&opcode, get_opcode(&instructions[i-1]), get_digit(&instructions[i-1])) {
                    if let Some(new_opcode) = transformation {
                        instructions[i-1] = Instruction::Op(new_opcode);
                        instructions.drain(i..i+1);
                    } else {
                        instructions.drain(i-1..i+1);
                        i-=1;
                    }
                    continue;
                }
            }
        }

        let instruction = &instructions[i];
        if get_digit(instruction).is_some() {
            let count = count_ahead(&instructions, i);
            let new_size = replace(&mut instructions, i, count);
            i += new_size;
        }


        i += 1;
    }


    from_vec(instructions)

}


#[cfg(test)]
mod tests {


    use crate::stack::StackTracker;

    use crate::debugger::debug_script;
    use crate::script_util::*;

    use super::*;

    fn sample_script() -> Script {
        let mut stack = StackTracker::new();
        for _ in 0..10 {
            stack.number(0);
        }
        stack.to_altstack();
        stack.number_u32(0x123455);
        stack.hexstr("01020304050607080901020304");
        stack.get_script()
    }


    fn duplicated_script(n: usize) -> ScriptBuf {
        let mut stack = StackTracker::new();
        for _ in 0..n {
            stack.number(0);
        }
        stack.get_script().compile()
    }

    #[test]
    fn test_dup() {

        let mut stack = StackTracker::new();
        stack.number(0);
        stack.op_dup();
        stack.op_2dup();
        assert_eq!(optimize(duplicated_script(4)), stack.get_script().compile());
        
        let mut stack = StackTracker::new();
        stack.number(0);
        stack.op_dup();
        stack.op_dup();
        stack.op_2dup();
        assert_eq!(optimize(duplicated_script(5)), stack.get_script().compile());

        let mut stack = StackTracker::new();
        stack.number(0);
        stack.op_dup();
        stack.op_2dup();
        stack.op_3dup();
        stack.op_3dup();
        stack.op_3dup();
        stack.op_3dup();
        assert_eq!(optimize(duplicated_script(16)), stack.get_script().compile());
        
    }

    #[test]
    fn test_to_from_alt() {
        let mut stack = StackTracker::new();
        stack.number(0);
        stack.to_altstack();
        stack.from_altstack();
        stack.number(1);
        stack.number(1);
        stack.number(1);
        stack.number(1);
        stack.to_altstack();
        stack.from_altstack();

        let mut stack2 = StackTracker::new();
        stack2.number(0);
        stack2.number(1);
        stack2.op_dup();
        stack2.op_2dup();

        assert_eq!(optimize(stack.get_script().compile()), stack2.get_script().compile());
 
    }


    #[test]
    fn test_pick_0() {
        let mut stack = StackTracker::new();
        stack.number(1);
        stack.number(20);
        stack.number(0);
        stack.op_pick();
        stack.number(20);
        stack.op_equalverify();
        stack.op_drop();

        assert!(stack.run().success);

        let optimized = optimize(stack.get_script().compile());
        let ret= debug_script(optimized);
        assert!(ret.0.result().unwrap().success);

    }

    #[test]
    fn test_pick_1() {
        let mut stack = StackTracker::new();
        stack.number(1);
        stack.number(20);
        stack.number(1);
        stack.op_pick();
        stack.number(1);
        stack.op_equalverify();
        stack.op_drop();

        assert!(stack.run().success);

        let optimized = optimize(stack.get_script().compile());
        println!("{:?}", optimized.to_asm_string());
        let ret= debug_script(optimized);
        assert!(ret.0.result().unwrap().success);

    }

    #[test]
    fn test_roll_0() {
        let mut stack = StackTracker::new();
        stack.number(1);
        let x = stack.number(20);
        stack.move_var(x);
        stack.number(20);
        stack.op_equalverify();

        assert!(stack.run().success);

        let optimized = optimize(stack.get_script().compile());
        println!("{:?}", stack.get_script().compile().to_asm_string());
        println!("{:?}", optimized.to_asm_string());
        let ret= debug_script(optimized);
        assert!(ret.0.result().unwrap().success);

    }

    #[test]
    fn test_roll_1() {
        let mut stack = StackTracker::new();
        stack.number(1);
        let x = stack.number(20);
        stack.number(2);
        stack.move_var(x);
        stack.number(20);
        stack.op_equalverify();
        stack.op_drop();

        assert!(stack.run().success);

        let optimized = optimize(stack.get_script().compile());
        println!("{:?}", stack.get_script().compile().to_asm_string());
        println!("{:?}", optimized.to_asm_string());
        let ret= debug_script(optimized);
        assert!(ret.0.result().unwrap().success);

    }

    #[test]
    fn test_roll_2() {
        let mut stack = StackTracker::new();
        stack.number(1);
        let x = stack.number(20);
        stack.number(2);
        stack.number(2);
        stack.move_var(x);
        stack.number(20);
        stack.op_equalverify();
        stack.op_2drop();

        assert!(stack.run().success);

        let optimized = optimize(stack.get_script().compile());
        println!("{:?}", stack.get_script().compile().to_asm_string());
        println!("{:?}", optimized.to_asm_string());
        let ret= debug_script(optimized);
        assert!(ret.0.result().unwrap().success);

    }
    #[test]
    fn test_from_to() {
        let script =  sample_script().compile();
        let new_script = from_vec(to_vec(&script));
        assert_eq!(script.as_script().to_asm_string(), new_script.as_script().to_asm_string());
    }

}