mod builtin;
mod job;

use crate::job::{ProccessState,Job,Jobs};
use std::process::{self,Command, Stdio, Child};
use std::env;
use std::io::{self,Write};
use std::fs::File;
use std::io::prelude::*;
use std::os::unix::process::CommandExt;
use signal_hook::{consts::*, iterator::Signals};
use std::thread;
use nix::unistd::Pid;
use nix::unistd::pause;
use nix::sys::signal::{self, Signal};
use nix::sys::wait;
use std::collections::BTreeMap;





const PROMPT: &str = "tsh> ";
static mut VERBOSE:i32 = 0;
static mut JOBS: Jobs = Jobs::new();



fn main() {
    let mut aliases: BTreeMap<String,(String,Vec<String>)> = BTreeMap::new();
    let mut variables: BTreeMap<String,String> = BTreeMap::new();
    let args: Vec<String> = env::args().collect();
    let mut emit_prompt = true;
    let mut path_in_prompt = false;
   
    if args.len() > 1 {

        if args[1].contains("-") {
            let mut bad_input = true;
            if args[1].contains("h") {
                usage();
            }
            if args[1].contains("v") {
                unsafe { VERBOSE = 1 };
                bad_input = false;
            }
            if args[1].contains("p") {
                emit_prompt = false;
                bad_input = false;
            }
            if args[1].contains("a") {
                path_in_prompt = true;
                bad_input = false;
            }
            if bad_input {
                usage();
            }
        }
    }    

    setup_signal_handlers();

    
    match parse_rshrc(&mut aliases,&mut variables) {
        Err(e) => eprintln!("{}",e),
        Ok(_) => (),
    }
    


    loop {
        let mut buffer = String::new();
        if emit_prompt {
            let curr_dir = env::current_dir().unwrap();
            let print_prompt;
            if path_in_prompt {
                print_prompt = format!("tsh {} > ",curr_dir.into_os_string().to_str().unwrap());
            }
            else {
                print_prompt = PROMPT.to_string();
            }
            print!("{}",print_prompt);
            io::stdout().flush().unwrap();
        }
        io::stdin().read_line(&mut buffer)
            .expect("Failed to read line");

        eval(&buffer,&mut aliases,&mut variables);
    }
}

fn parse_rshrc(aliases: &mut BTreeMap<String, (String,Vec<String>)>, variables: &mut BTreeMap<String, String>) -> std::io::Result<()> {
    let key = "HOME";
    match env::var(key) {
        Err(_) => {
            eprintln!("User's home not set!\n Unable to read .rshrc");
        },
        Ok(val) => {
            let rshrc_location = val + "/.rshrc";
            
            let mut file = File::open(rshrc_location)?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            let lines: Vec<&str> = contents.split('\n').collect();

            for line in lines.iter() {
                eval(line,aliases,variables);
            }



        }
    }
    Ok(())
}

