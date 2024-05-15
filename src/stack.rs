use std::collections::HashMap;

use bitcoin::{opcodes::OP_TRUE, Opcode};
use bitcoin::opcodes::all::*;

pub use bitcoin_script::{define_pushable, script};
define_pushable!();
pub use bitcoin::ScriptBuf as Script;

use crate::debugger::{execute_step, print_execute_step, show_altstack, show_stack, StepResult};
use super::script_util::*;



#[derive(Clone, Debug, Copy)]
pub struct StackVariable {
    id: u32,
    size: u32,
}

impl StackVariable {
    pub fn new(id: u32, size: u32) -> Self {
        StackVariable { id, size }
    }
    pub fn null() -> Self {
        StackVariable { id: 0, size: 0 }
    }
    pub fn is_null(&self) -> bool {
        self.id == 0
    }
    pub fn id(&self) -> u32 {
        self.id 
    }
    pub fn size(&self) -> u32 {
        self.size
    }
}

enum RedoOps {
    PushStack(StackVariable),
    PushAltstack(StackVariable),
    PopStack,
    PopAltstack,
    SetName(StackVariable, String),
    RemoveName(StackVariable),
    RemoveVar(StackVariable),
    DecreaseSize(StackVariable),
    IncreaseSize(usize, u32),
}

pub struct StackData {
    pub(crate) stack: Vec<StackVariable>,
    pub(crate) altstack: Vec<StackVariable>,
    pub(crate) names: HashMap<u32, String>,
    redo_log: Vec<RedoOps>,
    with_redo_log: bool,
}

impl StackData {
    pub fn new(with_redo_log: bool) -> Self {
        StackData {
            stack: Vec::new(),
            altstack: Vec::new(),
            names: HashMap::new(),
            redo_log: Vec::new(),
            with_redo_log
        }
    }

    pub fn push_stack(&mut self, var: StackVariable) {
        self.stack.push(var);
        if self.with_redo_log {
            self.redo_log.push(RedoOps::PushStack(var));
        }
    }

    pub fn push_altstack(&mut self, var: StackVariable) {
        self.altstack.push(var);
        if self.with_redo_log {
            self.redo_log.push(RedoOps::PushAltstack(var));
        }
    }

    pub fn pop_stack(&mut self) -> StackVariable {
        if self.with_redo_log {
            self.redo_log.push(RedoOps::PopStack);
        }
        self.stack.pop().unwrap()
    }

    pub fn pop_altstack(&mut self) -> StackVariable {
        if self.with_redo_log {
            self.redo_log.push(RedoOps::PopAltstack);
        }
        self.altstack.pop().unwrap()
    }

    pub fn set_name(&mut self, var: StackVariable, name: &str) {
        self.names.insert(var.id, name.to_string());
        if self.with_redo_log {
            self.redo_log.push(RedoOps::SetName(var, name.to_string()));
        }
    }
    
    pub fn remove_name(&mut self, var: StackVariable) {
        self.names.remove(&var.id);
        if self.with_redo_log {
            self.redo_log.push(RedoOps::RemoveName(var));
        }
    }

    pub fn remove_var(&mut self, var: StackVariable) {
        self.stack.retain(|x| x.id != var.id);
        if self.with_redo_log {
            self.redo_log.push(RedoOps::RemoveVar(var));
        }
    }

    pub fn increase_size(&mut self, idx: usize, next_size: u32) {
        self.stack[idx].size += next_size;
        if self.with_redo_log {
            self.redo_log.push(RedoOps::IncreaseSize(idx, next_size));
        }
    }

    pub fn decrease_size(&mut self, var: StackVariable) {
        for v in self.stack.iter_mut() {
            if v.id == var.id {
                v.size -= 1;
            }
        }
        if self.with_redo_log {
            self.redo_log.push(RedoOps::DecreaseSize(var));
        }
    }

