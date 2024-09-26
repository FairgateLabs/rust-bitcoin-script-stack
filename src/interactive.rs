
use std::io::Stdout; 
use std::io::stdout;

use crossterm::style::SetBackgroundColor;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;
use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor, SetAttribute, Attribute},
    cursor::MoveTo,
    event::{read, KeyEventKind, Event, KeyCode},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, Clear, ClearType},
    terminal::size as terminal_size,
    ExecutableCommand,
};

use crate::debugger::execute_step;
use crate::stack::StackTracker;

fn show_command(stdout: &mut Stdout, command: &str, help: &str ) {
    execute! (
        stdout,

        SetAttribute(Attribute::Bold),
        SetForegroundColor(Color::Green),
        Print(command),
        ResetColor,
        SetAttribute(Attribute::Reset),
        Print(help)
    ).unwrap();
}

fn print_cut_text(text: &str) {
    let mut stdout = stdout();
    let (width, _) = terminal_size().unwrap_or((80,0)); // Get terminal dimensions (width and height)

    // Cut the string to fit the terminal width, if necessary
    let cut_text = if text.len() as u16 > width {
        &text[0..width as usize]
    } else {
        text
    };

    // Print the cut text
    execute!(
        stdout,
        Print(cut_text),
        Print("\r\n")
    ).unwrap();
}
fn print_stack_line(i:usize, s: &str, trim:bool) {
    let mut stdout = stdout();
    if i % 2 == 0 {
        execute!(stdout, SetBackgroundColor(Color::Rgb{r:30,g:30,b:30})).unwrap();
    } else {
        execute!(stdout, SetBackgroundColor(Color::Rgb{r:20,g:20,b:20})).unwrap();
    }
    if trim {
        print_cut_text(s);
    } else {
        print!("{}\r\n", s);
    }
    execute!(stdout, SetBackgroundColor(Color::Reset)).unwrap();
}

fn show_step(stdout : &mut Stdout, stack: &StackTracker, step: usize, bp_name: &str, trim: bool) {

    // Enter an alternate screen to not mess up the user's terminal buffer
    stdout.execute(EnterAlternateScreen).unwrap();
    stdout.execute(Clear(ClearType::All)).unwrap();

    stdout.execute(MoveTo(0, 0)).unwrap();

    
    execute!(stdout, SetForegroundColor(Color::DarkGreen), Print("Interactive mode. ")).unwrap();
    show_command(stdout, "n", ": next bp | ");
    show_command(stdout, "p", ": previous bp | ");
    execute!( stdout, SetForegroundColor(Color::DarkGreen), Print("Step commands: "), ResetColor).unwrap();
    show_command(stdout, "<-", " (-1) | ");
    show_command(stdout, "->", " (+1) | ");
    show_command(stdout, "Up", " (-100) | ");
    show_command(stdout, "Down", " (+100) | ");
    show_command(stdout, "PgUp", " (-100) | ");
    show_command(stdout, "PgDown", " (+100) | ");
    show_command(stdout, "+Shift", " (x10) | ");
    show_command(stdout, "t", " (trim) | ");
    show_command(stdout, "q", " (exit)");
    execute!(stdout, 
                Print("\r\n"),
                SetForegroundColor(Color::Blue), Print("Step: "), ResetColor,
                Print(step),
                SetForegroundColor(Color::Blue), Print(" BP: "), ResetColor,
                Print(bp_name),
            ).unwrap();

    let res = execute_step(stack, step);
    execute!(stdout, 
        Print("\r\n"),
        Print("Last opcode: "),
        SetForegroundColor(Color::DarkGrey), 
        Print(res.last_opcode),
        ResetColor,
    ).unwrap();

    if res.error {
        execute!(stdout, 
            SetForegroundColor(Color::Red), 
            Print(" Error: "), 
            SetAttribute(Attribute::Bold),
            Print(res.error_msg),
            SetAttribute(Attribute::Reset),
            ResetColor,
        ).unwrap();
    }
    if res.success {
        execute!(stdout, 
            SetForegroundColor(Color::Green), 
            SetAttribute(Attribute::Bold),
            Print(" Success!"),
            SetAttribute(Attribute::Reset),
            ResetColor,
        ).unwrap();
    }

    execute!(stdout, Print("\r\n")).unwrap();
    execute!(stdout, Print("======= STACK: ======\r\n")).unwrap();
    for (i, s) in res.stack.iter().enumerate() {
        print_stack_line(i, s, trim);
    }
    execute!(stdout, Print("==== ALT-STACK: ====\r\n")).unwrap();
    for (i,s) in res.altstack.iter().enumerate() {
        print_stack_line(i, s, trim);
    }


}


pub fn interactive(stack: &StackTracker) {
    let mut stdout = stdout();

    enable_raw_mode().expect("Failed to enable raw mode");

    show_step(&mut stdout, stack, 0, "start", true);

    let mut step : i32 = 0;
    let max_step = stack.get_script_len() as i32 - 1;
    let mut bp_name = String::new();
    let mut trim = true;

    // Wait for a key press
    loop {
        if let Event::Key(key_event) = read().unwrap() {
            if key_event.kind != KeyEventKind::Press {
                continue;
            }
            if key_event.code == KeyCode::Char('q') || key_event.code == KeyCode::Esc {
                break; // Exit if 'q' or 'Esc' is pressed
            }
            let mut mult = 1;
            if key_event.modifiers == crossterm::event::KeyModifiers::SHIFT {
                mult = 10;
            }
            let mut change : i32 = 0;
            if key_event.code == KeyCode::Char('n') {
                let x = stack.get_next_breakpoint(step as u32);
                if x.is_some() {
                    step = x.as_ref().unwrap().0 as i32;
                    bp_name = x.as_ref().unwrap().1.to_string();
                }
            }
            if key_event.code == KeyCode::Char('p') {
                let x = stack.get_prev_breakpoint(step as u32);
                if x.is_some() {
                    step = x.as_ref().unwrap().0 as i32;
                    bp_name = x.as_ref().unwrap().1.to_string();
                }
            }
            if key_event.code == KeyCode::Char('t') {
                trim = !trim;
            }
            if key_event.code == KeyCode::Left {
                change = -1;
            }
            if key_event.code == KeyCode::Right {
                change = 1;
            }
            if key_event.code == KeyCode::Home {
                step = 0;
            }
            if key_event.code == KeyCode::End {
                step = max_step;
            }
            if key_event.code == KeyCode::Up {
                change = -100;
            }
            if key_event.code == KeyCode::Down {
                change = 100;
            }
            if key_event.code == KeyCode::PageUp {
                change = -1000;
            }
            if key_event.code == KeyCode::PageDown {
                change = 1000;
            }
            change *= mult;
            if change < 0 {
                if step+change < 0 {
                     step = 0; 
                } else {
                    step += change;
                }
            }
            if change > 0 {
                if step + change < max_step  {
                    step += change;
                } else {
                    step = max_step;
                }
            }
            show_step(&mut stdout,stack, step as usize, &bp_name, trim);
        }
    }

    // Leave alternate screen to return to the normal terminal state
    stdout.execute(LeaveAlternateScreen).unwrap();
    disable_raw_mode().expect("Failed to disable raw mode");
}


