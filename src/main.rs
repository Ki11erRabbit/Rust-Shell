use std::process;
use std::env;
use std::io::{self,Write};

const PROMPT: &str = "tsh> ";
static mut VERBOSE:i32 = 0;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut emit_prompt = 1;
    
    if args.len() > 1 { 
        for c in 0..args[1].len() {
            match args[1].get(0..0).unwrap() {
                "-" => continue,
                "h" => usage(),
                "v" => unsafe { VERBOSE = 1},
                "p" => emit_prompt = 0,
                _ => usage(),
            }
        }
    }
    

    loop {
        let mut buffer = String::new();
        if emit_prompt == 1 {
            print!("{}",PROMPT);
            io::stdout().flush().unwrap();
        }
        io::stdin().read_line(&mut buffer)
            .expect("Failed to read line");

        eval(buffer);
    }
}






fn eval(cmdline: String) {
    println!("Eval");
    let argv: Vec<String>;
    let bg: i32;
    let pair = parseline(cmdline);
    bg = pair.0;
    argv = pair.1;
    
    println!("{:?}",argv);

    if builtin_cmd(argv) == 1 {
        return;
    }
}

fn parseline(cmdline: String) -> (i32,Vec<String>) {
    println!("Parseline");
    let mut argv: Vec<String> = Vec::new();
    let bg: i32;
    let mut array = cmdline.clone(); 
    array.pop();
    array.push(' ');

    if cmdline.get(cmdline.len()..(cmdline.len()-1)) == Some("&") {
        bg = 1;
    }
    else {
        bg = 0;
    }

    while array.len() != 0 {
        //println!("{}",array);
        //println!("array len: {}", array.len());
        match array.get(..1).unwrap() {
            "'" => {
                        let mut temp: String = array.drain(..1).collect();
                        //println!("{}!", array);
                        let temp2: String = array.drain(..array.find('\'').unwrap()+1).collect();
                        temp += &temp2;

                        argv.push(temp);
                   },
            " " => {
                        //println!("Space");
                        array.drain(..1);
                   },
            _ => {
                        //println!("Default");
                        argv.push(array.drain(0..array.find(' ').unwrap()).collect());

                 } 
        }

        //println!("{:?}",argv);

    }


    return (bg,argv);
}

fn parseargs(argv: Vec<&str>) -> (Vec<&str>,Vec<i32>,Vec<i32>) {
     
    return (vec!["temp"],vec![1],vec![1]);
}

fn builtin_cmd(argv: Vec<String>) -> i32 {
    if argv.len() == 0 {
        return 1;
    }
    else if argv[0].as_str() == "quit" {
        process::exit(0);
    }
    else if argv[0].as_str() == "exit" {
        process::exit(0);
    }
    
    return 0;

}


fn usage() {
    println!("Usage: shell [-hvp]");
    println!("   -h   print this message");
    println!("   -v   print additional diagnostic information");
    println!("   -p   do not emit a command prompt");
    process::exit(1);

}