    pub fn new_from_redo_height(&self, height: usize) -> Self {
        let mut new_stack = StackData::new(false);
        for i in 0..height {
            match &self.redo_log[i] {
                RedoOps::PushStack(var) => new_stack.push_stack(*var),
                RedoOps::PushAltstack(var) => new_stack.push_altstack(*var),
                RedoOps::PopStack => { let _ = new_stack.pop_stack(); },
                RedoOps::PopAltstack => { let _ = new_stack.pop_altstack(); },
                RedoOps::SetName(var, name) => new_stack.set_name(*var, name),
                RedoOps::RemoveName(var) => new_stack.remove_name(*var),
                RedoOps::RemoveVar(var) => new_stack.remove_var(*var),
                RedoOps::DecreaseSize(var) => new_stack.decrease_size(*var),
                RedoOps::IncreaseSize(idx, next_size) => new_stack.increase_size(*idx, *next_size),
            }
        }
        new_stack
    }


}

pub struct StackTracker {
    pub(crate) data: StackData,
    pub(crate) script: Vec<Script>,
    pub(crate) history: Vec<u32>,
    counter: u32,
    max_stack_size: u32,
    with_history: bool,
    pub(crate) breakpoint: Vec<(u32, String)>,
}

impl Default for StackTracker {
    fn default() -> Self {
         Self::new()
    }
}

impl StackTracker {

    pub fn new() -> Self {
        StackTracker {
            data: StackData::new(true),
            script: Vec::new(),
            history: Vec::new(),
            counter: 0,
            max_stack_size: 0,
            with_history: true,
            breakpoint: Vec::new(),
        }
    }

    fn push(&mut self, var: StackVariable) {
        self.data.push_stack(var);
        let totalsize = self.data.stack.iter().fold(0, |acc, f| acc + f.size);
        self.max_stack_size = self.max_stack_size.max(totalsize);
    }

    fn push_script(&mut self, script: Script) {
        self.script.push(script);
        if self.with_history {
            self.history.push(self.data.redo_log.len() as u32);
        }
    }

    pub fn set_breakpoint(&mut self, name: &str) {
        self.breakpoint.push((self.script.len()as u32, name.to_string()));
    }

    pub fn get_next_breakpoint(&self, from:u32) -> Option<(u32, String)> {
        for (pos, name) in self.breakpoint.iter() {
            if *pos > from {
                let pos = *pos;
                let pos = pos.min(self.script.len() as u32 - 1);
                return Some((pos, name.clone()));
            }
        }
        None
    }

    pub fn get_prev_breakpoint(&self, from:u32) -> Option<(u32, String)> {
        let mut ret = None;
        for (pos, name) in self.breakpoint.iter() {
            if *pos < from {
                ret = Some((*pos, name.clone()));
            }
            if *pos > from {
                break;
            }
        }
        ret
    }


    pub fn get_max_stack_size(&self) -> u32 {
        self.max_stack_size
    }

    pub fn next_counter(&mut self) -> u32 {
        self.counter += 1;
        self.counter
    }

    pub fn define(&mut self, size: u32, name: &str) -> StackVariable {
        let var = StackVariable::new(self.next_counter(), size);
        self.push(var);
        self.data.set_name(var, name);
        var
    }

    pub fn var(&mut self, size: u32, script: Script, name: &str) -> StackVariable {
        let var = StackVariable::new( self.next_counter(), size );
        self.push(var);
        self.data.set_name(var, name);
        self.push_script(script);
        var
    }

    pub fn rename(&mut self, var: StackVariable, name: &str) {
        self.data.set_name(var, name);
    }
    
    pub fn drop(&mut self, var: StackVariable) {
        assert!(self.data.stack.last().unwrap().id == var.id);
        self.data.pop_stack();
        self.data.remove_name(var);
        self.push_script(drop_count(var.size));
    }

    pub fn to_altstack(&mut self) -> StackVariable {
        let var = self.data.pop_stack();
        self.data.push_altstack(var);
        self.push_script( toaltstack(var.size) );
        var
    }

