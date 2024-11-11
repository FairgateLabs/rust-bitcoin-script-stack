use bitcoin::{script::{Instruction, PushBytes}, Opcode};
pub use bitcoin_script::{define_pushable, script};
define_pushable!();
pub use bitcoin::ScriptBuf as Script;
use bitcoin::opcodes::all::*;

fn to_vec(script: &Script) -> Vec<Instruction<'_>> {
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

fn from_vec(instructions: Vec<Instruction<'_>>) -> Script {
    let mut new_script = Script::new();
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

fn is_opcode(instruction: &Instruction, opcode: &Opcode) -> bool {
    match instruction {
        Instruction::Op(op) => {
            op == opcode
        }, 
        _ => false
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

fn replace(instructions: &mut Vec<Instruction>, mut i: usize, count: usize) -> (usize, usize) {
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
        _ => return (0, 0)
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
    (new_size, drain)
}


pub fn optimize(script: Script) -> Script {
    //println!("{:?}", script.as_script().to_asm_string()) ;

    let mut instructions = to_vec(&script);
    let mut len = instructions.len();
    let mut i = 0;
    while i < len {

        //println!("i {:?}", instructions[i]);
        //println!("op {:?}", instructions[i]);
        let is_to_altstack = is_opcode(&instructions[i], &OP_TOALTSTACK);
        if is_to_altstack && i + 1 < len {
            if let Instruction::Op(next_op) = &instructions[i+1] {
                if next_op == &OP_FROMALTSTACK {
                    instructions.drain(i..i+2);
                    len -= 2;
                    i -= 1;
                }
            }
        }
        if i >= len {
            break;
        }

        let is_pick = is_opcode(&instructions[i], &OP_PICK);
        if is_pick {
            let digit = get_digit(&instructions[i-1]);
            if let Some(digit) = digit {
                if digit == 0 {
                    instructions[i-1] = Instruction::Op(OP_DUP);
                    instructions.drain(i..i+1);
                    len -= 1;
                    i -= 1;
                }
                if digit == 1 {
                    instructions[i-1] = Instruction::Op(OP_OVER);
                    instructions.drain(i..i+1);
                    len -= 1;
                    i -= 1;

                }
            }
        }
        if i >= len {
            break;
        }

        let is_roll = is_opcode(&instructions[i], &OP_ROLL);
        if is_roll {
            let digit = get_digit(&instructions[i-1]);
            if let Some(digit) = digit {
                if digit == 0 {
                    instructions.drain(i-1..i+1);
                    len -= 2;
                    i -= 1;
                }
                if digit == 1 {
                    instructions[i-1] = Instruction::Op(OP_SWAP);
                    instructions.drain(i..i+1);
                    len -= 1;
                    i -= 1;
                }
                if digit == 2 {
                    instructions[i-1] = Instruction::Op(OP_ROT);
                    instructions.drain(i..i+1);
                    len -= 1;
                    i -= 1;
                }
            }
        }
        if i >= len {
            break;
        }


        let instruction = &instructions[i];
        if get_digit(instruction).is_some() {
            //println!("{:?}", instruction);
            let count = count_ahead(&instructions, i);
            let (new_size, drain) = replace(&mut instructions, i, count);
            len -= drain;
            i += new_size;
        }
        i += 1;
    }


    from_vec(instructions)

    //println!("{:?}", Script::from_hex(&script.to_hex_string()).unwrap().as_script().to_asm_string()) ;
}


#[cfg(test)]
mod tests {


    use bitcoin::hashes::serde::de;
    pub use bitcoin_script::{define_pushable, script};
    
    define_pushable!();
    use crate::stack::{StackData, StackTracker, StackVariable};

    use crate::debugger::{debug_script, show_altstack, show_stack};
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


    fn duplicated_script(n: usize) -> Script {
        let mut stack = StackTracker::new();
        for _ in 0..n {
            stack.number(0);
        }
        stack.get_script()
    }

    #[test]
    fn test_dup() {

        let mut stack = StackTracker::new();
        stack.number(0);
        stack.op_dup();
        stack.op_2dup();
        assert_eq!(optimize(duplicated_script(4)), stack.get_script());
        
        let mut stack = StackTracker::new();
        stack.number(0);
        stack.op_dup();
        stack.op_dup();
        stack.op_2dup();
        assert_eq!(optimize(duplicated_script(5)), stack.get_script());

        let mut stack = StackTracker::new();
        stack.number(0);
        stack.op_dup();
        stack.op_2dup();
        stack.op_3dup();
        stack.op_3dup();
        stack.op_3dup();
        stack.op_3dup();
        assert_eq!(optimize(duplicated_script(16)), stack.get_script());
        
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

        assert_eq!(optimize(stack.get_script()), stack2.get_script());
 
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

        let optimized = optimize(stack.get_script());
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

        let optimized = optimize(stack.get_script());
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
        stack.debug();

        assert!(stack.run().success);

        let optimized = optimize(stack.get_script());
        println!("{:?}", stack.get_script().to_asm_string());
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
        stack.debug();

        assert!(stack.run().success);

        let optimized = optimize(stack.get_script());
        println!("{:?}", stack.get_script().to_asm_string());
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
        stack.debug();

        assert!(stack.run().success);

        let optimized = optimize(stack.get_script());
        println!("{:?}", stack.get_script().to_asm_string());
        println!("{:?}", optimized.to_asm_string());
        let ret= debug_script(optimized);
        assert!(ret.0.result().unwrap().success);

    }
    #[test]
    fn test_from_to() {
        let script =  sample_script();
        let new_script = from_vec(to_vec(&script));
        assert_eq!(script.as_script().to_asm_string(), new_script.as_script().to_asm_string());
    }

}