fn setup_signal_handlers() {

   let signals = Signals::new(&[SIGINT,SIGCHLD,SIGTSTP]);
    thread::spawn(move || {
        for sig in &mut signals.unwrap(){

            if sig == SIGINT {
                if unsafe {VERBOSE} == 1 {
                    println!("sigint_handler");
                }
                
                for job in unsafe {JOBS.iter_mut()} {
                    
                    match job.state {
                        ProccessState::FG => {
                                
                                signal::kill(Pid::from_raw(-job.pgid),Signal::SIGINT).unwrap();
                                unsafe {
                                    match JOBS.delete_job(*job.pids.last().unwrap()) {
                                        Err(e) => eprintln!("{}",e),
                                        Ok(_) => (),
                                    }
                                }

                        }
                        _ => {
                                continue;
                        }
                    }

                }

            }
            else if sig == SIGCHLD {
                if unsafe {VERBOSE} == 1 {
                    println!("sigchild_handler");
                }
                let flags: wait::WaitPidFlag = wait::WaitPidFlag::WNOHANG | wait::WaitPidFlag::WUNTRACED;

                loop {
                    match wait::waitpid(Pid::from_raw(-1), Some(flags)) {
                        Err(_) => break,
                        Ok(x) => {
                            match x {
                                wait::WaitStatus::StillAlive => break,
                                wait::WaitStatus::Exited(pid,_status) => {
                                    unsafe { 
                                        match JOBS.delete_job(pid.as_raw()) {
                                            Ok(_) => (),
                                            Err(_) => (),
                                        } 
                                    };

                                }
                                wait::WaitStatus::Signaled(pid, signal, _core_dump) => {
                                    let job;
                                    unsafe { 
                                        match JOBS.get_job_pid(pid.as_raw()) {
                                            Some(x) => {
                                                job = x;
                                                println!("Job [{}] ({}) terminated by signal {}",job.jid,pid,signal);
                                            },
                                            None => (),
                                        } 
                                    };

                                    unsafe { 
                                        match JOBS.delete_job(pid.as_raw()) {
                                            Ok(_) => (),
                                            Err(_) => (),
                                        } 
                                    };
                                },
                                wait::WaitStatus::Stopped(pid,signal) => {
                                    let mut job;
                                    unsafe { job = JOBS.get_job_pid(pid.as_raw()).unwrap(); };
                                    
                                    job.state = ProccessState::ST;
                                     
                                    
                                    println!("Job [{}] ({}) stopped by signal {}",job.jid,pid,signal);
                                }
                                _ => (),
                            }
                        }

                    }
                }
                        
                    
                


            }
            else if sig == SIGTSTP {

                if unsafe {VERBOSE} == 1 {
                    println!("sigtstp_handler");
                }
                
                for job in unsafe {JOBS.iter_mut()} {
                    
                    match job.state {
                        ProccessState::FG => {

                                signal::kill(Pid::from_raw(-job.pgid),Signal::SIGTSTP).unwrap();        
                        }
                        _ => {
                                continue;
                        }
                    }

                }

            }
        }
    });

}


fn eval(cmdline: &str, aliases: &mut BTreeMap<String,(String,Vec<String>)>, variables: &mut BTreeMap<String, String>) {
    if unsafe { VERBOSE == 1 } {
        println!("Eval");
    }
    let argv: Vec<String>;
    let bg: bool;
    let pair = parseline(&cmdline);
    bg = pair.0;
    argv = pair.1;
    
    if unsafe { VERBOSE == 1 } {
        println!("{:?}",argv);
    }

    if builtin_cmd(&argv,aliases,variables) == 1 {
        return;
    }

    let set = parseargs(&argv,&aliases,variables);

    let cmds = set.0;
    let args = set.1;
    let env = set.2;
    let stdin_redir = set.3;
    let stdout_redir = set.4;

    create_subproccesses(cmdline,argv,cmds, args, env,stdin_redir, stdout_redir,bg);


}

