pub use bitcoin_script::{define_pushable, script};
define_pushable!();
pub use bitcoin::ScriptBuf as Script;

use crate::stack::StackTracker;

pub fn move_from(address: u32, size: u32) -> Script {
    script! {
        for _ in 0..size {
            { address + size - 1 }
            OP_ROLL
        }
    }
}

pub fn copy_from(address: u32, size: u32) -> Script {
    script! {
        for _ in 0..size {
            { address + size - 1 }
            OP_PICK
        }
    }
}

pub fn drop_count(n: u32) -> Script {
    script! {
        for _ in 0..n / 2 {
            OP_2DROP
        }
        if n & 1 == 1 {
            OP_DROP
        }
    }
}

pub fn toaltstack(n: u32) -> Script {
    script! {
        for _ in 0..n {
            OP_TOALTSTACK
        }
    }
}

pub fn fromaltstack(n: u32) -> Script {
    script! {
        for _ in 0..n {
            OP_FROMALTSTACK
        }
    }
}

pub fn number_to_byte(n: u32) -> Script { 
    script! {
       for i in (0..4).rev() { 
            { (n >> (i * 8)) & 0xFF } 
        } 
    }
}


pub fn number_16_to_nibble(n: u16) -> Script { 
    script! {
       for i in (0..4).rev() { 
            { (n as u32 >> (i * 4)) & 0xF } 
        } 
    }
}

pub fn number_to_nibble(n: u32) -> Script { 
    script! {
       for i in (0..8).rev() { 
            { (n >> (i * 4)) & 0xF } 
        } 
    }
}


pub fn byte_to_nibble(n: u8) -> Script { 
    script! {
       for i in (0..2).rev() { 
            { (n >> (i * 4)) & 0xF } 
        } 
    }
}

pub fn verify_n(n: u32) -> Script {
    script! {
        for i in 0..n {
            { n - i}
            OP_ROLL
            OP_EQUALVERIFY
        }
    }
}

pub fn reverse_u32() -> Script {
    script! {
        OP_SWAP
        OP_ROT
        3
        OP_ROLL
        OP_2ROT
        OP_SWAP
        6
        OP_ROLL
        7
        OP_ROLL
    }
}


pub fn quot_and_modulo_big(stack: &mut StackTracker, number: u32, quot: u32, quotient: bool) {
    if quotient {

        stack.custom(script! {
            OP_DUP
            { number }
            OP_GREATERTHANOREQUAL
            OP_IF
                { number }
                OP_SUB
                { quot }
            OP_ELSE
                0
            OP_ENDIF
        }, 0, true, 0, "quotient");

    } else {
        stack.custom(script! {
            OP_DUP
            { number }
            OP_GREATERTHANOREQUAL
            OP_IF
                { number }
                OP_SUB
            OP_ENDIF
        }, 0, false, 0, "" );
    }

}

