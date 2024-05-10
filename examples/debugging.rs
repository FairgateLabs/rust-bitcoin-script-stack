use bitcoin_script_stack::stack::StackTracker;



pub fn main() {
    let mut stack = StackTracker::new();        
                                            
    let var1 = stack.number(1);             
    let var2 = stack.number(10);        
    let _ = stack.copy_var(var1);   
    
    stack.debug();

    stack.move_var(var2);
    stack.drop(var2);      
    stack.op_equalverify();
    stack.op_true();       

}