    pub fn to_altstack_count(&mut self, count: u32) -> Vec<StackVariable> {
        let mut ret = Vec::new();
        for _ in 0..count {
            let var = self.to_altstack();
            ret.push(var);
        }
        ret
   }


    pub fn from_altstack(&mut self) -> StackVariable {
        let var = self.data.pop_altstack();
        self.push(var);
        self.push_script( fromaltstack(var.size) );
        var
    }

    pub fn from_altstack_count(&mut self, count: u32) -> Vec<StackVariable> {
        let mut ret = Vec::new();
        for _ in 0..count {
            let var = self.from_altstack();
            ret.push(var);
        }
        ret
    }

    pub fn from_altstack_joined(&mut self, count: u32, name: &str) -> StackVariable {
        assert!(count > 1, "from_altstack_joined requires count > 1");
        let mut tmp = self.from_altstack_count(count);
        self.join_count(&mut tmp[0], count - 1);
        self.rename(tmp[0], name);
        tmp[0]
    }

    pub fn get_script(&self) -> Script {
        script! {
            for s in self.script.iter() {
                { s.clone() }
            }
        }
    }

    pub fn move_var(&mut self, var: StackVariable) -> StackVariable {
        let offset = self.get_offset(var);
        self.data.remove_var(var);
        self.push(var);
        self.push_script( move_from(offset, var.size));
        var
    }
    
    pub fn copy_var(&mut self, var: StackVariable) -> StackVariable {
        let offset = self.get_offset(var);
        let new_var = StackVariable::new(self.next_counter(), var.size);
        self.push(new_var);
        self.rename(new_var, &format!("copy({})", self.data.names[&var.id]));
        self.push_script( copy_from(offset, var.size));
        new_var
    }

    pub fn get_offset(&self, var: StackVariable) -> u32 {
        let mut count = 0;
        for v in self.data.stack.iter().rev() {
            if var.id == v.id {
                return count;
            }
            count += v.size;
        }
        panic!("The var {:?} is not part of the stack", var);
    }

    pub fn get_var_from_stack(&self, depth: u32) -> StackVariable {
        self.data.stack[self.data.stack.len() - 1 - depth as usize]
    }
    
    pub fn get_var_name(&self, var: StackVariable) -> String {
        self.data.names[&var.id].clone()
    }

    pub fn get_script_len(&self) -> usize {
        self.script.len()
    }

    pub fn run(&self) -> StepResult {
        execute_step(self, self.script.len()-1)
    }

 
    pub fn show_stack(&self) {
        show_stack(&self.data, vec![]);
    }

    pub fn show_altstack(&self) {
        show_altstack(&self.data, vec![]);
    }

    pub fn copy_var_sub_n(&mut self, var: StackVariable, n: u32) -> StackVariable {
        let offset = self.get_offset(var);
        let offset_n = offset + var.size - 1 - n;

        let new_var = StackVariable::new(self.next_counter(), 1);
        self.push(new_var);
        self.push_script( copy_from(offset_n, 1));
        new_var
    }

    pub fn move_var_sub_n(&mut self, var: &mut StackVariable, n: u32) -> StackVariable {
        assert!(var.size > n, "The variable {:?} is not big enough to move n={}", var, n);
        let offset = self.get_offset(*var);
        let offset_n = offset + var.size - 1 - n;

        var.size -= 1;
        
        self.data.decrease_size(*var);

        if var.size == 0 {
            self.data.remove_var(*var);
        }

        let new_var = StackVariable::new(self.next_counter(), 1);
        self.push(new_var);
        self.push_script( move_from(offset_n, 1));
        new_var
    }

    pub fn join(&mut self, var1: &mut StackVariable) {

        let len = self.data.stack.len();
        for i in 0..len {
            if self.data.stack[i].id == var1.id {
                assert!(i + 1 < len, "The variable {:?} is the last one on the stack", var1);

                let next_size = self.data.stack[i+1].size;
                var1.size += next_size;
                self.data.increase_size(i, next_size);

                self.data.remove_var(self.data.stack[i+1]);
                break;
            }
        }
    }