fn create_subproccesses(cmdline:&str,argv: Vec<String>,cmds: Vec<String>, args: Vec<Vec<String>>, env: Vec<(String,String)>,stdin_redir: Vec<usize>, stdout_redir: Vec<usize>,bg: bool) {

    if unsafe { VERBOSE == 1 } {
        println!("cmds {:?}",cmds);
        println!("args {:?}",args);
        println!("env {:?}",env);
        println!("stdin {:?}",stdin_redir);
        println!("stdout {:?}",stdout_redir);
        
        println!("\npid = {}", process::id());
    }
    
    let mut processes: Vec<Child> = Vec::new(); 
    let mut pids: Vec<i32> = Vec::new(); 
    let mut group_id = 0;
    for i in 0..cmds.len() {
        let mut command: &mut Command = &mut Command::new(cmds[i].as_str());
        command = command.process_group(group_id);
        //println!("{}",cmds[i].len());
        command = command.args(args[i].as_slice());
        
        for (key, val) in env.iter() {
            command = command.env(key,val);
        }

        if stdout_redir[i] == usize::MAX {
            command = command.stdout(Stdio::piped());
        }
        else if stdout_redir[i] != usize::MAX && stdout_redir[i] != usize::MAX -1{
            let file = File::create(argv[stdout_redir[i]].as_str()).expect("Bad file path");
            command = command.stdout(file);
        }
        if stdin_redir[i] == usize::MAX {
            command = command.stdin(processes[i-1].stdout.take().unwrap());
        }
        else if stdin_redir[i] != usize::MAX && stdin_redir[i] != usize::MAX -1{
            let file = File::open(argv[stdin_redir[i]].as_str()).expect("Bad file path");
            command = command.stdin(file);
        } 
        

        match command.spawn() {
            Ok(x) => processes.push(x),
            Err(_) => {
                eprintln!("{}: Command not found", cmds[i]);
                return;
            },
        }
        if unsafe { VERBOSE == 1 } {
            
            println!("pid child = {}", processes[i].id());
        }
        

        if i == 0 {
            group_id = processes[i].id().try_into().unwrap();
            pids.push(group_id);
        }
        else {
            pids.push(processes[i].id().try_into().unwrap());
        }
    }
    
    if !bg {
        if unsafe { VERBOSE == 1 } {
            println!("spawning in forground");
        }
        unsafe {
            JOBS.addjob(&pids, pids[0], ProccessState::FG, cmdline);
        } 
        waitfg(pids[0]);
    }
    else if bg {
        if unsafe { VERBOSE == 1 } {
            println!("spawning in background");
        }

        unsafe {
            JOBS.addjob(&pids, pids[0], ProccessState::BG, cmdline);
        } 

    }

}

fn parseline(cmdline: &str) -> (bool,Vec<String>) {
    if unsafe { VERBOSE == 1 } {
        println!("Parseline");
    }
    let mut argv: Vec<String> = Vec::new();
    let bg: bool;
    let mut array = cmdline.to_string(); 
    if array.contains("\n") {
        array.pop();
    } 
    array.push(' ');

    if cmdline.rfind("&") != None {
        bg = true;
    }
    else {
        bg = false;
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
            "|" => {
                        //println!("Space");
                        argv.push(array.drain(..1).collect());
                   },
            "<" => {
                        //println!("Space");
                        argv.push(array.drain(..1).collect());
                   },
            ">" => {
                        //println!("Space");
                        argv.push(array.drain(..1).collect());
                   },
            "=" => {
                        //println!("Space");
                        argv.push(array.drain(..1).collect());
                   },
            "&" => {
                        //println!("Space");
                        array.drain(..1);
                   },
            _ => {
                        //println!("Default");
                        argv.push(array.drain(0..array.find(|c: char| c == '>' || c == '|' || c == '<' || c == ' ' || c == '=').unwrap()).collect());

                 } 
        }

        //println!("{:?}",argv);

    }


    return (bg,argv);
}

