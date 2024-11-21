pub use bitcoin_script::script;
pub use bitcoin_script::builder::StructuredScript as Script;

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

pub fn number_to_nibble_u8(n: u32) -> Script { 
    script! {
       for i in (0..4).rev() { 
            { (n >> (i * 8)) & 0xFF } 
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
