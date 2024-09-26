### Disclaimer
This is a work in progress library and future changes might break backward compatibility. 

### Bitcoin Script Stack

This library aims to help in the development of complex bitcoin scripts.

In the process of creating an optimized version of SHA256 on chain, it was necessary to track the position of moving parts on the stack and a lot of debugging effort.

StackTracker and StackVariable are the core parts of this lib and as this an example of usage it that allows to do the following:
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

At any moment that StackTracker objects are being constructed it is possible to debug its internal state using the functions inside the debugger.
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
There is also an interactive debugger that allows running the script step by step.
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

### Breakpoints
When writing complex functions or scripts that perform a lot of operations, tracking the right step becomes a challenge.
So it's is possible to set up breakpoints that will make the debugging easier.
```
stack.custom(....)                          // some complex operation
stack.set_breakpoint("breakpoint-name-1");  // set breakpoint
```

### OP_ROLL
Op roll is not implemented as direct operation as the modification of the stack can not be calculated in advance.
Use `move_var` and `move_var_sub_n` to achieve the same goal, and take advantage of position tracking.


### Conditionals
For now, conditional branches are not 100% supported.
The main constraint is that on each branch result in the same amount of variables consumed and produced needs to be the same.

There are two ways of using it: 
With `custom` (or `custom_ex`) and writing the script as it is done in this example: `test_conditional` on [src/stack.rs](src/stack.rs)

Or using `open_if` and `end_if`. The first function returns two copies of the stack, one for the true branch and one for the false branch.
Take a look to the example:
`test_open_if` on [src/stack.rs](src/stack.rs). Some internal branch debugging seems to be possible but it was not very well tested yet.

At some point a different way to handle conditionals might be implemented as part of the lib, allowing bettery debugging of each branch.


### OP_IFDUP
As this op modifies the stack depth at runtime it is not possible to implement it here.

-----

### TODO:
List of pending tasks:
- Improve branching debugging 
- Define transaction templates to validate withness inputs 