    pub fn get_var(&self, depth: u32) -> StackVariable {
        let mut count = 0;
        for v in self.data.stack.iter().rev() {
            if count == depth {
                return *v;
            }
            count += v.size;
        }
        panic!("The depth {} is not valid", depth);
    }

    pub fn join_count(&mut self, var: &mut StackVariable, count: u32) -> StackVariable {
        for _ in 0..count {
            self.join(var)
        }
        *var
    }

    pub fn explode(&mut self, var: StackVariable) -> Vec<StackVariable> {
        let mut ret = Vec::new();
        let mut found  = self.data.stack.len();
        for n in 0..self.data.stack.len() {
            if self.data.stack[n].id == var.id {
                self.data.stack.remove(n);
                found = n;
                break;
            }
        }

        for i in 0..var.size {
            let new_var = StackVariable::new(self.next_counter(), 1);
            ret.push(new_var);
            self.data.stack.insert(found + i as usize, new_var);
        }
        ret

    }

    pub fn custom(&mut self, script: Script, consumes: u32, output: bool, to_altstack: u32, name: &str ) -> Option<StackVariable> {


        for _ in 0..consumes {
            self.data.pop_stack();
        }

        if output {
            let ret = Some(self.define(1, name));
            self.push_script(script);
            return ret;
        }

        for _ in 0..to_altstack {
            let c = self.next_counter();
            self.data.push_altstack(StackVariable::new(c, 1));
        }

        self.push_script(script);
        None
    }

    fn op(&mut self, op: Opcode, consumes: u32, output: bool, name: &str ) -> Option<StackVariable> {
        let mut s = Script::new();
        s.push_opcode(op);
        self.custom(s, consumes, output, 0, name)
    }

    pub fn op_negate(&mut self) -> StackVariable {
        self.op(OP_NEGATE, 1, true, "OP_NEGATE()").unwrap()
    }

    pub fn op_abs(&mut self) -> StackVariable {
        self.op(OP_ABS, 1, true, "OP_ABS()").unwrap()
    }

    pub fn op_add(&mut self) -> StackVariable {
        self.op(OP_ADD, 2, true, "OP_ADD()").unwrap()
    }

    pub fn op_sub(&mut self) -> StackVariable {
        self.op(OP_SUB, 2, true, "OP_SUB()").unwrap()
    }

    pub fn op_min(&mut self) -> StackVariable {
        self.op(OP_MIN, 2, true, "OP_MIN()").unwrap()
    }

    pub fn op_max(&mut self) -> StackVariable {
        self.op(OP_MAX, 2, true, "OP_MAX()").unwrap()
    }

    pub fn op_within(&mut self) -> StackVariable {
        self.op(OP_WITHIN, 3, true, "OP_WITHIN()").unwrap()
    }

    pub fn op_1add(&mut self) -> StackVariable {
        self.op(OP_1ADD, 1, true, "OP_1ADD()").unwrap()
    }

    pub fn op_1sub(&mut self) -> StackVariable {
        self.op(OP_1SUB, 1, true, "OP_1SUB()").unwrap()
    }

    pub fn op_not(&mut self) -> StackVariable {
        self.op(OP_NOT, 1, true, "OP_NOT()").unwrap()
    }

    pub fn op_booland(&mut self) -> StackVariable {
        self.op(OP_BOOLAND, 2, true, "OP_BOOLAND()").unwrap()
    }

    pub fn op_boolor(&mut self) -> StackVariable {
        self.op(OP_BOOLOR, 2, true, "OP_BOOLOR()").unwrap()
    }

    pub fn op_equal(&mut self) -> StackVariable {
        self.op(OP_EQUAL, 2, true, "OP_EQUAL()").unwrap()
    }

    pub fn op_numequal(&mut self) -> StackVariable {
        self.op(OP_NUMEQUAL, 2, true, "OP_NUMEQUAL()").unwrap()
    }

