use bitcoin_script_stack::stack::StackTracker;
#[cfg(feature = "interactive")]
use bitcoin_script_stack::interactive::interactive;

#[allow(dead_code)]
fn example(error: bool) -> StackTracker {

    let mut stack = StackTracker::new();
    stack.number(1);
    stack.number(10);
    stack.number(5);
    stack.number(3);
    stack.number(3);
    if error {
        stack.number(5);
    } else {
        stack.number(3);
    }
    stack.op_equalverify();
    stack.op_add();
    stack.to_altstack();
    stack.to_altstack();
    stack.from_altstack();
    stack.from_altstack();
    stack.op_2drop();
    stack
}


fn main() {
    #[cfg(feature = "interactive")] 
    {
        interactive(&example(false));
        interactive(&example(true));
    }
    #[cfg(not(feature = "interactive"))]
    println!("Executed with --features interactive");
}