fn parseargs(argv: &Vec<String>,aliases: & BTreeMap<String,(String,Vec<String>)>, variables: &mut BTreeMap<String, String>) -> (Vec<String>,Vec<Vec<String>>,Vec<(String,String)>,Vec<usize>,Vec<usize>) {
    if unsafe { VERBOSE == 1 } {
        println!("parseargs");
    }
    let mut cmds: Vec<String> = Vec::new();
    let mut args: Vec<Vec<String>> = Vec::new();
    let mut env: Vec<(String,String)> = Vec::new();
    let mut stdin_redir: Vec<usize> = Vec::new();
    let mut stdout_redir: Vec<usize> = Vec::new();

    let mut curr_cmd = 0;
    cmds.push("".to_string());
    args.push(Vec::new());
    stdin_redir.push(usize::MAX -1);
    stdout_redir.push(usize::MAX -1);

    let mut skip = false;
    for i in 0..argv.len() {
        match argv[i].as_str() {
            "|" => {
                    stdout_redir[curr_cmd] = usize::MAX;
                    stdin_redir.push(usize::MAX);
                    stdout_redir.push(usize::MAX-1);
                    cmds.push("".to_string());
                    args.push(Vec::new());
                    curr_cmd += 1;
                },
            "<" => {
                    //stdin_redir[curr_cmd] = args[curr_cmd].len();
                    stdin_redir[curr_cmd] = i + 1;
                    skip = true;
                },
            ">" => {
                    //stdout_redir[curr_cmd] = args[curr_cmd].len();
                    stdout_redir[curr_cmd] = i + 1;
                    skip = true;
                },
            "=" => {
                    skip = true;
                },
            _ => {
                    if skip {
                        skip = false;
                        continue;
                    }

                    if i + 2 < argv.len() && argv[i+1].as_str() == "=" {
                        env.push((argv[i].clone(),argv[i+2].clone()));
                        skip = true;
                        if unsafe { VERBOSE == 1} {
                            println!("env: {:?}",env);
                        }
                        continue;
                    }

                    if cmds[curr_cmd].as_str() == "" {
                        let cmd;
                        match aliases.get(&argv[i]) {
                            Some(val) => {
                                cmd = val.0.clone();
                                args[curr_cmd] = val.1.clone();
                            },
                            None =>  {

                                if argv[i].get(..1) == Some("$") {
                                    match env::var(argv[i].clone().drain(1..).collect::<String>()) {
                                        Ok(val) => {
                                            if val.contains(" ") {
                                                let mut var:Vec<&str> = val.split(" ").collect();
                                                cmd = var[0].to_string();
                                                var.remove(0);
                                                for arg in var.iter() {
                                                    args[curr_cmd].push(arg.to_string());
                                                } 
                                            }
                                            else {
                                                cmd = val;
                                            }
                                        }
                                        Err(_) => {
                                            match variables.get(&argv[i].clone().drain(1..).collect::<String>()) {
                                                Some(val) => {
                                                    if val.contains(" ") {
                                                        let mut var: Vec<&str> = val.split(" ").collect();
                                                        cmd = var[0].to_string();
                                                        var.remove(0);
                                                        for arg in var.iter() {
                                                            args[curr_cmd].push(arg.to_string());
                                                        }
                                                    }
                                                    else {
                                                        cmd = val.to_string();
                                                    }
                                                },
                                                None => cmd = argv[i].as_str().to_string(),
                                            }

                                        },//cmd = argv[i].as_str().to_string(),
                                    }
                                }
                                else {
                                    cmd = argv[i].as_str().to_string();
                                }

                            },
                        }
                        cmds[curr_cmd] = cmd;
                    }
                    else {
                        if argv[i].get(..1) == Some("$") {

                            match env::var(argv[i].clone().drain(1..).collect::<String>()) {
                                Ok(val) => {
                                    if val.contains(" ") {
                                        let var:Vec<&str> = val.split(" ").collect();
                                        for arg in var.iter() {
                                            args[curr_cmd].push(arg.to_string());
                                        } 
                                    }
                                    else {
                                        args[curr_cmd].push(val);
                                    }
                                }
                                Err(_) => {
                                    match variables.get(&argv[i].clone().drain(1..).collect::<String>()) {
                                        Some(val) => {
                                            if val.contains(" ") {
                                                let var: Vec<&str> = val.split(" ").collect();
                                                for arg in var.iter() {
                                                    args[curr_cmd].push(arg.to_string());
                                                }
                                            }
                                            else {
                                                args[curr_cmd].push(val.to_string());
                                            }
                                        },
                                        None => args[curr_cmd].push(argv[i].as_str().to_string()),
                                    }
                                },//args[curr_cmd].push(argv[i].as_str().to_string()),
                            }

                        }
                        else {
                            args[curr_cmd].push(argv[i].as_str().to_string());
                        }
                    }
                }
        } 

    }
    

    return (cmds,args,env,stdin_redir,stdout_redir);
}