    pub fn op_numnotequal(&mut self) -> StackVariable {
        self.op(OP_NUMNOTEQUAL, 2, true, "OP_NUMNOTEQUAL()").unwrap()
    }

    pub fn op_lessthan(&mut self) -> StackVariable {
        self.op(OP_LESSTHAN, 2, true, "OP_LESSTHAN()").unwrap()
    }

    pub fn op_lessthanorequal(&mut self) -> StackVariable {
        self.op(OP_LESSTHANOREQUAL, 2, true, "OP_LESSTHANOREQUAL()").unwrap()
    }

    pub fn op_greaterthan(&mut self) -> StackVariable {
        self.op(OP_GREATERTHAN, 2, true, "OP_GREATERTHAN()").unwrap()
    }

    pub fn op_greaterthanorequal(&mut self) -> StackVariable {
        self.op(OP_GREATERTHANOREQUAL, 2, true, "OP_GREATERTHANOREQUAL()").unwrap()
    }

    pub fn op_numequalverify(&mut self) {
        self.op(OP_NUMEQUALVERIFY, 2, false, "OP_NUMEQUALVERIFY()");
    }

    pub fn op_0notequal(&mut self) -> StackVariable {
        self.op(OP_0NOTEQUAL, 1, true, "OP_0NOTEQUAL()").unwrap()
    }

    pub fn op_pick(&mut self) -> StackVariable {
        self.op(OP_PICK, 1, true, "OP_PICK()").unwrap()
    }

    pub fn op_ifdup(&mut self) -> StackVariable {
        panic!("OP_IFDUP not implemented as it's not possible to know if it would output a value");
    }

    pub fn op_roll(&mut self) -> StackVariable {
        panic!("OP_ROLL not implemented as it would consume an undefined position on the stack");
    }

    pub fn op_swap(&mut self) {
        let x = self.data.pop_stack();
        let y = self.data.pop_stack();
        self.data.push_stack(x);
        self.data.push_stack(y);

        self.op(OP_SWAP, 0, false, "OP_SWAP()");
    }

    pub fn op_2swap(&mut self) {
        let d = self.data.pop_stack();
        let c = self.data.pop_stack();
        let b = self.data.pop_stack();
        let a = self.data.pop_stack();
        self.data.push_stack(c);
        self.data.push_stack(d);
        self.data.push_stack(a);
        self.data.push_stack(b);

        self.op(OP_2SWAP, 0, false, "OP_2SWAP()");
    }

    pub fn op_tuck(&mut self) -> StackVariable {

        let var = StackVariable::new( self.next_counter(), 1 );
        let x = self.data.pop_stack();
        let y = self.data.pop_stack();
        assert!(x.size == 1 && y.size == 1, "OP_TUCK requires two elements of size 1");

        self.push(var);
        self.push(y);
        self.push(x);
        self.data.set_name(var, "OP_TuCK()");
        self.push_script(script!{OP_TUCK});
        var

    }

    pub fn op_rot(&mut self) {
        let x = self.data.pop_stack();
        let y = self.data.pop_stack();
        let z = self.data.pop_stack();
        assert!(x.size == 1 && y.size == 1 && z.size == 1, "OP_ROT requires three elements of size 1");
        self.data.push_stack(y);
        self.data.push_stack(x);
        self.data.push_stack(z);
        self.op(OP_ROT, 0, false, "OP_ROT()");
    }

    pub fn op_2rot(&mut self) {
        let f = self.data.pop_stack();
        let e = self.data.pop_stack();
        let d = self.data.pop_stack();
        let c = self.data.pop_stack();
        let b = self.data.pop_stack();
        let a = self.data.pop_stack();
        self.data.push_stack(c);
        self.data.push_stack(d);
        self.data.push_stack(e);
        self.data.push_stack(f);
        self.data.push_stack(a);
        self.data.push_stack(b);
        self.op(OP_2ROT, 0, false, "OP_2ROT()");
    }


