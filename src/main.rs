use std::process;
use std::env;
use std::io;

const PROMPT: &str = "tsh> ";
static mut VERBOSE:i32 = 0;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut emit_prompt = 1;

    for c in 0..args[0].len() {
        match args[0].get(0..0).unwrap() {
            "-" => continue,
            "h" => usage(),
            "v" => unsafe { VERBOSE = 1},
            "p" => emit_prompt = 0,
            _ => usage(),
        }
    }
    

    loop {
        let mut buffer = String::new();
        if emit_prompt == 0 {
            println!("{}",PROMPT);
        }
        io::stdin().read_line(&mut buffer)
            .expect("Failed to read line");

        eval(buffer);
    }
}






fn eval(cmdline: String) {
    
}

fn parseline(cmdline: String) -> (i32,Vec<String>) {
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
        match array.get(0..0).unwrap() {
            "'" => {
                        let mut temp: String = array.drain(..0).collect();
                        let temp2: String = array.drain(0..array.find("'").unwrap()).collect();
                        temp += &temp2;

                        argv.push(temp);
                   },
            " " => {
                        array.drain(..0);
                   },
            _ => {
                        argv.push(array.drain(0..array.find(' ').unwrap()).collect());

                 }
            
        }

    }


    return (bg,argv);
}

fn parseargs(argv: Vec<&str>) -> (Vec<&str>,Vec<i32>,Vec<i32>) {
 
    return (vec!["temp"],vec![1],vec![1]);
}

fn builtin_cmd(argv: Vec<&str>) -> i32 {
    if argv[0] == "quit" {
        process::exit(0);
    }
    else if argv[0] == "exit" {
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