pub fn do_bgfg(argv: &Vec<String>) {
    if argv.len() == 1 {
        println!("{} command requires PID or %jobid argument",argv[0]);
        return;
    }

    if argv[0].as_str() == "fg" {
        let result;
        let num;
        let mut jid = false;
        if argv[1].find("%") != None {
            jid = true;
            result = argv[1].clone().drain(1..).collect::<String>().parse::<u32>();
        }
        else {
            result = argv[1].clone().drain(..).collect::<String>().parse::<u32>();
        }

        match result {
            Ok(val) => num = val,
            Err(_) => {
                eprintln!("{}: argument must be a PID or %jobid",argv[0]);
                return;
            },
        }
        
        let job: Option<&mut Job>;
        if jid {
            job = unsafe { JOBS.get_job_jid(num) };
        }
        else {
            job = unsafe { JOBS.get_job_pid(num as i32) };
        }

        match job {
            Some(job) => {
                job.state = ProccessState::FG;
                signal::kill(Pid::from_raw(-job.pgid),Signal::SIGCONT).unwrap();

                waitfg(job.pgid);
            },
            None => {
                if jid {
                    eprintln!("%{}: No such job",num);
                }
                else {
                    eprintln!("({}): No such process",num);
                }
            }
        }
    }
    else if argv[0].as_str() == "bg" {
        let result;
        let num;
        let mut jid = false;
        if argv[1].find("%") != None {
            jid = true;
            result = argv[1].clone().drain(1..).collect::<String>().parse::<u32>();
        }
        else {
            result = argv[1].clone().drain(..).collect::<String>().parse::<u32>();
        }

        match result {
            Ok(val) => num = val,
            Err(_) => {
                eprintln!("{}: argument must be a PID or %jobid",argv[0]);
                return;
            },
        }
        
        let job: Option<&mut Job>;
        if jid {
            job = unsafe { JOBS.get_job_jid(num) };
        }
        else {
            job = unsafe { JOBS.get_job_pid(num as i32) };
        }

        match job {
            Some(job) => {
                job.state = ProccessState::BG;
                signal::kill(Pid::from_raw(-job.pgid),Signal::SIGCONT).unwrap();
            },
            None => {
                if jid {
                    eprintln!("%{}: No such job",num);
                }
                else {
                    eprintln!("({}): No such process",num);
                }
            }
        }
    }
}

fn waitfg(pid: i32) {
    let mut counter = 0;
    loop {
        let job = unsafe {JOBS.get_job_pid(pid)};
       
        match job {
            Some(x) => match x.state {
                ProccessState::FG => {
                    if unsafe { VERBOSE == 1} {
                        println!("{}", x);
                    }
                    if counter == 0 {
                        pause();
                    }
                }, 
                _ => break
            }
            None => break,
        }
        counter += 1;
    }

    if unsafe { VERBOSE == 1} {
        println!("Broke out");
    }
}


fn builtin_cmd(argv: &Vec<String>,aliases: &mut BTreeMap<String,(String,Vec<String>)>, variables: &mut BTreeMap<String, String>) -> i32 {
    if argv.len() == 0 {
        return 1;
    }
    else if argv[0].as_str() == "quit" {
        process::exit(0);
    }
    else if argv[0].as_str() == "exit" {
        process::exit(0);
    }
    else if argv[0].as_str() == "jobs" {
        unsafe {print!("{}",JOBS);}
        io::stdout().flush().unwrap();
 
        return 1;
    }
    else if argv[0].as_str() == "fg" {
        do_bgfg(&argv);
        return 1;
    }
    else if argv[0].as_str() == "bg" {
        do_bgfg(&argv);
        return 1;
    }
    else if argv[0].as_str() == "cd" {
        builtin::change_dir(argv); 
        return 1;
    }
    else if argv[0].as_str() == "alias" {
        builtin::alias(argv, aliases);
        return 1;
    }
    else if argv[0].as_str() == "export" {
        builtin::export(argv);
        return 1;
    }
    else if argv[0].as_str() == "vars" {
        builtin::print_vars(variables);
        return 1;
    }
    else if argv.len() == 3 && argv[1].as_str() == "=" {
        builtin::variable(argv,variables);
        return 1;
    }
    
    return 0;

}


fn usage() {
    println!("Usage: shell [-hvp]");
    println!("   -h   print this message");
    println!("   -v   print additional diagnostic information");
    println!("   -p   do not emit a command prompt");
    println!("   -a   include the path in the prompt");
    process::exit(1);

}