    pub fn op_over(&mut self) -> StackVariable {
        let x = self.get_var_from_stack(1);
        let name = self.get_var_name(x);
        self.op(OP_OVER, 0, true, &name).unwrap()
    }
    
    pub fn op_2over(&mut self) -> (StackVariable, StackVariable) {
        let x = self.get_var_from_stack(3);
        let name = self.get_var_name(x);
        let y = self.get_var_from_stack(2);
        let namey = self.get_var_name(y);
        self.define(1, &name);
        (x, self.op(OP_2OVER, 0, true, &namey).unwrap())
    }

    pub fn op_verify(&mut self) {
        let _ = self.op(OP_VERIFY, 1, false, "OP_VERIFY()");
    }

    pub fn op_equalverify(&mut self) {
        let _ = self.op(OP_EQUALVERIFY, 2, false, "OP_EQUALVERIFY()");
    }

    pub fn number(&mut self, value: u32) -> StackVariable {
        self.var(1, script!{{value}}, &format!("number({:#x})", value))
    }

    pub fn number_u32(&mut self, value: u32) -> StackVariable {
        self.var(8, number_to_nibble(value), &format!("number_u32({:#x})", value))
    }

    pub fn op_true(&mut self) -> StackVariable {
        self.op(OP_TRUE, 0, true, "OP_TRUE").unwrap()
    }

    pub fn op_drop(&mut self) {
        self.op(OP_DROP, 1, false, "OP_DROP");
    }

    pub fn op_2drop(&mut self) {
        self.op(OP_2DROP, 2, false, "OP_2DROP");
    }

    pub fn op_depth(&mut self) -> StackVariable {
        self.op(OP_DEPTH, 0, true, "OP_DEPTH").unwrap()
    }

    pub fn op_nip(&mut self)  {
        let x = self.data.pop_stack();
        self.data.pop_stack();
        self.data.push_stack(x);
        self.op(OP_NIP, 0, false, "OP_NIP");
    }

    pub fn op_dup(&mut self) -> StackVariable {
        self.op(OP_DUP, 0, true, "OP_DUP").unwrap()
    }
    
    pub fn op_2dup(&mut self) -> (StackVariable, StackVariable) {
        let x = self.define(1, "OP_DUP");
        (x, self.op(OP_2DUP, 0, true, "OP_DUP").unwrap())
    }

    pub fn op_3dup(&mut self) -> (StackVariable, StackVariable, StackVariable) {
        let x = self.define(1, "OP_DUP");
        let y = self.define(1, "OP_DUP");
        (x, y, self.op(OP_3DUP, 0, true, "OP_DUP").unwrap())
    }


    pub fn get_value_from_table(&mut self, table: StackVariable, offset: Option<u32> ) -> StackVariable {
        self.number(self.get_offset(table)-1 + offset.unwrap_or(0));
        self.op_add();
        let v = self.op_pick();
        self.rename(v, &format!("from:({})", self.data.names[&table.id]));
        v
    }

    pub fn debug(&mut self) {
        println!("Max stack size: {}", self.max_stack_size);
        self.push_script(script!{});
        print_execute_step(self, self.script.len()-1);
    }



}



#[cfg(test)]
mod tests {


    pub use bitcoin_script::{define_pushable, script};
    define_pushable!();
    use super::{StackData, StackTracker, StackVariable};

    use crate::debugger::{debug_script, show_altstack, show_stack};
    use crate::script_util::*;

    #[test]
    fn test_one_var() {
        let mut stack = StackTracker::new();
        stack.number_u32(1234);
        stack.number_u32(1234);
        stack.custom(script!{ {verify_n(8)} }, 2, false, 0, "verify");
        stack.op_true();
        assert!(stack.run().success);

    }

    #[test]
    fn test_move_var() {
        let mut stack = StackTracker::new();
        let x = stack.number_u32(1234);
        let y = stack.number_u32(2345);
        stack.move_var(x);
        stack.number_u32(1234);
        stack.custom(script!{ {verify_n(8)} }, 2, false, 0, "verify");
        stack.drop(y);
        stack.op_true();
        assert!(stack.run().success);
    }

