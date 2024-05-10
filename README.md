### Disclaimer
This is a work in progress library.

### Bitcoin Script Stack

This library aims to help in the development of complex bitcoin scripts.

On the process of creating an optimized version of SHA256 on chain, was necessary to track the position of moving parts on the stack and a lot of debugging effort.

StackTracker and StackVariable are the core parts of this lib and as this an example of ussage it that allows to do the following:
```
let mut stack = StackTracker::new();    //creates the tracker
                                        // STACK:
let var1 = stack.number(1);             // 1
let var2 = stack.number(10);            // 1 10
let copy_of_1 = stack.copy_var(var1);   // 1 10 1
stack.move_var(&mut var2);              // 1 1 10
stack.drop(var2);                       // 1 1
stack.op_equalverify();                 // 
stack.op_true();                        // 1
assert!(stack.run().success)
```

### Debugging
Debugging the scripts

On any moment that StackTracker objects is being constructed is possible to debug it's internal state using the functions inside debugger.
i.e:

```
let mut stack = StackTracker::new();        //creates the tracker
                                        // STACK:
let var1 = stack.number(1);             // 1
let mut var2 = stack.number(10);        // 1 10
let copy_of_1 = stack.copy_var(var1);   // 1 10 1
....
stack.debug();
...

```
This would output something like this:
```
Last opcode: "OP_PICK"
======= STACK: ======
id: 1       | size: 1       | name: number(0x1)          |  1
id: 2       | size: 1       | name: number(0xa)          |  a
id: 3       | size: 1       | name: copy(number(0x1))    |  1
==== ALT-STACK: ====
```

### Interactive Debugging
There is also an interactive debugger that allows to run the script step by step.
Take a look to [examples/interactive.rs](examples/interactive.rs)
To enable it it requires `--features interactive`

```
Interactive mode. n: next bp | p: previous bp | Step commands: <- (-1) | -> (+1) | Up (-100) | Down (+100) | PgUp (-100) | PgDown (+100) | +Shift (x10) | t (trim) | q (exit)
Step: 8 BP:
Last opcode: OP_TOALTSTACK
======= STACK: ======
id: 1       | size: 1       | name: number(0x1)          |  1
id: 2       | size: 1       | name: number(0xa)          |  a
==== ALT-STACK: ====
id: 7       | size: 1       | name: OP_ADD()             |  8
```

### Breakpoint
When writing complex functions or sripts that performs a lot of operations tracking the right step becomes a challenge.
So it's is possible to set up breakpoints that will make the debugging easier.
```
stack.custom(....)                          // some coplex operation
stack.set_breakpoint("breakpoint-name-1");  // set breakpoint
```