    #[test]
    fn test_copy_var() {
        let mut stack = StackTracker::new();
        let x = stack.number_u32(1234);
        let y = stack.number_u32(2345);
        let _ = stack.copy_var(x);
        let _ = stack.number_u32(1234);
        stack.custom(script!{ {verify_n(8)} }, 2, false, 0, "verify");
        stack.drop(y);
        stack.drop(x);
        stack.op_true();
        assert!(stack.run().success);
    }

    #[test]
    fn test_define_var() {
        let mut stack = StackTracker::new();
        let pre_existent = stack.define(8, "pre_existent");
        let _ = stack.number_u32(4444);
        let _ = stack.move_var(pre_existent);

        let script = script! {
            { number_to_nibble(1234) }
            { stack.get_script()}
            { number_to_nibble(1234) }
            { verify_n(8) }
            { drop_count(8) }
            OP_TRUE
        };

        let (ret,_) = debug_script(script);
        assert!(ret.result().unwrap().success);
    }


    #[test]
    fn test_copy_var_sub_n() {
        let mut stack = StackTracker::new();
        let x = stack.number_u32(0xdeadbeaf);
        let _ = stack.copy_var_sub_n(x, 0);
        let _ = stack.copy_var_sub_n(x, 7);

        let script = script! {
            { stack.get_script()}
            OP_15
            { verify_n(1) }
            OP_13
            { verify_n(1) }
            { drop_count(8) }
            OP_TRUE
        };

        let (ret,_) = debug_script(script);
        assert!(ret.result().unwrap().success);
    }

    #[test]
    fn test_move_sub_n() {
        let mut stack = StackTracker::new();
        let mut x = stack.number_u32(0xdeadbeaf);
        let _ = stack.move_var_sub_n(&mut x, 1);
        let _ = stack.move_var_sub_n(&mut x, 1);

        let script = script! {
            { stack.get_script()}
            OP_10
            { verify_n(1) }
            OP_14
            { verify_n(1) }
            { drop_count(6) }
            OP_TRUE
        };

        let (ret,_) = debug_script(script);
        assert!(ret.result().unwrap().success);
    }


    #[test]
    fn test_join() {
        let mut stack = StackTracker::new();
        let mut x = stack.number_u32(0xdeadbeaf);
        let _y = stack.number_u32(0x12345678);
        stack.join(&mut x);
        let _  = stack.number_u32(0x00000000);
        
        stack.move_var(x);


        let script = script! {
            { stack.get_script()}
            { number_to_nibble(0xdeadbeaf) }
            { number_to_nibble(0x12345678) }
            { verify_n(16) }
            { drop_count(8) }
            OP_TRUE
        };

        let (ret,_) = debug_script(script);
        assert!(ret.result().unwrap().success);
    }

    #[test]
    fn test_explode() {
        let mut stack = StackTracker::new();
        let x = stack.number_u32(0xdeadbeaf);
        let x_parts = stack.explode(x);

        stack.move_var(x_parts[2]);

        let script = script! {
            { stack.get_script()}
            OP_10
            { verify_n(1) }
            { drop_count(7) }
            OP_TRUE
        };

        let (ret,_) = debug_script(script);
        assert!(ret.result().unwrap().success);
    }

    #[test]
    fn test_get_from_table() {
        //one element table
        let mut stack = StackTracker::new();
        let x = stack.number(123);
        stack.number(0);
        stack.get_value_from_table(x, None);
        stack.number(123);
        stack.op_equalverify();
        stack.drop(x);
        stack.op_true();
        assert!(stack.run().success);


        //two element tables
        let mut stack = StackTracker::new();
        let x = stack.var(2, script!{ OP_15 OP_8}, "small table");

        stack.number(0);
        stack.get_value_from_table(x, None);
        stack.number(8);
        stack.op_equalverify();

        stack.number(1);
        stack.get_value_from_table(x, None);
        stack.number(15);
        stack.op_equalverify();

        stack.drop(x);
        stack.op_true();
        assert!(stack.run().success);

    }


    #[test]
    fn test_redo_log() {
        let mut data = StackData::new(true);
        let var1 = StackVariable::new(1, 1);
        let var2 = StackVariable::new(2, 1);
        data.push_stack(var1);
        data.set_name(var1, "var1");
        data.push_stack(var2);
        data.set_name(var2, "var2");
        data.pop_stack();
        data.push_altstack(var2);

        show_stack(&data, vec![]);
        show_altstack(&data, vec![]);

        let new_data = data.new_from_redo_height(data.redo_log.len());
        show_stack(&new_data, vec![]);
        show_altstack(&new_data, vec![]);

    }

    #[test]
    fn test_op_rot() {
        let mut stack = StackTracker::new();

        stack.number(1);
        let x = stack.number(2);
        stack.number(3);
        stack.op_rot();
        stack.number(1);
        stack.op_equalverify();

        stack.number(3);
        stack.op_equalverify();

        stack.drop(x);
        
        stack.op_true();

        assert!(stack.run().success);

    }

    #[test]
    fn test_op_over() {
        let mut stack = StackTracker::new();

        stack.number(1);
        let x = stack.number(2);
        stack.op_over();
        stack.number(1);
        stack.op_equalverify();

        stack.drop(x);
        

        assert!(stack.run().success);

    }

    #[test]
    fn test_op_tuck() {
        let mut stack = StackTracker::new();

        stack.number(0);
        stack.number(1);
        stack.number(2);
        stack.op_tuck();

        stack.op_nip();
        stack.op_equalverify();
        stack.op_1add();

        assert!(stack.run().success);

    }


    #[test]
    fn test_op_2over() {
        let mut stack = StackTracker::new();

        stack.number(0);
        stack.number(1);
        stack.number(2);
        stack.number(4);

        stack.op_2over();
        stack.to_altstack();

        stack.number(0);
        stack.op_equalverify();

        stack.op_2drop();
        stack.op_2drop();
        stack.from_altstack();

        assert!(stack.run().success);

    }

    #[test]
    fn test_op_2swap() {
        let mut stack = StackTracker::new();

        stack.number(0);
        stack.number(1);
        stack.number(2);
        stack.number(3);

        stack.op_2swap();

        stack.number(1);
        stack.op_equalverify();
        stack.number(0);
        stack.op_equalverify();

        stack.op_2drop();
        stack.op_true();

        assert!(stack.run().success);

    }

    #[test]
    fn test_op_2rot() {
        let mut stack = StackTracker::new();

        stack.number(1);
        stack.number(2);
        stack.number(3);
        stack.number(4);
        stack.number(5);
        stack.number(6);

        stack.op_2rot();

        stack.number(2);
        stack.op_equalverify();
        stack.number(1);
        stack.op_equalverify();

        stack.op_2drop();
        stack.op_2drop();
        stack.op_true();

        assert!(stack.run().success);

    }

    #[test]
    fn test_conditional() {
        let mut stack = StackTracker::new();

        stack.number(1);
        stack.number(2);
        stack.debug();
        stack.custom(script!{ 
            OP_DUP
            2
            OP_EQUAL
            OP_IF
                OP_1ADD
            OP_ELSE
                OP_1SUB
            OP_ENDIF
        }, 1, true, 0, "cond");

        stack.debug();
        stack.number(3);
        stack.debug();
        stack.op_equalverify();

        stack.debug();
        assert!(stack.run().success);

    }

    fn test_debug_visualization() {
        let mut stack = StackTracker::new();

        stack.custom(script!{ 1}, 0, false, 0, " ");
        stack.define(1, "one var");
        stack.debug();
        stack.number(1);
        stack.op_equal();
        stack.debug();

    }


}
